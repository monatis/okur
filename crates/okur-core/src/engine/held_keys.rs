//! Keys held for one of Okur's dialogs. Insert+F7 asks for the Links List, and okur-ui takes a
//! moment to show it and give it the focus (about 100 ms on Windows, 400 ms in the Linux lab).
//! A key pressed meanwhile went to the page: a letter meant for the list's first-letter search
//! was the page's quick key, and the dialog's first Down moved the virtual cursor (the Linux
//! lab's 222, the first Down). JAWS's lists, in its own process, are up before the next key.
//!
//! So a key pressed after a dialog was asked for, and before the focus came to it, is held,
//! and pressed again, in order, as the dialog's focus comes (or the dialog tells what the user
//! did in it): the platform sends it as the screen reader's own key, by its native code, and
//! it reaches the dialog as typed. A dialog whose focus does not come within [`HOLD_MS`] lets
//! the keys go where the focus is, the key that found the wait over after them: no key is lost
//! or put out of order.

use super::{ScreenReaderEngine, OKUR_WINDOWS};
use crate::input::{Key, KeyAction, KeyEvent};

/// How long keys wait for a dialog's focus, on the key clock: past it they go where the focus
/// is. Okur's dialogs take the focus within half a second on both platforms.
const HOLD_MS: u64 = 2000;

/// A dialog asked to show: its id and its title, as its window has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AskedDialog {
    pub(super) id: u32,
    pub(super) title: String,
}

/// The keys pressed while a dialog's focus is awaited, and until when they wait.
#[derive(Debug)]
pub(super) struct HeldForDialog {
    dialog: AskedDialog,
    until_ms: u64,
    keys: Vec<KeyEvent>,
}

impl ScreenReaderEngine {
    /// A key pressed while a dialog asked for has not yet taken the focus: held for it, and
    /// `Consume` returned. The modifiers, the toggles, and the Okur key with what it is held
    /// for, are handled as ever: a modifier changes nothing where it lands, and an Okur command
    /// is Okur's wherever the focus is.
    pub(super) fn hold_for_dialog(&mut self, event: &KeyEvent) -> Option<KeyAction> {
        let asked = self
            .dialog_asked
            .lock()
            .ok()
            .and_then(|mut asked| asked.take());
        // Without the key clock the wait could not end in time: nothing is held.
        if let Some(dialog) = asked.filter(|_| {
            event.time_ms > 0
                && self.settings_place.0.as_deref() != Some(OKUR_WINDOWS)
                && self.actions.as_ref().is_some_and(|a| a.presses_keys())
        }) {
            // From the key that asked for the dialog: the last before this one.
            let until_ms = self
                .last_key_at
                .map_or(event.time_ms, |(at, _)| at)
                .saturating_add(HOLD_MS);
            match &mut self.held_for_dialog {
                // Another dialog asked for over the first: the keys held wait on for it.
                Some(held) => {
                    held.dialog = dialog;
                    held.until_ms = until_ms;
                }
                None => {
                    self.held_for_dialog = Some(HeldForDialog {
                        dialog,
                        until_ms,
                        keys: Vec::new(),
                    });
                }
            }
        }
        let until_ms = self.held_for_dialog.as_ref()?.until_ms;
        let over = event.time_ms > until_ms;
        let modifier = matches!(
            event.key,
            Key::Control | Key::Alt | Key::Shift | Key::Windows | Key::CapsLock | Key::ScrollLock
        ) || self.settings.keyboard.is_okur_key(event.key)
            || self.settings.keyboard.resolve(event).modifiers.insert;
        if modifier {
            if over {
                self.let_held_keys_go("the dialog's focus did not come");
            }
            return None;
        }
        tracing::debug!(key = ?event.key, "a key held for the dialog asked for");
        self.held_for_dialog.as_mut()?.keys.push(event.clone());
        if over {
            // The keys held go first, this one after them.
            self.let_held_keys_go("the dialog's focus did not come");
        }
        Some(KeyAction::Consume)
    }

    /// The dialog asked for is up: its window has the focus, or it told what the user did in
    /// it. The keys held for it are pressed in it. Another of Okur's windows (the dialog just
    /// closed, its late focus or its close) is not it.
    pub(super) fn dialog_came(&mut self, event: &crate::AccessibilityEvent) {
        let is = |dialog: &AskedDialog| match event {
            crate::AccessibilityEvent::Focus(focus) => {
                focus
                    .app
                    .as_deref()
                    .map(crate::settings_layers::application_id)
                    .is_some_and(|app| app == OKUR_WINDOWS)
                    && focus.window_title.as_deref() == Some(dialog.title.as_str())
            }
            crate::AccessibilityEvent::Ui(message) => {
                !matches!(message, crate::ui::FromUi::DialogClosed { .. })
                    && super::dialog_of(message) == Some(dialog.id)
            }
            _ => false,
        };
        if let Ok(mut asked) = self.dialog_asked.lock() {
            if asked.as_ref().is_some_and(is) {
                *asked = None;
            }
        }
        if self.held_for_dialog.as_ref().is_some_and(|h| is(&h.dialog)) {
            self.let_held_keys_go("the dialog's focus came");
        }
    }

    /// The keys held past [`HOLD_MS`] at `now_ms` on the key clock, with no key after them:
    /// let go where the focus is.
    pub(super) fn held_keys_due(&mut self, now_ms: u64) {
        if self
            .held_for_dialog
            .as_ref()
            .is_some_and(|held| now_ms > held.until_ms)
        {
            self.let_held_keys_go("the dialog's focus did not come");
        }
    }

    /// Presses the held keys again, in order, each as it was pressed.
    fn let_held_keys_go(&mut self, why: &str) {
        let Some(held) = self.held_for_dialog.take() else {
            return;
        };
        tracing::debug!(
            keys = held.keys.len(),
            why,
            "the keys held for a dialog let go"
        );
        let pressed = self
            .actions
            .as_ref()
            .is_some_and(|actions| actions.press_again(&held.keys));
        if !pressed {
            tracing::warn!(
                keys = held.keys.len(),
                "keys held for a dialog could not all be pressed"
            );
        }
    }
}
