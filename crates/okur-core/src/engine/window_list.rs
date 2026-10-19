//! The Window List (Insert+F10): the platform lists the applications' windows, Okur shows them
//! in its dialog, and Switch To brings the one chosen to the front (JAWS 2025 help, Window List
//! Dialog Box).

use super::ScreenReaderEngine;
use crate::event::TopWindow;
use crate::ui::windows::{Outcome, WindowListDialog};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// Insert+F10: the windows are asked for; the dialog opens when they come
    /// ([`ScreenReaderEngine::open_window_list`]).
    pub(super) fn ask_window_list(&mut self) -> Option<String> {
        self.ui.as_ref()?;
        self.window_list_asked = self
            .actions
            .as_ref()
            .is_some_and(|actions| actions.list_windows());
        None
    }

    /// The windows asked for came: the Window List opens on them.
    pub(super) fn open_window_list(&mut self, windows: &[TopWindow]) -> Option<String> {
        if !std::mem::take(&mut self.window_list_asked) {
            return None;
        }
        let ui = self.ui.as_ref()?;
        self.dialog_count += 1;
        let dialog = WindowListDialog::new(self.dialog_count, windows.to_vec());
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.window_list = Some(dialog);
        None
    }

    /// Whether `message` is the Window List's.
    pub(super) fn is_window_list_event(&self, message: &FromUi) -> bool {
        self.window_list
            .as_ref()
            .is_some_and(|d| Some(d.id) == super::dialog_of(message))
    }

    /// What the user did to the Window List.
    pub(super) fn handle_window_list_event(&mut self, message: &FromUi) -> Option<String> {
        match self.window_list.as_mut()?.handle(message) {
            Outcome::Nothing => {}
            Outcome::Send(messages) => {
                if let Some(ui) = &self.ui {
                    messages.into_iter().for_each(|m| ui.send(m));
                }
            }
            // The window chosen comes to the front, and its focus is said as any other.
            Outcome::SwitchTo(id) => {
                let dialog = self.window_list.take()?;
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogClose { id: dialog.id });
                }
                if let Some(actions) = &self.actions {
                    actions.switch_to_window(id);
                }
            }
            // JAWS announces the window the focus goes back to: "SR Dialog Fixture | Save...
            // Button" [obs, 511/s3].
            Outcome::Closed => self.window_list = None,
        }
        None
    }
}
