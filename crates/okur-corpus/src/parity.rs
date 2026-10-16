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
use okur_core::clock::{FixedClock, LocalDateTime, LocaleFormats};
use okur_core::{Modifiers, ScreenReaderEngine, VirtualBuffer, VirtualCursor};
use okur_platform::event_trace::{is_focus, menu_event_from_trace, EventConverter, TraceEvent};
use serde::Serialize;
use serde_json::Value;
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
    /// The spoken layer, for evidence recorded through the speech recorder voice: what our
    /// synthesizer is given, against what JAWS sent its synthesizer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoken: Option<SpokenParity>,
}

/// One step's spoken-layer comparison ([`okur_core::pronounce`]).
#[derive(Debug, Clone, Serialize)]
pub struct SpokenParity {
    pub engine: String,
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

    /// Steps compared on the spoken layer: how many match, of how many.
    pub fn spoken(&self) -> (usize, usize) {
        let compared: Vec<&SpokenParity> = self
            .steps
            .iter()
            .filter_map(|s| s.spoken.as_ref())
            .collect();
        (
            compared.iter().filter(|s| s.matched).count(),
            compared.len(),
        )
    }
}

/// What JAWS speaks for the capture tooling's own requests (clearing and copying its speech
/// history around every step), which can fall inside a step's spoken window.
const HARNESS_CONFIRMATIONS: [&str; 2] =
    ["Speech history cleared", "Copy speech history to clipboard"];

