//! [`okur_core::actions::ApplicationActions`] for Chromium pages, through MSAA/IA2.
//!
//! Buffer nodes mapped from IA2 carry the object's IA2 `uniqueID`
//! ([`UNIQUE_ID_ATTR`]). An action resolves it with `accChild(uniqueID)` on the foreground
//! window's document (Chromium's IDs are negative, which makes them valid child ids) and
//! calls `accSelect(SELFLAG_TAKEFOCUS)` or `accDoDefaultAction`. The calls cross into the browser process, so they run on a worker
//! thread: the engine calls actions from the keyboard hook, which must return at once.

pub use okur_core::aom::UNIQUE_ID_ATTR;

/// AOM attribute holding a node's UIA RuntimeId (comma-separated), for a page read through
/// UIA, whose nodes have no IA2 `uniqueID`.
pub const UIA_RUNTIME_ID_ATTR: &str = "uia_runtime_id";

/// The UIA RuntimeId recorded on a node, if any.
pub fn runtime_id(node: &okur_core::AomNode) -> Option<Vec<i32>> {
    node.attributes
        .get(UIA_RUNTIME_ID_ATTR)?
        .split(',')
        .map(|part| part.parse().ok())
        .collect()
}

/// The IA2 `uniqueID` recorded on a node, if any.
pub fn unique_id(node: &okur_core::AomNode) -> Option<i32> {
    node.attributes.get(UNIQUE_ID_ATTR)?.parse().ok()
}

/// Whether two reads of a field have the same text, caret and selection: what a key
/// changes. The font is left out: the read the engine knows can carry what a poll does not
/// look up (a page paragraph's alignment, `text_context::with_paragraph_alignment`).
fn same_place(a: &Option<okur_core::TextContext>, b: &Option<okur_core::TextContext>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.text == b.text && a.caret == b.caret && a.selection == b.selection,
        (None, None) => true,
        _ => false,
    }
}

#[cfg(windows)]
pub use windows_actions::WindowsActions;

#[cfg(windows)]
mod windows_actions {
    use super::{runtime_id, unique_id};
    use crate::text_context::{focus_has_taken_its_keys, read_focused, read_focused_outside_page};
    use okur_core::{AccessibilityEvent, TextContext};
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc::UnboundedSender;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::UI::Accessibility::IUIAutomation;
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use xa11y_windows::ia2::{has_chromium_document, perform_chromium_action, Ia2Action};

    /// How long to wait for the application to act on a key.
    const SETTLE: Duration = Duration::from_millis(250);
    const POLL: Duration = Duration::from_millis(15);
    /// How often a page's field is polled while a key takes effect: a poll is a few calls.
    const FIELD_POLL: Duration = Duration::from_millis(2);
    /// The application's head start on the first read: an idle application has acted on a
    /// key within a millisecond or two of the hook passing it on.
    const HEAD_START: Duration = Duration::from_millis(3);
    /// How long a page action waits for a page to be in front.
    const PAGE_WAIT: Duration = Duration::from_millis(500);

    /// The window in front, for a page action. An action asked for as one of Okur's own
    /// windows closes (OK in Select a Form Field) finds that window still in front for some
    /// tens of milliseconds, so a page is waited for (old-pc, 2026-10-18).
    fn page_in_front() -> HWND {
        let started = Instant::now();
        loop {
            // SAFETY: plain Win32 call with no preconditions.
            let window = unsafe { GetForegroundWindow() };
            if has_chromium_document(window) || started.elapsed() >= PAGE_WAIT {
                return window;
            }
            std::thread::sleep(POLL);
        }
    }

    enum Request {
        Object(i32, Ia2Action),
        /// The same, for a page object read through UIA, found by its RuntimeId.
        UiaObject(Vec<i32>, Ia2Action),
        /// The field's text once the application has acted on a key, compared with the
        /// field as the engine last knew it.
        Text(Option<Box<TextContext>>, bool),
        /// The focused text as it is now, answered even when there is none.
        TextNow,
        /// The caret to a cell of the focused document's table (row, column, from 1), then
        /// the field's text, compared with the field as the engine last knew it.
        Cell(Option<Box<TextContext>>, usize, usize),
        /// The focused document's headings, answered even when there are none.
        DocumentHeadings,
        /// The focused document's links, answered even when there are none.
        DocumentLinks,
        /// The line spacing at the focused document's caret, answered always.
        LineSpacing,
        /// What one of Word's keys left in the focused document, read once Word has acted on
        /// it, answered always.
        WordKey(okur_core::WordKey),
        /// The focused document's fields, answered even when there are none.
        DocumentFields,
        /// The caret to a field of the focused document (from 0), then the field's text.
        Field(Option<Box<TextContext>>, usize),
        /// The focused document's comments or footnotes, answered even when there are none.
        DocumentNotes(okur_core::NoteKind),
        /// The caret to a comment's or footnote's mark in the focused document (from 0), then
        /// the field's text.
        Note(Option<Box<TextContext>>, okur_core::NoteKind, usize),
        /// The focused document's link (from 0, as its links are listed) followed.
        ActivateDocumentLink(usize),
        /// The focused document's paragraphs, answered even when there are none.
        DocumentParagraphs,
        /// The view Word's window in front shows its document in, answered always.
        DocumentView,
        StatusBar,
        /// The caret to a paragraph of the focused document (from 0), then the field's text.
        Paragraph(Option<Box<TextContext>>, usize),
        /// A chord of virtual keys, sent marked as the screen reader's own.
        Keys(Vec<u16>),
        /// The Virtual Viewer's lines for the window in front (`true`) or the focused
        /// control, answered even when nothing could be read.
        Window(bool),
        /// The page's elements with these DOM ids, answered even when none was found.
        PageElements(Vec<String>),
        /// Text for the clipboard. Not set in the keyboard hook: emptying the clipboard tells
        /// its last owner at once, and a hung owner would hold the hook (responsiveness
        /// review, 2026-10-19).
        Copy(String),
        /// The focused edit's caret to a character offset of its text as last read.
        Caret(String, usize),
        /// The applications' top-level windows, answered even when there are none.
        ListWindows,
        /// A window to the front.
        SwitchTo(u64),
        /// The notification area's icons, answered even when there are none.
        ListTray,
        /// What the mouse points to at a point of the screen (mouse echo), answered when
        /// something is there.
        Point(i32, i32),
        /// The mouse onto the focused edit's character at an offset of its text as last read
        /// (the Okur cursor).
        MouseToText(String, usize),
        /// The mouse onto a page object, by its IA2 uniqueID or, read through UIA, its
        /// RuntimeId (the Okur cursor on a page).
        MouseToObject(i32),
        MouseToUiaObject(Vec<i32>),
        /// A drag with the left button. Not done in the keyboard hook: it moves the pointer in
        /// steps a hand would take, 120 ms in all (performance review 2026-10-23, T3).
        Drag((i32, i32), (i32, i32)),
        /// The text the window in front shows, each piece where it is drawn (the Okur cursor's
        /// model), answered even when nothing could be read.
        Screen,
        /// The words recognized in an area of the screen, in a language, answered even when
        /// none was.
        Recognize(okur_core::actions::RecognitionArea, String),
        /// The text recognized in the file selected in the window in front, in a language,
        /// answered always.
        RecognizeFile(String),
    }

