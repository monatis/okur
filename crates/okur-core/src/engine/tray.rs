//! The system tray list (Insert+F11): the platform lists the notification area's icons, Okur
//! shows them in its dialog, and the click chosen goes to the icon (JAWS 2025 help, Select a
//! System Tray Icon Dialog Box).

use super::ScreenReaderEngine;
use crate::event::TrayIcon;
use crate::ui::tray::{Outcome, TrayDialog};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// Insert+F11: the icons are asked for; the dialog opens when they come
    /// ([`ScreenReaderEngine::open_tray_list`]).
    pub(super) fn ask_tray_list(&mut self) -> Option<String> {
        self.ui.as_ref()?;
        self.tray_asked = self
            .actions
            .as_ref()
            .is_some_and(|actions| actions.list_tray_icons());
        None
    }

    /// The icons asked for came: the list opens on them.
    pub(super) fn open_tray_list(&mut self, icons: &[TrayIcon]) -> Option<String> {
        if !std::mem::take(&mut self.tray_asked) {
            return None;
        }
        let ui = self.ui.as_ref()?;
        self.dialog_count += 1;
        let dialog = TrayDialog::new(self.dialog_count, icons.to_vec());
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.tray_dialog = Some(dialog);
        None
    }

    /// Whether `message` is the system tray list's.
    pub(super) fn is_tray_event(&self, message: &FromUi) -> bool {
        self.tray_dialog
            .as_ref()
            .is_some_and(|d| Some(d.id) == super::dialog_of(message))
    }

    /// What the user did to the system tray list.
    pub(super) fn handle_tray_event(&mut self, message: &FromUi) -> Option<String> {
        match self.tray_dialog.as_mut()?.handle(message) {
            Outcome::Nothing => {}
            // The dialog closes, and the click goes to the icon: a right click's context menu
            // or what the icon opens is said as any other focus.
            Outcome::Click(icon, click) => {
                let dialog = self.tray_dialog.take()?;
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogClose { id: dialog.id });
                }
                if let Some(actions) = &self.actions {
                    actions.click_mouse(icon.x, icon.y, click);
                }
            }
            Outcome::Closed => self.tray_dialog = None,
        }
        None
    }
}
