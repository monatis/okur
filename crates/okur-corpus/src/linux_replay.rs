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

/// The replay's baseline, beside the live runs' (`corpus/live/linux-replay-baseline.json`): the
/// replayed steps that match JAWS, which may only grow. A change to the converter or the engine
/// that costs Linux a step fails the corpus test without the lab.
pub const BASELINE: &str = "linux-replay";

/// A quiet moment, after which the host gives the engine an idle tick (its event loop's
/// `idle`): speech held until no event supersedes it is said then.
const IDLE_MS: u64 = 150;

/// How the replay differs from the run, to try a change of the driver on the runs first.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReplayOptions<'a> {
    /// Only the runs whose names start with one of these, where any are given.
    pub only: &'a [String],
    /// Every run replayed with the settings JAWS recorded its scenario with, whatever the run
    /// gave Okur: what a run with them would say, before the lab runs it.
    pub recorded_settings: bool,
    /// Each page the driver read for a focus given to the engine after that focus, as a
    /// page-reader thread off the event thread would give it (the code review of 2026-11-08,
    /// finding 6): whether the engine still says what it said with the page first.
    pub pages_after_focus: bool,
}

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
    /// The replayed speech against JAWS's evidence, as the live runs' report is kept, for its
    /// baseline (`crate::live::check_baseline` with [`BASELINE`]).
    pub fn against_jaws_report(&self) -> live::LiveReport {
        live::LiveReport {
            runs: self.runs,
            steps: self.against_jaws.clone(),
            errors: self.errors.clone(),
        }
    }

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

/// A trace's records, one JSON line each, in the order the engine was given them: a record
/// stamped earlier than the one before it is an error, which a replay placing keys among them
/// by their times would hide (the code review of 2026-11-09, finding 6). A trace from before the
/// stamps is taken in its order.
pub fn read_trace(text: &str) -> Result<Vec<AtspiRecord>, String> {
    let mut records: Vec<AtspiRecord> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line).map_err(|e| format!("trace line {}: {e}", index + 1))
        })
        .collect::<Result<_, _>>()?;
    // A trace before the stamps is in the order the engine took its records, their events'
    // times not: a page record has the time it was asked for, a caret read the time it was
    // read. Each is given to the engine no earlier than the one before it.
    if records.iter().all(|r| r.delivered_ms.is_none()) {
        let mut last = 0;
        for record in &mut records {
            last = last.max(record.at_ms);
            record.delivered_ms = Some(last);
        }
    }
    if let Some(at) = records
        .windows(2)
        .position(|pair| matches!(pair, [a, b] if b.handed_ms() < a.handed_ms()))
    {
        return Err(format!(
            "trace record {} was given to the engine before the one before it",
            at + 2
        ));
    }
    Ok(records)
}

/// What Okur's Linux driver does for the engine (okur-linux's `LinuxActions`): it focuses and
/// activates a page's element, recorded as a test's actions are, and reads a text field at its
/// caret where the run's driver did (its trace holds the reads, given to the engine at their
/// times); it takes nothing else. A replay given more than the driver did says what no live run
/// could.
struct LinuxDriverActions {
    recorded: okur_core::actions::RecordingActions,
    /// The run's driver read fields at their carets: its trace has the reads.
    reads_carets: bool,
}

impl okur_core::actions::ApplicationActions for LinuxDriverActions {
    fn focus(&self, node: &okur_core::AomNode) -> bool {
        self.recorded.focus(node)
    }

    fn activate(&self, node: &okur_core::AomNode) -> bool {
        self.recorded.activate(node)
    }

    fn read_at_caret(
        &self,
        _before: Option<&okur_core::event::TextContext>,
        _in_page: bool,
    ) -> bool {
        self.reads_carets
    }

    /// A field's caret put, as the driver puts it where it reads carets.
    fn move_caret(&self, _context: &okur_core::event::TextContext, _offset: usize) -> bool {
        self.reads_carets
    }
}

/// The engine as the Linux lab's `okur run` runs it: the default settings, the page verbosity
/// the run was given, no page in front, the records converted as its driver converts them.
struct Replayer<'a> {
    options: ReplayOptions<'a>,
    engine: ScreenReaderEngine,
    speech: RecordingSpeech,
    actions: okur_core::actions::RecordingActions,
    converter: Converter,
    /// The time of the last event or key, for the idle ticks between them.
    last_ms: Option<u64>,
}

