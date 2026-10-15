//! Harness tools for live checks on a machine the user lets the agent drive (`sr-cli keys`,
//! `sr-cli jaws-history`): key chords sent only while an allowed application is in front,
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

/// Keys that need the extended-key flag (the navigation block, Insert, the Windows key).
pub fn is_extended(vk: u16) -> bool {
    matches!(vk, 0x21..=0x28 | 0x2D | 0x2E | 0x5B | 0x5C)
}

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
pub use windows_harness::{foreground_process, jaws_history, send_key, send_steps};

#[cfg(windows)]
mod windows_harness {
    use super::{is_extended, KeyStep};
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

    /// Sends one key event, scan-coded, with `extra` as its `dwExtraInfo`.
    pub fn send_key(vk: u16, up: bool, extra: usize) {
        // SAFETY: a scan-code lookup with no preconditions.
        let scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) };
        let mut flags = KEYBD_EVENT_FLAGS(0);
        if is_extended(vk) {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        if up {
            flags |= KEYEVENTF_KEYUP;
        }
        let input = INPUT {
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
        };
        let size = i32::try_from(std::mem::size_of::<INPUT>()).unwrap_or(0);
        // SAFETY: one well-formed keyboard input.
        unsafe { SendInput(&[input], size) };
    }

    /// Sends `steps`, each only while the foreground application is one of `allowed`
    /// (process names, case-insensitive). A held key is released before a refusal returns.
    pub fn send_steps(steps: &[KeyStep], allowed: &[String]) -> Result<(), String> {
        let mut held: Vec<u16> = Vec::new();
        let check = || {
            let process = foreground_process();
            if allowed.iter().any(|a| a.eq_ignore_ascii_case(&process)) {
                Ok(())
            } else {
                Err(format!("refused: {process:?} is in front"))
            }
        };
        let result = steps.iter().try_for_each(|step| match step {
            KeyStep::Chord(keys) => {
                check()?;
                keys.iter().for_each(|&k| key(k, false));
                keys.iter().rev().for_each(|&k| key(k, true));
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
        result
    }

    /// JAWS's speech history, read by asking JAWS to copy it to the clipboard (its FSAPI
    /// `RunScript`); the clipboard's text is put back afterwards. `clear` empties the history
    /// after reading, so the next read holds only what follows.
    pub fn jaws_history(clear: bool) -> Result<String, String> {
        let saved = clipboard_text();
        let sentinel = format!("__sr_history_{}__", std::process::id());
        crate::actions::set_clipboard_text(&sentinel)?;
        let jaws = Fsapi::connect()?;
        jaws.run_script("CopySpeechHistoryToClipboard")?;
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut history = None;
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
            if let Some(text) = clipboard_text().filter(|t| *t != sentinel) {
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

    /// The clipboard's Unicode text, if it holds any.
    fn clipboard_text() -> Option<String> {
        use windows::Win32::Foundation::HGLOBAL;
        use windows::Win32::System::DataExchange::{
            CloseClipboard, GetClipboardData, OpenClipboard,
        };
        use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
        const CF_UNICODETEXT: u32 = 13;
        // SAFETY: the clipboard is opened and closed here; the data is only read while locked,
        // up to its terminating zero.
        unsafe {
            OpenClipboard(None).ok()?;
            let text = GetClipboardData(CF_UNICODETEXT).ok().and_then(|data| {
                let block = HGLOBAL(data.0);
                let ptr = GlobalLock(block).cast::<u16>();
                if ptr.is_null() {
                    return None;
                }
                let mut len = 0usize;
                while *ptr.add(len) != 0 {
                    len += 1;
                }
                let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
                let _ = GlobalUnlock(block);
                Some(text)
            });
            let _ = CloseClipboard();
            text
        }
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
    }
}
