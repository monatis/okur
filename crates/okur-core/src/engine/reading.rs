//! The reading keys where they read something other than the virtual cursor's line: a native
//! control, a text field's selection, the window's top line, and the line to and from the
//! cursor. JAWS's reading commands work in every cursor and read where it is (JAWS 2025 help,
//! Keystrokes for Reading Text).

use super::{AppMode, ScreenReaderEngine};
use crate::command::Command;
use crate::messages::msg;

impl ScreenReaderEngine {
    /// The reading commands this module answers, or `None` to leave `command` to the virtual
    /// cursor and the rest of the engine.
    pub(super) fn read_where_focused(&mut self, command: &Command) -> Option<Option<String>> {
        let native = self.app_mode != AppMode::VirtualBuffer;
        let speech = match command {
            // On a native control that is not a text field, Say Line, Say Word and the next
            // and prior word read the control as Say Focus does, without its hint: "Subscribe
            // to updates check box checked", "General Tab selected" [obs, 503/s3, s5-s7, s16,
            // s18, s20].
            Command::SayLine | Command::SayWord | Command::SayNextWord | Command::SayPriorWord
                if native && !self.editing_text() =>
            {
                self.last_native_focus
                    .as_ref()
                    .map(crate::native_focus::reading_line)
                    .unwrap_or_default()
            }
            // "Selection is" and the selected text, of a text field or the virtual cursor;
            // "Nothing selected" without one [obs, 501/s8, 503/s8, 504/s2].
            Command::SaySelectedText => {
                let selected = if self.editing_text() {
                    self.last_text
                        .as_ref()
                        .map(crate::editing::selected_text)
                        .unwrap_or_default()
                } else if native {
                    String::new()
                } else {
                    self.cursor.selected_text()
                };
                let selected = selected.trim();
                if selected.is_empty() {
                    msg::selection_none().to_string()
                } else {
                    format!("{}\n{selected}", msg::selection_is())
                }
            }
            // The window's top line is its title bar [obs, 503/s10, 504/s23].
            Command::SayTopLineOfWindow if native => self.last_window_title.clone()?,
            // The caret's line before the caret, and from it: "\tx", "= 1.5; a.b end." with
            // the caret on "=" [obs, 504/s9, s10].
            Command::SayToCursor | Command::SayFromCursor if self.editing_text() => {
                let context = self.last_text.as_ref()?;
                let (before, after) = crate::editing::line_around_caret(context);
                if *command == Command::SayToCursor {
                    before
                } else {
                    after
                }
            }
            Command::SayToCursor | Command::SayFromCursor if !native => {
                let (before, after) = self.cursor.line_around_cursor();
                if *command == Command::SayToCursor {
                    before
                } else {
                    after
                }
            }
            _ => return None,
        };
        if !speech.trim().is_empty() {
            self.speech.speak(&speech, true);
        }
        Some(Some(speech))
    }
}

impl ScreenReaderEngine {
    /// Ctrl+C with a selection made with the virtual cursor: the selection goes on the
    /// clipboard and JAWS's "Copied selection to clipboard" is said [obs, 501/s9, s14].
    /// Without one the key is the browser's.
    pub(super) fn copy_selection(&mut self) -> Option<String> {
        let text = self.cursor.selection_for_clipboard();
        if text.is_empty() {
            return None;
        }
        if let Some(actions) = &self.actions {
            actions.copy_text(&text);
        }
        Some(msg::clipboard_copied_selection().to_string())
    }
}
