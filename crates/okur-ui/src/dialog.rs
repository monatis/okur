//! Okur's dialogs on Windows: real dialogs (`#32770`) made from an in-memory template
//! ([`crate::template`]), modeless, in front of the application the user was in. The
//! controls start with the description's values; each change, and each button, is told to
//! Okur, which decides what happens.

use std::cell::RefCell;

use okur_ui_model::{ControlKind, DialogSpec, FromUi, Value};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Controls::{
    BST_CHECKED, BST_UNCHECKED, TBM_SETPAGESIZE, TBM_SETPOS, TBM_SETRANGEMAX, TBM_SETRANGEMIN,
    TBM_SETTICFREQ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateDialogIndirectParamW, DestroyWindow, GetDlgCtrlID, GetDlgItem,
    GetForegroundWindow, GetWindowThreadProcessId, IsDialogMessageW, IsWindow, SendMessageW,
    SetForegroundWindow, ShowWindow, BM_GETCHECK, BM_SETCHECK, BN_CLICKED, CBN_SELCHANGE,
    CB_ADDSTRING, CB_GETCURSEL, CB_RESETCONTENT, CB_SETCURSEL, DLGTEMPLATE, MSG, SW_SHOW,
    WM_COMMAND, WM_HSCROLL, WM_INITDIALOG,
};

use crate::template::{self, IDCANCEL};

/// TBM_GETPOS: a slider's position.
const TBM_GETPOS: u32 = 0x0400;

/// A dialog on screen.
struct OpenDialog {
    window: HWND,
    id: u32,
    /// The window in front when the dialog opened: it has the focus back when the dialog
    /// closes, as when a dialog of its own closes.
    previous: HWND,
    /// Each control's window id, its id in the description, its kind, and the value last
    /// told to Okur (sliders send a message per step and another when they stop).
    controls: Vec<(u16, String, ControlKind, Option<Value>)>,
}

thread_local! {
    static DIALOGS: RefCell<Vec<OpenDialog>> = const { RefCell::new(Vec::new()) };
}

fn window_of(id: u32) -> Option<HWND> {
    DIALOGS.with(|d| {
        d.try_borrow()
            .ok()
            .and_then(|d| d.iter().find(|o| o.id == id).map(|o| o.window))
    })
}

/// Shows `spec` in front, or brings it forward when it is showing.
pub fn show(spec: &DialogSpec) {
    if let Some(window) = window_of(spec.id) {
        bring_forward(window);
        return;
    }
    // SAFETY: a plain query.
    let previous = unsafe { GetForegroundWindow() };
    let layout = template::layout(spec);
    let words = template::encode(&spec.title, &layout);
    // SAFETY: the template is a well-formed DLGTEMPLATEEX, four-byte aligned, alive for the
    // call; the dialog procedure is defined below.
    let window = unsafe {
        let instance = GetModuleHandleW(None).ok().map(Into::into);
        CreateDialogIndirectParamW(
            instance,
            words.as_ptr().cast::<DLGTEMPLATE>(),
            None,
            Some(dialog_proc),
            LPARAM(0),
        )
    };
    let Ok(window) = window else {
        eprintln!("okur-ui: dialog {:?} could not be created", spec.title);
        return;
    };
    let controls = spec
        .controls
        .iter()
        .zip(&layout.ids)
        .map(|(c, &id)| (id, c.id.clone(), c.kind.clone(), None))
        .collect();
    DIALOGS.with(|d| {
        if let Ok(mut d) = d.try_borrow_mut() {
            d.push(OpenDialog {
                window,
                id: spec.id,
                previous,
                controls,
            });
        }
    });
    for (control, &id) in spec.controls.iter().zip(&layout.ids) {
        // SAFETY: a child of the dialog just created.
        let Ok(item) = (unsafe { GetDlgItem(Some(window), i32::from(id)) }) else {
            continue;
        };
        fill(item, &control.kind);
        if !control.enabled {
            // SAFETY: disabling our own control.
            let _ = unsafe { EnableWindow(item, false) };
        }
    }
    bring_forward(window);
}