    impl Request {
        /// Whether the request is a long read for the bulk lane.
        fn is_bulk(&self) -> bool {
            matches!(
                self,
                Request::Screen
                    | Request::Recognize(..)
                    | Request::RecognizeFile(_)
                    | Request::Window(_)
                    | Request::PageElements(_)
                    | Request::Point(..)
                    | Request::ListWindows
                    | Request::ListTray
                    | Request::Drag(..)
            )
        }

        /// The request's kind, for the log.
        fn kind(&self) -> &'static str {
            match self {
                Request::Object(..) | Request::UiaObject(..) => "page action",
                Request::Text(..) | Request::TextNow => "text read",
                Request::Cell(..) => "table cell",
                Request::DocumentHeadings => "document headings",
                Request::DocumentLinks => "document links",
                Request::LineSpacing => "line spacing",
                Request::WordKey(_) => "word key",
                Request::DocumentFields => "document fields",
                Request::Field(..) => "field",
                Request::DocumentNotes(_) => "document notes",
                Request::Note(..) => "note",
                Request::ActivateDocumentLink(_) => "document link",
                Request::DocumentParagraphs => "document paragraphs",
                Request::DocumentView => "document view",
                Request::StatusBar => "status bar",
                Request::Paragraph(..) => "paragraph",
                Request::Keys(_) => "keys",
                Request::Window(_) => "viewer read",
                Request::PageElements(_) => "page elements",
                Request::Copy(_) => "copy",
                Request::Caret(..) => "caret",
                Request::ListWindows => "window list",
                Request::SwitchTo(_) => "switch",
                Request::ListTray => "tray list",
                Request::Point(..) => "mouse point",
                Request::MouseToText(..)
                | Request::MouseToObject(_)
                | Request::MouseToUiaObject(_) => "mouse move",
                Request::Drag(..) => "drag",
                Request::Screen => "screen read",
                Request::Recognize(..) => "recognition",
                Request::RecognizeFile(_) => "file recognition",
            }
        }
    }

    /// The virtual key of a key the screen reader sends for a reading command.
    fn virtual_key(key: &okur_core::Key) -> Option<u16> {
        use okur_core::Key;
        Some(match key {
            Key::UpArrow => 0x26,
            Key::DownArrow => 0x28,
            Key::LeftArrow => 0x25,
            Key::RightArrow => 0x27,
            Key::Home => 0x24,
            Key::End => 0x23,
            Key::PageUp => 0x21,
            Key::PageDown => 0x22,
            Key::Control => 0x11,
            Key::Shift => 0x10,
            _ => return None,
        })
    }

    /// Acts on the foreground application from a COM worker thread: page objects by IA2
    /// uniqueID, and the focused text field's text, caret and selection after edit keys.
    pub struct WindowsActions {
        /// The keys, the text reads after them, the caret and page actions: what a key waits
        /// for.
        requests: Mutex<Sender<Request>>,
        /// The long reads (the window's text, recognition, the viewer's, the mouse's point,
        /// window and tray lists, drags), on a lane of their own, so that none of them comes
        /// before an edit key's text read (performance review 2026-10-24, 2).
        bulk: Mutex<Sender<Request>>,
    }

    impl WindowsActions {
        /// `events` receives the text contexts read after edit keys.
        pub fn start(events: Option<UnboundedSender<AccessibilityEvent>>) -> Self {
            let (tx, rx) = channel::<Request>();
            let (bulk_tx, bulk_rx) = channel::<Request>();
            let bulk_events = events.clone();
            std::thread::spawn(move || Self::serve(&bulk_rx, bulk_events.as_ref()));
            std::thread::spawn(move || Self::serve(&rx, events.as_ref()));
            Self {
                requests: Mutex::new(tx),
                bulk: Mutex::new(bulk_tx),
            }
        }

        /// One lane of the worker: its requests in order, each in a panic guard, so one that
        /// panics is logged and the lane goes on (performance review 2026-10-24, 5).
        fn serve(rx: &Receiver<Request>, events: Option<&UnboundedSender<AccessibilityEvent>>) {
            // SAFETY: COM initialization for this worker thread.
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            // UI Automation's default lets an application take 20 s over an answer, and
            // every read and injected key queued behind it waited (performance review
            // 2026-10-23, P6); 5 s is long enough for a big window's whole tree.
            let uia: Option<IUIAutomation> =
                crate::uia_snapshot::capture::automation_within(2000, 5000).ok();
            while let Ok(first) = rx.recv() {
                // Keys arrive faster than a read settles (key repeat): of the text reads
                // queued meanwhile only the newest matters, since the engine speaks for
                // the latest key. Page actions still run, in order.
                let mut batch = vec![first];
                batch.extend(rx.try_iter());
                // The same for the mouse's point, the window's text and a recognition: the
                // newest answers for all (performance review 2026-10-24, 2).
                let last = |kind: fn(&Request) -> bool| batch.iter().rposition(kind);
                let newest = [
                    last(|r| matches!(r, Request::Text(..))),
                    last(|r| matches!(r, Request::Point(..))),
                    last(|r| matches!(r, Request::Screen)),
                    last(|r| matches!(r, Request::Recognize(..))),
                ];
                let batch = batch.into_iter().enumerate().filter_map(|(i, r)| {
                    let coalesced = matches!(
                        r,
                        Request::Text(..)
                            | Request::Point(..)
                            | Request::Screen
                            | Request::Recognize(..)
                    );
                    (!coalesced || newest.contains(&Some(i))).then_some(r)
                });
                for request in batch {
                    let kind = request.kind();
                    let served = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        match request {
                            Request::Keys(keys) => {
                                use crate::hook::windows::INJECTED_MARK;
                                for &vk in &keys {
                                    crate::key_input::send_key(vk, false, INJECTED_MARK);
                                }
                                for &vk in keys.iter().rev() {
                                    crate::key_input::send_key(vk, true, INJECTED_MARK);
                                }
                            }
                            Request::Object(id, action) => {
                                let window = page_in_front();
                                if let Err(e) = perform_chromium_action(window, id, action) {
                                    tracing::warn!("{action:?} on IA2 object {id} failed: {e}");
                                }
                            }
                            Request::UiaObject(id, action) => {
                                let Some(uia) = uia.as_ref() else {
                                    return;
                                };
                                if let Err(e) = perform_uia_action(uia, &id, action) {
                                    tracing::warn!("{action:?} on UIA object {id:?} failed: {e}");
                                }
                            }
                            Request::Text(before, in_page) => {
                                let (Some(uia), Some(events)) = (uia.as_ref(), events.as_ref())
                                else {
                                    tracing::warn!(
                                        "text read impossible: no UIA client or channel"
                                    );
                                    return;
                                };
                                let started = Instant::now();
                                let context = read_after_key(uia, before.map(|b| *b), in_page);
                                // Offsets, not the text: a document's whole text on every key
                                // made the debug log huge.
                                tracing::debug!(
                                    chars = context.as_ref().map(|c| c.text.chars().count()),
                                    caret = context.as_ref().map(|c| c.caret),
                                    selection = ?context.as_ref().and_then(|c| c.selection),
                                    word = ?context.as_ref().and_then(|c| c.word),
                                    ms = started.elapsed().as_millis(),
                                    "text context read"
                                );
                                if let Some(context) = context {
                                    let _ = events.send(AccessibilityEvent::TextContext(context));
                                }
                            }
                            Request::Cell(before, row, column) => {
                                let (Some(uia), Some(events)) = (uia.as_ref(), events.as_ref())
                                else {
                                    return;
                                };
                                if !crate::text_context::move_to_cell(uia, row, column) {
                                    tracing::debug!(row, column, "caret not moved to the cell");
                                    return;
                                }
                                if let Some(context) =
                                    read_after_key(uia, before.map(|b| *b), false)
                                {
                                    let _ = events.send(AccessibilityEvent::TextContext(context));
                                }
                            }
                            Request::DocumentHeadings => {
                                let Some(events) = events.as_ref() else {
                                    return;
                                };
                                let started = Instant::now();
                                let headings = uia
                                    .as_ref()
                                    .and_then(|uia| {
                                        crate::text_context::word_document_headings(uia, None)
                                    })
                                    .unwrap_or_default();
                                tracing::debug!(
                                    headings = headings.len(),
                                    ms = started.elapsed().as_millis(),
                                    "document headings read"
                                );
                                let _ = events.send(AccessibilityEvent::DocumentHeadings(headings));
                            }
                            Request::DocumentLinks => {
                                let Some(events) = events.as_ref() else {
                                    return;
                                };
                                let started = Instant::now();
                                let links = uia
                                    .as_ref()
                                    .and_then(|uia| {
                                        crate::text_context::word_document_links(uia, None)
                                    })
                                    .unwrap_or_default();
                                tracing::debug!(
                                    links = links.len(),
                                    ms = started.elapsed().as_millis(),
                                    "document links read"
                                );
                                let _ = events.send(AccessibilityEvent::DocumentLinks(links));
                            }
                            Request::LineSpacing => {
                                let Some(events) = events.as_ref() else {
                                    return;
                                };
                                let started = Instant::now();
                                let spacing = crate::text_context::word_line_spacing();
                                tracing::debug!(
                                    ?spacing,
                                    ms = started.elapsed().as_millis(),
                                    "line spacing read"
                                );
                                let _ = events.send(AccessibilityEvent::LineSpacing(spacing));
                            }
                            Request::WordKey(key) => {
                                let Some(events) = events.as_ref() else {
                                    return;
                                };
                                let now =
                                    after_word_key(|| crate::text_context::word_key_state(key));
                                tracing::debug!(?key, ?now, "word key read");
                                let _ = events.send(AccessibilityEvent::WordKeyState(now));
                            }
                            Request::DocumentFields => {
                                let Some(events) = events.as_ref() else {
                                    return;
                                };
                                let started = Instant::now();
                                let fields =
                                    crate::text_context::word_document_fields().unwrap_or_default();
                                tracing::debug!(
                                    fields = fields.len(),
                                    ms = started.elapsed().as_millis(),
                                    "document fields read"
                                );
                                let _ = events.send(AccessibilityEvent::DocumentFields(fields));
                            }
                            Request::Field(before, index) => {
                                let (Some(uia), Some(events)) = (uia.as_ref(), events.as_ref())
                                else {
                                    return;
                                };
                                if !crate::text_context::word_select_field(index) {
                                    tracing::debug!(index, "caret not moved to the field");
                                    return;
                                }
                                if let Some(context) =
                                    read_after_key(uia, before.map(|b| *b), false)
                                {
                                    let _ = events.send(AccessibilityEvent::TextContext(context));
                                }
                            }
                            Request::DocumentNotes(kind) => {
                                let Some(events) = events.as_ref() else {
                                    return;
                                };
                                let started = Instant::now();
                                let notes = crate::text_context::word_document_notes(kind)
                                    .unwrap_or_default();
                                tracing::debug!(
                                    ?kind,
                                    notes = notes.len(),
                                    ms = started.elapsed().as_millis(),
                                    "document notes read"
                                );
                                let _ = events.send(AccessibilityEvent::DocumentNotes(kind, notes));
                            }
                            Request::Note(before, kind, index) => {
                                let (Some(uia), Some(events)) = (uia.as_ref(), events.as_ref())
                                else {
                                    return;
                                };
                                if !crate::text_context::word_select_note(kind, index) {
                                    tracing::debug!(?kind, index, "caret not moved to the note");
                                    return;
                                }
                                if let Some(context) =
                                    read_after_key(uia, before.map(|b| *b), false)
                                {
                                    let _ = events.send(AccessibilityEvent::TextContext(context));
                                }
                            }
                            Request::ActivateDocumentLink(index) => {
                                let followed = uia.as_ref().is_some_and(|uia| {
                                    crate::text_context::activate_word_document_link(uia, index)
                                });
                                tracing::debug!(index, followed, "document link followed");
                            }
                            Request::DocumentParagraphs => {
                                let Some(events) = events.as_ref() else {
                                    return;
                                };
                                let started = Instant::now();
                                let paragraphs = uia
                                    .as_ref()
                                    .and_then(|uia| {
                                        crate::text_context::word_document_paragraphs(uia, None)
                                    })
                                    .unwrap_or_default();
                                tracing::debug!(
                                    paragraphs = paragraphs.len(),
                                    ms = started.elapsed().as_millis(),
                                    "document paragraphs read"
                                );
                                let _ =
                                    events.send(AccessibilityEvent::DocumentParagraphs(paragraphs));
                            }
                            Request::DocumentView => {
                                let Some(events) = events.as_ref() else {
                                    return;
                                };
                                let view = uia
                                    .as_ref()
                                    .and_then(|uia| crate::text_context::word_view(uia, None));
                                tracing::debug!(?view, "document view read");
                                let _ = events.send(AccessibilityEvent::DocumentView(view));
                            }
                            Request::StatusBar => {
                                let Some(events) = events.as_ref() else {
                                    return;
                                };
                                let items = uia
                                    .as_ref()
                                    .and_then(|uia| crate::text_context::word_status_bar(uia, None))
                                    .unwrap_or_default();
                                tracing::debug!(items = items.len(), "status bar read");
                                let _ = events.send(AccessibilityEvent::StatusBar(items));
                            }
                            Request::Paragraph(before, paragraph) => {
                                let (Some(uia), Some(events)) = (uia.as_ref(), events.as_ref())
                                else {
                                    return;
                                };
                                if !crate::text_context::move_to_paragraph(uia, paragraph) {
                                    tracing::debug!(paragraph, "caret not moved to the paragraph");
                                    return;
                                }
                                if let Some(context) =
                                    read_after_key(uia, before.map(|b| *b), false)
                                {
                                    let _ = events.send(AccessibilityEvent::TextContext(context));
                                }
                            }
                            Request::TextNow => {
                                let Some(events) = events.as_ref() else {
                                    tracing::warn!("text read impossible: no event channel");
                                    return;
                                };
                                let started = Instant::now();
                                let context = uia.as_ref().and_then(read_focused);
                                tracing::debug!(
                                    chars = context.as_ref().map(|c| c.text.chars().count()),
                                    ms = started.elapsed().as_millis(),
                                    "focused text read"
                                );
                                // Answered even when empty: the engine waits for it.
                                let context = context.unwrap_or(okur_core::TextContext {
                                    text: String::new(),
                                    caret: 0,
                                    selection: None,
                                    font: None,
                                    word: None,
                                    misspelled: Vec::new(),
                                    structure: None,
                                });
                                let _ = events.send(AccessibilityEvent::TextContext(context));
                            }
                            Request::Window(window) => {
                                let Some(events) = events.as_ref() else {
                                    tracing::warn!("window read impossible: no event channel");
                                    return;
                                };
                                let started = Instant::now();
                                let lines = uia.as_ref().and_then(|uia| {
                                    if window {
                                        // SAFETY: plain Win32 call with no preconditions.
                                        let hwnd = unsafe { GetForegroundWindow() };
                                        crate::window_text::window_lines(uia, hwnd)
                                    } else {
                                        crate::window_text::control_lines(uia)
                                    }
                                });
                                tracing::debug!(
                                    window,
                                    lines = lines.as_ref().map(Vec::len),
                                    ms = started.elapsed().as_millis(),
                                    "viewer text read"
                                );
                                // Answered even when empty: the engine waits for it.
                                let lines = lines.unwrap_or_default();
                                let _ = events.send(AccessibilityEvent::ViewerText(lines));
                            }
                            Request::PageElements(ids) => {
                                let Some(events) = events.as_ref() else {
                                    tracing::warn!("page read impossible: no event channel");
                                    return;
                                };
                                let started = Instant::now();
                                let elements: Vec<okur_core::AomNode> = uia
                                    .as_ref()
                                    .map(|uia| {
                                        crate::uia_snapshot::capture::page_elements(uia, &ids)
                                    })
                                    .unwrap_or_default()
                                    .iter()
                                    .map(crate::uia_snapshot::to_aom)
                                    .collect();
                                tracing::debug!(
                                    asked = ids.len(),
                                    found = elements.len(),
                                    ms = started.elapsed().as_millis(),
                                    "page elements read"
                                );
                                // Answered even when none was found: the engine waits for it.
                                let _ = events.send(AccessibilityEvent::PageElements(elements));
                            }
                            Request::Drag(from, to) => super::drag(from, to),
                            Request::Screen => {
                                let Some(events) = events.as_ref() else {
                                    tracing::warn!("screen read impossible: no event channel");
                                    return;
                                };
                                let started = Instant::now();
                                // SAFETY: plain Win32 call with no preconditions.
                                let hwnd = unsafe { GetForegroundWindow() };
                                let text = uia
                                    .as_ref()
                                    .and_then(|uia| crate::window_text::window_runs(uia, hwnd))
                                    .unwrap_or_default();
                                tracing::debug!(
                                    runs = text.runs.len(),
                                    layout = ?text.layout,
                                    ms = started.elapsed().as_millis(),
                                    "screen text read"
                                );
                                // Answered even when empty: the engine keeps the window's model.
                                let _ = events.send(AccessibilityEvent::ScreenText(text));
                            }
                            Request::Recognize(area, language) => {
                                let Some(events) = events.as_ref() else {
                                    tracing::warn!("recognition impossible: no event channel");
                                    return;
                                };
                                let started = Instant::now();
                                let words =
                                    crate::ocr::recognize(area, &language).unwrap_or_else(|e| {
                                        tracing::warn!("recognition failed: {e}");
                                        Vec::new()
                                    });
                                tracing::debug!(
                                    ?area,
                                    words = words.len(),
                                    ms = started.elapsed().as_millis(),
                                    "screen recognized"
                                );
                                // Answered even when none: the engine waits for it.
                                let _ = events.send(AccessibilityEvent::Recognized(words));
                            }
                            Request::RecognizeFile(language) => {
                                let Some(events) = events.as_ref() else {
                                    tracing::warn!("recognition impossible: no event channel");
                                    return;
                                };
                                let started = Instant::now();
                                let recognition =
                                    crate::file_text::recognize_selected_file(&language, || {
                                        let _ =
                                            events.send(AccessibilityEvent::FileRecognitionStarted);
                                    });
                                tracing::debug!(
                                    lines = recognition.lines().count(),
                                    ms = started.elapsed().as_millis(),
                                    "file recognized"
                                );
                                let _ =
                                    events.send(AccessibilityEvent::FileRecognized(recognition));
                            }
                            Request::Copy(text) => {
                                if let Err(e) = super::set_clipboard_text(&text) {
                                    tracing::warn!("copy to the clipboard failed: {e}");
                                }
                            }
                            Request::MouseToText(text, offset) => {
                                if !crate::text_context::mouse_to_character_in_edit(&text, offset) {
                                    tracing::debug!(offset, "mouse not moved: not a Win32 edit");
                                }
                            }
                            Request::MouseToObject(id) => {
                                let window = page_in_front();
                                match xa11y_windows::ia2::chromium_object_location(window, id) {
                                    Ok(place) => mouse_onto(place),
                                    Err(e) => tracing::debug!("no place for IA2 object {id}: {e}"),
                                }
                            }
                            Request::MouseToUiaObject(id) => {
                                let Some(uia) = uia.as_ref() else {
                                    return;
                                };
                                match uia_object_location(uia, &id) {
                                    Ok(place) => mouse_onto(place),
                                    Err(e) => {
                                        tracing::debug!("no place for UIA object {id:?}: {e}")
                                    }
                                }
                            }
                            Request::Caret(text, offset) => {
                                if !crate::text_context::move_caret_in_edit(&text, offset) {
                                    tracing::debug!(offset, "caret not moved: not a Win32 edit");
                                }
                            }
                            Request::ListWindows => {
                                let Some(events) = events.as_ref() else {
                                    tracing::warn!("window list impossible: no event channel");
                                    return;
                                };
                                let windows = crate::top_windows::list();
                                tracing::debug!(windows = windows.len(), "windows listed");
                                let _ = events.send(AccessibilityEvent::TopWindows(windows));
                            }
                            Request::SwitchTo(id) => crate::top_windows::switch_to(id),
                            Request::Point(x, y) => {
                                let (Some(uia), Some(events)) = (uia.as_ref(), events.as_ref())
                                else {
                                    return;
                                };
                                let started = Instant::now();
                                let Some((chain, texts)) = crate::mouse_point::read(uia, x, y)
                                else {
                                    return;
                                };
                                tracing::debug!(
                                    ms = started.elapsed().as_millis(),
                                    "mouse point read"
                                );
                                let point = crate::event_trace::TracePoint { at: [x, y], texts };
                                let _ = events
                                    .send(crate::event_trace::mouse_point_event(&chain, &point));
                            }
                            Request::ListTray => {
                                let Some(events) = events.as_ref() else {
                                    tracing::warn!("tray list impossible: no event channel");
                                    return;
                                };
                                let icons = uia
                                    .as_ref()
                                    .map(crate::uia_snapshot::capture::tray_icons)
                                    .unwrap_or_default();
                                tracing::debug!(icons = icons.len(), "tray icons listed");
                                let _ = events.send(AccessibilityEvent::TrayIcons(icons));
                            }
                        }
                    }));
                    if served.is_err() {
                        tracing::error!(kind, "an action's work panicked; the next goes on");
                    }
                }
            }
        }

        fn send(&self, request: Request) -> bool {
            let lane = if request.is_bulk() {
                &self.bulk
            } else {
                &self.requests
            };
            lane.lock().is_ok_and(|tx| tx.send(request).is_ok())
        }

        /// Acts on a page node's live object: by its IA2 uniqueID, or, for a page read through
        /// UIA (while its IA2 capture is still running), by its UIA RuntimeId.
        fn act(&self, node: &okur_core::AomNode, action: Ia2Action) -> bool {
            if let Some(id) = unique_id(node) {
                return self.send(Request::Object(id, action));
            }
            runtime_id(node).is_some_and(|id| self.send(Request::UiaObject(id, action)))
        }
    }

    /// Where the UIA element with RuntimeId `id` in the window in front is on the screen:
    /// `[left, top, width, height]`.
    fn uia_object_location(uia: &IUIAutomation, id: &[i32]) -> Result<[i32; 4], String> {
        use xa11y_windows::uia_element::find_by_runtime_id;
        // SAFETY: plain Win32 and UIA calls on valid objects.
        let root =
            unsafe { uia.ElementFromHandle(GetForegroundWindow()) }.map_err(|e| e.to_string())?;
        let element = find_by_runtime_id(uia, &root, id).ok_or("not found")?;
        // SAFETY: a UIA property read on a valid element.
        let rect = unsafe { element.CurrentBoundingRectangle() }.map_err(|e| e.to_string())?;
        Ok([
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        ])
    }

    /// The mouse onto the start of a place `[left, top, width, height]`: a pixel in, half its
    /// first line down (a line taken as 20 pixels at most).
    fn mouse_onto([left, top, _width, height]: [i32; 4]) {
        use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;
        let y = top + height.clamp(0, 20) / 2;
        // SAFETY: a plain Win32 call moving the pointer.
        let _ = unsafe { SetCursorPos(left + 1, y) };
    }

    /// Focuses or activates the UIA element with RuntimeId `id` in the window in front.
    fn perform_uia_action(
        uia: &IUIAutomation,
        id: &[i32],
        action: Ia2Action,
    ) -> Result<(), String> {
        use xa11y_windows::uia_element::{do_default_action, find_by_runtime_id};
        // SAFETY: plain Win32 and UIA calls on valid objects.
        let root =
            unsafe { uia.ElementFromHandle(GetForegroundWindow()) }.map_err(|e| e.to_string())?;
        let element = find_by_runtime_id(uia, &root, id).ok_or("not found")?;
        match action {
            // SAFETY: a UIA call on a valid element.
            Ia2Action::Focus => unsafe { element.SetFocus() }.map_err(|e| e.to_string()),
            // Focus first, as IA2's default action leaves it on the object: its state
            // changes are then the focused control's, which are read.
            Ia2Action::DefaultAction => {
                // SAFETY: a UIA call on a valid element.
                let _ = unsafe { element.SetFocus() };
                do_default_action(&element).map_err(|e| e.to_string())
            }
        }
    }

    /// A state of Word's that a key just passed on changes (Track Changes, a paragraph's
    /// alignment, a font size), read once Word has acted on it: polled until it differs from its first read,
    /// or the settle time passes (Word had acted before the first read, or nothing changed).
    fn after_word_key<T: PartialEq>(read: impl Fn() -> Option<T>) -> Option<T> {
        let started = Instant::now();
        let first = read();
        while started.elapsed() < SETTLE {
            std::thread::sleep(POLL);
            let now = read();
            if now != first {
                return now;
            }
        }
        first
    }

    /// The field's state once the application has acted on the key.
    ///
    /// An idle application acts on a key before a read made as the key is passed on can
    /// reach it, so such a read is no "before": with it, an arrow key often waited the whole
    /// settle time for a change that had already happened (p90 280 ms, `okur-lab latency`). The
    /// field as the engine last knew it (`known`) is the "before". After a short head start
    /// the field is read; unchanged, it is polled until it changes or the settle time passes
    /// (a key at a boundary changes nothing). Without a known state, a first read is the
    /// "before", as it was.
    fn read_after_key(
        uia: &IUIAutomation,
        known: Option<TextContext>,
        in_page: bool,
    ) -> Option<TextContext> {
        // A key can have changed a form field the text shows nothing of.
        crate::text_context::forget_form_fields();
        if !in_page {
            return read_other_after_key(uia, known);
        }
        // SAFETY: plain Win32 call with no preconditions.
        let window = unsafe { GetForegroundWindow() };
        match xa11y_windows::ia2::ChromiumTextField::focused(window) {
            // A field whose paragraphs are objects of their own (Google Docs with braille
            // support): its IA2 text is those objects, and UIA reads it as one text, with
            // the caret where it is in it and the font there.
            Ok(Some(field)) if field.is_compound() => {
                read_other_after_key(uia, known).map(crate::text_context::with_paragraph_alignment)
            }
            Ok(Some(field)) => read_chromium_after_key(&field, known),
            Ok(None) => None,
            // Not a Chromium window.
            Err(_) => read_other_after_key(uia, known),
        }
    }

    /// A page's field (a web editor, a text area) after a key. The field is found once,
    /// and its caret, selection and length are polled every couple of milliseconds until
    /// they differ from what the engine knew; then its text is read once. Reading the whole
    /// field each time (a window's object, its focus, all its text: about 7 ms) every 15 ms
    /// kept a web editor's arrow keys at 33–43 ms (vscode.dev, `okur-lab latency`).
    fn read_chromium_after_key(
        field: &xa11y_windows::ia2::ChromiumTextField,
        known: Option<TextContext>,
    ) -> Option<TextContext> {
        use crate::text_context::{from_ia2, ia2_position};
        let started = Instant::now();
        let before = match &known {
            Some(known) => {
                std::thread::sleep(HEAD_START);
                Some(ia2_position(known))
            }
            None => field.position().ok(),
        };
        let mut now = field.position().ok();
        while now == before && started.elapsed() < SETTLE {
            std::thread::sleep(FIELD_POLL);
            now = field.position().ok();
        }
        // A selection still being extended is read once two reads agree.
        if now != before && now.is_some_and(|p| p.selection.is_some()) {
            loop {
                std::thread::sleep(POLL);
                let again = field.position().ok();
                if again == now || started.elapsed() >= SETTLE * 2 {
                    break;
                }
                now = again;
            }
        }
        field.state().ok().map(from_ia2)
    }

    /// Any other field after a key, read whole each time.
    fn read_other_after_key(
        uia: &IUIAutomation,
        known: Option<TextContext>,
    ) -> Option<TextContext> {
        let started = Instant::now();
        let (before, mut now) = match known {
            Some(known) => {
                std::thread::sleep(HEAD_START);
                (Some(known), read_focused_outside_page(uia))
            }
            None => {
                let before = read_focused_outside_page(uia);
                (before.clone(), before)
            }
        };
        // A key at an edge (Home at a line's start, Down on the last line) changes nothing,
        // and the field was read again and again for the whole settle time, 250 ms before
        // speech (performance review 2026-10-23, P3). Once the application has taken the key
        // and the field read after is still the same, twice over, the key changed nothing.
        let mut unchanged_after_taken = 0;
        while super::same_place(&now, &before) && started.elapsed() < SETTLE {
            if unchanged_after_taken < 2 && focus_has_taken_its_keys() {
                now = read_focused_outside_page(uia);
                if super::same_place(&now, &before) {
                    unchanged_after_taken += 1;
                    if unchanged_after_taken == 2 {
                        tracing::debug!(
                            ms = started.elapsed().as_millis(),
                            "the key changed nothing"
                        );
                        break;
                    }
                }
                continue;
            }
            std::thread::sleep(POLL);
            now = read_focused_outside_page(uia);
        }
        // A selection still being extended (Ctrl+Shift+End over many lines) is read once two
        // reads agree; a caret move is read at once.
        if !super::same_place(&now, &before) && now.as_ref().is_some_and(|c| c.selection.is_some())
        {
            loop {
                std::thread::sleep(POLL);
                let again = read_focused_outside_page(uia);
                if super::same_place(&again, &now) || started.elapsed() >= SETTLE * 2 {
                    break;
                }
                now = again;
            }
        }
        now
    }

    impl okur_core::actions::ApplicationActions for WindowsActions {
        fn focus(&self, node: &okur_core::AomNode) -> bool {
            self.act(node, Ia2Action::Focus)
        }

        fn activate(&self, node: &okur_core::AomNode) -> bool {
            self.act(node, Ia2Action::DefaultAction)
        }

        fn read_at_caret(&self, before: Option<&TextContext>, in_page: bool) -> bool {
            self.send(Request::Text(before.cloned().map(Box::new), in_page))
        }

        fn read_text(&self) -> bool {
            self.send(Request::TextNow)
        }

        fn move_to_cell(&self, before: Option<&TextContext>, row: usize, column: usize) -> bool {
            self.send(Request::Cell(before.cloned().map(Box::new), row, column))
        }

        fn read_document_headings(&self) -> bool {
            self.send(Request::DocumentHeadings)
        }

        fn read_document_links(&self) -> bool {
            self.send(Request::DocumentLinks)
        }

        fn read_line_spacing(&self) -> bool {
            self.send(Request::LineSpacing)
        }

        fn activate_document_link(&self, index: usize) -> bool {
            self.send(Request::ActivateDocumentLink(index))
        }

        fn read_document_fields(&self) -> bool {
            self.send(Request::DocumentFields)
        }

        fn move_to_document_field(&self, before: Option<&TextContext>, index: usize) -> bool {
            self.send(Request::Field(before.cloned().map(Box::new), index))
        }

        fn read_document_notes(&self, kind: okur_core::NoteKind) -> bool {
            self.send(Request::DocumentNotes(kind))
        }

        fn move_to_document_note(
            &self,
            before: Option<&TextContext>,
            kind: okur_core::NoteKind,
            index: usize,
        ) -> bool {
            self.send(Request::Note(before.cloned().map(Box::new), kind, index))
        }

        fn read_document_view(&self) -> bool {
            self.send(Request::DocumentView)
        }

        fn read_status_bar(&self) -> bool {
            self.send(Request::StatusBar)
        }

        fn read_word_key(&self, key: okur_core::WordKey) -> bool {
            self.send(Request::WordKey(key))
        }

        fn read_document_paragraphs(&self) -> bool {
            self.send(Request::DocumentParagraphs)
        }

        fn move_to_paragraph(&self, before: Option<&TextContext>, paragraph: usize) -> bool {
            self.send(Request::Paragraph(before.cloned().map(Box::new), paragraph))
        }

        fn move_caret(&self, context: &TextContext, offset: usize) -> bool {
            self.send(Request::Caret(context.text.clone(), offset))
        }

        fn move_mouse_to_text(&self, context: &TextContext, offset: usize) -> bool {
            self.send(Request::MouseToText(context.text.clone(), offset))
        }

        fn move_mouse_to_node(&self, node: &okur_core::AomNode) -> bool {
            if let Some(id) = unique_id(node) {
                return self.send(Request::MouseToObject(id));
            }
            runtime_id(node).is_some_and(|id| self.send(Request::MouseToUiaObject(id)))
        }

        fn read_window(&self, window: bool) -> bool {
            self.send(Request::Window(window))
        }

        fn list_windows(&self) -> bool {
            self.send(Request::ListWindows)
        }

        fn switch_to_window(&self, id: u64) -> bool {
            self.send(Request::SwitchTo(id))
        }

        fn list_tray_icons(&self) -> bool {
            self.send(Request::ListTray)
        }

        fn read_at_point(&self, x: i32, y: i32) -> bool {
            self.send(Request::Point(x, y))
        }

        fn read_screen(&self) -> bool {
            self.send(Request::Screen)
        }

        fn recognize(&self, area: okur_core::actions::RecognitionArea, language: &str) -> bool {
            self.send(Request::Recognize(area, language.to_owned()))
        }

        fn recognize_selected_file(&self, language: &str) -> bool {
            self.send(Request::RecognizeFile(language.to_owned()))
        }

        /// At once, not queued: the bulk lane is busy with the file.
        fn cancel_file_recognition(&self) {
            crate::file_text::cancel();
        }

        /// On a thread of its own, once Okur's dialog has had time to close: a click while it
        /// still had the focus would close the menu the click opens.
        fn click_mouse(&self, x: i32, y: i32, click: okur_core::actions::MouseClick) -> bool {
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(250));
                super::click_at(x, y, click);
            });
            true
        }

        fn read_page_elements(&self, ids: &[&str]) -> bool {
            self.send(Request::PageElements(
                ids.iter().map(|id| (*id).to_owned()).collect(),
            ))
        }

        fn refresh_page(&self) -> bool {
            crate::events::windows::refresh_page_for_focus()
        }

        fn page_changed(&self, now: bool) -> bool {
            crate::events::windows::refresh_changed_page(now)
        }

        fn press(&self, chord: &[okur_core::Key]) -> bool {
            let keys: Option<Vec<u16>> = chord.iter().map(virtual_key).collect();
            keys.is_some_and(|keys| self.send(Request::Keys(keys)))
        }

        fn copy_text(&self, text: &str) -> bool {
            self.send(Request::Copy(text.to_owned()))
        }

        fn clipboard_text(&self) -> Option<String> {
            crate::actions::clipboard_text()
        }

        fn drag_mouse(&self, from: (i32, i32), to: (i32, i32)) -> bool {
            self.send(Request::Drag(from, to))
        }

        fn move_mouse(&self, x: i32, y: i32) -> bool {
            use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;
            // SAFETY: plain Win32 call; it only moves the pointer.
            unsafe { SetCursorPos(x, y) }.is_ok()
        }
    }
}

