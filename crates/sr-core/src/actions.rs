//! Acting on the real application behind the virtual buffer.
//!
//! The virtual cursor reads a copy of the page. Commands that must reach the page itself
//! (Enter on an edit puts real keyboard focus in it; Enter or Space on a button clicks it)
//! go through [`ApplicationActions`], implemented per platform. Implementations must return
//! quickly: the engine calls them while handling a key, inside the keyboard hook.

use crate::aom::AomNode;

/// Platform actions on the live element a buffer node was captured from.
pub trait ApplicationActions: Send {
    /// Requests keyboard focus on the node's element. Returns false when the node has no
    /// live counterpart.
    fn focus(&self, node: &AomNode) -> bool;

    /// Requests the element's default action (click a button or link, toggle a check box).
    fn activate(&self, node: &AomNode) -> bool;

    /// Asks for the text at the focused edit's caret once the application has handled the
    /// navigation key being passed to it. The text arrives later as
    /// [`crate::event::AccessibilityEvent::CaretText`]. Returns false when unsupported.
    fn read_at_caret(&self, unit: crate::event::CaretUnit) -> bool {
        let _ = unit;
        false
    }

    /// Puts `text` on the system clipboard (the Virtual Viewer's Ctrl+C). Returns false when
    /// unsupported.
    fn copy_text(&self, text: &str) -> bool {
        let _ = text;
        false
    }
}

/// Records requested actions; for tests and headless runs.
#[derive(Debug, Default, Clone)]
pub struct RecordingActions {
    log: std::sync::Arc<std::sync::Mutex<Vec<(String, usize)>>>,
}

impl RecordingActions {
    /// The requested actions so far, as `(action, node id)`.
    pub fn requests(&self) -> Vec<(String, usize)> {
        self.log.lock().map(|l| l.clone()).unwrap_or_default()
    }

    fn record(&self, action: &str, node: &AomNode) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((action.to_owned(), node.id));
        }
        true
    }
}

impl ApplicationActions for RecordingActions {
    fn focus(&self, node: &AomNode) -> bool {
        self.record("focus", node)
    }

    fn activate(&self, node: &AomNode) -> bool {
        self.record("activate", node)
    }

    fn read_at_caret(&self, unit: crate::event::CaretUnit) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((format!("caret {unit:?}"), 0));
        }
        true
    }

    fn copy_text(&self, text: &str) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((format!("copy {text}"), 0));
        }
        true
    }
}
