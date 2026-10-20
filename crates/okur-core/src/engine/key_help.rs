//! Keyboard Help (Insert+1): while it is on, a key is described instead of run: its name in
//! JAWS's key notation, then what it does in Okur's own words, more when pressed twice quickly;
//! a key Okur does not use goes to the application. Insert+1 again turns it off. JAWS says
//! "Keyboard Help on", "Insert+UpArrow | Says the current line." and "Tab | Sends the tab key
//! through to the application." [obs, 509 and 634]; its descriptions are Freedom Scientific's
//! writing, and Okur's are its own ([`crate::keyboard_help`]).

use super::{Press, ScreenReaderEngine};
use crate::command::Command;
use crate::input::{Key, KeyAction, KeyEvent};
use crate::messages::msg;

impl ScreenReaderEngine {
    /// Insert+1: Keyboard Help on, or off.
    pub(super) fn toggle_keyboard_help(&mut self) -> Option<String> {
        self.keyboard_help = !self.keyboard_help;
        let said = if self.keyboard_help {
            msg::keyboard_help_on()
        } else {
            msg::keyboard_help_off()
        }
        .to_string();
        self.speech.speak(&said, true);
        Some(said)
    }

    /// A key pressed with Keyboard Help on (`event` resolved for the Okur key): described and
    /// kept from the application, or `None` for what is handled as usual: a modifier pressed
    /// alone, and Insert+1, which turns Keyboard Help off.
    pub(super) fn describe_key(&mut self, event: &KeyEvent) -> Option<KeyAction> {
        let modifier = matches!(
            event.key,
            Key::Control
                | Key::Alt
                | Key::Shift
                | Key::Windows
                | Key::Insert
                | Key::CapsLock
                | Key::ScrollLock
        );
        if modifier {
            return None;
        }
        if !event.is_down {
            return Some(KeyAction::Consume);
        }
        let command = crate::command::layout_command_for(&self.settings.keyboard, event)
            .or_else(|| crate::command::command_for(event));
        if command == Some(Command::KeyboardHelp) {
            return None;
        }
        // Tab and Shift+Tab are the application's outside a page's virtual cursor: in a dialog
        // JAWS says Tab goes to the application [obs, 509/s4].
        let in_page = self.app_mode == super::AppMode::VirtualBuffer && !self.forms_mode;
        let command = command
            .filter(|c| in_page || !matches!(c, Command::NextTabStop | Command::PriorTabStop));
        let press = Press::Described(event.key, event.modifiers);
        let twice = self.pressed_twice(press, event.time_ms);
        let what = match &command {
            Some(command) => crate::keyboard_help::description(command, twice),
            None => crate::keyboard_help::passes(),
        };
        let said = [crate::keyboard_help::key_name(event), what].join("\n");
        self.speech.speak(&said, true);
        Some(KeyAction::Consume)
    }
}
