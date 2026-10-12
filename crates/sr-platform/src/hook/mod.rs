#[cfg(target_os = "macos")]
pub mod macos;
pub mod mock;
#[cfg(target_os = "windows")]
pub mod windows;

use crate::PlatformError;
pub use sr_core::input::{Key, KeyAction, KeyEvent, Modifiers};

/// Type alias for keyboard hook event callbacks.
pub type HookCallback = Box<dyn FnMut(KeyEvent) -> KeyAction + Send>;

/// Low-level keyboard hook abstraction.
pub trait KeyboardHook: Send + Sync {
    /// Starts intercepting keyboard events, invoking the given callback for each event.
    fn start(&mut self, callback: HookCallback) -> Result<(), PlatformError>;

    /// Stops intercepting keyboard events and removes OS hooks.
    fn stop(&mut self) -> Result<(), PlatformError>;

    /// Returns whether the hook is currently listening.
    fn is_active(&self) -> bool;
}

/// Creates the platform-appropriate keyboard hook implementation.
pub fn create_keyboard_hook() -> Box<dyn KeyboardHook> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsKeyboardHook::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacKeyboardHook::new())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Box::new(mock::MockKeyboardHook::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_event_to_command_string_translations() {
        // Arrow navigation
        assert_eq!(
            KeyEvent::new(Key::DownArrow, Modifiers::none(), true).to_command_string(),
            Some("{DOWN}".to_string())
        );
        assert_eq!(
            KeyEvent::new(Key::UpArrow, Modifiers::none(), true).to_command_string(),
            Some("{UP}".to_string())
        );
        assert_eq!(
            KeyEvent::new(
                Key::RightArrow,
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                true
            )
            .to_command_string(),
            Some("^{RIGHT}".to_string())
        );
        assert_eq!(
            KeyEvent::new(
                Key::LeftArrow,
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                true
            )
            .to_command_string(),
            Some("^{LEFT}".to_string())
        );
        assert_eq!(
            KeyEvent::new(
                Key::RightArrow,
                Modifiers {
                    ctrl: true,
                    alt: true,
                    ..Default::default()
                },
                true
            )
            .to_command_string(),
            Some("^%{RIGHT}".to_string())
        );

        // Document boundary navigation
        assert_eq!(
            KeyEvent::new(
                Key::Home,
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                true
            )
            .to_command_string(),
            Some("^{HOME}".to_string())
        );

        // Quick nav keys
        assert_eq!(
            KeyEvent::new(Key::Alpha('h'), Modifiers::none(), true).to_command_string(),
            Some("H".to_string())
        );
        assert_eq!(
            KeyEvent::new(
                Key::Alpha('h'),
                Modifiers {
                    shift: true,
                    ..Default::default()
                },
                true
            )
            .to_command_string(),
            Some("+H".to_string())
        );
        assert_eq!(
            KeyEvent::new(Key::Digit(2), Modifiers::none(), true).to_command_string(),
            Some("2".to_string())
        );
        assert_eq!(
            KeyEvent::new(
                Key::Digit(2),
                Modifiers {
                    shift: true,
                    ..Default::default()
                },
                true
            )
            .to_command_string(),
            Some("+2".to_string())
        );

        // Interactive keys
        assert_eq!(
            KeyEvent::new(Key::Tab, Modifiers::none(), true).to_command_string(),
            Some("{TAB}".to_string())
        );
        assert_eq!(
            KeyEvent::new(
                Key::Tab,
                Modifiers {
                    shift: true,
                    ..Default::default()
                },
                true
            )
            .to_command_string(),
            Some("+{TAB}".to_string())
        );
        assert_eq!(
            KeyEvent::new(Key::Enter, Modifiers::none(), true).to_command_string(),
            Some("{ENTER}".to_string())
        );
        assert_eq!(
            KeyEvent::new(Key::Escape, Modifiers::none(), true).to_command_string(),
            Some("{ESC}".to_string())
        );
        assert_eq!(
            KeyEvent::new(Key::Space, Modifiers::none(), true).to_command_string(),
            Some("{SPACE}".to_string())
        );

        // Key up produces no command
        assert_eq!(
            KeyEvent::new(Key::DownArrow, Modifiers::none(), false).to_command_string(),
            None
        );
    }

    #[test]
    fn test_create_keyboard_hook() {
        let hook = create_keyboard_hook();
        assert!(!hook.is_active());
    }
}
