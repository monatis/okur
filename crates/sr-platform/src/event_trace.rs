//! Accessibility event traces.
//!
//! `sr-cli trace` records the system-wide accessibility event stream while a JAWS trial runs
//! on the reference machine: one JSON object per line, each event carrying the MSAA view of
//! its object and, for focus and foreground changes, a UIA snapshot of the focused element
//! and its ancestors. Steps in the evidence carry start/end timestamps, so a step's events
//! can be replayed into the engine hermetically and its speech compared with JAWS's: the
//! verification path for focus-driven behavior (Tab order, forms mode, dialogs, menus,
//! desktop apps) that static document snapshots cannot reach.

use crate::uia_snapshot::UiaNode;
use serde::{Deserialize, Serialize};

/// UIA control type ids used to read focus context.
const UIA_LISTITEM: i32 = 50007;
const UIA_WINDOW: i32 = 50032;
const UIA_DOCUMENT: i32 = 50030;
/// Window classes that host web page content: Chromium (Chrome, Edge, Electron) and Firefox.
const WEB_CONTENT_WINDOW_CLASSES: [&str; 2] = ["Chrome_RenderWidgetHostHWND", "MozillaWindowClass"];
const UIA_TREEITEM: i32 = 50024;
/// MSAA `ROLE_SYSTEM_MENUITEM`.
const MSAA_MENUITEM: i32 = 0x0C;
/// MSAA `ROLE_SYSTEM_BUTTONMENU`.
const MSAA_BUTTONMENU: i32 = 0x39;

/// Bumped whenever the recorded fields or their meaning change.
pub const TRACE_FORMAT_VERSION: u32 = 3;

/// The MSAA view of an event's object (`AccessibleObjectFromEvent`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TraceObject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub states: u32,
}

/// One recorded accessibility event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Unix time in milliseconds when the event was received.
    pub t_ms: u128,
    /// Short event name (`focus`, `foreground`, `name_change`, …).
    pub event: String,
    /// Raw `EVENT_*` id.
    pub win_event: u32,
    pub hwnd: isize,
    pub id_object: i32,
    pub id_child: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<TraceObject>,
    /// For focus and foreground events: the focused element and its ancestors, outermost
    /// first, read through UIA with the snapshot property set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus_chain: Vec<UiaNode>,
    /// For focus events: the focused element's control-view siblings, itself included,
    /// without their children.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus_siblings: Vec<UiaNode>,
    /// For focus events: the focused element with its control-view subtree to depth 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_subtree: Option<UiaNode>,
    /// For focus on a Win32 combo box or list box: the selection as `[index, count]`,
    /// with a 0-based index, asked of the control itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_position: Option<[i32; 2]>,
}

/// The first line of a trace file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceHeader {
    pub trace_format_version: u32,
    pub tool_version: String,
    pub host: String,
    pub started_unix_ms: u128,
}

/// Reads a trace file: the header line, then one event per line (malformed lines skipped).
pub fn read_trace(
    path: impl AsRef<std::path::Path>,
) -> Result<(TraceHeader, Vec<TraceEvent>), String> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
    let mut lines = text.lines();
    let header: TraceHeader = lines
        .next()
        .ok_or_else(|| format!("{path:?}: empty trace"))
        .and_then(|l| {
            serde_json::from_str(l.trim_start_matches('\u{feff}'))
                .map_err(|e| format!("{path:?}: header: {e}"))
        })?;
    let events = lines.filter_map(|l| serde_json::from_str(l).ok()).collect();
    Ok((header, events))
}

/// Name of a recorded `EVENT_*` id, or `None` for events the tracer ignores.
pub fn event_name(win_event: u32, id_object: i32) -> Option<&'static str> {
    const OBJID_CARET: i32 = -8;
    Some(match win_event {
        0x0002 => "alert",
        0x0003 => "foreground",
        0x0004 => "menu_start",
        0x0005 => "menu_end",
        0x0006 => "menu_popup_start",
        0x0007 => "menu_popup_end",
        0x0010 => "dialog_start",
        0x0011 => "dialog_end",
        0x8005 => "focus",
        0x8006 => "selection",
        0x8007 => "selection_add",
        0x8008 => "selection_remove",
        0x8009 => "selection_within",
        0x800A => "state_change",
        0x800B if id_object == OBJID_CARET => "caret",
        0x800C => "name_change",
        0x800D => "description_change",
        0x800E => "value_change",
        0x8014 => "text_selection_change",
        0x8019 => "live_region_change",
        _ => return None,
    })
}