/// What JAWS sent its synthesizer per step, across successful trials, when the evidence was
/// recorded through the speech recorder voice (`observation.spoken`); `None` otherwise.
fn jaws_spoken_variants(evidence: &Value) -> Option<BTreeMap<String, Vec<String>>> {
    let trials: Vec<&Value> = evidence
        .get("trials")
        .and_then(Value::as_array)?
        .iter()
        .filter(|t| t.get("trial_success").and_then(Value::as_bool) == Some(true))
        .collect();
    let steps = || {
        trials.iter().flat_map(|t| {
            t.get("step_results")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
    };
    let recorded = steps().any(|s| {
        s.pointer("/observation/spoken")
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
    });
    if !recorded {
        return None;
    }
    let mut variants: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for step in steps() {
        let Some(id) = step.get("step_id").and_then(Value::as_str) else {
            continue;
        };
        // The records keep each fragment's pitch (a capital letter's); `spoken` is their text.
        let records: Vec<String> = match step.pointer("/observation/spoken_records") {
            Some(Value::Array(records)) => records.iter().map(record_text).collect(),
            Some(record @ Value::Object(_)) => vec![record_text(record)],
            _ => step
                .pointer("/observation/spoken")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
        };
        let spoken = records
            .iter()
            .filter(|s| !HARNESS_CONFIRMATIONS.contains(&s.trim()))
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        let entry = variants.entry(id.to_owned()).or_default();
        if !entry.contains(&spoken) {
            entry.push(spoken);
        }
    }
    Some(variants)
}

/// One speech recorder record's text, as run-scenario.ps1 derives `spoken` from it (its
/// spoken fragments joined by spaces), with each fragment's pitch change marked.
fn record_text(record: &Value) -> String {
    let frags = match record.get("frags") {
        Some(Value::Array(frags)) => frags.iter().collect(),
        Some(frag @ Value::Object(_)) => vec![frag],
        _ => Vec::new(),
    };
    frags
        .into_iter()
        .filter(|f| {
            matches!(
                f.get("action").and_then(Value::as_str),
                Some("speak" | "spell" | "pronounce")
            )
        })
        .map(|f| {
            let text = f.get("text").and_then(Value::as_str).unwrap_or_default();
            let pitch = f.get("pitch").and_then(Value::as_i64).unwrap_or(0);
            okur_core::pronounce::pitched(text, i32::try_from(pitch).unwrap_or(0))
        })
        .collect::<Vec<_>>()
        .join(" ")
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
    let evidence = read_json(&evidence_path)?;
    let variants = jaws_variants(&evidence);
    let spoken_variants = jaws_spoken_variants(&evidence);

    // Desktop scenarios have no document: the engine starts in native-app mode and its speech
    // comes from the replayed OS events. Browser scenarios load the captured document tree.
    let desktop = scenario.launch.is_some();
    let aom = if desktop {
        okur_core::AomNode::new(0, okur_core::Role::Document)
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
    engine.set_actions(Box::new(okur_core::actions::RecordingActions::default()));
    // JAWS's settings when the evidence was recorded.
    engine.settings = replay_settings(&evidence);
    if desktop {
        engine.app_mode = okur_core::engine::AppMode::NativeControl;
    }
    // Say System Time reads a clock standing at each step's recorded start.
    let clock = replay_clock(&evidence)?;
    if let Some((clock, _)) = &clock {
        engine.set_clock(Box::new(clock.clone()));
    }
    for chord in scenario.reset.iter().flat_map(|r| r.keys.iter().flatten()) {
        press(&mut engine, chord, 0, Modifiers::none())?;
    }
    spoken.take();
    spoken.take_spoken();

    let replay = StepEvents::load(corpus_dir, &scenario.id, &evidence)?;
    // One converter for the whole run, as live: it tracks the focused object.
    let mut converter = EventConverter::default();
    for event in &replay.setup {
        for converted in converter.convert(event) {
            let _ = engine.handle_accessibility_event(&converted);
        }
    }
    spoken.take();
    spoken.take_spoken();

    let mut steps = Vec::new();
    // Keys held down by key_down steps, part of every key until released.
    let mut held = Modifiers::none();
    for (index, step) in scenario.steps.iter().enumerate() {
        // Key times keep separate steps from looking like a double press; without recorded
        // times, steps are spaced apart.
        let start = replay
            .starts
            .get(&step.id)
            .copied()
            .unwrap_or(10_000 * (index as u64 + 1));
        if let Some((clock, offset)) = &clock {
            clock.set(LocalDateTime::from_unix_ms(
                i64::try_from(start).unwrap_or(0),
                *offset,
            ));
        }
        let engine_speech = match step.step_type.as_str() {
            "key" | "key_down" | "key_up" => {
                match step.step_type.as_str() {
                    // A key held across steps (Alt through several Tabs), and its release.
                    "key_down" | "key_up" => {
                        let down = step.step_type == "key_down";
                        let mut event = crate::keys::parse_chord(&step.command)?;
                        hold(&mut held, event.key, down);
                        event.modifiers = with_held(event.modifiers, held);
                        event.is_down = down;
                        event.time_ms = start;
                        let _ = engine.handle_key_event(&event);
                    }
                    _ => {
                        for press_index in 0..step.repeat.unwrap_or(1).max(1) {
                            press(
                                &mut engine,
                                &step.command,
                                start + u64::from(press_index) * REPEAT_GAP_MS,
                                held,
                            )?;
                        }
                    }
                }
                // Replay the OS events recorded while JAWS handled this step (e.g. the focus
                // change that follows Tab), exactly as the live listener would deliver them.
                for event in replay.for_step(&step.id) {
                    for converted in converter.convert(&event) {
                        if let Some(speech) = engine.handle_accessibility_event(&converted) {
                            if spoken.is_empty() {
                                spoken.push(&speech);
                            }
                        }
                    }
                }
                // An edit key that changed nothing reports the field unchanged, as live.
                if let Some(speech) = engine.settle_pending_edit() {
                    if spoken.is_empty() {
                        spoken.push(&speech);
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
        let engine_spoken = spoken.take_spoken().join(" ");
        let spoken_parity = spoken_variants.as_ref().map(|all| {
            let jaws_variants = all.get(&step.id).cloned().unwrap_or_default();
            let ours = normalize(&engine_spoken);
            SpokenParity {
                matched: jaws_variants.iter().any(|v| normalize(v) == ours),
                engine: engine_spoken,
                jaws_variants,
            }
        });
        steps.push(StepParity {
            key: format!("{}/{}", scenario.id, step.id),
            command: step.command.clone(),
            engine: engine_speech,
            jaws_variants: jaws,
            matched,
            spoken: spoken_parity,
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
        notice:
            "GENERATED FILE - do not edit. Regenerate: cargo run -p okur -- parity --write-baseline",
        matched_steps: report.matched(),
        total_steps: report.steps.len(),
        errors: &report.errors,
        steps: report
            .steps
            .iter()
            .map(|s| (s.key.as_str(), if s.matched { "match" } else { "mismatch" }))
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
            "{BASELINE_FILE} is stale (parity changed). Review `cargo run -p okur -- parity`, \
             then regenerate with: cargo run -p okur -- parity --write-baseline"
        ));
    }
    Ok(())
}

pub fn write_baseline(corpus_dir: &Path, report: &ParityReport) -> Result<(), String> {
    fs::write(corpus_dir.join(BASELINE_FILE), render_baseline(report)?).map_err(|e| e.to_string())
}

type Log = std::sync::Arc<std::sync::Mutex<Vec<String>>>;

/// Speech sink that records every utterance twice, shared with the harness after the engine
/// owns it: as JAWS's speech history would keep it, and as the synthesizer is asked to say it
/// ([`okur_core::pronounce`]).
#[derive(Debug, Clone, Default)]
struct RecordingSpeech {
    history: Log,
    spoken: Log,
}

fn append(log: &Log, text: String) {
    if let Ok(mut log) = log.lock() {
        log.push(text);
    }
}

fn drain(log: &Log) -> Vec<String> {
    log.lock()
        .map(|mut log| std::mem::take(&mut *log))
        .unwrap_or_default()
}

impl RecordingSpeech {
    fn is_empty(&self) -> bool {
        self.history
            .lock()
            .map(|log| log.is_empty())
            .unwrap_or(true)
    }

    fn push(&self, text: &str) {
        append(&self.history, text.to_owned());
        append(&self.spoken, okur_core::pronounce::text(text));
    }

    fn take(&self) -> Vec<String> {
        drain(&self.history)
    }

    fn take_spoken(&self) -> Vec<String> {
        drain(&self.spoken)
    }
}

impl okur_core::speech::SpeechOutput for RecordingSpeech {
    /// JAWS's speech history has no typing echo, but its synthesizer speaks it.
    fn speak_echo(&mut self, text: &str) {
        append(&self.spoken, text.to_owned());
    }

    fn speak(&mut self, text: &str, _interrupt: bool) {
        self.push(text);
    }

    fn speak_character(&mut self, text: &str, _interrupt: bool) {
        append(&self.history, text.to_owned());
        let pitch = okur_core::pronounce::character_pitch(text);
        append(
            &self.spoken,
            okur_core::pronounce::pitched(&okur_core::pronounce::character(text), pitch),
        );
    }

    fn stop(&mut self) {}
}

/// Milliseconds between the presses of a repeated step: run-scenario.ps1 sends the chord
/// (30 ms per key down and up) and waits 120 ms before pressing it again.
const REPEAT_GAP_MS: u64 = 300;

/// The replay clock and its UTC offset, from the evidence's recorded culture and time zone.
/// Evidence recorded before the culture was captured has no clock: time commands are silent.
fn replay_clock(evidence: &Value) -> Result<Option<(FixedClock, i32)>, String> {
    let Some(culture) = evidence
        .pointer("/provenance/culture")
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    let formats = match culture {
        "en-US" => LocaleFormats::en_us(),
        other => return Err(format!("no locale formats for recorded culture {other:?}")),
    };
    let offset = evidence
        .pointer("/provenance/utc_offset_minutes")
        .and_then(Value::as_i64)
        .and_then(|m| i32::try_from(m).ok())
        .unwrap_or(0);
    let clock = FixedClock::new(LocalDateTime::from_unix_ms(0, offset), formats);
    Ok(Some((clock, offset)))
}

/// The settings JAWS ran with: from the evidence's recorded `DEFAULT.JCF` options, else the
/// reference machine's configuration (unchanged since before any evidence was recorded).
fn replay_settings(evidence: &Value) -> okur_core::settings::Settings {
    use okur_core::settings::{KeyboardLayout, KeyboardSettings, Settings, TypingEcho};
    let mut settings = Settings::reference_machine();
    let options = evidence.pointer("/provenance/jaws_settings/DEFAULT.JCF/Options");
    if let Some(options) = options {
        // JAWS writes only changed options: no KeyboardType means the desktop layout.
        let layout = options.get("KeyboardType").and_then(Value::as_str);
        let layout = if layout.is_some_and(|l| l.eq_ignore_ascii_case("Laptop")) {
            KeyboardLayout::Laptop
        } else {
            KeyboardLayout::Desktop
        };
        settings.keyboard = KeyboardSettings::for_layout(layout);
        settings.typing_echo = options
            .get("TypingEcho")
            .and_then(Value::as_str)
            .map(TypingEcho::from_jcf)
            .unwrap_or_default();
    }
    settings
}

/// Presses and releases `chord` at `time_ms` (0: unknown) through the engine's live
/// key-handling entry point.
fn press(
    engine: &mut ScreenReaderEngine,
    chord: &str,
    time_ms: u64,
    held: Modifiers,
) -> Result<(), String> {
    let mut event = crate::keys::parse_chord(chord)?;
    event.modifiers = with_held(event.modifiers, held);
    event.time_ms = time_ms;
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
            "focus"
                | "uia_focus"
                | "uia_selected"
                | "foreground"
                | "caret"
                | "text_selection_change"
                | "live_region_change"
                | "state_change"
                | "value_change"
                | "selection"
        ) && (!event.focus_chain.is_empty() || event.object.is_some()))
}

/// Replayable events recorded during each step of the first successful traced trial
/// (`corpus/traces/<trace_file>`, recorded with `run-scenario.ps1 -TraceDir`).
#[derive(Default)]
struct StepEvents {
    by_step: std::collections::HashMap<String, Vec<okur_platform::event_trace::TraceEvent>>,
    /// Focus events before the first step (launch and reset), replayed silently so the
    /// engine knows the window and focus JAWS saw when step 1 began.
    setup: Vec<okur_platform::event_trace::TraceEvent>,
    /// When each step began (unix ms), for the replay clock and key times.
    starts: std::collections::HashMap<String, u64>,
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
        let (_, events) = okur_platform::event_trace::read_trace(&trace_path)?;
        let mut by_step = std::collections::HashMap::new();
        let mut starts = std::collections::HashMap::new();
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
            .filter(|e| is_focus(e) && e.t_ms < first_start && replayable(e))
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
            starts.insert(id.to_owned(), start);
        }
        Ok(Self {
            by_step,
            setup,
            starts,
        })
    }

    fn for_step(&self, step_id: &str) -> Vec<okur_platform::event_trace::TraceEvent> {
        self.by_step.get(step_id).cloned().unwrap_or_default()
    }
}

/// Records a modifier key held down (`down`) or released by a key_down/key_up step.
fn hold(held: &mut Modifiers, key: okur_core::Key, down: bool) {
    use okur_core::Key;
    match key {
        Key::Alt => held.alt = down,
        Key::Control => held.ctrl = down,
        Key::Shift => held.shift = down,
        Key::Windows => held.win = down,
        Key::Insert => held.insert = down,
        Key::CapsLock => held.caps_lock = down,
        _ => {}
    }
}

/// `modifiers` with the held keys added.
fn with_held(mut modifiers: Modifiers, held: Modifiers) -> Modifiers {
    modifiers.alt |= held.alt;
    modifiers.ctrl |= held.ctrl;
    modifiers.shift |= held.shift;
    modifiers.win |= held.win;
    modifiers.insert |= held.insert;
    modifiers.caps_lock |= held.caps_lock;
    modifiers
}
