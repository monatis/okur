//! The Window List (Insert+F10): the platform lists the applications' windows, Okur shows them
//! in its dialog, and Switch To brings the one chosen to the front (JAWS 2025 help, Window List
//! Dialog Box). Delete closes the one chosen: JAWS "switches to the application which will then
//! prompt the user to confirm exit or save any unsaved data", and "after responding to the close
//! confirmation JAWS will re-focus back onto the task list again" (Sight and Sound's article
//! "Quit applications from JAWS task list dialogue", JAWS 2024.2403 and later).

use super::ScreenReaderEngine;
use crate::event::{FocusEvent, TopWindow};
use crate::ui::windows::{Outcome, WindowListDialog};
use crate::ui::{FromUi, ToUi};

/// The window the Window List's Delete asked to close, until the list comes back.
#[derive(Debug)]
pub(super) struct ClosingWindow {
    id: u64,
    title: String,
    /// The focus has left the window: its application's question, or another window once
    /// it closed.
    left: bool,
    /// The focus came back to the window after it left: its application kept it (Cancel to
    /// its question), and the list comes back all the same.
    answered: bool,
}

impl ScreenReaderEngine {
    /// A focus while a window closes from the Window List: once it has left that window, the
    /// windows are asked for, and the list comes back when the window is gone, or when the
    /// focus has come back to it (its application's question answered). Okur's own windows
    /// do not count: the list itself closing.
    pub(super) fn window_list_after_focus(&mut self, focus: &FocusEvent) {
        let Some(closing) = self.window_closing.as_mut() else {
            return;
        };
        let own = focus
            .app
            .as_deref()
            .map(crate::settings_layers::application_id)
            .is_some_and(|app| app == super::OKUR_WINDOWS);
        if own {
            return;
        }
        let in_window = focus.window_title.as_deref() == Some(closing.title.as_str());
        match (in_window, closing.left) {
            (true, false) => return,
            (true, true) => closing.answered = true,
            (false, _) => closing.left = true,
        }
        self.window_list_asked = self
            .actions
            .as_ref()
            .is_some_and(|actions| actions.list_windows());
    }

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
        // Back after Delete: once the window is gone, or its question answered; else the
        // next focus asks again.
        if let Some(closing) = &self.window_closing {
            let gone = windows.iter().all(|w| w.id != closing.id);
            if !gone && !closing.answered {
                return None;
            }
            self.window_closing = None;
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
            // Delete: the window chosen comes to the front and is asked to close; the list
            // comes back once it is gone or its application's question is answered.
            Outcome::Close(id, title) => {
                let dialog = self.window_list.take()?;
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogClose { id: dialog.id });
                }
                let closing = self
                    .actions
                    .as_ref()
                    .is_some_and(|actions| actions.close_window(id));
                self.window_closing = closing.then_some(ClosingWindow {
                    id,
                    title,
                    left: false,
                    answered: false,
                });
            }
            // JAWS announces the window the focus goes back to: "SR Dialog Fixture | Save...
            // Button" [obs, 511/s3].
            Outcome::Closed => self.window_list = None,
        }
        None
    }
}
