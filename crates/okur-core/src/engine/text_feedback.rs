//! What edit keys say: the text, caret and selection the application reports after a key.

use super::ScreenReaderEngine;
use crate::input::KeyEvent;
use crate::messages::msg;
use crate::speech::SpeechFormatter;

impl ScreenReaderEngine {
    /// Feedback for a key passed to a text field: keys that move the caret, select or
    /// delete are spoken once the application has handled them (from the field's text
    /// before and after); typed characters are echoed, as JAWS does with its default typing
    /// echo (characters).
    pub(super) fn edit_key_feedback(&mut self, event: &KeyEvent) {
        // An application that reads the key itself (Windows search: Up and Down select
        // results, read as they are selected).
        if self.last_native_focus.as_ref().is_some_and(|field| {
            self.app_for(field)
                .is_some_and(|app| app.reads_key(field, event))
        }) {
            return;
        }
        // Up and Down in a field pointing at a list's option (an autocomplete) move to the
        // next option, which its focus event says; the field's text is not read.
        if self.option_list.is_some()
            && matches!(event.key, crate::Key::UpArrow | crate::Key::DownArrow)
        {
            return;
        }
        let own_key = self.last_native_focus.as_ref().and_then(|field| {
            self.app_for(field)
                .and_then(|app| app.edit_key(field, event))
        });
        if let Some(key) = crate::editing::EditKey::of(event).or(own_key) {
            // The word being typed ends where the caret moves; Backspace takes back its last
            // character.
            if key == crate::editing::EditKey::DeleteBackward {
                self.typed_word.pop();
            } else {
                self.typed_word.clear();
            }
            // A document (Windows 11 Notepad; a page's multi-line field, Google Docs' with
            // braille support, where JAWS said "top of file" for Ctrl+Home and the character
            // for Home on the dev box, 2026-10-16) reads some keys otherwise.
            let key = if self.editing_document() {
                key.in_document(event.key, event.modifiers.ctrl)
            } else {
                key
            };
            self.expect_text_change(key);
        } else if event.key == crate::Key::Enter && event.modifiers.is_empty() {
            self.echo_enter();
        } else if let Some(c) = event.text {
            self.echo_typed(c);
            // Keep the field's known text current (no speech follows), so the next edit key
            // (Backspace) is spoken from the text as typed.
            if let Some(actions) = &self.actions {
                actions.read_at_caret(
                    self.last_text.as_ref(),
                    self.app_mode == super::AppMode::VirtualBuffer,
                );
            }
        }
    }

    /// Says a typed character, as the typing echo setting has it: the character, and with
    /// word echo the word a space ends. Space is named only when characters alone are echoed
    /// [obs, 403/s10, s17, s24].
    pub(super) fn echo_typed(&mut self, c: char) {
        use crate::settings::TypingEcho;
        let echo = self.settings.typing_echo;
        if c == ' ' {
            self.echo_word_end();
            if echo == TypingEcho::Characters {
                self.speak_echo(msg::typing_echo_space().as_str());
            }
            return;
        }
        self.typed_word.push(c);
        if echo.says_characters() {
            self.speak_echo(&SpeechFormatter::format_char(c));
        }
    }

    /// Enter in a text field ends the word typed, and is named unless the echo is off
    /// [obs, 403/s13, s20, s27, s34].
    fn echo_enter(&mut self) {
        self.echo_word_end();
        if self.settings.typing_echo != crate::settings::TypingEcho::None {
            self.speak_echo(msg::typing_echo_enter().as_str());
        }
    }

    /// The word typed so far ends: word echo says it.
    fn echo_word_end(&mut self) {
        let word = std::mem::take(&mut self.typed_word);
        if self.settings.typing_echo.says_words() && !word.is_empty() {
            self.speak_echo(&word);
        }
    }

    fn speak_echo(&mut self, echo: &str) {
        tracing::debug!(%echo, "typing echo");
        self.speech.speak_echo(echo);
    }

    /// Remembers the edit key until the field reports its new text, and asks the platform
    /// to read it (applications do not all report caret moves).
    pub(super) fn expect_text_change(&mut self, key: crate::editing::EditKey) {
        tracing::debug!(?key, "edit key; reading the field");
        self.pending_edit = Some(key);
        if let Some(actions) = &self.actions {
            actions.read_at_caret(
                self.last_text.as_ref(),
                self.app_mode == super::AppMode::VirtualBuffer,
            );
        }
    }

