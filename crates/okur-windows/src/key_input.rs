//! Keys Okur sends itself: the screen reader's own chords (a reading command's arrow key, the
//! menu mask after a Windows key) and the harness's. Kept apart from the harness's tools
//! (`crate::harness`), which `okur.exe` does not use (AGENTS.md §6).

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

#[cfg(windows)]
pub use windows_input::{send_chord, send_key};

#[cfg(windows)]
mod windows_input {
    use super::is_extended;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, VIRTUAL_KEY,
    };

    /// Presses `keys` in order and releases them in reverse, all in one `SendInput` call.
    /// Windows queues a call's events together, so every key-up is in the queue before the
    /// chord's command can open a window that would refuse it.
    pub fn send_chord(keys: &[u16]) {
        let inputs: Vec<INPUT> = keys
            .iter()
            .map(|&k| key_input(k, false, 0))
            .chain(keys.iter().rev().map(|&k| key_input(k, true, 0)))
            .collect();
        send_inputs(&inputs);
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
}
