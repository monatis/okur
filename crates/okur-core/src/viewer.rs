//! The Virtual Viewer: JAWS's read-only text window, opened by Virtualize Window
//! (`Insert+Alt+W`) and Virtualize Current Control (`Insert+Shift+V`).
//!
//! Observed behavior [obs, 311, 312]:
//! - opening speaks "Virtualizing current window" (or "current control") and the first line;
//! - Up and Down read lines, and at the top or bottom read the line again;
//! - Ctrl+Home and Ctrl+End read the first and last line;
//! - Left and Right read characters;
//! - Ctrl+Left and Ctrl+Right read words, where a run of punctuation is a word of its own
//!   ("Save... Button" is "Save", "...", "Button");
//! - Insert+Up reads the line;
//! - Ctrl+A speaks "selected" and the whole text; Ctrl+C speaks "Copied selection to
//!   clipboard";
//! - Escape closes it, and the focus is read again. The viewer is a window of its own:
//!   other keys (Tab, Shift+Tab, Enter) stay in it silently [obs, 311/s28–s31], although
//!   JAWS help says Tab closes it; Virtualize again says "The current location is already
//!   in a virtual view" (311/s32).

use crate::input::{Key, KeyEvent};
use crate::ui::TextSpan;

/// What the viewer does with a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerAction {
    /// The key was handled: speak this, if anything.
    Speak(Option<String>),
    /// A character was read: speak it as a character (named when it is a space or symbol).
    Character(String),
    /// Copy this text to the clipboard and speak the confirmation.
    Copy(String),
    /// Close the viewer.
    Close,
    /// Not a viewer key (e.g. another JAWS command): handle it as usual.
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualViewer {
    lines: Vec<Vec<char>>,
    line: usize,
    column: usize,
    all_selected: bool,
    /// What was read last on the line, as (start column, length): the line, a word or a
    /// character. A window showing the viewer marks it.
    read: (usize, usize),
}

/// Word classes: a word is a run of one class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Space,
    Word,
    Punctuation,
}

fn class(c: char) -> Class {
    if c.is_whitespace() {
        Class::Space
    } else if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Punctuation
    }
}

impl VirtualViewer {
    /// A viewer over `lines`, at the start of the first.
    pub fn new(lines: Vec<String>) -> Self {
        Self::at_line(lines, 0)
    }

    /// A viewer over `lines`, at the start of `line` (a console's: the cursor's line).
    pub fn at_line(lines: Vec<String>, line: usize) -> Self {
        let mut lines: Vec<Vec<char>> = lines.iter().map(|l| l.chars().collect()).collect();
        if lines.is_empty() {
            lines.push(Vec::new());
        }
        let line = line.min(lines.len() - 1);
        let mut viewer = Self {
            lines,
            line,
            column: 0,
            all_selected: false,
            read: (0, 0),
        };
        viewer.read_line();
        viewer
    }

    /// A JAWS message (Say Font twice quickly): JAWS reads it all on opening, so the cursor
    /// ends on its last line [obs, 314/s2–s5: Down repeats "Press ESCAPE to close this
    /// message."].
    pub fn message(lines: Vec<String>) -> Self {
        let mut viewer = Self::new(lines);
        viewer.line = viewer.lines.len() - 1;
        viewer.read_line();
        viewer
    }

    /// What JAWS reads when a message opens: its lines, skipping blank ones.
    pub fn message_speech(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.iter().collect::<String>())
            .filter(|l| !l.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The whole text, lines joined with Windows line breaks (what Ctrl+C copies).
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\r\n")
    }