impl<'a> Replayer<'a> {
    fn new(
        settings: okur_core::settings::Settings,
        options: ReplayOptions<'a>,
        reads_carets: bool,
    ) -> Self {
        // Marks given back as the live run's speech thread gives them, each once handed over:
        // the silent synthesizer reads a Say All as fast as the event loop takes them.
        let speech = RecordingSpeech::reporting_marks();
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&okur_core::AomNode::new(
            0,
            okur_core::Role::Document,
        )));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
        engine.set_settings(settings);
        // Okur's own dialogs opened as live, for the trace's okur-ui records to act on them (a
        // dialog's choices, its buttons, its close), as parity's replay has them.
        engine.set_ui(Box::new(okur_core::ui::RecordingUiHost::default()));
        // The host starts with no page: native navigation until one comes.
        engine.app_mode = okur_core::engine::AppMode::NativeControl;
        let actions = okur_core::actions::RecordingActions::default();
        engine.set_actions(Box::new(LinuxDriverActions {
            recorded: actions.clone(),
            reads_carets,
        }));
        Self {
            options,
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

    /// The Say All parts handed over, reached: the engine hands over the next, as live.
    fn reach_marks(&mut self) {
        for _ in 0..10_000 {
            let marks = self.speech.take_marks();
            if marks.is_empty() {
                return;
            }
            for mark in marks {
                self.engine.speech_reached(mark);
            }
        }
    }

    fn feed(&mut self, record: &AtspiRecord) {
        self.advance(record.handed_ms());
        // As the driver converted it: GTK's twin focus passed over, then what was read.
        let mut events = EventModel::convert(&mut self.converter, record);
        if self.options.pages_after_focus {
            // The order kept otherwise: the pages, in theirs, after the rest.
            let (pages, mut rest): (Vec<_>, Vec<_>) = events
                .into_iter()
                .partition(|e| matches!(e, okur_core::AccessibilityEvent::DocumentLoaded { .. }));
            rest.extend(pages);
            events = rest;
        }
        for event in events {
            let said = self.engine.handle_accessibility_event(&event);
            // What each record became and what the engine said to it, for a step whose speech
            // is not where it is expected (RUST_LOG=okur_corpus=debug with --only).
            tracing::debug!(
                at_ms = record.handed_ms(),
                kind = ?record.kind,
                event = %format!("{event:?}").chars().take(160).collect::<String>(),
                said = ?said,
                "replayed record"
            );
            // A big page changed: the host builds it off the engine and installs it.
            if let Some((generation, page)) = self.engine.take_page_build() {
                let buffer = VirtualBuffer::from_aom_owned(okur_core::AomNode::clone(&page));
                let _ = self.engine.install_page_build(generation, buffer);
            }
            self.reach_marks();
        }
    }

    fn key(&mut self, chord: &str, at: u64) -> Result<(), String> {
        self.advance(at);
        tracing::debug!(at_ms = at, chord, "replayed key");
        press(
            &mut self.engine,
            &self.actions,
            chord,
            at,
            Modifiers::none(),
        )?;
        self.reach_marks();
        Ok(())
    }
}

fn number(value: &Value, pointer: &str) -> Option<u64> {
    value.pointer(pointer).and_then(Value::as_u64)
}

/// The settings a live run gave Okur: the ones JAWS recorded `evidence` with, where the runner
/// wrote them for it (`provenance.settings` "recorded"); else the defaults, with the page
/// verbosity it was given (the runs before, 2026-11-08).
fn run_settings(run: &Value, evidence: &Value) -> okur_core::settings::Settings {
    if run.pointer("/provenance/settings").and_then(Value::as_str) == Some("recorded") {
        return crate::parity::recorded_settings(evidence);
    }
    let mut settings = okur_core::settings_layers::SettingsLayers::default().resolve(None, None);
    if let Some(level) = run
        .pointer("/provenance/web_verbosity")
        .and_then(Value::as_str)
    {
        settings.web_verbosity = okur_core::settings::WebVerbosity::from_jcf(level);
    }
    settings
}