/// Builds the engine's focus event from a focus chain (outermost first), using the same
/// UIA-to-AOM mapping as document snapshots. Used by trace replay, and by the live listener,
/// so both exercise one conversion.
pub fn focus_event_from_chain(
    chain: &[UiaNode],
    window_class: Option<&str>,
    window_title: Option<&str>,
) -> Option<sr_core::FocusEvent> {
    use sr_core::Role;
    let focused = chain.last()?;
    let mut leaf = focused.clone();
    leaf.children.clear();
    let aom = crate::uia_snapshot::to_aom(&leaf);

    let mut event = sr_core::FocusEvent::new(aom.role.clone());
    event.name = aom.name.clone();
    event.value = aom.value.clone();
    event.states = aom.states.clone();
    event.item_index = focused
        .position_in_set
        .and_then(|p| usize::try_from(p).ok());
    event.item_count = focused.size_of_set.and_then(|s| usize::try_from(s).ok());
    if focused.control_type == UIA_TREEITEM {
        // Nesting level: tree items above this one in the control view.
        event.tree_level = Some(
            chain
                .iter()
                .rev()
                .skip(1)
                .filter(|a| a.control_type == UIA_TREEITEM)
                .count(),
        );
    }
    event.help_text = focused
        .full_description
        .clone()
        .or_else(|| focused.help_text.clone());
    event.window_class = window_class.map(str::to_owned);
    event.window_title = window_title.map(str::to_owned);
    // Inside a browser document, Chrome reports the DOM id as AutomationId.
    // A page: a document in a browser's web content window. Documents elsewhere (Windows 11
    // Notepad, Word) are native text controls, read by caret, not by the virtual cursor.
    event.in_document = window_class.is_some_and(|c| WEB_CONTENT_WINDOW_CLASSES.contains(&c))
        && chain.iter().any(|a| a.control_type == UIA_DOCUMENT);
    if event.in_document {
        event.dom_id = focused.automation_id.clone().filter(|id| !id.is_empty());
    }

    // The nearest structural container (list, group, tab list, tree, table, dialog, landmark).
    for ancestor in chain.iter().rev().skip(1) {
        let mut a = ancestor.clone();
        a.children.clear();
        let role = crate::uia_snapshot::to_aom(&a).role;
        let is_container = matches!(
            role,
            Role::List
                | Role::TabList
                | Role::Tree
                | Role::Table
                | Role::Dialog
                | Role::AlertDialog
                | Role::Landmark { .. }
        ) || (role == Role::Group
            && a.name.as_deref().is_some_and(|n| !n.trim().is_empty()));
        if is_container {
            event.container_name = a.name.clone();
            event.container_class = a.class_name.clone();
            event.container_role = Some(role);
            break;
        }
    }
    Some(event)
}