    fn current(&self) -> &[char] {
        self.lines
            .get(self.line)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// The current line.
    pub fn line_text(&self) -> String {
        self.current().iter().collect()
    }

    /// The viewer's lines.
    pub fn lines(&self) -> Vec<String> {
        self.lines.iter().map(|l| l.iter().collect()).collect()
    }

    /// What the cursor read last: the line, the word or the character.
    pub fn mark(&self) -> TextSpan {
        TextSpan {
            line: self.line,
            column: self.read.0,
            length: self.read.1,
        }
    }

    /// Marks the whole current line as read, its cursor at its start.
    fn read_line(&mut self) {
        self.column = 0;
        self.read = (0, self.current().len());
    }

    fn char_text(&self) -> String {
        self.current()
            .get(self.column)
            .map(char::to_string)
            .unwrap_or_default()
    }

    fn word_text(&self) -> String {
        let line = self.current();
        let Some(&first) = line.get(self.column) else {
            return String::new();
        };
        let kind = class(first);
        line.iter()
            .skip(self.column)
            .take_while(|&&c| class(c) == kind)
            .collect()
    }

    fn move_line(&mut self, down: bool) -> String {
        if down && self.line + 1 < self.lines.len() {
            self.line += 1;
        } else if !down && self.line > 0 {
            self.line -= 1;
        }
        self.read_line();
        self.line_text()
    }

    fn move_char(&mut self, right: bool) -> String {
        let len = self.current().len();
        if right {
            if self.column + 1 < len {
                self.column += 1;
            } else if self.line + 1 < self.lines.len() {
                self.line += 1;
                self.column = 0;
            }
        } else if self.column > 0 {
            self.column -= 1;
        } else if self.line > 0 {
            self.line -= 1;
            self.column = self.current().len().saturating_sub(1);
        }
        let text = self.char_text();
        self.read = (self.column, text.chars().count());
        text
    }

    /// The start of the next (or previous) word on the line, then on the following lines.
    fn move_word(&mut self, right: bool) -> String {
        let line = self.current().to_vec();
        let starts: Vec<usize> = (0..line.len())
            .filter(|&i| {
                let c = line.get(i).copied().map(class);
                c != Some(Class::Space) && (i == 0 || line.get(i - 1).copied().map(class) != c)
            })
            .collect();
        let target = if right {
            starts.iter().copied().find(|&s| s > self.column)
        } else {
            starts.iter().copied().rev().find(|&s| s < self.column)
        };
        match target {
            Some(start) => self.column = start,
            None if right && self.line + 1 < self.lines.len() => {
                self.line += 1;
                self.column = 0;
            }
            None if !right && self.line > 0 => {
                self.line -= 1;
                let previous = self.current().to_vec();
                self.column = (0..previous.len())
                    .rev()
                    .find(|&i| {
                        let c = previous.get(i).copied().map(class);
                        c != Some(Class::Space)
                            && (i == 0 || previous.get(i - 1).copied().map(class) != c)
                    })
                    .unwrap_or(0);
            }
            None => {}
        }
        let text = self.word_text();
        self.read = (self.column, text.chars().count());
        text
    }

    /// The viewer's response to a key press.
    pub fn key(&mut self, event: &KeyEvent) -> ViewerAction {
        let m = event.modifiers;
        let plain = !m.ctrl && !m.alt && !m.shift && !m.win && !m.insert;
        let ctrl_only = m.ctrl && !m.alt && !m.shift && !m.win && !m.insert;
        let speak = |s: String| ViewerAction::Speak(Some(s));
        // An empty line is read as its line break: JAWS's history keeps nothing and the
        // synthesizer says "blank" (Ctrl+End to a console's empty rows) [obs, 321/s6].
        let line = |s: String| {
            if s.trim().is_empty() {
                ViewerAction::Character("\n".to_owned())
            } else {
                ViewerAction::Speak(Some(s))
            }
        };
        match event.key {
            Key::Escape if plain => ViewerAction::Close,
            Key::DownArrow | Key::UpArrow if plain => {
                self.all_selected = false;
                line(self.move_line(event.key == Key::DownArrow))
            }
            Key::UpArrow if m.insert && !m.ctrl && !m.alt && !m.shift => line(self.line_text()),
            Key::RightArrow | Key::LeftArrow if plain => {
                self.all_selected = false;
                ViewerAction::Character(self.move_char(event.key == Key::RightArrow))
            }
            Key::RightArrow | Key::LeftArrow if ctrl_only => {
                self.all_selected = false;
                speak(self.move_word(event.key == Key::RightArrow))
            }
            Key::Home | Key::End if ctrl_only => {
                self.all_selected = false;
                self.line = if event.key == Key::Home {
                    0
                } else {
                    self.lines.len() - 1
                };
                self.read_line();
                line(self.line_text())
            }
            Key::Alpha(c) if ctrl_only && c.eq_ignore_ascii_case(&'A') => {
                self.all_selected = true;
                let all = self
                    .lines
                    .iter()
                    .map(|l| l.iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join("\n");
                speak(format!("selected\n{all}"))
            }
            Key::Alpha(c) if ctrl_only && c.eq_ignore_ascii_case(&'C') => {
                if self.all_selected {
                    ViewerAction::Copy(self.text())
                } else {
                    ViewerAction::Speak(None)
                }
            }
            // Other JAWS commands work as usual; anything else stays in the viewer.
            _ if m.insert => ViewerAction::Ignore,
            _ => ViewerAction::Speak(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Modifiers;

    fn press(viewer: &mut VirtualViewer, key: Key, ctrl: bool) -> ViewerAction {
        let mut event = KeyEvent::new(key, Modifiers::none(), true);
        event.modifiers.ctrl = ctrl;
        viewer.key(&event)
    }

    fn said(action: ViewerAction) -> String {
        match action {
            ViewerAction::Speak(s) => s.unwrap_or_default(),
            ViewerAction::Character(c) => c,
            other => format!("{other:?}"),
        }
    }

    /// The message box of 311 (s14–s22).
    #[test]
    fn lines_repeat_at_the_bottom_and_select_all_reads_everything() {
        let mut viewer = VirtualViewer::new(vec![
            "SR Dialog Fixture".into(),
            "Save changes to the document?".into(),
            "Yes No Cancel".into(),
        ]);
        assert_eq!(viewer.line_text(), "SR Dialog Fixture");
        assert_eq!(
            said(press(&mut viewer, Key::DownArrow, false)),
            "Save changes to the document?"
        );
        assert_eq!(
            said(press(&mut viewer, Key::DownArrow, false)),
            "Yes No Cancel"
        );
        assert_eq!(
            said(press(&mut viewer, Key::DownArrow, false)),
            "Yes No Cancel"
        );
        assert_eq!(
            said(press(&mut viewer, Key::Home, true)),
            "SR Dialog Fixture"
        );
        assert_eq!(
            said(press(&mut viewer, Key::Alpha('A'), true)),
            "selected\nSR Dialog Fixture\nSave changes to the document?\nYes No Cancel"
        );
        assert_eq!(
            press(&mut viewer, Key::Alpha('C'), true),
            ViewerAction::Copy(
                "SR Dialog Fixture\r\nSave changes to the document?\r\nYes No Cancel".into()
            )
        );
        assert_eq!(press(&mut viewer, Key::Escape, false), ViewerAction::Close);
        // The viewer is a window of its own: Tab stays in it [obs, 311/s28–s32].
        assert_eq!(
            press(&mut viewer, Key::Tab, false),
            ViewerAction::Speak(None)
        );
    }

    /// The button of 311 (s1–s9). Its blank line is read as a line break: JAWS was heard,
    /// with nothing in its history [obs, 311/s2–s3].
    #[test]
    fn characters_and_words_split_punctuation_runs() {
        let mut viewer = VirtualViewer::new(vec!["Save... Button".into(), " ".into()]);
        let blank = ViewerAction::Character("\n".to_owned());
        assert_eq!(press(&mut viewer, Key::DownArrow, false), blank);
        assert_eq!(press(&mut viewer, Key::End, true), blank);
        assert_eq!(
            said(press(&mut viewer, Key::UpArrow, false)),
            "Save... Button"
        );
        assert_eq!(said(press(&mut viewer, Key::RightArrow, false)), "a");
        assert_eq!(said(press(&mut viewer, Key::RightArrow, true)), "...");
        assert_eq!(said(press(&mut viewer, Key::RightArrow, true)), "Button");
        assert_eq!(said(press(&mut viewer, Key::LeftArrow, true)), "...");
        assert_eq!(said(press(&mut viewer, Key::LeftArrow, true)), "Save");
    }

    /// The window that shows the viewer marks what was read: a line, a word, a character.
    #[test]
    fn the_mark_follows_what_was_read() {
        let span = |line, column, length| TextSpan {
            line,
            column,
            length,
        };
        let mut viewer = VirtualViewer::new(vec!["Save... Button".into(), "OK".into()]);
        assert_eq!(viewer.mark(), span(0, 0, 14));
        press(&mut viewer, Key::RightArrow, true);
        assert_eq!(viewer.mark(), span(0, 4, 3));
        press(&mut viewer, Key::RightArrow, false);
        assert_eq!(viewer.mark(), span(0, 5, 1));
        press(&mut viewer, Key::DownArrow, false);
        assert_eq!(viewer.mark(), span(1, 0, 2));
        press(&mut viewer, Key::Home, true);
        assert_eq!(viewer.mark(), span(0, 0, 14));
        assert_eq!(viewer.lines(), ["Save... Button", "OK"]);
        // A message opens on its last line.
        let message = VirtualViewer::message(vec!["Font information:".into(), "Press".into()]);
        assert_eq!(message.mark(), span(1, 0, 5));
    }
}