/// Replays the first trial of the live `run` from its trace's `records`, with the settings
/// the run gave Okur for JAWS's `evidence` of its scenario.
pub fn replay_run(
    run: &Value,
    evidence: &Value,
    records: &[AtspiRecord],
    options: ReplayOptions<'_>,
) -> Result<Vec<ReplayStep>, String> {
    let scenario = run
        .get("scenario_id")
        .and_then(Value::as_str)
        .ok_or("the run has no scenario_id")?;
    let trial = run.pointer("/trials/0").ok_or("the run has no trial")?;
    let steps = trial
        .get("step_results")
        .and_then(Value::as_array)
        .ok_or("the trial has no steps")?;
    let settings = if options.recorded_settings {
        crate::parity::recorded_settings(evidence)
    } else {
        run_settings(run, evidence)
    };
    // A run whose driver read text fields at their carets has the reads in its trace; a run
    // before that driver read none, and its engine was told so.
    let reads_carets = records.iter().any(|r| r.text.is_some());
    let mut replayer = Replayer::new(settings, options, reads_carets);
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
        while let Some(record) = records.next_if(|r| r.handed_ms() < at) {
            replayer.feed(record);
        }
        replayer.key(&chord, at)?;
    }
    while let Some(record) = records.next_if(|r| r.handed_ms() < first) {
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
        while let Some(record) = records.next_if(|r| r.handed_ms() < start) {
            replayer.feed(record);
        }
        // Each key at its time, where the run kept them (a repeated step's presses 120 ms
        // apart), else one after another from the step's start.
        let key_times = step.get("key_times").and_then(Value::as_array);
        for (index, chord) in keys.iter().enumerate() {
            let chord = chord
                .as_str()
                .ok_or_else(|| format!("{key}: a key is not a chord"))?;
            let at = key_times
                .and_then(|times| times.get(index))
                .and_then(Value::as_u64)
                .unwrap_or_else(|| start.saturating_add(index as u64));
            // The events before this key, as they came between the presses.
            while let Some(record) = records.next_if(|r| r.handed_ms() < at) {
                replayer.feed(record);
            }
            replayer.key(chord, at)?;
        }
        while let Some(record) = records.next_if(|r| r.handed_ms() < end) {
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
pub fn run(corpus_dir: &Path, options: ReplayOptions<'_>) -> ReplayReport {
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
        if !options.only.is_empty() && !options.only.iter().any(|o| name.starts_with(o.as_str())) {
            continue;
        }
        if !trace.exists() {
            report.without_trace.push(name);
            continue;
        }
        let replayed = read_json(&path).and_then(|run| {
            let text = std::fs::read_to_string(&trace).map_err(|e| format!("{trace:?}: {e}"))?;
            let records = read_trace(&text).map_err(|e| format!("{name}: {e}"))?;
            let scenario = run
                .get("scenario_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let evidence = read_json(
                &corpus_dir
                    .join("evidence")
                    .join(format!("{scenario}-evidence.json")),
            )?;
            let steps = replay_run(&run, &evidence, &records, options)
                .map_err(|e| format!("{name}: {e}"))?;
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
        let replayed = replay_run(
            &run(steps),
            &Value::Null,
            &records,
            ReplayOptions::default(),
        )
        .unwrap_or_default();
        assert_eq!(replayed.len(), 2);
        let said: Vec<&str> = replayed.iter().map(|s| s.replayed.as_str()).collect();
        assert!(matches!(said.as_slice(), [first, second]
            if first.contains("OK") && !first.contains("Before") && second.contains("Cancel")));
    }

    /// A trace is read in the order the engine was given its records: a record stamped as
    /// given before the one before it is refused; one held for its page, given after a later
    /// event, is in order by its stamp.
    #[test]
    fn a_trace_out_of_its_delivery_order_is_refused() {
        let line = |at: u64, delivered: u64| {
            let mut record = focus(at, Role::Button, "OK");
            record.delivered_ms = Some(delivered);
            serde_json::to_string(&record).unwrap_or_default()
        };
        let held = format!("{}\n{}", line(100, 150), line(120, 160));
        assert_eq!(read_trace(&held).map(|r| r.len()), Ok(2));
        let backwards = format!("{}\n{}", line(100, 160), line(120, 150));
        assert!(read_trace(&backwards).is_err());
    }

    /// A run recorded before the runner kept its keys cannot be replayed: said, not guessed.
    #[test]
    fn a_run_without_its_keys_is_not_replayed() {
        let steps = json!([
            {"step_id": "s1", "t_start_ms": 1000, "t_end_ms": 2000,
             "action": {"type": "key", "command": "Tab"},
             "observation": {"speech_raw": ""}},
        ]);
        assert!(replay_run(&run(steps), &Value::Null, &[], ReplayOptions::default()).is_err());
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
