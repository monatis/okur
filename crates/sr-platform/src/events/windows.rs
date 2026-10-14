//! Windows accessibility events for the live screen reader.
//!
//! Events come from the WinEvent capture that `sr-cli trace` records
//! ([`crate::event_trace::run_live`]). They are converted by the functions parity replay uses
//! ([`focus_event_from_trace`], [`menu_event_from_trace`]), so the live screen reader consumes
//! the event stream the harness records, replays and verifies against JAWS.

use super::AccessibilityEventListener;
use crate::event_trace::{run_live, EventConverter, TraceEvent};
use crate::PlatformError;
use sr_core::AccessibilityEvent;
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
        let thread = std::thread::Builder::new()
            .name("sr-winevents".to_owned())
            .spawn(move || {
                let mut page: Option<String> = None;
                let mut converter = EventConverter::default();
                let sink =
                    move |event: TraceEvent| deliver(&event, &mut converter, &mut page, &sender);
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
) {
    let Some(converted) = converter.convert(event) else {
        if event.event == "focus" {
            tracing::debug!(class = ?event.window_class, "focus event without a usable focus chain");
        }
        return;
    };
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
    match crate::actions::capture_foreground_page() {
        Ok(root) => {
            let title = document.and_then(|d| d.name.clone()).unwrap_or_default();
            tracing::debug!(%title, "page loaded");
            let _ = sender.send(AccessibilityEvent::DocumentLoaded { title, root });
        }
        Err(e) => tracing::warn!("page capture failed: {e}"),
    }
}
