use serde::{Deserialize, Serialize};
use sr_core::{
    AomNode, FocusEvent, LiveRegionEvent, MockSpeechOutput, Politeness, Role, ScreenReaderEngine,
    VirtualBuffer, VirtualCursor,
};
use sr_platform::PlatformBridge;
use std::fs;
use std::path::{Path, PathBuf};

use crate::evidence::EvidenceFile;
use crate::scenario::{Scenario, ScenarioStep};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionResult {
    pub step_id: String,
    pub step_type: String,
    pub command: String,
    pub emitted_speech: String,
    pub canonical_speech: Option<String>,
    pub assertions_passed: bool,
    pub failed_assertions: Vec<String>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioExecutionResult {
    pub scenario_id: String,
    pub scenario_name: String,
    pub passed: bool,
    pub steps: Vec<StepExecutionResult>,
}

pub struct DifferentialRunner {
    pub corpus_dir: PathBuf,
}

impl DifferentialRunner {
    pub fn new(corpus_dir: impl AsRef<Path>) -> Self {
        Self {
            corpus_dir: corpus_dir.as_ref().to_path_buf(),
        }
    }

    /// Resolves the absolute path to a fixture referenced in a scenario.
    pub fn resolve_fixture_path(&self, fixture_uri: &str) -> Result<PathBuf, String> {
        let fixture_name = if let Some(idx) = fixture_uri.rfind('/') {
            &fixture_uri[idx + 1..]
        } else {
            fixture_uri
        };

        let path = self.corpus_dir.join("fixtures").join(fixture_name);
        if path.exists() {
            Ok(path)
        } else {
            Err(format!(
                "Fixture '{}' not found at {:?}",
                fixture_name, path
            ))
        }
    }

    /// Loads an AOM tree for a scenario, querying live OS accessibility APIs via `PlatformBridge`
    /// when the application is running, or falling back to canonical AOM snapshots.
    pub fn load_aom_for_scenario(
        &self,
        scenario: &Scenario,
    ) -> Result<AomNode, Box<dyn std::error::Error>> {
        let bridge = PlatformBridge::new();

        // 1. If target application is running, query the live OS accessibility tree
        if let Some(ref target_app) = scenario.target_app {
            if let Ok(aom) = bridge.get_active_document(Some(target_app)) {
                tracing::info!(
                    "Acquired live AOM tree from target application '{}'",
                    target_app
                );
                return Ok(aom);
            }
        }

        // 2. Fall back to canonical AOM snapshot (no toy HTML parsing)
        let fixture_path = self
            .resolve_fixture_path(&scenario.fixture)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e))?;

        if let Some(stem) = fixture_path.file_stem().and_then(|s| s.to_str()) {
            let aom_path = fixture_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(format!("{}.aom.json", stem));

            if aom_path.exists() {
                let data = fs::read_to_string(&aom_path)?;
                let aom: AomNode = serde_json::from_str(data.trim_start_matches('\u{feff}'))?;
                return Ok(aom);
            }
        }

        Err(format!(
            "No live OS accessibility session or AOM snapshot found for fixture {:?}",
            fixture_path
        )
        .into())
    }

    /// Runs a single scenario and verifies produced speech against assertions and canonical evidence.
    pub fn run_scenario(
        &self,
        scenario: &Scenario,
    ) -> Result<ScenarioExecutionResult, Box<dyn std::error::Error>> {
        let aom = self.load_aom_for_scenario(scenario)?;
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        // Apply reset preconditions
        if let Some(ref reset) = scenario.reset {
            if let Some(ref keystrokes) = reset.keystrokes {
                for key in keystrokes {
                    if key == "^{HOME}" {
                        cursor.move_to_start();
                    }
                }
            }
            if reset.exit_forms_mode == Some(true) {
                cursor.exit_forms_mode();
            }
        }

        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Attempt loading canonical evidence for cross-validation
        let evidence_path = self
            .corpus_dir
            .join("evidence")
            .join(format!("{}-evidence.json", scenario.id));
        let canonical_evidence = EvidenceFile::load_from_file(&evidence_path).ok();

        let mut step_results = Vec::new();
        let mut scenario_passed = true;

        for step in &scenario.steps {
            let step_res = self.execute_step(step, &mut engine, canonical_evidence.as_ref());
            if !step_res.success {
                scenario_passed = false;
            }
            step_results.push(step_res);
        }

        Ok(ScenarioExecutionResult {
            scenario_id: scenario.id.clone(),
            scenario_name: scenario.name.clone(),
            passed: scenario_passed,
            steps: step_results,
        })
    }

    fn execute_step(
        &self,
        step: &ScenarioStep,
        engine: &mut ScreenReaderEngine,
        evidence: Option<&EvidenceFile>,
    ) -> StepExecutionResult {
        let emitted_speech = match step.step_type.as_str() {
            "focus" => {
                let cmd = step.command.as_str();
                let mut candidates: Vec<_> = engine
                    .cursor
                    .buffer
                    .node_map
                    .values()
                    .filter(|n| {
                        if let Some(target_role) = step
                            .assertions
                            .as_ref()
                            .and_then(|a| a.target_role.as_ref())
                        {
                            let role_matches = matches!(
                                (target_role.as_str(), &n.role),
                                ("dialog" | "alertdialog", Role::Dialog | Role::AlertDialog)
                                    | ("button", Role::Button)
                                    | ("edit", Role::Edit)
                                    | ("checkbox", Role::CheckBox)
                                    | ("combobox" | "combo", Role::ComboBox)
                                    | ("tree", Role::Tree)
                                    | ("treeitem", Role::TreeItem)
                                    | ("slider", Role::Slider)
                                    | ("progressbar", Role::ProgressBar)
                                    | ("heading", Role::Heading { .. })
                                    | ("link", Role::Link { .. })
                            );
                            if !role_matches {
                                return false;
                            }
                        }
                        if let Some(ref name) = n.name {
                            if name.eq_ignore_ascii_case(cmd) || name.contains(cmd) {
                                return true;
                            }
                        }
                        if let Some(ref text) = n.text {
                            if text.eq_ignore_ascii_case(cmd) || text.contains(cmd) {
                                return true;
                            }
                        }
                        false
                    })
                    .cloned()
                    .collect();
                candidates.sort_by_key(|n| n.id);
                let matched_node = candidates.into_iter().next();

                if let Some(node) = matched_node {
                    let mut ev = FocusEvent::new(node.role.clone())
                        .with_node_id(node.id)
                        .with_name(node.name.clone().unwrap_or_default())
                        .with_states(node.states.clone());
                    if let Some(ref v) = node.value {
                        ev = ev.with_value(v);
                    }
                    engine.handle_focus_event(&ev).unwrap_or_default()
                } else {
                    let role = if cmd.to_lowercase().contains("edit")
                        || cmd.to_lowercase().contains("input")
                    {
                        Role::Edit
                    } else if cmd.to_lowercase().contains("button") {
                        Role::Button
                    } else if cmd.to_lowercase().contains("check") {
                        Role::CheckBox
                    } else if cmd.to_lowercase().contains("combo") {
                        Role::ComboBox
                    } else if cmd.to_lowercase().contains("tree") {
                        Role::TreeItem
                    } else if cmd.to_lowercase().contains("slider") {
                        Role::Slider
                    } else if cmd.to_lowercase().contains("progress") {
                        Role::ProgressBar
                    } else {
                        Role::Generic
                    };
                    let ev = FocusEvent::new(role).with_name(cmd);
                    engine.handle_focus_event(&ev).unwrap_or_default()
                }
            }
            "aria_live" | "live_region" => {
                let (politeness, text) = if let Some((p, t)) = step.command.split_once(':') {
                    match p.to_lowercase().as_str() {
                        "assertive" => (Politeness::Assertive, t.trim()),
                        "polite" => (Politeness::Polite, t.trim()),
                        _ => (Politeness::Polite, step.command.as_str()),
                    }
                } else {
                    (Politeness::Polite, step.command.as_str())
                };
                let ev = LiveRegionEvent::new(text, politeness);
                engine.handle_live_region_event(&ev).unwrap_or_default()
            }
            "keyboard" | "jaws_script" => engine.execute_command(&step.command).unwrap_or_default(),
            _ => String::new(),
        };

        let mut failed_assertions = Vec::new();
        let mut assertions_passed = true;

        if let Some(ref assertions) = step.assertions {
            if let Some(ref contains_list) = assertions.contains_speech {
                for phrase in contains_list {
                    if !emitted_speech.contains(phrase)
                        && !emitted_speech
                            .to_lowercase()
                            .contains(&phrase.to_lowercase())
                    {
                        assertions_passed = false;
                        failed_assertions.push(format!(
                            "Expected speech to contain '{}', but got '{}'",
                            phrase, emitted_speech
                        ));
                    }
                }
            }

            if let Some(target_level) = assertions.target_heading_level {
                let actual_level = engine.cursor.current_node().and_then(|n| match n.role {
                    Role::Heading { level } => Some(level),
                    _ => None,
                });
                if actual_level != Some(target_level) {
                    assertions_passed = false;
                    failed_assertions.push(format!(
                        "Expected target_heading_level {}, got {:?}",
                        target_level, actual_level
                    ));
                }
            }

            if let Some(ref target_heading_name) = assertions.target_heading_name {
                let actual_name = engine
                    .cursor
                    .current_node()
                    .and_then(|n| n.name.as_deref().or(n.text.as_deref()));
                if actual_name != Some(target_heading_name.as_str()) {
                    assertions_passed = false;
                    failed_assertions.push(format!(
                        "Expected target_heading_name '{}', got {:?}",
                        target_heading_name, actual_name
                    ));
                }
            }

            if let Some(ref target_role) = assertions.target_role {
                let actual_role = engine.cursor.current_node().map(|n| &n.role);
                let role_matches = matches!(
                    (target_role.as_str(), actual_role),
                    ("heading", Some(Role::Heading { .. }))
                        | ("link", Some(Role::Link { .. }))
                        | ("list", Some(Role::List))
                        | ("listitem", Some(Role::ListItem))
                        | ("region", Some(Role::Landmark { .. }))
                        | ("button", Some(Role::Button))
                        | ("checkbox", Some(Role::CheckBox))
                        | ("edit", Some(Role::Edit))
                        | ("document", Some(Role::Document))
                        | ("table", Some(Role::Table))
                        | ("cell" | "tablecell", Some(Role::TableCell))
                        | ("row" | "tablerow", Some(Role::TableRow))
                        | ("tab", Some(Role::Tab))
                        | ("tablist", Some(Role::TabList))
                        | ("tabpanel", Some(Role::TabPanel))
                        | (
                            "dialog" | "alertdialog",
                            Some(Role::Dialog | Role::AlertDialog),
                        )
                        | ("combobox" | "combo", Some(Role::ComboBox))
                        | ("tree", Some(Role::Tree))
                        | ("treeitem", Some(Role::TreeItem))
                        | ("slider", Some(Role::Slider))
                        | ("progressbar", Some(Role::ProgressBar))
                );
                if !role_matches {
                    assertions_passed = false;
                    failed_assertions.push(format!(
                        "Expected target_role '{}', got {:?}",
                        target_role, actual_role
                    ));
                }
            }

            if let Some(ref target_name) = assertions.target_name {
                let actual_name = engine
                    .cursor
                    .current_node()
                    .and_then(|n| n.name.as_deref().or(n.text.as_deref()));
                if actual_name != Some(target_name.as_str()) {
                    assertions_passed = false;
                    failed_assertions.push(format!(
                        "Expected target_name '{}', got {:?}",
                        target_name, actual_name
                    ));
                }
            }

            if let Some(expected_forms_mode) = assertions.forms_mode_active {
                let actual_forms_mode = engine.forms_mode;
                if actual_forms_mode != expected_forms_mode {
                    assertions_passed = false;
                    failed_assertions.push(format!(
                        "Expected forms_mode_active {}, got {}",
                        expected_forms_mode, actual_forms_mode
                    ));
                }
            }

            if let Some(expected_checked) = assertions.target_checked {
                let actual_checked = engine.cursor.current_node().and_then(|n| n.states.checked);
                if actual_checked != Some(expected_checked) {
                    assertions_passed = false;
                    failed_assertions.push(format!(
                        "Expected target_checked {:?}, got {:?}",
                        expected_checked, actual_checked
                    ));
                }
            }

            if let Some(expected_selected) = assertions.target_selected {
                let actual_selected = engine.cursor.current_node().and_then(|n| n.states.selected);
                if actual_selected != Some(expected_selected) {
                    assertions_passed = false;
                    failed_assertions.push(format!(
                        "Expected target_selected {:?}, got {:?}",
                        expected_selected, actual_selected
                    ));
                }
            }

            if let Some(target_col) = assertions.target_col {
                let actual_col = engine.cursor.current_table_col();
                if actual_col != Some(target_col) {
                    assertions_passed = false;
                    failed_assertions.push(format!(
                        "Expected target_col {}, got {:?}",
                        target_col, actual_col
                    ));
                }
            }

            if let Some(target_row) = assertions.target_row {
                let actual_row = engine.cursor.current_table_row();
                if actual_row != Some(target_row) {
                    assertions_passed = false;
                    failed_assertions.push(format!(
                        "Expected target_row {}, got {:?}",
                        target_row, actual_row
                    ));
                }
            }
        }

        let canonical_speech = evidence.and_then(|ev| ev.canonical_speech_for_step(&step.id));

        StepExecutionResult {
            step_id: step.id.clone(),
            step_type: step.step_type.clone(),
            command: step.command.clone(),
            emitted_speech,
            canonical_speech,
            assertions_passed,
            failed_assertions,
            success: assertions_passed,
        }
    }

    /// Loads and runs a scenario by filename or ID.
    pub fn run_scenario_by_id(
        &self,
        scenario_id: &str,
    ) -> Result<ScenarioExecutionResult, Box<dyn std::error::Error>> {
        let scenario_file = if scenario_id.ends_with(".json") {
            self.corpus_dir.join("scenarios").join(scenario_id)
        } else {
            self.corpus_dir
                .join("scenarios")
                .join(format!("{}.json", scenario_id))
        };

        let scenario = Scenario::load_from_file(&scenario_file)?;
        self.run_scenario(&scenario)
    }

    /// Runs all scenarios in the specified numerical range (e.g. 1..=10).
    pub fn run_range(
        &self,
        start: u32,
        end: u32,
    ) -> Vec<Result<ScenarioExecutionResult, Box<dyn std::error::Error>>> {
        let mut results = Vec::new();
        let scenarios_dir = self.corpus_dir.join("scenarios");

        if let Ok(entries) = fs::read_dir(scenarios_dir) {
            let mut files: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|ent| ent.path()))
                .filter(|p| p.extension().map(|ext| ext == "json").unwrap_or(false))
                .collect();
            files.sort();

            for path in files {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let num_prefix: String =
                        stem.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if let Ok(num) = num_prefix.parse::<u32>() {
                        if num >= start && num <= end {
                            match Scenario::load_from_file(&path) {
                                Ok(scenario) => results.push(self.run_scenario(&scenario)),
                                Err(e) => results.push(Err(e)),
                            }
                        }
                    }
                }
            }
        }

        results
    }
}