/// Every UI Automation text attribute of the focused text at its caret, by `unit` around it
/// (0 character, 1 format, 2 word, 3 line, 4 paragraph): "id name: value" lines, for research
/// (`okur-lab text-attributes`). Empty without a text pattern.
#[cfg(windows)]
pub fn text_attributes_now(unit: i32) -> Vec<String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::{
        CUIAutomation8, IUIAutomation, IUIAutomationTextPattern, TextUnit, UIA_TextPatternId,
        UIA_TEXTATTRIBUTE_ID,
    };
    /// UI Automation's text attribute ids run from 40000 (AnimationStyle) to 40042
    /// (AfterParagraphSpacing) as of Windows 10.
    const FIRST: i32 = 40_000;
    const LAST: i32 = 40_042;
    let read = || -> Option<Vec<String>> {
        // SAFETY: COM initialization, in-process activation and UIA client calls on valid
        // objects.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let uia: IUIAutomation =
                CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).ok()?;
            let pattern: IUIAutomationTextPattern = uia
                .GetFocusedElement()
                .ok()?
                .GetCurrentPatternAs(UIA_TextPatternId)
                .ok()?;
            let range = pattern.GetSelection().ok()?.GetElement(0).ok()?;
            range.ExpandToEnclosingUnit(TextUnit(unit)).ok()?;
            let mixed = uia.ReservedMixedAttributeValue().ok();
            let unsupported = uia.ReservedNotSupportedValue().ok();
            Some(
                (FIRST..=LAST)
                    .filter_map(|id| {
                        let value: VARIANT =
                            range.GetAttributeValue(UIA_TEXTATTRIBUTE_ID(id)).ok()?;
                        let shown = match windows::core::IUnknown::try_from(&value) {
                            Ok(object) if Some(&object) == unsupported.as_ref() => return None,
                            Ok(object) if Some(&object) == mixed.as_ref() => "(mixed)".to_owned(),
                            _ => value.to_string(),
                        };
                        Some(format!("{id}: {shown}"))
                    })
                    .collect(),
            )
        }
    };
    read().unwrap_or_default()
}

