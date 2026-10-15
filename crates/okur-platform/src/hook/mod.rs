#[cfg(target_os = "macos")]
pub mod macos;
pub mod mock;
#[cfg(target_os = "windows")]
pub mod windows;

use crate::PlatformError;
pub use okur_core::input::{Key, KeyAction, KeyEvent, Modifiers};

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
    fn test_create_keyboard_hook() {
        let hook = create_keyboard_hook();
        assert!(!hook.is_active());
    }
}
