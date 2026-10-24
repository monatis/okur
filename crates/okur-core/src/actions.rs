//! Acting on the real application behind the virtual buffer.
//!
//! The virtual cursor reads a copy of the page. Commands that must reach the page itself
//! (Enter on an edit puts real keyboard focus in it; Enter or Space on a button clicks it)
//! go through [`ApplicationActions`], implemented per platform. Implementations must return
//! quickly: the engine calls them while handling a key, inside the keyboard hook.

use crate::aom::AomNode;

/// A click of the mouse, as JAWS's system tray list offers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseClick {
    LeftSingle,
    RightSingle,
    LeftDouble,
    /// A button pressed and held, or let go: the mouse button locks.
    LeftDown,
    LeftUp,
    RightDown,
    RightUp,
}

/// What Insert+Space, O recognizes (JAWS's Convenient OCR layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecognitionArea {
    /// The window in front (W, `RecognizeRealWindow`).
    Window,
    /// Every screen (S, `RecognizeScreen`).
    Screen,
    /// A rectangle in desktop pixels: the focused control's (C, `RecognizeControl`).
    Rect(i32, i32, i32, i32),
}

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
    /// Asks for the applications' top-level windows; they come back as
    /// [`crate::event::AccessibilityEvent::TopWindows`]. False when unsupported.
    fn list_windows(&self) -> bool {
        false
    }

    /// Brings window `id` (a [`crate::event::TopWindow`]'s) to the front, restored if minimized.
    fn switch_to_window(&self, id: u64) -> bool {
        let _ = id;
        false
    }

    /// Asks for the notification area's icons; they come back as
    /// [`crate::event::AccessibilityEvent::TrayIcons`]. False when unsupported.
    fn list_tray_icons(&self) -> bool {
        false
    }

    /// Clicks the mouse at `x`, `y` in screen pixels (a system tray icon chosen in its list).
    /// False when unsupported.
    fn click_mouse(&self, x: i32, y: i32, click: MouseClick) -> bool {
        let _ = (x, y, click);
        false
    }

    /// Reads what the mouse points to at `x`, `y` in screen pixels, answered with
    /// [`crate::AccessibilityEvent::MousePoint`] (mouse echo). False when unsupported.
    fn read_at_point(&self, x: i32, y: i32) -> bool {
        let _ = (x, y);
        false
    }

    /// Asks for the text the accessibility interfaces place on the window in front, each piece
    /// where it is drawn, for the Okur cursor. It arrives later as
    /// [`crate::event::AccessibilityEvent::ScreenText`], always: empty when nothing could be
    /// read. Returns false when unsupported.
    fn read_screen(&self) -> bool {
        false
    }

    /// Asks for the words recognized from the screen's pixels in `area`, with the system's
    /// recognizer for `language` (a BCP 47 tag, "en-US"), else for the user's languages. They
    /// arrive later as [`crate::event::AccessibilityEvent::Recognized`], always: empty when
    /// none was recognized. Returns false when unsupported.
    fn recognize(&self, area: RecognitionArea, language: &str) -> bool {
        let _ = (area, language);
        false
    }

    /// Asks for the text recognized in the picture or PDF selected in the window in front (File
    /// Explorer's), with the system's recognizer for `language`. It arrives later as
    /// [`crate::event::AccessibilityEvent::FileRecognized`], always. Returns false when
    /// unsupported.
    fn recognize_selected_file(&self, language: &str) -> bool {
        let _ = language;
        false
    }

    /// Stops a file's recognition under way at its next page.
    fn cancel_file_recognition(&self) {}

    /// Drags with the left button from `from` to `to`, in screen pixels (drag and drop).
    /// False when unsupported.
    fn drag_mouse(&self, from: (i32, i32), to: (i32, i32)) -> bool {
        let _ = (from, to);
        false
    }

    fn read_window(&self, window: bool) -> bool {
        let _ = window;
        false
    }

    /// Asks for the elements of the page in front with these DOM ids, each with its subtree,
    /// as they are now: what a page shows in its own controls (Google Docs' toolbar shows
    /// the formatting at the caret). They arrive later as
    /// [`crate::event::AccessibilityEvent::PageElements`], always: empty when none was found.
    /// Returns false when unsupported (recorded traces hold no such reads).
    fn read_page_elements(&self, ids: &[&str]) -> bool {
        let _ = ids;
        false
    }

    /// Puts `text` on the system clipboard (the Virtual Viewer's Ctrl+C). Returns false when
    /// unsupported.
    fn copy_text(&self, text: &str) -> bool {
        let _ = text;
        false
    }

    /// Moves the mouse pointer to `x`, `y` in screen pixels (routing the JAWS cursor). Returns
    /// false when unsupported.
    fn move_mouse(&self, x: i32, y: i32) -> bool {
        let _ = (x, y);
        false
    }

    /// The clipboard's text, read now (the clipboard keys). `None` when it holds none or
    /// when unsupported.
    fn clipboard_text(&self) -> Option<String> {
        None
    }

    /// An application copied `text` from its field (Ctrl+C, Ctrl+X): the application filled
    /// the system clipboard itself, so nothing is done; a replay, which has no application,
    /// keeps it as its clipboard.
    fn application_copied(&self, text: &str) {
        let _ = text;
    }

    /// Puts the focused text field's caret at character `offset` of `context`, the field as last
    /// read: the sentence keys move it. The move comes back as the field's caret event. Returns
    /// false when unsupported or not a field it can move.
    fn move_caret(&self, context: &crate::event::TextContext, offset: usize) -> bool {
        let _ = (context, offset);
        false
    }

    /// Puts the mouse on the focused text field's character at `offset` of `context`, the
    /// field as last read: the Okur cursor's place. False when unsupported or not a field it
    /// can find the character of.
    fn move_mouse_to_text(&self, context: &crate::event::TextContext, offset: usize) -> bool {
        let _ = (context, offset);
        false
    }

    /// Puts the mouse on a page element's first line, at its start: the Okur cursor's place
    /// on a page. False when unsupported.
    fn move_mouse_to_node(&self, node: &AomNode) -> bool {
        let _ = node;
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

    /// Captures the page in front again and delivers it as
    /// [`crate::event::AccessibilityEvent::DocumentLoaded`]: the focus landed on an element
    /// the buffer does not hold yet (WhatsApp's search results, built as the search box is
    /// typed in). Returns false when unsupported.
    fn refresh_page(&self) -> bool {
        false
    }

    /// Captures the page in front again after it changed
    /// ([`crate::event::AccessibilityEvent::PageChanged`]), and delivers it as
    /// [`crate::event::AccessibilityEvent::DocumentLoaded`]: once its changes settle, or at
    /// once (`now`) when they are over (they came while forms mode was on). Returns false
    /// when unsupported.
    fn page_changed(&self, now: bool) -> bool {
        let _ = now;
        false
    }
}

