use super::{HookCallback, Key, KeyAction, KeyEvent, KeyboardHook, Modifiers};
use crate::PlatformError;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, VIRTUAL_KEY, VK_ADD, VK_BACK, VK_CAPITAL, VK_CLEAR, VK_CONTROL,
    VK_DELETE, VK_DIVIDE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT,
    VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_MULTIPLY, VK_NEXT, VK_OEM_1, VK_OEM_7, VK_PRIOR,
    VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SPACE,
    VK_SUBTRACT, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static ACTIVE_CALLBACK: Mutex<Option<HookCallback>> = Mutex::new(None);
static CONSUMED_KEYS: Mutex<Option<HashSet<u32>>> = Mutex::new(None);
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);
/// Whether the keys that can be the JAWS key (Insert, CapsLock, ScrollLock) are held. Tracked
/// here because the engine consumes the JAWS key, so the system's asynchronous key state may
/// not see it.
static INSERT_HELD: AtomicBool = AtomicBool::new(false);
static CAPS_LOCK_HELD: AtomicBool = AtomicBool::new(false);
static SCROLL_LOCK_HELD: AtomicBool = AtomicBool::new(false);
/// Modifiers whose press the engine consumed, as [`modifier_bit`]s. A modifier pressed while
/// the JAWS key is held (Insert+Alt+W) is consumed, so the application never sees it (Alt
/// alone would open its menu bar). The system's key state then says it is up, so it is held
/// here until its release arrives.
static CONSUMED_MODIFIERS: AtomicU32 = AtomicU32::new(0);
/// `dwExtraInfo` of the keys the screen reader sends itself ("OKUR").
pub const INJECTED_MARK: usize = 0x4F4B_5552;
/// The last keys pressed that the engine saw, as `(virtual key, hook time in ms)`. JAWS,
/// running alongside, takes navigation keys and sends its own copies (injected, marked)
/// after the whole chord (Ctrl, Right, then Ctrl's and Right's copies).
static RECENT_KEYS: Mutex<[(u32, u32); 4]> = Mutex::new([(0, 0); 4]);
/// How long after a key JAWS's copy of it can come (about 5 ms on the dev box).
const COPY_WINDOW_MS: u32 = 50;
/// How long after a key a copy with JAWS's known mark can come: JAWS runs a script for some
/// keys first (Ctrl+Home's copy came 230 ms later, dev box).
const MARKED_COPY_WINDOW_MS: u32 = 1000;
/// JAWS's `dwExtraInfo`, learned from its first copy that came within [`COPY_WINDOW_MS`].
static JAWS_MARK: AtomicUsize = AtomicUsize::new(0);
/// `KBDLLHOOKSTRUCT::flags`: the event was injected.
const LLKHF_INJECTED: u32 = 0x10;
/// `KBDLLHOOKSTRUCT::flags`: an extended key (the gray arrows, not the keypad's).
const LLKHF_EXTENDED: u32 = 0x01;
const SHIFT_BIT: u32 = 1;
const CTRL_BIT: u32 = 2;
const ALT_BIT: u32 = 4;
const WIN_BIT: u32 = 8;

/// Whether `vk` was pressed within `window` ms before `time`: the press is then forgotten,
/// as each original has one copy.
fn take_recent(vk: u32, time: u32, window: u32) -> bool {
    RECENT_KEYS.lock().is_ok_and(|mut recent| {
        recent
            .iter_mut()
            .find(|(key, at)| *key == vk && time.wrapping_sub(*at) <= window)
            .map(|entry| *entry = (0, 0))
            .is_some()
    })
}

/// The modifier a virtual key is, either side, as a [`CONSUMED_MODIFIERS`] bit (0 for others).
fn modifier_bit(vk: u32) -> u32 {
    match vk {
        0x10 | 0xA0 | 0xA1 => SHIFT_BIT,
        0x11 | 0xA2 | 0xA3 => CTRL_BIT,
        0x12 | 0xA4 | 0xA5 => ALT_BIT,
        0x5B | 0x5C => WIN_BIT,
        _ => 0,
    }
}

pub struct WindowsKeyboardHook {
    running: ArcBool,
    thread_handle: Option<JoinHandle<()>>,
}

type ArcBool = std::sync::Arc<AtomicBool>;

