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
        let refresh = start_page_worker(sender.clone())?;
        let thread = std::thread::Builder::new()
            .name("okur-winevents".to_owned())
            .spawn(move || {
                let mut page: Option<String> = None;
                let mut converter = EventConverter::default();
                let sink = move |event: TraceEvent| {
                    deliver(&event, &mut converter, &mut page, &sender, &refresh)
                };
                let count = run_live(Box::new(sink), &stop);
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
    refresh: &std::sync::mpsc::Sender<()>,
) {
    let events = converter.convert(event);
    if events.is_empty() && crate::event_trace::is_focus(event) {
        tracing::debug!(class = ?event.window_class, "focus event without a usable focus chain");
    }
    for converted in events {
        // The page after a live region change is captured off this thread.
        if matches!(converted, AccessibilityEvent::LiveRegion(_)) {
            let _ = refresh.send(());
        }
        deliver_one(event, converted, page, sender);
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
fn settle(
    requests: &std::sync::mpsc::Receiver<()>,
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
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => return true,
            Err(RecvTimeoutError::Disconnected) => return false,
        }
    }
}

fn start_page_worker(
    sender: UnboundedSender<AccessibilityEvent>,
) -> Result<std::sync::mpsc::Sender<()>, PlatformError> {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::Builder::new()
        .name("okur-page-refresh".to_owned())
        .spawn(move || {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
            // SAFETY: COM initialization for this worker thread.
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            while rx.recv().is_ok() {
                if !settle(&rx, REFRESH_QUIET, REFRESH_MAX_DELAY) {
                    break;
                }
                let started = std::time::Instant::now();
                match crate::actions::capture_foreground_page() {
                    Ok(root) => {
                        let title = root.name.clone().unwrap_or_default();
                        let ms = started.elapsed().as_millis();
                        tracing::debug!(%title, ms, "page refreshed");
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
) {
    match &converted {
        AccessibilityEvent::Focus(focus) => {
            tracing::debug!(
                role = ?focus.role,
                name = ?focus.name,
                in_document = focus.in_document,
                class = ?focus.window_class,
                "focus event"
            );
            if focus.in_document {
                load_page_if_new(event, page, sender);
            }
        }
        other => tracing::debug!(event = ?other, "event"),
    }
    let _ = sender.send(converted);
}

fn load_page_if_new(
    event: &TraceEvent,
    page: &mut Option<String>,
    sender: &UnboundedSender<AccessibilityEvent>,
) {
    let document = event
        .focus_chain
        .iter()
        .find(|n| n.control_type == UIA_DOCUMENT);
    let key = document.map(|d| {
        format!(
            "{}\u{1}{}",
            d.name.as_deref().unwrap_or_default(),
            d.value.as_deref().unwrap_or_default()
        )
    });
    if key == *page {
        return;
    }
    *page = key;
    // Events wait while the page is captured, which on a big page takes long.
    let started = std::time::Instant::now();
    match crate::actions::capture_foreground_page() {
        Ok(root) => {
            let title = document.and_then(|d| d.name.clone()).unwrap_or_default();
            let ms = started.elapsed().as_millis();
            tracing::debug!(%title, ms, "page loaded");
            let _ = sender.send(AccessibilityEvent::DocumentLoaded { title, root });
        }
        Err(e) => tracing::warn!("page capture failed: {e}"),
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
