pub mod mock;
#[cfg(target_os = "windows")]
pub mod windows;

use crate::PlatformError;
use okur_core::event::AccessibilityEvent;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_event_listener() {
        #[cfg(target_os = "windows")]
        let listener = windows::WindowsAccessibilityEventListener::new();
        #[cfg(not(target_os = "windows"))]
        let listener = mock::MockAccessibilityEventListener::new();
        assert!(!listener.is_active());
    }
}
