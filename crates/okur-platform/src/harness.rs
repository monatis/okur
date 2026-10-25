//! Harness tools for live checks on a machine the user lets the agent drive (`okur-lab keys`,
//! `okur-lab jaws-history`): key chords sent only while an allowed application is in front,
//! so a key never lands in another window, and JAWS's speech history read through its
//! FSAPI COM object with the user's clipboard put back afterwards.

/// One step of a key sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyStep {
    /// Keys pressed in order and released in reverse ("Insert+Alt+W").
    Chord(Vec<u16>),
    /// A key pressed and held ("down:Alt").
    Down(u16),
    /// A held key released ("up:Alt").
    Up(u16),
    /// A pause in milliseconds ("wait:500").
    Wait(u64),
}

/// The virtual key of a key name, as scenarios write them ("Insert", "CapsLock", "Alt",
/// "Down", "F12", "W", "5").
pub fn virtual_key(name: &str) -> Option<u16> {
    let named = match name.to_ascii_lowercase().as_str() {
        "insert" => 0x2D,
        "capslock" => 0x14,
        "scrolllock" => 0x91,
        "alt" => 0x12,
        "ctrl" | "control" => 0x11,
        "shift" => 0x10,
        "win" => 0x5B,
        "up" => 0x26,
        "down" => 0x28,
        "left" => 0x25,
        "right" => 0x27,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "delete" => 0x2E,
        "escape" | "esc" => 0x1B,
        "tab" => 0x09,
        "enter" => 0x0D,
        "space" => 0x20,
        "backspace" => 0x08,
        "comma" | "," => 0xBC,
        "period" | "." => 0xBE,
        "semicolon" | ";" => 0xBA,
        "apostrophe" | "'" => 0xDE,
        "minus" | "-" => 0xBD,
        // The numeric keypad's operators, the same on every layout (Calculator), also by the
        // names scenarios give them as the cursor keys ("NumpadMinus", "NumpadSlash").
        "numpadadd" | "numpadplus" => 0x6B,
        "numpadsubtract" | "numpadminus" => 0x6D,
        "numpadmultiply" | "numpadstar" => 0x6A,
        "numpaddivide" | "numpadslash" => 0x6F,
        // The keypad's digits as with Num Lock off, as JAWS's desktop layout uses them
        // ("Insert+Numpad5" is Say Word): the gray keys' virtual keys, Numpad 5 Clear, sent
        // without the extended flag. As digits, a key JAWS did not take typed "5" into a Word
        // document (old-pc, 2026-10-24).
        "numpaddelete" | "numpaddecimal" => KEYPAD | 0x2E,
        other if other.len() == 7 && other.starts_with("numpad") => {
            const AS_CURSOR_KEYS: [u16; 10] =
                [0x2D, 0x23, 0x28, 0x22, 0x25, 0x0C, 0x27, 0x24, 0x26, 0x21];
            return other
                .get(6..)
                .and_then(|d| d.parse::<usize>().ok())
                .and_then(|d| AS_CURSOR_KEYS.get(d))
                .map(|vk| KEYPAD | vk);
        }
        other => {
            let mut chars = other.chars();
            return match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphanumeric() => {
                    u16::try_from(u32::from(c.to_ascii_uppercase())).ok()
                }
                (Some('f'), Some(_)) => other
                    .get(1..)
                    .and_then(|n| n.parse::<u16>().ok())
                    .filter(|n| (1..=24).contains(n))
                    .map(|n| 0x6F + n),
                _ => None,
            };
        }
    };
    Some(named)
}

pub use crate::key_input::{is_extended, KEYPAD};

/// The modifier keys, left and right apart, and Insert (the JAWS key): the keys a chord holds
/// while the others are pressed, so a lost key-up leaves one of them down for every
/// application.
pub const HELD_KEYS: [(u16, &str); 9] = [
    (0xA0, "left Shift"),
    (0xA1, "right Shift"),
    (0xA2, "left Ctrl"),
    (0xA3, "right Ctrl"),
    (0xA4, "left Alt"),
    (0xA5, "right Alt"),
    (0x5B, "left Windows"),
    (0x5C, "right Windows"),
    (0x2D, "Insert"),
];

