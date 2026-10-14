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
pub use windows_actions::{read_at_caret_now, WindowsActions};

#[cfg(windows)]
mod windows_actions {
    use super::unique_id;
    use sr_core::{AccessibilityEvent, CaretUnit};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc::UnboundedSender;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation8, IUIAutomation, IUIAutomationTextPattern, IUIAutomationTextRange,
        TextPatternRangeEndpoint_Start, TextUnit, TextUnit_Character, TextUnit_Line, TextUnit_Word,
        UIA_TextPatternId,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use xa11y_windows::ia2::{perform_chromium_action, Ia2Action};

    /// How long to wait for the application to move its caret after a key.
    const CARET_SETTLE: Duration = Duration::from_millis(250);
    const CARET_POLL: Duration = Duration::from_millis(15);

    enum Request {
        Object(i32, Ia2Action),
        Caret(CaretUnit),
    }

    /// Acts on the foreground application from a COM worker thread: page objects by IA2
    /// uniqueID, and text at the focused edit's caret through UIA TextPattern.
    pub struct WindowsActions {
        requests: Mutex<Sender<Request>>,
    }

    impl WindowsActions {
        /// `events` receives the caret text read after navigation keys.
        pub fn start(events: Option<UnboundedSender<AccessibilityEvent>>) -> Self {
            let (tx, rx) = channel::<Request>();
            std::thread::spawn(move || {
                // SAFETY: COM initialization for this worker thread.
                let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
                // SAFETY: in-process COM activation.
                let uia: Option<IUIAutomation> =
                    unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) }.ok();
                for request in rx {
                    match request {
                        Request::Object(id, action) => {
                            // SAFETY: plain Win32 call with no preconditions.
                            let window = unsafe { GetForegroundWindow() };
                            if let Err(e) = perform_chromium_action(window, id, action) {
                                tracing::warn!("{action:?} on IA2 object {id} failed: {e}");
                            }
                        }
                        Request::Caret(unit) => {
                            let (Some(uia), Some(events)) = (uia.as_ref(), events.as_ref()) else {
                                continue;
                            };
                            if let Some(text) = read_after_caret_move(uia, unit) {
                                let _ = events.send(AccessibilityEvent::CaretText { unit, text });
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

    /// The caret (the selection's start) of the focused element's text.
    fn caret(uia: &IUIAutomation) -> Option<IUIAutomationTextRange> {
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let focused = uia.GetFocusedElement().ok()?;
            let text: IUIAutomationTextPattern =
                focused.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
            text.GetSelection().ok()?.GetElement(0).ok()
        }
    }

    fn same_caret(a: &IUIAutomationTextRange, b: &IUIAutomationTextRange) -> bool {
        // SAFETY: comparing two ranges of the same text provider.
        unsafe {
            a.CompareEndpoints(
                TextPatternRangeEndpoint_Start,
                b,
                TextPatternRangeEndpoint_Start,
            )
        }
        .is_ok_and(|order| order == 0)
    }

    /// Waits for the caret to move (or the settle time to pass, at a boundary the caret
    /// stays put), then reads `unit` at the caret.
    fn read_after_caret_move(uia: &IUIAutomation, unit: CaretUnit) -> Option<String> {
        let before = caret(uia);
        let started = Instant::now();
        let mut now = caret(uia)?;
        while started.elapsed() < CARET_SETTLE
            && before.as_ref().is_some_and(|b| same_caret(b, &now))
        {
            std::thread::sleep(CARET_POLL);
            now = caret(uia)?;
        }
        let text_unit: TextUnit = match unit {
            CaretUnit::Character => TextUnit_Character,
            CaretUnit::Word => TextUnit_Word,
            CaretUnit::Line => TextUnit_Line,
        };
        // SAFETY: UIA text range calls on a valid range.
        unsafe {
            let range = now.Clone().ok()?;
            range.ExpandToEnclosingUnit(text_unit).ok()?;
            range.GetText(-1).ok().map(|s| s.to_string())
        }
    }

    /// Reads `unit` at the focused edit's caret right away, for harness checks.
    pub fn read_at_caret_now(unit: CaretUnit) -> Option<String> {
        // SAFETY: COM initialization; repeated initialization is tolerated.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        // SAFETY: in-process COM activation.
        let uia: IUIAutomation =
            unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) }.ok()?;
        let text_unit: TextUnit = match unit {
            CaretUnit::Character => TextUnit_Character,
            CaretUnit::Word => TextUnit_Word,
            CaretUnit::Line => TextUnit_Line,
        };
        let now = caret(&uia)?;
        // SAFETY: UIA text range calls on a valid range.
        unsafe {
            now.ExpandToEnclosingUnit(text_unit).ok()?;
            now.GetText(-1).ok().map(|s| s.to_string())
        }
    }

    impl sr_core::actions::ApplicationActions for WindowsActions {
        fn focus(&self, node: &sr_core::AomNode) -> bool {
            unique_id(node).is_some_and(|id| self.send(Request::Object(id, Ia2Action::Focus)))
        }

        fn activate(&self, node: &sr_core::AomNode) -> bool {
            unique_id(node)
                .is_some_and(|id| self.send(Request::Object(id, Ia2Action::DefaultAction)))
        }

        fn read_at_caret(&self, unit: CaretUnit) -> bool {
            self.send(Request::Caret(unit))
        }
    }
}
/// Performs an action on a node's live object right away, on the calling thread, reporting
/// why it failed. For harness checks; the engine goes through [`WindowsActions`].
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
#[cfg(windows)]
pub fn capture_foreground_page() -> Result<sr_core::AomNode, crate::PlatformError> {
    let snapshot = crate::ia2_snapshot::capture_foreground_document(None, None)?;
    Ok(crate::ia2_snapshot::ia2_to_aom(&snapshot.root))
}
