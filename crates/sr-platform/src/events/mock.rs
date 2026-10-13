use crate::PlatformError;
use sr_core::event::AccessibilityEvent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

use super::AccessibilityEventListener;

/// Mock accessibility event listener for testing, differential verification, and headless CI.
#[derive(Debug, Default, Clone)]
pub struct MockAccessibilityEventListener {
    active: Arc<AtomicBool>,
    sender: Arc<Mutex<Option<UnboundedSender<AccessibilityEvent>>>>,
}

impl MockAccessibilityEventListener {
    pub fn new() -> Self {
        Self::default()
    }

    /// Injects a synthetic accessibility event directly into the listener channel.
    pub fn inject_event(&self, event: AccessibilityEvent) -> Result<(), PlatformError> {
        if !self.is_active() {
            return Err(PlatformError::EventListenerError(
                "Listener is not active".into(),
            ));
        }
        let guard = self.sender.lock().unwrap();
        if let Some(ref tx) = *guard {
            let _ = tx.send(event);
            Ok(())
        } else {
            Err(PlatformError::EventListenerError(
                "No active sender channel".into(),
            ))
        }
    }
}

impl AccessibilityEventListener for MockAccessibilityEventListener {
    fn start(&mut self, sender: UnboundedSender<AccessibilityEvent>) -> Result<(), PlatformError> {
        self.active.store(true, Ordering::SeqCst);
        let mut guard = self.sender.lock().unwrap();
        *guard = Some(sender);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        self.active.store(false, Ordering::SeqCst);
        let mut guard = self.sender.lock().unwrap();
        *guard = None;
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sr_core::event::{FocusEvent, Role};

    #[tokio::test]
    async fn test_mock_event_listener_lifecycle() {
        let mut listener = MockAccessibilityEventListener::new();
        assert!(!listener.is_active());

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        listener.start(tx).expect("Start listener");
        assert!(listener.is_active());

        // Inject focus event
        let ev = AccessibilityEvent::Focus(FocusEvent::new(Role::Edit).with_name("Test Input"));
        listener.inject_event(ev.clone()).expect("Inject event");

        let received = rx.recv().await.expect("Receive event");
        assert_eq!(received, ev);

        listener.stop().expect("Stop listener");
        assert!(!listener.is_active());
    }
}
