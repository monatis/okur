//! The applications' top-level windows, as the taskbar and Alt+Tab show them, for the Window
//! List (Insert+F10), and switching to one of them.

use okur_core::event::{TopWindow, WindowState};

/// The visible application windows, front to back: each one's title, state, and whether it is
/// kept on top. Owned and tool windows, cloaked ones (a suspended Store app's) and the desktop
/// are left out, as the taskbar leaves them out.
#[cfg(windows)]
pub fn list() -> Vec<TopWindow> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowLongW, GetWindowTextLengthW, GetWindowTextW, IsIconic,
        IsWindowVisible, IsZoomed, GWL_EXSTYLE, GW_OWNER, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        WS_EX_TOPMOST,
    };

    unsafe extern "system" fn collect(hwnd: HWND, found: LPARAM) -> BOOL {
        // SAFETY: `found` is the address of the Vec below, alive for the whole enumeration.
        let found = unsafe { &mut *(found.0 as *mut Vec<HWND>) };
        found.push(hwnd);
        BOOL(1)
    }
    let mut handles: Vec<HWND> = Vec::new();
    // SAFETY: the callback only pushes to `handles`, which outlives the call.
    let _ = unsafe {
        EnumWindows(
            Some(collect),
            LPARAM(std::ptr::addr_of_mut!(handles) as isize),
        )
    };
    let mut windows = Vec::new();
    for hwnd in handles {
        // SAFETY: plain Win32 queries on a window handle, which may be gone by now.
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                continue;
            }
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            let owned = GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.is_invalid());
            let app_window = ex_style & WS_EX_APPWINDOW.0 != 0;
            if !app_window && (owned || ex_style & WS_EX_TOOLWINDOW.0 != 0) {
                continue;
            }
            let mut cloaked = 0u32;
            let read = DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                std::ptr::addr_of_mut!(cloaked).cast(),
                std::mem::size_of::<u32>() as u32,
            );
            if read.is_ok() && cloaked != 0 {
                continue;
            }
            let length = usize::try_from(GetWindowTextLengthW(hwnd)).unwrap_or(0);
            let mut buffer = vec![0u16; length + 1];
            let copied = usize::try_from(GetWindowTextW(hwnd, &mut buffer)).unwrap_or(0);
            let title = String::from_utf16_lossy(buffer.get(..copied).unwrap_or(&[]));
            let title = title.trim();
            if title.is_empty() || title == "Program Manager" {
                continue;
            }
            let state = if IsIconic(hwnd).as_bool() {
                WindowState::Minimized
            } else if IsZoomed(hwnd).as_bool() {
                WindowState::Maximized
            } else {
                WindowState::Restored
            };
            windows.push(TopWindow {
                id: hwnd.0 as usize as u64,
                title: title.to_owned(),
                state,
                on_top: ex_style & WS_EX_TOPMOST.0 != 0,
            });
        }
    }
    windows
}

#[cfg(not(windows))]
pub fn list() -> Vec<TopWindow> {
    Vec::new()
}

/// Brings window `id` to the front, restored first if it is minimized, as Alt+Tab does.
#[cfg(windows)]
pub fn switch_to(id: u64) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, SetForegroundWindow, ShowWindow, SwitchToThisWindow, SW_RESTORE,
    };
    let hwnd = HWND(id as usize as *mut core::ffi::c_void);
    // SAFETY: plain Win32 calls on a window handle, which may be gone by now.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        // SwitchToThisWindow does what Alt+Tab does, which a process without the foreground
        // (Okur, its dialog just closed) may do where SetForegroundWindow is refused.
        SwitchToThisWindow(hwnd, true);
        let _ = SetForegroundWindow(hwnd);
    }
}

#[cfg(not(windows))]
pub fn switch_to(id: u64) {
    let _ = id;
}
