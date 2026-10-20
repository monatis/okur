//! Say All (Insert+Down) as a reading position. The lines go to the synthesizer a few at a
//! time, and the cursor moves to each line as the synthesizer starts it: JAWS's reading
//! commands "move the active cursor to the beginning of the text being read", so reading
//! stops on the line last heard (JAWS 2025 help, Keystrokes for Reading Text). In a page the
//! virtual cursor moves; in a text field the caret does [obs, 505/s2].
//!
//! A synthesizer that does not report its progress (replay, tests) is given every line at
//! once; in a page the cursor ends on the last line, where reading to the end leaves it.

use std::collections::VecDeque;

use super::ScreenReaderEngine;
use crate::input::{Key, KeyAction, KeyEvent};

/// Lines with the synthesizer ahead of the one being spoken: enough that no pause falls
/// between lines, few enough that a long page is not handed over whole.
const AHEAD: usize = 3;

/// A Say All in progress.
#[derive(Debug, Default)]
pub(super) struct SayAll {
    /// The lines with the synthesizer: each one's mark and line index, oldest first.
    queued: VecDeque<(u64, usize)>,
    /// The next line to hand over.
    next_line: usize,
    /// The line reading started on, which rewinding goes no further back than.
    start_line: usize,
    /// In a text field, its lines from the caret's on, each with the character offset it
    /// starts at; the line indexes above are into them. `None` in a page.
    field: Option<Vec<(usize, String)>>,
}

impl ScreenReaderEngine {
    /// Starts reading from the virtual cursor's line to the end of the page.
    pub(super) fn start_say_all(&mut self) -> Option<String> {
        let line = self.cursor.current_line_index();
        self.begin_say_all(line, None, true)
    }

    /// Starts reading a text field from the caret's line to its end, the caret following.
    pub(super) fn start_field_say_all(&mut self) -> Option<String> {
        let lines = crate::editing::lines_from_caret(self.last_text.as_ref()?);
        self.begin_say_all(0, Some(lines), true)
    }

    /// A page just loaded is read from its top, behind what was said of it (its title and
    /// summary): JAWS's "Automatically read pages when loaded", on by default [obs, 507/s1].
    /// Without the synthesizer's progress the cursor stays at the top: nobody has heard the
    /// page's end.
    pub(super) fn start_page_say_all(&mut self) -> Option<String> {
        self.cursor.move_to_start();
        let said = self.begin_say_all(0, None, false);
        if self.say_all.is_none() {
            self.cursor.move_to_start();
        }
        said
    }

    fn begin_say_all(
        &mut self,
        line: usize,
        field: Option<Vec<(usize, String)>>,
        interrupt: bool,
    ) -> Option<String> {
        let mut reading = SayAll {
            queued: VecDeque::new(),
            next_line: line,
            start_line: line,
            field,
        };
        let progress = self.speech.reports_marks();
        let ahead = if progress { AHEAD + 1 } else { usize::MAX };
        let said = self.hand_over(&mut reading, ahead, interrupt);
        if progress {
            self.say_all = Some(reading);
        } else if let (Some(&(_, last)), None) = (reading.queued.back(), &reading.field) {
            self.cursor.move_to_line(last);
        }
        Some(said.join("\n"))
    }

    /// The synthesizer started the part marked `mark`: during a Say All, the cursor moves to
    /// its line and the next lines are handed over.
    pub fn speech_reached(&mut self, mark: u64) {
        tracing::debug!(
            mark,
            following = self.say_all.is_some(),
            "Say All mark back"
        );
        let Some(mut reading) = self.say_all.take() else {
            return;
        };
        let Some(at) = reading.queued.iter().position(|&(m, _)| m == mark) else {
            self.say_all = Some(reading);
            return;
        };
        reading.queued.drain(..at);
        if let Some(&(_, line)) = reading.queued.front() {
            self.follow_reading(&reading, line);
        }
        let _ = self.hand_over(&mut reading, AHEAD + 1, false);
        self.say_all = Some(reading);
    }

