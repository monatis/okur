//! Okur's dialogs on Windows: real dialogs (`#32770`) made from an in-memory template
//! ([`crate::template`]), modeless, in front of the application the user was in. The
//! controls start with the description's values; each change, and each button, is told to
//! Okur, which decides what happens.

use std::cell::RefCell;

use okur_ui_model::{ControlKind, DialogKey, DialogSpec, FromUi, TreeItem, Value};
use windows::core::PWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Controls::{
    BST_CHECKED, BST_UNCHECKED, EM_SETSEL, HTREEITEM, LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_STATE,
    LVIF_TEXT, LVIS_FOCUSED, LVIS_SELECTED, LVIS_STATEIMAGEMASK, LVITEMW, LVM_DELETEALLITEMS,
    LVM_EDITLABELW, LVM_ENSUREVISIBLE, LVM_GETNEXTITEM, LVM_INSERTCOLUMNW, LVM_INSERTITEMW,
    LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMCOUNT, LVM_SETITEMSTATE, LVM_SETITEMTEXTW,
    LVNI_SELECTED, LVN_DELETEALLITEMS, LVN_ENDLABELEDITW, LVN_ITEMCHANGED, LVN_KEYDOWN,
    LVS_EX_CHECKBOXES, LVS_EX_FULLROWSELECT, NMHDR, NMLISTVIEW, NMLVDISPINFOW, NMLVKEYDOWN,
    NMTREEVIEWW, NMTVKEYDOWN, NM_DBLCLK, TBM_SETPAGESIZE, TBM_SETPOS, TBM_SETRANGEMAX,
    TBM_SETRANGEMIN, TBM_SETTICFREQ, TREE_VIEW_ITEM_STATE_FLAGS, TVE_EXPAND, TVGN_CARET,
    TVGN_CHILD, TVGN_NEXT, TVGN_ROOT, TVIF_STATE, TVIF_TEXT, TVINSERTSTRUCTW, TVINSERTSTRUCTW_0,
    TVIS_EXPANDED, TVIS_STATEIMAGEMASK, TVITEMW, TVI_LAST, TVI_ROOT, TVM_DELETEITEM, TVM_EXPAND,
    TVM_GETNEXTITEM, TVM_INSERTITEMW, TVM_SELECTITEM, TVM_SETITEMW, TVN_ITEMEXPANDEDW, TVN_KEYDOWN,
    TVN_SELCHANGEDW, TVS_CHECKBOXES,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetKeyState, VIRTUAL_KEY, VK_DELETE, VK_F2, VK_SHIFT, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateDialogIndirectParamW, DestroyWindow, GetClientRect, GetDlgCtrlID,
    GetDlgItem, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsDialogMessageW, IsWindow, SendMessageW, SetForegroundWindow,
    SetWindowTextW, ShowWindow, BM_GETCHECK, BM_SETCHECK, BN_CLICKED, CBN_EDITCHANGE,
    CBN_SELCHANGE, CB_ADDSTRING, CB_GETCURSEL, CB_GETLBTEXT, CB_GETLBTEXTLEN, CB_RESETCONTENT,
    CB_SETCURSEL, DLGTEMPLATE, EN_CHANGE, LBN_SELCHANGE, LB_ADDSTRING, LB_GETCURSEL,
    LB_RESETCONTENT, LB_SETCURSEL, MSG, SW_SHOW, WM_COMMAND, WM_HSCROLL, WM_INITDIALOG,
    WM_NCDESTROY, WM_NOTIFY,
};

use crate::template::{self, IDCANCEL};

/// NM_TVSTATEIMAGECHANGING (NM_FIRST - 24): a tree view's item's state image about to change.
const NM_TVSTATEIMAGECHANGING: u32 = 0u32.wrapping_sub(24);
/// DWLP_MSGRESULT: a dialog procedure's answer to a notification.
const DWLP_MSGRESULT: windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX =
    windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX(0);

/// A dialog's answer to a notification that reads its result.
fn answer(window: HWND, result: isize) -> isize {
    // SAFETY: our own dialog's message result.
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(window, DWLP_MSGRESULT, result);
    }
    1
}

/// TBM_GETPOS: a slider's position.
const TBM_GETPOS: u32 = 0x0400;
/// BM_SETSTYLE, and the push button styles it sets: the default button's frame.
const BM_SETSTYLE: u32 = 0x00F4;
const BS_PUSHBUTTON: usize = 0;
const BS_DEFPUSHBUTTON: usize = 1;
/// DM_SETDEFID: the button Enter presses.
const DM_SETDEFID: u32 = 0x0401;

/// One control of an open dialog.
struct OpenControl {
    /// Its window id (a radio group's: its box's).
    window_id: u16,
    /// Its id in the description.
    name: String,
    kind: ControlKind,
    /// The value last told to Okur: sliders send a message per step and another when they
    /// stop, and a list selects an item twice (focus, then selection).
    last: Option<Value>,
}

/// A dialog on screen.
struct OpenDialog {
    window: HWND,
    id: u32,
    /// The window in front when the dialog opened: it has the focus back when the dialog
    /// closes, as when a dialog of its own closes.
    previous: HWND,
    controls: Vec<OpenControl>,
    /// A wizard's page: a child of its sheet, whose keys the sheet's dialog manager runs.
    page: bool,
    /// The dialog's own keys ([`okur_ui_model::DialogShortcut`]), as virtual key and
    /// modifiers, with the control each goes to and the one the focus must be in.
    keys: Vec<(Chord, String, Option<String>)>,
}

/// A key with its modifiers held: virtual key, Ctrl, Shift, Alt.
type Chord = (u16, bool, bool, bool);

/// A dialog key's text ("Ctrl+Shift+D", "F6") as a chord; `None` for one it cannot name.
fn chord(text: &str) -> Option<Chord> {
    let (mut ctrl, mut shift, mut alt) = (false, false, false);
    let mut key = None;
    for part in text.split('+').map(str::trim) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "shift" => shift = true,
            "alt" => alt = true,
            "enter" => key = Some(0x0D),
            "apps" => key = Some(0x5D),
            "escape" => key = Some(0x1B),
            "up" => key = Some(0x26),
            "down" => key = Some(0x28),
            name => {
                let upper = name.to_ascii_uppercase();
                key = match upper.as_bytes() {
                    [c] if c.is_ascii_alphanumeric() => Some(u16::from(*c)),
                    [b'F', rest @ ..] => std::str::from_utf8(rest)
                        .ok()
                        .and_then(|n| n.parse::<u16>().ok())
                        .filter(|n| (1..=24).contains(n))
                        .map(|n| 0x6F + n),
                    _ => None,
                };
            }
        }
    }
    Some((key?, ctrl, shift, alt))
}

