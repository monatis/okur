//! The live capture of accessibility events: WinEvent and UI Automation hooks on a thread of
//! their own, each event read into a `TraceEvent` (`okur_windows_model::event_trace`) as it
//! happens. The screen reader consumes the stream (`run_live`), the harness records it
//! (`run_recording`, written to a trace file by `okur-lab trace`), and parity replays the
//! recordings through the same converter.

use crate::uia_capture::{
    automation, column_headers_of, focus_chain_from, focus_context_of, focus_subtree_of,
    focused_element, window_subtree, window_text, with_properties,
};
use okur_windows_model::event_trace::{
    event_name, in_dialog_box, keeps_dialog_read, reports_position, TraceEvent, TraceObject,
    TracePoint, UIA_FOCUS_WINDOWS, UIA_SELECTION_PROCESSES,
};
use std::cell::{Cell, RefCell};
use windows::core::implement;
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    AccessibleObjectFromEvent, IAccessible, IUIAutomation, SetWinEventHook, UnhookWinEvent,
    HWINEVENTHOOK,
};
use windows::Win32::UI::Accessibility::{
    IUIAutomationElement, IUIAutomationEventHandler, IUIAutomationEventHandler_Impl,
    IUIAutomationFocusChangedEventHandler, IUIAutomationFocusChangedEventHandler_Impl,
    IUIAutomationNotificationEventHandler, IUIAutomationNotificationEventHandler_Impl,
    NotificationKind, NotificationProcessing, TreeScope_Subtree, UIA_LiveRegionChangedEventId,
    UIA_SelectionItem_ElementSelectedEventId, UIA_Window_WindowOpenedEventId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetWindowThreadProcessId, MsgWaitForMultipleObjects, PeekMessageW,
    TranslateMessage, MSG, PM_REMOVE, QS_ALLINPUT,
};

const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
const WINEVENT_SKIPOWNPROCESS: u32 = 0x0002;

/// How soon a focus event naming the focused element again is a repeat of the last one.
const REPEATED_FOCUS: std::time::Duration = std::time::Duration::from_millis(1000);
/// How soon after a focus a selection or value change of the same element is its echo.
const FOCUS_ECHO: std::time::Duration = std::time::Duration::from_millis(300);
/// A WinEvent about an object itself rather than one of its children.
const CHILDID_SELF: i32 = 0;

/// A focus event that was read: the element it named, when, its event's window and MSAA
/// object.
#[derive(Clone)]
struct SeenFocus {
    element: IUIAutomationElement,
    at: std::time::Instant,
    window: isize,
    object: i32,
}

impl SeenFocus {
    /// Whether `element` is this focus's element, named again within `within`.
    fn names(
        &self,
        uia: &IUIAutomation,
        element: &IUIAutomationElement,
        within: std::time::Duration,
    ) -> bool {
        self.at.elapsed() < within
                // SAFETY: comparing two valid automation elements.
                && unsafe { uia.CompareElements(element, &self.element) }
                    .is_ok_and(|same| same.as_bool())
    }
}

struct TraceState {
    /// Receives every event: the trace file, or the live screen reader.
    sink: Box<dyn FnMut(TraceEvent)>,
    uia: Option<IUIAutomation>,
    count: usize,
    /// Feeding the live screen reader: events it does not use are dropped before any
    /// cross-process read, and live-region pages are captured elsewhere.
    live: bool,
    /// The last focus event about a whole object (not a child item): the same element
    /// named again right after is not read twice.
    last_object_focus: Option<SeenFocus>,
    /// The last focus event read, of any kind: a selection or value change for its
    /// element right after is its echo.
    last_focus_read: Option<SeenFocus>,
    /// Told when the page in front changes (live only), at most every
    /// [`PAGE_CHANGE_INTERVAL`].
    page_changed: Option<Box<dyn FnMut(PageChange)>>,
    page_changed_at: Option<std::time::Instant>,
    /// Page changes not told since the last one, for the debug log.
    page_changes_dropped: usize,
    /// Tracing (not live): every page change is recorded as a `page_change` event, with
    /// its object and nothing read (`okur-lab trace --page-changes`).
    record_page_changes: bool,
    /// Where the mouse pointer was when last looked at.
    last_mouse: Option<[i32; 2]>,
    /// When the mouse last moved, until what it points to is traced where it rests.
    mouse_moved_at: Option<std::time::Instant>,
    /// The controls of the dialog in front as last read, for the next focus in it.
    dialog_read: Option<DialogRead>,
    /// The object of the last focus event, by window and object id: the one NetUI state
    /// change that is read.
    focused_object: Option<(isize, i32)>,
}

/// A dialog's controls as read for a focus in it: its layout and the control tree of the
/// page the focus was on (or of the dialog). The Font dialog's tree took 801 ms to read
/// per Tab (median, old-pc, 541; performance review P6); the next focus uses it again
/// while no change came from the dialog's process since ([`DIALOG_CHANGES`]).
struct DialogRead {
    dialog: isize,
    root: isize,
    changes: u64,
    layout: crate::screen_lines::DialogLayout,
    tree: okur_windows_model::uia_snapshot::UiaNode,
}

thread_local! {
    /// The process of the dialog whose controls were last read, and how many changes came
    /// from it. A change is counted here at once, outside the trace state, so none is lost
    /// while that state is in use.
    static DIALOG_CHANGES: Cell<(u32, u64)> = const { Cell::new((0, 0)) };
}

/// The process that owns a window, or 0 for none.
fn process_id(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    // SAFETY: valid out-pointer; a stale or null hwnd yields pid 0.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

/// An event of `hwnd` that may have changed a dialog: counted when it comes from the
/// process of the dialog last read.
fn dialog_changed(event: &str, hwnd: HWND) {
    DIALOG_CHANGES.with(|changes| {
        let (watched, count) = changes.get();
        if watched != 0 && process_id(hwnd) == watched {
            changes.set((watched, count + 1));
            tracing::debug!(event, hwnd = hwnd.0 as isize, "dialog changed");
        }
    });
}

/// The changes counted for `dialog`'s process, which is watched from now on (moving the
/// watch to another process counts as a change).
fn dialog_changes(dialog: HWND) -> u64 {
    let pid = process_id(dialog);
    DIALOG_CHANGES.with(|changes| {
        let (watched, count) = changes.get();
        if watched == pid {
            count
        } else {
            changes.set((pid, count + 1));
            count + 1
        }
    })
}

/// Whether a window is a Win32 push button (`BS_PUSHBUTTON` or `BS_DEFPUSHBUTTON`).
fn is_push_button(hwnd: HWND, class: &str) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongW, GWL_STYLE};
    const BS_TYPEMASK: i32 = 0xF;
    const BS_DEFPUSHBUTTON: i32 = 1;
    class.eq_ignore_ascii_case("Button") && {
        // SAFETY: a style read of a window; a stale one reads 0.
        let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) };
        style & BS_TYPEMASK <= BS_DEFPUSHBUTTON
    }
}

/// The shortest time between two page changes told: a page being built reports its
/// objects shown and reordered by the hundred.
const PAGE_CHANGE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);

thread_local! {
    // The out-of-context hook calls back on the installing thread, so thread-local
    // state is sufficient and needs no locking.
    static STATE: RefCell<Option<TraceState>> = const { RefCell::new(None) };
    /// Set while an event is read (see [`in_turn`]).
    static READING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The events that arrived while one was read, in order.
    static DEFERRED: RefCell<std::collections::VecDeque<Deferred>> =
        const { RefCell::new(std::collections::VecDeque::new()) };
    /// How many waiting events were dropped since the queue last drained.
    static DROPPED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// When a console last wrote to its screen, and its window: read once its writing
    /// pauses ([`record_console_text`]).
    static CONSOLE_WRITTEN: std::cell::Cell<Option<(std::time::Instant, isize)>> =
        const { std::cell::Cell::new(None) };
}

/// How long a console's writing pauses before its screen is read: a command's output
/// comes in many updates, and the screen is read once for them.
const CONSOLE_QUIET: std::time::Duration = std::time::Duration::from_millis(150);

/// A classic console wrote to its screen (`EVENT_CONSOLE_UPDATE_REGION`, `_SIMPLE`,
/// `_SCROLL`): noted, and read once it pauses. Nothing is read here: output comes in bursts.
unsafe extern "system" fn on_console_update(
    _hook: HWINEVENTHOOK,
    _win_event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    CONSOLE_WRITTEN.with(|written| {
        written.set(Some((std::time::Instant::now(), hwnd.0 as isize)));
    });
}

/// The console in front's screen, once its writing has paused for [`CONSOLE_QUIET`]: a
/// `console_update` with the rows it shows, which the engine compares with the rows before
/// for Screen Echo.
fn record_console_text() {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let Some((at, window)) = CONSOLE_WRITTEN.with(std::cell::Cell::get) else {
        return;
    };
    if at.elapsed() < CONSOLE_QUIET {
        return;
    }
    CONSOLE_WRITTEN.with(|written| written.set(None));
    // SAFETY: plain Win32 call with no preconditions.
    if unsafe { GetForegroundWindow() }.0 as isize != window {
        return;
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        let Some(text) = state
            .uia
            .as_ref()
            .and_then(crate::text_context::visible_text)
        else {
            return;
        };
        let mut event = mouse_event(None);
        event.event = "console_update".to_owned();
        event.hwnd = window;
        event.window_class = Some("ConsoleWindowClass".to_owned());
        event.text_context = Some(okur_core::TextContext {
            text,
            caret: 0,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        });
        (state.sink)(event);
        state.count += 1;
    });
}

/// A WinEvent that arrived while another was read, read after it.
struct Deferred {
    t_ms: u128,
    name: &'static str,
    win_event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
}

/// The most events kept waiting: past it the oldest are dropped, since the newest say where
/// the focus is now, and their count is logged once the queue drains (performance review
/// 2026-10-25, 20).
const MAX_DEFERRED: usize = 512;

/// Whether a newer caret or selection event of `hwnd` is already waiting: the text read
/// for this one would see what that one's sees, after the reads before it. Word fires dozens
/// of caret events a key, and reading each had the tracer seconds behind the keys in Word's
/// form (one trial of 1055: 117 such reads, 27 s).
fn newer_caret_waiting(hwnd: HWND) -> bool {
    DEFERRED.with(|d| {
        d.borrow()
            .iter()
            .any(|e| e.hwnd == hwnd && matches!(e.name, "caret" | "text_selection_change"))
    })
}
/// Office's NetUI windows: the ribbon, the status bar.
const NETUI_CLASS: &str = "NetUIHWND";
/// How long a stopping trace reads what still waits, at most, and without a message.
const DRAIN_MAX: std::time::Duration = std::time::Duration::from_secs(60);
const DRAIN_IDLE: std::time::Duration = std::time::Duration::from_secs(1);

/// Runs `read` with the tracer marked busy, then the events that arrived meanwhile, in
/// order. A cross-process read (MSAA's `WM_GETOBJECT`) waits in `SendMessage`, where
/// Windows delivers the next WinEvent on this thread before the read returns: read there,
/// it borrowed `STATE` inside the first event's borrow and aborted the reader
/// (performance review 2026-10-23, P1).
///
/// Each read runs under [`guarded`]: the message pump's own reads did not, and a panic in
/// one would have ended event speech for the session (performance review 2026-10-25,
/// finding 11); the tracer is never left marked busy either.
fn in_turn(read: impl FnOnce()) {
    READING.with(|r| r.set(true));
    guarded("read", read);
    while let Some(event) = DEFERRED.with(|d| d.borrow_mut().pop_front()) {
        guarded("deferred event", || {
            record_at(
                event.t_ms,
                event.name,
                event.win_event,
                event.hwnd,
                event.id_object,
                event.id_child,
                None,
            )
        });
    }
    READING.with(|r| r.set(false));
    let dropped = DROPPED.with(|d| d.take());
    if dropped > 0 {
        tracing::warn!(
            dropped,
            "events dropped while one was read: too many waiting"
        );
    }
}

/// [`record_at`] now, or after the event being read when one is.
fn record_in_turn(event: Deferred) {
    if READING.with(|r| r.get()) {
        DEFERRED.with(|d| {
            let mut deferred = d.borrow_mut();
            if deferred.len() >= MAX_DEFERRED {
                deferred.pop_front();
                DROPPED.with(|n| n.set(n.get() + 1));
            }
            deferred.push_back(event);
        });
        return;
    }
    in_turn(|| {
        record_at(
            event.t_ms,
            event.name,
            event.win_event,
            event.hwnd,
            event.id_object,
            event.id_child,
            None,
        )
    });
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn process_name(hwnd: HWND) -> Option<String> {
    let mut pid = 0u32;
    // SAFETY: valid out-pointer; a stale hwnd yields pid 0.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    process_name_of(pid)
}

/// The product name and version of the executable of the process that owns `hwnd`, from
/// its version resource (ProductName, ProductVersion), read once per executable.
fn process_product_of(hwnd: HWND) -> (Option<String>, Option<String>) {
    type Product = (Option<String>, Option<String>);
    thread_local! {
        static PRODUCTS: std::cell::RefCell<std::collections::HashMap<String, Product>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
    }
    let mut pid = 0u32;
    // SAFETY: valid out-pointer; a stale hwnd yields pid 0.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    let Some(path) = process_image_of(pid) else {
        return (None, None);
    };
    PRODUCTS.with(|products| {
        products
            .borrow_mut()
            .entry(path.clone())
            .or_insert_with(|| version_strings(&path))
            .clone()
    })
}