/// A step from its text: a chord ("Ctrl+Home"), "down:Key", "up:Key" or "wait:Ms".
pub fn parse_step(text: &str) -> Result<KeyStep, String> {
    let key = |name: &str| virtual_key(name).ok_or_else(|| format!("unknown key {name:?}"));
    if let Some(name) = text.strip_prefix("down:") {
        return Ok(KeyStep::Down(key(name)?));
    }
    if let Some(name) = text.strip_prefix("up:") {
        return Ok(KeyStep::Up(key(name)?));
    }
    if let Some(ms) = text.strip_prefix("wait:") {
        return ms
            .parse()
            .map(KeyStep::Wait)
            .map_err(|_| format!("bad wait {ms:?}"));
    }
    text.split('+')
        .map(key)
        .collect::<Result<Vec<_>, _>>()
        .map(KeyStep::Chord)
}

#[cfg(windows)]
pub use crate::key_input::send_key;
#[cfg(windows)]
pub use windows_harness::{
    bring_to_front, foreground_process, foreground_title, ia2_probe_focus, jaws_history,
    msaa_focus_probe, send_steps, shell_window, window_titled,
};

#[cfg(windows)]
mod windows_harness {
    use super::{KeyStep, HELD_KEYS};
    use std::time::{Duration, Instant};

    /// Pause after each key event, as run-scenario.ps1 sends chords.
    const KEY_GAP: Duration = Duration::from_millis(30);

