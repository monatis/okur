//! Windows accessibility events for the live screen reader.
//!
//! Events come from the WinEvent capture that `okur trace` records
//! ([`crate::event_trace::run_live`]). They are converted by the converter parity replay uses
//! ([`EventConverter`]), so the live screen reader consumes
//! the event stream the harness records, replays and verifies against JAWS.

use super::AccessibilityEventListener;
use crate::event_trace::{run_live, EventConverter, TraceEvent};
use crate::PlatformError;
use okur_core::AccessibilityEvent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

/// UIA `Document` control type.
const UIA_DOCUMENT: i32 = 50030;

/// What the page worker is asked to capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageRequest {
    /// The page after live region changes or other page changes, once they settle.
    Refresh(ForegroundWindow),
    /// The page in full through IA2, at once: just read quickly through UIA, or changed
    /// while the engine was not reading it (forms mode).
    Refine(ForegroundWindow),
    /// The page quickly through UIA, then in full through IA2: for a focus the engine's
    /// buffer does not hold (content a web application added).
    Focus(ForegroundWindow),
    /// The page's changes at once, now that they are over (they came while forms mode was on):
    /// the changed parts when they are known, else the whole page.
    Changes(ForegroundWindow),
}

/// The objects of the page in front whose children changed since its last capture
/// (`EVENT_OBJECT_REORDER`), so that only those parts are read again.
#[derive(Debug, Default)]
struct ChangedParts {
    objects: Vec<i32>,
    /// More parts changed than are worth reading one by one: the whole page is read.
    whole_page: bool,
}

/// The changes the hook has passed on since the last capture.
static CHANGED_PARTS: std::sync::Mutex<ChangedParts> = std::sync::Mutex::new(ChangedParts {
    objects: Vec::new(),
    whole_page: false,
});

/// The most changed parts read one by one; past it the whole page is read.
const MAX_CHANGED_PARTS: usize = 8;

/// Notes a change the hook passed on. Chromium reorders the parent of every object shown or
/// hidden, and raises IA2's text events on the block whose text changed in place, so these
/// name each changed part; shows and hides alone add nothing to read (old-pc, 2026-10-18).
fn note_page_change(change: crate::event_trace::PageChange) {
    use crate::event_trace::{IA2_EVENT_TEXT_INSERTED, IA2_EVENT_TEXT_REMOVED};
    const EVENT_OBJECT_REORDER: u32 = 0x8004;
    if !matches!(
        change.event,
        EVENT_OBJECT_REORDER | IA2_EVENT_TEXT_INSERTED | IA2_EVENT_TEXT_REMOVED
    ) {
        return;
    }
    if let Ok(mut parts) = CHANGED_PARTS.lock() {
        if parts.objects.contains(&change.object) {
            return;
        }
        if parts.objects.len() >= MAX_CHANGED_PARTS {
            parts.whole_page = true;
        } else {
            parts.objects.push(change.object);
        }
    }
}

/// The parts changed since the last capture, forgotten as they are taken: `None` when the
/// whole page is to be read (too many parts, or none named).
fn take_changed_parts() -> Option<Vec<i32>> {
    let parts = CHANGED_PARTS
        .lock()
        .ok()
        .map(|mut parts| std::mem::take(&mut *parts))?;
    (!parts.whole_page && !parts.objects.is_empty()).then_some(parts.objects)
}

/// Reads the changed parts of the page in `window` again, through IA2, and sends them together
/// as [`AccessibilityEvent::PagePartsLoaded`], spliced with one rebuild. False when one cannot be read: the caller reads the
/// whole page.
fn read_changed_parts(
    window: ForegroundWindow,
    objects: &[i32],
    sender: &UnboundedSender<AccessibilityEvent>,
) -> bool {
    use windows::Win32::Foundation::HWND;
    let started = std::time::Instant::now();
    let top = HWND(window.0 as *mut core::ffi::c_void);
    let mut parts = Vec::with_capacity(objects.len());
    for &object in objects {
        match xa11y_windows::ia2::capture_chromium_object(top, object) {
            Ok(node) => parts.push(crate::ia2_snapshot::ia2_to_aom(&node)),
            Err(e) => {
                tracing::debug!(object, "page part not read again ({e}): the whole page is");
                return false;
            }
        }
    }
    tracing::debug!(
        parts = parts.len(),
        ms = started.elapsed().as_millis(),
        "page parts read again"
    );
    let _ = sender.send(AccessibilityEvent::PagePartsLoaded(parts));
    true
}