/// The focus event a recorded `focus` trace event delivers to the engine: the chain mapping
/// of [`focus_event_from_chain`], plus the context a live listener would query.
///
/// Without a provider PositionInSet (WinForms and most Win32 controls report none), the
/// position is counted among the focused element's siblings of the same control type.
pub fn focus_event_from_trace(event: &TraceEvent) -> Option<sr_core::FocusEvent> {
    // Menus: UIA keeps reporting the control focused before the menu (WinForms MenuStrip),
    // while the MSAA focus event names the menu item, as JAWS reads it [obs, 302].
    if let Some(object) = event
        .object
        .as_ref()
        .filter(|o| o.role == Some(MSAA_MENUITEM))
    {
        return Some(
            sr_core::FocusEvent::new(sr_core::Role::MenuItem)
                .with_name(object.name.clone().unwrap_or_default()),
        );
    }
    // The recorded window text belongs to the focused HWND (an edit's text is its content);
    // the window the user is in is the outermost element of the focus chain.
    let top_window = event
        .focus_chain
        .first()
        .filter(|w| w.control_type == UIA_WINDOW)
        .and_then(|w| w.name.as_deref());
    let mut focus = focus_event_from_chain(
        &event.focus_chain,
        event.window_class.as_deref(),
        top_window,
    )?;
    let focused = event.focus_chain.last()?;
    // JAWS names buttons by their MSAA role: ROLE_SYSTEM_BUTTONMENU is a "button menu"
    // even when UIA reports a plain button [obs, 204/s11–s12].
    if focus.role == sr_core::Role::Button
        && event.object.as_ref().and_then(|o| o.role) == Some(MSAA_BUTTONMENU)
    {
        focus.role = sr_core::Role::MenuButton;
    }
    if let Some(subtree) = &event.focus_subtree {
        // A list-view item's further columns are its text children after the name.
        if focus.role == sr_core::Role::ListItem {
            focus.columns = subtree
                .children
                .iter()
                .filter_map(|c| c.name.clone())
                .filter(|n| !n.trim().is_empty() && Some(n) != focused.name.as_ref())
                .collect();
        }
        // A combo box's position is its selected item's, among its list items.
        if focus.role == sr_core::Role::ComboBox && focus.item_index.is_none() {
            let items: Vec<&UiaNode> = subtree
                .children
                .iter()
                .flat_map(|c| std::iter::once(c).chain(c.children.iter()))
                .filter(|n| n.control_type == UIA_LISTITEM)
                .collect();
            if let Some(i) = items.iter().position(|n| n.is_selected == Some(true)) {
                focus.item_index = Some(i + 1);
                focus.item_count = Some(items.len());
            }
        }
    }
    if let (None, Some([index, count])) = (focus.item_index, event.list_position) {
        focus.item_index = usize::try_from(index).ok().map(|i| i + 1);
        focus.item_count = usize::try_from(count).ok();
    }
    if focus.item_index.is_none() && focus.role != sr_core::Role::ComboBox {
        let peers: Vec<&UiaNode> = event
            .focus_siblings
            .iter()
            .filter(|s| s.control_type == focused.control_type)
            .collect();
        let index = peers
            .iter()
            .position(|s| s.has_keyboard_focus)
            .or_else(|| peers.iter().position(|s| s.name == focused.name));
        if let Some(index) = index {
            focus.item_index = Some(index + 1);
            focus.item_count = Some(peers.len());
        }
    }
    Some(focus)
}

#[cfg(windows)]
pub use capture::{run_live, run_trace};

/// The menu mode transition a recorded event reports, if any.
pub fn menu_event_from_trace(event: &TraceEvent) -> Option<sr_core::MenuEvent> {
    use sr_core::MenuEvent;
    match event.event.as_str() {
        "menu_start" => Some(MenuEvent::Start),
        "menu_end" => Some(MenuEvent::End),
        "menu_popup_start" => Some(MenuEvent::PopupStart),
        "menu_popup_end" => Some(MenuEvent::PopupEnd),
        _ => None,
    }
}

#[cfg(windows)]
mod capture {
    use super::{event_name, TraceEvent, TraceHeader, TraceObject, TRACE_FORMAT_VERSION};
    use crate::uia_snapshot::capture::{automation, focus_chain, focus_context, window_text};
    use crate::PlatformError;
    use std::cell::RefCell;
    use std::io::Write;
    use windows::Win32::Foundation::{CloseHandle, HWND};
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::{
        AccessibleObjectFromEvent, IAccessible, IUIAutomation, SetWinEventHook, UnhookWinEvent,
        HWINEVENTHOOK,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetWindowThreadProcessId, MsgWaitForMultipleObjects, PeekMessageW,
        TranslateMessage, MSG, PM_REMOVE, QS_ALLINPUT,
    };

    const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
    const WINEVENT_SKIPOWNPROCESS: u32 = 0x0002;

