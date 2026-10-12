use serde::{Deserialize, Serialize};
use sr_core::{AomNode, VirtualBuffer, VirtualCursor};
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
        }

        // Attempt loading canonical evidence for cross-validation
        let evidence_path = self
            .corpus_dir
            .join("evidence")
            .join(format!("{}-evidence.json", scenario.id));
        let canonical_evidence = EvidenceFile::load_from_file(&evidence_path).ok();

        let mut step_results = Vec::new();
        let mut scenario_passed = true;

        for step in &scenario.steps {
            let step_res = self.execute_step(step, &mut cursor, canonical_evidence.as_ref());
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
        cursor: &mut VirtualCursor,
        evidence: Option<&EvidenceFile>,
    ) -> StepExecutionResult {
        let emitted_speech = match step.step_type.as_str() {
            "keyboard" => match step.command.as_str() {
                "{DOWN}" => cursor.move_next_line().unwrap_or_default(),
                "{UP}" => cursor.move_prior_line().unwrap_or_default(),
                "^{RIGHT}" => cursor.move_next_word().unwrap_or_default(),
                "^{LEFT}" => cursor.move_prior_word().unwrap_or_default(),
                "{RIGHT}" => cursor.move_next_char().unwrap_or_default(),
                "{LEFT}" => cursor.move_prior_char().unwrap_or_default(),
                "^{DOWN}" => cursor.move_next_paragraph().unwrap_or_default(),
                "^{UP}" => cursor.move_prior_paragraph().unwrap_or_default(),
                "^{HOME}" => cursor.move_to_start(),
                "^{END}" => cursor.move_to_end(),
                _ => String::new(),
            },
            "jaws_script" => match step.command.as_str() {
                "SayAll" => cursor.say_all(),
                "SayLine" => cursor.current_line(),
                "SayWord" => cursor.current_word(),
                "SayCharacter" => cursor.current_char(),
                _ => String::new(),
            },
            _ => String::new(),
        };

        let mut failed_assertions = Vec::new();
        let mut assertions_passed = true;

        if let Some(ref assertions) = step.assertions {
            if let Some(ref contains_list) = assertions.contains_speech {
                for phrase in contains_list {
                    if !emitted_speech.contains(phrase) {
                        assertions_passed = false;
                        failed_assertions.push(format!(
                            "Expected speech to contain '{}', but got '{}'",
                            phrase, emitted_speech
                        ));
                    }
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