impl WindowsKeyboardHook {
    pub fn new() -> Self {
        Self {
            running: std::sync::Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }
}

impl Default for WindowsKeyboardHook {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardHook for WindowsKeyboardHook {
    fn start(&mut self, callback: HookCallback) -> Result<(), PlatformError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        if let Ok(mut cb) = ACTIVE_CALLBACK.lock() {
            *cb = Some(callback);
        }
        if let Ok(mut consumed) = CONSUMED_KEYS.lock() {
            *consumed = Some(HashSet::new());
        }

        self.running.store(true, Ordering::SeqCst);
        let running_clone = self.running.clone();

        let (init_tx, init_rx) = std::sync::mpsc::channel();

        #[link(name = "user32")]
        extern "system" {
            fn OpenDesktopW(
                lpszDesktop: *const u16,
                dwFlags: u32,
                fInherit: i32,
                dwDesiredAccess: u32,
            ) -> *mut std::ffi::c_void;
            fn SetThreadDesktop(hDesktop: *mut std::ffi::c_void) -> i32;
        }

        let handle = thread::spawn(move || {
            // Attach thread to interactive user desktop
            unsafe {
                let default_name = windows::core::w!("default");
                let h = OpenDesktopW(default_name.as_ptr(), 0, 0, 0x01FF);
                if !h.is_null() {
                    let _ = SetThreadDesktop(h);
                }
            }

            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

            let hook = unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), None, 0)
            };

            let hook_handle = match hook {
                Ok(h) => {
                    let tid = unsafe { GetCurrentThreadId() };
                    HOOK_THREAD_ID.store(tid, Ordering::SeqCst);
                    let _ = init_tx.send(Ok(()));
                    h
                }
                Err(e) => {
                    let _ = init_tx.send(Err(PlatformError::HookError(e.to_string())));
                    running_clone.store(false, Ordering::SeqCst);
                    return;
                }
            };

            tracing::info!("Windows low-level keyboard hook installed successfully");

            // Windows message pump for the hook thread
            unsafe {
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                let _ = UnhookWindowsHookEx(hook_handle);
                HOOK_THREAD_ID.store(0, Ordering::SeqCst);
                CoUninitialize();
            }

