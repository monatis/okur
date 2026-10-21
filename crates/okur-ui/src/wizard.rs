//! Okur's wizards, as Windows property-sheet wizards: a modeless sheet (`PSH_WIZARD`) under
//! Back, Next or Finish, and Cancel, showing one page at a time, as JAWS's Flexible Web is
//! [obs, 705's trace: a `#32770` sheet with a hidden tab control, its page a `#32770` child
//! named by the page's title]. A page is a dialog of its own for its controls
//! ([`crate::dialog::adopt_page`]), so its values are set and told as any dialog's.
//!
//! Okur decides where the wizard goes: Back, Next and Finish are told to it and the sheet
//! stays where it is ([`FromUi::DialogPressed`] "back", "next", "finish"); Okur answers with
//! the next page ([`turn`]), which is added and shown, the page before it removed. Cancel and
//! Escape end the wizard, and Okur hears of it ([`FromUi::DialogClosed`]).

use std::cell::RefCell;

use okur_ui_model::{DialogSpec, FromUi, WizardButtons, WizardSpec};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    CreatePropertySheetPageW, PropertySheetW, HPROPSHEETPAGE, NMHDR, PROPSHEETHEADERW_V2,
    PROPSHEETHEADERW_V2_2, PROPSHEETPAGEW, PROPSHEETPAGEW_0, PSH_MODELESS, PSH_WIZARD, PSM_ADDPAGE,
    PSM_GETCURRENTPAGEHWND, PSM_ISDIALOGMESSAGE, PSM_SETCURSEL, PSM_SETWIZBUTTONS, PSN_QUERYCANCEL,
    PSN_SETACTIVE, PSN_WIZBACK, PSN_WIZFINISH, PSN_WIZNEXT, PSP_DLGINDIRECT, PSP_USETITLE,
    PSWIZB_BACK, PSWIZB_DISABLEDFINISH, PSWIZB_FINISH, PSWIZB_NEXT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, GetForegroundWindow, GetNextDlgTabItem, IsWindow, PostMessageW, SendMessageW,
    SetWindowLongPtrW, DLGTEMPLATE, MSG, WINDOW_LONG_PTR_INDEX, WM_INITDIALOG, WM_NEXTDLGCTL,
    WM_NOTIFY,
};

use crate::template;

/// `SetWindowLongPtr`'s index for a dialog procedure's message result.
const DWLP_MSGRESULT: WINDOW_LONG_PTR_INDEX = WINDOW_LONG_PTR_INDEX(0);

/// A page's template and title, kept alive while the sheet holds the page: never read, only
/// held.
struct Kept {
    _template: Vec<u32>,
    _title: Vec<u16>,
}

/// A wizard on screen.
struct OpenWizard {
    id: u32,
    sheet: HWND,
    /// The page shown.
    page: HWND,
    buttons: WizardButtons,
    /// The window in front when the wizard opened, which has the focus back after it.
    previous: HWND,
    kept: Vec<Kept>,
}