/// ProductName and ProductVersion from the version resource of the file at `path`.
fn version_strings(path: &str) -> (Option<String>, Option<String>) {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
    };
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: NUL-terminated path; the size query writes nothing else.
    let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(wide.as_ptr()), None) };
    if size == 0 {
        return (None, None);
    }
    let mut data = vec![0u8; size as usize];
    // SAFETY: the buffer is `size` bytes, as asked.
    let read =
        unsafe { GetFileVersionInfoW(PCWSTR(wide.as_ptr()), None, size, data.as_mut_ptr().cast()) };
    if read.is_err() {
        return (None, None);
    }
    // A value of the version block: a pointer into `data` and its length.
    let query = |sub: &str| -> Option<(*const std::ffi::c_void, u32)> {
        let sub: Vec<u16> = sub.encode_utf16().chain(std::iter::once(0)).collect();
        let mut value: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        // SAFETY: `data` holds the block GetFileVersionInfoW filled; the result points
        // into it.
        let found = unsafe {
            VerQueryValueW(
                data.as_ptr().cast(),
                PCWSTR(sub.as_ptr()),
                &mut value,
                &mut len,
            )
        }
        .as_bool();
        (found && !value.is_null() && len > 0).then_some((value.cast_const(), len))
    };
    // The first language and code page the file lists, else US English in Unicode.
    let translation = query("\\VarFileInfo\\Translation")
        .filter(|&(_, len)| len >= 4)
        .map(|(value, _)| {
            // SAFETY: at least four bytes: a language and a code page, two u16s.
            let pair = unsafe { std::slice::from_raw_parts(value.cast::<u16>(), 2) };
            format!(
                "{:04x}{:04x}",
                pair.first().copied().unwrap_or(0x0409),
                pair.get(1).copied().unwrap_or(0x04b0)
            )
        })
        .unwrap_or_else(|| "040904b0".to_owned());
    let string = |name: &str| -> Option<String> {
        let (value, len) = query(&format!("\\StringFileInfo\\{translation}\\{name}"))?;
        // SAFETY: a string value of `len` UTF-16 units, its NUL included.
        let units = unsafe { std::slice::from_raw_parts(value.cast::<u16>(), len as usize) };
        let text = String::from_utf16_lossy(units);
        let text = text.trim_end_matches('\0').trim().to_owned();
        (!text.is_empty()).then_some(text)
    };
    (string("ProductName"), string("ProductVersion"))
}

/// Notepad's own version resource names Windows as its product (live: reads a system
/// file).
#[test]
#[ignore = "live: reads C:\\Windows\\notepad.exe"]
fn a_system_executable_names_its_product_and_version() {
    let (product, version) = version_strings("C:\\Windows\\notepad.exe");
    assert!(product.is_some_and(|p| p.contains("Windows")));
    assert!(version.is_some_and(|v| v.starts_with("10.")));
}

/// The executable's file name of process `pid`.
fn process_name_of(pid: u32) -> Option<String> {
    process_image_of(pid).and_then(|path| path.rsplit('\\').next().map(str::to_owned))
}

/// The executable's full path of process `pid`.
fn process_image_of(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    // SAFETY: limited-information query of another process; the handle is closed below.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(handle);
        if !ok {
            return None;
        }
        Some(String::from_utf16_lossy(
            buf.get(..len as usize).unwrap_or_default(),
        ))
    }
}

/// The selection of a Win32 combo box or list box as `[index, count]`, asked of the
/// control itself (`CB_GETCURSEL`/`CB_GETCOUNT`, `LB_GETCURSEL`/`LB_GETCOUNT`). A
/// collapsed WinForms combo box exposes no items through UIA, yet JAWS says "2 of 3".
fn win32_list_position(hwnd: HWND, class: &str) -> Option<[i32; 2]> {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetParent, SendMessageTimeoutW, SMTO_ABORTIFHUNG,
    };
    // An edit inside a combo box: the combo box's selection.
    if class.eq_ignore_ascii_case("Edit") {
        // SAFETY: plain Win32 query on a window handle.
        let parent = unsafe { GetParent(hwnd) }.ok()?;
        let (_, parent_class) = window_text(parent);
        return parent_class
            .to_ascii_uppercase()
            .contains("COMBOBOX")
            .then(|| win32_list_position(parent, &parent_class))
            .flatten();
    }
    let upper = class.to_ascii_uppercase();
    // A list view: its focused item and item count (LVM_GETNEXTITEM with LVNI_FOCUSED,
    // searching from the start; LVM_GETITEMCOUNT). Counting its items through UIA took
    // 2 s per item in a 300-item list (dev box, 2026-10-15).
    if upper.contains("SYSLISTVIEW32") {
        const LVM_GETITEMCOUNT: u32 = 0x1004;
        const LVM_GETNEXTITEM: u32 = 0x100C;
        const LVNI_FOCUSED: isize = 0x0001;
        let ask = |msg: u32, wparam: usize, lparam: isize| {
            let mut result = 0usize;
            // SAFETY: read-only integer queries with a timeout, to a window that may be
            // gone.
            let sent = unsafe {
                SendMessageTimeoutW(
                    hwnd,
                    msg,
                    WPARAM(wparam),
                    LPARAM(lparam),
                    SMTO_ABORTIFHUNG,
                    200,
                    Some(&mut result),
                )
            };
            (sent.0 != 0).then_some(result as i32)
        };
        let count = ask(LVM_GETITEMCOUNT, 0, 0).filter(|&n| n > 0)?;
        let index = ask(LVM_GETNEXTITEM, usize::MAX, LVNI_FOCUSED).filter(|&i| i >= 0)?;
        return Some([index, count]);
    }
    if upper == "BUTTON" {
        return win32_radio_position(hwnd);
    }
    let (get_sel, get_count) = if upper.contains("COMBOBOX") {
        (0x0147, 0x0146)
    } else if upper.contains("LISTBOX") {
        (0x0188, 0x018B)
    } else {
        return None;
    };
    let ask_with = |msg: u32, wparam: WPARAM, lparam: LPARAM| {
        let mut result = 0usize;
        // SAFETY: a read-only query message with a timeout, to a window that may be gone;
        // a string parameter outlives the call (the system copies it across processes).
        let sent = unsafe {
            SendMessageTimeoutW(
                hwnd,
                msg,
                wparam,
                lparam,
                SMTO_ABORTIFHUNG,
                200,
                Some(&mut result),
            )
        };
        // CB_ERR/LB_ERR is -1, as the low 32 bits of the result.
        (sent.0 != 0).then_some(result as i32)
    };
    let ask = |msg: u32| ask_with(msg, WPARAM(0), LPARAM(0));
    let count = ask(get_count).filter(|&n| n > 0)?;
    let index = ask(get_sel).filter(|&i| i >= 0).or_else(|| {
        // An editable combo box's typed text selects nothing: its position is that of
        // the list item with the same text (JAWS on the Run dialog: "C: | 1 of 6").
        const CB_FINDSTRINGEXACT: u32 = 0x0158;
        if !upper.contains("COMBOBOX") {
            return None;
        }
        let text = crate::screen_lines::control_text(hwnd);
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        ask_with(
            CB_FINDSTRINGEXACT,
            WPARAM(usize::MAX),
            LPARAM(wide.as_ptr() as isize),
        )
        .filter(|&i| i >= 0)
    })?;
    Some([index, count])
}

/// A radio button's place among the radio buttons of its group, as Windows groups them
/// (from a control with `WS_GROUP` to the next, the run its arrow keys move in): JAWS's
/// Links List says "Sort Links In Tab Order radio button checked 1 of 2" and, in the
/// group after it, "Display All Links ... 1 of 3" [obs, 222/s5-s6]. Not a radio button:
/// `None`.
fn win32_radio_position(hwnd: HWND) -> Option<[i32; 2]> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindow, GetWindowLongW, GWL_STYLE, GW_HWNDNEXT, GW_HWNDPREV,
    };
    const WS_GROUP: i32 = 0x0002_0000;
    const BS_TYPEMASK: i32 = 0x000F;
    const BS_RADIOBUTTON: i32 = 0x0004;
    const BS_AUTORADIOBUTTON: i32 = 0x0009;
    // Enough for any dialog; a broken window chain cannot loop forever.
    const MOST: usize = 256;
    // SAFETY: plain Win32 queries on window handles that may be gone (they then fail).
    let style = |h: HWND| unsafe { GetWindowLongW(h, GWL_STYLE) };
    let is_radio = |h: HWND| {
        window_text(h).1.eq_ignore_ascii_case("Button")
            && matches!(style(h) & BS_TYPEMASK, BS_RADIOBUTTON | BS_AUTORADIOBUTTON)
    };
    // SAFETY: as above.
    let step = |h: HWND, way| {
        unsafe { GetWindow(h, way) }
            .ok()
            .filter(|n| !n.is_invalid())
    };
    if !is_radio(hwnd) {
        return None;
    }
    let mut first = hwnd;
    for _ in 0..MOST {
        if style(first) & WS_GROUP != 0 {
            break;
        }
        match step(first, GW_HWNDPREV) {
            Some(previous) => first = previous,
            None => break,
        }
    }
    let (mut index, mut count, mut window) = (None, 0, first);
    for _ in 0..MOST {
        if is_radio(window) {
            if window == hwnd {
                index = Some(count);
            }
            count += 1;
        }
        match step(window, GW_HWNDNEXT).filter(|n| style(*n) & WS_GROUP == 0) {
            Some(next) => window = next,
            None => break,
        }
    }
    Some([index?, count])
}

/// A Chromium live region that changed, with its subtree, and the page after the change.
fn chromium_live_region(
    hwnd: HWND,
    class: &str,
    child_id: i32,
    with_page: bool,
) -> (
    Option<okur_windows_model::ia2_snapshot::Ia2Node>,
    Option<okur_windows_model::ia2_snapshot::Ia2Node>,
) {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT};
    if class != "Chrome_RenderWidgetHostHWND" {
        return (None, None);
    }
    // SAFETY: plain Win32 query on a window handle.
    let top = unsafe { GetAncestor(hwnd, GA_ROOT) };
    (
        xa11y_windows::ia2::capture_chromium_object(top, child_id).ok(),
        with_page
            .then(|| xa11y_windows::ia2::capture_chromium_document(top).ok())
            .flatten(),
    )
}

/// The title and class of `hwnd`'s top-level window, and the virtual desktop's name when
/// that window is the desktop.
/// The top-level window of `hwnd`: its title, its class, the virtual desktop's name when
/// it is the desktop, and whether its owner stays available (a modeless dialog's does).
fn top_window(hwnd: HWND) -> TopWindow {
    use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GetWindow, GA_ROOT, GW_OWNER};
    // SAFETY: plain Win32 query on a window handle.
    let top = unsafe { GetAncestor(hwnd, GA_ROOT) };
    if top.is_invalid() {
        return TopWindow::default();
    }
    let (title, class) = window_text(top);
    let desktop =
        matches!(class.as_str(), "Progman" | "WorkerW").then(crate::virtual_desktop::current_name);
    // SAFETY: plain Win32 queries on window handles.
    let owner_enabled = unsafe { GetWindow(top, GW_OWNER) }
        .ok()
        .filter(|owner| !owner.is_invalid())
        .map(|owner| unsafe { IsWindowEnabled(owner) }.as_bool());
    TopWindow {
        title: Some(title).filter(|t| !t.is_empty()),
        class: Some(class).filter(|c| !c.is_empty()),
        desktop,
        owner_enabled,
    }
}

/// The window in front, as [`top_window`] reads it.
#[derive(Default)]
struct TopWindow {
    title: Option<String>,
    class: Option<String>,
    desktop: Option<String>,
    owner_enabled: Option<bool>,
}

/// The events the live screen reader consumes ([`EventConverter`]). Not caret and text
/// selection events: live, the actions worker reads the field after each key, and without
/// their text the converter makes nothing of them, though reading their MSAA object cost
/// the WinEvent thread a cross-process call or six for each, 16% of all events (performance
/// review, 2026-10-20, P5; `caret_events_without_their_text_give_nothing`).
const LIVE_EVENTS: [&str; 14] = [
    "focus",
    "dialog_shown",
    "uia_focus",
    "uia_selected",
    "foreground",
    "value_change",
    "state_change",
    "selection",
    "live_region_change",
    "alert",
    "menu_start",
    "menu_end",
    "menu_popup_start",
    "menu_popup_end",
];

/// Whether the application of `hwnd`'s window has stopped answering (Windows calls a window
/// hung once it has pumped no message for 5 s).
fn in_hung_window(hwnd: HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, IsHungAppWindow, GA_ROOT};
    // SAFETY: plain Win32 queries on a window handle that may be gone.
    unsafe {
        let top = GetAncestor(hwnd, GA_ROOT);
        !top.is_invalid() && IsHungAppWindow(top).as_bool()
    }
}

