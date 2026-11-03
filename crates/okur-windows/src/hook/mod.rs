pub mod mock;
#[cfg(target_os = "windows")]
pub mod windows;

use crate::PlatformError;
use okur_core::input::{KeyAction, KeyEvent};

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

/// The keyboard hook: Windows' low-level hook on Windows, the headless mock elsewhere (the
/// macOS driver will be a crate of its own, `okur-macos`).
pub fn create_keyboard_hook() -> Box<dyn KeyboardHook> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsKeyboardHook::new())
    }
    #[cfg(not(target_os = "windows"))]
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
