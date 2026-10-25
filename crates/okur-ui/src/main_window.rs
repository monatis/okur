//! Okur's main window (Insert+J), as JAWS's: a title, a line or two of text (Okur's version),
//! and a menu bar (Options, Utilities, Help) that Alt goes to. An item chosen is told to Okur
//! by its id; closing the window asks Okur to exit, as Alt+F4 in JAWS's window does.

use std::cell::RefCell;

use okur_ui_model::{FromUi, MainWindowSpec, MenuItemSpec};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateMenu, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    GetClientRect, GetMenu, MoveWindow, SetMenu, SetWindowTextW, ShowWindow, HMENU, MF_CHECKED,
    MF_POPUP, MF_STRING, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_SETFOCUS,
    WM_SIZE, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE, WS_VSCROLL,
};

/// The window, its text, and the ids of its menu items by command number (from 1).
struct Main {
    window: HWND,
    text: HWND,
    ids: Vec<String>,
}

thread_local! {
    static MAIN: RefCell<Option<Main>> = const { RefCell::new(None) };
}

/// Its window class, registered with the others.
pub const CLASS: PCWSTR = w!("OkurMainWindow");

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// Shows the main window in front, made or remade from `spec`.
pub fn show(spec: &MainWindowSpec) {
    let existing = MAIN.with(|m| m.borrow().as_ref().map(|m| (m.window, m.text)));
    let (window, text) = match existing {
        Some(handles) => handles,
        None => {
            let Some(handles) = create() else {
                return;
            };
            handles
        }
    };
    let mut ids = Vec::new();
    // SAFETY: our own window, its text control, and the menus made here; the old menu bar is
    // destroyed once replaced.
    unsafe {
        let title = wide(&spec.title);
        let _ = SetWindowTextW(window, PCWSTR(title.as_ptr()));
        let lines = wide(&spec.lines.join("\r\n"));
        let _ = SetWindowTextW(text, PCWSTR(lines.as_ptr()));
        if let Ok(bar) = CreateMenu() {
            for menu in &spec.menus {
                if let Some(popup) = popup(&menu.items, &mut ids) {
                    let label = wide(&menu.label);
                    let _ = AppendMenuW(bar, MF_POPUP, popup.0 as usize, PCWSTR(label.as_ptr()));
                }
            }
            let old = GetMenu(window);
            let _ = SetMenu(window, Some(bar));
            if !old.is_invalid() {
                let _ = DestroyMenu(old);
            }
        }
    }
    // Known before it shows: its first WM_SIZE and WM_SETFOCUS size and focus the text, which
    // stayed 0 by 0 when they came first (performance review 2026-10-25, finding 17).
    MAIN.with(|m| {
        *m.borrow_mut() = Some(Main { window, text, ids });
    });
    // SAFETY: our own window and its text control.
    unsafe {
        let _ = ShowWindow(window, SW_SHOW);
        let mut client = RECT::default();
        let _ = GetClientRect(window, &mut client);
        let _ = MoveWindow(text, 0, 0, client.right, client.bottom, true);
    }
    crate::dialog::bring_forward(window);
}

/// A popup menu of `items`, their submenus within; each command item's id kept in `ids`, its
/// command number its place there plus one.
fn popup(items: &[MenuItemSpec], ids: &mut Vec<String>) -> Option<HMENU> {
    // SAFETY: menus made here and handed to the menu bar, which owns them from then on.
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        for item in items {
            let text = match &item.key {
                Some(key) => format!("{}\t{key}", item.label),
                None => item.label.clone(),
            };
            let text = wide(&text);
            if item.items.is_empty() {
                ids.push(item.id.clone());
                let flags = if item.checked {
                    MF_STRING | MF_CHECKED
                } else {
                    MF_STRING
                };
                let _ = AppendMenuW(menu, flags, ids.len(), PCWSTR(text.as_ptr()));
            } else if let Some(submenu) = popup(&item.items, ids) {
                let _ = AppendMenuW(menu, MF_POPUP, submenu.0 as usize, PCWSTR(text.as_ptr()));
            }
        }
        Some(menu)
    }
}

/// Makes the window and its text, hidden.
fn create() -> Option<(HWND, HWND)> {
    use windows::Win32::UI::WindowsAndMessaging::{ES_AUTOVSCROLL, ES_MULTILINE, ES_READONLY};
    // SAFETY: creating our own window of the class registered in `run`, and its child.
    unsafe {
        let instance = GetModuleHandleW(None).ok()?;
        let window = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            CLASS,
            w!("Okur"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            480,
            200,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .ok()?;
        let text = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("EDIT"),
            PCWSTR::null(),
            WS_CHILD
                | WS_VISIBLE
                | WS_VSCROLL
                | WINDOW_STYLE((ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL) as u32),
            0,
            0,
            0,
            0,
            Some(window),
            None,
            Some(instance.into()),
            None,
        )
        .ok()?;
        Some((window, text))
    }
}

/// The main window's messages: its menu's commands, its size, its focus, its closing.
pub unsafe extern "system" fn main_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let text = || MAIN.with(|m| m.borrow().as_ref().map(|m| m.text));
    match msg {
        // A menu item (no control, no accelerator): its id to Okur.
        WM_COMMAND if lparam.0 == 0 && (wparam.0 >> 16) & 0xFFFF == 0 => {
            let command = wparam.0 & 0xFFFF;
            let id = MAIN.with(|m| {
                m.borrow()
                    .as_ref()
                    .and_then(|m| command.checked_sub(1).and_then(|i| m.ids.get(i)).cloned())
            });
            if let Some(id) = id {
                crate::win32::tell(&FromUi::MainMenuCommand { id });
            }
            LRESULT(0)
        }
        // Alt+F4 and the close box: Okur is asked to exit, as JAWS's window does.
        WM_CLOSE => {
            crate::win32::tell(&FromUi::MainMenuCommand {
                id: okur_ui_model::MAIN_WINDOW_EXIT.to_owned(),
            });
            LRESULT(0)
        }
        WM_SIZE => {
            if let Some(text) = text() {
                let mut client = RECT::default();
                // SAFETY: our own window and its child.
                unsafe {
                    let _ = GetClientRect(hwnd, &mut client);
                    let _ = MoveWindow(text, 0, 0, client.right, client.bottom, true);
                }
            }
            LRESULT(0)
        }
        WM_SETFOCUS => {
            if let Some(text) = text() {
                // SAFETY: our own child window.
                let _ =
                    unsafe { windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(text)) };
            }
            LRESULT(0)
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
