use super::{HookCallback, Key, KeyAction, KeyEvent, KeyboardHook, Modifiers};
use crate::PlatformError;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE,
    VK_HOME, VK_INSERT, VK_LEFT, VK_MENU, VK_NEXT, VK_OEM_1, VK_PRIOR, VK_RETURN, VK_RIGHT,
    VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static ACTIVE_CALLBACK: Mutex<Option<HookCallback>> = Mutex::new(None);
static CONSUMED_KEYS: Mutex<Option<HashSet<u32>>> = Mutex::new(None);
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

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

        {
            let mut cb = ACTIVE_CALLBACK.lock().unwrap();
            *cb = Some(callback);
        }
        {
            let mut consumed = CONSUMED_KEYS.lock().unwrap();
            *consumed = Some(HashSet::new());
        }

        self.running.store(true, Ordering::SeqCst);
        let running_clone = self.running.clone();

        let (init_tx, init_rx) = std::sync::mpsc::channel();

        let handle = thread::spawn(move || {
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

        {
            let mut cb = ACTIVE_CALLBACK.lock().unwrap();
            *cb = None;
        }
        {
            let mut consumed = CONSUMED_KEYS.lock().unwrap();
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

    // If key up was previously consumed on key down, consume key up too
    if is_up {
        let mut consumed_guard = CONSUMED_KEYS.lock().unwrap();
        if let Some(ref mut set) = *consumed_guard {
            if set.remove(&vk_code) {
                return LRESULT(1);
            }
        }
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    // Determine modifier state
    let ctrl = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
    let alt = (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
    let shift = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
    let win = ((GetAsyncKeyState(0x5B) as u16 & 0x8000) != 0) // VK_LWIN
        || ((GetAsyncKeyState(0x5C) as u16 & 0x8000) != 0); // VK_RWIN
    let insert = (GetAsyncKeyState(VK_INSERT.0 as i32) as u16 & 0x8000) != 0;

    let modifiers = Modifiers {
        ctrl,
        alt,
        shift,
        win,
        insert,
    };

    let key = translate_vk_code(vk_code);
    let event = KeyEvent::new(key, modifiers, is_down);

    let mut action = KeyAction::PassThrough;
    {
        let mut cb_guard = ACTIVE_CALLBACK.lock().unwrap();
        if let Some(ref mut cb) = *cb_guard {
            action = cb(event);
        }
    }

    if action == KeyAction::Consume {
        let mut consumed_guard = CONSUMED_KEYS.lock().unwrap();
        if let Some(ref mut set) = *consumed_guard {
            set.insert(vk_code);
        }
        LRESULT(1)
    } else {
        CallNextHookEx(None, ncode, wparam, lparam)
    }
}

fn translate_vk_code(vk: u32) -> Key {
    match VIRTUAL_KEY(vk as u16) {
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

        VK_OEM_1 => Key::Semicolon,

        VK_CONTROL => Key::Control,
        VK_MENU => Key::Alt,
        VK_SHIFT => Key::Shift,

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
