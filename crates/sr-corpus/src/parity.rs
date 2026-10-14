//! Differential parity against recorded JAWS speech.
//!
//! For every scenario step with genuine evidence, the engine is driven with the same
//! command JAWS received, over the snapshot Chrome produced for the same fixture on the
//! reference machine (through IA2, as JAWS reads Chrome; see
//! [`crate::snapshots::PARITY_API`]), and its speech is compared with every distinct
//! transcript JAWS produced across trials. The per-step outcome is ratcheted in `corpus/parity-baseline.json`, a
//! generated file: `cargo test` fails on any regression and whenever the baseline is stale.

use crate::scenario::Scenario;
use crate::snapshots::{self, Api};
use serde::Serialize;
use serde_json::Value;
use sr_core::{ScreenReaderEngine, VirtualBuffer, VirtualCursor};
use sr_platform::event_trace::{menu_event_from_trace, EventConverter, TraceEvent};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const BASELINE_FILE: &str = "parity-baseline.json";

/// The documented normalizer: unify line endings, collapse whitespace runs (including
/// newlines) to one space, and trim. The `\u{1d}`/`\u{1e}` role markers are kept, because
/// they encode JAWS speech structure.
pub fn normalize(speech: &str) -> String {
    speech.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, Clone, Serialize)]
pub struct StepParity {
    /// `<scenario_id>/<step_id>`.
    pub key: String,
    pub command: String,
    pub engine: String,
    /// Distinct transcripts JAWS produced for this step across successful trials.
    pub jaws_variants: Vec<String>,
    pub matched: bool,
}

#[derive(Debug, Default)]
pub struct ParityReport {
    pub steps: Vec<StepParity>,
    /// Scenarios that could not be evaluated, with the reason. These count as failures.
    pub errors: Vec<String>,
}

impl ParityReport {
    pub fn matched(&self) -> usize {
        self.steps.iter().filter(|s| s.matched).count()
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("{path:?}: {e}"))
}

/// Distinct raw transcripts per step id, in first-seen order, from successful trials only.
fn jaws_variants(evidence: &Value) -> BTreeMap<String, Vec<String>> {
    let mut variants: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let trials = evidence
        .get("trials")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for trial in trials
        .iter()
        .filter(|t| t.get("trial_success").and_then(Value::as_bool) == Some(true))
    {
        for step in trial
            .get("step_results")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let (Some(id), Some(speech)) = (
                step.get("step_id").and_then(Value::as_str),
                step.pointer("/observation/speech_raw")
                    .and_then(Value::as_str),
            ) else {
                continue;
            };
            let entry = variants.entry(id.to_owned()).or_default();
            if !entry.iter().any(|v| v == speech) {
                entry.push(speech.to_owned());
            }
        }
    }
    variants
}