/// Gives a control its initial value.
fn fill(item: HWND, kind: &ControlKind) {
    // SAFETY: messages to our own controls; the strings outlive each call.
    unsafe {
        match kind {
            ControlKind::Choice { options, selected } => {
                SendMessageW(item, CB_RESETCONTENT, None, None);
                for option in options {
                    let text: Vec<u16> = option.encode_utf16().chain([0]).collect();
                    SendMessageW(
                        item,
                        CB_ADDSTRING,
                        None,
                        Some(LPARAM(text.as_ptr() as isize)),
                    );
                }
                set(item, kind, Value::Selected(*selected));
            }
            ControlKind::CheckBox { checked } => set(item, kind, Value::Checked(*checked)),
            ControlKind::Slider { min, max, value } => {
                SendMessageW(
                    item,
                    TBM_SETRANGEMIN,
                    Some(WPARAM(0)),
                    Some(LPARAM(*min as isize)),
                );
                SendMessageW(
                    item,
                    TBM_SETRANGEMAX,
                    Some(WPARAM(1)),
                    Some(LPARAM(*max as isize)),
                );
                SendMessageW(item, TBM_SETPAGESIZE, Some(WPARAM(0)), Some(LPARAM(1)));
                // About ten ticks, whatever the range.
                let step = usize::try_from((max - min) / 10).unwrap_or(1).max(1);
                SendMessageW(item, TBM_SETTICFREQ, Some(WPARAM(step)), None);
                set(item, kind, Value::Number(*value));
            }
            ControlKind::Group | ControlKind::Button { .. } => {}
        }
    }
}

/// Sets a control's value.
fn set(item: HWND, kind: &ControlKind, value: Value) {
    // SAFETY: messages to our own controls.
    unsafe {
        match (kind, value) {
            (ControlKind::Choice { .. }, Value::Selected(index)) => {
                SendMessageW(item, CB_SETCURSEL, Some(WPARAM(index)), None);
            }
            (ControlKind::CheckBox { .. }, Value::Checked(checked)) => {
                let state = if checked { BST_CHECKED } else { BST_UNCHECKED };
                SendMessageW(item, BM_SETCHECK, Some(WPARAM(state.0 as usize)), None);
            }
            (ControlKind::Slider { .. }, Value::Number(number)) => {
                SendMessageW(
                    item,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(number as isize)),
                );
            }
            _ => {}
        }
    }
}

/// Sets the value of `control` in dialog `id`, as Okur asks.
pub fn set_value(id: u32, control: &str, value: Value) {
    let found = DIALOGS.with(|d| {
        d.try_borrow().ok().and_then(|d| {
            let dialog = d.iter().find(|o| o.id == id)?;
            let (window_id, _, kind, _) = dialog.controls.iter().find(|c| c.1 == control)?;
            Some((dialog.window, *window_id, kind.clone()))
        })
    });
    if let Some((window, window_id, kind)) = found {
        // SAFETY: a child of our own dialog.
        if let Ok(item) = unsafe { GetDlgItem(Some(window), i32::from(window_id)) } {
            set(item, &kind, value);
        }
    }
}

/// Closes dialog `id`.
pub fn close(id: u32) {
    let window = DIALOGS.with(|d| {
        d.try_borrow_mut().ok().and_then(|mut d| {
            let index = d.iter().position(|o| o.id == id)?;
            let dialog = d.remove(index);
            Some((dialog.window, dialog.previous))
        })
    });
    if let Some((window, previous)) = window {
        // SAFETY: destroying our own dialog, then a query on the window it came from.
        let _ = unsafe { DestroyWindow(window) };
        if unsafe { IsWindow(Some(previous)) }.as_bool() {
            bring_forward(previous);
        }
    }
}

/// Lets the dialog manager handle `msg` (Tab, access keys, Enter, Escape); true when it did.
pub fn is_dialog_message(msg: &MSG) -> bool {
    let windows: Vec<HWND> = DIALOGS.with(|d| {
        d.try_borrow()
            .map(|d| d.iter().map(|o| o.window).collect())
            .unwrap_or_default()
    });
    windows
        .into_iter()
        // SAFETY: our own dialogs, and the message the loop just received.
        .any(|w| unsafe { IsDialogMessageW(w, msg) }.as_bool())
}