            tracing::info!("Windows low-level keyboard hook uninstalled");
        });

        match init_rx.recv() {
            Ok(Ok(())) => {
                self.thread_handle = Some(handle);
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(PlatformError::HookError(
                "Failed to initialize hook thread".into(),
            )),
        }
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(false, Ordering::SeqCst);

        let tid = HOOK_THREAD_ID.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        if let Ok(mut cb) = ACTIVE_CALLBACK.lock() {
            *cb = None;
        }
        if let Ok(mut consumed) = CONSUMED_KEYS.lock() {
            *consumed = None;
        }

        Ok(())
    }

    fn is_active(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for WindowsKeyboardHook {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Win32 low-level keyboard callback
unsafe extern "system" fn low_level_keyboard_proc(
    ncode: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if ncode < 0 {
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);
    let msg_type = wparam.0 as u32;

    let is_down = msg_type == WM_KEYDOWN || msg_type == WM_SYSKEYDOWN;
    let is_up = msg_type == WM_KEYUP || msg_type == WM_SYSKEYUP;

    if !is_down && !is_up {
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    let vk_code = kbd.vkCode;
    // A down event for a key already held is the keyboard's auto-repeat: holding CapsLock
    // as the JAWS key must not count as pressing it twice.
    let mut is_repeat = false;
    let held_flag = match vk_code {
        v if v == u32::from(VK_INSERT.0) => Some(&INSERT_HELD),
        v if v == u32::from(VK_CAPITAL.0) => Some(&CAPS_LOCK_HELD),
        v if v == u32::from(VK_SCROLL.0) => Some(&SCROLL_LOCK_HELD),
        _ => None,
    };
    if let Some(flag) = held_flag {
        is_repeat = is_down && flag.swap(is_down, Ordering::Relaxed);
        if !is_down {
            flag.store(false, Ordering::Relaxed);
        }
    }

    // If key up was previously consumed on key down, consume key up too
    if is_up {
        CONSUMED_MODIFIERS.fetch_and(!modifier_bit(vk_code), Ordering::Relaxed);
        if let Ok(mut consumed_guard) = CONSUMED_KEYS.lock() {
            if let Some(ref mut set) = *consumed_guard {
                if set.remove(&vk_code) {
                    return LRESULT(1);
                }
            }
        }
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    // Determine modifier state: the system's, and the modifiers it never saw pressed. Keys
    // the screen reader sends itself (a reading command's arrow key, sent while the JAWS key
    // is still held) are plain keys: only the modifiers they carry count.
    // JAWS, running alongside, takes navigation keys and sends its own copies: injected, with
    // its own nonzero `dwExtraInfo` (0x45789400 on the dev box), a few milliseconds after the
    // original. A copy goes on to the application without the engine seeing the key twice
    // (an edit key read the field twice, a key at a boundary spoke twice), and without
    // reading as a JAWS chord while the JAWS key is still held (both seen live on the dev box).
    // Keys other tools inject (an on-screen keyboard, voice control) carry no mark, so a
    // letter they type twice is never taken for a copy.
    let marked = kbd.dwExtraInfo == INJECTED_MARK;
    let foreign_mark = kbd.dwExtraInfo != 0 && !marked;
    if foreign_mark && kbd.flags.0 & LLKHF_INJECTED != 0 {
        let jaws_mark = JAWS_MARK.load(Ordering::Relaxed);
        let window = if jaws_mark != 0 && kbd.dwExtraInfo == jaws_mark {
            MARKED_COPY_WINDOW_MS
        } else {
            COPY_WINDOW_MS
        };
        if take_recent(vk_code, kbd.time, window) {
            if jaws_mark == 0 {
                JAWS_MARK.store(kbd.dwExtraInfo, Ordering::Relaxed);
            }
            return CallNextHookEx(None, ncode, wparam, lparam);
        }
    }
    if let Ok(mut recent) = RECENT_KEYS.lock() {
        recent.rotate_right(1);
        if let Some(first) = recent.first_mut() {
            *first = (vk_code, kbd.time);
        }
    }
    let ours = marked;
    let consumed = if ours {
        0
    } else {
        CONSUMED_MODIFIERS.load(Ordering::Relaxed)
    };
    let ctrl =
        (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 || consumed & CTRL_BIT != 0;
    let alt = (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0 || consumed & ALT_BIT != 0;
    let shift =
        (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0 || consumed & SHIFT_BIT != 0;
    let win = ((GetAsyncKeyState(0x5B) as u16 & 0x8000) != 0) // VK_LWIN
        || ((GetAsyncKeyState(0x5C) as u16 & 0x8000) != 0) // VK_RWIN
        || consumed & WIN_BIT != 0;
    let modifiers = Modifiers {
        ctrl,
        alt,
        shift,
        win,
        insert: !ours && INSERT_HELD.load(Ordering::Relaxed),
        caps_lock: !ours && CAPS_LOCK_HELD.load(Ordering::Relaxed),
        scroll_lock: !ours && SCROLL_LOCK_HELD.load(Ordering::Relaxed),
    };
    let insert = modifiers.insert || modifiers.caps_lock || modifiers.scroll_lock;

    let key = translate_vk_code(vk_code, kbd.scanCode, kbd.flags.0 & LLKHF_EXTENDED != 0);
    let mut event = KeyEvent::new(key, modifiers, is_down);
    event.time_ms = u64::from(kbd.time);
    event.is_repeat = is_repeat;
    event.caps_lock_on = Some(GetKeyState(i32::from(VK_CAPITAL.0)) & 1 != 0);
    if !ctrl && !alt && !win && !insert {
        event.text = typed_char(vk_code, kbd.scanCode, shift);
    }

    let mut action = KeyAction::PassThrough;
    if let Ok(mut cb_guard) = ACTIVE_CALLBACK.lock() {
        if let Some(ref mut cb) = *cb_guard {
            action = cb(event);
        }
    }

    if action == KeyAction::Consume {
        CONSUMED_MODIFIERS.fetch_or(modifier_bit(vk_code), Ordering::Relaxed);
        if let Ok(mut consumed_guard) = CONSUMED_KEYS.lock() {
            if let Some(ref mut set) = *consumed_guard {
                set.insert(vk_code);
            }
        }
        LRESULT(1)
    } else {
        CallNextHookEx(None, ncode, wparam, lparam)
    }
}

/// The character a key press types in the foreground window's keyboard layout, for typing
/// echo. Flag 0x4 keeps `ToUnicodeEx` from changing the keyboard state, so dead keys still
/// compose in the application.
fn typed_char(vk: u32, scan: u32, shift: bool) -> Option<char> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayout, ToUnicodeEx};
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    let mut state = [0u8; 256];
    if shift {
        state[0x10] = 0x80; // VK_SHIFT
    }
    // SAFETY: plain Win32 state and layout queries with valid arguments.
    unsafe {
        if GetKeyState(0x14) & 1 != 0 {
            state[0x14] = 0x01; // VK_CAPITAL toggled
        }
        let thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let layout = GetKeyboardLayout(thread);
        let mut buf = [0u16; 8];
        let n = ToUnicodeEx(vk, scan, &state, &mut buf, 0x4, Some(layout));
        if n != 1 {
            return None;
        }
        char::from_u32(u32::from(buf[0])).filter(|c| !c.is_control())
    }
}

fn translate_vk_code(vk: u32, scan: u32, extended: bool) -> Key {
    // The keys at the US comma, period, minus, equals, slash and backslash places, by scan code
    // as JAWS takes them: on old-pc's Turkish Q layout they carry other virtual keys (ö, ç).
    // The keypad's slash is the slash place's scan code, extended.
    match (scan, extended) {
        (0x33, false) => return Key::Comma,
        (0x34, false) => return Key::Period,
        (0x0C, false) => return Key::Minus,
        (0x0D, false) => return Key::Equals,
        (0x35, false) => return Key::Slash,
        (0x2B, false) => return Key::Backslash,
        _ => {}
    }
    match VIRTUAL_KEY(vk as u16) {
        // The keypad's 5 with Num Lock off.
        VK_CLEAR => Key::Numpad5,
        VK_ADD => Key::NumpadPlus,
        VK_SUBTRACT => Key::NumpadMinus,
        VK_DIVIDE => Key::NumpadSlash,
        VK_MULTIPLY => Key::NumpadStar,
        VK_UP => Key::UpArrow,
        VK_DOWN => Key::DownArrow,
        VK_LEFT => Key::LeftArrow,
        VK_RIGHT => Key::RightArrow,

        VK_HOME => Key::Home,
        VK_END => Key::End,
        VK_PRIOR => Key::PageUp,
        VK_NEXT => Key::PageDown,

        VK_TAB => Key::Tab,
        VK_RETURN => Key::Enter,
        VK_ESCAPE => Key::Escape,
        VK_SPACE => Key::Space,
        VK_BACK => Key::Backspace,
        VK_DELETE => Key::Delete,
        VK_INSERT => Key::Insert,
        VK_CAPITAL => Key::CapsLock,
        VK_SCROLL => Key::ScrollLock,

        // Physical-key bindings, as JAWS does: the key at the US ";" and "'" positions,
        // whatever character the active layout prints on it.
        VK_OEM_1 => Key::Semicolon,
        VK_OEM_7 => Key::Apostrophe,

        // A low-level hook reports the modifiers by side (VK_LCONTROL, VK_RSHIFT, ...): Ctrl
        // alone, which stops speech and Say All, came as an unknown key (old-pc, 2026-10-19).
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Key::Control,
        VK_MENU | VK_LMENU | VK_RMENU => Key::Alt,
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => Key::Shift,
        VK_LWIN | VK_RWIN => Key::Windows,

        _ => {
            // Check F1..F24 (0x70..=0x87)
            if (0x70..=0x87).contains(&vk) {
                Key::F((vk - 0x70 + 1) as u8)
            }
            // Check '0'..='9'
            else if (0x30..=0x39).contains(&vk) {
                Key::Digit((vk - 0x30) as u8)
            }
            // Check 'A'..='Z'
            else if (0x41..=0x5A).contains(&vk) {
                Key::Alpha((vk as u8) as char)
            } else {
                Key::Other(vk)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_on_either_side_share_a_bit() {
        assert_eq!(modifier_bit(0xA4), ALT_BIT);
        assert_eq!(modifier_bit(0xA5), ALT_BIT);
        assert_eq!(modifier_bit(0x12), ALT_BIT);
        assert_eq!(modifier_bit(0xA1), SHIFT_BIT);
        assert_eq!(modifier_bit(0xA2), CTRL_BIT);
        assert_eq!(modifier_bit(0x5C), WIN_BIT);
        assert_eq!(modifier_bit(u32::from('W')), 0);
    }
}