fn run_scenario(
    corpus_dir: &Path,
    scenario: &Scenario,
    api: Api,
) -> Result<Vec<StepParity>, String> {
    let evidence_path = corpus_dir
        .join("evidence")
        .join(format!("{}-evidence.json", scenario.id));
    let variants = jaws_variants(&read_json(&evidence_path)?);

    // Desktop scenarios have no document: the engine starts in native-app mode and its speech
    // comes from the replayed OS events. Browser scenarios load the captured document tree.
    let desktop = scenario.launch.is_some();
    let aom = if desktop {
        sr_core::AomNode::new(0, sr_core::Role::Document)
    } else {
        let fixture = scenario.fixture.rsplit('/').next().unwrap_or_default();
        let snapshot = snapshots::load(&snapshots::path(corpus_dir, fixture, api))?;
        let fixture_bytes =
            fs::read(corpus_dir.join("fixtures").join(fixture)).map_err(|e| format!("{e}"))?;
        let fixture_hash = crate::manifest::sha256_hex(&fixture_bytes);
        if snapshot.fixture_sha256.as_deref() != Some(fixture_hash.as_str()) {
            return Err(format!(
                "snapshot for {fixture} was not captured from the current fixture bytes"
            ));
        }
        snapshot.aom
    };
    let mut cursor = VirtualCursor::new(VirtualBuffer::from_aom(&aom));
    let spoken = RecordingSpeech::default();
    if let Some(reset) = &scenario.reset {
        for key in reset.keystrokes.iter().flatten() {
            match key.as_str() {
                "^{HOME}" => {
                    // Ctrl+Home is silent setup; its announcement is not part of any step.
                    let _ = cursor.move_to_start();
                }
                other => return Err(format!("unsupported reset keystroke {other:?}")),
            }
        }
        if reset.exit_forms_mode == Some(true) {
            cursor.exit_forms_mode();
        }
    }
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(spoken.clone()));
    // Actions succeed as they did live; their effects arrive as the recorded events.
    engine.set_actions(Box::new(sr_core::actions::RecordingActions::default()));
    if desktop {
        engine.app_mode = sr_core::engine::AppMode::NativeControl;
    }
    for chord in scenario.reset.iter().flat_map(|r| r.keys.iter().flatten()) {
        press(&mut engine, chord)?;
    }
    spoken.take();

    let replay = StepEvents::load(corpus_dir, &scenario.id, &read_json(&evidence_path)?)?;
    // One converter for the whole run, as live: it tracks the focused object.
    let mut converter = EventConverter::default();
    for event in &replay.setup {
        if let Some(converted) = converter.convert(event) {
            let _ = engine.handle_accessibility_event(&converted);
        }
    }
    spoken.take();

    let mut steps = Vec::new();
    for step in &scenario.steps {
        let engine_speech = match step.step_type.as_str() {
            "key" => {
                press(&mut engine, &step.command)?;
                // Replay the OS events recorded while JAWS handled this step (e.g. the focus
                // change that follows Tab), exactly as the live listener would deliver them.
                for event in replay.for_step(&step.id) {
                    let Some(converted) = converter.convert(&event) else {
                        continue;
                    };
                    if let Some(speech) = engine.handle_accessibility_event(&converted) {
                        if spoken.is_empty() {
                            spoken.push(&speech);
                        }
                    }
                }
                // The step's events are over; JAWS then speaks what it deferred.
                let _ = engine.on_idle();
                spoken.take().join("\n")
            }
            "keyboard" | "jaws_script" => {
                let speech = engine.execute_command(&step.command).unwrap_or_default();
                spoken.take();
                speech
            }
            other => {
                return Err(format!(
                    "step {} has type {other:?}, which the capture tooling cannot replay",
                    step.id
                ))
            }
        };
        let jaws = variants.get(&step.id).cloned().unwrap_or_default();
        if jaws.is_empty() {
            return Err(format!("no JAWS transcript recorded for step {}", step.id));
        }
        let normalized = normalize(&engine_speech);
        let matched = jaws.iter().any(|v| normalize(v) == normalized);
        steps.push(StepParity {
            key: format!("{}/{}", scenario.id, step.id),
            command: step.command.clone(),
            engine: engine_speech,
            jaws_variants: jaws,
            matched,
        });
    }
    Ok(steps)
}

/// Runs every scenario in `corpus_dir/scenarios` against its evidence and snapshot.
pub fn run(corpus_dir: &Path) -> ParityReport {
    run_with(corpus_dir, snapshots::PARITY_API)
}

/// Runs every scenario against its evidence and the `api` snapshot of its fixture.
pub fn run_with(corpus_dir: &Path, api: Api) -> ParityReport {
    let mut report = ParityReport::default();
    let mut files: Vec<PathBuf> = fs::read_dir(corpus_dir.join("scenarios"))
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.retain(|p| p.extension().is_some_and(|e| e == "json"));
    files.sort();

    for path in files {
        match Scenario::load_from_file(&path) {
            Ok(scenario) => match run_scenario(corpus_dir, &scenario, api) {
                Ok(steps) => report.steps.extend(steps),
                Err(e) => report.errors.push(format!("{}: {e}", scenario.id)),
            },
            Err(e) => report.errors.push(format!("{path:?}: {e}")),
        }
    }
    report
}

#[derive(Debug, Serialize)]
struct Baseline<'a> {
    notice: &'static str,
    matched_steps: usize,
    total_steps: usize,
    errors: &'a [String],
    steps: BTreeMap<&'a str, &'static str>,
}

/// Renders the baseline exactly as stored on disk.
pub fn render_baseline(report: &ParityReport) -> Result<String, String> {
    let baseline = Baseline {
        notice: "GENERATED FILE - do not edit. Regenerate: cargo run -p sr-cli -- parity --write-baseline",
        matched_steps: report.matched(),
        total_steps: report.steps.len(),
        errors: &report.errors,
        steps: report
            .steps
            .iter()
            .map(|s| {
                (
                    s.key.as_str(),
                    if s.matched { "match" } else { "mismatch" },
                )
            })
            .collect(),
    };
    serde_json::to_string_pretty(&baseline)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}

/// Compares `report` with the committed baseline. Regressions (a step that matched JAWS and
/// no longer does) are reported first; any other difference means the baseline is stale.
pub fn check_baseline(corpus_dir: &Path, report: &ParityReport) -> Result<(), String> {
    let committed_text = fs::read_to_string(corpus_dir.join(BASELINE_FILE)).unwrap_or_default();
    let committed: Value = serde_json::from_str(&committed_text).unwrap_or(Value::Null);
    let regressions: Vec<&str> = report
        .steps
        .iter()
        .filter(|s| {
            !s.matched
                && committed.pointer(&format!("/steps/{}", s.key.replace('/', "~1")))
                    == Some(&Value::String("match".to_owned()))
        })
        .map(|s| s.key.as_str())
        .collect();
    if !regressions.is_empty() {
        return Err(format!(
            "JAWS parity regressed for: {}",
            regressions.join(", ")
        ));
    }
    if committed_text.replace("\r\n", "\n") != render_baseline(report)? {
        return Err(format!(
            "{BASELINE_FILE} is stale (parity changed). Review `cargo run -p sr-cli -- parity`, \
             then regenerate with: cargo run -p sr-cli -- parity --write-baseline"
        ));
    }
    Ok(())
}

