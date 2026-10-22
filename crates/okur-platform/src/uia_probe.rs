//! A harness probe: which UI Automation registrations hear an application's notifications. The
//! tracer, registered on the desktop root, missed every notification a WinForms button raised,
//! though JAWS spoke each one (820, rounds 247 to 249); the probe registers several ways at
//! once and says which heard what. Research, never evidence.

/// What each registration heard while a window titled `title` was watched for `millis`, one
/// line each ("window subtree: Upload finished: report.pdf (pid 1234)"), after the lines saying
/// how each registration went. The window is waited for up to ten seconds.
#[cfg(windows)]
pub fn notification_registrations(title: &str, millis: u64) -> Result<Vec<String>, String> {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use windows::core::{implement, Interface, Ref, BSTR, HSTRING};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation8, IUIAutomation, IUIAutomation5, IUIAutomationCacheRequest,
        IUIAutomationElement, IUIAutomationEventHandler, IUIAutomationEventHandler_Impl,
        IUIAutomationNotificationEventHandler, IUIAutomationNotificationEventHandler_Impl,
        NotificationKind, NotificationProcessing, TreeScope, TreeScope_Ancestors,
        TreeScope_Element, TreeScope_Subtree, UIA_NotificationEventId, UIA_EVENT_ID,
    };
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    type Log = Arc<Mutex<Vec<String>>>;

    fn pid_of(sender: &Ref<IUIAutomationElement>) -> i32 {
        // SAFETY: a property read on the element UIA passed in.
        sender
            .as_ref()
            .and_then(|e| unsafe { e.CurrentProcessId() }.ok())
            .unwrap_or(0)
    }

    #[implement(IUIAutomationNotificationEventHandler)]
    struct Heard {
        tag: &'static str,
        log: Log,
    }

    impl IUIAutomationNotificationEventHandler_Impl for Heard_Impl {
        fn HandleNotificationEvent(
            &self,
            sender: Ref<IUIAutomationElement>,
            _kind: NotificationKind,
            _processing: NotificationProcessing,
            display: &BSTR,
            _activity: &BSTR,
        ) -> windows::core::Result<()> {
            let pid = pid_of(&sender);
            if let Ok(mut log) = self.log.lock() {
                log.push(format!("{}: {display} (pid {pid})", self.tag));
            }
            Ok(())
        }
    }

    #[implement(IUIAutomationEventHandler)]
    struct HeardEvent {
        tag: &'static str,
        log: Log,
    }

    impl IUIAutomationEventHandler_Impl for HeardEvent_Impl {
        fn HandleAutomationEvent(
            &self,
            sender: Ref<IUIAutomationElement>,
            event: UIA_EVENT_ID,
        ) -> windows::core::Result<()> {
            let pid = pid_of(&sender);
            if let Ok(mut log) = self.log.lock() {
                log.push(format!("{}: event {} (pid {pid})", self.tag, event.0));
            }
            Ok(())
        }
    }

    let failed = |what: &str, e: windows::core::Error| format!("{what}: {e}");
    // SAFETY: COM on this thread, a multithreaded apartment: UIA calls the handlers on its own
    // threads, so nothing here pumps messages.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    // SAFETY: plain UIA factory and tree calls.
    let uia: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) }
            .map_err(|e| failed("CUIAutomation8", e))?;
    let uia5: IUIAutomation5 = uia.cast().map_err(|e| failed("IUIAutomation5", e))?;
    let root = unsafe { uia.GetRootElement() }.map_err(|e| failed("root", e))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let hwnd = loop {
        // SAFETY: a window lookup by its title.
        match unsafe { FindWindowW(None, &HSTRING::from(title)) } {
            Ok(hwnd) if !hwnd.is_invalid() => break hwnd,
            _ if Instant::now() > deadline => return Err(format!("no window titled {title:?}")),
            _ => std::thread::sleep(Duration::from_millis(200)),
        }
    };
    let window = unsafe { uia.ElementFromHandle(hwnd) }.map_err(|e| failed("window", e))?;
    // The focus once the window shows: the control a notification is raised on, in a window
    // that raises them from its focused control.
    std::thread::sleep(Duration::from_millis(1500));
    let focused = unsafe { uia.GetFocusedElement() }.map_err(|e| failed("focus", e))?;
    // SAFETY: plain UIA factory calls.
    let raw_view: Option<IUIAutomationCacheRequest> = unsafe {
        uia.CreateCacheRequest().ok().and_then(|request| {
            request
                .SetTreeFilter(&uia.RawViewCondition().ok()?)
                .ok()
                .map(|()| request)
        })
    };
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let note = |line: String| {
        if let Ok(mut log) = log.lock() {
            log.push(line);
        }
    };
    let registrations: [(
        &'static str,
        &IUIAutomationElement,
        TreeScope,
        Option<&IUIAutomationCacheRequest>,
    ); 6] = [
        ("root subtree", &root, TreeScope_Subtree, None),
        (
            "root subtree, raw view",
            &root,
            TreeScope_Subtree,
            raw_view.as_ref(),
        ),
        ("window subtree", &window, TreeScope_Subtree, None),
        (
            "window subtree, raw view",
            &window,
            TreeScope_Subtree,
            raw_view.as_ref(),
        ),
        ("focused element", &focused, TreeScope_Element, None),
        (
            "focused element and its ancestors",
            &focused,
            TreeScope(TreeScope_Element.0 | TreeScope_Ancestors.0),
            None,
        ),
    ];
    for (tag, element, scope, cache) in registrations {
        let handler: IUIAutomationNotificationEventHandler = Heard {
            tag,
            log: log.clone(),
        }
        .into();
        // SAFETY: a UIA handler, removed below with all the others.
        let added = unsafe { uia5.AddNotificationEventHandler(element, scope, cache, &handler) };
        note(format!("registered {tag}: {added:?}"));
    }
    let handler: IUIAutomationEventHandler = HeardEvent {
        tag: "root subtree, as an automation event",
        log: log.clone(),
    }
    .into();
    // SAFETY: as above.
    let added = unsafe {
        uia.AddAutomationEventHandler(
            UIA_NotificationEventId,
            &root,
            TreeScope_Subtree,
            None,
            &handler,
        )
    };
    note(format!(
        "registered root subtree, as an automation event: {added:?}"
    ));
    // Every WinEvent the window's process fires meanwhile, UI Automation's own range
    // (0x4E00-0x4EFF) among them: a WinEvent hook is how JAWS hears much of Windows.
    let mut pid = 0u32;
    // SAFETY: a plain query with a valid out-pointer.
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid))
    };
    let win_events = win_events_from(pid, Duration::from_millis(millis));
    // SAFETY: removes every handler this object registered.
    let _ = unsafe { uia.RemoveAllEventHandlers() };
    let mut lines = log.lock().map(|log| log.clone()).unwrap_or_default();
    lines.extend(win_events);
    Ok(lines)
}