/// The focused text field's text context right now, for harness checks (`okur-lab caret`).
#[cfg(windows)]
pub fn read_text_now() -> Option<okur_core::TextContext> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    // As the tracer decides: a page's field only with a document above the focus.
    if crate::text_context::focus_is_in_page(&uia) {
        crate::text_context::read_focused(&uia)
    } else {
        crate::text_context::read_focused_outside_page(&uia)
    }
}
/// The headings of Word's document, read right away on the calling thread: the focused one,
/// or the one in the top-level window titled `window_title`. For harness checks and the
/// recorder; the engine goes through `WindowsActions`. `None` without Word's document.
#[cfg(windows)]
pub fn document_headings_now(
    window_title: Option<&str>,
) -> Option<Vec<okur_core::DocumentHeading>> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    let window = match window_title {
        Some(title) => Some(crate::harness::window_titled(title)?),
        None => None,
    };
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    crate::text_context::word_document_headings(&uia, window)
}

/// The links of Word's document, read right away on the calling thread, as
/// [`document_headings_now`] reads its headings. For the recorder.
#[cfg(windows)]
pub fn document_links_now(window_title: Option<&str>) -> Option<Vec<okur_core::DocumentLink>> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    let window = match window_title {
        Some(title) => Some(crate::harness::window_titled(title)?),
        None => None,
    };
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    crate::text_context::word_document_links(&uia, window)
}