pub fn write_baseline(corpus_dir: &Path, report: &ParityReport) -> Result<(), String> {
    fs::write(corpus_dir.join(BASELINE_FILE), render_baseline(report)?).map_err(|e| e.to_string())
}

/// Speech sink that records every utterance, shared with the harness after the engine owns it.
#[derive(Debug, Clone, Default)]
struct RecordingSpeech(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl RecordingSpeech {
    fn is_empty(&self) -> bool {
        self.0.lock().map(|log| log.is_empty()).unwrap_or(true)
    }

    fn push(&self, text: &str) {
        if let Ok(mut log) = self.0.lock() {
            log.push(text.to_owned());
        }
    }

    fn take(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|mut log| std::mem::take(&mut *log))
            .unwrap_or_default()
    }
}

impl sr_core::speech::SpeechOutput for RecordingSpeech {
    /// JAWS's speech history has no typing echo, so the replay records none either.
    fn speak_echo(&mut self, _text: &str) {}

    fn speak(&mut self, text: &str, _interrupt: bool) {
        if let Ok(mut log) = self.0.lock() {
            log.push(text.to_owned());
        }
    }

    fn stop(&mut self) {}
}

/// Presses and releases `chord` through the engine's live key-handling entry point.
fn press(engine: &mut ScreenReaderEngine, chord: &str) -> Result<(), String> {
    let mut event = crate::keys::parse_chord(chord)?;
    let _ = engine.handle_key_event(&event);
    event.is_down = false;
    let _ = engine.handle_key_event(&event);
    Ok(())
}

/// Events the engine consumes: focus changes (with a UIA chain or an MSAA object) and menu
/// mode transitions.
fn replayable(event: &TraceEvent) -> bool {
    menu_event_from_trace(event).is_some()
        || (matches!(
            event.event.as_str(),
            "focus" | "foreground" | "caret" | "state_change" | "value_change" | "selection"
        ) && (!event.focus_chain.is_empty() || event.object.is_some()))
}

/// Replayable events recorded during each step of the first successful traced trial
/// (`corpus/traces/<trace_file>`, recorded with `run-scenario.ps1 -TraceDir`).
#[derive(Default)]
struct StepEvents {
    by_step: std::collections::HashMap<String, Vec<sr_platform::event_trace::TraceEvent>>,
    /// Focus events before the first step (launch and reset), replayed silently so the
    /// engine knows the window and focus JAWS saw when step 1 began.
    setup: Vec<sr_platform::event_trace::TraceEvent>,
}

impl StepEvents {
    fn load(corpus_dir: &Path, scenario_id: &str, evidence: &Value) -> Result<Self, String> {
        // The first successful trial that recorded a trace, with its own step timings.
        let traced = evidence
            .get("trials")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|t| t.get("trial_success").and_then(Value::as_bool) == Some(true))
            .find_map(|t| Some((t, t.get("trace_file")?.as_str()?)));
        let Some((trial, trace_file)) = traced else {
            return Ok(Self::default());
        };
        let trace_path = corpus_dir.join("traces").join(trace_file);
        let (_, events) = sr_platform::event_trace::read_trace(&trace_path)?;
        let mut by_step = std::collections::HashMap::new();
        let steps = trial
            .get("step_results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let first_start = steps
            .first()
            .and_then(|s| s.get("t_start_ms"))
            .and_then(Value::as_u64)
            .map(u128::from)
            .unwrap_or(0);
        let setup = events
            .iter()
            .filter(|e| e.event == "focus" && e.t_ms < first_start && replayable(e))
            .cloned()
            .collect();
        for step in steps {
            let (Some(id), Some(start), Some(end)) = (
                step.get("step_id").and_then(Value::as_str),
                step.get("t_start_ms").and_then(Value::as_u64),
                step.get("t_end_ms").and_then(Value::as_u64),
            ) else {
                return Err(format!(
                    "{scenario_id}: traced trial steps lack t_start_ms/t_end_ms"
                ));
            };
            let window: Vec<_> = events
                .iter()
                .filter(|e| {
                    replayable(e) && e.t_ms >= u128::from(start) && e.t_ms <= u128::from(end)
                })
                .cloned()
                .collect();
            by_step.insert(id.to_owned(), window);
        }
        Ok(Self { by_step, setup })
    }

    fn for_step(&self, step_id: &str) -> Vec<sr_platform::event_trace::TraceEvent> {
        self.by_step.get(step_id).cloned().unwrap_or_default()
    }
}