/// Records requested actions; for tests and headless runs.
#[derive(Debug, Default, Clone)]
pub struct RecordingActions {
    log: std::sync::Arc<std::sync::Mutex<Vec<(String, usize)>>>,
    /// Chords sent to the application, until taken: a replay hands them back to the engine
    /// as the keyboard hook does live.
    pressed: std::sync::Arc<std::sync::Mutex<Vec<Vec<crate::input::Key>>>>,
    /// Takes `read_window`, as a live platform does. Replay leaves it off: recorded traces
    /// hold no window text to answer with.
    windows: bool,
    /// Takes `read_page_elements`, as a live platform does; replay leaves it off too.
    page_elements: bool,
    /// Takes `refresh_page`, as a live platform does; replay leaves it off too.
    pages: bool,
    /// What the clipboard holds: what `copy_text` put there, or what a replay recorded.
    clipboard: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl RecordingActions {
    /// Records actions and takes `read_window` requests; the test delivers the text.
    pub fn reading_windows() -> Self {
        Self {
            windows: true,
            ..Self::default()
        }
    }

    /// Records actions and takes `read_page_elements` requests; the test delivers the
    /// elements.
    pub fn reading_page_elements() -> Self {
        Self {
            page_elements: true,
            ..Self::default()
        }
    }

    /// Records actions and takes `refresh_page` requests; the test delivers the page.
    pub fn refreshing_pages() -> Self {
        Self {
            pages: true,
            ..Self::default()
        }
    }

    /// Puts `text` on the recorded clipboard, as the application's Ctrl+C did in a
    /// recording (a step's captured clipboard).
    pub fn set_clipboard(&self, text: &str) {
        if let Ok(mut clipboard) = self.clipboard.lock() {
            *clipboard = Some(text.to_owned());
        }
    }

    /// The chords sent to the application since last asked, emptied.
    pub fn take_pressed(&self) -> Vec<Vec<crate::input::Key>> {
        self.pressed
            .lock()
            .map(|mut pressed| std::mem::take(&mut *pressed))
            .unwrap_or_default()
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

    /// Taken where windows are read ([`RecordingActions::reading_windows`]); replay's traces hold no
    /// window list to answer with.
    fn list_windows(&self) -> bool {
        if self.windows {
            if let Ok(mut log) = self.log.lock() {
                log.push(("list windows".to_owned(), 0));
            }
        }
        self.windows
    }

    fn switch_to_window(&self, id: u64) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((
                "switch to window".to_owned(),
                usize::try_from(id).unwrap_or(usize::MAX),
            ));
        }
        true
    }