thread_local! {
    static DIALOGS: RefCell<Vec<OpenDialog>> = const { RefCell::new(Vec::new()) };
    /// Set while a tree is filled: the items it opens itself are no user's opening.
    static FILLING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// A control's value as the dialog opens: Okur knows it, so it is never told back.
fn initial(kind: &ControlKind) -> Option<Value> {
    match kind {
        ControlKind::Choice { selected, .. }
        | ControlKind::List { selected, .. }
        | ControlKind::CheckList { selected, .. }
        | ControlKind::ListBox { selected, .. }
        | ControlKind::Tree { selected, .. }
        | ControlKind::Radios { selected, .. }
        | ControlKind::RadioButtons { selected, .. } => Some(Value::Selected(*selected)),
        ControlKind::Table { selected, .. } => selected.map(Value::Selected),
        ControlKind::CheckBox { checked } => Some(Value::Checked(*checked)),
        ControlKind::Slider { value, .. } => Some(Value::Number(*value)),
        ControlKind::Text { text }
        | ControlKind::Edit { text }
        | ControlKind::ComboEdit { text, .. } => Some(Value::Text(text.clone())),
        ControlKind::Group
        | ControlKind::Button { .. }
        | ControlKind::Message { .. }
        | ControlKind::Note { .. } => None,
    }
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
    // A dialog opened from one of Okur's own is owned by it, as JAWS's Add PlaceMarker is by
    // its PlaceMarker List: the focus is then in both, and both are named [obs, 702/s5].
    let owner = DIALOGS.with(|d| {
        d.try_borrow()
            .ok()
            .and_then(|d| d.iter().any(|o| o.window == previous).then_some(previous))
    });
    let layout = template::layout(spec);
    let words = template::encode(&spec.title, &layout);
    // SAFETY: the template is a well-formed DLGTEMPLATEEX, four-byte aligned, alive for the
    // call; the dialog procedure is defined below.
    let window = unsafe {
        let instance = GetModuleHandleW(None).ok().map(Into::into);
        CreateDialogIndirectParamW(
            instance,
            words.as_ptr().cast::<DLGTEMPLATE>(),
            owner,
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
        .map(|(c, &window_id)| OpenControl {
            window_id,
            name: c.id.clone(),
            kind: c.kind.clone(),
            last: initial(&c.kind),
        })
        .collect();
    DIALOGS.with(|d| {
        if let Ok(mut d) = d.try_borrow_mut() {
            d.push(OpenDialog {
                window,
                id: spec.id,
                previous,
                controls,
                page: false,
                keys: spec
                    .keys
                    .iter()
                    .filter_map(|k| Some((chord(&k.key)?, k.control.clone(), k.from.clone())))
                    .collect(),
            });
        }
    });
    fill_all(window, &spec.controls, &layout.ids);
    bring_forward(window);
}

/// The value a pane's creation passes to its first message, which leaves the focus alone.
const PANE: isize = 0x0050_414E;

/// Shows `page`'s controls in dialog `id`'s pane, right of its tree, in place of the pane
/// shown before, the dialog widened to hold it. The focus stays where it is, as Settings
/// Center's pane changes with the tree's item and F6 alone goes to it [obs, the walk of
/// Settings Center's F6]. Its controls report as the dialog's (`page.id`).
pub fn show_pane(id: u32, page: &DialogSpec) {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::MapWindowPoints;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    };
    let Some(parent) = DIALOGS.with(|d| {
        d.try_borrow()
            .ok()
            .and_then(|d| d.iter().find(|o| o.id == id && !o.page).map(|o| o.window))
    }) else {
        return;
    };
    let old = DIALOGS.with(|d| {
        d.try_borrow()
            .ok()
            .and_then(|d| d.iter().find(|o| o.id == id && o.page).map(|o| o.window))
    });
    let layout = template::page_layout(page);
    let words = template::encode_pane(&page.title, &layout);
    // SAFETY: a well-formed template alive for the call, a child of our own dialog.
    let pane = unsafe {
        let instance = GetModuleHandleW(None).ok().map(Into::into);
        CreateDialogIndirectParamW(
            instance,
            words.as_ptr().cast::<DLGTEMPLATE>(),
            Some(parent),
            Some(dialog_proc),
            LPARAM(PANE),
        )
    };
    let Ok(pane) = pane else {
        eprintln!("okur-ui: pane {:?} could not be created", page.title);
        return;
    };
    if let Some(old) = old {
        forget_page(old);
        // SAFETY: our own pane, replaced.
        let _ = unsafe { DestroyWindow(old) };
    }
    adopt_page(pane, page, &layout.ids);
    // Right of the tree, its top level with the tree's; the dialog as wide and tall as needed.
    let tree = find(id, "tree").and_then(|(window, window_id, _, _)| {
        // SAFETY: a child of our own dialog.
        unsafe { GetDlgItem(Some(window), i32::from(window_id)) }.ok()
    });
    // SAFETY: rectangles of our own windows, and moving them.
    unsafe {
        let mut at = RECT::default();
        if let Some(tree) = tree {
            let _ = GetWindowRect(tree, &mut at);
            let mut corners = [
                POINT {
                    x: at.left,
                    y: at.top,
                },
                POINT {
                    x: at.right,
                    y: at.bottom,
                },
            ];
            MapWindowPoints(None, Some(parent), &mut corners);
            at = RECT {
                left: corners[0].x,
                top: corners[0].y,
                right: corners[1].x,
                bottom: corners[1].y,
            };
        }
        let mut size = RECT::default();
        let _ = GetWindowRect(pane, &mut size);
        let (x, y) = (at.right + 8, at.top);
        let _ = SetWindowPos(
            pane,
            None,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let need_right = x + (size.right - size.left) + 8;
        let need_bottom = y + (size.bottom - size.top) + 8;
        let mut client = RECT::default();
        let _ = GetClientRect(parent, &mut client);
        let wider = (need_right - client.right).max(0);
        let taller = (need_bottom - client.bottom).max(0);
        if wider > 0 || taller > 0 {
            let mut outer = RECT::default();
            let _ = GetWindowRect(parent, &mut outer);
            let _ = SetWindowPos(
                parent,
                None,
                0,
                0,
                outer.right - outer.left + wider,
                outer.bottom - outer.top + taller,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

/// Gives each control of a dialog just created its value, and dims those not available. Radio
/// buttons standing alone have no window of their own id: each of theirs is set.
fn fill_all(window: HWND, controls: &[okur_ui_model::Control], ids: &[u16]) {
    for (index, (control, &id)) in controls.iter().zip(ids).enumerate() {
        if let ControlKind::RadioButtons { options, .. } = &control.kind {
            fill(window, index, HWND::default(), &control.kind);
            if !control.enabled {
                for option in 0..options.len() {
                    // SAFETY: a child of the dialog just created, dimmed.
                    if let Ok(radio) = unsafe {
                        GetDlgItem(Some(window), i32::from(template::radio_id(index, option)))
                    } {
                        let _ = unsafe { EnableWindow(radio, false) };
                    }
                }
            }
            continue;
        }
        // SAFETY: a child of the dialog just created.
        let Ok(item) = (unsafe { GetDlgItem(Some(window), i32::from(id)) }) else {
            continue;
        };
        fill(window, index, item, &control.kind);
        if !control.enabled {
            // SAFETY: disabling our own control.
            let _ = unsafe { EnableWindow(item, false) };
        }
    }
}

/// Gives a control its initial value.
fn fill(window: HWND, index: usize, item: HWND, kind: &ControlKind) {
    // SAFETY: messages to our own controls; the strings outlive each call.
    unsafe {
        match kind {
            ControlKind::Choice { options, selected } => {
                SendMessageW(item, CB_RESETCONTENT, None, None);
                for option in options {
                    let text = wide(option);
                    SendMessageW(
                        item,
                        CB_ADDSTRING,
                        None,
                        Some(LPARAM(text.as_ptr() as isize)),
                    );
                }
                set(window, index, item, kind, &Value::Selected(*selected));
            }
            ControlKind::CheckBox { checked } => {
                set(window, index, item, kind, &Value::Checked(*checked));
            }
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
                set(window, index, item, kind, &Value::Number(*value));
            }
            ControlKind::List { items, selected } => {
                SendMessageW(
                    item,
                    LVM_SETEXTENDEDLISTVIEWSTYLE,
                    Some(WPARAM(LVS_EX_FULLROWSELECT as usize)),
                    Some(LPARAM(LVS_EX_FULLROWSELECT as isize)),
                );
                // One column, as wide as the list.
                let mut client = windows::Win32::Foundation::RECT::default();
                let _ = GetClientRect(item, &mut client);
                let column = LVCOLUMNW {
                    mask: LVCF_WIDTH,
                    cx: client.right - client.left,
                    ..Default::default()
                };
                SendMessageW(
                    item,
                    LVM_INSERTCOLUMNW,
                    Some(WPARAM(0)),
                    Some(LPARAM(&column as *const LVCOLUMNW as isize)),
                );
                set_items(item, items, *selected);
            }
            ControlKind::CheckList {
                columns,
                rows,
                checked,
                selected,
                ..
            } => {
                SendMessageW(
                    item,
                    LVM_SETEXTENDEDLISTVIEWSTYLE,
                    Some(WPARAM(LVS_EX_CHECKBOXES as usize)),
                    Some(LPARAM(LVS_EX_CHECKBOXES as isize)),
                );
                // One column without headers, or its columns under theirs.
                let one = [String::new()];
                let columns = if columns.is_empty() {
                    &one[..]
                } else {
                    columns
                };
                fill_table(item, columns, rows, Some(*selected));
                for (index, checked) in checked.iter().enumerate() {
                    check_item(item, index, *checked);
                }
            }
            ControlKind::Table {
                columns,
                rows,
                selected,
            } => fill_table(item, columns, rows, *selected),
            ControlKind::Tree { items, selected } => fill_tree(item, items, *selected),
            ControlKind::ListBox { items, selected } => fill_list_box(item, items, *selected),
            ControlKind::Radios { selected, .. } | ControlKind::RadioButtons { selected, .. } => {
                set(window, index, item, kind, &Value::Selected(*selected));
            }
            ControlKind::Text { text } | ControlKind::Note { text } => {
                let text = wide(text);
                let _ = SetWindowTextW(item, windows::core::PCWSTR(text.as_ptr()));
            }
            ControlKind::Edit { text } => {
                set(window, index, item, kind, &Value::Text(text.clone()));
            }
            ControlKind::ComboEdit { text, history } => {
                SendMessageW(item, CB_RESETCONTENT, None, None);
                for earlier in history {
                    let earlier = wide(earlier);
                    SendMessageW(
                        item,
                        CB_ADDSTRING,
                        None,
                        Some(LPARAM(earlier.as_ptr() as isize)),
                    );
                }
                set(window, index, item, kind, &Value::Text(text.clone()));
            }
            // Its text is the template's.
            ControlKind::Group | ControlKind::Button { .. } | ControlKind::Message { .. } => {}
        }
    }
}

/// Fills a table: its columns' headers, then its rows; the row `selected`, or, with none, the
/// first has the focus without being selected.
fn fill_table(list: HWND, columns: &[String], rows: &[Vec<String>], selected: Option<usize>) {
    // SAFETY: messages to our own list view; each string outlives its call.
    unsafe {
        SendMessageW(
            list,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            Some(WPARAM(LVS_EX_FULLROWSELECT as usize)),
            Some(LPARAM(LVS_EX_FULLROWSELECT as isize)),
        );
        let mut client = windows::Win32::Foundation::RECT::default();
        let _ = GetClientRect(list, &mut client);
        let width = client.right - client.left;
        let others = i32::try_from(columns.len().saturating_sub(1)).unwrap_or(1);
        for (index, header) in columns.iter().enumerate() {
            let mut text = wide(header);
            // The first column, the item's own text, takes most of the width.
            let cx = match (index, others) {
                (_, 0) => width,
                (0, _) => width * 3 / 5,
                _ => width * 2 / 5 / others,
            };
            let column = LVCOLUMNW {
                mask: LVCF_WIDTH | LVCF_TEXT,
                cx,
                pszText: PWSTR(text.as_mut_ptr()),
                ..Default::default()
            };
            SendMessageW(
                list,
                LVM_INSERTCOLUMNW,
                Some(WPARAM(index)),
                Some(LPARAM(&column as *const LVCOLUMNW as isize)),
            );
        }
    }
    fill_rows(list, rows, selected);
}

/// Runs `fill` with `list` not drawn, then draws it once: a list refilled item by item was
/// drawn again for each, and the Links List on a big page was sorted and filled anew on each
/// change of order (performance review 2026-10-23, T16).
fn without_redraw(list: HWND, fill: impl FnOnce()) {
    use windows::Win32::Graphics::Gdi::InvalidateRect;
    use windows::Win32::UI::WindowsAndMessaging::WM_SETREDRAW;
    // SAFETY: messages to our own control.
    unsafe {
        SendMessageW(list, WM_SETREDRAW, Some(WPARAM(0)), None);
    }
    fill();
    // SAFETY: messages to our own control; it is drawn whole again.
    unsafe {
        SendMessageW(list, WM_SETREDRAW, Some(WPARAM(1)), None);
        let _ = InvalidateRect(Some(list), None, true);
    }
}

/// A list view's rows, each cell in its column, `selected` selected (none: the first has the
/// focus).
fn fill_rows(list: HWND, rows: &[Vec<String>], selected: Option<usize>) {
    without_redraw(list, || insert_rows(list, rows));
    select_row(list, rows, selected);
}

/// A list view's rows put in it, in place of what it had.
fn insert_rows(list: HWND, rows: &[Vec<String>]) {
    // SAFETY: messages to our own list view; each string outlives its call.
    unsafe {
        SendMessageW(list, LVM_DELETEALLITEMS, None, None);
        SendMessageW(list, LVM_SETITEMCOUNT, Some(WPARAM(rows.len())), None);
        for (row_index, row) in rows.iter().enumerate() {
            for (cell_index, cell) in row.iter().enumerate() {
                let mut text = wide(cell);
                let item = LVITEMW {
                    mask: LVIF_TEXT,
                    iItem: i32::try_from(row_index).unwrap_or(i32::MAX),
                    iSubItem: i32::try_from(cell_index).unwrap_or(i32::MAX),
                    pszText: PWSTR(text.as_mut_ptr()),
                    ..Default::default()
                };
                let message = if cell_index == 0 {
                    LVM_INSERTITEMW
                } else {
                    LVM_SETITEMTEXTW
                };
                SendMessageW(
                    list,
                    message,
                    Some(WPARAM(row_index)),
                    Some(LPARAM(&item as *const LVITEMW as isize)),
                );
            }
        }
    }
}

/// `selected` selected in a list view of `rows`; none: the first has the focus.
fn select_row(list: HWND, rows: &[Vec<String>], selected: Option<usize>) {
    // SAFETY: messages to our own list view; the item state outlives its call.
    unsafe {
        match selected {
            Some(index) => select_item(list, index),
            None if !rows.is_empty() => {
                let state = LVITEMW {
                    mask: LVIF_STATE,
                    state: LVIS_FOCUSED,
                    stateMask: LVIS_FOCUSED,
                    ..Default::default()
                };
                SendMessageW(
                    list,
                    LVM_SETITEMSTATE,
                    Some(WPARAM(0)),
                    Some(LPARAM(&state as *const LVITEMW as isize)),
                );
            }
            None => {}
        }
    }
}

/// Fills a list box with `items` and selects one.
fn fill_list_box(list: HWND, items: &[String], selected: usize) {
    without_redraw(list, || {
        // SAFETY: messages to our own list box; each string is copied before the call returns.
        unsafe {
            SendMessageW(list, LB_RESETCONTENT, None, None);
            for text in items {
                let text = wide(text);
                SendMessageW(
                    list,
                    LB_ADDSTRING,
                    None,
                    Some(LPARAM(text.as_ptr() as isize)),
                );
            }
        }
    });
    // SAFETY: a message to our own list box.
    unsafe {
        SendMessageW(list, LB_SETCURSEL, Some(WPARAM(selected)), None);
    }
}

/// A tree item's check box as a tree view's state image: none, not checked (1), checked (2).
fn state_image(checked: Option<bool>) -> TREE_VIEW_ITEM_STATE_FLAGS {
    let index: u32 = match checked {
        None => 0,
        Some(false) => 1,
        Some(true) => 2,
    };
    TREE_VIEW_ITEM_STATE_FLAGS(index << 12)
}

/// Sets a tree view item's text and check box.
fn set_tree_item(tree: HWND, handle: HTREEITEM, item: &TreeItem) {
    let mut text = wide(&item.text);
    let update = TVITEMW {
        mask: TVIF_TEXT | TVIF_STATE,
        hItem: handle,
        state: state_image(item.checked),
        stateMask: TVIS_STATEIMAGEMASK,
        pszText: PWSTR(text.as_mut_ptr()),
        ..Default::default()
    };
    // SAFETY: a message to our own tree view about its own item; the text outlives the call.
    unsafe {
        SendMessageW(
            tree,
            TVM_SETITEMW,
            None,
            Some(LPARAM(&update as *const TVITEMW as isize)),
        );
    }
}

/// Gives a tree view check boxes, which Windows asks to be set after the tree is made and before
/// it is filled; its items without one get none.
fn allow_check_boxes(tree: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE,
    };
    // SAFETY: our own tree view's style.
    unsafe {
        let style = GetWindowLongPtrW(tree, GWL_STYLE);
        let checks = TVS_CHECKBOXES as isize;
        if style & checks == 0 {
            SetWindowLongPtrW(tree, GWL_STYLE, style | checks);
        }
    }
}

/// Empties a tree view.
fn clear_tree(tree: HWND) {
    // SAFETY: a message to our own tree view.
    unsafe {
        SendMessageW(tree, TVM_DELETEITEM, None, Some(LPARAM(TVI_ROOT.0)));
    }
}

/// A list's items, `selected` selected, focused and in view.
/// Fills a tree view with `items`, each under the last item one level up, the open ones open,
/// with their check boxes, and selects one.
fn fill_tree(tree: HWND, items: &[TreeItem], selected: usize) {
    if items.iter().any(|item| item.checked.is_some()) {
        allow_check_boxes(tree);
    }
    let mut parents: Vec<HTREEITEM> = Vec::new();
    let mut inserted = Vec::new();
    for item in items {
        let mut text = wide(&item.text);
        parents.truncate(item.level);
        let parent = item
            .level
            .checked_sub(1)
            .and_then(|up| parents.get(up))
            .copied()
            .unwrap_or(TVI_ROOT);
        let insert = TVINSERTSTRUCTW {
            hParent: parent,
            hInsertAfter: TVI_LAST,
            Anonymous: TVINSERTSTRUCTW_0 {
                item: TVITEMW {
                    mask: TVIF_TEXT,
                    pszText: PWSTR(text.as_mut_ptr()),
                    ..Default::default()
                },
            },
        };
        // SAFETY: a message to our own tree view; the item and its text outlive the call.
        let handle = HTREEITEM(unsafe {
            SendMessageW(
                tree,
                TVM_INSERTITEMW,
                None,
                Some(LPARAM(&insert as *const TVINSERTSTRUCTW as isize)),
            )
            .0
        });
        parents.push(handle);
        set_tree_item(tree, handle, item);
        if item.open {
            inserted.push(handle);
        }
    }
    // The open items shown open; a closed one opens with Right Arrow. Each new item's first
    // opening is notified as a user's would be, and was told to Okur: every group a search
    // showed open stayed open after it (performance review 2026-10-25, finding 8).
    FILLING.with(|f| f.set(true));
    for handle in inserted {
        // SAFETY: a message to our own tree view about its own item.
        unsafe {
            SendMessageW(
                tree,
                TVM_EXPAND,
                Some(WPARAM(TVE_EXPAND.0 as usize)),
                Some(LPARAM(handle.0)),
            );
        }
    }
    FILLING.with(|f| f.set(false));
    select_tree_item(tree, selected);
}

/// A tree view's items in order, an item's items right after it, shown or not.
fn tree_order(tree: HWND) -> Vec<HTREEITEM> {
    let next = |relation: u32, item: isize| {
        // SAFETY: a query on our own tree view, from an item it gave (or none for its root).
        unsafe {
            SendMessageW(
                tree,
                TVM_GETNEXTITEM,
                Some(WPARAM(relation as usize)),
                Some(LPARAM(item)),
            )
        }
        .0
    };
    let mut order = Vec::new();
    // The items still to visit, the next one last.
    let mut pending = vec![next(TVGN_ROOT, 0)];
    while let Some(item) = pending.pop() {
        if item == 0 {
            continue;
        }
        order.push(HTREEITEM(item));
        pending.push(next(TVGN_NEXT, item));
        pending.push(next(TVGN_CHILD, item));
    }
    order
}

fn select_tree_item(tree: HWND, index: usize) {
    if let Some(item) = tree_order(tree).get(index) {
        // SAFETY: a message to our own tree view about its own item.
        unsafe {
            SendMessageW(
                tree,
                TVM_SELECTITEM,
                Some(WPARAM(TVGN_CARET as usize)),
                Some(LPARAM(item.0)),
            );
        }
    }
}

fn set_items(list: HWND, items: &[String], selected: usize) {
    without_redraw(list, || insert_items(list, items));
    select_item(list, selected);
}

/// A list view's items put in it, in place of what it had.
fn insert_items(list: HWND, items: &[String]) {
    // SAFETY: messages to our own list view; each text outlives its call.
    unsafe {
        SendMessageW(list, LVM_DELETEALLITEMS, None, None);
        SendMessageW(list, LVM_SETITEMCOUNT, Some(WPARAM(items.len())), None);
        for (index, text) in items.iter().enumerate() {
            let mut text = wide(text);
            let item = LVITEMW {
                mask: LVIF_TEXT,
                iItem: i32::try_from(index).unwrap_or(i32::MAX),
                pszText: PWSTR(text.as_mut_ptr()),
                ..Default::default()
            };
            SendMessageW(
                list,
                LVM_INSERTITEMW,
                None,
                Some(LPARAM(&item as *const LVITEMW as isize)),
            );
        }
    }
}

/// A check list's item checked or not: its state image, 2 checked and 1 not.
fn check_item(list: HWND, index: usize, checked: bool) {
    let image = if checked { 2u32 } else { 1 };
    let state = LVITEMW {
        mask: LVIF_STATE,
        state: windows::Win32::UI::Controls::LIST_VIEW_ITEM_STATE_FLAGS(image << 12),
        stateMask: LVIS_STATEIMAGEMASK,
        ..Default::default()
    };
    // SAFETY: a message to our own list view; the item state outlives the call.
    unsafe {
        SendMessageW(
            list,
            LVM_SETITEMSTATE,
            Some(WPARAM(index)),
            Some(LPARAM(&state as *const LVITEMW as isize)),
        );
    }
}

fn select_item(list: HWND, index: usize) {
    let state = LVITEMW {
        mask: LVIF_STATE,
        state: windows::Win32::UI::Controls::LIST_VIEW_ITEM_STATE_FLAGS(
            LVIS_SELECTED.0 | LVIS_FOCUSED.0,
        ),
        stateMask: windows::Win32::UI::Controls::LIST_VIEW_ITEM_STATE_FLAGS(
            LVIS_SELECTED.0 | LVIS_FOCUSED.0,
        ),
        ..Default::default()
    };
    // SAFETY: messages to our own list view; the item state outlives the call.
    unsafe {
        SendMessageW(
            list,
            LVM_SETITEMSTATE,
            Some(WPARAM(index)),
            Some(LPARAM(&state as *const LVITEMW as isize)),
        );
        SendMessageW(
            list,
            LVM_ENSUREVISIBLE,
            Some(WPARAM(index)),
            Some(LPARAM(0)),
        );
    }
}

/// Sets the value of a control: `item` is its window (a radio group's box), `index` its
/// place in the description.
fn set(window: HWND, index: usize, item: HWND, kind: &ControlKind, value: &Value) {
    // SAFETY: messages to our own controls; the strings outlive each call.
    unsafe {
        match (kind, value) {
            (ControlKind::Choice { .. }, Value::Selected(i)) => {
                SendMessageW(item, CB_SETCURSEL, Some(WPARAM(*i)), None);
            }
            (ControlKind::CheckBox { .. }, Value::Checked(checked)) => {
                let state = if *checked { BST_CHECKED } else { BST_UNCHECKED };
                SendMessageW(item, BM_SETCHECK, Some(WPARAM(state.0 as usize)), None);
            }
            (ControlKind::Slider { .. }, Value::Number(number)) => {
                SendMessageW(
                    item,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(*number as isize)),
                );
                show_slider_value(item, *number);
            }
            (
                ControlKind::List { .. }
                | ControlKind::CheckList { .. }
                | ControlKind::Table { .. },
                Value::Selected(i),
            ) => select_item(item, *i),
            (ControlKind::CheckList { .. }, Value::ItemChecked(i, checked)) => {
                check_item(item, *i, *checked)
            }
            (ControlKind::Tree { .. }, Value::Selected(i)) => select_tree_item(item, *i),
            (ControlKind::Tree { .. }, Value::TreeItem(i, changed)) => {
                if let Some(handle) = tree_order(item).get(*i) {
                    set_tree_item(item, *handle, changed);
                }
            }
            (ControlKind::ListBox { .. }, Value::Selected(i)) => {
                SendMessageW(item, LB_SETCURSEL, Some(WPARAM(*i)), None);
            }
            (
                ControlKind::Radios { options, .. } | ControlKind::RadioButtons { options, .. },
                Value::Selected(chosen),
            ) => {
                for option in 0..options.len() {
                    let id = template::radio_id(index, option);
                    if let Ok(radio) = GetDlgItem(Some(window), i32::from(id)) {
                        let state = if option == *chosen {
                            BST_CHECKED
                        } else {
                            BST_UNCHECKED
                        };
                        SendMessageW(radio, BM_SETCHECK, Some(WPARAM(state.0 as usize)), None);
                    }
                }
            }
            (ControlKind::Text { .. }, Value::Text(text)) => {
                let text = wide(text);
                let _ = SetWindowTextW(item, windows::core::PCWSTR(text.as_ptr()));
            }
            // A text field's text, selected, so that what is typed replaces it.
            (ControlKind::Edit { .. }, Value::Text(text)) => {
                let text = wide(text);
                let _ = SetWindowTextW(item, windows::core::PCWSTR(text.as_ptr()));
                SendMessageW(item, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
            }
            // An editable combo box's text goes to its edit: set on the combo box, the item that
            // matches it is selected in the list, and the field was read with its position, "board
            // 1 of 1", where JAWS Find's has none [obs, 506/s12].
            (ControlKind::ComboEdit { .. }, Value::Text(text)) => {
                let text = wide(text);
                let edit = windows::Win32::UI::WindowsAndMessaging::FindWindowExW(
                    Some(item),
                    None,
                    windows::core::w!("Edit"),
                    None,
                )
                .unwrap_or(item);
                let _ = SetWindowTextW(edit, windows::core::PCWSTR(text.as_ptr()));
            }
            _ => {}
        }
    }
}

/// The open dialog `id`'s window, and the window id, place and kind of its `control`.
fn find(id: u32, control: &str) -> Option<(HWND, u16, usize, ControlKind)> {
    // The dialog's own windows, its pane's too.
    DIALOGS.with(|d| {
        d.try_borrow().ok().and_then(|d| {
            d.iter().filter(|o| o.id == id).find_map(|dialog| {
                let (index, c) = dialog
                    .controls
                    .iter()
                    .enumerate()
                    .find(|(_, c)| c.name == control)?;
                Some((dialog.window, c.window_id, index, c.kind.clone()))
            })
        })
    })
}

/// Sets the value of `control` in dialog `id`, as Okur asks.
pub fn set_value(id: u32, control: &str, value: Value) {
    if let Some((window, window_id, index, kind)) = find(id, control) {
        // SAFETY: a child of our own dialog.
        if let Ok(item) = unsafe { GetDlgItem(Some(window), i32::from(window_id)) } {
            remember(window, window_id, value.clone());
            set(window, index, item, &kind, &value);
        }
    }
}

/// Makes `control` of dialog `id` available, or not.
pub fn enable(id: u32, control: &str, enabled: bool) {
    if let Some((window, window_id, _, _)) = find(id, control) {
        // SAFETY: a child of our own dialog.
        if let Ok(item) = unsafe { GetDlgItem(Some(window), i32::from(window_id)) } {
            let _ = unsafe { EnableWindow(item, enabled) };
        }
    }
}

/// Moves the focus of dialog `id` to `control`, as the dialog manager moves it
/// (WM_NEXTDLGCTL), the default button following.
pub fn focus(id: u32, control: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT, WM_NEXTDLGCTL};
    if let Some((window, window_id, index, kind)) = find(id, control) {
        // A radio group's focus is its checked button's, as Tab gives it.
        let window_id = match &kind {
            ControlKind::Radios { options, .. } | ControlKind::RadioButtons { options, .. } => {
                (0..options.len())
                    .map(|option| template::radio_id(index, option))
                    .find(|&radio| {
                        // SAFETY: a child of our own dialog, and a query of its state.
                        unsafe { GetDlgItem(Some(window), i32::from(radio)) }.is_ok_and(|item| {
                            unsafe { SendMessageW(item, BM_GETCHECK, None, None) }.0
                                == BST_CHECKED.0 as isize
                        })
                    })
                    .unwrap_or_else(|| template::radio_id(index, 0))
            }
            _ => window_id,
        };
        // SAFETY: a child of our own dialog (or of its pane), and a message to the dialog,
        // whose dialog manager keeps the focus.
        if let Ok(item) = unsafe { GetDlgItem(Some(window), i32::from(window_id)) } {
            let root = unsafe { GetAncestor(window, GA_ROOT) };
            let _ = unsafe {
                SendMessageW(
                    root,
                    WM_NEXTDLGCTL,
                    Some(WPARAM(item.0 as usize)),
                    Some(LPARAM(1)),
                )
            };
        }
    }
}

/// Shows the system's Open dialog over dialog `id`, modal, and puts the file chosen in its
/// edit `control`, reported as the edit's change. `folder` none opens Windows' sounds
/// (`%WINDIR%\Media`).
pub fn open_file(id: u32, control: &str, filter: &[(String, String)], folder: Option<&str>) {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_HIDEREADONLY, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST,
        OPENFILENAMEW,
    };
    let Some((window, _, _, _)) = find(id, control) else {
        return;
    };
    let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain(std::iter::once(0)).collect() };
    let filter: Vec<u16> = filter
        .iter()
        .flat_map(|(name, pattern)| wide(name).into_iter().chain(wide(pattern)))
        .chain(std::iter::once(0))
        .collect();
    let folder = folder
        .map(str::to_owned)
        .or_else(sounds_folder)
        .map(|f| wide(&f));
    let mut file = vec![0u16; 1024];
    let mut request = OPENFILENAMEW {
        lStructSize: u32::try_from(std::mem::size_of::<OPENFILENAMEW>()).unwrap_or(0),
        hwndOwner: window,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(file.as_mut_ptr()),
        nMaxFile: u32::try_from(file.len()).unwrap_or(0),
        lpstrInitialDir: folder
            .as_ref()
            .map_or(PCWSTR::null(), |f| PCWSTR(f.as_ptr())),
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    // SAFETY: every buffer the request points to outlives the modal call.
    if !unsafe { GetOpenFileNameW(&mut request) }.as_bool() {
        return;
    }
    let length = file.iter().position(|&c| c == 0).unwrap_or(file.len());
    let Some(path) = file.get(..length).map(String::from_utf16_lossy) else {
        return;
    };
    set_value(id, control, Value::Text(path.clone()));
    crate::win32::tell(&FromUi::DialogChanged {
        id,
        control: control.to_owned(),
        value: Value::Text(path),
    });
}

/// Windows' sounds folder, `%WINDIR%\Media`.
fn sounds_folder() -> Option<String> {
    use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;
    let mut buffer = [0u16; 260];
    // SAFETY: a buffer of the length given.
    let length = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) };
    let windows = String::from_utf16_lossy(buffer.get(..usize::try_from(length).ok()?)?);
    (!windows.is_empty()).then(|| format!("{windows}\\Media"))
}

/// Makes the button `control` the default of dialog `id`, the one Enter presses, framed as
/// the default.
pub fn make_default(id: u32, control: &str) {
    let buttons = DIALOGS.with(|d| {
        d.try_borrow_mut().ok().and_then(|mut d| {
            let dialog = d.iter_mut().find(|o| o.id == id)?;
            let window = dialog.window;
            let buttons: Vec<(u16, bool)> = dialog
                .controls
                .iter_mut()
                .filter_map(|c| match &mut c.kind {
                    ControlKind::Button { default } => {
                        *default = c.name == control;
                        Some((c.window_id, *default))
                    }
                    _ => None,
                })
                .collect();
            Some((window, buttons))
        })
    });
    let Some((window, buttons)) = buttons else {
        return;
    };
    for (window_id, default) in buttons {
        // SAFETY: messages to our own dialog and its buttons.
        unsafe {
            if let Ok(button) = GetDlgItem(Some(window), i32::from(window_id)) {
                let style = if default {
                    BS_DEFPUSHBUTTON
                } else {
                    BS_PUSHBUTTON
                };
                SendMessageW(button, BM_SETSTYLE, Some(WPARAM(style)), Some(LPARAM(1)));
            }
            if default {
                SendMessageW(
                    window,
                    DM_SETDEFID,
                    Some(WPARAM(usize::from(window_id))),
                    None,
                );
            }
        }
    }
}

/// Replaces the items of the list `control` in dialog `id`.
pub fn set_list(id: u32, control: &str, items: &[String], selected: usize) {
    if let Some((window, window_id, index, kind)) = find(id, control) {
        // SAFETY: a child of our own dialog.
        if let Ok(list) = unsafe { GetDlgItem(Some(window), i32::from(window_id)) } {
            // An editable combo box's items are its history, and its text the item selected
            // (none past the end).
            if matches!(kind, ControlKind::ComboEdit { .. }) {
                let text = items.get(selected).cloned().unwrap_or_default();
                remember(window, window_id, Value::Text(text.clone()));
                let refilled = ControlKind::ComboEdit {
                    text,
                    history: items.to_vec(),
                };
                fill(window, index, list, &refilled);
                return;
            }
            remember(window, window_id, Value::Selected(selected));
            if matches!(kind, ControlKind::ListBox { .. }) {
                fill_list_box(list, items, selected);
                return;
            }
            // A drop-down list's choices (Voice Adjustment's Profile Name after Save As).
            if matches!(kind, ControlKind::Choice { .. }) {
                let refilled = ControlKind::Choice {
                    options: items.to_vec(),
                    selected,
                };
                fill(window, index, list, &refilled);
                return;
            }
            set_items(list, items, selected);
        }
    }
}

/// Shows a context menu of `items` on `control` of dialog `id` (at a tree's selected item),
/// and tells Okur what was chosen.
pub fn show_menu(id: u32, control: &str, items: &[okur_ui_model::MenuEntry]) {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::Controls::{TVGN_CARET, TVM_GETITEMRECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, TrackPopupMenu, MF_CHECKED, MF_GRAYED,
        MF_STRING, TPM_LEFTALIGN, TPM_RETURNCMD,
    };
    let Some((window, window_id, _, kind)) = find(id, control) else {
        return;
    };
    // SAFETY: our own dialog's child, a menu of our own made and destroyed here, and its
    // modal loop on this thread.
    let chosen = unsafe {
        let Ok(item) = GetDlgItem(Some(window), i32::from(window_id)) else {
            return;
        };
        // Where: the tree's selected item, else the control's corner.
        let mut corner = POINT { x: 8, y: 8 };
        if matches!(kind, ControlKind::Tree { .. }) {
            let selected = SendMessageW(
                item,
                TVM_GETNEXTITEM,
                Some(WPARAM(TVGN_CARET as usize)),
                None,
            );
            let mut rect = RECT::default();
            // TVM_GETITEMRECT takes the item in the rectangle's place: written unaligned,
            // as a RECT's four-byte alignment is not an isize's (performance review
            // 2026-10-25, finding 12).
            std::ptr::write_unaligned((&mut rect as *mut RECT).cast::<isize>(), selected.0);
            if SendMessageW(
                item,
                TVM_GETITEMRECT,
                Some(WPARAM(1)),
                Some(LPARAM(&mut rect as *mut RECT as isize)),
            )
            .0 != 0
            {
                corner = POINT {
                    x: rect.left,
                    y: rect.bottom,
                };
            }
        }
        let _ = ClientToScreen(item, &mut corner);
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        for (index, entry) in items.iter().enumerate() {
            let text = wide(&entry.text);
            let mut flags = MF_STRING;
            if entry.checked {
                flags |= MF_CHECKED;
            }
            if !entry.enabled {
                flags |= MF_GRAYED;
            }
            let _ = AppendMenuW(menu, flags, index + 1, windows::core::PCWSTR(text.as_ptr()));
        }
        let command = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_RETURNCMD,
            corner.x,
            corner.y,
            None,
            window,
            None,
        );
        let _ = DestroyMenu(menu);
        usize::try_from(command.0)
            .ok()
            .and_then(|c| c.checked_sub(1))
    };
    crate::win32::tell(&FromUi::DialogMenuChosen {
        id,
        control: control.to_owned(),
        index: chosen,
    });
}

