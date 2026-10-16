//! [`okur_core::actions::ApplicationActions`] for Chromium pages, through MSAA/IA2.
//!
//! Buffer nodes mapped from IA2 carry the object's IA2 `uniqueID`
//! ([`UNIQUE_ID_ATTR`]). An action resolves it with `accChild(uniqueID)` on the foreground
//! window's document (Chromium's IDs are negative, which makes them valid child ids) and
//! calls `accSelect(SELFLAG_TAKEFOCUS)` or `accDoDefaultAction`. The calls cross into the browser process, so they run on a worker
//! thread: the engine calls actions from the keyboard hook, which must return at once.

/// AOM attribute holding a node's IA2 `uniqueID`.
pub const UNIQUE_ID_ATTR: &str = "ia2_unique_id";

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

#[cfg(windows)]
pub use windows_actions::WindowsActions;

#[cfg(windows)]
mod windows_actions {
    use super::{runtime_id, unique_id};
    use crate::text_context::{read_focused, read_focused_outside_page};
    use okur_core::{AccessibilityEvent, TextContext};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc::UnboundedSender;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use xa11y_windows::ia2::{perform_chromium_action, Ia2Action};

    /// How long to wait for the application to act on a key.
    const SETTLE: Duration = Duration::from_millis(250);
    const POLL: Duration = Duration::from_millis(15);
    /// How often a page's field is polled while a key takes effect: a poll is a few calls.
    const FIELD_POLL: Duration = Duration::from_millis(2);
    /// The application's head start on the first read: an idle application has acted on a
    /// key within a millisecond or two of the hook passing it on.
    const HEAD_START: Duration = Duration::from_millis(3);

    enum Request {
        Object(i32, Ia2Action),
        /// The same, for a page object read through UIA, found by its RuntimeId.
        UiaObject(Vec<i32>, Ia2Action),
        /// The field's text once the application has acted on a key, compared with the
        /// field as the engine last knew it.
        Text(Option<TextContext>, bool),
        /// The focused text as it is now, answered even when there is none.
        TextNow,
        /// A chord of virtual keys, sent marked as the screen reader's own.
        Keys(Vec<u16>),
        /// The Virtual Viewer's lines for the window in front (`true`) or the focused
        /// control, answered even when nothing could be read.
        Window(bool),
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
        requests: Mutex<Sender<Request>>,
    }

    impl WindowsActions {
        /// `events` receives the text contexts read after edit keys.
        pub fn start(events: Option<UnboundedSender<AccessibilityEvent>>) -> Self {
            let (tx, rx) = channel::<Request>();
            std::thread::spawn(move || {
                // SAFETY: COM initialization for this worker thread.
                let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
                // SAFETY: in-process COM activation.
                let uia: Option<IUIAutomation> =
                    unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) }.ok();
                while let Ok(first) = rx.recv() {
                    // Keys arrive faster than a read settles (key repeat): of the text reads
                    // queued meanwhile only the newest matters, since the engine speaks for
                    // the latest key. Page actions still run, in order.
                    let mut batch = vec![first];
                    batch.extend(rx.try_iter());
                    let last_text = batch.iter().rposition(|r| matches!(r, Request::Text(..)));
                    let batch = batch.into_iter().enumerate().filter_map(|(i, r)| {
                        (!matches!(r, Request::Text(..)) || Some(i) == last_text).then_some(r)
                    });
                    for request in batch {
                        match request {
                            Request::Keys(keys) => {
                                use crate::hook::windows::INJECTED_MARK;
                                for &vk in &keys {
                                    crate::harness::send_key(vk, false, INJECTED_MARK);
                                }
                                for &vk in keys.iter().rev() {
                                    crate::harness::send_key(vk, true, INJECTED_MARK);
                                }
                            }
                            Request::Object(id, action) => {
                                // SAFETY: plain Win32 call with no preconditions.
                                let window = unsafe { GetForegroundWindow() };
                                if let Err(e) = perform_chromium_action(window, id, action) {
                                    tracing::warn!("{action:?} on IA2 object {id} failed: {e}");
                                }
                            }
                            Request::UiaObject(id, action) => {
                                let Some(uia) = uia.as_ref() else {
                                    continue;
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
                                    continue;
                                };
                                let started = Instant::now();
                                let context = read_after_key(uia, before, in_page);
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
                            Request::TextNow => {
                                let Some(events) = events.as_ref() else {
                                    tracing::warn!("text read impossible: no event channel");
                                    continue;
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
                                });
                                let _ = events.send(AccessibilityEvent::TextContext(context));
                            }
                            Request::Window(window) => {
                                let Some(events) = events.as_ref() else {
                                    tracing::warn!("window read impossible: no event channel");
                                    continue;
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
                        }
                    }
                }
            });
            Self {
                requests: Mutex::new(tx),
            }
        }