    struct TraceState {
        /// Receives every event: the trace file, or the live screen reader.
        sink: Box<dyn FnMut(TraceEvent)>,
        uia: Option<IUIAutomation>,
        count: usize,
    }

    thread_local! {
        // The out-of-context hook calls back on the installing thread, so thread-local
        // state is sufficient and needs no locking.
        static STATE: RefCell<Option<TraceState>> = const { RefCell::new(None) };
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
            let path = String::from_utf16_lossy(buf.get(..len as usize).unwrap_or_default());
            path.rsplit('\\').next().map(str::to_owned)
        }
    }

    /// The selection of a Win32 combo box or list box as `[index, count]`, asked of the
    /// control itself (`CB_GETCURSEL`/`CB_GETCOUNT`, `LB_GETCURSEL`/`LB_GETCOUNT`). A
    /// collapsed WinForms combo box exposes no items through UIA, yet JAWS says "2 of 3".
    fn win32_list_position(hwnd: HWND, class: &str) -> Option<[i32; 2]> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG};
        let upper = class.to_ascii_uppercase();
        let (get_sel, get_count) = if upper.contains("COMBOBOX") {
            (0x0147, 0x0146)
        } else if upper.contains("LISTBOX") {
            (0x0188, 0x018B)
        } else {
            return None;
        };
        let ask = |msg: u32| {
            let mut result = 0usize;
            // SAFETY: a read-only query message with a timeout, to a window that may be gone.
            let sent = unsafe {
                SendMessageTimeoutW(
                    hwnd,
                    msg,
                    WPARAM(0),
                    LPARAM(0),
                    SMTO_ABORTIFHUNG,
                    200,
                    Some(&mut result),
                )
            };
            // CB_ERR/LB_ERR is -1, as the low 32 bits of the result.
            (sent.0 != 0).then_some(result as i32)
        };
        let index = ask(get_sel).filter(|&i| i >= 0)?;
        let count = ask(get_count).filter(|&n| n > 0)?;
        Some([index, count])
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
            })
        }
    }

    unsafe extern "system" fn on_event(
        _hook: HWINEVENTHOOK,
        win_event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        let Some(name) = event_name(win_event, id_object) else {
            return;
        };
        let t_ms = now_ms();
        let (title, class) = if hwnd.is_invalid() {
            (String::new(), String::new())
        } else {
            window_text(hwnd)
        };
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let Some(state) = state.as_mut() else { return };
            let focus = if matches!(name, "focus" | "foreground") {
                state.uia.as_ref().map(focus_chain).unwrap_or_default()
            } else {
                Vec::new()
            };
            let (focus_siblings, focus_subtree) = match (name, state.uia.as_ref()) {
                ("focus", Some(uia)) => focus_context(uia),
                _ => (Vec::new(), None),
            };
            let list_position = if name == "focus" {
                win32_list_position(hwnd, &class)
            } else {
                None
            };
            let event = TraceEvent {
                t_ms,
                event: name.to_owned(),
                win_event,
                hwnd: hwnd.0 as isize,
                id_object,
                id_child,
                window_class: Some(class).filter(|s| !s.is_empty()),
                window_title: Some(title).filter(|s| !s.is_empty()),
                process: process_name(hwnd),
                object: msaa_object(hwnd, id_object, id_child),
                focus_chain: focus,
                focus_siblings,
                focus_subtree,
                list_position,
            };
            (state.sink)(event);
            state.count += 1;
        });
    }

    /// Records accessibility events to `out` (JSON Lines) until `stop_file` exists or
    /// `max_ms` elapses. Returns the number of events recorded.
    pub fn run_trace(
        out: &std::path::Path,
        stop_file: Option<&std::path::Path>,
        max_ms: u64,
    ) -> Result<usize, PlatformError> {
        let io_err = |e: std::io::Error| PlatformError::EventListenerError(format!("{out:?}: {e}"));
        let mut file = std::fs::File::create(out).map_err(io_err)?;
        let header = TraceHeader {
            trace_format_version: TRACE_FORMAT_VERSION,
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
            host: std::env::var("COMPUTERNAME").unwrap_or_default(),
            started_unix_ms: now_ms(),
        };
        let header = serde_json::to_string(&header)
            .map_err(|e| PlatformError::EventListenerError(e.to_string()))?;
        file.write_all(format!("{header}\n").as_bytes())
            .map_err(io_err)?;

        let sink = move |event: TraceEvent| {
            if let Ok(mut line) = serde_json::to_string(&event) {
                line.push('\n');
                let _ = file.write_all(line.as_bytes());
            }
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(max_ms);
        Ok(run_hooks(Box::new(sink), || {
            std::time::Instant::now() < deadline && !stop_file.is_some_and(|p| p.exists())
        }))
    }

    /// Delivers accessibility events to `sink` as they happen, with the same capture as
    /// [`run_trace`], until `stop` is set. This is the live screen reader's event source,
    /// so the engine consumes exactly the event stream the harness records and replays.
    pub fn run_live(
        sink: Box<dyn FnMut(TraceEvent)>,
        stop: &std::sync::atomic::AtomicBool,
    ) -> usize {
        run_hooks(sink, || !stop.load(std::sync::atomic::Ordering::Relaxed))
    }

    /// Installs the WinEvent hooks on this thread, pumps messages while `keep_running`,
    /// and returns the number of events delivered.
    fn run_hooks(sink: Box<dyn FnMut(TraceEvent)>, keep_running: impl Fn() -> bool) -> usize {
        // SAFETY: COM for UIA queries on this thread, which also pumps the hook's messages.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        STATE.with(|s| {
            *s.borrow_mut() = Some(TraceState {
                sink,
                uia: automation().ok(),
                count: 0,
            })
        });

        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        // SAFETY: out-of-context hooks with a valid callback; unhooked before returning.
        let hooks = unsafe {
            [
                SetWinEventHook(0x0002, 0x0011, None, Some(on_event), 0, 0, flags),
                SetWinEventHook(0x8005, 0x8019, None, Some(on_event), 0, 0, flags),
            ]
        };

        let mut msg = MSG::default();
        while keep_running() {
            // SAFETY: standard message pump; waits at most 50 ms between stop checks.
            unsafe {
                let _ = MsgWaitForMultipleObjects(None, false, 50, QS_ALLINPUT);
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        for hook in hooks {
            if !hook.is_invalid() {
                // SAFETY: hooks installed above.
                let _ = unsafe { UnhookWinEvent(hook) };
            }
        }
        STATE.with(|s| s.borrow_mut().take().map(|s| s.count).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RADIO: i32 = 50013;
    const GROUP: i32 = 50026;
    const TEXT: i32 = 50020;

    fn node(control_type: i32, name: &str, focused: bool) -> UiaNode {
        UiaNode {
            control_type,
            name: Some(name.to_owned()),
            has_keyboard_focus: focused,
            is_enabled: true,
            ..UiaNode::default()
        }
    }

    fn focus_event(siblings: Vec<UiaNode>) -> TraceEvent {
        TraceEvent {
            t_ms: 0,
            event: "focus".to_owned(),
            win_event: 0x8005,
            hwnd: 0,
            id_object: -4,
            id_child: 0,
            window_class: None,
            window_title: None,
            process: None,
            object: None,
            focus_chain: vec![node(GROUP, "Plan", false), node(RADIO, "Pro", true)],
            focus_siblings: siblings,
            focus_subtree: None,
            list_position: None,
        }
    }

    /// WinForms radio buttons report no PositionInSet; JAWS still says "2 of 3" [obs, 301/s4].
    #[test]
    fn position_is_counted_among_same_type_siblings() {
        let event = focus_event(vec![
            node(TEXT, "Choose a plan", false),
            node(RADIO, "Basic", false),
            node(RADIO, "Pro", true),
            node(RADIO, "Team", false),
        ]);
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus.map(|f| (f.item_index, f.item_count)),
            Some((Some(2), Some(3)))
        );
    }

    #[test]
    fn no_siblings_means_no_position() {
        let focus = focus_event_from_trace(&focus_event(Vec::new()));
        assert_eq!(focus.map(|f| f.item_index), Some(None));
    }
}