    /// The foreground window's process, by executable name without its extension.
    pub fn foreground_process() -> String {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };
        let mut pid = 0u32;
        // SAFETY: plain Win32 queries; the process handle is closed below.
        unsafe {
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
            let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return String::new();
            };
            let mut buf = [0u16; 512];
            let mut len = u32::try_from(buf.len()).unwrap_or(0);
            let ok = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok();
            let _ = CloseHandle(process);
            if !ok {
                return String::new();
            }
            let path = String::from_utf16_lossy(buf.get(..len as usize).unwrap_or_default());
            let file = path.rsplit('\\').next().unwrap_or_default();
            file.strip_suffix(".exe")
                .or_else(|| file.strip_suffix(".EXE"))
                .unwrap_or(file)
                .to_owned()
        }
    }

    fn key(vk: u16, up: bool) {
        send_key(vk, up, 0);
        std::thread::sleep(KEY_GAP);
    }

    /// A chord, then the pause run-scenario.ps1 leaves after one.
    fn chord(keys: &[u16]) {
        crate::key_input::send_chord(keys);
        std::thread::sleep(KEY_GAP);
    }

    use crate::key_input::send_key;

    /// Whether the foreground window's process runs at a higher integrity level than ours
    /// (JAWS's dialogs, an elevated application). Windows refuses our input to it and hides
    /// the key state from us while it is in front. A process whose token we may not read is
    /// taken as higher.
    fn foreground_is_higher() -> bool {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::Security::{
            GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
            TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
        };
        use windows::Win32::System::Threading::{
            GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };
        fn integrity(process: HANDLE) -> Option<u32> {
            let mut token = HANDLE::default();
            // u64s, so the label read from the buffer is aligned for its pointer.
            let mut buf = [0u64; 16];
            let mut len = 0u32;
            // SAFETY: the token is closed below; the label is read from our own buffer,
            // which GetTokenInformation filled and whose size it checked.
            unsafe {
                OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
                let read = GetTokenInformation(
                    token,
                    TokenIntegrityLevel,
                    Some(buf.as_mut_ptr().cast()),
                    u32::try_from(std::mem::size_of_val(&buf)).unwrap_or(0),
                    &mut len,
                );
                let _ = CloseHandle(token);
                read.ok()?;
                let label = &*buf.as_ptr().cast::<TOKEN_MANDATORY_LABEL>();
                let sid = label.Label.Sid;
                let count = *GetSidSubAuthorityCount(sid);
                Some(*GetSidSubAuthority(sid, u32::from(count.checked_sub(1)?)))
            }
        }
        let mut pid = 0u32;
        // SAFETY: plain Win32 queries; the process handle is closed below.
        unsafe {
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
            let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return true;
            };
            let theirs = integrity(process);
            let _ = CloseHandle(process);
            match (theirs, integrity(GetCurrentProcess())) {
                (Some(theirs), Some(ours)) => theirs > ours,
                _ => true,
            }
        }
    }

    /// Brings the visible top-level window whose title contains `title` (case-insensitive)
    /// to the front, restoring it if minimized, and returns its whole title. The input of
    /// the window in front is attached for the switch, as Windows lets only the foreground
    /// thread hand the foreground on.
    pub fn bring_to_front(title: &str) -> Result<String, String> {
        use windows::core::BOOL;
        use windows::Win32::Foundation::{HWND, LPARAM};
        use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowTextW,
            GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow,
            SW_RESTORE,
        };
        struct Search {
            wanted: String,
            found: Option<(HWND, String)>,
        }
        unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> BOOL {
            // SAFETY: `data` is the `Search` passed to EnumWindows below, alive for the call.
            let search = unsafe { &mut *(data.0 as *mut Search) };
            let mut buf = [0u16; 512];
            // SAFETY: plain Win32 queries into a buffer of the given size.
            let (visible, len) = unsafe {
                (
                    IsWindowVisible(hwnd).as_bool(),
                    GetWindowTextW(hwnd, &mut buf),
                )
            };
            let text = String::from_utf16_lossy(
                buf.get(..usize::try_from(len).unwrap_or(0))
                    .unwrap_or_default(),
            );
            if visible && text.to_lowercase().contains(&search.wanted) {
                search.found = Some((hwnd, text));
                return BOOL(0);
            }
            BOOL(1)
        }
        let mut search = Search {
            wanted: title.to_lowercase(),
            found: None,
        };
        // SAFETY: the callback reads `search` only during the call.
        let _ = unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) };
        let (hwnd, text) = search
            .found
            .ok_or_else(|| format!("no visible window's title contains {title:?}"))?;
        // SAFETY: plain Win32 window calls; the thread input is detached again below.
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let front = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let ours = GetCurrentThreadId();
            let attached = front != ours && AttachThreadInput(ours, front, true).as_bool();
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
            if attached {
                let _ = AttachThreadInput(ours, front, false);
            }
        }
        std::thread::sleep(Duration::from_millis(300));
        if foreground_title() == text {
            Ok(text)
        } else {
            Err(format!("{text:?} did not come to the front"))
        }
    }

    /// What the window with the keyboard focus, and the window in front, offer through
    /// IAccessible2: one JSON line each, with the window's class. Research: whether an
    /// application that is not a browser (Word) implements IA2, and what it reports there.
    pub fn ia2_probe_focus() -> Vec<String> {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        // SAFETY: COM for MSAA on this thread; repeated initialization is tolerated.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let (front, focus) = front_and_focus_windows();
        focus
            .into_iter()
            .map(|w| ("focus", w))
            .chain(std::iter::once(("front", front)))
            .map(|(which, window)| {
                let probe = xa11y_windows::ia2::probe_window(window);
                serde_json::json!({ "window": which, "class": class_of(window), "probe": probe })
                    .to_string()
            })
            .collect()
    }

    /// The window in front, and the window with the keyboard focus in its thread.
    fn front_and_focus_windows() -> (
        windows::Win32::Foundation::HWND,
        Option<windows::Win32::Foundation::HWND>,
    ) {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
        };
        // SAFETY: plain Win32 queries; the info struct is sized as the call requires.
        unsafe {
            let front = GetForegroundWindow();
            let thread = GetWindowThreadProcessId(front, None);
            let mut info = GUITHREADINFO {
                cbSize: u32::try_from(std::mem::size_of::<GUITHREADINFO>()).unwrap_or(0),
                ..Default::default()
            };
            let focus = GetGUIThreadInfo(thread, &mut info)
                .ok()
                .map(|_| info.hwndFocus)
                .filter(|h| !h.is_invalid());
            (front, focus)
        }
    }

    /// A window's class name; empty for a window gone.
    fn class_of(window: windows::Win32::Foundation::HWND) -> String {
        use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
        let mut buffer = [0u16; 256];
        // SAFETY: the buffer outlives the call; a stale window gives 0.
        let n = unsafe { GetClassNameW(window, &mut buffer) };
        String::from_utf16_lossy(
            buffer
                .get(..usize::try_from(n).unwrap_or(0))
                .unwrap_or_default(),
        )
    }

    /// The first object below `acc`, depth first, whose state is focused and that is an
    /// object of its own (not a child id), while `budget` objects last.
    fn focused_descendant(
        acc: &windows::Win32::UI::Accessibility::IAccessible,
        budget: &mut usize,
    ) -> Option<windows::Win32::UI::Accessibility::IAccessible> {
        use windows::core::Interface;
        use windows::Win32::System::Variant::{VARIANT, VT_DISPATCH};
        use windows::Win32::UI::Accessibility::{AccessibleChildren, IAccessible};
        const FOCUSED: i32 = 0x4;
        // SAFETY: a property read on a valid IAccessible.
        let count = unsafe { acc.accChildCount() }.unwrap_or(0).clamp(0, 500);
        let mut children = vec![VARIANT::default(); usize::try_from(count).unwrap_or(0)];
        let mut obtained = 0i32;
        // SAFETY: the slice's length is the count asked for.
        unsafe { AccessibleChildren(acc, 0, &mut children, &mut obtained) }.ok()?;
        for item in children.iter().take(usize::try_from(obtained).unwrap_or(0)) {
            if *budget == 0 {
                return None;
            }
            *budget -= 1;
            // SAFETY: reading the discriminant and, for VT_DISPATCH, the dispatch pointer.
            let object = unsafe {
                (item.Anonymous.Anonymous.vt == VT_DISPATCH)
                    .then(|| item.Anonymous.Anonymous.Anonymous.pdispVal.as_ref())
                    .flatten()
                    .and_then(|d| d.cast::<IAccessible>().ok())
            };
            let Some(object) = object else { continue };
            // SAFETY: a property read on a valid IAccessible.
            let state = unsafe { object.get_accState(&VARIANT::from(0i32)) }
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
                .unwrap_or(0);
            if state & FOCUSED != 0 {
                return Some(object);
            }
            if let Some(found) = focused_descendant(&object, budget) {
                return Some(found);
            }
        }
        None
    }

    /// The MSAA object with the focus, followed from the focus window's client object by
    /// `accFocus`, and the UI Automation element Windows gives for it
    /// (`ElementFromIAccessible`), with what each says beside the name: MSAA's value,
    /// description and help, UIA's help text, full description, item status, toggle and
    /// expand state; and UIA's own focused element. One JSON line. Research: where a control's
    /// tooltip and toggle state come from in a ribbon that UIA's focus does not enter (Word's
    /// NetUI).
    pub fn msaa_focus_probe() -> String {
        use windows::core::Interface;
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        use windows::Win32::System::Variant::{VARIANT, VT_DISPATCH, VT_I4};
        use windows::Win32::UI::Accessibility::{
            AccessibleObjectFromWindow, IAccessible, TreeScope_Descendants,
            UIA_HasKeyboardFocusPropertyId, UIA_NamePropertyId,
        };
        const OBJID_CLIENT: u32 = 0xFFFF_FFFC;
        // SAFETY: COM for MSAA and UIA on this thread; repeated initialization is tolerated.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let (front, focus) = front_and_focus_windows();
        let window = focus.unwrap_or(front);
        let mut raw = std::ptr::null_mut();
        // SAFETY: requests IAccessible for the client object of a window.
        let found = unsafe {
            AccessibleObjectFromWindow(window, OBJID_CLIENT, &IAccessible::IID, &mut raw)
        };
        let mut msaa = serde_json::Map::new();
        msaa.insert("window_class".into(), class_of(window).into());
        let mut object = (!raw.is_null() && found.is_ok())
            // SAFETY: AccessibleObjectFromWindow returned an owned IAccessible pointer.
            .then(|| unsafe { IAccessible::from_raw(raw) });
        let mut child = VARIANT::from(0i32);
        // accFocus down to the object that has it: an object of its own, or a child id.
        let mut path = Vec::new();
        for _ in 0..12 {
            let Some(acc) = object.as_ref() else { break };
            // SAFETY: a property read on a valid IAccessible.
            let Ok(focus) = (unsafe { acc.accFocus() }) else {
                break;
            };
            // SAFETY: reading the discriminant and the member it names.
            let (vt, dispatch, id) = unsafe {
                let inner = &focus.Anonymous.Anonymous;
                (
                    inner.vt,
                    (inner.vt == VT_DISPATCH)
                        .then(|| inner.Anonymous.pdispVal.as_ref().cloned())
                        .flatten(),
                    (inner.vt == VT_I4).then_some(inner.Anonymous.lVal),
                )
            };
            if let Some(next) = dispatch.as_ref().and_then(|d| d.cast::<IAccessible>().ok()) {
                // SAFETY: a property read on a valid IAccessible.
                path.push(
                    unsafe { next.get_accName(&VARIANT::from(0i32)) }
                        .map(|b| b.to_string())
                        .unwrap_or_default(),
                );
                object = Some(next);
                continue;
            }
            if let Some(id) = id.filter(|&id| id != 0) {
                child = VARIANT::from(id);
                path.push(format!("child {id}"));
            }
            if dispatch.is_none() && id.is_none() {
                path.push(format!("vt {}", vt.0));
                // accFocus answers nothing below Word's ribbon: the object with the focused
                // state is searched for in its tree instead.
                if let Some(found) = focused_descendant(acc, &mut 4000) {
                    path.push("searched".to_owned());
                    object = Some(found);
                }
            }
            break;
        }
        msaa.insert("focus_path".into(), path.into());
        let text = |b: windows::core::Result<windows::core::BSTR>| {
            b.ok()
                .map(|b| b.to_string())
                .filter(|t| !t.is_empty())
                .map_or(serde_json::Value::Null, Into::into)
        };
        let mut uia = serde_json::Value::Null;
        let mut msaa_center = None;
        if let Some(acc) = object.as_ref() {
            let (mut x, mut y, mut w, mut h) = (0, 0, 0, 0);
            // SAFETY: out-pointers to locals on a valid IAccessible.
            if unsafe { acc.accLocation(&mut x, &mut y, &mut w, &mut h, &child) }.is_ok() {
                msaa.insert("location".into(), serde_json::json!([x, y, w, h]));
                msaa_center = Some(windows::Win32::Foundation::POINT {
                    x: x + w / 2,
                    y: y + h / 2,
                });
            }
            // SAFETY: property reads on a valid IAccessible.
            unsafe {
                msaa.insert("name".into(), text(acc.get_accName(&child)));
                msaa.insert(
                    "role".into(),
                    acc.get_accRole(&child)
                        .ok()
                        .and_then(|v| i32::try_from(&v).ok())
                        .into(),
                );
                msaa.insert(
                    "states".into(),
                    acc.get_accState(&child)
                        .ok()
                        .and_then(|v| i32::try_from(&v).ok())
                        .into(),
                );
                msaa.insert("value".into(), text(acc.get_accValue(&child)));
                msaa.insert("description".into(), text(acc.get_accDescription(&child)));
                msaa.insert("help".into(), text(acc.get_accHelp(&child)));
                msaa.insert("shortcut".into(), text(acc.get_accKeyboardShortcut(&child)));
                msaa.insert(
                    "default_action".into(),
                    text(acc.get_accDefaultAction(&child)),
                );
            }
            let child_id = i32::try_from(&child).unwrap_or(0);
            uia = match crate::uia_snapshot::capture::automation() {
                // SAFETY: a UIA read on a valid automation object.
                Ok(automation) => match unsafe { automation.ElementFromIAccessible(acc, child_id) }
                {
                    Ok(element) => uia_json(&element),
                    Err(e) => serde_json::json!({ "error": e.to_string() }),
                },
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            };
        }
        let uia_focus = crate::uia_snapshot::capture::automation()
            .ok()
            .map(|automation| crate::uia_snapshot::capture::focus_chain(&automation))
            .and_then(|chain| chain.last().cloned())
            .map(|n| serde_json::json!({ "name": n.name, "class_name": n.class_name }));
        // Office's own UIA tree below the window: the element with the keyboard focus, or
        // else the first one with the MSAA object's name.
        let msaa_name = msaa.get("name").and_then(|n| n.as_str()).map(str::to_owned);
        let uia_native = crate::uia_snapshot::capture::automation()
            .ok()
            .and_then(|automation| {
                // SAFETY: UIA reads on a valid automation object and its elements.
                unsafe {
                    let root = automation.ElementFromHandle(window).ok()?;
                    let focused = automation
                        .CreatePropertyCondition(
                            UIA_HasKeyboardFocusPropertyId,
                            &VARIANT::from(true),
                        )
                        .ok()?;
                    let clock = std::time::Instant::now();
                    let by_focus = root.FindFirst(TreeScope_Descendants, &focused).ok();
                    let search_ms = clock.elapsed().as_millis();
                    let by_name = || {
                        let name = msaa_name.as_deref()?;
                        let condition = automation
                            .CreatePropertyCondition(
                                UIA_NamePropertyId,
                                &VARIANT::from(windows::core::BSTR::from(name)),
                            )
                            .ok()?;
                        root.FindFirst(TreeScope_Descendants, &condition).ok()
                    };
                    let (how, element) = match by_focus {
                        Some(e) => ("keyboard focus", e),
                        None => ("name", by_name()?),
                    };
                    let mut json = uia_json(&element);
                    if let Some(map) = json.as_object_mut() {
                        map.insert("found_by".into(), how.into());
                        map.insert(
                            "search_ms".into(),
                            u64::try_from(search_ms).unwrap_or(u64::MAX).into(),
                        );
                        // The same element from the point at the MSAA object's center.
                        if let Some(center) = msaa_center {
                            let clock = std::time::Instant::now();
                            let at = automation.ElementFromPoint(center).ok();
                            let point_ms = clock.elapsed().as_millis();
                            let same = at
                                .as_ref()
                                .and_then(|a| automation.CompareElements(a, &element).ok())
                                .is_some_and(|b| b.as_bool());
                            map.insert(
                                "point_ms".into(),
                                u64::try_from(point_ms).unwrap_or(u64::MAX).into(),
                            );
                            map.insert("point_same".into(), same.into());
                        }
                    }
                    Some(json)
                }
            });
        serde_json::json!({
            "msaa": msaa,
            "uia_of_msaa": uia,
            "uia_native": uia_native,
            "uia_focus": uia_focus,
        })
        .to_string()
    }

    /// What a UIA element says beside its name, for [`msaa_focus_probe`].
    fn uia_json(
        element: &windows::Win32::UI::Accessibility::IUIAutomationElement,
    ) -> serde_json::Value {
        use windows::Win32::UI::Accessibility::{
            IUIAutomationExpandCollapsePattern, IUIAutomationTogglePattern,
            UIA_ExpandCollapsePatternId, UIA_FullDescriptionPropertyId, UIA_TogglePatternId,
        };
        let text = |b: windows::core::Result<windows::core::BSTR>| {
            b.ok()
                .map(|b| b.to_string())
                .filter(|t| !t.is_empty())
                .map_or(serde_json::Value::Null, Into::into)
        };
        // SAFETY: UIA reads on a valid element.
        unsafe {
            let toggle = element
                .GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
                .ok()
                .and_then(|p| p.CurrentToggleState().ok())
                .map(|s| s.0);
            let expand = element
                .GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(
                    UIA_ExpandCollapsePatternId,
                )
                .ok()
                .and_then(|p| p.CurrentExpandCollapseState().ok())
                .map(|s| s.0);
            let full = element
                .GetCurrentPropertyValue(UIA_FullDescriptionPropertyId)
                .ok()
                .and_then(|v| windows::core::BSTR::try_from(&v).ok())
                .map(|b| b.to_string())
                .filter(|t| !t.is_empty());
            serde_json::json!({
                "name": text(element.CurrentName()),
                "control_type": element.CurrentControlType().ok().map(|c| c.0),
                "localized_control_type": text(element.CurrentLocalizedControlType()),
                "class_name": text(element.CurrentClassName()),
                "help_text": text(element.CurrentHelpText()),
                "full_description": full,
                "item_status": text(element.CurrentItemStatus()),
                "accelerator_key": text(element.CurrentAcceleratorKey()),
                "access_key": text(element.CurrentAccessKey()),
                "toggle_state": toggle,
                "expand_collapse_state": expand,
            })
        }
    }

    /// The top-level window titled exactly `title` (a scenario's fixture), wherever the focus
    /// is.
    pub fn window_titled(title: &str) -> Option<windows::Win32::Foundation::HWND> {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
        // SAFETY: a plain Win32 lookup by title.
        unsafe { FindWindowW(PCWSTR::null(), &HSTRING::from(title)) }.ok()
    }

    /// The File Explorer window titled `title` (a folder's name), wherever the focus is.
    pub fn shell_window(title: &str) -> Option<windows::Win32::Foundation::HWND> {
        use windows::core::{w, HSTRING};
        use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
        // SAFETY: a plain Win32 lookup by class and title.
        unsafe { FindWindowW(w!("CabinetWClass"), &HSTRING::from(title)) }.ok()
    }

    /// The foreground window's title.
    pub fn foreground_title() -> String {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
        let mut buf = [0u16; 512];
        // SAFETY: plain Win32 queries into a buffer of the given size.
        let len = unsafe { GetWindowTextW(GetForegroundWindow(), &mut buf) };
        let len = usize::try_from(len).unwrap_or(0);
        String::from_utf16_lossy(buf.get(..len).unwrap_or_default())
    }

    /// The held keys (`HELD_KEYS`) Windows has down now, by name.
    fn keys_down() -> Vec<(u16, &'static str)> {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        HELD_KEYS
            .into_iter()
            // SAFETY: a key-state query with no preconditions.
            .filter(|&(vk, _)| unsafe { GetAsyncKeyState(i32::from(vk)) } < 0)
            .collect()
    }

    /// Releases the held keys still down, and names those that stay down.
    fn release_keys_down() -> Vec<&'static str> {
        let down = keys_down();
        down.iter().for_each(|&(vk, _)| key(vk, true));
        if down.is_empty() {
            return Vec::new();
        }
        keys_down().into_iter().map(|(_, name)| name).collect()
    }

    /// Sends `steps`, each only while the foreground application is one of `allowed`
    /// (process names, case-insensitive) and, with `title`, while the foreground window's
    /// title contains it: one process can own both the user's windows and the test's
    /// (Windows Terminal). A held key is released before a refusal returns.
    ///
    /// Windows drops keys sent while a window that runs elevated is in front (JAWS's own
    /// dialogs, under UIPI) without saying so, and hides the key state from us meanwhile.
    /// When a chord opened such a window between its key events, the key-ups after it were
    /// lost, and a modifier stayed down for every application, the user's typing included
    /// (observed on the dev box, 2026-10-17: Shift stuck after Ctrl+Shift+Insert+Down opened
    /// JAWS's Skim Reading dialog, and the next Escape opened Chrome's Task Manager). So a
    /// chord is sent in one batch; the keys still down are released before the first step,
    /// once an allowed window is in front; and they are checked after the last, unless a
    /// higher window in front hides them: one still down is an error.
    pub fn send_steps(
        steps: &[KeyStep],
        allowed: &[String],
        title: Option<&str>,
    ) -> Result<(), String> {
        let mut held: Vec<u16> = Vec::new();
        let check = || {
            let process = foreground_process();
            if !allowed.iter().any(|a| a.eq_ignore_ascii_case(&process)) {
                return Err(format!("refused: {process:?} is in front"));
            }
            if let Some(title) = title {
                let current = foreground_title();
                if !current.to_lowercase().contains(&title.to_lowercase()) {
                    return Err(format!("refused: the window in front is {current:?}"));
                }
            }
            Ok(())
        };
        if check().is_ok() {
            let released: Vec<_> = keys_down().into_iter().map(|(_, name)| name).collect();
            if !released.is_empty() {
                let still = release_keys_down();
                eprintln!(
                    "released {}, down before the first key (a lost key-up?){}",
                    released.join(", "),
                    if still.is_empty() {
                        String::new()
                    } else {
                        format!("; still down: {}", still.join(", "))
                    }
                );
            }
        }
        let result: Result<(), String> = steps.iter().try_for_each(|step| match step {
            KeyStep::Chord(keys) => {
                check()?;
                chord(keys);
                Ok(())
            }
            KeyStep::Down(k) => {
                check()?;
                key(*k, false);
                held.push(*k);
                Ok(())
            }
            KeyStep::Up(k) => {
                key(*k, true);
                held.retain(|h| h != k);
                Ok(())
            }
            KeyStep::Wait(ms) => {
                std::thread::sleep(Duration::from_millis(*ms));
                Ok(())
            }
        });
        for k in held.into_iter().rev() {
            key(k, true);
        }
        result?;
        if foreground_is_higher() {
            eprintln!(
                "note: {:?} runs at a higher integrity level, so Windows hides the key state; \
                 the next `okur-lab keys` with a normal window in front releases any key left down",
                foreground_title()
            );
            return Ok(());
        }
        let still = release_keys_down();
        if still.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} still down after the keys, with {:?} in front",
                still.join(", "),
                foreground_title()
            ))
        }
    }

    /// JAWS's speech history, read by asking JAWS to copy it to the clipboard (its FSAPI
    /// `RunScript`); the clipboard's text is put back afterwards. `clear` empties the history
    /// after reading, so the next read holds only what follows.
    pub fn jaws_history(clear: bool) -> Result<String, String> {
        let saved = crate::actions::clipboard_text();
        let sentinel = format!("__sr_history_{}__", std::process::id());
        crate::actions::set_clipboard_text(&sentinel)?;
        let jaws = Fsapi::connect()?;
        jaws.run_script("CopySpeechHistoryToClipboard")?;
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut history = None;
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
            if let Some(text) = crate::actions::clipboard_text().filter(|t| *t != sentinel) {
                history = Some(text);
                break;
            }
        }
        if clear {
            jaws.run_script("ClearSpeechHistory")?;
        }
        crate::actions::set_clipboard_text(saved.as_deref().unwrap_or_default())?;
        // JAWS leaves the clipboard alone when its history is empty.
        Ok(history.unwrap_or_default())
    }

    /// JAWS's FSAPI automation object (`FreedomSci.JawsApi`), called through IDispatch.
    struct Fsapi(windows::Win32::System::Com::IDispatch);

    impl Fsapi {
        fn connect() -> Result<Self, String> {
            use windows::Win32::System::Com::{
                CLSIDFromProgID, CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER,
                CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED,
            };
            // SAFETY: COM initialization and activation of a registered automation object.
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                let clsid = CLSIDFromProgID(windows::core::w!("FreedomSci.JawsApi"))
                    .map_err(|e| format!("FSAPI is not registered (is JAWS installed?): {e}"))?;
                CoCreateInstance(&clsid, None, CLSCTX_INPROC_SERVER | CLSCTX_LOCAL_SERVER)
                    .map(Self)
                    .map_err(|e| format!("FSAPI: {e}"))
            }
        }

        fn run_script(&self, name: &str) -> Result<(), String> {
            use windows::core::{BSTR, GUID, PCWSTR};
            use windows::Win32::System::Com::{DISPATCH_METHOD, DISPPARAMS};
            use windows::Win32::System::Variant::VARIANT;
            let method: Vec<u16> = "RunScript\0".encode_utf16().collect();
            let names = [PCWSTR(method.as_ptr())];
            let mut id = 0i32;
            // SAFETY: IDispatch calls with valid name, argument and result buffers that
            // outlive them.
            unsafe {
                self.0
                    .GetIDsOfNames(&GUID::zeroed(), names.as_ptr(), 1, 0, &mut id)
                    .map_err(|e| format!("FSAPI RunScript: {e}"))?;
                let mut args = [VARIANT::from(BSTR::from(name))];
                let params = DISPPARAMS {
                    rgvarg: args.as_mut_ptr(),
                    cArgs: 1,
                    ..Default::default()
                };
                let mut result = VARIANT::default();
                self.0
                    .Invoke(
                        id,
                        &GUID::zeroed(),
                        0,
                        DISPATCH_METHOD,
                        &params,
                        Some(&mut result),
                        None,
                        None,
                    )
                    .map_err(|e| format!("FSAPI RunScript({name}): {e}"))?;
                // RunScript answers false when JAWS is not running or has no such script.
                if bool::try_from(&result).unwrap_or(true) {
                    Ok(())
                } else {
                    Err(format!("JAWS did not run {name} (is JAWS running?)"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_parse_as_scenarios_write_keys() {
        assert_eq!(
            parse_step("Insert+Alt+W"),
            Ok(KeyStep::Chord(vec![0x2D, 0x12, 0x57]))
        );
        assert_eq!(parse_step("down:Alt"), Ok(KeyStep::Down(0x12)));
        assert_eq!(parse_step("up:alt"), Ok(KeyStep::Up(0x12)));
        assert_eq!(parse_step("wait:500"), Ok(KeyStep::Wait(500)));
        assert_eq!(
            parse_step("CapsLock+F12"),
            Ok(KeyStep::Chord(vec![0x14, 0x7B]))
        );
        assert!(parse_step("Hyper+Q").is_err());
        assert!(is_extended(0x2D) && is_extended(0x28) && !is_extended(0x12));
        // The right Ctrl and Alt are extended keys; releasing a stuck one needs the flag.
        assert!(is_extended(0xA3) && is_extended(0xA5) && !is_extended(0xA2));
        assert_eq!(parse_step("NumpadAdd"), Ok(KeyStep::Chord(vec![0x6B])));
        assert_eq!(
            parse_step("Insert+NumpadMinus"),
            Ok(KeyStep::Chord(vec![0x2D, 0x6D]))
        );
        assert_eq!(parse_step("NumpadSlash"), Ok(KeyStep::Chord(vec![0x6F])));
        // As with Num Lock off: Numpad 7 is Home and Numpad 5 Clear, neither extended.
        assert_eq!(
            parse_step("Numpad7"),
            Ok(KeyStep::Chord(vec![KEYPAD | 0x24]))
        );
        assert_eq!(
            parse_step("Insert+Numpad5"),
            Ok(KeyStep::Chord(vec![0x2D, KEYPAD | 0x0C]))
        );
        assert!(!is_extended(KEYPAD | 0x24) && is_extended(0x24));
        assert!(parse_step("NumpadX").is_err());
    }
}