    /// A key pressed during a Say All that steers it instead of ending it (JAWS 2025 help,
    /// Keystrokes for Reading Text): Right fast-forwards to the next line and Left rewinds to
    /// the one before, no further back than where reading started; PageUp and PageDown make
    /// the voice faster and slower; in a page a quick key moves, and reading goes on from
    /// there [obs, 500/s10, s11]. `None` for any other key, which ends the Say All.
    pub(super) fn say_all_key(&mut self, event: &KeyEvent) -> Option<KeyAction> {
        let reading = self.say_all.as_ref()?;
        let in_page = reading.field.is_none();
        let m = &event.modifiers;
        if m.ctrl || m.alt || m.insert || m.win || m.caps_lock || (in_page && self.forms_mode) {
            return None;
        }
        let current = reading
            .queued
            .front()
            .map_or(reading.next_line, |&(_, line)| line);
        let start_line = reading.start_line;
        let from = match event.key {
            Key::RightArrow if !m.shift => current + 1,
            Key::LeftArrow if !m.shift => current.saturating_sub(1).max(start_line),
            Key::PageUp | Key::PageDown if !m.shift => {
                let base = self.settings.voice.rate;
                let step = if event.key == Key::PageUp { 1 } else { -1 };
                self.rate_change = (base + self.rate_change + step).clamp(-10, 10) - base;
                // The synthesizer takes the new rate in the line it is speaking; reading goes
                // on without starting the line again.
                self.configure_voice();
                return Some(KeyAction::Consume);
            }
            Key::Alpha(_) | Key::Digit(_) if in_page && !self.quick_key_passes(event) => {
                let command = crate::command::command_for(event)?;
                self.speech.stop();
                self.say_all = None;
                let _ = self.execute(command);
                let moved = self.cursor.current_line_index();
                if moved == current {
                    // Nothing more to move to: JAWS's message is said and reading ends.
                    return Some(KeyAction::Consume);
                }
                self.say_all = Some(SayAll {
                    start_line: start_line.min(moved),
                    ..SayAll::default()
                });
                moved
            }
            _ => return None,
        };
        self.read_on_from(from);
        Some(KeyAction::Consume)
    }

    /// Reading starts again on line `from`, the earlier part of the Say All kept for rewinding.
    fn read_on_from(&mut self, from: usize) {
        self.speech.stop();
        let Some(old) = self.say_all.take() else {
            return;
        };
        let mut reading = SayAll {
            queued: VecDeque::new(),
            next_line: from,
            start_line: old.start_line.min(from),
            field: old.field,
        };
        if self.say_all_line(&reading, from).is_none() {
            // Past the last line: reading ends where it was.
            return;
        }
        self.follow_reading(&reading, from);
        let _ = self.hand_over(&mut reading, AHEAD + 1, true);
        self.say_all = Some(reading);
    }

    /// Ends a Say All: the cursor stays on the line last started.
    pub(super) fn end_say_all(&mut self) {
        self.say_all = None;
    }

    /// Line `index` of the Say All: the page's, or the field's.
    fn say_all_line(&self, reading: &SayAll, index: usize) -> Option<String> {
        match &reading.field {
            Some(lines) => lines.get(index).map(|(_, text)| text.clone()),
            None => self.cursor.line_text(index),
        }
    }

    /// The cursor goes to line `index` as it is read: the virtual cursor, or the field's
    /// caret, to the line's start.
    fn follow_reading(&mut self, reading: &SayAll, index: usize) {
        let Some(lines) = &reading.field else {
            self.cursor.move_to_line(index);
            return;
        };
        let (Some(&(offset, _)), Some(context)) = (lines.get(index), self.last_text.clone()) else {
            return;
        };
        if context.caret == offset {
            return;
        }
        if let Some(actions) = &self.actions {
            actions.move_caret(&context, offset);
        }
        if let Some(known) = self.last_text.as_mut() {
            known.caret = offset;
            known.selection = None;
        }
    }

