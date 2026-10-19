//! JAWS Find in the engine. The dialog opens over the page; Find closes it, moves the virtual
//! cursor to the match and reads its line. The history is kept per site while Context
//! Specific History is checked, as JAWS keeps it
//! (`docs/references/jaws-dialogs-and-visual-features.md`); Okur keeps it while it runs.

use super::{AppMode, ScreenReaderEngine};
use crate::ui::find::{FindDialog, Outcome};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// Opens JAWS Find over the page (Ctrl+F, Insert+Ctrl+F), or brings it to the front. Okur
    /// says nothing itself: it reads the dialog as the dialog takes the focus.
    pub(super) fn open_find(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let ui = self.ui.as_ref()?;
        let dialog = match self.find_dialog.take() {
            Some(dialog) => dialog,
            None => {
                self.dialog_count += 1;
                let history = self
                    .find_histories
                    .get(&self.find_history_key())
                    .cloned()
                    .unwrap_or_default();
                FindDialog::new(self.dialog_count, &history, self.find_options)
            }
        };
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.find_dialog = Some(dialog);
        None
    }

    /// Whose history the dialog shows: the page's site, or everyone's.
    fn find_history_key(&self) -> String {
        if !self.find_options.context_specific {
            return String::new();
        }
        self.cursor
            .buffer
            .document_url()
            .and_then(crate::settings_layers::site_of)
            .unwrap_or_default()
    }

    /// Whether `message` is JAWS Find's.
    pub(super) fn is_find_event(&self, message: &FromUi) -> bool {
        self.find_dialog
            .as_ref()
            .is_some_and(|d| Some(d.id) == super::dialog_of(message))
    }

    /// What the user did to JAWS Find.
    pub(super) fn handle_find_event(&mut self, message: &FromUi) -> Option<String> {
        let outcome = self.find_dialog.as_mut()?.handle(message);
        match outcome {
            Outcome::Nothing => None,
            Outcome::Send(messages) => {
                if let Some(ui) = &self.ui {
                    messages.into_iter().for_each(|m| ui.send(m));
                }
                None
            }
            Outcome::History => {
                let dialog = self.find_dialog.take()?;
                self.keep_find(&dialog);
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogItems {
                        id: dialog.id,
                        control: "what".to_owned(),
                        items: dialog.history().to_vec(),
                        selected: 0,
                    });
                }
                self.find_dialog = Some(dialog);
                None
            }
            Outcome::Search(search) => {
                let dialog = self.find_dialog.take()?;
                self.keep_find(&dialog);
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogClose { id: dialog.id });
                }
                self.returning_from_dialog();
                let case_sensitive = !search.ignore_case;
                let speech = if search.forward {
                    self.cursor.find_text(&search.text, case_sensitive)
                } else {
                    self.cursor.find_text_backward(&search.text, case_sensitive)
                }?;
                self.speech.speak(&speech, true);
                Some(speech)
            }
            Outcome::Closed => {
                if let Some(dialog) = self.find_dialog.take() {
                    self.keep_find(&dialog);
                }
                self.returning_from_dialog();
                None
            }
        }
    }

    /// Keeps the dialog's options and history for its next opening.
    fn keep_find(&mut self, dialog: &FindDialog) {
        self.find_options = dialog.options();
        let key = self.find_history_key();
        self.find_histories.insert(key, dialog.history().to_vec());
    }
}

#[cfg(test)]
mod tests {
    use crate::aom::{AomNode, Role};
    use crate::buffer::VirtualBuffer;
    use crate::cursor::VirtualCursor;
    use crate::engine::ScreenReaderEngine;
    use crate::event::AccessibilityEvent;
    use crate::input::{Key, KeyEvent, Modifiers};
    use crate::speech::MockSpeechOutput;
    use crate::ui::{FromUi, RecordingUiHost, ToUi, Value};

    fn engine() -> (ScreenReaderEngine, RecordingUiHost) {
        let mut page = AomNode::new(0, Role::Document).with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("The library opens late."),
            AomNode::new(2, Role::Paragraph).with_text("The board met on Tuesday."),
            AomNode::new(3, Role::Paragraph).with_text("The board voted."),
        ]);
        page.attributes
            .insert("url".to_owned(), "https://news.example.org/a".to_owned());
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&page)),
            Box::new(MockSpeechOutput::new()),
        );
        engine.cursor.move_to_start();
        let ui = RecordingUiHost::default();
        engine.set_ui(Box::new(ui.clone()));
        (engine, ui)
    }

    fn ctrl_f(engine: &mut ScreenReaderEngine) {
        let mut ctrl = Modifiers::none();
        ctrl.ctrl = true;
        let _ = engine.handle_key_event(&KeyEvent::new(Key::Alpha('F'), ctrl, true));
    }

    fn send(engine: &mut ScreenReaderEngine, message: FromUi) -> Option<String> {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(message))
    }

    #[test]
    fn ctrl_f_opens_jaws_find_and_find_moves_to_the_match() {
        let (mut engine, ui) = engine();
        ctrl_f(&mut engine);
        let shown = ui.take();
        let dialog = match shown.first() {
            Some(ToUi::DialogShow { dialog }) => dialog.clone(),
            other => {
                assert!(
                    matches!(other, Some(ToUi::DialogShow { .. })),
                    "JAWS Find did not open: {shown:?}"
                );
                return;
            }
        };
        assert_eq!(dialog.title, "JAWS Find");
        let _ = send(
            &mut engine,
            FromUi::DialogChanged {
                id: dialog.id,
                control: "what".to_owned(),
                value: Value::Text("board".to_owned()),
            },
        );
        let said = send(
            &mut engine,
            FromUi::DialogPressed {
                id: dialog.id,
                control: "find".to_owned(),
            },
        );
        assert_eq!(said.as_deref(), Some("The board met on Tuesday."));
        assert!(matches!(
            ui.take().first(),
            Some(ToUi::DialogClose { id }) if *id == dialog.id
        ));
        // F3 finds the next one; the next opening shows the search in its field.
        let _ = engine.handle_key_event(&KeyEvent::new(Key::F(3), Modifiers::none(), true));
        assert_eq!(engine.cursor.current_line(), "The board voted.");
        ctrl_f(&mut engine);
        let shown = ui.take();
        let dialog = match shown.first() {
            Some(ToUi::DialogShow { dialog }) => dialog.clone(),
            other => {
                assert!(
                    matches!(other, Some(ToUi::DialogShow { .. })),
                    "JAWS Find did not open again: {shown:?}"
                );
                return;
            }
        };
        assert!(matches!(
            dialog.controls.first().map(|c| &c.kind),
            Some(crate::ui::ControlKind::ComboEdit { text, history })
                if text == "board" && history == &["board".to_owned()]
        ));
    }
}
