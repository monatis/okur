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

/// Marks a key of the numeric keypad that shares its virtual key with a gray key (Numpad 4 and
/// Left): sent without the extended flag. Above every virtual key.
pub const KEYPAD: u16 = 0x1000;

/// Keys that need the extended-key flag (the navigation block, Insert, the Windows keys, the
/// numeric keypad's divide, the right Ctrl and Alt); never a keypad key.
pub fn is_extended(vk: u16) -> bool {
    if vk & KEYPAD != 0 {
        return false;
    }
    matches!(
        vk,
        0x21..=0x28 | 0x2D | 0x2E | 0x5B | 0x5C | 0x6F | 0xA3 | 0xA5
    )
}

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
pub use windows_harness::{
    bring_to_front, foreground_process, foreground_title, jaws_history, send_key, send_steps,
    shell_window, window_titled,
};

#[cfg(windows)]
mod windows_harness {
    use super::{is_extended, KeyStep, HELD_KEYS};
    use std::time::{Duration, Instant};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, VIRTUAL_KEY,
    };

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

    /// Presses `keys` in order and releases them in reverse, all in one `SendInput` call.
    /// Windows queues a call's events together, so every key-up is in the queue before the
    /// chord's command can open a window that would refuse it.
    fn chord(keys: &[u16]) {
        let inputs: Vec<INPUT> = keys
            .iter()
            .map(|&k| key_input(k, false, 0))
            .chain(keys.iter().rev().map(|&k| key_input(k, true, 0)))
            .collect();
        send_inputs(&inputs);
        std::thread::sleep(KEY_GAP);
    }

    /// Sends one key event, scan-coded, with `extra` as its `dwExtraInfo`.
    pub fn send_key(vk: u16, up: bool, extra: usize) {
        send_inputs(&[key_input(vk, up, extra)]);
    }

    fn key_input(key: u16, up: bool, extra: usize) -> INPUT {
        let vk = key & !super::KEYPAD;
        // SAFETY: a scan-code lookup with no preconditions.
        let scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) };
        let mut flags = KEYBD_EVENT_FLAGS(0);
        if is_extended(key) {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        if up {
            flags |= KEYEVENTF_KEYUP;
        }
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: u16::try_from(scan).unwrap_or(0),
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: extra,
                },
            },
        }
    }

    fn send_inputs(inputs: &[INPUT]) {
        let size = i32::try_from(std::mem::size_of::<INPUT>()).unwrap_or(0);
        // SAFETY: well-formed keyboard inputs.
        unsafe { SendInput(inputs, size) };
    }

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
