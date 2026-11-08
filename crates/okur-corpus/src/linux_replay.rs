//! A Linux live run replayed off the lab (docs/plans/2026-11-08-structure-for-three-platforms.md,
//! step 1): the records the run's driver traced (`corpus/live/linux/<id>.trace.jsonl`, written by
//! `okur_linux::events::AtspiEventSource::with_trace`), each what was read for one AT-SPI event,
//! through okur-linux-model's converter and the engine, with the keys the runner pressed at
//! their times. Where the replay says what the run said, the converter and the engine alone
//! made the run's speech, and a change to either is checked against the run without the lab; a
//! step it says otherwise is timing the replay does not model (the host's threads), or a change
//! since the run.

use crate::live::{self, LiveStep, LIVE_DIR};
use crate::parity::{normalize, press, RecordingSpeech};
use okur_core::{Modifiers, ScreenReaderEngine, VirtualBuffer, VirtualCursor};
use okur_linux_model::convert::{AtspiRecord, Converter};
use okur_model::event_model::EventModel;
use serde_json::Value;
use std::path::Path;

/// The platform whose runs are replayed.
const PLATFORM: &str = "linux";

/// A quiet moment, after which the host gives the engine an idle tick (its event loop's
/// `idle`): speech held until no event supersedes it is said then.
const IDLE_MS: u64 = 150;

/// One step of a live run, replayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayStep {
    /// `<scenario_id>/<step_id>`.
    pub key: String,
    /// What Okur said in the run.
    pub live: String,
    /// What the replay said.
    pub replayed: String,
}

impl ReplayStep {
    /// Whether the replay said what the run said, whitespace aside.
    pub fn faithful(&self) -> bool {
        normalize(&self.live) == normalize(&self.replayed)
    }
}

/// The live runs that kept a trace, replayed.
#[derive(Debug, Default)]
pub struct ReplayReport {
    pub runs: usize,
    pub steps: Vec<ReplayStep>,
    /// The replayed speech against JAWS's evidence, as the live runs are compared.
    pub against_jaws: Vec<LiveStep>,
    /// Runs recorded before the driver traced, or whose trace was not kept.
    pub without_trace: Vec<String>,
    pub errors: Vec<String>,
}

impl ReplayReport {
    pub fn faithful(&self) -> usize {
        self.steps.iter().filter(|s| s.faithful()).count()
    }

    /// Replayed steps matching JAWS, and those compared (accepted divergences aside).
    pub fn jaws_matched(&self) -> (usize, usize) {
        let matched = self.against_jaws.iter().filter(|s| s.matched).count();
        let accepted = self.against_jaws.iter().filter(|s| s.accepted).count();
        (matched, self.against_jaws.len() - accepted)
    }
}

/// A trace's records, one JSON line each.
pub fn read_trace(text: &str) -> Result<Vec<AtspiRecord>, String> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line).map_err(|e| format!("trace line {}: {e}", index + 1))
        })
        .collect()
}

/// The engine as the Linux lab's okur_linux runs it: the default settings, the page verbosity
/// the run was given, no page in front, the records converted as its driver converts them.
struct Replayer {
    engine: ScreenReaderEngine,
    speech: RecordingSpeech,
    actions: okur_core::actions::RecordingActions,
    converter: Converter,
    /// The time of the last event or key, for the idle ticks between them.
    last_ms: Option<u64>,
}

impl Replayer {
    fn new(web_verbosity: Option<&str>) -> Self {
        let mut settings =
            okur_core::settings_layers::SettingsLayers::default().resolve(None, None);
        if let Some(level) = web_verbosity {
            settings.web_verbosity = okur_core::settings::WebVerbosity::from_jcf(level);
        }
        let speech = RecordingSpeech::default();
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&okur_core::AomNode::new(
            0,
            okur_core::Role::Document,
        )));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
        engine.set_settings(settings);
        // The host starts with no page: native navigation until one comes.
        engine.app_mode = okur_core::engine::AppMode::NativeControl;
        let actions = okur_core::actions::RecordingActions::default();
        engine.set_actions(Box::new(actions.clone()));
        Self {
            engine,
            speech,
            actions,
            converter: Converter::default(),
            last_ms: None,
        }
    }

    /// Time going on to `now`: the idle tick a quiet moment before it gave, and what waits on
    /// time alone and fell due.
    fn advance(&mut self, now: u64) {
        if let Some(last) = self.last_ms {
            if now > last.saturating_add(IDLE_MS) {
                let _ = self.engine.on_idle_at(Some(last.saturating_add(IDLE_MS)));
            }
        }
        let _ = self.engine.on_time(now);
        self.last_ms = Some(self.last_ms.map_or(now, |last| last.max(now)));
    }

    fn feed(&mut self, record: &AtspiRecord) {
        self.advance(record.at_ms);
        // As the driver converted it: GTK's twin focus passed over, then what was read.
        for event in EventModel::convert(&mut self.converter, record) {
            let _ = self.engine.handle_accessibility_event(&event);
            // A big page changed: the host builds it off the engine and installs it.
            if let Some((generation, page)) = self.engine.take_page_build() {
                let buffer = VirtualBuffer::from_aom_owned(okur_core::AomNode::clone(&page));
                let _ = self.engine.install_page_build(generation, buffer);
            }
        }
    }

    fn key(&mut self, chord: &str, at: u64) -> Result<(), String> {
        self.advance(at);
        press(
            &mut self.engine,
            &self.actions,
            chord,
            at,
            Modifiers::none(),
        )
    }
}

