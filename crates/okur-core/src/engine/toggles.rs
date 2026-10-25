//! Turning the quick keys and the virtual cursor off and on, refreshing the screen, and the
//! voice rate changed for a while (JAWS 2025 help: Navigation Quick Keys, Change Voice Rate
//! on the Fly) [obs, 298, 512].

use super::{AppMode, ScreenReaderEngine};
use crate::input::{Key, KeyEvent};
use crate::messages::msg;

/// What the quick keys do (Insert+N cycles through them).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum QuickKeys {
    #[default]
    On,
    Off,
    /// Only while Say All reads; until Say All takes keys, they are off.
    SayAllOnly,
}

impl ScreenReaderEngine {
    /// Insert+N: "Navigation Quick Keys", then Off, Say all only, On [obs, 298/s1, s3].
    pub(super) fn next_quick_keys_mode(&mut self) -> Option<String> {
        let (mode, word) = match self.quick_keys {
            QuickKeys::On => (QuickKeys::Off, msg::quick_keys_off()),
            QuickKeys::Off => (QuickKeys::SayAllOnly, msg::quick_keys_say_all_only()),
            QuickKeys::SayAllOnly => (QuickKeys::On, msg::quick_keys_on()),
        };
        self.quick_keys = mode;
        self.say_message(format!("{}\n{word}", msg::quick_keys_navigation()))
    }

    /// Insert+Shift+Z: "Quick Keys Suspended", and "Resuming Quick Keys", which turns them on
    /// whatever Insert+N set [obs, 298/s5-s8].
    pub(super) fn suspend_or_resume_quick_keys(&mut self) -> Option<String> {
        self.quick_keys_suspended = !self.quick_keys_suspended;
        if self.quick_keys_suspended {
            self.say_message(msg::quick_keys_suspended().to_string())
        } else {
            self.quick_keys = QuickKeys::On;
            self.say_message(msg::quick_keys_resumed().to_string())
        }
    }

    /// Insert+Z: the virtual cursor off, the page's keys the browser's; and on again, from the
    /// page's top [obs, 298/s9-s12].
    pub(super) fn toggle_virtual_cursor(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        self.virtual_cursor_off = !self.virtual_cursor_off;
        // Short messages say "Off" and "On" alone [obs, 841/s9, s11].
        let short =
            self.settings.verbosity.message_length() == crate::settings::MessageLength::Short;
        if self.virtual_cursor_off {
            let said = if short {
                msg::virtual_cursor_off_short()
            } else {
                msg::virtual_cursor_off()
            };
            self.say_message(said.to_string())
        } else {
            let _ = self.cursor.move_to_start();
            let said = if short {
                msg::virtual_cursor_on_short()
            } else {
                msg::virtual_cursor_on()
            };
            self.say_message(said.to_string())
        }
    }

    /// Insert+Escape: the page read again, and the rate back to the settings' [obs, 298/s13,
    /// 512/s7].
    pub(super) fn refresh_screen(&mut self) -> Option<String> {
        if self.rate_change != 0 {
            self.rate_change = 0;
            self.configure_voice();
        }
        if self.app_mode == AppMode::VirtualBuffer {
            if let Some(actions) = &self.actions {
                actions.refresh_page();
            }
        }
        // A message of JAWS's own, which Advanced leaves out [obs, 842/s13].
        if !self
            .settings
            .verbosity
            .is_on(crate::settings::OutputType::OkurMessage)
        {
            return None;
        }
        self.say_message(msg::screen_updated().to_string())
    }

    /// Ctrl+Alt+PageUp and PageDown: the rate one step faster or slower, said at the new rate,
    /// until Insert+Escape [obs, 512/s1-s6].
    pub(super) fn change_rate(&mut self, faster: bool) -> Option<String> {
        let base = self.settings.voice.rate;
        let rate = (base + self.rate_change + if faster { 1 } else { -1 }).clamp(-10, 10);
        self.rate_change = rate - base;
        self.configure_voice();
        let word = if faster {
            msg::rate_faster()
        } else {
            msg::rate_slower()
        };
        self.say_message(word.to_string())
    }

    /// The synthesizer set to the settings' voice with the rate changed for now.
    pub(super) fn configure_voice(&mut self) {
        let mut voice = self.settings.voice.clone();
        voice.rate = voice.rate.saturating_add(self.rate_change).clamp(-10, 10);
        self.speech.configure(&voice);
    }

    /// Whether `event` is a quick key that goes to the page: with the quick keys off or
    /// suspended, H types an H [obs, 298/s2, s4, s6].
    pub(super) fn quick_key_passes(&self, event: &KeyEvent) -> bool {
        let off = self.quick_keys_suspended || self.quick_keys != QuickKeys::On;
        let m = &event.modifiers;
        off && !m.ctrl
            && !m.alt
            && !m.insert
            && !m.win
            && matches!(
                event.key,
                Key::Alpha(_)
                    | Key::Digit(_)
                    | Key::Apostrophe
                    | Key::Semicolon
                    | Key::Minus
                    | Key::Equals
                    | Key::Slash
                    | Key::Backslash
                    | Key::Comma
                    | Key::Period
            )
    }

    pub(super) fn say(&mut self, speech: String) -> Option<String> {
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// Says one of Okur's own messages about what a key did ("Okur Cursor", "Faster"), in the
    /// Tutor and Message voice, as JAWS says its messages.
    pub(super) fn say_message(&mut self, speech: String) -> Option<String> {
        self.speech.speak_message(&speech, true);
        Some(speech)
    }
}