/// Whether `hwnd` belongs to the process of the foreground window.
fn in_foreground_process(hwnd: HWND) -> bool {
    in_front_application(hwnd) == Some(true)
}

/// Whether `hwnd` is the application in front's: a window of its process, of the UWP app
/// its frame shows ([`foreground_processes`]), or a window of another process inside the
/// one in front. Windows Settings' controls are its own process's core window, inside a
/// frame of ApplicationFrameHost's: a focus there taken for a process behind left Okur
/// silent in all of Settings (old-pc, 2026-11-02). None while either is unknown (the
/// window in front is briefly none while windows switch).
fn in_front_application(hwnd: HWND) -> Option<bool> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetForegroundWindow, GetWindowThreadProcessId, GA_ROOT,
    };
    let mut event_pid = 0u32;
    // SAFETY: plain Win32 queries with a valid out-pointer.
    let (foreground, root) = unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut event_pid));
        (GetForegroundWindow(), GetAncestor(hwnd, GA_ROOT))
    };
    let [frame, app] = processes_of_front(foreground);
    if event_pid == 0 || frame == 0 {
        return None;
    }
    Some(event_pid == frame || event_pid == app || root == foreground)
}

/// Whether `hwnd` is inside a file dialog's navigation pane (Windows'
/// `NamespaceTreeControl`), a few windows up.
fn in_namespace_tree(hwnd: HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetParent;
    let mut window = hwnd;
    for _ in 0..6 {
        // SAFETY: plain Win32 query on a window handle, which may be gone.
        let Ok(parent) = (unsafe { GetParent(window) }) else {
            return false;
        };
        if window_text(parent).1 == "NamespaceTreeControl" {
            return true;
        }
        window = parent;
    }
    false
}

/// Whether `hwnd` belongs to an application other than the one in front, both known (a
/// window in front is briefly none while windows switch).
fn in_background_process(hwnd: HWND) -> bool {
    in_front_application(hwnd) == Some(false)
}

fn msaa_object(hwnd: HWND, id_object: i32, id_child: i32) -> Option<TraceObject> {
    let mut acc: Option<IAccessible> = None;
    let mut child = VARIANT::default();
    // SAFETY: standard MSAA event-object resolution with valid out-pointers.
    unsafe {
        AccessibleObjectFromEvent(
            hwnd,
            id_object as u32,
            id_child as u32,
            &mut acc,
            &mut child,
        )
        .ok()?;
        let acc = acc?;
        Some(TraceObject {
            role: acc
                .get_accRole(&child)
                .ok()
                .and_then(|v| i32::try_from(&v).ok()),
            name: acc.get_accName(&child).ok().map(|b| b.to_string()),
            value: acc
                .get_accValue(&child)
                .ok()
                .map(|b| b.to_string())
                .filter(|s| !s.is_empty()),
            states: acc
                .get_accState(&child)
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
                .map(|s| s as u32)
                .unwrap_or(0),
            shortcut: acc
                .get_accKeyboardShortcut(&child)
                .ok()
                .map(|b| b.to_string())
                .filter(|s| !s.trim().is_empty()),
        })
    }
}

/// The window class of NetUI, the ribbon's framework (WordPad's, Paint's, Office's).
const NETUI_WINDOW: &str = "NetUIHWND";
/// How deep and how many of a NetUI focus's MSAA objects are read: its ribbon's tabs and
/// their groups, not every button of every group.
const NETUI_DEPTH: usize = 3;
const NETUI_BUDGET: usize = 150;

/// MSAA's roles of the objects whose children a NetUI focus reads: the ribbon itself (a
/// property page) and the panes and groupings around a tab row.
const NETUI_CONTAINERS: [i32; 4] = [0x0A, 0x10, 0x14, 0x26];
/// MSAA's window role: where a NetUI control's ancestors stop.
const MSAA_WINDOW: i32 = 0x09;
/// How many of a NetUI control's ancestors are read at most.
const NETUI_ANCESTORS: usize = 8;

/// What a NetUI focus reads of its MSAA objects ([`netui_read`]).
#[derive(Default)]
struct NetuiRead {
    tree: Option<okur_windows_model::event_trace::MsaaNode>,
    siblings: Vec<okur_windows_model::event_trace::MsaaNode>,
    ancestors: Vec<okur_windows_model::event_trace::MsaaNode>,
    description: Option<String>,
    control: Option<okur_windows_model::event_trace::NetuiControl>,
}

/// For a NetUI focus whose MSAA object has `role`: a container's children to
/// [`NETUI_DEPTH`] (the ribbon Alt focuses), at most [`NETUI_BUDGET`] objects; a control's
/// siblings without theirs, its ancestors up to the window, its description, and what
/// Office's own UIA element for it says ([`netui_control`]).
fn netui_read(
    uia: Option<&IUIAutomation>,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    role: i32,
) -> NetuiRead {
    use windows::Win32::UI::Accessibility::IAccessible;
    let mut acc: Option<IAccessible> = None;
    let mut child = VARIANT::default();
    // SAFETY: standard MSAA event-object resolution with valid out-pointers.
    let found = unsafe {
        AccessibleObjectFromEvent(
            hwnd,
            id_object as u32,
            id_child as u32,
            &mut acc,
            &mut child,
        )
    };
    let Some(acc) = acc.filter(|_| found.is_ok()) else {
        return NetuiRead::default();
    };
    let mut budget = NETUI_BUDGET;
    if NETUI_CONTAINERS.contains(&role) {
        return NetuiRead {
            tree: Some(msaa_node(&acc, &child, NETUI_DEPTH, &mut budget)),
            ..NetuiRead::default()
        };
    }
    // SAFETY: a property read on a valid IAccessible.
    let description = unsafe { acc.get_accDescription(&child) }
        .ok()
        .map(|d| d.to_string())
        .filter(|d| !d.trim().is_empty());
    let control = uia.and_then(|uia| netui_control(uia, &acc, &child));
    // A simple element's parent is the object that named it; a full object asks its own.
    let parent_of = |object: &IAccessible| {
        // SAFETY: a property read on a valid IAccessible.
        unsafe { object.accParent() }
            .ok()
            .and_then(|p| windows::core::Interface::cast::<IAccessible>(&p).ok())
    };
    let parent = if is_self(&child) {
        parent_of(&acc)
    } else {
        Some(acc)
    };
    let siblings = parent
        .as_ref()
        .map(|p| msaa_node(p, &VARIANT::from(0i32), 1, &mut budget).children)
        .unwrap_or_default();
    let mut ancestors = Vec::new();
    let mut next = parent;
    while let Some(object) = next.take() {
        let mut node = msaa_node(&object, &VARIANT::from(0i32), 0, &mut budget);
        if node.role == Some(MSAA_WINDOW) || ancestors.len() == NETUI_ANCESTORS {
            break;
        }
        let own = VARIANT::from(0i32);
        // SAFETY: property reads on a valid IAccessible.
        let (value, description) = unsafe {
            (
                object.get_accValue(&own).ok(),
                object.get_accDescription(&own).ok(),
            )
        };
        let text = |b: Option<windows::core::BSTR>| {
            b.map(|b| b.to_string()).filter(|t| !t.trim().is_empty())
        };
        node.value = text(value);
        node.description = text(description);
        ancestors.push(node);
        next = parent_of(&object);
    }
    ancestors.reverse();
    NetuiRead {
        tree: None,
        siblings,
        ancestors,
        description,
        control,
    }
}

/// What Office's own UIA element for a NetUI control says ([`okur_windows_model::event_trace::NetuiControl`]): the
/// element at the MSAA object's center, when it has the object's name. 10 ms a control on
/// old-pc, where searching the ribbon's UIA tree for the element with the keyboard focus
/// took 20 to 50 (`okur-lab msaa-focus`, 2026-10-25).
fn netui_control(
    uia: &IUIAutomation,
    acc: &windows::Win32::UI::Accessibility::IAccessible,
    child: &VARIANT,
) -> Option<okur_windows_model::event_trace::NetuiControl> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::Accessibility::{IUIAutomationTogglePattern, UIA_TogglePatternId};
    let (mut x, mut y, mut w, mut h) = (0, 0, 0, 0);
    // SAFETY: out-pointers to locals on a valid IAccessible.
    unsafe { acc.accLocation(&mut x, &mut y, &mut w, &mut h, child) }.ok()?;
    // SAFETY: a property read on a valid IAccessible.
    let name = unsafe { acc.get_accName(child) }
        .ok()
        .map(|b| b.to_string());
    let center = POINT {
        x: x.saturating_add(w / 2),
        y: y.saturating_add(h / 2),
    };
    // SAFETY: UIA reads on a valid automation object and the element it gives.
    let element = unsafe { uia.ElementFromPoint(center) }
        .ok()
        .filter(|e| unsafe { e.CurrentName() }.ok().map(|b| b.to_string()) == name);
    // SAFETY: UIA reads on a valid element.
    let (help_text, toggle_state, control_type) =
        element.map_or((None, None, None), |element| unsafe {
            (
                element
                    .CurrentHelpText()
                    .ok()
                    .map(|b| b.to_string())
                    .filter(|t| !t.trim().is_empty()),
                element
                    .GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
                    .ok()
                    .and_then(|p| p.CurrentToggleState().ok())
                    .map(|s| s.0),
                element.CurrentControlType().ok().map(|c| c.0),
            )
        });
    Some(okur_windows_model::event_trace::NetuiControl {
        bounds: Some([x, y, x.saturating_add(w), y.saturating_add(h)]),
        help_text,
        toggle_state,
        control_type,
    })
}

/// Whether an MSAA child variant names the object itself (`CHILDID_SELF`).
fn is_self(child: &VARIANT) -> bool {
    i32::try_from(child).map_or(true, |id| id == 0)
}

/// One MSAA object, and its children to `depth` while `budget` lasts.
fn msaa_node(
    acc: &windows::Win32::UI::Accessibility::IAccessible,
    child: &VARIANT,
    depth: usize,
    budget: &mut usize,
) -> okur_windows_model::event_trace::MsaaNode {
    use windows::Win32::System::Variant::VT_DISPATCH;
    use windows::Win32::UI::Accessibility::{AccessibleChildren, IAccessible};
    *budget = budget.saturating_sub(1);
    // SAFETY: property reads on a valid IAccessible.
    let mut node = unsafe {
        okur_windows_model::event_trace::MsaaNode {
            role: acc
                .get_accRole(child)
                .ok()
                .and_then(|v| i32::try_from(&v).ok()),
            name: acc
                .get_accName(child)
                .ok()
                .map(|b| b.to_string())
                .filter(|n| !n.trim().is_empty()),
            states: acc
                .get_accState(child)
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
                .map_or(0, |s| s as u32),
            value: None,
            description: None,
            children: Vec::new(),
        }
    };
    if depth == 0 || *budget == 0 || !is_self(child) {
        return node;
    }
    // SAFETY: a property read on a valid IAccessible.
    let count = unsafe { acc.accChildCount() }.unwrap_or(0).clamp(0, 200);
    let mut children = vec![VARIANT::default(); usize::try_from(count).unwrap_or(0)];
    let mut obtained = 0i32;
    // SAFETY: the slice's length is the count asked for.
    if unsafe { AccessibleChildren(acc, 0, &mut children, &mut obtained) }.is_err() {
        return node;
    }
    for item in children.iter().take(usize::try_from(obtained).unwrap_or(0)) {
        if *budget == 0 {
            break;
        }
        // SAFETY: reading the discriminant and, for VT_DISPATCH, the dispatch pointer.
        let object = unsafe {
            (item.Anonymous.Anonymous.vt == VT_DISPATCH)
                .then(|| item.Anonymous.Anonymous.Anonymous.pdispVal.as_ref())
                .flatten()
                .and_then(|d| windows::core::Interface::cast::<IAccessible>(d).ok())
        };
        node.children.push(match object {
            Some(object) => msaa_node(&object, &VARIANT::from(0i32), depth - 1, budget),
            None => msaa_node(acc, item, 0, budget),
        });
    }
    node
}

/// A panic unwinding out of a WinEvent callback, an `extern "system"` function, aborts the
/// process: each callback's work runs under this guard, and a panic is logged instead
/// (performance review 2026-10-24, 5).
fn guarded(callback: &'static str, work: impl FnOnce()) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).is_err() {
        tracing::error!(
            callback,
            "a WinEvent callback panicked; the event is dropped"
        );
    }
}

unsafe extern "system" fn on_event(
    hook: HWINEVENTHOOK,
    win_event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    thread: u32,
    time: u32,
) {
    // SAFETY: the same call with the same arguments Windows passed.
    guarded("on_event", || unsafe {
        on_event_guarded(hook, win_event, hwnd, id_object, id_child, thread, time)
    });
}

unsafe fn on_event_guarded(
    _hook: HWINEVENTHOOK,
    win_event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _thread: u32,
    time: u32,
) {
    let Some(name) = event_name(win_event, id_object) else {
        return;
    };
    record_in_turn(Deferred {
        t_ms: event_ms(time),
        name,
        win_event,
        hwnd,
        id_object,
        id_child,
    });
}

