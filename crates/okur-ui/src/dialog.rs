//! Okur's dialogs on Windows: real dialogs (`#32770`) made from an in-memory template
//! ([`crate::template`]), modeless, in front of the application the user was in. The
//! controls start with the description's values; each change, and each button, is told to
//! Okur, which decides what happens.

use std::cell::RefCell;

use okur_ui_model::{ControlKind, DialogSpec, FromUi, TreeItem, Value};
use windows::core::PWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Controls::{
    BST_CHECKED, BST_UNCHECKED, EM_SETSEL, HTREEITEM, LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_STATE,
    LVIF_TEXT, LVIS_FOCUSED, LVIS_SELECTED, LVITEMW, LVM_DELETEALLITEMS, LVM_ENSUREVISIBLE,
    LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMSTATE,
    LVM_SETITEMTEXTW, LVN_ITEMCHANGED, LVS_EX_FULLROWSELECT, NMHDR, NMLISTVIEW, NMTREEVIEWW,
    NM_DBLCLK, TBM_SETPAGESIZE, TBM_SETPOS, TBM_SETRANGEMAX, TBM_SETRANGEMIN, TBM_SETTICFREQ,
    TVE_EXPAND, TVGN_CARET, TVGN_NEXTVISIBLE, TVGN_ROOT, TVIF_TEXT, TVINSERTSTRUCTW,
    TVINSERTSTRUCTW_0, TVITEMW, TVI_LAST, TVI_ROOT, TVM_EXPAND, TVM_GETNEXTITEM, TVM_INSERTITEMW,
    TVM_SELECTITEM, TVN_SELCHANGEDW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateDialogIndirectParamW, DestroyWindow, GetClientRect, GetDlgCtrlID,
    GetDlgItem, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsDialogMessageW, IsWindow, SendMessageW, SetForegroundWindow,
    SetWindowTextW, ShowWindow, BM_GETCHECK, BM_SETCHECK, BN_CLICKED, CBN_EDITCHANGE,
    CBN_SELCHANGE, CB_ADDSTRING, CB_GETCURSEL, CB_GETLBTEXT, CB_GETLBTEXTLEN, CB_RESETCONTENT,
    CB_SETCURSEL, DLGTEMPLATE, EN_CHANGE, LBN_SELCHANGE, LB_ADDSTRING, LB_GETCURSEL,
    LB_RESETCONTENT, LB_SETCURSEL, MSG, SW_SHOW, WM_COMMAND, WM_HSCROLL, WM_INITDIALOG, WM_NOTIFY,
};

use crate::template::{self, IDCANCEL};

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
}

thread_local! {
    static DIALOGS: RefCell<Vec<OpenDialog>> = const { RefCell::new(Vec::new()) };
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// A control's value as the dialog opens: Okur knows it, so it is never told back.
fn initial(kind: &ControlKind) -> Option<Value> {
    match kind {
        ControlKind::Choice { selected, .. }
        | ControlKind::List { selected, .. }
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
        ControlKind::Group | ControlKind::Button { .. } | ControlKind::Message { .. } => None,
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
            });
        }
    });
    for (index, (control, &id)) in spec.controls.iter().zip(&layout.ids).enumerate() {
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
    bring_forward(window);
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
            ControlKind::Text { text } => {
                set(window, index, item, kind, &Value::Text(text.clone()));
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
        SendMessageW(list, LVM_DELETEALLITEMS, None, None);
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
        SendMessageW(list, LB_SETCURSEL, Some(WPARAM(selected)), None);
    }
}

/// A list's items, `selected` selected, focused and in view.
/// Fills a tree view with `items`, each under the last item one level up, every item open, and
/// selects one.
fn fill_tree(tree: HWND, items: &[TreeItem], selected: usize) {
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
        inserted.push(handle);
    }
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
    select_tree_item(tree, selected);
}

/// A tree view's items in order: every item is open, so the visible order is the tree's.
fn tree_order(tree: HWND) -> Vec<HTREEITEM> {
    let mut order = Vec::new();
    // SAFETY: queries on our own tree view.
    let mut item = unsafe {
        SendMessageW(
            tree,
            TVM_GETNEXTITEM,
            Some(WPARAM(TVGN_ROOT as usize)),
            None,
        )
    }
    .0;
    while item != 0 {
        order.push(HTREEITEM(item));
        // SAFETY: as above, from an item it gave.
        item = unsafe {
            SendMessageW(
                tree,
                TVM_GETNEXTITEM,
                Some(WPARAM(TVGN_NEXTVISIBLE as usize)),
                Some(LPARAM(item)),
            )
        }
        .0;
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
    // SAFETY: messages to our own list view; each text outlives its call.
    unsafe {
        SendMessageW(list, LVM_DELETEALLITEMS, None, None);
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
        select_item(list, selected);
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
            }
            (ControlKind::List { .. } | ControlKind::Table { .. }, Value::Selected(i)) => {
                select_item(item, *i)
            }
            (ControlKind::Tree { .. }, Value::Selected(i)) => select_tree_item(item, *i),
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
    DIALOGS.with(|d| {
        d.try_borrow().ok().and_then(|d| {
            let dialog = d.iter().find(|o| o.id == id)?;
            let (index, c) = dialog
                .controls
                .iter()
                .enumerate()
                .find(|(_, c)| c.name == control)?;
            Some((dialog.window, c.window_id, index, c.kind.clone()))
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
            set_items(list, items, selected);
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
            for other in d.iter_mut().filter(|o| o.previous == dialog.window) {
                other.previous = dialog.previous;
            }
            Some((dialog.window, dialog.previous))
        })
    });
    if let Some((window, previous)) = window {
        // SAFETY: a query, destroying our own dialog, then a query on the window it came
        // from.
        let in_front = unsafe { GetForegroundWindow() } == window;
        let _ = unsafe { DestroyWindow(window) };
        if in_front && unsafe { IsWindow(Some(previous)) }.as_bool() {
            bring_forward(previous);
        }
    }
}

/// Lets the dialog manager handle `msg` (Tab, access keys, Enter, Escape); true when it did.
/// A wizard's pages are left to their sheet's ([`crate::wizard::is_dialog_message`]).
pub fn is_dialog_message(msg: &MSG) -> bool {
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
            });
        }
    });
    for (index, (control, &id)) in spec.controls.iter().zip(ids).enumerate() {
        // SAFETY: a child of the page just created.
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
                    _ => return 0,
                }
                return 1;
            }
            if !matches!(
                hit.kind,
                ControlKind::List { .. } | ControlKind::Table { .. }
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
                }
                NM_DBLCLK => press_default(window),
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
                if let Some(hit) = lookup(window, control_id) {
                    changed(window, &hit, Value::Number(position));
                }
            }
            1
        }
        _ => 0,
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