    /// Hands lines to the synthesizer until `keep` are with it, the first interrupting what
    /// is spoken when `interrupt`; returns their text.
    fn hand_over(&mut self, reading: &mut SayAll, keep: usize, interrupt: bool) -> Vec<String> {
        let mut said = Vec::new();
        while reading.queued.len() < keep {
            let Some(text) = self.say_all_line(reading, reading.next_line) else {
                break;
            };
            let mark = self.next_mark;
            self.next_mark += 1;
            self.speech
                .speak_marked(&text, mark, interrupt && said.is_empty());
            reading.queued.push_back((mark, reading.next_line));
            reading.next_line += 1;
            said.push(text);
        }
        said
    }
}

#[cfg(test)]
mod tests {
    use crate::aom::{AomNode, Role};
    use crate::buffer::VirtualBuffer;
    use crate::cursor::VirtualCursor;
    use crate::engine::ScreenReaderEngine;
    use crate::input::{Key, KeyEvent, Modifiers};
    use crate::speech::SpeechOutput;
    use std::sync::{Arc, Mutex};

    /// A synthesizer that reports progress: the test says when each part starts.
    #[derive(Clone, Default)]
    struct Progress {
        parts: Arc<Mutex<Vec<(String, u64, bool)>>>,
    }

    impl SpeechOutput for Progress {
        fn speak(&mut self, _text: &str, _interrupt: bool) {}
        fn stop(&mut self) {}
        fn speak_marked(&mut self, text: &str, mark: u64, interrupt: bool) {
            if let Ok(mut parts) = self.parts.lock() {
                parts.push((text.to_owned(), mark, interrupt));
            }
        }
        fn reports_marks(&self) -> bool {
            true
        }
    }

    fn page(lines: usize) -> VirtualBuffer {
        let doc = AomNode::new(0, Role::Document).with_children(
            (1..=lines)
                .map(|i| AomNode::new(i, Role::Paragraph).with_text(format!("Line {i}")))
                .collect(),
        );
        VirtualBuffer::from_aom(&doc)
    }

    fn engine(lines: usize, speech: Progress) -> ScreenReaderEngine {
        let mut engine = ScreenReaderEngine::new(VirtualCursor::new(page(lines)), Box::new(speech));
        engine.cursor.move_to_start();
        engine
    }

    fn marks(speech: &Progress) -> Vec<(String, u64)> {
        speech
            .parts
            .lock()
            .map(|p| p.iter().map(|(t, m, _)| (t.clone(), *m)).collect())
            .unwrap_or_default()
    }

    #[test]
    fn the_cursor_follows_the_line_being_spoken_and_stays_where_reading_stopped() {
        let speech = Progress::default();
        let mut engine = engine(8, speech.clone());
        let _ = engine.start_say_all();
        // Four lines are handed over: the first, and three ahead of it.
        let handed = marks(&speech);
        assert_eq!(
            handed.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(),
            ["Line 1", "Line 2", "Line 3", "Line 4"]
        );
        // The synthesizer starts line 3: the cursor is there, and lines 5 and 6 follow.
        let third = handed.get(2).map(|(_, m)| *m).unwrap_or_default();
        engine.speech_reached(third);
        assert_eq!(engine.cursor.current_line(), "Line 3");
        assert_eq!(
            marks(&speech)
                .iter()
                .skip(4)
                .map(|(t, _)| t.as_str())
                .collect::<Vec<_>>(),
            ["Line 5", "Line 6"]
        );
        // Ctrl ends Say All; the cursor stays on the line last heard.
        let _ = engine.handle_key_event(&KeyEvent::new(Key::Control, Modifiers::none(), true));
        engine.speech_reached(third + 1);
        assert_eq!(engine.cursor.current_line(), "Line 3");
    }