fn number(value: &Value, pointer: &str) -> Option<u64> {
    value.pointer(pointer).and_then(Value::as_u64)
}

/// Replays the first trial of the live `run` from its trace's `records`.
pub fn replay_run(run: &Value, records: &[AtspiRecord]) -> Result<Vec<ReplayStep>, String> {
    let scenario = run
        .get("scenario_id")
        .and_then(Value::as_str)
        .ok_or("the run has no scenario_id")?;
    let trial = run.pointer("/trials/0").ok_or("the run has no trial")?;
    let steps = trial
        .get("step_results")
        .and_then(Value::as_array)
        .ok_or("the trial has no steps")?;
    let level = run
        .pointer("/provenance/web_verbosity")
        .and_then(Value::as_str);
    let mut replayer = Replayer::new(level);
    let mut records = records.iter().peekable();
    // The setup: the records before the first key, the reset's keys among them at their times;
    // what it said is not a step's.
    let first = steps
        .iter()
        .find_map(|s| number(s, "/t_start_ms"))
        .unwrap_or(u64::MAX);
    let mut reset: Vec<(String, u64)> = trial
        .get("reset_keys")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|k| Some((k.get("key")?.as_str()?.to_owned(), number(k, "/t_ms")?)))
        .collect();
    reset.sort_by_key(|(_, at)| *at);
    for (chord, at) in reset {
        while let Some(record) = records.next_if(|r| r.at_ms < at) {
            replayer.feed(record);
        }
        replayer.key(&chord, at)?;
    }
    while let Some(record) = records.next_if(|r| r.at_ms < first) {
        replayer.feed(record);
    }
    replayer.speech.take();
    let mut replayed = Vec::new();
    for step in steps {
        let id = step
            .get("step_id")
            .and_then(Value::as_str)
            .ok_or("a step has no step_id")?;
        let live = step
            .pointer("/observation/speech_raw")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let key = format!("{scenario}/{id}");
        // A step the runner skipped pressed nothing.
        let (Some(start), Some(end)) = (number(step, "/t_start_ms"), number(step, "/t_end_ms"))
        else {
            replayed.push(ReplayStep {
                key,
                live,
                replayed: String::new(),
            });
            continue;
        };
        let keys = step
            .get("keys")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{key}: the run did not keep its keys"))?;
        // The events after the step before, and the step's key: said in this step, as the
        // runner's speech log has it.
        while let Some(record) = records.next_if(|r| r.at_ms < start) {
            replayer.feed(record);
        }
        for (index, chord) in keys.iter().enumerate() {
            let chord = chord
                .as_str()
                .ok_or_else(|| format!("{key}: a key is not a chord"))?;
            replayer.key(chord, start.saturating_add(index as u64))?;
        }
        while let Some(record) = records.next_if(|r| r.at_ms < end) {
            replayer.feed(record);
        }
        replayer.advance(end);
        let _ = replayer.engine.on_idle_at(Some(end));
        replayed.push(ReplayStep {
            key,
            live,
            replayed: replayer.speech.take().join("\n"),
        });
    }
    Ok(replayed)
}

/// The run with the replay's speech in its steps, for comparing with JAWS's evidence as the run
/// itself is compared.
fn with_replayed(run: &Value, steps: &[ReplayStep]) -> Value {
    let mut run = run.clone();
    if let Some(results) = run
        .pointer_mut("/trials/0/step_results")
        .and_then(Value::as_array_mut)
    {
        for (result, step) in results.iter_mut().zip(steps) {
            if let Some(speech) = result.pointer_mut("/observation/speech_raw") {
                *speech = Value::String(step.replayed.clone());
            }
        }
    }
    run
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("{path:?}: {e}"))
}