/// The paragraphs of Word's document, read right away on the calling thread, as
/// [`document_headings_now`] reads its headings. For the recorder.
#[cfg(windows)]
pub fn document_paragraphs_now(
    window_title: Option<&str>,
) -> Option<Vec<okur_core::DocumentParagraph>> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    let window = match window_title {
        Some(title) => Some(crate::harness::window_titled(title)?),
        None => None,
    };
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    crate::text_context::word_document_paragraphs(&uia, window)
}

/// The view Word's window shows its document in, read right away on the calling thread: the
/// window in front, or the top-level window titled `window_title`. For the recorder.
#[cfg(windows)]
pub fn document_view_now(window_title: Option<&str>) -> Option<String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    let window = match window_title {
        Some(title) => Some(crate::harness::window_titled(title)?),
        None => None,
    };
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    crate::text_context::word_view(&uia, window)
}

/// The items of the status bar of Word's window, read right away on the calling thread: the
/// window in front, or the top-level window titled `window_title`. For the recorder.
#[cfg(windows)]
pub fn status_bar_now(window_title: Option<&str>) -> Option<Vec<String>> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    let window = match window_title {
        Some(title) => Some(crate::harness::window_titled(title)?),
        None => None,
    };
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    crate::text_context::word_status_bar(&uia, window)
}