    fn press(engine: &mut ScreenReaderEngine, key: Key) {
        let _ = engine.handle_key_event(&KeyEvent::new(key, Modifiers::none(), true));
    }

    /// The mark of the latest part handed over with `text`.
    fn mark_of(speech: &Progress, text: &str) -> u64 {
        marks(speech)
            .iter()
            .rev()
            .find(|(t, _)| t == text)
            .map(|(_, m)| *m)
            .unwrap_or_default()
    }

    #[test]
    fn right_and_left_skip_lines_and_rewinding_stops_where_reading_started() {
        let speech = Progress::default();
        let mut engine = engine(8, speech.clone());
        engine.cursor.move_to_line(1);
        let _ = engine.start_say_all();
        press(&mut engine, Key::RightArrow);
        assert_eq!(engine.cursor.current_line(), "Line 3");
        assert_eq!(
            marks(&speech).get(4).map(|(t, _)| t.as_str()),
            Some("Line 3")
        );
        engine.speech_reached(mark_of(&speech, "Line 4"));
        press(&mut engine, Key::LeftArrow);
        assert_eq!(engine.cursor.current_line(), "Line 3");
        press(&mut engine, Key::LeftArrow);
        press(&mut engine, Key::LeftArrow);
        // Reading started on line 2: no further back.
        assert_eq!(engine.cursor.current_line(), "Line 2");
        assert!(engine.say_all.is_some());
        // Any other key ends it there.
        press(&mut engine, Key::Control);
        assert!(engine.say_all.is_none());
        assert_eq!(engine.cursor.current_line(), "Line 2");
    }

    /// In a text field the caret goes to the start of each line as it is spoken, and Left
    /// takes it back a line [obs, 505/s2].
    #[test]
    fn in_a_text_field_the_caret_follows_the_line_being_spoken() {
        let speech = Progress::default();
        let mut engine = engine(1, speech.clone());
        let actions = crate::actions::RecordingActions::default();
        engine.set_actions(Box::new(actions.clone()));
        engine.last_text = Some(crate::event::TextContext {
            text: "one\r\ntwo\r\n\r\nthree".to_owned(),
            caret: 1,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
        });
        let _ = engine.start_field_say_all();
        assert_eq!(
            marks(&speech)
                .iter()
                .map(|(t, _)| t.as_str())
                .collect::<Vec<_>>(),
            ["one", "two", "three"]
        );
        engine.speech_reached(mark_of(&speech, "two"));
        engine.speech_reached(mark_of(&speech, "three"));
        press(&mut engine, Key::LeftArrow);
        let carets: Vec<usize> = actions
            .requests()
            .into_iter()
            .filter(|(action, _)| action == "caret to")
            .map(|(_, offset)| offset)
            .collect();
        assert_eq!(carets, [5, 12, 5]);
        assert_eq!(engine.last_text.as_ref().map(|t| t.caret), Some(5));
    }

    #[test]
    fn the_rate_keys_change_the_voice_and_reading_goes_on() {
        let speech = Progress::default();
        let mut engine = engine(4, speech.clone());
        let _ = engine.start_say_all();
        press(&mut engine, Key::PageDown);
        assert_eq!(engine.rate_change, -1);
        assert!(engine.say_all.is_some());
        assert_eq!(engine.cursor.current_line(), "Line 1");
    }

    #[test]
    fn without_progress_every_line_is_read_and_the_cursor_ends_on_the_last() {
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(page(5)),
            Box::new(crate::speech::MockSpeechOutput::default()),
        );
        engine.cursor.move_to_start();
        assert_eq!(
            engine.start_say_all().as_deref(),
            Some("Line 1\nLine 2\nLine 3\nLine 4\nLine 5")
        );
        assert_eq!(engine.cursor.current_line(), "Line 5");
    }
}
