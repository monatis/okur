//! The desktop's accessibility events on Linux: every application's AT-SPI events, through the
//! vendored xa11y's desktop subscription (registered with the AT-SPI registry, without which
//! GTK 4 and Chrome send nothing: the Linux lab's probe 2), as the engine takes them. First the
//! focus: an element taking the keyboard focus is the engine's focus event
//! (`okur_linux_model::focus`); the other kinds follow as the engine's Linux path grows.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use okur_core::event::AccessibilityEvent;
use okur_linux_model::focus_track::{FocusTrack, Reread};
use okur_runtime::events::{EventSink, EventSource};
use okur_runtime::RuntimeError;
use xa11y_core::{ElementData, Event, EventKind, RecvStatus, Role};
use xa11y_linux::{AtspiObject, AtspiTree, LinuxProvider};

/// The objects of the page read last, its application's bus name and each object's path, in the
/// order the page's nodes are numbered ([`okur_linux_model::page::to_ia2`], depth first from 1):
/// what acting on a page's node finds its object by ([`crate::actions::LinuxActions`]).
pub type PageObjects = Arc<Mutex<Vec<(String, String)>>>;

/// `tree`'s objects, depth first, as the page's nodes are numbered.
fn objects_of(tree: &AtspiTree, out: &mut Vec<(String, String)>) {
    out.push((tree.bus_name.clone(), tree.path.clone()));
    for child in &tree.children {
        objects_of(child, out);
    }
}

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
        uri: tree.uri,
        children: tree.children.into_iter().map(snapshot).collect(),
    }
}

/// How far up from a focused element a page's document is looked for.
const PAGE_DEPTH: usize = 40;

/// The most objects a native focus's subtree, and each surrounding subtree, is read with.
const CONTROL_LIMIT: usize = 200;

/// How far up from a native focus its window is looked for.
const WINDOW_DEPTH: usize = 12;

/// A focus outside a page with what surrounds it, as okur-linux-model's `native_focus` reads
/// it: its own subtree, its parent's, and above it a list's, a column view's or a group box's
/// (their items, columns, label), the other ancestors alone, up to its window. A few calls on
/// the bus per object read.
fn native_focus(provider: &LinuxProvider, element: &ElementData) -> okur_core::event::FocusEvent {
    let base = okur_linux_model::focus::focus_event(element);
    let Some(object) = AtspiObject::of(element) else {
        tracing::debug!("a native focus without its object's names");
        return base;
    };
    let focus = snapshot(provider.atspi_tree(&object, CONTROL_LIMIT));
    let mut ancestors = Vec::new();
    let mut next = provider.atspi_parent(&object);
    while let Some(parent) = next {
        if ancestors.len() >= WINDOW_DEPTH {
            break;
        }
        let whole = ancestors.is_empty()
            || (ancestors.len() == 1
                && matches!(parent.role.as_str(), "list" | "tree table" | "grouping"));
        let limit = if whole { CONTROL_LIMIT } else { 1 };
        next = provider.atspi_parent(&parent);
        let node = snapshot(provider.atspi_tree(&parent, limit));
        let window = matches!(node.role.as_str(), "frame" | "dialog" | "window" | "alert");
        ancestors.push(node);
        if window {
            break;
        }
    }
    tracing::debug!(
        focus = %focus.role,
        attributes = ?focus.attributes,
        states = focus.states,
        ancestors = ?ancestors.iter().map(|a| a.role.as_str()).collect::<Vec<_>>(),
        parent = ?ancestors.first().map(|a| &a.attributes),
        "native focus read"
    );
    okur_linux_model::focus::native_focus(base, &focus, &ancestors)
}

/// An element's AT-SPI object, by its bus name and path.
fn object_of(element: &ElementData) -> Option<String> {
    AtspiObject::of(element).map(|object| key_of(&object))
}

/// The page's document the focused element is in, or is.
fn document_of(provider: &LinuxProvider, element: &ElementData) -> Option<AtspiObject> {
    let object = AtspiObject::of(element)?;
    if element.role == Role::WebArea {
        Some(object)
    } else {
        provider.atspi_ancestor(&object, "document web", PAGE_DEPTH)
    }
}

/// An AT-SPI object's key, its bus name and path.
fn key_of(object: &AtspiObject) -> String {
    format!("{}{}", object.bus_name, object.path)
}

/// Whether `document` is a page in view: a document shown, not a frame's inside another page
/// (each frame's document loads too), nor a hidden tab's.
fn shown_page(provider: &LinuxProvider, document: &ElementData) -> bool {
    // The parent by the document's object: an event's element is in no provider's handle cache,
    // so get_parent never found one and every frame's document passed (the code review of
    // 2026-11-08).
    document.role == Role::WebArea
        && document.states.visible
        && AtspiObject::of(document)
            .and_then(|object| provider.atspi_parent(&object))
            .and_then(|parent| provider.atspi_ancestor(&parent, "document web", PAGE_DEPTH))
            .is_none()
}

