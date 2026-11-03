//! A system's accessibility events: the focus, menus, text and values, as the engine takes them.

use crate::RuntimeError;
use okur_core::event::AccessibilityEvent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

/// Where a system's events go, in order: the runtime's event loop.
#[derive(Debug, Clone)]
pub struct EventSink(UnboundedSender<AccessibilityEvent>);

impl EventSink {
    pub fn new(sender: UnboundedSender<AccessibilityEvent>) -> Self {
        Self(sender)
    }

    /// Passes `event` on; false once the event loop has gone.
    pub fn send(&self, event: AccessibilityEvent) -> bool {
        self.0.send(event).is_ok()
    }

    /// Whether the event loop has gone.
    pub fn is_closed(&self) -> bool {
        self.0.is_closed()
    }
}

/// A system's source of accessibility events.
pub trait EventSource: Send + Sync {
    /// Starts listening for the system's accessibility events, passing them to `sink`.
    fn start(&mut self, sink: EventSink) -> Result<(), RuntimeError>;

    /// Stops listening for platform accessibility events.
    fn stop(&mut self) -> Result<(), RuntimeError>;

    /// Returns whether the listener is actively running.
    fn is_active(&self) -> bool;
}

/// Mock accessibility event source for testing, differential verification, and headless CI.
#[derive(Debug, Default, Clone)]
pub struct MockEventSource {
    active: Arc<AtomicBool>,
    sink: Arc<Mutex<Option<EventSink>>>,
}

impl MockEventSource {
    pub fn new() -> Self {
        Self::default()
    }

    /// Injects a synthetic accessibility event directly into the sink.
    pub fn inject_event(&self, event: AccessibilityEvent) -> Result<(), RuntimeError> {
        if !self.is_active() {
            return Err(RuntimeError::EventSource("Listener is not active".into()));
        }
        let guard = match self.sink.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        match *guard {
            Some(ref sink) => {
                sink.send(event);
                Ok(())
            }
            None => Err(RuntimeError::EventSource("No active sender channel".into())),
        }
    }
}

impl EventSource for MockEventSource {
    fn start(&mut self, sink: EventSink) -> Result<(), RuntimeError> {
        self.active.store(true, Ordering::SeqCst);
        let mut guard = match self.sink.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *guard = Some(sink);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        self.active.store(false, Ordering::SeqCst);
        let mut guard = match self.sink.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
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
    use okur_core::event::{FocusEvent, Role};

    #[tokio::test]
    async fn test_mock_event_listener_lifecycle() {
        let mut listener = MockEventSource::new();
        assert!(!listener.is_active());

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        assert!(listener.start(EventSink::new(tx)).is_ok(), "Start listener");
        assert!(listener.is_active());

        // Inject focus event
        let ev = AccessibilityEvent::Focus(FocusEvent::new(Role::Edit).with_name("Test Input"));
        assert!(listener.inject_event(ev.clone()).is_ok(), "Inject event");

        let received = rx.recv().await;
        assert_eq!(received, Some(ev));

        assert!(listener.stop().is_ok(), "Stop listener");
        assert!(!listener.is_active());
    }
}
