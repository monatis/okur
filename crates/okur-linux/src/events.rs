//! The desktop's accessibility events on Linux: every application's AT-SPI events, through the
//! vendored xa11y's desktop subscription (registered with the AT-SPI registry, without which
//! GTK 4 and Chrome send nothing: the Linux lab's probe 2), as the engine takes them. First the
//! focus: an element taking the keyboard focus is the engine's focus event
//! (`okur_linux_model::focus`); the other kinds follow as the engine's Linux path grows.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use okur_core::event::AccessibilityEvent;
use okur_runtime::events::{EventSink, EventSource};
use okur_runtime::RuntimeError;
use xa11y_core::{Event, EventKind, RecvStatus};
use xa11y_linux::LinuxProvider;

fn source_error(what: &str, error: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::EventSource(format!("AT-SPI: {what}: {error}"))
}

/// The engine's event for an AT-SPI event, when it takes one.
pub fn convert(event: &Event) -> Option<AccessibilityEvent> {
    match event.kind {
        EventKind::FocusChanged => event.target.as_ref().map(|element| {
            AccessibilityEvent::Focus(okur_linux_model::focus::focus_event(element))
        }),
        _ => None,
    }
}

/// Every application's accessibility events, from the AT-SPI bus.
#[derive(Default)]
pub struct AtspiEventSource {
    running: Option<Running>,
}

struct Running {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl AtspiEventSource {
    pub fn new() -> Self {
        Self::default()
    }
}

impl EventSource for AtspiEventSource {
    fn start(&mut self, sink: EventSink) -> Result<(), RuntimeError> {
        if self.running.is_some() {
            return Ok(());
        }
        let provider = LinuxProvider::new().map_err(|e| source_error("the bus", e))?;
        let subscription = provider
            .subscribe_desktop()
            .map_err(|e| source_error("the desktop's events", e))?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                // The subscription lives in this thread: dropped here, it deregisters its
                // events and ends its own reader.
                while !stop.load(Ordering::SeqCst) {
                    match subscription.recv_status(Duration::from_millis(100)) {
                        RecvStatus::Event(event) => {
                            if let Some(event) = convert(&event) {
                                if !sink.send(event) {
                                    return;
                                }
                            }
                        }
                        RecvStatus::Timeout => {}
                        RecvStatus::Disconnected => return,
                    }
                }
            })
        };
        self.running = Some(Running { stop, thread });
        Ok(())
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        if let Some(running) = self.running.take() {
            running.stop.store(true, Ordering::SeqCst);
            let _ = running.thread.join();
        }
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.running.is_some()
    }
}

impl Drop for AtspiEventSource {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