/// Replays every Linux live run in the corpus that kept its trace.
pub fn run(corpus_dir: &Path) -> ReplayReport {
    let mut report = ReplayReport::default();
    let dir = corpus_dir.join(LIVE_DIR).join(PLATFORM);
    let mut paths: Vec<_> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect(),
        Err(e) => {
            report.errors.push(format!("{dir:?}: {e}"));
            return report;
        }
    };
    paths.sort();
    for path in paths {
        let trace = path.with_extension("trace.jsonl");
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !trace.exists() {
            report.without_trace.push(name);
            continue;
        }
        let replayed = read_json(&path).and_then(|run| {
            let text = std::fs::read_to_string(&trace).map_err(|e| format!("{trace:?}: {e}"))?;
            let records = read_trace(&text).map_err(|e| format!("{name}: {e}"))?;
            let steps = replay_run(&run, &records).map_err(|e| format!("{name}: {e}"))?;
            let scenario = run
                .get("scenario_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let evidence = read_json(
                &corpus_dir
                    .join("evidence")
                    .join(format!("{scenario}-evidence.json")),
            )?;
            let against = live::compare(&with_replayed(&run, &steps), &evidence)
                .map_err(|e| format!("{name}: {e}"))?;
            Ok((steps, against))
        });
        match replayed {
            Ok((steps, against)) => {
                report.runs += 1;
                report.steps.extend(steps);
                report.against_jaws.extend(against);
            }
            Err(e) => report.errors.push(e),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use xa11y_core::{ElementData, EventKind, Role};

    fn focus(at_ms: u64, role: Role, name: &str) -> AtspiRecord {
        let mut element = ElementData::for_role(role);
        element.name = Some(name.to_owned());
        element.states.focused = true;
        AtspiRecord::bare(
            at_ms,
            EventKind::FocusChanged,
            "desktop-forms.py".to_owned(),
            Some(element),
        )
    }

    fn run(steps: Value) -> Value {
        json!({
            "scenario_id": "301-desktop-tab-order",
            "provenance": {"reader": "okur", "platform": "linux", "web_verbosity": null},
            "trials": [{"trial_index": 1, "trial_success": true, "step_results": steps}]
        })
    }

    /// Each step says what the events traced while it ran made the engine say; the setup's
    /// speech is no step's.
    #[test]
    fn a_runs_records_are_said_in_their_steps() {
        let steps = json!([
            {"step_id": "s1", "t_start_ms": 1000, "t_end_ms": 2000, "keys": ["Tab"],
             "action": {"type": "key", "command": "Tab"},
             "observation": {"speech_raw": "OK button"}},
            {"step_id": "s2", "t_start_ms": 2000, "t_end_ms": 3000, "keys": ["Tab"],
             "action": {"type": "key", "command": "Tab"},
             "observation": {"speech_raw": "Cancel button"}},
        ]);
        let records = [
            focus(500, Role::Button, "Before"),
            focus(1100, Role::Button, "OK"),
            focus(2100, Role::Button, "Cancel"),
        ];
        let replayed = replay_run(&run(steps), &records).unwrap_or_default();
        assert_eq!(replayed.len(), 2);
        let said: Vec<&str> = replayed.iter().map(|s| s.replayed.as_str()).collect();
        assert!(matches!(said.as_slice(), [first, second]
            if first.contains("OK") && !first.contains("Before") && second.contains("Cancel")));
    }

    /// A run recorded before the runner kept its keys cannot be replayed: said, not guessed.
    #[test]
    fn a_run_without_its_keys_is_not_replayed() {
        let steps = json!([
            {"step_id": "s1", "t_start_ms": 1000, "t_end_ms": 2000,
             "action": {"type": "key", "command": "Tab"},
             "observation": {"speech_raw": ""}},
        ]);
        assert!(replay_run(&run(steps), &[]).is_err());
    }

    #[test]
    fn a_trace_is_read_a_line_a_record() {
        let record = focus(5, Role::Button, "OK");
        let line = serde_json::to_string(&record).unwrap_or_default();
        let read = read_trace(&format!("{line}\n\n{line}\n")).unwrap_or_default();
        assert_eq!(read.len(), 2);
        assert!(read_trace("{not json").is_err());
    }
}
