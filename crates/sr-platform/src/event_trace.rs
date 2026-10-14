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

/// Bumped whenever the recorded fields or their meaning change.
pub const TRACE_FORMAT_VERSION: u32 = 1;

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
    event.help_text = focused
        .full_description
        .clone()
        .or_else(|| focused.help_text.clone());
    event.window_class = window_class.map(str::to_owned);
    event.window_title = window_title.map(str::to_owned);

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
            event.container_role = Some(role);
            break;
        }
    }
    Some(event)
}

#[cfg(windows)]
pub use capture::run_trace;

#[cfg(windows)]
mod capture {
    use super::{event_name, TraceEvent, TraceHeader, TraceObject, TRACE_FORMAT_VERSION};
    use crate::uia_snapshot::capture::{automation, focus_chain, window_text};
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
        out: std::fs::File,
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
            };
            if let Ok(mut line) = serde_json::to_string(&event) {
                line.push('\n');
                if state.out.write_all(line.as_bytes()).is_ok() {
                    state.count += 1;
                }
            }
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

        // SAFETY: COM for UIA queries on this thread, which also pumps the hook's messages.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        STATE.with(|s| {
            *s.borrow_mut() = Some(TraceState {
                out: file,
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

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(max_ms);
        let mut msg = MSG::default();
        while std::time::Instant::now() < deadline && !stop_file.is_some_and(|p| p.exists()) {
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
        let count = STATE.with(|s| s.borrow_mut().take().map(|s| s.count).unwrap_or(0));
        Ok(count)
    }
}