    /// The focused text field's text changed: speaks the pending edit key's effect.
    pub(super) fn handle_text_context(
        &mut self,
        context: &crate::event::TextContext,
    ) -> Option<String> {
        // A Virtual Viewer waiting for a console's text takes it.
        if let Some(spoken) = self.open_console_viewer(context) {
            return spoken;
        }
        // A read that caught the page's document, not the field the focus is moving to: its
        // text is the document's embedded objects alone ("￼"), right after a field took the
        // focus (280/s5, s11, s20 traces). The field's own text is kept.
        if !context.text.is_empty() && context.text.chars().all(|c| c == '\u{fffc}') {
            return None;
        }
        // A field's misspellings stay while its text does: a read that does not report them
        // keeps the last ones (the word keys' reads in a page field gave none, where the read
        // at its end gave "teh" [567 traces]).
        let mut context = context.clone();
        if context.misspelled.is_empty() {
            if let Some(last) = self.last_text.as_ref().filter(|l| l.text == context.text) {
                context.misspelled = last.misspelled.clone();
            }
        }
        let context = &context;
        let before = self.last_text.replace(context.clone());
        let key = self.pending_edit.take()?;
        let field = self.text_field_line();
        let mut speech =
            crate::editing::speech_in(key, before.as_ref(), context, &field, self.in_plain_edit())?;
        // An edit combo's line is the item its list selected: said with its position
        // ("Constantia | 39 of 187" [obs, 541/s3]).
        if key == crate::editing::EditKey::Line {
            let count = self
                .last_native_focus
                .as_ref()
                .filter(|f| f.container_role == Some(crate::aom::Role::ComboBox))
                .and_then(|f| f.item_count);
            let index = self
                .combo_items
                .iter()
                .find(|(name, _)| name == speech.trim())
                .map(|(_, index)| *index);
            if let (Some(index), Some(count)) = (index, count) {
                speech = format!("{speech}\n{}", msg::position(index, count));
            }
        }
        tracing::debug!(?key, %speech, "edit speech");
        self.edit_read_in_console = self
            .last_native_focus
            .as_ref()
            .is_some_and(|field| self.app_for(field).is_some_and(|app| app.types_into(field)));
        use crate::editing::EditKey;
        // An empty line or word is read as its line break, which the synthesizer says as
        // "blank" ([`crate::editing::speech`]).
        let character = (matches!(
            key,
            EditKey::Character | EditKey::DeleteForward | EditKey::DeleteBackward
        ) && speech.chars().count() == 1)
            || (!speech.is_empty() && speech.chars().all(crate::editing::is_line_break));
        // A word and a selection are read with every symbol named [obs, 213/s11, 313/s7].
        let symbols = matches!(
            key,
            EditKey::Word | EditKey::DeleteWordBackward | EditKey::SelectMore | EditKey::SelectAll
        );
        if character {
            self.speech.speak_character(&speech, true);
        } else if symbols {
            self.speech.speak_symbols(&speech, true);
        } else {
            self.speech.speak(&speech, true);
        }
        Some(speech)
    }

    /// The focused text field is a plain Win32 edit ([`crate::event::FocusEvent::plain_edit`]).
    pub(super) fn in_plain_edit(&self) -> bool {
        self.app_mode != super::AppMode::VirtualBuffer
            && self
                .last_native_focus
                .as_ref()
                .is_some_and(|f| f.plain_edit)
    }

    /// The focused text field's name and role, as Say Line speaks them.
    pub(super) fn text_field_line(&self) -> String {
        let name = if self.forms_mode {
            self.node_under_cursor().and_then(|n| n.name.clone())
        } else {
            self.last_native_focus.as_ref().and_then(|f| f.name.clone())
        };
        // A required page field says so after its role: "Full name Edit Required" [obs,
        // 567/s3].
        let required = self.forms_mode
            && self
                .node_under_cursor()
                .is_some_and(|n| n.attributes.get("required").is_some_and(|r| r == "true"));
        let mut line = format!(
            "{}  \u{1d} {} \u{1e} ",
            name.as_deref().unwrap_or_default().trim(),
            crate::messages::msg::role_edit()
        );
        if required {
            line.push_str(&format!(
                " \u{1d}{}\u{1e} ",
                crate::messages::msg::state_required()
            ));
        }
        line
    }

    /// A page's edit field as JAWS says it when one of its lists puts it in forms mode: its
    /// name and role, then its line, as Say Line reads it in forms mode ("Email address Edit
    /// | sam@example.com" [obs, 249/s3, s4]; a text area's role is "edit" [obs, 228/s9]).
    pub(super) fn field_forms_line(&self, node_id: usize) -> Option<String> {
        use crate::messages::msg;
        let node = self.cursor.buffer.node(node_id)?;
        let role = if node
            .attributes
            .get("multiline")
            .is_some_and(|v| v == "true")
        {
            msg::role_edit_multiline()
        } else {
            msg::role_edit()
        };
        let field = format!(
            "{}  \u{1d} {role} \u{1e} ",
            node.name.as_deref().unwrap_or_default().trim()
        );
        let line = node
            .value
            .as_deref()
            .and_then(|v| v.lines().next())
            .unwrap_or_default()
            .trim();
        Some(if line.is_empty() {
            field
        } else {
            format!("{field}\n{line}")
        })
    }
}