/// Performs an action on a node's live object right away, on the calling thread, reporting
/// why it failed. For harness checks; the engine goes through `WindowsActions`.
#[cfg(windows)]
pub fn perform_now(node: &okur_core::AomNode, activate: bool) -> Result<(), String> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use xa11y_windows::ia2::{perform_chromium_action, Ia2Action};
    let id = unique_id(node).ok_or("node has no IA2 uniqueID")?;
    let action = if activate {
        Ia2Action::DefaultAction
    } else {
        Ia2Action::Focus
    };
    // SAFETY: COM initialization; repeated initialization is tolerated.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    // SAFETY: plain Win32 call with no preconditions.
    let window = unsafe { GetForegroundWindow() };
    perform_chromium_action(window, id, action).map_err(|e| format!("IA2 object {id}: {e}"))
}

/// Captures the foreground Chromium window's page through IA2 and maps it with
/// [`crate::ia2_snapshot::ia2_to_aom`]: the same pipeline the parity harness verifies.
///
/// When IA2 yields no page (only Chrome's "Chrome Legacy Window" placeholder), the UIA
/// capture and [`crate::uia_snapshot::to_aom`] are used instead. Page actions need IA2
/// uniqueIDs, so a UIA page can be read but not acted on.
#[cfg(windows)]
pub fn capture_foreground_page() -> Result<okur_core::AomNode, crate::PlatformError> {
    let ia2 = crate::ia2_snapshot::capture_foreground_document(None, None);
    match ia2 {
        Ok(snapshot) if snapshot.root.node_count() > 1 => {
            Ok(crate::ia2_snapshot::ia2_to_aom(&snapshot.root))
        }
        other => {
            tracing::warn!(
                "IA2 page capture unusable ({} node(s)); falling back to UIA",
                other.as_ref().map(|s| s.root.node_count()).unwrap_or(0)
            );
            let snapshot = crate::uia_snapshot::capture_foreground_document(None, None)?;
            Ok(crate::uia_snapshot::to_aom(&snapshot.root))
        }
    }
}