/// Replaces tree `control` of dialog `id` with `items`, `selected` selected: the settings a
/// search found.
pub fn set_tree(id: u32, control: &str, items: &[TreeItem], selected: usize) {
    let Some((window, window_id, _, ControlKind::Tree { .. })) = find(id, control) else {
        return;
    };
    // SAFETY: a child of our own dialog.
    if let Ok(tree) = unsafe { GetDlgItem(Some(window), i32::from(window_id)) } {
        remember(window, window_id, Value::Selected(selected));
        clear_tree(tree);
        fill_tree(tree, items, selected);
    }
}

/// Replaces a check list's rows and their checks, and selects one.
pub fn set_rows(id: u32, control: &str, rows: &[Vec<String>], checked: &[bool], selected: usize) {
    let Some((window, window_id, _, ControlKind::CheckList { .. })) = find(id, control) else {
        return;
    };
    // SAFETY: a child of our own dialog.
    if let Ok(list) = unsafe { GetDlgItem(Some(window), i32::from(window_id)) } {
        remember(window, window_id, Value::Selected(selected));
        fill_rows(list, rows, Some(selected));
        for (index, checked) in checked.iter().enumerate() {
            check_item(list, index, *checked);
        }
    }
}

/// Closes dialog `id`, and gives the focus back to the window it came from when the focus is
/// in it. A dialog opened from this one (Virtual HTML Features opens the Heading List in its
/// place) is in front already, keeps it, and will give the focus back to this one's window.
pub fn close(id: u32) {
    let window = DIALOGS.with(|d| {
        d.try_borrow_mut().ok().and_then(|mut d| {
            let index = d.iter().position(|o| o.id == id)?;
            let dialog = d.remove(index);
            // Its pane goes with it, a child of its window.
            d.retain(|o| o.id != id);
            for other in d.iter_mut().filter(|o| o.previous == dialog.window) {
                other.previous = dialog.previous;
            }
            Some((dialog.window, dialog.previous))
        })
    });
    if let Some((window, previous)) = window {
        hand_on_owned(window);
        // SAFETY: a query, destroying our own dialog, then a query on the window it came
        // from.
        let in_front = unsafe { GetForegroundWindow() } == window;
        let _ = unsafe { DestroyWindow(window) };
        if in_front && unsafe { IsWindow(Some(previous)) }.as_bool() {
            bring_forward(previous);
        }
    }
}