/// The running listener's page worker, for the engine's page requests.
static PAGE_REQUESTS: std::sync::Mutex<Option<std::sync::mpsc::Sender<PageRequest>>> =
    std::sync::Mutex::new(None);

/// Asks the page worker for a capture of the window in front now. False when no listener
/// runs.
fn request_page(request: fn(ForegroundWindow) -> PageRequest) -> bool {
    PAGE_REQUESTS
        .lock()
        .ok()
        .and_then(|pages| {
            pages
                .as_ref()
                .map(|tx| tx.send(request(ForegroundWindow::now())).is_ok())
        })
        .unwrap_or(false)
}

/// Asks the page worker to read the page in front again, for a focus the engine's buffer
/// does not hold. False when no listener runs.
pub fn refresh_page_for_focus() -> bool {
    request_page(PageRequest::Focus)
}

/// Asks the page worker to read the page in front again after it changed: once the changes
/// settle, or at once (`now`) when they are over. False when no listener runs.
pub fn refresh_changed_page(now: bool) -> bool {
    request_page(if now {
        PageRequest::Changes
    } else {
        PageRequest::Refresh
    })
}

/// The top-level window a page request is for, as a plain handle value: the page is captured
/// only while that window is still in front, never some other application's document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ForegroundWindow(isize);

impl ForegroundWindow {
    fn now() -> Self {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        // SAFETY: plain Win32 call with no preconditions.
        Self(unsafe { GetForegroundWindow() }.0 as isize)
    }
}

pub struct WindowsAccessibilityEventListener {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl WindowsAccessibilityEventListener {
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }
}

