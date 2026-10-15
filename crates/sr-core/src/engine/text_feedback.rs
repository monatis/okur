//! What edit keys say: the text, caret and selection the application reports after a key.

use super::ScreenReaderEngine;
use crate::aom::Role;
use crate::input::KeyEvent;
use crate::speech::SpeechFormatter;

impl ScreenReaderEngine {
    /// Feedback for a key passed to a text field: keys that move the caret, select or
    /// delete are spoken once the application has handled them (from the field's text
    /// before and after); typed characters are echoed, as JAWS does with its default typing
    /// echo (characters).
    pub(super) fn edit_key_feedback(&mut self, event: &KeyEvent) {
        // Windows search's box: Up and Down select results, read as they are selected.
        if self
            .last_native_focus
            .as_ref()
            .is_some_and(|f| crate::apps::windows_search::arrows_select(f, event))
        {
            return;
        }
        if let Some(key) = crate::editing::EditKey::of(event) {
            // A multi-line native document (Windows 11 Notepad) reads some keys otherwise.
            let document = !self.forms_mode
                && self
                    .last_native_focus
                    .as_ref()
                    .is_some_and(|f| f.role == Role::Document);
            let key = if document {
                key.in_document(event.key, event.modifiers.ctrl)
            } else {
                key
            };
            self.expect_text_change(key);
        } else if let Some(c) = event.text {
            let echo = if c == ' ' {
                "space".to_owned()
            } else {
                SpeechFormatter::format_char(c)
            };
            tracing::debug!(%echo, "typing echo");
            self.speech.speak_echo(&echo);
            // Keep the field's known text current (no speech follows), so the next edit key
            // (Backspace) is spoken from the text as typed.
            if let Some(actions) = &self.actions {
                actions.read_at_caret(crate::event::CaretUnit::Character);
            }
        }
    }

    /// Remembers the edit key until the field reports its new text, and asks the platform
    /// to read it (applications do not all report caret moves).
    pub(super) fn expect_text_change(&mut self, key: crate::editing::EditKey) {
        tracing::debug!(?key, "edit key; reading the field");
        self.pending_edit = Some(key);
        if let Some(actions) = &self.actions {
            actions.read_at_caret(crate::event::CaretUnit::Character);
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
        let before = self.last_text.replace(context.clone());
        let key = self.pending_edit.take()?;
        let field = self.text_field_line();
        let speech = crate::editing::speech(key, before.as_ref(), context, &field)?;
        tracing::debug!(?key, %speech, "edit speech");
        use crate::editing::EditKey;
        let character = matches!(
            key,
            EditKey::Character | EditKey::DeleteForward | EditKey::DeleteBackward
        ) && speech.chars().count() == 1;
        if character {
            self.speech.speak_character(&speech, true);
        } else {
            self.speech.speak(&speech, true);
        }
        Some(speech)
    }

    /// The focused text field's name and role, as Say Line speaks them.
    pub(super) fn text_field_line(&self) -> String {
        let name = if self.forms_mode {
            self.node_under_cursor().and_then(|n| n.name)
        } else {
            self.last_native_focus.as_ref().and_then(|f| f.name.clone())
        };
        format!(
            "{}  \u{1d} Edit \u{1e} ",
            name.as_deref().unwrap_or_default().trim()
        )
    }
}