/// Gives the dialogs `window` owns to its own owner before it is destroyed: Windows destroys
/// owned windows with their owner, and a list chosen in Virtual HTML Features, opened over it
/// and so owned by it (`show`), takes its place as it closes [obs, 226/s3]. Shown and gone
/// with it, the list had left the focus nowhere and Okur believing it open (performance
/// review 2026-10-25, finding 1).
fn hand_on_owned(window: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindow, SetWindowLongPtrW, GWLP_HWNDPARENT, GW_OWNER,
    };
    // SAFETY: a query on our own dialog.
    let owner = unsafe { GetWindow(window, GW_OWNER) }.unwrap_or_default();
    let owned: Vec<HWND> = DIALOGS.with(|d| {
        d.try_borrow()
            .map(|d| d.iter().map(|o| o.window).collect())
            .unwrap_or_default()
    });
    for dialog in owned {
        // SAFETY: queries on our own dialogs, then a change of the owner of one of them
        // (GWLP_HWNDPARENT of a top-level window is its owner).
        unsafe {
            if GetWindow(dialog, GW_OWNER).is_ok_and(|o| o == window) {
                SetWindowLongPtrW(dialog, GWLP_HWNDPARENT, owner.0 as isize);
            }
        }
    }
}

/// The control dialog `window`'s own Escape presses, anywhere in it: its Cancel and close box's
/// too.
fn escape_key(window: HWND) -> Option<String> {
    let escape = chord("Escape")?;
    DIALOGS.with(|d| {
        d.try_borrow().ok().and_then(|d| {
            d.iter().find(|o| o.window == window).and_then(|o| {
                o.keys
                    .iter()
                    .find(|(chord, _, from)| *chord == escape && from.is_none())
                    .map(|(_, control, _)| control.clone())
            })
        })
    })
}