        fn send(&self, request: Request) -> bool {
            self.requests
                .lock()
                .is_ok_and(|tx| tx.send(request).is_ok())
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

    /// The field's state once the application has acted on the key.
    ///
    /// An idle application acts on a key before a read made as the key is passed on can
    /// reach it, so such a read is no "before": with it, an arrow key often waited the whole
    /// settle time for a change that had already happened (p90 280 ms, `okur latency`). The
    /// field as the engine last knew it (`known`) is the "before". After a short head start
    /// the field is read; unchanged, it is polled until it changes or the settle time passes
    /// (a key at a boundary changes nothing). Without a known state, a first read is the
    /// "before", as it was.
    fn read_after_key(
        uia: &IUIAutomation,
        known: Option<TextContext>,
        in_page: bool,
    ) -> Option<TextContext> {
        if !in_page {
            return read_other_after_key(uia, known);
        }
        // SAFETY: plain Win32 call with no preconditions.
        let window = unsafe { GetForegroundWindow() };
        match xa11y_windows::ia2::ChromiumTextField::focused(window) {
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
    /// kept a web editor's arrow keys at 33–43 ms (vscode.dev, `okur latency`).
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
        while now == before && started.elapsed() < SETTLE {
            std::thread::sleep(POLL);
            now = read_focused_outside_page(uia);
        }
        // A selection still being extended (Ctrl+Shift+End over many lines) is read once two
        // reads agree; a caret move is read at once.
        if now != before && now.as_ref().is_some_and(|c| c.selection.is_some()) {
            loop {
                std::thread::sleep(POLL);
                let again = read_focused_outside_page(uia);
                if again == now || started.elapsed() >= SETTLE * 2 {
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
            self.send(Request::Text(before.cloned(), in_page))
        }

        fn read_text(&self) -> bool {
            self.send(Request::TextNow)
        }

        fn read_window(&self, window: bool) -> bool {
            self.send(Request::Window(window))
        }

        fn press(&self, chord: &[okur_core::Key]) -> bool {
            let keys: Option<Vec<u16>> = chord.iter().map(virtual_key).collect();
            keys.is_some_and(|keys| self.send(Request::Keys(keys)))
        }

        fn copy_text(&self, text: &str) -> bool {
            match super::set_clipboard_text(text) {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!("copy to the clipboard failed: {e}");
                    false
                }
            }
        }
    }
}

/// The focused text field's text context right now, for harness checks (`okur caret`).
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
    crate::text_context::read_focused(&uia)
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

/// Puts `text` on the clipboard as Unicode text (CF_UNICODETEXT).
#[cfg(windows)]
pub fn set_clipboard_text(text: &str) -> Result<(), String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    const CF_UNICODETEXT: u32 = 13;
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: the clipboard is opened and closed here; the global block is sized for the
    // text and, once set, owned by the clipboard.
    unsafe {
        OpenClipboard(None).map_err(|e| format!("OpenClipboard: {e}"))?;
        let result = (|| {
            EmptyClipboard().map_err(|e| format!("EmptyClipboard: {e}"))?;
            let block = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2)
                .map_err(|e| format!("GlobalAlloc: {e}"))?;
            let target = GlobalLock(block).cast::<u16>();
            if target.is_null() {
                return Err("GlobalLock failed".to_owned());
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), target, wide.len());
            let _ = GlobalUnlock(block);
            SetClipboardData(CF_UNICODETEXT, Some(HANDLE(block.0)))
                .map_err(|e| format!("SetClipboardData: {e}"))?;
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