    /// Taken where windows are read, as [`RecordingActions::list_windows`]: replay's traces
    /// hold no tray icons to answer with.
    fn list_tray_icons(&self) -> bool {
        if self.windows {
            if let Ok(mut log) = self.log.lock() {
                log.push(("list tray icons".to_owned(), 0));
            }
        }
        self.windows
    }

    fn click_mouse(&self, x: i32, y: i32, click: MouseClick) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((
                format!("{click:?} click at {x}, {y}"),
                usize::try_from(x).unwrap_or(0),
            ));
        }
        true
    }

    fn read_page_elements(&self, ids: &[&str]) -> bool {
        if self.page_elements {
            if let Ok(mut log) = self.log.lock() {
                log.push((format!("read page elements {}", ids.join(" ")), 0));
            }
        }
        self.page_elements
    }

    fn move_caret(&self, _context: &crate::event::TextContext, offset: usize) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push(("caret to".to_owned(), offset));
        }
        true
    }

    fn move_mouse_to_text(&self, _context: &crate::event::TextContext, offset: usize) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push(("mouse to text".to_owned(), offset));
        }
        true
    }

    fn move_mouse_to_node(&self, node: &AomNode) -> bool {
        self.record("mouse to node", node)
    }

    fn copy_text(&self, text: &str) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((format!("copy {text}"), 0));
        }
        self.set_clipboard(text);
        true
    }

    fn clipboard_text(&self) -> Option<String> {
        self.clipboard.lock().ok().and_then(|c| c.clone())
    }

    fn application_copied(&self, text: &str) {
        self.set_clipboard(text);
    }

    fn move_mouse(&self, x: i32, y: i32) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((format!("mouse to {x}, {y}"), 0));
        }
        true
    }

    fn read_screen(&self) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push(("read screen".to_owned(), 0));
        }
        true
    }

    fn recognize(&self, area: RecognitionArea, _language: &str) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((format!("recognize {area:?}"), 0));
        }
        true
    }

    fn recognize_selected_file(&self, _language: &str) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push(("recognize selected file".to_owned(), 0));
        }
        true
    }

    fn cancel_file_recognition(&self) {
        if let Ok(mut log) = self.log.lock() {
            log.push(("cancel file recognition".to_owned(), 0));
        }
    }

    fn press(&self, chord: &[crate::input::Key]) -> bool {
        if let Ok(mut log) = self.log.lock() {
            log.push((format!("press {chord:?}"), 0));
        }
        if let Ok(mut pressed) = self.pressed.lock() {
            pressed.push(chord.to_vec());
        }
        true
    }

    fn refresh_page(&self) -> bool {
        self.page_request("refresh page")
    }

    fn page_changed(&self, now: bool) -> bool {
        self.page_request(if now {
            "page changed, now"
        } else {
            "page changed"
        })
    }
}

impl RecordingActions {
    fn page_request(&self, action: &str) -> bool {
        if self.pages {
            if let Ok(mut log) = self.log.lock() {
                log.push((action.to_owned(), 0));
            }
        }
        self.pages
    }
}
