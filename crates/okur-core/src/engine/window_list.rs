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

/// How long the Window List waits to come back after Delete: past it, a question of the
/// application's left unanswered while the user works elsewhere brings no list up later (the
/// code review of 2026-11-10, second pass, 1).
const CLOSING_WAIT: std::time::Duration = std::time::Duration::from_secs(60);

/// The window the Window List's Delete asked to close, until the list comes back.
#[derive(Debug)]
pub(super) struct ClosingWindow {
    id: u64,
    title: String,
    /// When it was asked to close.
    asked: std::time::Instant,
    /// The focus has been in the window since: the list's own return of the focus to the
    /// window it was opened over comes first, and does not count as the window left.
    reached: bool,
    /// The focus has left the window after it was there: its application's question, or
    /// another window once it closed.
    away: bool,
    /// The focus came back to the window after it was away: its application kept it (Cancel
    /// to its question), and the list comes back all the same.
    answered: bool,
}

impl ScreenReaderEngine {
    /// A focus while a window closes from the Window List: once it has left that window, the
    /// windows are asked for, and the list comes back when the window is gone, or when the
    /// focus has come back to it (its application's question answered). Okur's own windows
    /// do not count: the list itself closing.
    pub(super) fn window_list_after_focus(&mut self, focus: &FocusEvent) {
        if self
            .window_closing
            .as_ref()
            .is_some_and(|closing| closing.asked.elapsed() > CLOSING_WAIT)
        {
            self.window_closing = None;
        }
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
        // By its title: Windows' focus names no window handle. A twin of the same title is
        // taken for it.
        let in_window = focus.window_title.as_deref() == Some(closing.title.as_str());
        match (in_window, closing.reached, closing.away) {
            (true, false, _) => {
                closing.reached = true;
                return;
            }
            (true, true, false) => return,
            (true, true, true) => closing.answered = true,
            (false, true, _) => closing.away = true,
            // Not yet there: the list's own return of the focus. The windows are asked all
            // the same, for a window that closed before its focus was told.
            (false, false, _) => {}
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
        // The user's own Insert+F10 ends any wait after Delete: this list is theirs.
        self.window_closing = None;
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
            // Delete: the window chosen comes to the front and is asked to close, before the
            // list goes (okur-ui gives the focus back as it closes, racing the switch); the
            // list comes back once it is gone or its application's question is answered.
            Outcome::Close(id, title) => {
                let dialog = self.window_list.take()?;
                let closing = self
                    .actions
                    .as_ref()
                    .is_some_and(|actions| actions.close_window(id));
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogClose { id: dialog.id });
                }
                self.window_closing = closing.then(|| ClosingWindow {
                    id,
                    title,
                    asked: std::time::Instant::now(),
                    reached: false,
                    away: false,
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