/// The page the focus is in, read whole, as the engine takes a page: when the focus lands on a
/// page's document (Chrome gives it when a page loads and when Tab comes back to it), and when
/// it lands inside a page other than the one read last (a link of a page newly in front).
fn page(
    provider: &LinuxProvider,
    element: &ElementData,
    document: &AtspiObject,
    last: &mut Option<String>,
    objects: &PageObjects,
) -> Option<AccessibilityEvent> {
    let on_document = element.role == Role::WebArea;
    let object = key_of(document);
    if !on_document && last.as_deref() == Some(object.as_str()) {
        return None;
    }
    *last = Some(object);
    let started = std::time::Instant::now();
    let tree = provider.atspi_tree(document, PAGE_LIMIT);
    let mut read = Vec::new();
    objects_of(&tree, &mut read);
    if let Ok(mut objects) = objects.lock() {
        *objects = read;
    }
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
/// Whether `event` is a change the engine is told of: the focused control's state (a check box
/// checked, a tab selected, a button expanded), as Windows' state change gives it.
fn control_change(event: &Event) -> bool {
    okur_linux_model::focus_track::is_control_change(&event.kind, focused(event))
}

/// Whether `event`'s object has the focus.
fn focused(event: &Event) -> bool {
    event.target.as_ref().is_some_and(|e| e.states.focused)
}

/// The page whose address ends with `address_end`, read whole as Okur reads a page, as Chrome
/// gives the focus to it, for the Linux lab's snapshots (`examples/atspi_snapshot.rs`, the
/// plan's §2.3), with the time reading it took; an error if no focus reached it in `wait`.
/// Start it before Chrome.
pub fn capture_page(
    address_end: &str,
    wait: Duration,
) -> Result<(okur_linux_model::page::AtspiNode, Duration), RuntimeError> {
    let provider = LinuxProvider::new().map_err(|e| source_error("the bus", e))?;
    let subscription = provider
        .subscribe_desktop_kinds(Box::new(okur_linux_model::focus_track::is_heard))
        .map_err(|e| source_error("the desktop's events", e))?;
    let started = std::time::Instant::now();
    while started.elapsed() < wait {
        let RecvStatus::Event(event) = subscription.recv_status(Duration::from_millis(200)) else {
            continue;
        };
        if !matches!(
            event.kind,
            EventKind::FocusChanged | EventKind::DocumentLoaded
        ) {
            continue;
        }
        let Some(document) = event
            .target
            .as_ref()
            .and_then(|e| document_of(&provider, e))
        else {
            continue;
        };
        let address = provider.atspi_tree(&document, 1).uri;
        if !address.is_some_and(|a| a.ends_with(address_end)) {
            continue;
        }
        let reading = std::time::Instant::now();
        let tree = provider.atspi_tree(&document, PAGE_LIMIT);
        return Ok((snapshot(tree), reading.elapsed()));
    }
    Err(source_error(
        "the page",
        format!("no focus reached a page at …{address_end} in {wait:?}"),
    ))
}

pub fn convert(event: &Event, in_page: bool) -> Option<AccessibilityEvent> {
    match event.kind {
        EventKind::FocusChanged => event.target.as_ref().map(|element| {
            let mut focus = okur_linux_model::focus::focus_event(element);
            focus.in_document = in_page;
            AccessibilityEvent::Focus(focus)
        }),
        EventKind::StateChanged { .. } if control_change(event) => {
            event.target.as_ref().map(|element| {
                let mut control = okur_linux_model::focus::focus_event(element);
                control.in_document = in_page;
                AccessibilityEvent::Change {
                    kind: okur_core::event::ChangeKind::State,
                    control,
                }
            })
        }
        _ => None,
    }
}

/// Every application's accessibility events, from the AT-SPI bus.
#[derive(Default)]
pub struct AtspiEventSource {
    running: Option<Running>,
    objects: PageObjects,
}

struct Running {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl AtspiEventSource {
    pub fn new() -> Self {
        Self::default()
    }

    /// The objects of the page it read last, for acting on the page's nodes.
    pub fn objects(&self) -> PageObjects {
        Arc::clone(&self.objects)
    }
}

impl EventSource for AtspiEventSource {
    fn start(&mut self, sink: EventSink) -> Result<(), RuntimeError> {
        if self.running.is_some() {
            return Ok(());
        }
        let provider = LinuxProvider::new().map_err(|e| source_error("the bus", e))?;
        let subscription = provider
            .subscribe_desktop_kinds(Box::new(okur_linux_model::focus_track::is_heard))
            .map_err(|e| source_error("the desktop's events", e))?;
        tracing::debug!("AT-SPI: the desktop's events subscribed");
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            let objects = Arc::clone(&self.objects);
            std::thread::spawn(move || {
                // The subscription lives in this thread: dropped here, it deregisters its
                // events and ends its own reader.
                let mut last_page = None;
                // The focus as the engine was last told of it, and the control outside a page
                // that has it.
                let mut track = FocusTrack::default();
                let clock = std::time::Instant::now();
                let mut native_element: Option<ElementData> = None;
                let objects = objects;
                while !stop.load(Ordering::SeqCst) {
                    match subscription.recv_status(Duration::from_millis(100)) {
                        RecvStatus::Event(event) => {
                            tracing::trace!(
                                kind = ?event.kind,
                                role = ?event.target.as_ref().map(|e| e.role),
                                name = ?event.target.as_ref().and_then(|e| e.name.as_deref()),
                                "AT-SPI event"
                            );
                            if event.kind == EventKind::FocusChanged {
                                tracing::debug!(
                                    app = %event.app_name,
                                    role = ?event.target.as_ref().map(|e| e.role),
                                    "AT-SPI focus"
                                );
                            }
                            // The focus's page, looked up once: read when it is new, and
                            // telling the engine the focus is in it; a focused control's
                            // change is told the same way, whether it is on a page.
                            let focus = event.kind == EventKind::FocusChanged;
                            // GTK 4's second focus event for one move, passed over before
                            // anything is read.
                            let at_ms =
                                u64::try_from(clock.elapsed().as_millis()).unwrap_or(u64::MAX);
                            if focus
                                && track.is_twin(
                                    event.target.as_ref().and_then(object_of).as_deref(),
                                    at_ms,
                                )
                            {
                                tracing::debug!("AT-SPI focus's twin");
                                continue;
                            }
                            let document = event
                                .target
                                .as_ref()
                                .filter(|_| focus || control_change(&event))
                                .and_then(|e| Some((e, document_of(&provider, e)?)));
                            let loaded = document
                                .as_ref()
                                .filter(|_| focus)
                                .and_then(|(e, d)| page(&provider, e, d, &mut last_page, &objects));
                            // A page that finished loading, a reload's too, is read again:
                            // the engine takes the page it already has as a refresh, and as
                            // a new load after F5 or Ctrl+R (the lab's live run of 712).
                            let reloaded = event
                                .target
                                .as_ref()
                                .filter(|d| {
                                    event.kind == EventKind::DocumentLoaded
                                        && shown_page(&provider, d)
                                })
                                .and_then(|d| {
                                    let document = AtspiObject::of(d)?;
                                    page(&provider, d, &document, &mut last_page, &objects)
                                });
                            for loaded in loaded.into_iter().chain(reloaded) {
                                if !sink.send(loaded) {
                                    return;
                                }
                            }
                            // The focus's repeats and a control's own changes told from a focus
                            // move: okur_linux_model::focus_track's rules.
                            let object = event.target.as_ref().and_then(object_of);
                            match event.kind {
                                EventKind::WindowActivated => {
                                    tracing::debug!(window = ?object, "AT-SPI window activated");
                                    track.window_activated(object.clone());
                                }
                                EventKind::WindowDeactivated => track.window_deactivated(),
                                _ => {}
                            }
                            // A focus outside a page, and its control's change, read with what
                            // surrounds it: the change names the control as its focus did.
                            let changed = control_change(&event);
                            let native = event
                                .target
                                .as_ref()
                                .filter(|_| (focus || changed) && document.is_none())
                                .map(|e| native_focus(&provider, e))
                                .map(|control| {
                                    if changed {
                                        AccessibilityEvent::Change {
                                            kind: okur_core::event::ChangeKind::State,
                                            control,
                                        }
                                    } else {
                                        AccessibilityEvent::Focus(control)
                                    }
                                });
                            if let Some(AccessibilityEvent::Focus(read)) = &native {
                                if !track.native_focus(object.clone(), read) {
                                    tracing::debug!(name = ?read.name, "AT-SPI focus repeated");
                                    continue;
                                }
                                native_element = event.target.clone();
                            } else if focus && document.is_some() {
                                track.page_focus(object.clone());
                                native_element = None;
                            }
                            // A label renamed, or an object selected without the focus: the focused
                            // control may read otherwise. Read again, it is said as a focus when
                            // it reads as another control (GTK's notebook keeps the focus as Right
                            // selects its next tab [obs, 303/s13]), as its value changing when
                            // only that changed (GTK's drop-down shows its chosen item in its
                            // button's label, renamed as Down or Up chooses another [obs,
                            // 304/s9-s10: "Brazil | 3 of 3"]).
                            let inside = okur_linux_model::focus_track::is_inside_change(
                                &event.kind,
                                focused(&event),
                            ) && track.in_focused_application(object.as_deref());
                            if inside {
                                if let Some(element) = native_element.as_ref() {
                                    let control = native_focus(&provider, element);
                                    let reread = match track.reread(&control) {
                                        Reread::Focus => Some(AccessibilityEvent::Focus(control)),
                                        Reread::Value => Some(AccessibilityEvent::Change {
                                            kind: okur_core::event::ChangeKind::Value,
                                            control,
                                        }),
                                        Reread::Same => None,
                                    };
                                    if let Some(reread) = reread {
                                        if !sink.send(reread) {
                                            return;
                                        }
                                    }
                                }
                            }
                            if let Some(event) =
                                native.or_else(|| convert(&event, document.is_some()))
                            {
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
