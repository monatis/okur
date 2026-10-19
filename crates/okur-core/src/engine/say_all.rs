//! Say All (Insert+Down) as a reading position. The page's lines go to the synthesizer a few
//! at a time, and the virtual cursor moves to each line as the synthesizer starts it: JAWS's
//! reading commands "move the active cursor to the beginning of the text being read", so
//! reading stops on the line last heard (JAWS 2025 help, Keystrokes for Reading Text).
//!
//! A synthesizer that does not report its progress (replay, tests) is given every line at
//! once, and the cursor ends on the last line, where reading to the end leaves it.

use std::collections::VecDeque;

use super::ScreenReaderEngine;

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
}

impl ScreenReaderEngine {
    /// Starts reading from the virtual cursor's line to the end of the page.
    pub(super) fn start_say_all(&mut self) -> Option<String> {
        let mut reading = SayAll {
            queued: VecDeque::new(),
            next_line: self.cursor.current_line_index(),
        };
        let progress = self.speech.reports_marks();
        let ahead = if progress { AHEAD + 1 } else { usize::MAX };
        let said = self.hand_over(&mut reading, ahead, true);
        if progress {
            self.say_all = Some(reading);
        } else if let Some(&(_, last)) = reading.queued.back() {
            self.cursor.move_to_line(last);
        }
        Some(said.join("\n"))
    }

    /// The synthesizer started the part marked `mark`: during a Say All, the virtual cursor
    /// moves to its line and the next lines are handed over.
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
            self.cursor.move_to_line(line);
        }
        let _ = self.hand_over(&mut reading, AHEAD + 1, false);
        self.say_all = Some(reading);
    }

    /// Ends a Say All: the virtual cursor stays on the line last started.
    pub(super) fn end_say_all(&mut self) {
        self.say_all = None;
    }

    /// Hands lines to the synthesizer until `keep` are with it, the first interrupting what
    /// is spoken when `interrupt`; returns their text.
    fn hand_over(&mut self, reading: &mut SayAll, keep: usize, interrupt: bool) -> Vec<String> {
        let mut said = Vec::new();
        while reading.queued.len() < keep {
            let Some(text) = self.cursor.line_text(reading.next_line) else {
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