/// The WinEvents process `pid` fires during `duration`, as "WinEvent 0x800C object -4 child 0"
/// lines, on a hook this thread pumps for.
#[cfg(windows)]
fn win_events_from(pid: u32, duration: std::time::Duration) -> Vec<String> {
    use std::sync::Mutex;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, EVENT_MAX, EVENT_MIN, MSG, PM_REMOVE,
        WINEVENT_OUTOFCONTEXT,
    };
    static HEARD: Mutex<Vec<String>> = Mutex::new(Vec::new());
    unsafe extern "system" fn heard(
        _hook: HWINEVENTHOOK,
        event: u32,
        _hwnd: HWND,
        object: i32,
        child: i32,
        _thread: u32,
        _time: u32,
    ) {
        if let Ok(mut heard) = HEARD.lock() {
            heard.push(format!(
                "WinEvent {event:#06X} object {object} child {child}"
            ));
        }
    }
    // SAFETY: an out-of-context hook on one process, unhooked below; this thread pumps
    // messages until then.
    let hook = unsafe {
        SetWinEventHook(
            EVENT_MIN,
            EVENT_MAX,
            None,
            Some(heard),
            pid,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    let deadline = std::time::Instant::now() + duration;
    let mut msg = MSG::default();
    while std::time::Instant::now() < deadline {
        // SAFETY: a message loop on this thread.
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // SAFETY: the hook set above.
    let _ = unsafe { UnhookWinEvent(hook) };
    HEARD
        .lock()
        .map(|mut heard| std::mem::take(&mut *heard))
        .unwrap_or_default()
}

#[cfg(not(windows))]
pub fn notification_registrations(_title: &str, _millis: u64) -> Result<Vec<String>, String> {
    Err("UI Automation runs only on Windows".to_owned())
}
