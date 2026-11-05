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
use xa11y_core::{ElementData, Event, EventKind, RecvStatus, Role};
use xa11y_linux::{AtspiTree, LinuxProvider};

/// The most objects a page's capture reads: a large page is cut there, its first part read.
const PAGE_LIMIT: usize = 20_000;

/// xa11y-linux's capture as the model's snapshot node.
fn snapshot(tree: AtspiTree) -> okur_linux_model::page::AtspiNode {
    okur_linux_model::page::AtspiNode {
        role: tree.role,
        name: tree.name,
        attributes: tree.attributes.into_iter().collect(),
        states: tree.states,
        text: tree.text,
        children: tree.children.into_iter().map(snapshot).collect(),
    }
}

/// The page the focus landed on, read whole, as the engine takes a page: when the focus lands
/// on a page's document, as Chrome gives it when a page loads and when Tab comes back to it.
fn page(provider: &LinuxProvider, element: &ElementData) -> Option<AccessibilityEvent> {
    if element.role != Role::WebArea {
        return None;
    }
    let started = std::time::Instant::now();
    let tree = provider
        .atspi_tree(element, PAGE_LIMIT)
        .map_err(|e| tracing::debug!("the page was not read: {e}"))
        .ok()?;
    let root = okur_linux_model::page::to_aom(&snapshot(tree));
    tracing::debug!(ms = started.elapsed().as_millis(), "page read");
    let title = root.name.clone().unwrap_or_default();
    Some(AccessibilityEvent::DocumentLoaded { title, root })
}

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
                            if event.kind == EventKind::FocusChanged {
                                let loaded = event.target.as_ref().and_then(|e| page(&provider, e));
                                if let Some(loaded) = loaded {
                                    if !sink.send(loaded) {
                                        return;
                                    }
                                }
                            }
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