/// When an event happened, in Unix milliseconds, from its `dwmsEventTime` (the tick count
/// when Windows raised it): events wait in the hook's queue while the ones before them are
/// read, and a trace places each in the step it belongs to by this time.
fn event_ms(time: u32) -> u128 {
    use windows::Win32::System::SystemInformation::GetTickCount;
    // SAFETY: plain Win32 call with no preconditions.
    let waited = unsafe { GetTickCount() }.wrapping_sub(time);
    now_ms().saturating_sub(u128::from(waited))
}

/// Objects shown, hidden or reordered in the page in front (`EVENT_OBJECT_SHOW`, `_HIDE`,
/// `_REORDER`): the page changed. Chromium reports its web objects on the page's
/// "Chrome Legacy Window" by their negative uniqueIDs. Nothing is read across processes:
/// a page being built sends these by the hundred.
unsafe extern "system" fn on_page_change(
    hook: HWINEVENTHOOK,
    win_event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    thread: u32,
    time: u32,
) {
    // SAFETY: the same call with the same arguments Windows passed.
    guarded("on_page_change", || unsafe {
        on_page_change_guarded(hook, win_event, hwnd, id_object, id_child, thread, time)
    });
}

unsafe fn on_page_change_guarded(
    _hook: HWINEVENTHOOK,
    win_event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _thread: u32,
    _time: u32,
) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetClassNameW, GetForegroundWindow, IsWindowVisible, GA_ROOT,
    };
    const OBJID_WINDOW: i32 = 0;
    const OBJID_CLIENT: i32 = -4;
    // A control shown, hidden or moved in a dialog last read (a property sheet's page
    // switched) changes it; the caret and the mouse pointer shown or hidden do not.
    if matches!(id_object, OBJID_WINDOW | OBJID_CLIENT) {
        dialog_changed(if win_event == 0x8003 { "hide" } else { "show" }, hwnd);
    }
    // The dialog in front shown: its controls are read again, a focus having arrived while
    // it was still hidden (Notepad's Font dialog), as Say Window reads it shown [obs,
    // 665/s3].
    const EVENT_OBJECT_SHOW: u32 = 0x8002;
    if win_event == EVENT_OBJECT_SHOW && id_object == OBJID_WINDOW && id_child == 0 {
        let mut class = [0u16; 16];
        // SAFETY: plain Win32 queries on the event's window, with a valid buffer.
        let front_dialog = unsafe {
            GetForegroundWindow() == hwnd && {
                let length = GetClassNameW(hwnd, &mut class);
                class.get(..length as usize).is_some_and(|c| {
                    okur_windows_model::event_trace::WIN32_DIALOG_CLASS
                        .encode_utf16()
                        .eq(c.iter().copied())
                })
            }
        };
        if front_dialog {
            record_in_turn(Deferred {
                t_ms: now_ms(),
                name: "dialog_shown",
                win_event,
                hwnd,
                id_object,
                id_child,
            });
        }
    }
    if id_object != OBJID_CLIENT || id_child >= 0 {
        return;
    }
    let mut class = [0u16; 32];
    // SAFETY: plain Win32 queries on the event's window, with a valid buffer.
    let in_page_in_front = unsafe {
        let length = GetClassNameW(hwnd, &mut class);
        class
            .get(..length as usize)
            .is_some_and(|class| CHROME_PAGE_WINDOW.encode_utf16().eq(class.iter().copied()))
            && IsWindowVisible(hwnd).as_bool()
            && GetAncestor(hwnd, GA_ROOT) == GetForegroundWindow()
    };
    if !in_page_in_front {
        return;
    }
    STATE.with(|s| {
        let Ok(mut state) = s.try_borrow_mut() else {
            return;
        };
        let Some(state) = state.as_mut() else {
            return;
        };
        // A trace of focus alone takes these events for its dialogs only.
        if !state.live && !state.record_page_changes {
            return;
        }
        if state.record_page_changes {
            let event = TraceEvent {
                t_ms: now_ms(),
                event: "page_change".to_owned(),
                win_event,
                hwnd: hwnd.0 as isize,
                id_object,
                id_child,
                window_class: Some(CHROME_PAGE_WINDOW.to_owned()),
                window_style: None,
                spin_box: false,
                focus_window_class: None,
                menu_bar: Vec::new(),
                status_bar: Vec::new(),
                window_title: None,
                process: None,
                process_product: None,
                process_version: None,
                object: None,
                focus_chain: Vec::new(),
                focus_siblings: Vec::new(),
                focus_subtree: None,
                list_position: None,
                column_headers: Vec::new(),
                text_context: None,
                region: None,
                page: None,
                top_window_title: None,
                top_window_class: None,
                top_window_owner_enabled: None,
                virtual_desktop: None,
                dialog_lines: Vec::new(),
                dialog_text: Vec::new(),
                dialog_page: None,
                group_box: None,
                default_button: None,
                dialog_tree: None,
                notification: None,
                mouse: None,
                point: None,
                msaa_tree: None,
                msaa_siblings: Vec::new(),
                msaa_ancestors: Vec::new(),
                msaa_description: None,
                netui_control: None,
                modal_dialog: None,
            };
            (state.sink)(event);
            state.count += 1;
            return;
        }
        // Every change is passed on with its object; the engine is told at most every
        // PAGE_CHANGE_INTERVAL.
        let notify = !state
            .page_changed_at
            .is_some_and(|at| at.elapsed() < PAGE_CHANGE_INTERVAL);
        if notify {
            tracing::debug!(
                event = win_event,
                object = id_child,
                dropped_before = state.page_changes_dropped,
                "page changed"
            );
            state.page_changed_at = Some(std::time::Instant::now());
            state.page_changes_dropped = 0;
        } else {
            state.page_changes_dropped += 1;
        }
        if let Some(page_changed) = state.page_changed.as_mut() {
            page_changed(PageChange {
                event: win_event,
                object: id_child,
                notify,
            });
        }
    });
}

/// The window class of a Chromium page ("Chrome Legacy Window").
const CHROME_PAGE_WINDOW: &str = "Chrome_RenderWidgetHostHWND";

/// IA2 text inserted into and removed from an object's text (`AccessibleEventId.idl`).
pub const IA2_EVENT_TEXT_INSERTED: u32 = 0x011E;
pub const IA2_EVENT_TEXT_REMOVED: u32 = 0x011F;

/// An object shown, hidden or reordered in the page in front (`EVENT_OBJECT_SHOW`,
/// `_HIDE`, `_REORDER`), or whose text changed (`IA2_EVENT_TEXT_INSERTED`, `_REMOVED`),
/// by its (negative) IA2 uniqueID. `notify`: the engine is to be
/// told now (at most every [`PAGE_CHANGE_INTERVAL`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageChange {
    pub event: u32,
    pub object: i32,
    pub notify: bool,
}

/// UIA's focus-changed event id.
const UIA_FOCUS_CHANGED: u32 = 20005;
/// An event read taking longer is logged with its stages.
const SLOW_EVENT: std::time::Duration = std::time::Duration::from_millis(250);
/// Posted to the tracer thread by [`UiaFocusHandler`].
const WM_UIA_FOCUS: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;

/// A UIA element handed from UIA's event thread to the tracer thread.
struct MtaElement(IUIAutomationElement);
// SAFETY: both threads are in the multithreaded apartment, where a COM object may be
// called from any thread.
unsafe impl Send for MtaElement {}

/// The elements of UIA focus changes not yet recorded, oldest first, for the tracer
/// thread: on Windows 11 the task switcher's items are named only by the event, while
/// `GetFocusedElement` answers with the switcher's input pane (seen live on the dev box,
/// 2026-10-15). Each is recorded: keeping only the latest lost the first item to the
/// switcher window's own focus right after it.
static UIA_FOCUSED: std::sync::Mutex<std::collections::VecDeque<MtaElement>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());
/// Focus changes kept for the tracer thread at most (Tab held down repeats fast).
const MAX_UIA_FOCUSED: usize = 8;

/// Wakes the tracer thread on UIA focus changes in [`UIA_FOCUS_WINDOWS`]. UIA calls it on
/// its own threads, and it also reports every WinEvent focus (UIA bridges MSAA), so it
/// only checks the foreground window's class, a call that crosses no process.
#[implement(IUIAutomationFocusChangedEventHandler)]
struct UiaFocusHandler {
    thread: u32,
}

impl IUIAutomationFocusChangedEventHandler_Impl for UiaFocusHandler_Impl {
    fn HandleFocusChangedEvent(
        &self,
        sender: windows::core::Ref<IUIAutomationElement>,
    ) -> windows::core::Result<()> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            GetClassNameW, GetForegroundWindow, PostThreadMessageW,
        };
        let mut class = [0u16; 64];
        // SAFETY: plain Win32 queries with a valid buffer; the post targets a thread
        // that pumps messages until the handler is removed.
        unsafe {
            let length = GetClassNameW(GetForegroundWindow(), &mut class);
            let class = String::from_utf16_lossy(class.get(..length as usize).unwrap_or(&[]));
            if UIA_FOCUS_WINDOWS.contains(&class.as_str()) {
                if let (Some(element), Ok(mut queue)) = (sender.as_ref(), UIA_FOCUSED.lock()) {
                    if queue.len() >= MAX_UIA_FOCUSED {
                        queue.pop_front();
                    }
                    queue.push_back(MtaElement(element.clone()));
                }
                let _ = PostThreadMessageW(self.thread, WM_UIA_FOCUS, WPARAM(0), LPARAM(0));
            }
        }
        Ok(())
    }
}

/// Records the focus as it is now, as a focus event on the foreground thread's focused
/// window (the foreground window when it has none).
fn record_current_focus() {
    const OBJID_CLIENT: i32 = -4;
    const EVENT_OBJECT_FOCUS: u32 = 0x8005;
    let focused = keyboard_focus_window();
    if !focused.is_invalid() {
        record("focus", EVENT_OBJECT_FOCUS, focused, OBJID_CLIENT, 0, None);
    }
}

/// The window with the keyboard focus in the foreground thread (the foreground window
/// when none has it). A focus event can arrive on another window: the dialog box whose
/// field has the focus.
fn keyboard_focus_window() -> HWND {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
    };
    // SAFETY: plain Win32 queries; the info struct is sized as the call requires.
    unsafe {
        let foreground = GetForegroundWindow();
        let thread = GetWindowThreadProcessId(foreground, None);
        let mut info = GUITHREADINFO {
            cbSize: u32::try_from(std::mem::size_of::<GUITHREADINFO>()).unwrap_or(0),
            ..Default::default()
        };
        match GetGUIThreadInfo(thread, &mut info) {
            Ok(()) if !info.hwndFocus.is_invalid() => info.hwndFocus,
            _ => foreground,
        }
    }
}

/// The window is an up-down control's buddy: an `msctls_updown32` beside it names it as the
/// window it steps (`UDM_GETBUDDY`). JAWS says such an edit as an "edit spin box" [obs,
/// 546/s8].
fn is_spin_buddy(hwnd: HWND) -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowExW, GetParent, SendMessageTimeoutW, SMTO_ABORTIFHUNG,
    };
    const UDM_GETBUDDY: u32 = 0x0400 + 106;
    // A dialog has a few up-down controls at most; the count bounds a window list that
    // changes as it is walked.
    const MAX_UP_DOWNS: usize = 32;
    // SAFETY: plain Win32 queries on window handles that may be gone (they then fail).
    let Ok(parent) = (unsafe { GetParent(hwnd) }) else {
        return false;
    };
    let mut after: Option<HWND> = None;
    for _ in 0..MAX_UP_DOWNS {
        // SAFETY: as above, with a static class name.
        let Ok(up_down) =
            (unsafe { FindWindowExW(Some(parent), after, w!("msctls_updown32"), None) })
        else {
            return false;
        };
        let mut buddy = 0usize;
        // SAFETY: a read-only query with a timeout, to a window that may be gone or hung.
        let sent = unsafe {
            SendMessageTimeoutW(
                up_down,
                UDM_GETBUDDY,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                100,
                Some(&mut buddy),
            )
        };
        if sent.0 != 0 && buddy == hwnd.0 as usize {
            return true;
        }
        after = Some(up_down);
    }
    false
}

/// Records the focus UIA reported last, on the tracer thread.
fn record_uia_focus() {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let elements: Vec<MtaElement> = UIA_FOCUSED
        .lock()
        .map(|mut queue| queue.drain(..).collect())
        .unwrap_or_default();
    // SAFETY: plain Win32 call with no preconditions.
    let foreground = unsafe { GetForegroundWindow() };
    for element in &elements {
        record(
            "uia_focus",
            UIA_FOCUS_CHANGED,
            foreground,
            0,
            0,
            Some(&element.0),
        );
    }
}

/// UIA's element-selected event id.
const UIA_ELEMENT_SELECTED: u32 = 20012;
/// Posted to the tracer thread by [`UiaSelectedHandler`].
const WM_UIA_SELECTED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 2;
/// Selected elements not yet recorded, oldest first.
static UIA_SELECTED: std::sync::Mutex<std::collections::VecDeque<MtaElement>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());

/// Wakes the tracer thread on UIA element selections in [`UIA_SELECTION_PROCESSES`] in
/// front. Selections everywhere reach it (it is registered on the desktop), so it only
/// looks up the foreground window's process before anything else.
#[implement(IUIAutomationEventHandler)]
struct UiaSelectedHandler {
    thread: u32,
}

