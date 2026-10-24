//! Okur's own windows, drawn by `okur-ui` (see `docs/references/helper-ui-architecture.md`).
//! The engine says what to show through a [`UiHost`]; what the user does to a window comes
//! back as [`crate::event::AccessibilityEvent::Ui`].

use std::sync::{Arc, Mutex};

pub mod applications;
pub mod basics;
pub mod clipboard;
pub mod find;
pub mod flexible_web;
pub mod jump;
pub mod label;
pub mod lists;
pub mod message_box;
pub mod notifications;
pub mod placemarkers;
pub mod quit;
pub mod settings;
pub mod settings_tree;
pub mod skim;
pub mod tray;
pub mod voice_adjustment;
pub mod voice_profile;
pub mod windows;

pub use okur_ui_model::{
    Control, ControlKind, DialogKey, DialogSpec, FromUi, MainWindowSpec, MenuItemSpec, MenuSpec,
    TextSpan, ToUi, Value, WizardButtons, WizardSpec, MAIN_WINDOW_EXIT,
};

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