/// How long [`clipboard_text`] waits for the clipboard: well within the time the keyboard
/// hook, which asks for it, may take over a key.
#[cfg(windows)]
const CLIPBOARD_WAIT: std::time::Duration = std::time::Duration::from_millis(150);

/// The clipboard's Unicode text (CF_UNICODETEXT), if it holds any and answers within
/// [`CLIPBOARD_WAIT`]. Read on a thread of its own: text put on the clipboard for later
/// rendering is asked of its owner (`WM_RENDERFORMAT`), and a hung owner would hold the
/// keyboard hook that asks (performance review 2026-10-23, P2); such a read finishes, or not,
/// on its own thread.
#[cfg(windows)]
pub fn clipboard_text() -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("clipboard read".to_owned())
        .spawn(move || {
            let _ = tx.send(read_clipboard_text());
        })
        .ok()?;
    match rx.recv_timeout(CLIPBOARD_WAIT) {
        Ok(text) => text,
        Err(_) => {
            tracing::warn!("the clipboard did not answer in time");
            None
        }
    }
}

/// The clipboard's Unicode text, read on the calling thread.
#[cfg(windows)]
fn read_clipboard_text() -> Option<String> {
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    const CF_UNICODETEXT: u32 = 13;
    // SAFETY: the clipboard is opened and closed here; the data is only read while locked,
    // within the block's size, up to its terminating zero.
    unsafe {
        OpenClipboard(None).ok()?;
        let text = GetClipboardData(CF_UNICODETEXT).ok().and_then(|data| {
            let block = HGLOBAL(data.0);
            let units = GlobalSize(block) / 2;
            let ptr = GlobalLock(block).cast::<u16>();
            if ptr.is_null() {
                return None;
            }
            let all = std::slice::from_raw_parts(ptr, units);
            let len = all.iter().position(|&unit| unit == 0).unwrap_or(units);
            let text = String::from_utf16_lossy(all.get(..len).unwrap_or_default());
            let _ = GlobalUnlock(block);
            Some(text)
        });
        let _ = CloseClipboard();
        text
    }
}

