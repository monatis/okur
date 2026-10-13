#[cfg(target_os = "macos")]
pub mod macos;
pub mod mock;
#[cfg(target_os = "windows")]
pub mod windows;

use crate::PlatformError;
pub use sr_core::event::{AccessibilityEvent, FocusEvent, LiveRegionEvent, Politeness};
use tokio::sync::mpsc::UnboundedSender;

/// Common cross-platform abstraction for OS-level accessibility event listeners.
pub trait AccessibilityEventListener: Send + Sync {
    /// Starts listening for platform accessibility events, forwarding them to the provided channel.
    fn start(&mut self, sender: UnboundedSender<AccessibilityEvent>) -> Result<(), PlatformError>;

    /// Stops listening for platform accessibility events.
    fn stop(&mut self) -> Result<(), PlatformError>;

    /// Returns whether the listener is actively running.
    fn is_active(&self) -> bool;
}

/// Factory creating the platform-appropriate accessibility event listener.
pub fn create_event_listener() -> Box<dyn AccessibilityEventListener> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsAccessibilityEventListener::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacAccessibilityEventListener::new())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Box::new(mock::MockAccessibilityEventListener::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_event_listener() {
        let listener = create_event_listener();
        assert!(!listener.is_active());
    }
}