impl IUIAutomationEventHandler_Impl for UiaSelectedHandler_Impl {
    fn HandleAutomationEvent(
        &self,
        sender: windows::core::Ref<IUIAutomationElement>,
        _event: windows::Win32::UI::Accessibility::UIA_EVENT_ID,
    ) -> windows::core::Result<()> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, PostThreadMessageW};
        // SAFETY: plain Win32 call with no preconditions.
        let process = process_name(unsafe { GetForegroundWindow() }).unwrap_or_default();
        if !UIA_SELECTION_PROCESSES
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&process))
        {
            return Ok(());
        }
        // The application in front's own selections only: the taskbar's news feed turns its
        // headlines with selections of its own, and with Settings in front its MSN page's
        // address reached a trace (3155, old-pc, 2026-11-02).
        // SAFETY: a property read on the element UIA passed in.
        let pid = sender
            .as_ref()
            .and_then(|e| unsafe { e.CurrentProcessId() }.ok())
            .and_then(|p| u32::try_from(p).ok())
            .unwrap_or(0);
        if pid == 0 || !foreground_processes().contains(&pid) {
            return Ok(());
        }
        if let (Some(element), Ok(mut queue)) = (sender.as_ref(), UIA_SELECTED.lock()) {
            if queue.len() >= MAX_UIA_FOCUSED {
                queue.pop_front();
            }
            queue.push_back(MtaElement(element.clone()));
        }
        // SAFETY: the post targets a thread that pumps messages until the handler is
        // removed.
        let _ = unsafe { PostThreadMessageW(self.thread, WM_UIA_SELECTED, WPARAM(0), LPARAM(0)) };
        Ok(())
    }
}

/// Records the selections UIA reported, on the tracer thread.
fn record_uia_selected() {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let elements: Vec<MtaElement> = UIA_SELECTED
        .lock()
        .map(|mut queue| queue.drain(..).collect())
        .unwrap_or_default();
    // SAFETY: plain Win32 call with no preconditions.
    let foreground = unsafe { GetForegroundWindow() };
    for element in &elements {
        record(
            "uia_selected",
            UIA_ELEMENT_SELECTED,
            foreground,
            0,
            0,
            Some(&element.0),
        );
    }
}

const WM_UIA_NOTIFICATION: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 3;
/// UIA notifications not yet recorded, oldest first.
static UIA_NOTIFICATIONS: std::sync::Mutex<
    std::collections::VecDeque<okur_windows_model::event_trace::TraceNotification>,
> = std::sync::Mutex::new(std::collections::VecDeque::new());

/// The processes of the window in front: its own, and for a UWP app in a frame
/// (ApplicationFrameHost: Windows Settings) the app's, which owns its core window.
/// Windows' shell, whose UIA notifications are its toasts ("New notification from Windows
/// PowerShell, ...", which JAWS says whatever is in front [obs, 533/s1]): the notification
/// host, or Explorer.
fn is_notification_host(pid: u32) -> bool {
    process_name_of(pid).is_some_and(|name| {
        name.eq_ignore_ascii_case("ShellExperienceHost.exe")
            || name.eq_ignore_ascii_case("explorer.exe")
    })
}

fn foreground_processes() -> [u32; 2] {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    // SAFETY: plain Win32 call with no preconditions.
    processes_of_front(unsafe { GetForegroundWindow() })
}

/// [`foreground_processes`] of the window `foreground`.
fn processes_of_front(foreground: HWND) -> [u32; 2] {
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowExW;
    let mut frame = 0u32;
    let mut app = 0u32;
    // SAFETY: plain Win32 queries with valid out-pointers and a static class name.
    unsafe {
        GetWindowThreadProcessId(foreground, Some(&mut frame));
        if let Ok(core) = FindWindowExW(
            Some(foreground),
            None,
            w!("Windows.UI.Core.CoreWindow"),
            None,
        ) {
            GetWindowThreadProcessId(core, Some(&mut app));
        }
    }
    [frame, app]
}

/// Wakes the tracer thread on UIA notifications from the application in front: what an
/// application says without a focus or a live region (Settings' search box: "Suggestions
/// available", which JAWS reads; dev box, 2026-10-16).
#[implement(IUIAutomationNotificationEventHandler)]
struct UiaNotificationHandler {
    thread: u32,
}

impl IUIAutomationNotificationEventHandler_Impl for UiaNotificationHandler_Impl {
    fn HandleNotificationEvent(
        &self,
        sender: windows::core::Ref<IUIAutomationElement>,
        _kind: NotificationKind,
        processing: NotificationProcessing,
        display: &windows::core::BSTR,
        _activity: &windows::core::BSTR,
    ) -> windows::core::Result<()> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;
        let text = display.to_string();
        if text.trim().is_empty() {
            return Ok(());
        }
        // SAFETY: a property read on the element UIA passed in.
        let pid = sender
            .as_ref()
            .and_then(|e| unsafe { e.CurrentProcessId() }.ok())
            .and_then(|p| u32::try_from(p).ok())
            .unwrap_or(0);
        if pid == 0 || !(foreground_processes().contains(&pid) || is_notification_host(pid)) {
            // What an application behind says stays out of the log: only its length
            // (performance review 2026-10-25, 20).
            let chars = text.chars().count();
            tracing::debug!(
                pid,
                chars,
                "UIA notification from a process behind, not taken"
            );
            return Ok(());
        }
        tracing::debug!(%text, "UIA notification");
        if let Ok(mut queue) = UIA_NOTIFICATIONS.lock() {
            if queue.len() >= MAX_UIA_FOCUSED {
                queue.pop_front();
            }
            queue.push_back(okur_windows_model::event_trace::TraceNotification {
                text,
                processing: processing.0,
                process: process_name_of(pid),
            });
        }
        // SAFETY: the post targets a thread that pumps messages until the handler is
        // removed.
        let _ =
            unsafe { PostThreadMessageW(self.thread, WM_UIA_NOTIFICATION, WPARAM(0), LPARAM(0)) };
        Ok(())
    }
}

/// Records the notifications UIA reported, on the tracer thread.
fn record_uia_notifications() {
    let notifications: Vec<okur_windows_model::event_trace::TraceNotification> = UIA_NOTIFICATIONS
        .lock()
        .map(|mut queue| queue.drain(..).collect())
        .unwrap_or_default();
    for notification in notifications {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let Some(state) = state.as_mut() else { return };
            let event = TraceEvent {
                t_ms: now_ms(),
                event: "uia_notification".to_owned(),
                win_event: UIA_NOTIFICATION_EVENT,
                hwnd: 0,
                id_object: 0,
                id_child: 0,
                window_class: None,
                window_style: None,
                spin_box: false,
                focus_window_class: None,
                menu_bar: Vec::new(),
                status_bar: Vec::new(),
                window_title: None,
                process: None,
                process_product: None,
                process_version: None,
                object: None,
                focus_chain: Vec::new(),
                focus_siblings: Vec::new(),
                focus_subtree: None,
                list_position: None,
                column_headers: Vec::new(),
                text_context: None,
                region: None,
                page: None,
                top_window_title: None,
                top_window_class: None,
                top_window_owner_enabled: None,
                virtual_desktop: None,
                dialog_lines: Vec::new(),
                dialog_text: Vec::new(),
                dialog_page: None,
                group_box: None,
                default_button: None,
                dialog_tree: None,
                notification: Some(notification),
                mouse: None,
                point: None,
                msaa_tree: None,
                msaa_siblings: Vec::new(),
                msaa_ancestors: Vec::new(),
                msaa_description: None,
                netui_control: None,
                modal_dialog: None,
            };
            (state.sink)(event);
            state.count += 1;
        });
    }
}

/// How long the mouse stays where it went before a trace reads what it points to.
const MOUSE_REST: std::time::Duration = std::time::Duration::from_millis(100);

/// Records where the mouse pointer went, when it moved since last looked at: the JAWS
/// cursor follows it, and JAWS's routing moves it. Windows raises no event for that. A
/// trace (not the live screen reader, which asks when it wants) also records what the
/// mouse points to once it rests there, a `mouse_point`, for mouse echo in replay.
fn record_mouse_move() {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut point = POINT::default();
    // SAFETY: valid out-pointer.
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        return;
    }
    let at = [point.x, point.y];
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        if state.last_mouse == Some(at) {
            let rested = state
                .mouse_moved_at
                .is_some_and(|moved| moved.elapsed() >= MOUSE_REST);
            if !state.live && rested {
                state.mouse_moved_at = None;
                let read = state
                    .uia
                    .as_ref()
                    .and_then(|uia| crate::mouse_point::read(uia, at[0], at[1]));
                if let Some((chain, texts)) = read {
                    let mut event = mouse_event(None);
                    event.event = "mouse_point".to_owned();
                    event.focus_chain = chain;
                    event.point = Some(TracePoint { at, texts });
                    (state.sink)(event);
                    state.count += 1;
                }
            }
            return;
        }
        state.last_mouse = Some(at);
        state.mouse_moved_at = Some(std::time::Instant::now());
        (state.sink)(mouse_event(Some(at)));
        state.count += 1;
    });
}

/// A `mouse_move` to `at`, with nothing else read.
fn mouse_event(at: Option<[i32; 2]>) -> TraceEvent {
    TraceEvent {
        t_ms: now_ms(),
        event: "mouse_move".to_owned(),
        win_event: 0,
        hwnd: 0,
        id_object: 0,
        id_child: 0,
        window_class: None,
        window_style: None,
        spin_box: false,
        focus_window_class: None,
        menu_bar: Vec::new(),
        status_bar: Vec::new(),
        window_title: None,
        process: None,
        process_product: None,
        process_version: None,
        object: None,
        focus_chain: Vec::new(),
        focus_siblings: Vec::new(),
        focus_subtree: None,
        list_position: None,
        column_headers: Vec::new(),
        text_context: None,
        region: None,
        page: None,
        top_window_title: None,
        top_window_class: None,
        top_window_owner_enabled: None,
        virtual_desktop: None,
        dialog_lines: Vec::new(),
        dialog_text: Vec::new(),
        dialog_page: None,
        group_box: None,
        default_button: None,
        dialog_tree: None,
        notification: None,
        mouse: at,
        point: None,
        msaa_tree: None,
        msaa_siblings: Vec::new(),
        msaa_ancestors: Vec::new(),
        msaa_description: None,
        netui_control: None,
        modal_dialog: None,
    }
}

/// UIA's notification event id.
const UIA_NOTIFICATION_EVENT: u32 = 20035;

const WM_UIA_LIVE_REGION: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 4;
/// Live regions changed and not yet recorded, oldest first.
static UIA_LIVE_REGIONS: std::sync::Mutex<std::collections::VecDeque<MtaElement>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());
/// UIA's live region changed event id.
const UIA_LIVE_REGION_EVENT: u32 = 20024;

/// Wakes the tracer thread when a live region changes in the application in front: a
/// native application's (Windows Settings' cards as a page is shown again). A browser's
/// page regions are read through IA2 from their WinEvent instead.
#[implement(IUIAutomationEventHandler)]
struct UiaLiveRegionHandler {
    thread: u32,
}

impl IUIAutomationEventHandler_Impl for UiaLiveRegionHandler_Impl {
    fn HandleAutomationEvent(
        &self,
        sender: windows::core::Ref<IUIAutomationElement>,
        _event: windows::Win32::UI::Accessibility::UIA_EVENT_ID,
    ) -> windows::core::Result<()> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;
        let Some(element) = sender.as_ref() else {
            return Ok(());
        };
        // SAFETY: property reads on the element UIA passed in.
        let pid = unsafe { element.CurrentProcessId() }
            .ok()
            .and_then(|p| u32::try_from(p).ok())
            .unwrap_or(0);
        if pid == 0 || !foreground_processes().contains(&pid) {
            return Ok(());
        }
        // SAFETY: as above.
        let framework = unsafe { element.CurrentFrameworkId() }
            .map(|f| f.to_string())
            .unwrap_or_default();
        if matches!(framework.as_str(), "Chrome" | "Gecko") {
            return Ok(());
        }
        if let Ok(mut queue) = UIA_LIVE_REGIONS.lock() {
            if queue.len() >= MAX_UIA_FOCUSED {
                queue.pop_front();
            }
            queue.push_back(MtaElement(element.clone()));
        }
        // SAFETY: the post targets a thread that pumps messages until the handler is
        // removed.
        let _ =
            unsafe { PostThreadMessageW(self.thread, WM_UIA_LIVE_REGION, WPARAM(0), LPARAM(0)) };
        Ok(())
    }
}

