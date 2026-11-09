//! Keys held for one of Okur's dialogs. Insert+F7 asks for the Links List, and okur-ui takes a
//! moment to show it and give it the focus (about 100 ms on Windows, 400 ms in the Linux lab).
//! A key pressed meanwhile went to the page: a letter meant for the list's first-letter search
//! was the page's quick key, and the dialog's first Down moved the virtual cursor (the Linux
//! lab's 222, the first Down). JAWS's lists, in its own process, are up before the next key.
//!
//! So a key pressed after a dialog was asked for, and before the focus came to Okur's windows,
//! is held, and pressed again, in order, as the dialog's focus comes: the platform sends it as
//! the screen reader's own key, and it reaches the dialog as typed. A dialog whose focus does not
//! come within [`HOLD_MS`] lets the keys go where the focus is, so no key is ever lost.

use std::sync::atomic::Ordering;

use super::{ScreenReaderEngine, OKUR_WINDOWS};
use crate::input::{Key, KeyAction, KeyEvent};

/// How long keys wait for a dialog's focus, on the key clock: past it they go where the focus
/// is. Okur's dialogs take the focus within half a second on both platforms.
const HOLD_MS: u64 = 2000;

/// The keys pressed while a dialog's focus is awaited, and until when they wait.
#[derive(Debug)]
pub(super) struct HeldForDialog {
    until_ms: u64,
    keys: Vec<KeyEvent>,
}

impl ScreenReaderEngine {
    /// A key pressed while a dialog asked for has not yet taken the focus: held for it, and
    /// `Consume` returned. The modifiers, and the Okur key with what it is held for, are
    /// handled as ever: a modifier changes nothing where it lands, and an Okur command is
    /// Okur's wherever the focus is.
    pub(super) fn hold_for_dialog(&mut self, event: &KeyEvent) -> Option<KeyAction> {
        // Without the key clock the wait could not end in time: nothing is held.
        if self.dialog_asked.swap(false, Ordering::SeqCst)
            && event.time_ms > 0
            && self.settings_place.0.as_deref() != Some(OKUR_WINDOWS)
            && self.actions.as_ref().is_some_and(|a| a.presses_keys())
        {
            // From the key that asked for the dialog: the last before this one.
            let asked_ms = self.last_key_at.map_or(event.time_ms, |(at, _)| at);
            self.held_for_dialog = Some(HeldForDialog {
                until_ms: asked_ms.saturating_add(HOLD_MS),
                keys: Vec::new(),
            });
        }
        let until_ms = self.held_for_dialog.as_ref()?.until_ms;
        if event.time_ms > until_ms {
            self.let_held_keys_go("the dialog's focus did not come");
            return None;
        }
        let modifier = matches!(
            event.key,
            Key::Control | Key::Alt | Key::Shift | Key::Windows
        ) || self.settings.keyboard.is_okur_key(event.key)
            || self.settings.keyboard.resolve(event).modifiers.insert;
        if modifier {
            return None;
        }
        tracing::debug!(key = ?event.key, "a key held for the dialog asked for");
        self.held_for_dialog.as_mut()?.keys.push(event.clone());
        Some(KeyAction::Consume)
    }

    /// The focus came to one of Okur's windows, or one of them told Okur what the user did
    /// there: the dialog asked for is up, and the keys held for it are pressed in it.
    pub(super) fn dialog_came(&mut self, event: &crate::AccessibilityEvent) {
        let up = match event {
            crate::AccessibilityEvent::Focus(focus) => focus
                .app
                .as_deref()
                .map(crate::settings_layers::application_id)
                .is_some_and(|app| app == OKUR_WINDOWS),
            crate::AccessibilityEvent::Ui(_) => true,
            _ => false,
        };
        if !up {
            return;
        }
        self.dialog_asked.store(false, Ordering::SeqCst);
        if self.held_for_dialog.is_some() {
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

    /// Presses the held keys again, in order, each with the modifiers it was pressed with.
    fn let_held_keys_go(&mut self, why: &str) {
        let Some(held) = self.held_for_dialog.take() else {
            return;
        };
        tracing::debug!(
            keys = held.keys.len(),
            why,
            "the keys held for a dialog let go"
        );
        let Some(actions) = self.actions.as_ref() else {
            return;
        };
        for event in &held.keys {
            let chord = chord_of(event);
            if !actions.press(&chord) {
                tracing::warn!(key = ?event.key, "a key held for a dialog could not be pressed");
            }
        }
    }
}

/// The keys to press for `event`: its modifiers, then the key.
fn chord_of(event: &KeyEvent) -> Vec<Key> {
    let m = &event.modifiers;
    [
        (m.ctrl, Key::Control),
        (m.alt, Key::Alt),
        (m.shift, Key::Shift),
        (m.win, Key::Windows),
    ]
    .into_iter()
    .filter_map(|(held, key)| held.then_some(key))
    .chain(std::iter::once(event.key))
    .collect()
}
