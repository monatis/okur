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

    /// Asks for the focused field's text, caret and selection once the application has
    /// handled the key being passed to it. `before` is the field as the engine last knew it:
    /// the application may act on the key before the platform's first read, so the read
    /// compares with it to see the key's effect. The text arrives later as
    /// [`crate::event::AccessibilityEvent::TextContext`]. `in_page` says the focus is in a
    /// page (a browser's or an Electron app's document), whose fields the platform reads
    /// through the page; a browser's own controls (its address bar) are read like any other
    /// application's. Returns false when unsupported.
    fn read_at_caret(&self, before: Option<&crate::event::TextContext>, in_page: bool) -> bool {
        let _ = (before, in_page);
        false
    }

    /// Asks for the focused element's text, caret and selection as they are now (a
    /// console's buffer, for the Virtual Viewer). It arrives later as
    /// [`crate::event::AccessibilityEvent::TextContext`], always: with empty text when the
    /// focus has none. Returns false when unsupported.
    fn read_text(&self) -> bool {
        false
    }

    /// Asks for the Virtual Viewer's lines for the window in front (`window`) or for the
    /// focused control, as they are on screen now. They arrive later as
    /// [`crate::event::AccessibilityEvent::ViewerText`], always: empty when nothing could be
    /// read. Returns false when unsupported (recorded traces hold no window text, so replay
    /// keeps the viewer built from the focus).
    fn read_window(&self, window: bool) -> bool {
        let _ = window;
        false
    }

    /// Puts `text` on the system clipboard (the Virtual Viewer's Ctrl+C). Returns false when
    /// unsupported.
    fn copy_text(&self, text: &str) -> bool {
        let _ = text;
        false
    }

    /// Sends a key chord to the application, pressed in order and released in reverse: a
    /// reading command that moves the caret is the application's own key (the laptop
    /// layout's CapsLock+O is Down). The keys come back through the keyboard hook marked as
    /// the screen reader's, so they are read as plain keys. Returns false when unsupported.
    fn press(&self, chord: &[crate::input::Key]) -> bool {
        let _ = chord;
        false
    }
}

/// Records requested actions; for tests and headless runs.
#[derive(Debug, Default, Clone)]
pub struct RecordingActions {
    log: std::sync::Arc<std::sync::Mutex<Vec<(String, usize)>>>,
    /// Takes `read_window`, as a live platform does. Replay leaves it off: recorded traces
    /// hold no window text to answer with.
    windows: bool,
}

impl RecordingActions {
    /// Records actions and takes `read_window` requests; the test delivers the text.
    pub fn reading_windows() -> Self {
        Self {
            windows: true,
            ..Self::default()
        }
    }

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

    fn read_at_caret(&self, _before: Option<&crate::event::TextContext>, _in_page: bool) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push(("caret".to_owned(), 0));
        }
        true
    }

    fn read_text(&self) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push(("read text".to_owned(), 0));
        }
        true
    }

    fn read_window(&self, window: bool) -> bool {
        if self.windows {
            if let Ok(mut log) = self.log.lock() {
                log.push((format!("read window {window}"), 0));
            }
        }
        self.windows
    }

    fn copy_text(&self, text: &str) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((format!("copy {text}"), 0));
        }
        true
    }

    fn press(&self, chord: &[crate::input::Key]) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((format!("press {chord:?}"), 0));
        }
        true
    }
}