/// Records the live regions UIA reported, each with what it holds, on the tracer thread.
fn record_uia_live_regions() {
    let elements: Vec<MtaElement> = UIA_LIVE_REGIONS
        .lock()
        .map(|mut queue| queue.drain(..).collect())
        .unwrap_or_default();
    for element in elements {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let Some(state) = state.as_mut() else { return };
            let Some(region) = state
                .uia
                .as_ref()
                .and_then(|uia| crate::uia_capture::element_subtree(uia, &element.0, 2))
            else {
                return;
            };
            let event = TraceEvent {
                t_ms: now_ms(),
                event: "uia_live_region".to_owned(),
                win_event: UIA_LIVE_REGION_EVENT,
                hwnd: 0,
                id_object: 0,
                id_child: 0,
                window_class: None,
                window_style: None,
                spin_box: false,
                focus_window_class: None,
                menu_bar: Vec::new(),
                status_bar: Vec::new(),
                window_title: None,
                process: None,
                process_product: None,
                process_version: None,
                object: None,
                focus_chain: Vec::new(),
                focus_siblings: Vec::new(),
                focus_subtree: Some(region),
                list_position: None,
                column_headers: Vec::new(),
                text_context: None,
                region: None,
                page: None,
                top_window_title: None,
                top_window_class: None,
                top_window_owner_enabled: None,
                virtual_desktop: None,
                dialog_lines: Vec::new(),
                dialog_text: Vec::new(),
                dialog_page: None,
                group_box: None,
                default_button: None,
                dialog_tree: None,
                notification: None,
                mouse: None,
                point: None,
                msaa_tree: None,
                msaa_siblings: Vec::new(),
                msaa_ancestors: Vec::new(),
                msaa_description: None,
                netui_control: None,
                modal_dialog: None,
            };
            (state.sink)(event);
            state.count += 1;
        });
    }
}

const WM_UIA_WINDOW_OPENED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 5;
/// The shell's windows opened and not yet recorded, oldest first, each with its process.
static UIA_WINDOWS_OPENED: std::sync::Mutex<std::collections::VecDeque<(MtaElement, u32)>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());
/// UIA's window opened event id.
const UIA_WINDOW_OPENED_EVENT: u32 = 20016;

/// Wakes the tracer thread when one of the shell's windows opens: a toast's banner, a
/// Windows.UI.Core.CoreWindow of ShellExperienceHost on Windows 10, which raises only UIA's
/// WindowOpened as it shows (`docs/references/notifications-research.md`). The 533 traces
/// had no event of the toast JAWS said [obs, 533/s1].
#[implement(IUIAutomationEventHandler)]
struct UiaWindowOpenedHandler {
    thread: u32,
}

impl IUIAutomationEventHandler_Impl for UiaWindowOpenedHandler_Impl {
    fn HandleAutomationEvent(
        &self,
        sender: windows::core::Ref<IUIAutomationElement>,
        _event: windows::Win32::UI::Accessibility::UIA_EVENT_ID,
    ) -> windows::core::Result<()> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;
        let Some(element) = sender.as_ref() else {
            return Ok(());
        };
        // SAFETY: a property read on the element UIA passed in.
        let pid = unsafe { element.CurrentProcessId() }
            .ok()
            .and_then(|p| u32::try_from(p).ok())
            .unwrap_or(0);
        if pid == 0 || !is_notification_host(pid) {
            return Ok(());
        }
        if let Ok(mut queue) = UIA_WINDOWS_OPENED.lock() {
            if queue.len() >= MAX_UIA_FOCUSED {
                queue.pop_front();
            }
            queue.push_back((MtaElement(element.clone()), pid));
        }
        // SAFETY: the post targets a thread that pumps messages until the handler is
        // removed.
        let _ =
            unsafe { PostThreadMessageW(self.thread, WM_UIA_WINDOW_OPENED, WPARAM(0), LPARAM(0)) };
        Ok(())
    }
}

/// Records the shell's windows UIA reported opened, each with what it holds (a toast's
/// title and text), on the tracer thread. Research for now: replay takes none of them.
fn record_uia_windows_opened() {
    let opened: Vec<(MtaElement, u32)> = UIA_WINDOWS_OPENED
        .lock()
        .map(|mut queue| queue.drain(..).collect())
        .unwrap_or_default();
    for (element, pid) in opened {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let Some(state) = state.as_mut() else { return };
            let Some(window) = state
                .uia
                .as_ref()
                .and_then(|uia| crate::uia_capture::element_subtree(uia, &element.0, 4))
            else {
                return;
            };
            let event = TraceEvent {
                t_ms: now_ms(),
                event: "uia_window_opened".to_owned(),
                win_event: UIA_WINDOW_OPENED_EVENT,
                hwnd: 0,
                id_object: 0,
                id_child: 0,
                window_class: None,
                window_style: None,
                spin_box: false,
                focus_window_class: None,
                menu_bar: Vec::new(),
                status_bar: Vec::new(),
                window_title: None,
                process: process_name_of(pid),
                process_product: None,
                process_version: None,
                object: None,
                focus_chain: Vec::new(),
                focus_siblings: Vec::new(),
                focus_subtree: Some(window),
                list_position: None,
                column_headers: Vec::new(),
                text_context: None,
                region: None,
                page: None,
                top_window_title: None,
                top_window_class: None,
                top_window_owner_enabled: None,
                virtual_desktop: None,
                dialog_lines: Vec::new(),
                dialog_text: Vec::new(),
                dialog_page: None,
                group_box: None,
                default_button: None,
                dialog_tree: None,
                notification: None,
                mouse: None,
                point: None,
                msaa_tree: None,
                msaa_siblings: Vec::new(),
                msaa_ancestors: Vec::new(),
                msaa_description: None,
                netui_control: None,
                modal_dialog: None,
            };
            (state.sink)(event);
            state.count += 1;
        });
    }
}

/// Records one event through the sink, with what the engine needs read now. `element`
/// is the focused element when the event names it (UIA); otherwise UIA's focus is read.
fn record(
    name: &'static str,
    win_event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    element: Option<&IUIAutomationElement>,
) {
    record_at(
        now_ms(),
        name,
        win_event,
        hwnd,
        id_object,
        id_child,
        element,
    );
}

