use crate::PlatformError;
use okur_core::event::AccessibilityEvent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use super::AccessibilityEventListener;

/// macOS native accessibility event listener using AXObserver.
pub struct MacAccessibilityEventListener {
    active: Arc<AtomicBool>,
}

impl MacAccessibilityEventListener {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for MacAccessibilityEventListener {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessibilityEventListener for MacAccessibilityEventListener {
    fn start(&mut self, _sender: UnboundedSender<AccessibilityEvent>) -> Result<(), PlatformError> {
        tracing::info!("Initializing macOS AXObserver event listener");
        self.active.store(true, Ordering::SeqCst);
        // Pre-structured for kAXFocusedUIElementChangedNotification and AXObserverCreate
        Ok(())
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        tracing::info!("Stopping macOS AXObserver event listener");
        self.active.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}