impl Default for WindowsAccessibilityEventListener {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessibilityEventListener for WindowsAccessibilityEventListener {
    fn start(&mut self, sender: UnboundedSender<AccessibilityEvent>) -> Result<(), PlatformError> {
        if self.thread.is_some() {
            return Ok(());
        }
        self.stop.store(false, Ordering::Relaxed);
        let stop = Arc::clone(&self.stop);
        let pages = start_page_worker(sender.clone())?;
        if let Ok(mut requests) = PAGE_REQUESTS.lock() {
            *requests = Some(pages.clone());
        }
        let thread = std::thread::Builder::new()
            .name("okur-winevents".to_owned())
            .spawn(move || {
                let mut page: Option<String> = None;
                let mut converter = EventConverter::default();
                let changes = sender.clone();
                let page_changed = move |change: crate::event_trace::PageChange| {
                    note_page_change(change);
                    if change.notify {
                        let _ = changes.send(AccessibilityEvent::PageChanged);
                    }
                };
                let sink = move |event: TraceEvent| {
                    deliver(&event, &mut converter, &mut page, &sender, &pages)
                };
                let count = run_live(Box::new(sink), Box::new(page_changed), &stop);
                tracing::debug!(count, "WinEvent listener stopped");
            })
            .map_err(|e| PlatformError::EventListenerError(e.to_string()))?;
        self.thread = Some(thread);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        // The worker ends with its last sender.
        if let Ok(mut requests) = PAGE_REQUESTS.lock() {
            *requests = None;
        }
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.thread.is_some()
    }
}

/// Converts one captured event for the engine, through the [`EventConverter`] parity replay
/// uses. A focus landing in a page that differs from the last one (by its document's title
/// and URL) first loads that page into the buffer.
fn deliver(
    event: &TraceEvent,
    converter: &mut EventConverter,
    page: &mut Option<String>,
    sender: &UnboundedSender<AccessibilityEvent>,
    pages: &std::sync::mpsc::Sender<PageRequest>,
) {
    let events = converter.convert(event);
    if events.is_empty() && crate::event_trace::is_focus(event) {
        tracing::debug!(class = ?event.window_class, "focus event without a usable focus chain");
    }
    for converted in events {
        // The page after a live region change is captured off this thread.
        if matches!(converted, AccessibilityEvent::LiveRegion(_)) {
            let _ = pages.send(PageRequest::Refresh(ForegroundWindow::now()));
        }
        deliver_one(event, converted, page, sender, pages);
    }
}

/// Captures the foreground page off the event thread, for refreshes after live region
/// changes: on a big page a capture takes long, and focus and caret events must keep
/// flowing meanwhile. Requests that arrive during a capture become one more capture.
/// How long live regions must be quiet before the page is captured again.
const REFRESH_QUIET: std::time::Duration = std::time::Duration::from_millis(700);
/// The longest a refresh waits for quiet: a reply streamed for a minute still refreshes.
const REFRESH_MAX_DELAY: std::time::Duration = std::time::Duration::from_secs(3);

/// Waits until no refresh request has come for `quiet`, or `max` has passed since the first:
/// a streamed chat reply changes its live region many times a second, and each change used
/// to start a capture of the whole page. Announcements do not wait for it; they compare
/// with the region's last text. Returns false when the requests' sender is gone.
fn settle<T>(
    requests: &std::sync::mpsc::Receiver<T>,
    quiet: std::time::Duration,
    max: std::time::Duration,
) -> bool {
    use std::sync::mpsc::RecvTimeoutError;
    let first = std::time::Instant::now();
    loop {
        let left = max.saturating_sub(first.elapsed());
        if left.is_zero() {
            return true;
        }
        match requests.recv_timeout(quiet.min(left)) {
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => return true,
            Err(RecvTimeoutError::Disconnected) => return false,
        }
    }
}

fn start_page_worker(
    sender: UnboundedSender<AccessibilityEvent>,
) -> Result<std::sync::mpsc::Sender<PageRequest>, PlatformError> {
    let (tx, rx) = std::sync::mpsc::channel::<PageRequest>();
    std::thread::Builder::new()
        .name("okur-page-refresh".to_owned())
        .spawn(move || {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
            // SAFETY: COM initialization for this worker thread.
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            while let Ok(request) = rx.recv() {
                let (PageRequest::Refresh(window)
                | PageRequest::Refine(window)
                | PageRequest::Focus(window)
                | PageRequest::Changes(window)) = request;
                if matches!(request, PageRequest::Refresh(_))
                    && !settle(&rx, REFRESH_QUIET, REFRESH_MAX_DELAY)
                {
                    break;
                }
                if ForegroundWindow::now() != window {
                    tracing::debug!(?request, "page request skipped: its window left the front");
                    continue;
                }
                // After changes, only the changed parts when they are known; a whole-page read
                // covers every change noted so far. A focus the buffer lacks is in a part that
                // changed (WhatsApp's options button, shown with its message's focus): reading
                // that part takes tens of milliseconds, the whole chat 2 s.
                let changed = take_changed_parts();
                if matches!(
                    request,
                    PageRequest::Refresh(_) | PageRequest::Changes(_) | PageRequest::Focus(_)
                ) {
                    if let Some(objects) = changed {
                        if read_changed_parts(window, &objects, &sender) {
                            continue;
                        }
                    }
                }
                let started = std::time::Instant::now();
                if matches!(request, PageRequest::Focus(_)) {
                    // The full capture below serves every request queued meanwhile.
                    rx.try_iter().for_each(drop);
                    match crate::actions::capture_foreground_page_quick() {
                        Ok(root) => {
                            let title = root.name.clone().unwrap_or_default();
                            let ms = started.elapsed().as_millis();
                            tracing::debug!(%title, ms, "page read again for the focus");
                            let _ = sender.send(AccessibilityEvent::DocumentLoaded { title, root });
                        }
                        Err(e) => tracing::debug!("quick page capture for the focus failed: {e}"),
                    }
                }
                match crate::actions::capture_foreground_page() {
                    Ok(root) => {
                        let title = root.name.clone().unwrap_or_default();
                        let ms = started.elapsed().as_millis();
                        match request {
                            PageRequest::Refresh(_) | PageRequest::Changes(_) => {
                                tracing::debug!(%title, ms, "page refreshed")
                            }
                            PageRequest::Refine(_) | PageRequest::Focus(_) => {
                                tracing::debug!(%title, ms, "page refined")
                            }
                        }
                        let _ = sender.send(AccessibilityEvent::DocumentLoaded { title, root });
                    }
                    Err(e) => tracing::warn!("page refresh failed: {e}"),
                }
            }
        })
        .map_err(|e| PlatformError::EventListenerError(e.to_string()))?;
    Ok(tx)
}

fn deliver_one(
    event: &TraceEvent,
    converted: AccessibilityEvent,
    page: &mut Option<String>,
    sender: &UnboundedSender<AccessibilityEvent>,
    pages: &std::sync::mpsc::Sender<PageRequest>,
) {
    match &converted {
        AccessibilityEvent::Focus(focus) => {
            tracing::debug!(
                role = ?focus.role,
                name = ?focus.name,
                in_document = focus.in_document,
                class = ?focus.platform.window_class,
                container = ?focus.container_role.as_ref().zip(focus.container_name.as_ref()),
                groups = ?focus.groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(),
                "focus event"
            );
            if focus.in_document {
                load_page_if_new(event, page, sender, pages);
            }
        }
        other => tracing::debug!(event = ?other, "event"),
    }
    let _ = sender.send(converted);
}

/// Loads a page new to the buffer. Events wait while it is captured, so it is read quickly
/// through UIA first, and the page worker captures it in full through IA2 right after
/// (GitHub's repository page: 3 s through IA2, 160 ms through UIA). The engine takes the full
/// capture as the same page again, keeping the virtual cursor's place.
fn load_page_if_new(
    event: &TraceEvent,
    page: &mut Option<String>,
    sender: &UnboundedSender<AccessibilityEvent>,
    pages: &std::sync::mpsc::Sender<PageRequest>,
) {
    let document = event
        .focus_chain
        .iter()
        .find(|n| n.control_type == UIA_DOCUMENT);
    // A page is known by its address (its title can change on it: WhatsApp's counts unread
    // chats), by its title when it has none.
    let key = document.map(|d| match d.value.as_deref().filter(|url| !url.is_empty()) {
        Some(url) => url.to_owned(),
        None => d.name.clone().unwrap_or_default(),
    });
    if key == *page {
        return;
    }
    *page = key;
    let title = document.and_then(|d| d.name.clone()).unwrap_or_default();
    let started = std::time::Instant::now();
    match crate::actions::capture_foreground_page_quick() {
        Ok(root) => {
            let ms = started.elapsed().as_millis();
            tracing::debug!(%title, ms, "page loaded");
            let _ = sender.send(AccessibilityEvent::DocumentLoaded { title, root });
            let _ = pages.send(PageRequest::Refine(ForegroundWindow::now()));
        }
        Err(e) => {
            tracing::debug!("quick page capture failed ({e}); capturing through IA2");
            match crate::actions::capture_foreground_page() {
                Ok(root) => {
                    let ms = started.elapsed().as_millis();
                    tracing::debug!(%title, ms, "page loaded");
                    let _ = sender.send(AccessibilityEvent::DocumentLoaded { title, root });
                }
                Err(e) => tracing::warn!("page capture failed: {e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::settle;
    use std::time::{Duration, Instant};

    #[test]
    fn a_burst_of_refresh_requests_settles_into_one() {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        for _ in 0..5 {
            let _ = tx.send(());
        }
        let started = Instant::now();
        assert!(settle(
            &rx,
            Duration::from_millis(20),
            Duration::from_secs(2)
        ));
        // The burst was taken, and the wait ended on quiet, long before the limit.
        assert!(rx.try_recv().is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_closed_request_channel_ends_the_wait() {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        drop(tx);
        assert!(!settle(
            &rx,
            Duration::from_millis(20),
            Duration::from_secs(2)
        ));
    }
}
