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
use okur_platform::event_trace::{
    is_focus, is_window_renamed, menu_event_from_trace, EventConverter, TraceEvent,
};
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

/// JAWS's name for itself, and Okur's name for itself in the same place.
pub const JAWS_NAME: &str = "JAWS";
pub const OKUR_NAME: &str = "Okur";

/// What every comparison of Okur with JAWS compares, on both sides: the speech
/// [`normalize`]d, with JAWS's name for itself read as Okur's ([`in_okurs_name`]).
pub fn comparable(speech: &str) -> String {
    in_okurs_name(&normalize(speech))
}

/// `text` with JAWS's name read as Okur's. Where JAWS names itself, Okur names itself ("JAWS
/// Cursor" is "Okur Cursor", "Route JAWS to PC" is "Route Okur to PC": AGENTS.md §1.5), so the
/// two names compare as equal. It is one rule for the whole corpus, not an accepted
/// divergence. Both sides go through it: a page's own "JAWS", which both read as written, stays
/// equal. Only the whole word in JAWS's capitals is read so ("JAWS's" too); "JAWSKey" and
/// "jaws" stay as they are.
pub fn in_okurs_name(text: &str) -> String {
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    for (at, name) in text.match_indices(JAWS_NAME) {
        let end = at + name.len();
        let before = text.get(..at).and_then(|t| t.chars().next_back());
        let after = text.get(end..).and_then(|t| t.chars().next());
        if word(before) || word(after) {
            continue;
        }
        out.push_str(text.get(copied..at).unwrap_or_default());
        out.push_str(OKUR_NAME);
        copied = end;
    }
    out.push_str(text.get(copied..).unwrap_or_default());
    out
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
    /// An accepted divergence covers the whole step ([`crate::divergences`]): it is not
    /// counted, on any layer.
    pub accepted: bool,
    /// Whether the step is compared on the history layer: false where the capture did not
    /// read JAWS's speech history (`"history": false`, Keyboard Help's steps), which only the
    /// spoken layer compares.
    pub history: bool,
    /// The spoken layer, for evidence recorded through the speech recorder voice: what our
    /// synthesizer is given, against what JAWS sent its synthesizer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoken: Option<SpokenParity>,
}

/// One step's spoken-layer comparison ([`okur_core::pronounce`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
        self.steps.iter().filter(|s| s.history && s.matched).count()
    }

    /// Steps an accepted divergence leaves out of the counts.
    pub fn accepted(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.history && s.accepted)
            .count()
    }

    /// Steps compared with JAWS's speech history: every step whose history was read but the
    /// accepted divergences.
    pub fn compared(&self) -> usize {
        self.steps.iter().filter(|s| s.history).count() - self.accepted()
    }

    /// Steps compared on the spoken layer: how many match, of how many.
    pub fn spoken(&self) -> (usize, usize) {
        let compared: Vec<&SpokenParity> = self
            .steps
            .iter()
            .filter(|s| !s.accepted)
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
pub(crate) fn jaws_spoken_variants(evidence: &Value) -> Option<BTreeMap<String, Vec<String>>> {
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
        let spoken = step_spoken(step);
        let entry = variants.entry(id.to_owned()).or_default();
        if !entry.contains(&spoken) {
            entry.push(spoken);
        }
    }
    Some(variants)
}