thread_local! {
    static WIZARDS: RefCell<Vec<OpenWizard>> = const { RefCell::new(Vec::new()) };
    /// The page being created: its description and window ids, which its first message takes.
    static CREATING: RefCell<Option<(DialogSpec, Vec<u16>)>> = const { RefCell::new(None) };
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn flags(buttons: WizardButtons) -> u32 {
    let mut flags = 0;
    if buttons.back {
        flags |= PSWIZB_BACK;
    }
    if buttons.next {
        flags |= PSWIZB_NEXT;
    }
    if buttons.finish {
        flags |= PSWIZB_FINISH;
    } else if !buttons.next {
        flags |= PSWIZB_DISABLEDFINISH;
    }
    flags
}

/// A property page for `page`, its template and title kept in `kept`.
fn create_page(page: &DialogSpec, kept: &mut Vec<Kept>) -> Option<HPROPSHEETPAGE> {
    let layout = template::page_layout(page);
    let template = template::encode_page(&page.title, &layout);
    let title = wide(&page.title);
    CREATING.with(|c| {
        if let Ok(mut c) = c.try_borrow_mut() {
            *c = Some((page.clone(), layout.ids.clone()));
        }
    });
    // SAFETY: a module handle query; the page's template and title live in `kept` as long as
    // the sheet holds the page; the procedure is defined below.
    let handle = unsafe {
        let instance = GetModuleHandleW(None)
            .ok()
            .map(Into::into)
            .unwrap_or_default();
        let mut sheet_page = PROPSHEETPAGEW {
            dwSize: u32::try_from(std::mem::size_of::<PROPSHEETPAGEW>()).unwrap_or(0),
            dwFlags: PSP_DLGINDIRECT | PSP_USETITLE,
            hInstance: instance,
            Anonymous1: PROPSHEETPAGEW_0 {
                pResource: template.as_ptr() as *mut DLGTEMPLATE,
            },
            pszTitle: PCWSTR(title.as_ptr()),
            pfnDlgProc: Some(page_proc),
            lParam: LPARAM(isize::try_from(page.id).unwrap_or(0)),
            ..Default::default()
        };
        CreatePropertySheetPageW(&mut sheet_page)
    };
    kept.push(Kept {
        _template: template,
        _title: title,
    });
    (!handle.is_invalid()).then_some(handle)
}

/// Shows `spec` in front on its first page; one already showing comes forward.
pub fn show(spec: &WizardSpec) {
    if let Some(sheet) = sheet_of(spec.id) {
        crate::dialog::bring_forward(sheet);
        return;
    }
    // SAFETY: a plain query.
    let previous = unsafe { GetForegroundWindow() };
    let mut kept = Vec::new();
    let Some(page) = create_page(&spec.page, &mut kept) else {
        eprintln!("okur-ui: wizard {:?} could not be created", spec.page.title);
        return;
    };
    WIZARDS.with(|w| {
        if let Ok(mut w) = w.try_borrow_mut() {
            w.push(OpenWizard {
                id: spec.id,
                sheet: HWND::default(),
                page: HWND::default(),
                buttons: spec.buttons,
                previous,
                kept,
            });
        }
    });
    let mut pages = [page];
    let caption = wide(&spec.page.title);
    // SAFETY: the header points at the page handle and caption above, alive for the call; a
    // modeless sheet returns its window.
    let sheet = unsafe {
        let instance = GetModuleHandleW(None)
            .ok()
            .map(Into::into)
            .unwrap_or_default();
        let mut header = PROPSHEETHEADERW_V2 {
            dwSize: u32::try_from(std::mem::size_of::<PROPSHEETHEADERW_V2>()).unwrap_or(0),
            dwFlags: PSH_WIZARD | PSH_MODELESS,
            hInstance: instance,
            pszCaption: PCWSTR(caption.as_ptr()),
            nPages: 1,
            Anonymous3: PROPSHEETHEADERW_V2_2 {
                phpage: pages.as_mut_ptr(),
            },
            ..Default::default()
        };
        HWND(PropertySheetW(&mut header) as *mut _)
    };
    if sheet.is_invalid() {
        eprintln!("okur-ui: wizard {:?} could not be shown", spec.page.title);
        forget(spec.id);
        return;
    }
    WIZARDS.with(|w| {
        if let Ok(mut w) = w.try_borrow_mut() {
            if let Some(open) = w.iter_mut().find(|o| o.id == spec.id) {
                open.sheet = sheet;
            }
        }
    });
    set_buttons(sheet, spec.buttons);
    crate::dialog::bring_forward(sheet);
}

/// Turns wizard `id` to `page`, with `buttons`: the page is added and shown, the focus on its
/// first control. The pages before it stay in the sheet, hidden: once one was removed, the
/// sheet sent Next to the page it had chosen before, not to the one shown, and Okur heard
/// nothing of it (old-pc, 2026-10-21).
pub fn turn(id: u32, page: &DialogSpec, buttons: WizardButtons) {
    let Some((sheet, old)) = WIZARDS.with(|w| {
        w.try_borrow_mut().ok().and_then(|mut w| {
            let open = w.iter_mut().find(|o| o.id == id)?;
            open.buttons = buttons;
            Some((open.sheet, open.page))
        })
    }) else {
        return;
    };
    let mut kept = Vec::new();
    let Some(handle) = create_page(page, &mut kept) else {
        return;
    };
    // SAFETY: messages to our own sheet with a page handle it now holds, which chooses the
    // page whatever the index.
    unsafe {
        SendMessageW(
            sheet,
            PSM_ADDPAGE,
            Some(WPARAM(0)),
            Some(LPARAM(handle.0 as isize)),
        );
        SendMessageW(
            sheet,
            PSM_SETCURSEL,
            Some(WPARAM(0)),
            Some(LPARAM(handle.0 as isize)),
        );
    }
    // The page before is hidden: its controls are no longer Okur's.
    crate::dialog::forget_page(old);
    WIZARDS.with(|w| {
        if let Ok(mut w) = w.try_borrow_mut() {
            if let Some(open) = w.iter_mut().find(|o| o.id == id) {
                open.kept.extend(kept);
            }
        }
    });
    focus_first_control(sheet);
}

/// Wizard `id`'s buttons on the page shown, and when it is shown again.
pub fn buttons(id: u32, buttons: WizardButtons) {
    let sheet = WIZARDS.with(|w| {
        w.try_borrow_mut().ok().and_then(|mut w| {
            let open = w.iter_mut().find(|o| o.id == id)?;
            open.buttons = buttons;
            Some(open.sheet)
        })
    });
    if let Some(sheet) = sheet {
        set_buttons(sheet, buttons);
    }
}

/// The focus on the shown page's first control, as on a dialog's opening.
fn focus_first_control(sheet: HWND) {
    // SAFETY: queries and a message to our own sheet and its page.
    unsafe {
        let page = HWND(SendMessageW(sheet, PSM_GETCURRENTPAGEHWND, None, None).0 as *mut _);
        if let Ok(first) = GetNextDlgTabItem(page, None, false) {
            SendMessageW(
                sheet,
                WM_NEXTDLGCTL,
                Some(WPARAM(first.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }
}

fn set_buttons(sheet: HWND, buttons: WizardButtons) {
    // SAFETY: a message to our own sheet; posted, as a page's activation asks.
    let _ = unsafe {
        PostMessageW(
            Some(sheet),
            PSM_SETWIZBUTTONS,
            WPARAM(0),
            LPARAM(isize::try_from(flags(buttons)).unwrap_or(0)),
        )
    };
}

fn sheet_of(id: u32) -> Option<HWND> {
    WIZARDS.with(|w| {
        w.try_borrow()
            .ok()
            .and_then(|w| w.iter().find(|o| o.id == id).map(|o| o.sheet))
    })
}

/// Forgets wizard `id`, and its page; its sheet and the window before it, if it had them.
fn forget(id: u32) -> Option<(HWND, HWND)> {
    let open = WIZARDS.with(|w| {
        w.try_borrow_mut().ok().and_then(|mut w| {
            let index = w.iter().position(|o| o.id == id)?;
            Some(w.remove(index))
        })
    })?;
    crate::dialog::forget_page(open.page);
    Some((open.sheet, open.previous))
}

/// Closes wizard `id`, as Okur asks; false when there is none.
pub fn close(id: u32) -> bool {
    let Some((sheet, previous)) = forget(id) else {
        return false;
    };
    // SAFETY: a query, destroying our own sheet, then a query on the window it came from.
    unsafe {
        let in_front = GetForegroundWindow() == sheet;
        let _ = DestroyWindow(sheet);
        if in_front && IsWindow(Some(previous)).as_bool() {
            crate::dialog::bring_forward(previous);
        }
    }
    true
}

/// Lets a wizard's sheet handle `msg` (Tab, access keys, Enter, Escape); true when it did. A
/// sheet that Cancel or Escape ended is destroyed here, as a modeless sheet's owner must, and
/// Okur hears of it.
pub fn is_dialog_message(msg: &MSG) -> bool {
    let sheets: Vec<(u32, HWND)> = WIZARDS.with(|w| {
        w.try_borrow()
            .map(|w| w.iter().map(|o| (o.id, o.sheet)).collect())
            .unwrap_or_default()
    });
    for (id, sheet) in sheets {
        // SAFETY: messages to our own sheet; the message the loop just received.
        let page = unsafe { SendMessageW(sheet, PSM_GETCURRENTPAGEHWND, None, None) };
        if page.0 == 0 {
            if close(id) {
                crate::win32::tell(&FromUi::DialogClosed { id });
            }
            continue;
        }
        let handled = unsafe {
            SendMessageW(
                sheet,
                PSM_ISDIALOGMESSAGE,
                Some(WPARAM(0)),
                Some(LPARAM(msg as *const MSG as isize)),
            )
        };
        if handled.0 != 0 {
            return true;
        }
    }
    false
}

/// The wizard a page of `window` belongs to: its id, sheet and buttons.
fn wizard_of_page(window: HWND) -> Option<(u32, HWND, WizardButtons)> {
    WIZARDS.with(|w| {
        w.try_borrow().ok().and_then(|w| {
            w.iter()
                .find(|o| o.page == window)
                .map(|o| (o.id, o.sheet, o.buttons))
        })
    })
}

/// A page's answer to the sheet's notification.
fn answer(window: HWND, result: isize) -> isize {
    // SAFETY: our own page's message result.
    unsafe {
        SetWindowLongPtrW(window, DWLP_MSGRESULT, result);
    }
    1
}

unsafe extern "system" fn page_proc(
    window: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            if let Some((spec, ids)) = CREATING.with(|c| c.try_borrow_mut().ok()?.take()) {
                crate::dialog::adopt_page(window, &spec, &ids);
                WIZARDS.with(|w| {
                    if let Ok(mut w) = w.try_borrow_mut() {
                        if let Some(open) = w.iter_mut().find(|o| o.id == spec.id) {
                            open.page = window;
                        }
                    }
                });
            }
            1
        }
        WM_NOTIFY => {
            // SAFETY: WM_NOTIFY's lparam is the sender's NMHDR.
            let header = unsafe { &*(lparam.0 as *const NMHDR) };
            let Some((id, sheet, buttons)) = wizard_of_page(window) else {
                // SAFETY: the dialog procedure the page shares with Okur's dialogs.
                return unsafe { crate::dialog::dialog_proc(window, msg, wparam, lparam) };
            };
            let press = |control: &str| {
                crate::win32::tell(&FromUi::DialogPressed {
                    id,
                    control: control.to_owned(),
                });
            };
            match header.code {
                PSN_SETACTIVE => {
                    set_buttons(sheet, buttons);
                    answer(window, 0)
                }
                // Okur turns the page: the sheet stays where it is.
                PSN_WIZNEXT => {
                    press("next");
                    answer(window, -1)
                }
                PSN_WIZBACK => {
                    press("back");
                    answer(window, -1)
                }
                // Okur closes the wizard once it has done what Finish asks.
                PSN_WIZFINISH => {
                    press("finish");
                    answer(window, 1)
                }
                // Cancel and Escape end the sheet, which the loop then destroys.
                PSN_QUERYCANCEL => answer(window, 0),
                // SAFETY: as above.
                _ => unsafe { crate::dialog::dialog_proc(window, msg, wparam, lparam) },
            }
        }
        // SAFETY: as above.
        _ => unsafe { crate::dialog::dialog_proc(window, msg, wparam, lparam) },
    }
}