/// A key down in one of our dialogs that is one of its own keys: the focus to its control,
/// or its press told to Okur. True when it was one.
fn dialog_key(msg: &MSG) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_MENU};
    use windows::Win32::UI::WindowsAndMessaging::{IsChild, WM_KEYDOWN, WM_SYSKEYDOWN};
    if msg.message != WM_KEYDOWN && msg.message != WM_SYSKEYDOWN {
        return false;
    }
    let Ok(vk) = u16::try_from(msg.wParam.0) else {
        return false;
    };
    // SAFETY: the key state of this thread's input.
    let held = |key: VIRTUAL_KEY| unsafe { GetKeyState(i32::from(key.0)) } < 0;
    let pressed: Chord = (vk, held(VK_CONTROL), held(VK_SHIFT), held(VK_MENU));
    // SAFETY: the focus of this thread's input.
    let focused = unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() };
    let found = DIALOGS.with(|d| {
        d.try_borrow().ok().and_then(|d| {
            d.iter()
                // SAFETY: our own dialog and the window the message is for.
                .filter(|o| {
                    o.window == msg.hwnd || unsafe { IsChild(o.window, msg.hwnd) }.as_bool()
                })
                .find_map(|o| {
                    // The focused window itself: a pane's controls have ids of the dialog's.
                    let in_control = |from: &Option<String>| {
                        from.as_ref().is_none_or(|from| {
                            o.controls.iter().any(|c| {
                                &c.name == from
                                    // SAFETY: a child of our own dialog.
                                    && unsafe { GetDlgItem(Some(o.window), i32::from(c.window_id)) }
                                        .is_ok_and(|item| item == focused)
                            })
                        })
                    };
                    let (_, control, _) = o
                        .keys
                        .iter()
                        .find(|(chord, _, from)| *chord == pressed && in_control(from))?;
                    let button = o
                        .controls
                        .iter()
                        .find(|c| &c.name == control)
                        .map(|c| matches!(c.kind, ControlKind::Button { .. }));
                    Some((o.id, control.clone(), button))
                })
        })
    });
    let Some((id, control, button)) = found else {
        return false;
    };
    match button {
        Some(false) => focus(id, &control),
        Some(true) | None => crate::win32::tell(&FromUi::DialogPressed { id, control }),
    }
    true
}

