//! Insert+F4 in the engine (Default.JKM: `JAWSKey+F4=ShutDownJAWS`). With `confirm_exit` on,
//! JAWS's default, the question opens; OK asks the host to end Okur, Cancel or Escape closes
//! it and the focus returns quietly, as from Okur's other dialogs. With it off, or with no
//! window to ask in, the host is asked at once.
//!
//! The engine never ends the process: the host (`okur run`, `okur-sr.exe`) does, down its one
//! exit path (`crates/okur/src/run.rs`), with the keyboard unhooked first so that no key is
//! held back as Okur goes (`docs/references/startup-and-secure-screens-proposal.md`, §5.9).

use super::ScreenReaderEngine;
use crate::messages::msg;
use crate::ui::quit::{Outcome, QuitDialog};
use crate::ui::{FromUi, ToUi};

/// How the engine asks its host to end Okur.
pub type ExitRequest = Box<dyn Fn() + Send>;

impl ScreenReaderEngine {
    /// Gives the engine the way to end Okur. Without it (headless, replay) Insert+F4 does
    /// nothing.
    pub fn set_exit(&mut self, exit: ExitRequest) {
        self.exit = Some(exit);
    }

    /// What Okur says as it exits.
    pub fn exit_words(&self) -> String {
        msg::okur_exiting().to_string()
    }

    /// Insert+F4: the question, or the exit at once. Okur says nothing itself: the question is
    /// read as it takes the focus.
    pub(super) fn shut_down(&mut self) -> Option<String> {
        self.exit.as_ref()?;
        let ui = match &self.ui {
            Some(ui) if self.settings.confirm_exit => ui,
            _ => {
                self.request_exit();
                return None;
            }
        };
        // Asked again while it is open: the question comes to the front.
        let dialog = match self.quit_dialog.take() {
            Some(dialog) => dialog,
            None => {
                self.dialog_count += 1;
                QuitDialog::new(self.dialog_count)
            }
        };
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.quit_dialog = Some(dialog);
        None
    }

    /// Whether `message` is the question's.
    pub(super) fn is_quit_event(&self, message: &FromUi) -> bool {
        self.quit_dialog
            .as_ref()
            .is_some_and(|d| Some(d.id) == super::dialog_of(message))
    }

    /// What the user did to the question.
    pub(super) fn handle_quit_event(&mut self, message: &FromUi) -> Option<String> {
        match self.quit_dialog.as_ref()?.handle(message) {
            Outcome::Nothing => {}
            Outcome::Exit => {
                if let Some(dialog) = self.quit_dialog.take() {
                    if let Some(ui) = &self.ui {
                        ui.send(ToUi::DialogClose { id: dialog.id });
                    }
                }
                self.request_exit();
            }
            // JAWS's own dialogs give the focus back without a word (JAWS Find [obs, 506]);
            // what JAWS says as its question closes is scenario 571's step s3.
            Outcome::Closed => {
                self.quit_dialog = None;
                self.returning_from_dialog();
            }
        }
        None
    }

    fn request_exit(&self) {
        if let Some(exit) = &self.exit {
            tracing::info!("Okur is asked to exit");
            exit();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::aom::{AomNode, Role};
    use crate::event::AccessibilityEvent;
    use crate::input::{Key, KeyEvent, Modifiers};
    use crate::ui::{FromUi, RecordingUiHost, ToUi};
    use crate::{MockSpeechOutput, ScreenReaderEngine, VirtualBuffer, VirtualCursor};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// An engine with a window host and an exit that counts its calls.
    fn engine() -> (ScreenReaderEngine, RecordingUiHost, Arc<AtomicU32>) {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        let ui = RecordingUiHost::default();
        engine.set_ui(Box::new(ui.clone()));
        let exits = Arc::new(AtomicU32::new(0));
        let counted = Arc::clone(&exits);
        engine.set_exit(Box::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
        }));
        (engine, ui, exits)
    }

    fn insert_f4() -> KeyEvent {
        let mut modifiers = Modifiers::none();
        modifiers.insert = true;
        KeyEvent::new(Key::F(4), modifiers, true)
    }

    /// The question's id, from what was sent to the window host.
    fn shown(ui: &RecordingUiHost) -> Option<u32> {
        ui.take().into_iter().find_map(|message| match message {
            ToUi::DialogShow { dialog } if dialog.title == "Exit Okur" => Some(dialog.id),
            _ => None,
        })
    }

    #[test]
    fn insert_f4_asks_first_and_ok_exits() {
        let (mut engine, ui, exits) = engine();
        let _ = engine.handle_key_event(&insert_f4());
        let id = shown(&ui);
        assert!(id.is_some());
        assert_eq!(exits.load(Ordering::SeqCst), 0, "not before the answer");
        let id = id.unwrap_or_default();
        let _ = engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
            id,
            control: "ok".into(),
        }));
        assert_eq!(exits.load(Ordering::SeqCst), 1);
        assert!(ui.take().contains(&ToUi::DialogClose { id }));
    }

    #[test]
    fn escape_keeps_okur_running() {
        let (mut engine, ui, exits) = engine();
        let _ = engine.handle_key_event(&insert_f4());
        let id = shown(&ui).unwrap_or_default();
        let _ =
            engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogClosed { id }));
        assert_eq!(exits.load(Ordering::SeqCst), 0);
        // Asked again, a new question opens.
        let _ = engine.handle_key_event(&insert_f4());
        assert!(shown(&ui).is_some_and(|again| again != id));
    }

    /// "When cleared, JAWS immediately closes when you use any of the commands to exit the
    /// program including INSERT+F4" (JAWS 2025 help).
    #[test]
    fn without_confirmation_insert_f4_exits_at_once() {
        let (mut engine, ui, exits) = engine();
        let mut settings = engine.settings.clone();
        settings.confirm_exit = false;
        engine.set_settings(settings);
        let _ = engine.handle_key_event(&insert_f4());
        assert_eq!(exits.load(Ordering::SeqCst), 1);
        assert_eq!(shown(&ui), None);
    }

    /// Headless, as in replay, nothing ends.
    #[test]
    fn without_a_host_insert_f4_does_nothing() {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        let ui = RecordingUiHost::default();
        engine.set_ui(Box::new(ui.clone()));
        let _ = engine.handle_key_event(&insert_f4());
        assert_eq!(shown(&ui), None);
        assert_eq!(engine.exit_words(), "Okur exiting.");
    }
}