/// [`record`] for an event raised at `t_ms`.
fn record_at(
    t_ms: u128,
    name: &'static str,
    win_event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    element: Option<&IUIAutomationElement>,
) {
    // Before anything is read: a read that never returns leaves this as the log's last
    // line, naming the event (613's tracer stopped a second after the page loaded).
    tracing::debug!(
        event = name,
        hwnd = hwnd.0 as isize,
        id_object,
        id_child,
        "reading event"
    );
    let (title, class) = if hwnd.is_invalid() {
        (String::new(), String::new())
    } else {
        window_text(hwnd)
    };
    // A change in the dialog last read makes that read stale (performance review P6).
    if !keeps_dialog_read(name, name == "state_change" && is_push_button(hwnd, &class)) {
        dialog_changed(name, hwnd);
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else { return };
        // A change to the focused control (check box toggled, tree item expanded, combo
        // box value) is spoken with the same context as a focus landing on it.
        let focused = matches!(name, "focus" | "uia_focus");
        let changes_focus_context = matches!(name, "state_change" | "value_change" | "selection");
        // Live, every cross-process read on this thread delays the events behind it: drop
        // what the engine never uses (name and description changes, alerts) and changes
        // in other applications (the taskbar, the tray clock) before reading anything.
        // The window in front renaming itself is kept: its title is read anew.
        let foreground_renamed = name == "name_change"
            && id_object == okur_windows_model::event_trace::OBJID_WINDOW
            && {
                use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                // SAFETY: plain Win32 call with no preconditions.
                let front = unsafe { GetForegroundWindow() };
                front == hwnd
            };
        // A focus in an application not in front is not the user's: Windows Settings,
        // running behind Chrome, reported its search box focused while the page kept the
        // focus, and JAWS said nothing of it (221, old-pc, 2026-10-18).
        if name == "focus" && in_background_process(hwnd) {
            return;
        }
        // Nor are its changes recorded: nothing replays them, and the taskbar's news and
        // weather widget put the reference machine's location into traces (its MSN
        // links, old-pc, 2026-10-18).
        let change = matches!(
            name,
            "name_change"
                | "description_change"
                | "value_change"
                | "state_change"
                | "selection"
                | "selection_add"
                | "selection_remove"
                | "selection_within"
                | "text_selection_change"
        );
        if change && in_background_process(hwnd) {
            return;
        }
        // Office's NetUI (its ribbon, its status bar) changes the state of dozens of its
        // objects at each tab or view: 423 state changes read in one trial of Word's
        // ribbon took 35 s and put the tracer 15 s behind the keys (1043, old-pc,
        // 2026-10-25). There only a change to the object last focused is read.
        if name == "focus" {
            state.focused_object = Some((hwnd.0 as isize, id_object));
        }
        if name == "state_change"
            && class == NETUI_CLASS
            && state.focused_object != Some((hwnd.0 as isize, id_object))
        {
            tracing::debug!(event = name, "NetUI state change not read");
            return;
        }
        // The taskbar's news and weather widget is never recorded, even when Explorer is
        // in front: it put the weather where the reference machine is into a trace (its
        // title "13°C ...", 504, old-pc, 2026-10-19).
        if class == okur_windows_model::event_trace::NEWS_AND_WEATHER_CLASS {
            return;
        }
        // A file dialog's navigation pane names the account's pinned and recent folders as
        // it fills its tree (Quick access: 524, round 38); only the focus moving into it is
        // recorded.
        if name != "focus" && class.eq_ignore_ascii_case("SysTreeView32") && in_namespace_tree(hwnd)
        {
            return;
        }
        if state.live
            && ((!LIVE_EVENTS.contains(&name) && !foreground_renamed)
                || (changes_focus_context && !in_foreground_process(hwnd)))
        {
            return;
        }
        // A hung application's window is not read live: each cross-process call into it
        // would wait for it, and every focus and event behind it with it (performance
        // review, 2026-10-20, P4). Its events are dropped until it answers again.
        if state.live && in_hung_window(hwnd) {
            tracing::debug!(event = name, "event of a hung window not read");
            return;
        }
        // A state or selection change of another control than the focused one is not read
        // further live: the converter would drop it, and deciding needs only the event's
        // MSAA object. The focus, its ancestors, siblings and text, read for a change, were
        // read in vain for 73% of the state changes recorded (performance review P5,
        // step 2).
        // The object read here is the one recorded: read twice, it cost six more
        // cross-process calls per change (performance review 2026-10-23, P10).
        let mut object_read: Option<Option<TraceObject>> = None;
        if state.live && matches!(name, "state_change" | "selection" | "value_change") {
            let object = msaa_object(hwnd, id_object, id_child);
            let in_focus_window =
                name == "value_change" && object.is_none() && keyboard_focus_window() == hwnd;
            if !okur_windows_model::event_trace::live_change_kept(
                name,
                (hwnd.0 as isize, id_object, id_child),
                object.as_ref(),
                &class,
                in_focus_window,
            ) {
                return;
            }
            object_read = Some(object);
        }
        // A live region outside a Chromium page is read from UIA's own event
        // ([`UiaLiveRegionHandler`]); this one has nothing to read, and its MSAA object took
        // 970 ms to answer as Windows Settings opened (dev box, 2026-10-17).
        if name == "live_region_change" && class != "Chrome_RenderWidgetHostHWND" {
            return;
        }
        // An alert outside a page (a message box's) is read from its dialog's focus.
        if state.live && name == "alert" && class != "Chrome_RenderWidgetHostHWND" {
            return;
        }
        // Each read's end, in milliseconds since the event arrived, for the debug log when
        // an event holds the thread up (a UIA call waiting out its timeout).
        let clock = std::time::Instant::now();
        let mut stages: Vec<(&str, u128)> = Vec::new();
        // The focused element is asked for once: chain and context are read around the
        // same element. A second GetFocusedElement right after a window switch could
        // block for seconds while UIA's focus settled (7 s into Windows 11 Notepad, dev
        // box), and could answer with another element than the chain's.
        let uia_named = matches!(name, "uia_focus" | "uia_selected");
        // The dialog in front shown after its focus was read: read again, as a focus is,
        // for its controls [obs, 665/s3].
        let shown = name == "dialog_shown";
        let reads_focus = focused
            || shown
            || name == "foreground"
            || changes_focus_context
            || name == "uia_selected";
        let asked = match (element, state.uia.as_ref()) {
            (None, Some(uia)) if reads_focus => focused_element(uia),
            _ => None,
        };
        let element = element.or(asked.as_ref());
        // The same element named by focus events right after one another is read once:
        // Windows Terminal reports one focus five times, once from each of its windows
        // (dev box, 2026-10-15). Any other focus between them makes the next one new. An
        // event about a child object (a WinForms menu item, while UIA still names the
        // control behind the menu [obs, 302]) is always read.
        let window = hwnd.0 as isize;
        if focused && !uia_named {
            if let (Some(uia), Some(element)) = (state.uia.as_ref(), element) {
                // Another MSAA object of the window is another focus: UIA names only the
                // window of Word's ribbon, whose controls are objects of their own, and took
                // Grow Font's focus right after the ribbon's for a repeat [obs, 1045/s9].
                // UIA's own focus events, passed on as WinEvents with ids of their own,
                // name no MSAA object: the same element again is still a repeat.
                let last = state
                    .last_object_focus
                    .as_ref()
                    .filter(|last| {
                        id_child == CHILDID_SELF && last.names(uia, element, REPEATED_FOCUS)
                    })
                    .map(|last| last.object);
                let repeated = last.is_some_and(|object| {
                    object == id_object || {
                        let read = msaa_object(hwnd, id_object, id_child);
                        let none = read.is_none();
                        object_read = Some(read);
                        none
                    }
                });
                let seen = SeenFocus {
                    element: element.clone(),
                    at: std::time::Instant::now(),
                    window,
                    // A repeat keeps the object it repeats: Word names Cut twice around
                    // UIA's own focus event [obs, 1045/s3].
                    object: last.filter(|_| repeated).unwrap_or(id_object),
                };
                state.last_object_focus = (id_child == CHILDID_SELF).then(|| seen.clone());
                state.last_focus_read = Some(seen);
                if repeated {
                    tracing::debug!(event = name, "repeated focus not read");
                    return;
                }
            }
        }
        // A list's selection and value change for the item just focused (a list view
        // reports both after each arrow key) say nothing the focus did not: 70 ms each,
        // per key (dev box, 2026-10-15). A state change (a check box toggled) is always
        // read.
        if matches!(name, "selection" | "value_change") {
            if let (Some(uia), Some(element)) = (state.uia.as_ref(), element) {
                let echo = state.last_focus_read.as_ref().is_some_and(|last| {
                    last.window == window && last.names(uia, element, FOCUS_ECHO)
                });
                if echo {
                    tracing::debug!(event = name, "focus echo not read");
                    return;
                }
            }
        }
        stages.push(("element", clock.elapsed().as_millis()));
        // The focused element's properties are fetched once, for the chain and the context.
        let element = match (state.uia.as_ref(), element) {
            (Some(uia), Some(element)) if reads_focus => with_properties(uia, element),
            _ => None,
        };
        stages.push(("properties", clock.elapsed().as_millis()));
        let element = element.as_ref();
        let mut focus = match (state.uia.as_ref(), element) {
            (Some(uia), Some(element)) if reads_focus => focus_chain_from(uia, element),
            _ => Vec::new(),
        };
        // Where the focus is on the screen: the PC cursor's place (Alt+Delete). A control
        // that is a window of its own (a Win32 or WinForms control) gives its window's
        // rectangle, in real pixels for this DPI-aware process whatever the window's own
        // awareness: UI Automation gave a WinForms button's unscaled at 125% (516/s8).
        if let (Some(element), Some(focused)) = (element.filter(|_| focused), focus.last_mut()) {
            // SAFETY: property reads on a valid element, then a Win32 read into a RECT.
            let window = unsafe { element.CurrentNativeWindowHandle() }
                .ok()
                .filter(|h| !h.is_invalid())
                .and_then(|h| {
                    let mut rect = windows::Win32::Foundation::RECT::default();
                    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowRect(h, &mut rect) }
                        .ok()
                        .map(|()| rect)
                });
            let from_window = window.is_some();
            let rect = window.or_else(|| unsafe { element.CurrentBoundingRectangle() }.ok());
            if let Some(r) = rect {
                focused.bounds = Some([r.left, r.top, r.right, r.bottom]);
                // SAFETY: reads of this thread's awareness; no preconditions.
                let awareness = unsafe {
                    windows::Win32::UI::HiDpi::GetAwarenessFromDpiAwarenessContext(
                        windows::Win32::UI::HiDpi::GetThreadDpiAwarenessContext(),
                    )
                };
                tracing::debug!(
                    bounds = ?focused.bounds,
                    from_window,
                    awareness = awareness.0,
                    "focus bounds"
                );
            }
        }
        stages.push(("chain", clock.elapsed().as_millis()));
        let list_position = if focused || changes_focus_context {
            win32_list_position(hwnd, &class).or_else(|| {
                let focus_window = keyboard_focus_window();
                (focus_window != hwnd)
                    .then(|| win32_list_position(focus_window, &window_text(focus_window).1))
                    .flatten()
            })
        } else {
            None
        };
        stages.push(("position", clock.elapsed().as_millis()));
        // Siblings give a position when nothing else does, and a dialog box's text and
        // buttons. Reading them in a long list took 2 s per item (dev box, 2026-10-15),
        // where the list already gives the position.
        let needs_siblings =
            in_dialog_box(&focus) || (list_position.is_none() && !reports_position(&focus));
        let (focus_siblings, focus_subtree) = match (name, state.uia.as_ref()) {
            // A focus UIA names (the task switcher's items) is read by name alone: on the
            // switcher's first opening its tree answers slowly, and reading around the item
            // held the thread for seconds while the next items went by (dev box).
            (_, Some(_)) if uia_named => (Vec::new(), None),
            (_, Some(uia)) if focused => match element {
                Some(element) => focus_context_of(uia, element, needs_siblings),
                None => (Vec::new(), None),
            },
            (_, Some(uia)) if changes_focus_context => {
                (Vec::new(), element.and_then(|e| focus_subtree_of(uia, e)))
            }
            _ => (Vec::new(), None),
        };
        let column_headers = match (state.uia.as_ref(), element) {
            (Some(uia), Some(element)) if focused && !uia_named => column_headers_of(uia, element),
            _ => Vec::new(),
        };
        stages.push(("context", clock.elapsed().as_millis()));
        // The window is the one in front now, when the focus chain was read: an event's
        // own window can have lost the foreground meanwhile (the task switcher's last
        // events arrive after the switch back, seen live on the dev box).
        let TopWindow {
            title: top_window_title,
            class: top_window_class,
            desktop: virtual_desktop,
            owner_enabled: top_window_owner_enabled,
        } = if focused || shown || name == "foreground" {
            use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
            // SAFETY: plain Win32 call with no preconditions.
            top_window(unsafe { GetForegroundWindow() })
        } else {
            TopWindow::default()
        };
        let (dialog, dialog_tree, dialog_page, group_box, default_button) = if (focused || shown)
            && top_window_class.as_deref() == Some("#32770")
        {
            use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
            // SAFETY: plain Win32 call with no preconditions.
            let window = unsafe { GetForegroundWindow() };
            // Say Window's controls: the page the focus is on, or the dialog.
            let root =
                crate::screen_lines::page_window(keyboard_focus_window(), window).unwrap_or(window);
            let changes = dialog_changes(window);
            let reused = state.dialog_read.as_ref().filter(|read| {
                read.dialog == window.0 as isize
                    && read.root == root.0 as isize
                    && read.changes == changes
            });
            let (layout, tree) = match reused {
                Some(read) => {
                    tracing::debug!("dialog read reused");
                    (read.layout.clone(), Some(read.tree.clone()))
                }
                None => {
                    let layout = crate::screen_lines::dialog_layout(window);
                    let tree = state.uia.as_ref().and_then(|uia| {
                        window_subtree(uia, root)
                            .map(okur_windows_model::event_trace::without_nested_tree_items)
                    });
                    // A dialog still hidden as its focus arrives (the Font dialog,
                    // 541) has no visible controls to lay out yet: read again.
                    // SAFETY: plain Win32 query on a window handle.
                    let shown =
                        unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(window) }
                            .as_bool();
                    state.dialog_read = tree.clone().filter(|_| shown).map(|tree| DialogRead {
                        dialog: window.0 as isize,
                        root: root.0 as isize,
                        changes,
                        layout: layout.clone(),
                        tree,
                    });
                    (layout, tree)
                }
            };
            (
                layout,
                tree,
                crate::screen_lines::page_of(keyboard_focus_window(), window),
                crate::screen_lines::group_box_of(keyboard_focus_window(), window),
                crate::screen_lines::default_button_of(window),
            )
        } else {
            (
                crate::screen_lines::DialogLayout::default(),
                None,
                None,
                None,
                None,
            )
        };
        stages.push(("window", clock.elapsed().as_millis()));
        // An alert in a page (role="alert") is read as a live region: Chromium tells of its
        // new content with an alert event alone, no live region change [obs, round-aria12,
        // 2346/s1].
        let page_alert = name == "alert" && class == "Chrome_RenderWidgetHostHWND";
        let (region, page) = if name == "live_region_change" || page_alert {
            // Live, the page after the change is captured by the listener's page worker.
            chromium_live_region(hwnd, &class, id_child, !state.live)
        } else {
            (None, None)
        };
        // Live, after an edit key the actions worker reads the field once it has settled.
        // Reading here too would race it with a half-updated state (a selection still
        // being extended), and whichever arrived first would be spoken.
        let caret_moved = matches!(name, "caret" | "text_selection_change");
        // The document's value changed: a form field's too, which the text does not show (a
        // check box toggled), so its form fields kept are read again; not for another
        // window's change (Word's ribbon boxes, which put the tracer a step behind, 1061).
        if name == "value_change" {
            crate::text_context::forget_form_fields_of(hwnd.0 as isize);
        }
        let text_context = if (focused || changes_focus_context || caret_moved)
            && !uia_named
            && !(state.live && caret_moved)
            && !(caret_moved && newer_caret_waiting(hwnd))
        {
            // A browser's own controls (its address bar) are read as an application's: the
            // page names a focus of its own, which is not the one typed in.
            let in_page = focus
                .iter()
                .any(|n| n.control_type == okur_windows_model::event_trace::UIA_DOCUMENT);
            let read = || {
                state.uia.as_ref().and_then(|uia| {
                    if in_page {
                        crate::text_context::read_focused(uia)
                    } else {
                        crate::text_context::read_focused_outside_page(uia)
                    }
                })
            };
            let first = read();
            // Chromium fires the caret event before its IA2 caret offset moves: a trace
            // reads the field again once it has settled (live, the actions worker polls
            // for the change after the key instead). Only a page's: elsewhere (Word) the
            // second read only doubled each caret event's cost.
            if !state.live && in_page && matches!(name, "caret" | "text_selection_change") {
                std::thread::sleep(std::time::Duration::from_millis(40));
                read().or(first)
            } else {
                first
            }
        } else {
            None
        };
        stages.push(("text", clock.elapsed().as_millis()));
        // A UIA focus has no MSAA object: the focused item is UIA's.
        let object = match object_read {
            Some(object) => object,
            None => (!uia_named)
                .then(|| msaa_object(hwnd, id_object, id_child))
                .flatten(),
        };
        // A list's rows changing as it draws them are not recorded: the reference machine's
        // own lists (its installed programs) stay out of the public corpus.
        if change
            && object
                .as_ref()
                .is_some_and(okur_windows_model::privacy::unvisited_list_item)
        {
            return;
        }
        // A NetUI window's focus, or an Office command bar's: its MSAA objects, which UI
        // Automation does not reach.
        let netui = match object.as_ref().and_then(|o| o.role) {
            Some(role)
                if focused
                    && (class == NETUI_WINDOW
                        || class == okur_windows_model::event_trace::COMMAND_BAR_CLASS) =>
            {
                netui_read(state.uia.as_ref(), hwnd, id_object, id_child, role)
            }
            _ => NetuiRead::default(),
        };
        let (msaa_tree, msaa_siblings) = (netui.tree, netui.siblings);
        stages.push(("msaa", clock.elapsed().as_millis()));
        if clock.elapsed() > SLOW_EVENT {
            tracing::debug!(event = name, %class, ?stages, "slow event read");
        }
        // Every read's time, for `okur-lab latency`.
        tracing::debug!(
            event = name,
            us = clock.elapsed().as_micros(),
            ?stages,
            "event read"
        );
        // The window with the keyboard focus, which a UIA focus event does not name (it
        // reports the top-level window: 316/s3 after Alt+Tab).
        let focus_window = focused.then(keyboard_focus_window);
        let window_style = focus_window.map(|w| {
            use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongW, GWL_STYLE};
            // SAFETY: plain Win32 call; a window that is gone gives 0.
            unsafe { GetWindowLongW(w, GWL_STYLE) as u32 }
        });
        let focus_window_class = focus_window
            .map(|w| window_text(w).1)
            .filter(|c| !c.is_empty());
        let spin_box = focus_window.is_some_and(|w| {
            focus_window_class
                .as_deref()
                .is_some_and(|c| c.eq_ignore_ascii_case("Edit"))
                && is_spin_buddy(w)
        });
        let (menu_bar, status_bar) = if focused {
            crate::screen_lines::window_bars(hwnd)
        } else {
            (Vec::new(), Vec::new())
        };
        // A focus in a page's dialog: the modal dialog around it, which only IA2 says.
        let modal_dialog = if name == "focus"
            && class == "Chrome_RenderWidgetHostHWND"
            && focus.iter().any(|n| n.is_dialog == Some(true))
        {
            use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT};
            // SAFETY: plain Win32 query on a window handle.
            let top = unsafe { GetAncestor(hwnd, GA_ROOT) };
            xa11y_windows::ia2::chromium_modal_dialog_of(top, id_child)
                .ok()
                .flatten()
        } else {
            None
        };
        // What Ctrl+Insert+V says: the focused application's product and version.
        let (process_product, process_version) = if focused {
            process_product_of(hwnd)
        } else {
            (None, None)
        };
        let event = TraceEvent {
            t_ms,
            event: name.to_owned(),
            win_event,
            hwnd: hwnd.0 as isize,
            id_object,
            id_child,
            window_class: Some(class).filter(|s| !s.is_empty()),
            window_style,
            focus_window_class,
            spin_box,
            menu_bar,
            status_bar,
            window_title: Some(title).filter(|s| !s.is_empty()),
            process: process_name(hwnd),
            process_product,
            process_version,
            object,
            focus_chain: focus,
            focus_siblings,
            focus_subtree,
            list_position,
            column_headers,
            text_context,
            region,
            page,
            top_window_title,
            top_window_class,
            top_window_owner_enabled,
            virtual_desktop,
            dialog_lines: dialog.rows,
            dialog_text: dialog.text,
            dialog_page,
            group_box,
            default_button,
            dialog_tree,
            notification: None,
            mouse: None,
            point: None,
            msaa_tree,
            msaa_siblings,
            msaa_ancestors: netui.ancestors,
            msaa_description: netui.description,
            netui_control: netui.control,
            modal_dialog,
        };
        (state.sink)(event);
        state.count += 1;
    });
}