/// What a recorded step sent the synthesizer, the harness's own confirmations left out.
pub(crate) fn step_spoken(step: &Value) -> String {
    {
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
        records
            .iter()
            .filter(|s| !HARNESS_CONFIRMATIONS.contains(&s.trim()))
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ")
    }
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

/// A step whose capture did not read JAWS's speech history (`"history": false`): only its
/// spoken layer was recorded.
fn is_spoken_only(step: &Value) -> bool {
    step.pointer("/observation/history_read") == Some(&Value::Bool(false))
}

/// Distinct raw transcripts per step id, in first-seen order, from successful trials only.
pub(crate) fn jaws_variants(evidence: &Value) -> BTreeMap<String, Vec<String>> {
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
            .filter(|step| !is_spoken_only(step))
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
    let spoken_only: std::collections::BTreeSet<String> = evidence
        .pointer("/trials")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|t| t.get("step_results").and_then(Value::as_array))
        .flatten()
        .filter(|step| is_spoken_only(step))
        .filter_map(|step| {
            step.get("step_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();

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
    let actions = okur_core::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    // JAWS's settings when the evidence was recorded.
    engine.set_settings(replay_settings(&evidence));
    if desktop {
        engine.app_mode = okur_core::engine::AppMode::NativeControl;
    }
    // Say System Time reads a clock standing at each step's recorded start.
    let clock = replay_clock(&evidence)?;
    if let Some((clock, _)) = &clock {
        engine.set_clock(Box::new(clock.clone()));
    }
    // Okur's own dialogs: the engine shows them through a window host, and JAWS's dialog in
    // the trace stands for the window (`crate::dialog_mirror`).
    let ui = okur_core::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let mut mirror = crate::dialog_mirror::DialogMirror::default();
    let replay = StepEvents::load(corpus_dir, &scenario.id, &evidence)?;
    // One converter for the whole run, as live: it tracks the focused object.
    let mut converter = EventConverter::default();
    // The reset keys and the focus events recorded before step 1, in the order they came: a
    // page's load events come before its reset keys, which act on the virtual cursor (E, then
    // Enter into a field: without Auto Forms Mode only that Enter turns forms mode on); an
    // application answers its reset keys (Ctrl+Home in an editor reads the line it reaches).
    let press_reset = |engine: &mut ScreenReaderEngine| -> Result<(), String> {
        for chord in scenario.reset.iter().flat_map(|r| r.keys.iter().flatten()) {
            press(engine, &actions, chord, 0, Modifiers::none())?;
        }
        Ok(())
    };
    if desktop {
        press_reset(&mut engine)?;
    }
    // The focused text as the trace last showed it, and the fresh reads of it answered.
    let mut last_text: Option<okur_core::TextContext> = None;
    let mut text_reads = 0;
    for event in &replay.setup {
        for converted in converter.convert(event) {
            note_text(&mut last_text, &converted);
            let _ = engine.handle_accessibility_event(&converted);
        }
    }
    if !desktop {
        press_reset(&mut engine)?;
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
        // A key JAWS typed into its own dialog (`TypeKey("Tab")`) is the user's key in Okur's.
        let typed = (step.step_type == "jaws_function")
            .then(|| typed_key(&step.command))
            .flatten();
        // A key sent with UI Access (into JAWS's modal dialogs) is the user's key too.
        let (step_type, command) = match typed {
            Some(key) => ("key", key),
            None if step.step_type == "key_ui_access" => ("key", step.command.as_str()),
            None => (step.step_type.as_str(), step.command.as_str()),
        };
        let engine_speech = match step_type {
            "key" | "key_down" | "key_up" | "text" => {
                match step_type {
                    // Text typed into the application reaches it, not the screen reader's
                    // commands; with the reference machine's typing echo off, only the
                    // step's events are Okur's to read.
                    "text" => {}
                    // A key held across steps (Alt through several Tabs), and its release.
                    "key_down" | "key_up" => {
                        let down = step_type == "key_down";
                        let mut event = crate::keys::parse_chord(command)?;
                        hold(&mut held, event.key, down);
                        event.modifiers = with_held(event.modifiers, held);
                        event.is_down = down;
                        event.time_ms = start;
                        let _ = engine.handle_key_event(&event);
                        hand_back(&mut engine, &actions, start);
                    }
                    _ => {
                        for press_index in 0..step.repeat.unwrap_or(1).max(1) {
                            press(
                                &mut engine,
                                &actions,
                                command,
                                start + u64::from(press_index) * REPEAT_GAP_MS,
                                held,
                            )?;
                            // A key in Okur's dialog: what its window reports.
                            mirror.sync(&ui);
                            let pressed = crate::keys::parse_chord(command)?;
                            for report in mirror.key(&pressed) {
                                let report = okur_core::AccessibilityEvent::Ui(report);
                                if let Some(speech) = engine.handle_accessibility_event(&report) {
                                    if spoken.is_empty() {
                                        spoken.push(&speech, false);
                                    }
                                }
                            }
                            mirror.sync(&ui);
                        }
                        // A reload: the platform reads the page again once it changes, as
                        // Okur's page worker does live (the trace records events, not the
                        // page reads); the engine then takes it as a new load [obs, 538/s1].
                        if !desktop && matches!(command, "Ctrl+R" | "F5") {
                            let reloaded = okur_core::AccessibilityEvent::DocumentLoaded {
                                title: aom.name.clone().unwrap_or_default(),
                                root: aom.clone(),
                            };
                            if let Some(speech) = engine.handle_accessibility_event(&reloaded) {
                                if spoken.is_empty() {
                                    spoken.push(&speech, false);
                                }
                            }
                        }
                        // Chords pressed while the step's speech went on, at their times.
                        let mut at = start;
                        for key in step.during.iter().flatten() {
                            at += key.after_ms;
                            press(&mut engine, &actions, &key.command, at, held)?;
                        }
                    }
                }
                answer_text_reads(
                    &mut engine,
                    &actions,
                    &mut text_reads,
                    last_text.as_ref(),
                    &spoken,
                );
                // Replay the OS events recorded while JAWS handled this step (e.g. the focus
                // change that follows Tab), exactly as the live listener would deliver them.
                for event in replay.for_step(&step.id) {
                    for mut converted in converter.convert(&event) {
                        // Where JAWS's dialog differs from Okur's window, the focus is Okur's;
                        // JAWS's dialog going away after Okur's closed is not.
                        if let okur_core::AccessibilityEvent::Focus(focus) = &mut converted {
                            if mirror.is_closed_dialog(focus) {
                                continue;
                            }
                            mirror.as_okur_window(focus);
                        }
                        note_text(&mut last_text, &converted);
                        if let Some(speech) = engine.handle_accessibility_event(&converted) {
                            if spoken.is_empty() {
                                spoken.push(&speech, false);
                            }
                        }
                        mirror.sync(&ui);
                        let reports = match &converted {
                            okur_core::AccessibilityEvent::Focus(focus)
                            | okur_core::AccessibilityEvent::Change { control: focus, .. } => {
                                mirror.focus(focus)
                            }
                            okur_core::AccessibilityEvent::TextContext(context) => {
                                mirror.typed(&context.text)
                            }
                            _ => Vec::new(),
                        };
                        for report in reports {
                            let report = okur_core::AccessibilityEvent::Ui(report);
                            let _ = engine.handle_accessibility_event(&report);
                            mirror.sync(&ui);
                        }
                    }
                }
                // The clipboard as the recording left it (an application's Ctrl+C), for the
                // clipboard keys of later steps.
                if let Some(text) = replay.clipboard_after(&step.id) {
                    actions.set_clipboard(text);
                }
                // An edit key that changed nothing reports the field unchanged, as live.
                if let Some(speech) = engine.settle_pending_edit() {
                    if spoken.is_empty() {
                        spoken.push(&speech, false);
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
        let history = !jaws.is_empty();
        if !history && !spoken_only.contains(step.id.as_str()) {
            return Err(format!("no JAWS transcript recorded for step {}", step.id));
        }
        use crate::divergences::{compare, Comparison};
        let key = format!("{}/{}", scenario.id, step.id);
        // The tracer redacts e-mail addresses outside the example domains (privacy.rs); where
        // Okur read a redacted one from the trace, JAWS's transcript is redacted the same way
        // (news-article.html's address [239/s3]).
        let redacted = |okur: &str, jaws: Vec<String>| -> Vec<String> {
            if okur.contains(okur_platform::privacy::ADDRESS_MARK) {
                jaws.iter()
                    .map(|t| okur_platform::privacy::redact_addresses(t))
                    .collect()
            } else {
                jaws
            }
        };
        let jaws = redacted(&engine_speech, jaws);
        let comparison = compare(&key, &engine_speech, &jaws);
        let engine_spoken = spoken.take_spoken().join(" ");
        let spoken_parity = spoken_variants.as_ref().map(|all| {
            let jaws_variants = all.get(&step.id).cloned().unwrap_or_default();
            let jaws_variants = redacted(&engine_spoken, jaws_variants);
            SpokenParity {
                matched: compare(&key, &engine_spoken, &jaws_variants) == Comparison::Matched,
                engine: engine_spoken,
                jaws_variants,
            }
        });
        steps.push(StepParity {
            key,
            command: step.command.clone(),
            engine: engine_speech,
            jaws_variants: jaws,
            matched: history && comparison == Comparison::Matched,
            accepted: history && comparison == Comparison::Accepted,
            history,
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
    run_matching(corpus_dir, api, &[])
}

/// [`run_with`] for the scenarios whose ids start with one of `only` (every scenario when it
/// is empty): `okur parity --only 228` replays one scenario, with the engine's debug log.
pub fn run_matching(corpus_dir: &Path, api: Api, only: &[String]) -> ParityReport {
    let mut report = ParityReport::default();
    let mut files: Vec<PathBuf> = fs::read_dir(corpus_dir.join("scenarios"))
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.retain(|p| p.extension().is_some_and(|e| e == "json"));
    files.retain(|p| {
        let id = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        only.is_empty() || only.iter().any(|prefix| id.starts_with(prefix.as_str()))
    });
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

/// Runs one scenario (`corpus_dir/scenarios/<id>.json`) against its evidence: for tests of
/// single steps, the known gaps among them.
pub fn run_one(corpus_dir: &Path, id: &str) -> Result<Vec<StepParity>, String> {
    let scenario =
        Scenario::load_from_file(corpus_dir.join("scenarios").join(format!("{id}.json")))
            .map_err(|e| format!("{id}: {e}"))?;
    run_scenario(corpus_dir, &scenario, snapshots::PARITY_API)
}

#[derive(Debug, Serialize)]
struct Baseline<'a> {
    notice: &'static str,
    matched_steps: usize,
    total_steps: usize,
    accepted_divergences: usize,
    errors: &'a [String],
    steps: BTreeMap<&'a str, &'static str>,
}

/// Renders the baseline exactly as stored on disk.
pub fn render_baseline(report: &ParityReport) -> Result<String, String> {
    let baseline = Baseline {
        notice:
            "GENERATED FILE - do not edit. Regenerate: cargo run --bin okur -- parity --write-baseline",
        matched_steps: report.matched(),
        total_steps: report.compared(),
        accepted_divergences: report.accepted(),
        errors: &report.errors,
        steps: report
            .steps
            .iter()
            .map(|s| {
                let status = match (s.matched, s.accepted) {
                    _ if !s.history => "spoken only",
                    (true, _) => "match",
                    (false, true) => "accepted divergence",
                    (false, false) => "mismatch",
                };
                (s.key.as_str(), status)
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
            "{BASELINE_FILE} is stale (parity changed). Review `cargo run --bin okur -- parity`, \
             then regenerate with: cargo run --bin okur -- parity --write-baseline"
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
    /// The punctuation level the engine's voice settings give the synthesizer.
    punctuation: std::sync::Arc<std::sync::Mutex<okur_core::settings::Punctuation>>,
    /// Speech muted (Insert+Space, S): nothing is kept, as JAWS keeps nothing [obs, 517/s7].
    muted: std::sync::Arc<std::sync::atomic::AtomicBool>,
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
    fn silent(&self) -> bool {
        self.muted.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn is_empty(&self) -> bool {
        self.history
            .lock()
            .map(|log| log.is_empty())
            .unwrap_or(true)
    }

    /// `symbols`: every symbol named (a selection, a word read on its own).
    fn push(&self, text: &str, symbols: bool) {
        if self.silent() {
            return;
        }
        let level = self
            .punctuation
            .lock()
            .map(|level| *level)
            .unwrap_or_default();
        append(&self.history, text.to_owned());
        let spoken = if symbols {
            okur_core::pronounce::spoken_symbols(text)
        } else {
            okur_core::pronounce::spoken(text, level)
        };
        append(&self.spoken, spoken);
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
        if self.silent() {
            return;
        }
        let pitch = okur_core::pronounce::character_pitch(text);
        append(
            &self.spoken,
            okur_core::pronounce::pitched(&okur_core::pronounce::character(text), pitch),
        );
    }

    fn speak(&mut self, text: &str, _interrupt: bool) {
        self.push(text, false);
    }

    fn speak_symbols(&mut self, text: &str, _interrupt: bool) {
        self.push(text, true);
    }

    fn configure(&mut self, voice: &okur_core::settings::VoiceSettings) {
        if let Ok(mut level) = self.punctuation.lock() {
            *level = voice.punctuation;
        }
    }

    fn speak_character(&mut self, text: &str, _interrupt: bool) {
        if self.silent() {
            return;
        }
        append(&self.history, text.to_owned());
        let pitch = okur_core::pronounce::character_pitch(text);
        append(
            &self.spoken,
            okur_core::pronounce::pitched(&okur_core::pronounce::character(text), pitch),
        );
    }

    fn speak_spelled(&mut self, text: &str, _interrupt: bool) {
        if self.silent() {
            return;
        }
        append(&self.history, okur_core::pronounce::spelled(text));
        for (part, pitch) in okur_core::pronounce::spelled_parts(text) {
            append(&self.spoken, okur_core::pronounce::pitched(&part, pitch));
        }
    }

    fn speak_spelled_selection(&mut self, text: &str, _interrupt: bool) {
        if self.silent() {
            return;
        }
        append(&self.history, text.to_owned());
        for (part, pitch) in okur_core::pronounce::spelled_parts(text) {
            append(&self.spoken, okur_core::pronounce::pitched(&part, pitch));
        }
    }

    fn speak_phonetic(&mut self, text: &str, _interrupt: bool) {
        if self.silent() {
            return;
        }
        append(&self.history, text.to_owned());
        let pitch = okur_core::pronounce::character_pitch(text);
        append(
            &self.spoken,
            okur_core::pronounce::pitched(&okur_core::pronounce::phonetic(text), pitch),
        );
    }

    fn stop(&mut self) {}

    fn set_muted(&mut self, muted: bool) {
        self.muted.store(muted, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Keeps the focused text a traced event showed.
fn note_text(last: &mut Option<okur_core::TextContext>, event: &okur_core::AccessibilityEvent) {
    match event {
        okur_core::AccessibilityEvent::TextContext(context) => *last = Some(context.clone()),
        okur_core::AccessibilityEvent::Focus(focus) if focus.text.is_some() => {
            last.clone_from(&focus.text)
        }
        _ => {}
    }
}

/// A fresh read of the focused text (a console's buffer for the Virtual Viewer) is answered
/// at once live, with the text as it is then. Replay answers it with the text the trace last
/// showed, not with whatever event the trace happens to hold next.
fn answer_text_reads(
    engine: &mut ScreenReaderEngine,
    actions: &okur_core::actions::RecordingActions,
    answered: &mut usize,
    last: Option<&okur_core::TextContext>,
    spoken: &RecordingSpeech,
) {
    let asked = actions
        .requests()
        .iter()
        .filter(|(action, _)| action == "read text")
        .count();
    while *answered < asked {
        *answered += 1;
        let Some(context) = last else {
            continue;
        };
        let event = okur_core::AccessibilityEvent::TextContext(context.clone());
        if let Some(speech) = engine.handle_accessibility_event(&event) {
            if spoken.is_empty() {
                spoken.push(&speech, false);
            }
        }
    }
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
    use okur_core::settings::{
        KeyboardLayout, KeyboardSettings, OutputType, Settings, TypingEcho, Verbosity,
    };
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
        // A missing option is JAWS's default: on, for both.
        let jcf = |section: &str, key: &str| {
            evidence
                .pointer(&format!(
                    "/provenance/jaws_settings/DEFAULT.JCF/{section}/{key}"
                ))
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        settings.auto_forms_mode = jcf("FormsMode", "AutoFormsMode").is_none_or(|v| v != "0");
        // The user verbosity level, `Verbosity` in Options: 0 Beginner (JAWS's default), 1
        // Intermediate, 2 Advanced (548-555 record each).
        settings.verbosity.level = match options.get("Verbosity").and_then(Value::as_str) {
            Some("1") => okur_core::settings::VerbosityLevel::Intermediate,
            Some("2") => okur_core::settings::VerbosityLevel::Advanced,
            _ => okur_core::settings::VerbosityLevel::Beginner,
        };
        // The verbosity output types JAWS's JCF names: one mode per level and the type's
        // name ("0|0|0|Access Key"). A type the file leaves out has JAWS's shipped modes.
        for output in OutputType::ALL {
            let Some(key) = output.jcf_key() else {
                continue;
            };
            let modes = jcf("OutputModes", key)
                .and_then(|v| Verbosity::modes_from_jcf(&v))
                .unwrap_or(output.default_modes());
            settings.verbosity.set_modes(output, modes);
        }
    }
    settings
}

/// Presses and releases `chord` at `time_ms` (0: unknown) through the engine's live
/// key-handling entry point.
fn press(
    engine: &mut ScreenReaderEngine,
    actions: &okur_core::actions::RecordingActions,
    chord: &str,
    time_ms: u64,
    held: Modifiers,
) -> Result<(), String> {
    let mut event = crate::keys::parse_chord(chord)?;
    event.modifiers = with_held(event.modifiers, held);
    event.time_ms = time_ms;
    let _ = engine.handle_key_event(&event);
    hand_back(engine, actions, time_ms);
    event.is_down = false;
    let _ = engine.handle_key_event(&event);
    Ok(())
}

/// The key a `TypeKey("…")` JAWS function call types, as a chord name.
fn typed_key(call: &str) -> Option<&str> {
    call.trim()
        .strip_prefix("TypeKey(\"")?
        .strip_suffix("\")")
        .filter(|key| !key.is_empty())
}

/// Keys the engine sent the application come back through the keyboard hook live, as plain
/// keys carrying only their own modifiers (a laptop reading key's Down Arrow, read as Down
/// Arrow is); replay hands them back the same way. A few rounds at most, in case a key sent
/// sends another.
fn hand_back(
    engine: &mut ScreenReaderEngine,
    actions: &okur_core::actions::RecordingActions,
    time_ms: u64,
) {
    use okur_core::input::Key;
    for _ in 0..4 {
        let chords = actions.take_pressed();
        if chords.is_empty() {
            return;
        }
        for chord in chords {
            let mut modifiers = Modifiers::none();
            let mut key = None;
            for k in chord {
                match k {
                    Key::Control => modifiers.ctrl = true,
                    Key::Shift => modifiers.shift = true,
                    Key::Alt => modifiers.alt = true,
                    other => key = Some(other),
                }
            }
            let Some(key) = key else { continue };
            let mut event = okur_core::input::KeyEvent::new(key, modifiers, true);
            event.time_ms = time_ms;
            let _ = engine.handle_key_event(&event);
            event.is_down = false;
            let _ = engine.handle_key_event(&event);
        }
    }
}

/// How early the tracer may stamp an event: it dates each from Windows's event time, a tick
/// count that advances every 10 to 16 ms, so an event that follows a key within that time
/// can be stamped before the key. 306/s2 (2026-10-19): the No button's focus 7 ms before the
/// Tab that moved it.
pub const TICK_MS: u128 = 16;

/// Events the engine consumes: focus changes (with a UIA chain or an MSAA object) and menu
/// mode transitions.
fn replayable(event: &TraceEvent) -> bool {
    // JAWS's own main window (hidden, class JFWUI2) takes the focus for a moment as JAWS
    // opens one of its dialogs; the dialog is its child. Okur, never running beside JAWS
    // live, would see neither, but its own dialog the same as JAWS's; the dialog's events
    // stay [obs, 224/s1, round 52].
    // The main window can also report the dialog's first focus, its chain in the dialog: that
    // event is the dialog's (249/s1, round 54: "focus" from JFWUI2 on "Full name Edit").
    const JAWS_MAIN_WINDOW: &str = "JFWUI2";
    const WIN32_DIALOG: &str = "#32770";
    let in_dialog = event
        .focus_chain
        .iter()
        .any(|n| n.class_name.as_deref() == Some(WIN32_DIALOG));
    // JAWS's Results Viewer (its Virtual Viewer, a process of its own) stands for Okur's own
    // viewer, which Okur speaks itself: "Speech History", not JAWS's window [obs, 517/s3].
    if event.process.as_deref() == Some("ResultsViewer.exe") {
        return false;
    }
    // A focus inside the main window itself, not in one of its dialogs (its "JAWS Version"
    // text, 242/s1), is JAWS's alone.
    let in_main_window = event
        .focus_chain
        .iter()
        .any(|n| n.class_name.as_deref() == Some(JAWS_MAIN_WINDOW));
    if (event.window_class.as_deref() == Some(JAWS_MAIN_WINDOW) || in_main_window) && !in_dialog {
        return false;
    }
    menu_event_from_trace(event).is_some()
        || (matches!(
            event.event.as_str(),
            "focus"
                | "uia_focus"
                | "uia_selected"
                | "foreground"
                | "dialog_shown"
                | "caret"
                | "text_selection_change"
                | "live_region_change"
                | "state_change"
                | "value_change"
                | "selection"
                | "name_change"
        ) && (!event.focus_chain.is_empty() || event.object.is_some()))
}

/// Replayable events recorded during each step of the first successful traced trial
/// (`corpus/traces/<trace_file>`, recorded with `run-scenario.ps1 -TraceDir`).
#[derive(Default)]
struct StepEvents {
    by_step: std::collections::HashMap<String, Vec<okur_platform::event_trace::TraceEvent>>,
    /// Focus events before the first step (launch and reset), and the window in front with
    /// its renaming (a console titled by what runs in it), and the caret's moves, replayed
    /// silently so the engine knows the window, focus and caret JAWS had when step 1 began.
    setup: Vec<okur_platform::event_trace::TraceEvent>,
    /// When each step began (unix ms), for the replay clock and key times.
    starts: std::collections::HashMap<String, u64>,
    /// What the clipboard held after each step that captured it (`capture_clipboard`): the
    /// application's Ctrl+C, which the clipboard keys then read.
    clipboards: std::collections::HashMap<String, String>,
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
        let mut clipboards = std::collections::HashMap::new();
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
            .unwrap_or(0)
            .saturating_sub(TICK_MS);
        // The caret moves too, so a field's text is known with its caret where the reset
        // keys left it (401: Ctrl+Home and twelve Downs before Say Character); and the
        // mouse, where the JAWS cursor starts (516/s2).
        let setup = events
            .iter()
            .filter(|e| {
                e.t_ms < first_start
                    && (e.mouse.is_some()
                        || ((is_focus(e)
                            || e.event == "foreground"
                            || is_window_renamed(e)
                            || matches!(e.event.as_str(), "caret" | "text_selection_change"))
                            && replayable(e)))
            })
            .cloned()
            .collect();
        let mut previous_end: Option<u128> = None;
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
            // A step's events may be stamped up to one tick before its key, never before the
            // step before it ended.
            let from = u128::from(start).saturating_sub(TICK_MS);
            let from = previous_end.map_or(from, |end| from.max(end + 1));
            let window: Vec<_> = events
                .iter()
                .filter(|e| replayable(e) && e.t_ms >= from && e.t_ms <= u128::from(end))
                .cloned()
                .collect();
            previous_end = Some(u128::from(end));
            by_step.insert(id.to_owned(), window);
            starts.insert(id.to_owned(), start);
            if let Some(text) = step
                .pointer("/observation/clipboard")
                .and_then(Value::as_str)
            {
                clipboards.insert(id.to_owned(), text.to_owned());
            }
        }
        Ok(Self {
            by_step,
            setup,
            starts,
            clipboards,
        })
    }

    fn for_step(&self, step_id: &str) -> Vec<okur_platform::event_trace::TraceEvent> {
        self.by_step.get(step_id).cloned().unwrap_or_default()
    }

    /// What the clipboard held after step `step_id`, when the step captured it.
    fn clipboard_after(&self, step_id: &str) -> Option<&str> {
        self.clipboards.get(step_id).map(String::as_str)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A step the capture did not read JAWS's history for (`"history": false`, Keyboard Help)
    /// has no history transcript; the other steps keep theirs.
    #[test]
    fn a_step_without_its_history_has_no_history_transcript() {
        let evidence = serde_json::json!({
            "trials": [{
                "trial_success": true,
                "step_results": [
                    { "step_id": "s1", "observation": { "speech_raw": "Title is" } },
                    {
                        "step_id": "s2",
                        "observation": { "speech_raw": "", "history_read": false }
                    }
                ]
            }]
        });
        let variants = jaws_variants(&evidence);
        assert_eq!(
            variants.get("s1").map(Vec::as_slice),
            Some(["Title is".to_owned()].as_slice())
        );
        assert!(!variants.contains_key("s2"));
    }
}