/// Brings `window` to the front. Windows lets only the foreground thread give the
/// foreground away, so the input of the window in front is attached for the switch, as
/// `okur front` does.
fn bring_forward(window: HWND) {
    // SAFETY: plain window calls; the thread input is detached again below.
    unsafe {
        let front = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let ours = GetCurrentThreadId();
        let attached =
            front != 0 && front != ours && AttachThreadInput(ours, front, true).as_bool();
        let _ = ShowWindow(window, SW_SHOW);
        let _ = BringWindowToTop(window);
        let _ = SetForegroundWindow(window);
        if attached {
            let _ = AttachThreadInput(ours, front, false);
        }
    }
}

/// The dialog and control a window id belongs to.
fn lookup(window: HWND, control_id: u16) -> Option<(u32, String, ControlKind, Option<Value>)> {
    DIALOGS.with(|d| {
        d.try_borrow().ok().and_then(|d| {
            let dialog = d.iter().find(|o| o.window == window)?;
            let (_, name, kind, last) = dialog.controls.iter().find(|c| c.0 == control_id)?;
            Some((dialog.id, name.clone(), kind.clone(), *last))
        })
    })
}

fn remember(window: HWND, control_id: u16, value: Value) {
    DIALOGS.with(|d| {
        if let Ok(mut d) = d.try_borrow_mut() {
            if let Some(dialog) = d.iter_mut().find(|o| o.window == window) {
                if let Some(control) = dialog.controls.iter_mut().find(|c| c.0 == control_id) {
                    control.3 = Some(value);
                }
            }
        }
    });
}

fn dialog_id(window: HWND) -> Option<u32> {
    DIALOGS.with(|d| {
        d.try_borrow()
            .ok()
            .and_then(|d| d.iter().find(|o| o.window == window).map(|o| o.id))
    })
}

/// Tells Okur that `value` was set, once per change.
fn changed(window: HWND, control_id: u16, value: Value) {
    let Some((id, name, _, last)) = lookup(window, control_id) else {
        return;
    };
    if last == Some(value) {
        return;
    }
    remember(window, control_id, value);
    crate::win32::tell(&FromUi::DialogChanged {
        id,
        control: name,
        value,
    });
}

unsafe extern "system" fn dialog_proc(
    window: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match msg {
        // The dialog manager puts the focus on the first control.
        WM_INITDIALOG => 1,
        WM_COMMAND => {
            let control_id = (wparam.0 & 0xFFFF) as u16;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u32;
            let item = HWND(lparam.0 as *mut _);
            if control_id == IDCANCEL {
                // Cancel, Escape, the close box: the dialog goes, and Okur hears of it.
                if let Some(id) = dialog_id(window) {
                    close(id);
                    crate::win32::tell(&FromUi::DialogClosed { id });
                }
                return 1;
            }
            let Some((id, name, kind, _)) = lookup(window, control_id) else {
                return 0;
            };
            match kind {
                ControlKind::Button { .. } if code == BN_CLICKED => {
                    crate::win32::tell(&FromUi::DialogPressed { id, control: name });
                }
                ControlKind::CheckBox { .. } if code == BN_CLICKED => {
                    // SAFETY: a state query on our own check box.
                    let state = unsafe { SendMessageW(item, BM_GETCHECK, None, None) };
                    let checked = state.0 == BST_CHECKED.0 as isize;
                    changed(window, control_id, Value::Checked(checked));
                }
                ControlKind::Choice { .. } if code == CBN_SELCHANGE => {
                    // SAFETY: a state query on our own combo box.
                    let index = unsafe { SendMessageW(item, CB_GETCURSEL, None, None) };
                    if let Ok(index) = usize::try_from(index.0) {
                        changed(window, control_id, Value::Selected(index));
                    }
                }
                _ => return 0,
            }
            1
        }
        WM_HSCROLL => {
            let item = HWND(lparam.0 as *mut _);
            // SAFETY: queries on our own slider, named by the message.
            let (control_id, position) = unsafe {
                (
                    GetDlgCtrlID(item),
                    SendMessageW(item, TBM_GETPOS, None, None),
                )
            };
            if let (Ok(control_id), Ok(position)) =
                (u16::try_from(control_id), i32::try_from(position.0))
            {
                changed(window, control_id, Value::Number(position));
            }
            1
        }
        _ => 0,
    }
}