/// Delivers accessibility events to `sink` until `keep_running` says to stop, as the
/// harness records them (`okur-lab trace` writes them to a trace file). Returns the number
/// of events delivered. With `page_changes`, the page in front's objects shown, hidden and
/// reordered are delivered too, as `page_change` events with their object ids and nothing
/// read.
pub fn run_recording(
    sink: Box<dyn FnMut(TraceEvent)>,
    page_changes: bool,
    keep_running: impl Fn() -> bool,
) -> usize {
    run_hooks(sink, None, page_changes, keep_running)
}

/// Delivers accessibility events to `sink` as they happen, with the same capture as
/// [`run_recording`], until `stop` is set. This is the live screen reader's event source,
/// so the engine consumes exactly the event stream the harness records and replays.
/// `page_changed` is told of each change to the page in front outside live regions.
pub fn run_live(
    sink: Box<dyn FnMut(TraceEvent)>,
    page_changed: Box<dyn FnMut(PageChange)>,
    stop: &std::sync::atomic::AtomicBool,
) -> usize {
    run_hooks(sink, Some(page_changed), false, || {
        !stop.load(std::sync::atomic::Ordering::Relaxed)
    })
}

/// Installs the WinEvent hooks on this thread, pumps messages while `keep_running`,
/// and returns the number of events delivered. Live (with `page_changed`), page changes
/// are hooked too.
fn run_hooks(
    sink: Box<dyn FnMut(TraceEvent)>,
    page_changed: Option<Box<dyn FnMut(PageChange)>>,
    record_page_changes: bool,
    keep_running: impl Fn() -> bool,
) -> usize {
    let live = page_changed.is_some();
    // SAFETY: COM for UIA queries on this thread, which also pumps the hook's messages.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    STATE.with(|s| {
        *s.borrow_mut() = Some(TraceState {
            sink,
            uia: automation().ok(),
            count: 0,
            live,
            last_object_focus: None,
            last_mouse: None,
            mouse_moved_at: None,
            last_focus_read: None,
            dialog_read: None,
            focused_object: None,
            page_changed,
            page_changed_at: None,
            page_changes_dropped: 0,
            record_page_changes,
        })
    });

    let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
    // SAFETY: out-of-context hooks with a valid callback; unhooked before returning.
    let hooks = unsafe {
        [
            SetWinEventHook(0x0002, 0x0011, None, Some(on_event), 0, 0, flags),
            // EVENT_OBJECT_FOCUS to EVENT_OBJECT_LIVEREGIONCHANGED. Live, without
            // EVENT_OBJECT_LOCATIONCHANGE: its caret moves are read by the actions worker
            // after a key, and the event cost an MSAA object read for nothing each time the
            // caret moved (performance review 2026-10-23, P18). A trace reads the field on
            // them.
            if live {
                SetWinEventHook(0x8005, 0x800A, None, Some(on_event), 0, 0, flags)
            } else {
                SetWinEventHook(0x8005, 0x8019, None, Some(on_event), 0, 0, flags)
            },
            if live {
                SetWinEventHook(0x800C, 0x8019, None, Some(on_event), 0, 0, flags)
            } else {
                HWINEVENTHOOK::default()
            },
            // EVENT_OBJECT_SHOW to EVENT_OBJECT_REORDER: a page's changes, and a dialog's,
            // whose controls read are used again until it changes.
            SetWinEventHook(0x8002, 0x8004, None, Some(on_page_change), 0, 0, flags),
            // EVENT_CONSOLE_UPDATE_REGION to EVENT_CONSOLE_UPDATE_SCROLL: a classic
            // console's output, for Screen Echo.
            SetWinEventHook(0x4002, 0x4004, None, Some(on_console_update), 0, 0, flags),
            // IA2's own events: text changed in place (a React time stamp) raises no show,
            // hide or reorder, only IA2_EVENT_TEXT_REMOVED and _INSERTED on the block holding
            // it (old-pc, 2026-10-18). Live takes those two; a trace of page changes takes
            // all of IA2's (IA2_EVENT_ACTION_CHANGED to IA2_EVENT_ROLE_CHANGED).
            if record_page_changes {
                SetWinEventHook(0x0101, 0x0123, None, Some(on_page_change), 0, 0, flags)
            } else if live {
                SetWinEventHook(
                    IA2_EVENT_TEXT_INSERTED,
                    IA2_EVENT_TEXT_REMOVED,
                    None,
                    Some(on_page_change),
                    0,
                    0,
                    flags,
                )
            } else {
                HWINEVENTHOOK::default()
            },
        ]
    };

    // SAFETY: plain Win32 call with no preconditions.
    let thread = unsafe { GetCurrentThreadId() };
    let uia_focus: IUIAutomationFocusChangedEventHandler = UiaFocusHandler { thread }.into();
    let uia = STATE.with(|s| s.borrow().as_ref().and_then(|s| s.uia.clone()));
    let uia_focus_added = uia.as_ref().is_some_and(|uia| {
        // SAFETY: a system-wide UIA handler, removed before this thread stops pumping.
        unsafe { uia.AddFocusChangedEventHandler(None, &uia_focus) }
            .inspect_err(|e| tracing::warn!("UIA focus handler not added: {e}"))
            .is_ok()
    });
    let uia_selected: IUIAutomationEventHandler = UiaSelectedHandler { thread }.into();
    // SAFETY: UIA client query on a valid automation object.
    let root = uia
        .as_ref()
        .and_then(|uia| unsafe { uia.GetRootElement() }.ok());
    let uia_selected_added = match (uia.as_ref(), root.as_ref()) {
        // SAFETY: a desktop-wide UIA handler, removed before this thread stops pumping.
        (Some(uia), Some(root)) => unsafe {
            uia.AddAutomationEventHandler(
                UIA_SelectionItem_ElementSelectedEventId,
                root,
                TreeScope_Subtree,
                None,
                &uia_selected,
            )
        }
        .inspect_err(|e| tracing::warn!("UIA selection handler not added: {e}"))
        .is_ok(),
        _ => false,
    };
    let uia_live_region: IUIAutomationEventHandler = UiaLiveRegionHandler { thread }.into();
    let uia_live_region_added = match (uia.as_ref(), root.as_ref()) {
        // SAFETY: a desktop-wide UIA handler, removed before this thread stops pumping.
        (Some(uia), Some(root)) => unsafe {
            uia.AddAutomationEventHandler(
                UIA_LiveRegionChangedEventId,
                root,
                TreeScope_Subtree,
                None,
                &uia_live_region,
            )
        }
        .inspect_err(|e| tracing::warn!("UIA live region handler not added: {e}"))
        .is_ok(),
        _ => false,
    };
    let uia_notification: IUIAutomationNotificationEventHandler =
        UiaNotificationHandler { thread }.into();
    let uia5 = uia.as_ref().and_then(|uia| {
        use windows::core::Interface;
        uia.cast::<windows::Win32::UI::Accessibility::IUIAutomation5>()
            .ok()
    });
    // On old-pc this handler heard none of the notifications a WinForms window raised, which
    // JAWS spoke (820, rounds 247 to 249), nor did any registration `okur-lab
    // probe-notifications` tries; why is open (the plan of round 21).
    let uia_notification_added = match (uia5.as_ref(), root.as_ref()) {
        // SAFETY: a desktop-wide UIA handler, removed before this thread stops pumping.
        (Some(uia5), Some(root)) => unsafe {
            uia5.AddNotificationEventHandler(root, TreeScope_Subtree, None, &uia_notification)
        }
        .inspect_err(|e| tracing::warn!("UIA notification handler not added: {e}"))
        .is_ok(),
        _ => false,
    };
    tracing::debug!(
        added = uia_notification_added,
        uia5 = uia5.is_some(),
        "UIA notification handler"
    );
    let uia_window_opened: IUIAutomationEventHandler = UiaWindowOpenedHandler { thread }.into();
    let uia_window_opened_added = match (uia.as_ref(), root.as_ref()) {
        // SAFETY: a desktop-wide UIA handler, removed before this thread stops pumping.
        (Some(uia), Some(root)) => unsafe {
            uia.AddAutomationEventHandler(
                UIA_Window_WindowOpenedEventId,
                root,
                TreeScope_Subtree,
                None,
                &uia_window_opened,
            )
        }
        .inspect_err(|e| tracing::warn!("UIA window opened handler not added: {e}"))
        .is_ok(),
        _ => false,
    };
    // Where focus is already, as JAWS reads it when it starts: no event will report it.
    in_turn(record_current_focus);

    let mut msg = MSG::default();
    // A trace keeps what happened before it was asked to stop: the events still waiting
    // are read, until no message has come for a second (at most a minute). Stopping at
    // once lost Word's lower ribbon and the return to the document, which the tracer had
    // not read yet (1043, old-pc, 2026-10-25). Live, Okur stops at once.
    let mut stop_asked: Option<std::time::Instant> = None;
    let mut last_message = std::time::Instant::now();
    loop {
        if !keep_running() {
            if live {
                break;
            }
            let asked = *stop_asked.get_or_insert_with(std::time::Instant::now);
            if asked.elapsed() >= DRAIN_MAX || last_message.elapsed() >= DRAIN_IDLE {
                break;
            }
        }
        // SAFETY: standard message pump; waits at most 50 ms between stop checks.
        unsafe {
            let _ = MsgWaitForMultipleObjects(None, false, 50, QS_ALLINPUT);
            let mut uia_focused = false;
            let mut uia_selected_queued = false;
            let mut uia_notified = false;
            let mut uia_live_region_queued = false;
            let mut uia_window_opened_queued = false;
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                last_message = std::time::Instant::now();
                if msg.hwnd.is_invalid() && msg.message == WM_UIA_FOCUS {
                    // Several focus changes queued meanwhile: the focus now is read once.
                    uia_focused = true;
                    continue;
                }
                if msg.hwnd.is_invalid() && msg.message == WM_UIA_SELECTED {
                    uia_selected_queued = true;
                    continue;
                }
                if msg.hwnd.is_invalid() && msg.message == WM_UIA_NOTIFICATION {
                    uia_notified = true;
                    continue;
                }
                if msg.hwnd.is_invalid() && msg.message == WM_UIA_LIVE_REGION {
                    uia_live_region_queued = true;
                    continue;
                }
                if msg.hwnd.is_invalid() && msg.message == WM_UIA_WINDOW_OPENED {
                    uia_window_opened_queued = true;
                    continue;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            in_turn(|| {
                if uia_focused {
                    record_uia_focus();
                }
                if uia_selected_queued {
                    record_uia_selected();
                }
                if uia_notified {
                    record_uia_notifications();
                }
                if uia_live_region_queued {
                    record_uia_live_regions();
                }
                if uia_window_opened_queued {
                    record_uia_windows_opened();
                }
                record_mouse_move();
                record_console_text();
            });
        }
    }
    if let (Some(uia), Some(root), true) = (uia.as_ref(), root.as_ref(), uia_window_opened_added) {
        // SAFETY: removes the handler added above.
        let _ = unsafe {
            uia.RemoveAutomationEventHandler(
                UIA_Window_WindowOpenedEventId,
                root,
                &uia_window_opened,
            )
        };
    }
    if let (Some(uia), Some(root), true) = (uia.as_ref(), root.as_ref(), uia_live_region_added) {
        // SAFETY: removes the handler added above.
        let _ = unsafe {
            uia.RemoveAutomationEventHandler(UIA_LiveRegionChangedEventId, root, &uia_live_region)
        };
    }
    if let (Some(uia5), Some(root), true) = (uia5.as_ref(), root.as_ref(), uia_notification_added) {
        // SAFETY: removes the handler added above.
        let _ = unsafe { uia5.RemoveNotificationEventHandler(root, &uia_notification) };
    }
    if let (Some(uia), Some(root), true) = (uia.as_ref(), root.as_ref(), uia_selected_added) {
        // SAFETY: removes the handler added above.
        let _ = unsafe {
            uia.RemoveAutomationEventHandler(
                UIA_SelectionItem_ElementSelectedEventId,
                root,
                &uia_selected,
            )
        };
    }

    if let (Some(uia), true) = (uia.as_ref(), uia_focus_added) {
        // SAFETY: removes the handler added above.
        let _ = unsafe { uia.RemoveFocusChangedEventHandler(&uia_focus) };
    }
    for hook in hooks {
        if !hook.is_invalid() {
            // SAFETY: hooks installed above.
            let _ = unsafe { UnhookWinEvent(hook) };
        }
    }
    STATE.with(|s| s.borrow_mut().take().map(|s| s.count).unwrap_or(0))
}
