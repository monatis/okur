//! JAWS's clipboard keys: Windows+Okur key+X says the clipboard's text (SayClipboard), the
//! layer's C shows it (ViewClipboardText), and Okur key+Windows+C adds the selection to it
//! (AppendSelectedTextToClipboard, JAWS's FSClipboard) [obs, 825].

use super::ScreenReaderEngine;
use crate::input::{Key, KeyEvent};
use crate::messages::msg;
use crate::ui::clipboard::{Outcome, OverwriteDialog};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// Windows+Okur key+X: the clipboard's text, a line each ("fn main left paren right paren
    /// left brace", then the line appended to it) [obs, 825/s3, s12, s20].
    pub(super) fn say_clipboard(&mut self) -> Option<String> {
        let text = self.actions.as_ref()?.clipboard_text()?;
        let lines = crate::editing::text_lines(&text)
            .into_iter()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
        if lines.is_empty() {
            return None;
        }
        let said = lines.join("\n");
        self.speech.speak(&said, true);
        Some(said)
    }

    /// Insert+Space, C: the clipboard's text in the viewer "Clipboard Text", said as it opens
    /// by nothing; its first line is its title, as JAWS's viewer's page title is, which Say
    /// Line reads [obs, 825/s5, s6], then the text and how to close it.
    pub(super) fn view_clipboard_text(&mut self) -> Option<String> {
        let text = self
            .actions
            .as_ref()
            .and_then(|actions| actions.clipboard_text())
            .unwrap_or_default();
        let title = msg::clipboard_viewer_title().to_string();
        let lines: Vec<String> = std::iter::once(title.clone())
            .chain(crate::editing::text_lines(&text))
            .chain(std::iter::once(msg::clipboard_viewer_close().to_string()))
            .collect();
        self.viewer = Some(
            crate::viewer::VirtualViewer::new(lines)
                .with_title(title)
                .in_own_window(),
        );
        self.show_viewer_window();
        Some(String::new())
    }

    /// Okur key+Windows+C: the selection added to the clipboard's text on a line of its own,
    /// "Appended selection to clipboard" [obs, 825/s11], "Appended" at Intermediate, nothing at
    /// Advanced, as JAWS's own messages are [obs, 859/s11, 860/s11]; Say Clipboard then reads both [obs,
    /// 825/s12]. The selection is the virtual cursor's in a page, else the text field's; with
    /// none, "Nothing selected" [obs, 826/s4].
    pub(super) fn append_selection_to_clipboard(&mut self) -> Option<String> {
        let selection = if self.app_mode == super::AppMode::VirtualBuffer {
            self.cursor.selection_for_clipboard()
        } else {
            String::new()
        };
        let selection = if selection.is_empty() {
            self.last_text
                .as_ref()
                .map(crate::editing::selected_text)
                .unwrap_or_default()
        } else {
            selection
        };
        if selection.is_empty() {
            let said = msg::selection_none().to_string();
            self.speech.speak(&said, true);
            return Some(said);
        }
        let actions = self.actions.as_ref()?;
        let text = match actions.clipboard_text().filter(|t| !t.is_empty()) {
            Some(held) => format!("{held}\r\n{selection}"),
            None => selection,
        };
        actions.copy_text(&text);
        self.clipboard_appended = true;
        let verbosity = &self.settings.verbosity;
        if !verbosity.is_on(crate::settings::OutputType::OkurMessage) {
            return Some(String::new());
        }
        let said = if verbosity.message_length() == crate::settings::MessageLength::Short {
            msg::clipboard_appended_selection_short()
        } else {
            msg::clipboard_appended_selection()
        }
        .to_string();
        self.speech.speak(&said, true);
        Some(said)
    }

    /// Ctrl+C or Ctrl+X in a text field with a selection, after the clipboard was added to:
    /// the question opens and the key is taken [obs, 825/s18]. Okur says nothing itself: the
    /// question is read as it takes the focus. Returns whether the key was taken.
    pub(super) fn ask_before_overwriting(&mut self, event: &KeyEvent) -> bool {
        let m = event.modifiers;
        let copy_or_cut = matches!(event.key, Key::Alpha('C') | Key::Alpha('X'))
            && m.ctrl
            && !m.alt
            && !m.shift
            && !m.insert
            && !m.win;
        if !copy_or_cut || !self.clipboard_appended || !self.editing_text() {
            return false;
        }
        let selection = self
            .last_text
            .as_ref()
            .map(crate::editing::selected_text)
            .unwrap_or_default();
        let Some(ui) = self.ui.as_ref().filter(|_| !selection.is_empty()) else {
            return false;
        };
        self.dialog_count += 1;
        let dialog = OverwriteDialog {
            id: self.dialog_count,
            cut: event.key == Key::Alpha('X'),
            selection,
            window: self.last_window_title.clone(),
            window_kind: self.last_window_kind.clone(),
        };
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.overwrite_dialog = Some(dialog);
        true
    }

    /// Whether `message` is the question's.
    pub(super) fn is_overwrite_event(&self, message: &FromUi) -> bool {
        self.overwrite_dialog
            .as_ref()
            .is_some_and(|d| Some(d.id) == super::dialog_of(message))
    }

    /// Yes copies the selection, "Copied selection to clipboard", or has the application cut
    /// it; No and Escape keep the clipboard. Either way the focus returns to the field, its
    /// window not said again [obs, 825/s19].
    pub(super) fn handle_overwrite_event(&mut self, message: &FromUi) -> Option<String> {
        let outcome = self.overwrite_dialog.as_ref()?.handle(message);
        if outcome == Outcome::Nothing {
            return None;
        }
        let dialog = self.overwrite_dialog.take()?;
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogClose { id: dialog.id });
        }
        if dialog.window.is_some() {
            self.last_window_title.clone_from(&dialog.window);
            self.last_window_kind.clone_from(&dialog.window_kind);
        }
        if outcome != Outcome::Overwrite {
            return None;
        }
        self.clipboard_appended = false;
        if dialog.cut {
            // The application cuts, and says nothing: the field's change is read as Ctrl+X's.
            if let Some(actions) = &self.actions {
                actions.press(&[Key::Control, Key::Alpha('X')]);
            }
            self.expect_text_change(crate::editing::EditKey::Cut);
            return None;
        }
        if let Some(actions) = &self.actions {
            actions.copy_text(&dialog.selection);
        }
        let said = self.copied_message()?;
        self.speech.speak(&said, true);
        Some(said)
    }
}
