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
        // The JAWS cursor reads the screen at the mouse, which Okur has no model of yet: its
        // reading keys say nothing, as JAWS's did in a WinForms dialog [obs, 516/s4, s6].
        if native && self.active_cursor == super::cursors::ActiveCursor::Jaws {
            if let Command::SayLine
            | Command::SayWord
            | Command::SayCharacter
            | Command::SayNextWord
            | Command::SayPriorWord = command
            {
                return Some(None);
            }
        }
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
            // "Nothing selected" without one [obs, 501/s8, 503/s8, 504/s2]. In a plain Win32
            // edit only its first line [obs, 529/s10].
            Command::SaySelectedText => {
                let selected = if self.editing_text() {
                    let text = self
                        .last_text
                        .as_ref()
                        .map(crate::editing::selected_text)
                        .unwrap_or_default();
                    if self.in_plain_edit() {
                        crate::editing::selection_lines(&text, true).join("\n")
                    } else {
                        text
                    }
                } else if native {
                    String::new()
                } else {
                    self.cursor.selected_text()
                };
                let selected = selected.trim();
                if selected.is_empty() {
                    msg::selection_none().to_string()
                } else if self.settings.verbosity.message_length()
                    == crate::settings::MessageLength::Short
                {
                    // Short messages say the selection alone [obs, 599/s2, 609/s2, 608/s4].
                    selected.to_owned()
                } else {
                    format!("{}\n{selected}", msg::selection_is())
                }
            }
            // Pressed twice quickly, the selection and the line to or from the cursor are
            // spelled, a tab as a space: "=", " space x space" [obs, 504/s11, s18].
            Command::SpellSelectedText | Command::SpellToCursor | Command::SpellFromCursor => {
                let text = match command {
                    Command::SpellSelectedText if self.editing_text() => self
                        .last_text
                        .as_ref()
                        .map(crate::editing::selected_text)
                        .unwrap_or_default(),
                    Command::SpellSelectedText if !native => self.cursor.selected_text(),
                    Command::SpellToCursor | Command::SpellFromCursor if self.editing_text() => {
                        let (before, after) =
                            crate::editing::line_around_caret(self.last_text.as_ref()?);
                        if *command == Command::SpellToCursor {
                            before
                        } else {
                            after
                        }
                    }
                    _ => return Some(None),
                };
                // Each run of white space is one space, kept at the ends but for a selection.
                let mut collapsed = String::new();
                for c in text.chars() {
                    let c = if c.is_whitespace() { ' ' } else { c };
                    if !(c == ' ' && collapsed.ends_with(' ')) {
                        collapsed.push(c);
                    }
                }
                let text = if *command == Command::SpellSelectedText {
                    collapsed.trim()
                } else {
                    collapsed.as_str()
                };
                // JAWS's history keeps a spelled selection as it is, "Th", though it is spoken
                // letter by letter [obs, 501/s4].
                if *command == Command::SpellSelectedText {
                    if !text.is_empty() {
                        self.speech.speak_spelled_selection(text, true);
                    }
                    return Some(Some(text.to_owned()));
                }
                if !text.trim().is_empty() {
                    self.speech.speak_spelled(text, true);
                }
                return Some(Some(crate::pronounce::spelled(text)));
            }
            // The window's top line is its title bar [obs, 503/s10, 504/s23].
            Command::SayTopLineOfWindow if native => self.last_window_title.clone()?,
            // The window's bottom line: its status bar, where the platform reports one (WordPad's
            // "100%" [obs, 522/s20]; "the bottom line is usually the status bar", JAWS 2025
            // help); else in an editor's page its text's last line, "teh quick brown fxo"
            // [obs, 510/s13, 504/s22]. Other windows wait for their screen lines.
            Command::SayBottomLineOfWindow
                if native
                    && self
                        .last_native_focus
                        .as_ref()
                        .is_some_and(|f| !f.status_bar.is_empty()) =>
            {
                let parts = &self.last_native_focus.as_ref()?.status_bar;
                parts
                    .iter()
                    .map(|part| part.trim())
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join("  ")
            }
            Command::SayBottomLineOfWindow if native && self.editing_document() => {
                crate::editing::text_lines(&self.last_text.as_ref()?.text)
                    .into_iter()
                    .rev()
                    .find(|line| !line.trim().is_empty())?
            }
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
            // In a text field the sentence keys read the caret's sentence, and the next and
            // prior one, which they move the caret to: "Hello, world.", "Is it done?"
            // [obs, 510/s1-s3, s7].
            Command::SaySentence | Command::SayNextSentence | Command::SayPriorSentence
                if self.editing_text() =>
            {
                let context = self.last_text.clone()?;
                let sentences = char_sentences(&context.text);
                let current = sentences
                    .iter()
                    .rposition(|&(start, _)| start <= context.caret)
                    .unwrap_or(0);
                let index = match command {
                    Command::SayNextSentence => {
                        (current + 1).min(sentences.len().saturating_sub(1))
                    }
                    Command::SayPriorSentence => current.saturating_sub(1),
                    _ => current,
                };
                let &(start, end) = sentences.get(index)?;
                if *command != Command::SaySentence && start != context.caret {
                    if let Some(actions) = &self.actions {
                        actions.move_caret(&context, start);
                    }
                    if let Some(known) = self.last_text.as_mut() {
                        known.caret = start;
                        known.selection = None;
                    }
                }
                // Read to the end of the line it starts on: "a !\"#…~ B" for a sentence that
                // goes on to the next line [obs, 504/s14].
                context
                    .text
                    .chars()
                    .skip(start)
                    .take(end - start)
                    .take_while(|&c| !crate::editing::is_line_break(c))
                    .collect()
            }
            // A text field's paragraph keys read the first line of the caret's paragraph and
            // stay put [obs, 510/s4-s6, 504/s15].
            Command::SayParagraph
            | Command::SayNextParagraph
            | Command::SayPriorParagraph
            | Command::NextParagraphOrArticles
                if native && self.editing_text() =>
            {
                crate::editing::paragraph_first_line(self.last_text.as_ref()?)
            }
            _ => return None,
        };
        if !speech.trim().is_empty() {
            self.speech.speak(&speech, true);
        }
        Some(Some(speech))
    }
}

/// `text`'s sentences as character ranges.
fn char_sentences(text: &str) -> Vec<(usize, usize)> {
    let char_at = |byte: usize| text.get(..byte).map_or(0, |before| before.chars().count());
    crate::editing::sentences(text)
        .into_iter()
        .map(|(start, end)| (char_at(start), char_at(end)))
        .collect()
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
        // Worded for the level: "Copied" when short, nothing at Advanced [obs, 608/s9]; the
        // key was taken either way.
        Some(self.copied_message().unwrap_or_default())
    }

    /// JAWS's "Copied selection to clipboard" as the verbosity level words it: short above
    /// Beginner, none at Advanced (JAWS Message) [obs, 599/s3, 609/s3].
    pub(super) fn copied_message(&self) -> Option<String> {
        let wording = crate::editing::Wording::of(&self.settings.verbosity);
        if !wording.messages {
            None
        } else if wording.short {
            Some(msg::clipboard_copied_selection_short().to_string())
        } else {
            Some(msg::clipboard_copied_selection().to_string())
        }
    }
}
