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

/// How far up from a focused element a page's document is looked for.
const PAGE_DEPTH: usize = 40;

/// An element's AT-SPI object, by its bus name and path.
fn object_of(element: &ElementData) -> Option<String> {
    let raw = |key: &str| element.raw.get(key).and_then(|v| v.as_str());
    Some(format!("{}{}", raw("bus_name")?, raw("object_path")?))
}

/// The page's document the focused element is in, or is.
fn document_of(provider: &LinuxProvider, element: &ElementData) -> Option<ElementData> {
    if element.role == Role::WebArea {
        Some(element.clone())
    } else {
        provider.atspi_ancestor(element, "document web", PAGE_DEPTH)
    }
}

/// The page the focus is in, read whole, as the engine takes a page: when the focus lands on a
/// page's document (Chrome gives it when a page loads and when Tab comes back to it), and when
/// it lands inside a page other than the one read last (a link of a page newly in front).
fn page(
    provider: &LinuxProvider,
    element: &ElementData,
    document: &ElementData,
    last: &mut Option<String>,
) -> Option<AccessibilityEvent> {
    let on_document = element.role == Role::WebArea;
    let object = object_of(document)?;
    if !on_document && last.as_deref() == Some(object.as_str()) {
        return None;
    }
    *last = Some(object);
    let started = std::time::Instant::now();
    let tree = provider
        .atspi_tree(document, PAGE_LIMIT)
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

/// The engine's event for an AT-SPI event, when it takes one. `in_page`: the focus is in a page's
/// document, or on it, where the virtual cursor reads and takes the keys, as the focus chain
/// that holds a document tells it on Windows.
pub fn convert(event: &Event, in_page: bool) -> Option<AccessibilityEvent> {
    match event.kind {
        EventKind::FocusChanged => event.target.as_ref().map(|element| {
            let mut focus = okur_linux_model::focus::focus_event(element);
            focus.in_document = in_page;
            AccessibilityEvent::Focus(focus)
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
                let mut last_page = None;
                while !stop.load(Ordering::SeqCst) {
                    match subscription.recv_status(Duration::from_millis(100)) {
                        RecvStatus::Event(event) => {
                            // The focus's page, looked up once: read when it is new, and
                            // telling the engine the focus is in it.
                            let document = event
                                .target
                                .as_ref()
                                .filter(|_| event.kind == EventKind::FocusChanged)
                                .and_then(|e| Some((e, document_of(&provider, e)?)));
                            let loaded = document
                                .as_ref()
                                .and_then(|(e, d)| page(&provider, e, d, &mut last_page));
                            if let Some(loaded) = loaded {
                                if !sink.send(loaded) {
                                    return;
                                }
                            }
                            if let Some(event) = convert(&event, document.is_some()) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn focus_on(role: Role) -> Event {
        let mut event = Event::new(EventKind::FocusChanged, "chrome", 1);
        event.target = Some(ElementData::for_role(role));
        event
    }

    /// The focus in a page is in its document, and the engine's virtual cursor takes the keys
    /// there: without it Down went to Chrome, and nothing was said (the lab's live run of
    /// 02-doc-next-line, 2026-11-05).
    #[test]
    fn a_focus_in_a_page_is_in_its_document() {
        let in_page = convert(&focus_on(Role::Link), true);
        assert!(matches!(in_page, Some(AccessibilityEvent::Focus(f)) if f.in_document));
        let outside = convert(&focus_on(Role::Button), false);
        assert!(matches!(outside, Some(AccessibilityEvent::Focus(f)) if !f.in_document));
        let other = Event::new(EventKind::Announcement, "chrome", 1);
        assert!(convert(&other, true).is_none());
    }
}
