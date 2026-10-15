//! [`sr_core::actions::ApplicationActions`] for Chromium pages, through MSAA/IA2.
//!
//! Buffer nodes mapped from IA2 carry the object's IA2 `uniqueID`
//! ([`UNIQUE_ID_ATTR`]). An action resolves it with `accChild(uniqueID)` on the foreground
//! window's document (Chromium's IDs are negative, which makes them valid child ids) and
//! calls `accSelect(SELFLAG_TAKEFOCUS)` or `accDoDefaultAction`. The calls cross into the browser process, so they run on a worker
//! thread: the engine calls actions from the keyboard hook, which must return at once.

/// AOM attribute holding a node's IA2 `uniqueID`.
pub const UNIQUE_ID_ATTR: &str = "ia2_unique_id";

/// The IA2 `uniqueID` recorded on a node, if any.
pub fn unique_id(node: &sr_core::AomNode) -> Option<i32> {
    node.attributes.get(UNIQUE_ID_ATTR)?.parse().ok()
}

#[cfg(windows)]
pub use windows_actions::WindowsActions;

#[cfg(windows)]
mod windows_actions {
    use super::unique_id;
    use crate::text_context::read_focused;
    use sr_core::{AccessibilityEvent, CaretUnit};
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

    enum Request {
        Object(i32, Ia2Action),
        /// The field's text once the application has acted on a key.
        Text,
        /// The focused text as it is now, answered even when there is none.
        TextNow,
        /// A chord of virtual keys, sent marked as the screen reader's own.
        Keys(Vec<u16>),
    }

    /// The virtual key of a key the screen reader sends for a reading command.
    fn virtual_key(key: &sr_core::Key) -> Option<u16> {
        use sr_core::Key;
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
                    let last_text = batch.iter().rposition(|r| matches!(r, Request::Text));
                    let batch = batch.into_iter().enumerate().filter_map(|(i, r)| {
                        (!matches!(r, Request::Text) || Some(i) == last_text).then_some(r)
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
                            Request::Text => {
                                let (Some(uia), Some(events)) = (uia.as_ref(), events.as_ref())
                                else {
                                    tracing::warn!(
                                        "text read impossible: no UIA client or channel"
                                    );
                                    continue;
                                };
                                let context = read_after_key(uia);
                                tracing::debug!(?context, "text context read");
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
                                let context = context.unwrap_or(sr_core::TextContext {
                                    text: String::new(),
                                    caret: 0,
                                    selection: None,
                                    font: None,
                                    word: None,
                                });
                                let _ = events.send(AccessibilityEvent::TextContext(context));
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
    }

    /// The field's state once the application has acted on the key: read as the key is
    /// passed on, then polled until it changes or the settle time passes (a key at a
    /// boundary changes nothing).
    fn read_after_key(uia: &IUIAutomation) -> Option<sr_core::TextContext> {
        let before = read_focused(uia);
        let started = Instant::now();
        let mut now = before.clone();
        while started.elapsed() < SETTLE {
            std::thread::sleep(POLL);
            now = read_focused(uia);
            if now != before {
                // Changed: wait until two reads agree, so a selection still being extended
                // (Ctrl+Shift+End over many lines) is read whole.
                loop {
                    std::thread::sleep(POLL);
                    let again = read_focused(uia);
                    if again == now || started.elapsed() >= SETTLE * 2 {
                        break;
                    }
                    now = again;
                }
                break;
            }
        }
        now
    }

    impl sr_core::actions::ApplicationActions for WindowsActions {
        fn focus(&self, node: &sr_core::AomNode) -> bool {
            unique_id(node).is_some_and(|id| self.send(Request::Object(id, Ia2Action::Focus)))
        }

        fn activate(&self, node: &sr_core::AomNode) -> bool {
            unique_id(node)
                .is_some_and(|id| self.send(Request::Object(id, Ia2Action::DefaultAction)))
        }

        fn read_at_caret(&self, _unit: CaretUnit) -> bool {
            self.send(Request::Text)
        }

        fn read_text(&self) -> bool {
            self.send(Request::TextNow)
        }

        fn press(&self, chord: &[sr_core::Key]) -> bool {
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

/// The focused text field's text context right now, for harness checks (`sr-cli caret`).
#[cfg(windows)]
pub fn read_text_now() -> Option<sr_core::TextContext> {
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
pub fn perform_now(node: &sr_core::AomNode, activate: bool) -> Result<(), String> {
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
pub fn capture_foreground_page() -> Result<sr_core::AomNode, crate::PlatformError> {
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