/// The notification area's icons now, for harness checks (`okur-lab tray`).
#[cfg(windows)]
pub fn tray_icons_now() -> Vec<okur_core::event::TrayIcon> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    // SAFETY: COM initialization for this thread.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    crate::uia_snapshot::capture::automation()
        .map(|uia| crate::uia_snapshot::capture::tray_icons(&uia))
        .unwrap_or_default()
}

/// The taskbar's control tree, one element a line (indented by depth: control type, class,
/// name, rectangle), for finding its notification area on a new Windows (`okur-lab tray --dump`).
#[cfg(windows)]
pub fn taskbar_dump_now() -> Vec<String> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    // SAFETY: COM initialization for this thread.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let (Ok(uia), Some((taskbar, _))) = (
        crate::uia_snapshot::capture::automation(),
        crate::uia_snapshot::capture::taskbar_windows(),
    ) else {
        return Vec::new();
    };
    crate::uia_snapshot::capture::taskbar_elements(&uia, taskbar)
        .into_iter()
        .map(|e| {
            let rect = e
                .rect
                .map(|r| format!("[{}, {}, {}, {}]", r.left, r.top, r.right, r.bottom))
                .unwrap_or_default();
            format!(
                "{}{} {} {:?} {rect}{}",
                "  ".repeat(e.depth),
                e.control_type,
                e.class_name.unwrap_or_default(),
                e.name.unwrap_or_default(),
                if e.offscreen { " offscreen" } else { "" }
            )
        })
        .collect()
}

/// Clicks the mouse at `x`, `y` (real pixels, the process being DPI aware): the pointer moves
/// there, then the buttons are pressed and released in one `SendInput` call, twice for a
/// double click.
#[cfg(windows)]
fn click_at(x: i32, y: i32, click: okur_core::actions::MouseClick) {
    use okur_core::actions::MouseClick;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, MOUSE_EVENT_FLAGS,
    };
    use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;
    let button = |flags: MOUSE_EVENT_FLAGS| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inputs = match click {
        MouseClick::RightSingle => vec![button(MOUSEEVENTF_RIGHTDOWN), button(MOUSEEVENTF_RIGHTUP)],
        MouseClick::LeftSingle => vec![button(MOUSEEVENTF_LEFTDOWN), button(MOUSEEVENTF_LEFTUP)],
        MouseClick::LeftDouble => vec![
            button(MOUSEEVENTF_LEFTDOWN),
            button(MOUSEEVENTF_LEFTUP),
            button(MOUSEEVENTF_LEFTDOWN),
            button(MOUSEEVENTF_LEFTUP),
        ],
        MouseClick::LeftDown => vec![button(MOUSEEVENTF_LEFTDOWN)],
        MouseClick::LeftUp => vec![button(MOUSEEVENTF_LEFTUP)],
        MouseClick::RightDown => vec![button(MOUSEEVENTF_RIGHTDOWN)],
        MouseClick::RightUp => vec![button(MOUSEEVENTF_RIGHTUP)],
    };
    let size = i32::try_from(std::mem::size_of::<INPUT>()).unwrap_or(0);
    // SAFETY: plain Win32 calls: the pointer moves, then well-formed mouse button inputs.
    unsafe {
        if SetCursorPos(x, y).is_err() {
            tracing::warn!(x, y, "the mouse could not be moved to click");
            return;
        }
        SendInput(&inputs, size);
    }
}

/// A drag with the left button: pressed at `from`, moved to `to` in steps, as a hand would,
/// let go there.
#[cfg(windows)]
fn drag(from: (i32, i32), to: (i32, i32)) {
    use okur_core::actions::MouseClick;
    use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;
    const STEPS: i32 = 8;
    click_at(from.0, from.1, MouseClick::LeftDown);
    for step in 1..=STEPS {
        let x = from.0 + (to.0 - from.0) * step / STEPS;
        let y = from.1 + (to.1 - from.1) * step / STEPS;
        // SAFETY: a plain Win32 call moving the pointer.
        let _ = unsafe { SetCursorPos(x, y) };
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    click_at(to.0, to.1, MouseClick::LeftUp);
}

/// Puts `text` on the clipboard as Unicode text (CF_UNICODETEXT).
#[cfg(windows)]
pub fn set_clipboard_text(text: &str) -> Result<(), String> {
    use windows::Win32::Foundation::{GlobalFree, HANDLE};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    const CF_UNICODETEXT: u32 = 13;
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: the clipboard is opened and closed here; the global block is sized for the
    // text and, once set, owned by the clipboard; not set, it is freed here.
    unsafe {
        OpenClipboard(None).map_err(|e| format!("OpenClipboard: {e}"))?;
        let result = (|| {
            EmptyClipboard().map_err(|e| format!("EmptyClipboard: {e}"))?;
            let block = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2)
                .map_err(|e| format!("GlobalAlloc: {e}"))?;
            let target = GlobalLock(block).cast::<u16>();
            if target.is_null() {
                let _ = GlobalFree(Some(block));
                return Err("GlobalLock failed".to_owned());
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), target, wide.len());
            let _ = GlobalUnlock(block);
            if let Err(e) = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(block.0))) {
                let _ = GlobalFree(Some(block));
                return Err(format!("SetClipboardData: {e}"));
            }
            Ok(())
        })();
        let _ = CloseClipboard();
        result
    }
}

/// Captures the foreground page through UIA alone, with one cached round trip: a page ready
/// to read in a fraction of the time the IA2 capture takes (GitHub's repository page, 1,182
/// objects: 160 ms against 3 s), without IA2's rendered line breaks and formatting runs.
/// The live screen reader reads this first and replaces it with
/// [`capture_foreground_page`]'s when that is done.
#[cfg(windows)]
pub fn capture_foreground_page_quick() -> Result<okur_core::AomNode, crate::PlatformError> {
    let snapshot = crate::uia_snapshot::capture_foreground_document(None, None)?;
    Ok(crate::uia_snapshot::to_aom(&snapshot.root))
}

#[cfg(test)]
mod tests {
    use super::same_place;

    fn read(caret: usize, alignment: Option<&str>) -> Option<okur_core::TextContext> {
        Some(okur_core::TextContext {
            text: "Second line".to_owned(),
            caret,
            selection: None,
            font: Some(okur_core::FontInfo {
                family: "Arial".to_owned(),
                alignment: alignment.map(str::to_owned),
                ..Default::default()
            }),
            word: None,
            misspelled: Vec::new(),
            structure: None,
        })
    }

    /// A poll without the paragraph's alignment is the same place as the read with it: a
    /// key has not acted until the text, caret or selection change.
    #[test]
    fn a_reads_place_leaves_the_font_out() {
        assert!(same_place(&read(3, Some("left")), &read(3, None)));
        assert!(!same_place(&read(3, Some("left")), &read(4, Some("left"))));
        assert!(!same_place(&read(3, None), &None));
    }
}
