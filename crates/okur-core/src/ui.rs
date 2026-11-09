//! Okur's own windows, drawn by `okur-ui` (see `docs/references/helper-ui-architecture.md`).
//! The engine says what to show through a [`UiHost`]; what the user does to a window comes
//! back as [`crate::event::AccessibilityEvent::Ui`].

use std::sync::{Arc, Mutex};

pub mod applications;
pub mod basics;
pub mod clipboard;
pub mod command_search;
pub mod dictionary;
pub mod find;
pub mod flexible_web;
pub mod jump;
pub mod key_labels;
pub mod label;
pub mod lists;
pub mod message_box;
pub mod notifications;
pub mod placemarkers;
pub mod quit;
pub mod settings;
pub mod settings_tree;
pub mod skim;
pub mod startup_wizard;
pub mod tray;
pub mod voice_adjustment;
pub mod voice_profile;
pub mod web_verbosity;
pub mod windows;

pub use okur_ui_model::{
    Control, ControlKind, DialogKey, DialogSpec, FromUi, MainWindowSpec, MenuItemSpec, MenuSpec,
    TextSpan, ToUi, TreeItem, Value, WizardButtons, WizardLabels, WizardSpec, MAIN_WINDOW_EXIT,
};

/// A wizard's buttons' labels from the catalog, for a toolkit that draws them itself.
pub fn wizard_labels() -> WizardLabels {
    use crate::messages::msg;
    WizardLabels {
        back: msg::wizard_back().to_string(),
        next: msg::wizard_next().to_string(),
        finish: msg::wizard_finish().to_string(),
        cancel: msg::wizard_cancel().to_string(),
    }
}

/// Where the engine sends what its windows show. Sending never waits for the window.
pub trait UiHost: Send {
    fn send(&self, message: ToUi);
}

/// A host that keeps what it was sent, for tests.
#[derive(Debug, Clone, Default)]
pub struct RecordingUiHost {
    sent: Arc<Mutex<Vec<ToUi>>>,
}

impl RecordingUiHost {
    /// The messages sent so far, emptied.
    pub fn take(&self) -> Vec<ToUi> {
        self.sent
            .lock()
            .map(|mut sent| std::mem::take(&mut *sent))
            .unwrap_or_default()
    }
}

impl UiHost for RecordingUiHost {
    fn send(&self, message: ToUi) {
        if let Ok(mut sent) = self.sent.lock() {
            sent.push(message);
        }
    }
}