/// Lets the dialog manager handle `msg` (Tab, access keys, Enter, Escape); true when it did.
/// A dialog's own keys come first ([`dialog_key`]). A wizard's pages are left to their sheet's
/// ([`crate::wizard::is_dialog_message`]).
pub fn is_dialog_message(msg: &MSG) -> bool {
    if dialog_key(msg) {
        return true;
    }
    let windows: Vec<HWND> = DIALOGS.with(|d| {
        d.try_borrow()
            .map(|d| d.iter().filter(|o| !o.page).map(|o| o.window).collect())
            .unwrap_or_default()
    });
    windows
        .into_iter()
        // SAFETY: our own dialogs, and the message the loop just received.
        .any(|w| unsafe { IsDialogMessageW(w, msg) }.as_bool())
}

/// Brings `window` to the front. Windows lets only the foreground thread give the
/// foreground away, so the input of the window in front is attached for the switch, as
/// `okur-lab front` does.
pub(crate) fn bring_forward(window: HWND) {
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

/// What a window id of dialog `window` stands for: the dialog's id, the control's name and
/// kind, and the value last told; for a radio button, its group and the option.
struct Hit {
    dialog: u32,
    name: String,
    kind: ControlKind,
    window_id: u16,
    last: Option<Value>,
    option: Option<usize>,
}

fn lookup(window: HWND, control_id: u16) -> Option<Hit> {
    let radio = template::radio_of(control_id);
    DIALOGS.with(|d| {
        d.try_borrow().ok().and_then(|d| {
            let dialog = d.iter().find(|o| o.window == window)?;
            let control = match radio {
                Some((index, _)) => dialog.controls.get(index)?,
                None => dialog.controls.iter().find(|c| c.window_id == control_id)?,
            };
            Some(Hit {
                dialog: dialog.id,
                name: control.name.clone(),
                kind: control.kind.clone(),
                window_id: control.window_id,
                last: control.last.clone(),
                option: radio.map(|(_, option)| option),
            })
        })
    })
}

/// The control of dialog `window` that has the focus.
fn focused_hit(window: HWND) -> Option<Hit> {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetFocus;
    use windows::Win32::UI::WindowsAndMessaging::GetDlgCtrlID;
    // SAFETY: plain Win32 queries; the focus may be any window of this thread.
    let id = unsafe { GetDlgCtrlID(GetFocus()) };
    lookup(window, u16::try_from(id).ok()?)
}

fn remember(window: HWND, window_id: u16, value: Value) {
    DIALOGS.with(|d| {
        if let Ok(mut d) = d.try_borrow_mut() {
            if let Some(dialog) = d.iter_mut().find(|o| o.window == window) {
                if let Some(control) = dialog
                    .controls
                    .iter_mut()
                    .find(|c| c.window_id == window_id)
                {
                    control.last = Some(value);
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
fn changed(window: HWND, hit: &Hit, value: Value) {
    if hit.last.as_ref() == Some(&value) {
        return;
    }
    remember(window, hit.window_id, value.clone());
    crate::win32::tell(&FromUi::DialogChanged {
        id: hit.dialog,
        control: hit.name.clone(),
        value,
    });
}

/// Presses the dialog's default button, as a double click on a list does.
fn press_default(window: HWND) {
    let default = DIALOGS.with(|d| {
        d.try_borrow().ok().and_then(|d| {
            let dialog = d.iter().find(|o| o.window == window)?;
            let button = dialog
                .controls
                .iter()
                .find(|c| matches!(c.kind, ControlKind::Button { default: true }))?;
            Some((dialog.id, button.name.clone()))
        })
    });
    if let Some((id, control)) = default {
        crate::win32::tell(&FromUi::DialogPressed { id, control });
    }
}

/// Takes the controls of a wizard's page, created by its sheet from `spec`, as an open
/// dialog's: their values are set and told as any dialog's are. `ids` are the template's
/// window ids, in the description's order.
pub(crate) fn adopt_page(window: HWND, spec: &DialogSpec, ids: &[u16]) {
    let controls = spec
        .controls
        .iter()
        .zip(ids)
        .map(|(c, &window_id)| OpenControl {
            window_id,
            name: c.id.clone(),
            kind: c.kind.clone(),
            last: initial(&c.kind),
        })
        .collect();
    DIALOGS.with(|d| {
        if let Ok(mut d) = d.try_borrow_mut() {
            d.push(OpenDialog {
                window,
                id: spec.id,
                previous: HWND::default(),
                controls,
                page: true,
                keys: Vec::new(),
            });
        }
    });
    fill_all(window, &spec.controls, ids);
}

/// Forgets a wizard's page as its sheet removes it.
pub(crate) fn forget_page(window: HWND) {
    DIALOGS.with(|d| {
        if let Ok(mut d) = d.try_borrow_mut() {
            d.retain(|o| o.window != window);
        }
    });
}

pub(crate) unsafe extern "system" fn dialog_proc(
    window: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match msg {
        // The dialog manager puts the focus on the first control.
        // A pane takes no focus as it is made: the focus stays where it is.
        WM_INITDIALOG => isize::from(lparam.0 != PANE),
        WM_COMMAND => {
            let control_id = (wparam.0 & 0xFFFF) as u16;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u32;
            let item = HWND(lparam.0 as *mut _);
            if control_id == IDCANCEL {
                // Cancel, Escape, the close box: the dialog goes, and Okur hears of it. A dialog
                // whose Escape is its own key stays, and Okur hears that key's press (Settings
                // Center asks first whether to save its changes).
                if let Some(id) = dialog_id(window) {
                    match escape_key(window) {
                        Some(control) => {
                            crate::win32::tell(&FromUi::DialogPressed { id, control });
                        }
                        None => {
                            close(id);
                            crate::win32::tell(&FromUi::DialogClosed { id });
                        }
                    }
                }
                return 1;
            }
            // Enter in a list box of a dialog with no default button: the item is chosen, as
            // Enter on a notification in JAWS's Notification History shows its details
            // [obs, 774/s11].
            if control_id == template::IDOK && lookup(window, control_id).is_none() {
                if let Some(list) = focused_hit(window)
                    .filter(|hit| matches!(hit.kind, ControlKind::ListBox { .. }))
                {
                    crate::win32::tell(&FromUi::DialogPressed {
                        id: list.dialog,
                        control: list.name,
                    });
                    return 1;
                }
            }
            let Some(hit) = lookup(window, control_id) else {
                return 0;
            };
            match (&hit.kind, hit.option) {
                (ControlKind::Radios { .. } | ControlKind::RadioButtons { .. }, Some(option))
                    if code == BN_CLICKED =>
                {
                    changed(window, &hit, Value::Selected(option));
                }
                (ControlKind::Button { .. }, _) if code == BN_CLICKED => {
                    crate::win32::tell(&FromUi::DialogPressed {
                        id: hit.dialog,
                        control: hit.name,
                    });
                }
                (ControlKind::CheckBox { .. }, _) if code == BN_CLICKED => {
                    // SAFETY: a state query on our own check box.
                    let state = unsafe { SendMessageW(item, BM_GETCHECK, None, None) };
                    let checked = state.0 == BST_CHECKED.0 as isize;
                    changed(window, &hit, Value::Checked(checked));
                }
                // A text field tells its text as it is typed.
                (ControlKind::Edit { .. }, _) if code == EN_CHANGE => {
                    changed(window, &hit, Value::Text(window_text(item)));
                }
                // An editable combo box tells its text as it is typed, or chosen from its list.
                (ControlKind::ComboEdit { .. }, _) if code == CBN_EDITCHANGE => {
                    changed(window, &hit, Value::Text(window_text(item)));
                }
                (ControlKind::ComboEdit { .. }, _) if code == CBN_SELCHANGE => {
                    changed(window, &hit, Value::Text(chosen_text(item)));
                }
                (ControlKind::ListBox { .. }, _) if code == LBN_SELCHANGE => {
                    // SAFETY: a state query on our own list box.
                    let index = unsafe { SendMessageW(item, LB_GETCURSEL, None, None) };
                    if let Ok(index) = usize::try_from(index.0) {
                        changed(window, &hit, Value::Selected(index));
                    }
                }
                (ControlKind::Choice { .. }, _) if code == CBN_SELCHANGE => {
                    // SAFETY: a state query on our own combo box.
                    let index = unsafe { SendMessageW(item, CB_GETCURSEL, None, None) };
                    if let Ok(index) = usize::try_from(index.0) {
                        changed(window, &hit, Value::Selected(index));
                    }
                }
                _ => return 0,
            }
            1
        }
        WM_NOTIFY => {
            // SAFETY: WM_NOTIFY's lparam is the sender's NMHDR, an NMLISTVIEW for a list
            // view's item change.
            let header = unsafe { &*(lparam.0 as *const NMHDR) };
            // A list cleared all at once: no LVN_DELETEITEM for each of its rows.
            if header.code == LVN_DELETEALLITEMS {
                return answer(window, 1);
            }
            // Only what is handled below looks the control up, which copies its items: each
            // other notification did, 0.4 s for a list of 2000 links refilled (performance
            // review 2026-10-25, finding 6).
            if !matches!(
                header.code,
                TVN_SELCHANGEDW
                    | NM_DBLCLK
                    | TVN_KEYDOWN
                    | NM_TVSTATEIMAGECHANGING
                    | TVN_ITEMEXPANDEDW
                    | LVN_ITEMCHANGED
                    | LVN_KEYDOWN
                    | LVN_ENDLABELEDITW
            ) {
                return 0;
            }
            let Ok(control_id) = u16::try_from(header.idFrom) else {
                return 0;
            };
            let Some(hit) = lookup(window, control_id) else {
                return 0;
            };
            if matches!(hit.kind, ControlKind::Tree { .. }) {
                match header.code {
                    TVN_SELCHANGEDW => {
                        // SAFETY: a tree view's selection change sends an NMTREEVIEWW.
                        let change = unsafe { &*(lparam.0 as *const NMTREEVIEWW) };
                        let order = tree_order(header.hwndFrom);
                        if let Some(index) = order.iter().position(|i| *i == change.itemNew.hItem) {
                            changed(window, &hit, Value::Selected(index));
                        }
                    }
                    NM_DBLCLK => press_default(window),
                    // Space changes the selected item's value, Shift+Space back (JAWS's
                    // Settings Center): Okur changes it, and the tree view's own check box turn
                    // and incremental search are kept out.
                    TVN_KEYDOWN => {
                        // SAFETY: a tree view's key notification sends an NMTVKEYDOWN.
                        let key = unsafe { &*(lparam.0 as *const NMTVKEYDOWN) };
                        if VIRTUAL_KEY(key.wVKey) != VK_SPACE {
                            return 0;
                        }
                        // SAFETY: the key state of this thread's input.
                        let back = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
                        crate::win32::tell(&FromUi::DialogKey {
                            id: hit.dialog,
                            control: hit.name.clone(),
                            key: if back {
                                DialogKey::ChangeBack
                            } else {
                                DialogKey::Change
                            },
                        });
                        return answer(window, 1);
                    }
                    // A check box turned by the tree view itself (Space, a click): refused,
                    // Okur turns it.
                    NM_TVSTATEIMAGECHANGING => return answer(window, 1),
                    TVN_ITEMEXPANDEDW if FILLING.with(std::cell::Cell::get) => return 0,
                    TVN_ITEMEXPANDEDW => {
                        // SAFETY: a tree view's expansion sends an NMTREEVIEWW.
                        let change = unsafe { &*(lparam.0 as *const NMTREEVIEWW) };
                        let open = change.itemNew.state.0 & TVIS_EXPANDED.0 != 0;
                        let order = tree_order(header.hwndFrom);
                        if let Some(index) = order.iter().position(|i| *i == change.itemNew.hItem) {
                            changed(window, &hit, Value::ItemOpened(index, open));
                        }
                    }
                    _ => return 0,
                }
                return 1;
            }
            if !matches!(
                hit.kind,
                ControlKind::List { .. }
                    | ControlKind::CheckList { .. }
                    | ControlKind::Table { .. }
            ) {
                return 0;
            }
            match header.code {
                LVN_ITEMCHANGED => {
                    // SAFETY: as above.
                    let change = unsafe { &*(lparam.0 as *const NMLISTVIEW) };
                    let now_selected = change.uNewState & LVIS_SELECTED.0 != 0
                        && change.uOldState & LVIS_SELECTED.0 == 0;
                    if let (true, Ok(index)) = (now_selected, usize::try_from(change.iItem)) {
                        changed(window, &hit, Value::Selected(index));
                    }
                    // A check box turned: its state image, 1 not checked and 2 checked; one
                    // set from none is the list being filled.
                    let image = |state: u32| (state & LVIS_STATEIMAGEMASK.0) >> 12;
                    let (old, new) = (image(change.uOldState), image(change.uNewState));
                    if let (true, Ok(index)) = (
                        old != 0 && new != 0 && old != new,
                        usize::try_from(change.iItem),
                    ) {
                        changed(window, &hit, Value::ItemChecked(index, new == 2));
                    }
                }
                NM_DBLCLK => press_default(window),
                // Delete and F2 on a check list's item: JAWS's rules list deletes or renames
                // the rule [obs, 748/s20; "To edit the selected item press f2.", 748/s7].
                LVN_KEYDOWN if matches!(hit.kind, ControlKind::CheckList { .. }) => {
                    // SAFETY: a list view's key notification sends an NMLVKEYDOWN.
                    let pressed = unsafe { &*(lparam.0 as *const NMLVKEYDOWN) };
                    let key = match VIRTUAL_KEY(pressed.wVKey) {
                        VK_DELETE => Some(DialogKey::Delete),
                        VK_F2 => Some(DialogKey::Rename),
                        _ => None,
                    };
                    if let Some(key) = key {
                        crate::win32::tell(&FromUi::DialogKey {
                            id: hit.dialog,
                            control: hit.name.clone(),
                            key,
                        });
                    }
                    // F2 edits the selected row's name in place, where the list allows it.
                    if key == Some(DialogKey::Rename)
                        && matches!(
                            hit.kind,
                            ControlKind::CheckList {
                                renamable: true,
                                ..
                            }
                        )
                    {
                        // SAFETY: messages to our own list view.
                        unsafe {
                            let row = SendMessageW(
                                header.hwndFrom,
                                LVM_GETNEXTITEM,
                                Some(WPARAM(usize::MAX)),
                                Some(LPARAM(LVNI_SELECTED as isize)),
                            );
                            SendMessageW(
                                header.hwndFrom,
                                LVM_EDITLABELW,
                                Some(WPARAM(row.0 as usize)),
                                None,
                            );
                        }
                    }
                }
                // The name edited in place: told, and the edit declined, the rows coming back
                // from Okur with the rule renamed, or as they were [obs, 751/s20].
                LVN_ENDLABELEDITW if matches!(hit.kind, ControlKind::CheckList { .. }) => {
                    // SAFETY: the end of a label edit sends an NMLVDISPINFOW.
                    let info = unsafe { &*(lparam.0 as *const NMLVDISPINFOW) };
                    if !info.item.pszText.is_null() {
                        // SAFETY: the edited text, null-terminated, for this notification's
                        // time.
                        let text = unsafe { info.item.pszText.to_string() }.unwrap_or_default();
                        if let Ok(row) = usize::try_from(info.item.iItem) {
                            changed(window, &hit, Value::ItemRenamed(row, text));
                        }
                    }
                    return 0;
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
                show_slider_value(item, position);
                if let Some(hit) = lookup(window, control_id) {
                    changed(window, &hit, Value::Number(position));
                }
            }
            1
        }
        // A dialog destroyed other than by `close` (with a window that owned it): forgotten,
        // and Okur hears it closed, which it would never hear otherwise (performance review
        // 2026-10-25, finding 9). `close` forgets its dialog first, so it is not told twice; a
        // pane, a child sharing its dialog's id, is not the dialog.
        WM_NCDESTROY => {
            use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT};
            // SAFETY: a query on the window being destroyed, still valid in this message.
            let top_level = unsafe { GetAncestor(window, GA_ROOT) } == window;
            if top_level {
                if let Some(id) = forget(window) {
                    crate::win32::tell(&FromUi::DialogClosed { id });
                }
            }
            0
        }
        _ => 0,
    }
}

/// Forgets the dialog `window` and its pane, as `close` does; its id, when it was one of ours.
fn forget(window: HWND) -> Option<u32> {
    DIALOGS.with(|d| {
        d.try_borrow_mut().ok().and_then(|mut d| {
            let id = d.iter().find(|o| o.window == window)?.id;
            let previous = d.iter().find(|o| o.window == window)?.previous;
            d.retain(|o| o.id != id);
            for other in d.iter_mut().filter(|o| o.previous == window) {
                other.previous = previous;
            }
            Some(id)
        })
    })
}

/// A slider's value in its label, after the label's colon: "Rate: 0", as JAWS's Voice
/// Adjustment shows "Rate: 20" [obs, the walk of Voice Adjustment]. The label is the static
/// made just before the slider (`template::push_labelled`).
fn show_slider_value(slider: HWND, value: i32) {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindow, GW_HWNDPREV};
    // SAFETY: our own dialog's static before our own slider.
    let Ok(label) = (unsafe { GetWindow(slider, GW_HWNDPREV) }) else {
        return;
    };
    let text = window_text(label);
    let Some((name, _)) = text.rsplit_once(':') else {
        return;
    };
    let shown = wide(&format!("{name}: {value}"));
    // SAFETY: our own static, and a string that outlives the call.
    unsafe {
        let _ = SetWindowTextW(label, windows::core::PCWSTR(shown.as_ptr()));
    }
}

/// A window's text: an editable combo box's typed text.
fn window_text(item: HWND) -> String {
    // SAFETY: plain reads of our own control's text into a buffer of the length it reports.
    unsafe {
        let length = usize::try_from(GetWindowTextLengthW(item)).unwrap_or(0);
        let mut buffer = vec![0u16; length + 1];
        let copied = usize::try_from(GetWindowTextW(item, &mut buffer)).unwrap_or(0);
        String::from_utf16_lossy(buffer.get(..copied).unwrap_or_default())
    }
}

/// The text of the item chosen in a combo box's list.
fn chosen_text(item: HWND) -> String {
    // SAFETY: messages to our own combo box; the buffer has room for the length it reports
    // and its terminating null.
    unsafe {
        let index = SendMessageW(item, CB_GETCURSEL, None, None).0;
        let Ok(chosen) = usize::try_from(index) else {
            return window_text(item);
        };
        let length = SendMessageW(item, CB_GETLBTEXTLEN, Some(WPARAM(chosen)), None).0;
        let Ok(length) = usize::try_from(length) else {
            return window_text(item);
        };
        let mut buffer = vec![0u16; length + 1];
        let copied = SendMessageW(
            item,
            CB_GETLBTEXT,
            Some(WPARAM(chosen)),
            Some(LPARAM(buffer.as_mut_ptr() as isize)),
        )
        .0;
        let copied = usize::try_from(copied).unwrap_or(0);
        String::from_utf16_lossy(buffer.get(..copied).unwrap_or_default())
    }
}
