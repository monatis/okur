//! A dialog description laid out and encoded as a Windows dialog template (`DLGTEMPLATEEX`)
//! in memory, so that Windows creates it as it creates a dialog from a resource: a `#32770`
//! window whose standard controls the dialog manager runs (Tab, access keys, the default
//! button, Escape) and UI Automation exposes as any application's. JAWS's own dialogs are
//! built this way.
//!
//! Layout, in dialog units ([`layout`]): a form, or a list dialog as JAWS's are. Pure, so it
//! is tested anywhere.

use okur_ui_model::{ControlKind, DialogSpec};

const WS_CHILD: u32 = 0x4000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WS_GROUP: u32 = 0x0002_0000;
const WS_VSCROLL: u32 = 0x0020_0000;
const WS_POPUP: u32 = 0x8000_0000;
const WS_CAPTION: u32 = 0x00C0_0000;
const WS_SYSMENU: u32 = 0x0008_0000;
const DS_SETFONT: u32 = 0x40;
const DS_MODALFRAME: u32 = 0x80;
const DS_CENTER: u32 = 0x800;
const WS_EX_TOPMOST: u32 = 0x8;
const WS_EX_CONTROLPARENT: u32 = 0x0001_0000;
const BS_PUSHBUTTON: u32 = 0;
const BS_DEFPUSHBUTTON: u32 = 1;
const BS_AUTOCHECKBOX: u32 = 3;
const BS_GROUPBOX: u32 = 7;
const CBS_DROPDOWNLIST: u32 = 3;
const BS_AUTORADIOBUTTON: u32 = 9;
const WS_BORDER: u32 = 0x0080_0000;
const ES_AUTOHSCROLL: u32 = 0x80;
const ES_READONLY: u32 = 0x800;
const LVS_REPORT: u32 = 0x1;
const TVS_HASBUTTONS: u32 = 0x1;
const TVS_HASLINES: u32 = 0x2;
const TVS_LINESATROOT: u32 = 0x4;
const TVS_SHOWSELALWAYS: u32 = 0x20;
const LVS_SINGLESEL: u32 = 0x4;
const LVS_SHOWSELALWAYS: u32 = 0x8;
const LVS_NOCOLUMNHEADER: u32 = 0x4000;
const TBS_AUTOTICKS: u32 = 1;

/// The dialog manager's ids for OK and Cancel: Enter without a default button presses OK,
/// and Escape and the close box press Cancel.
pub const IDOK: u16 = 1;
pub const IDCANCEL: u16 = 2;
/// Labels have no id of their own.
const LABEL_ID: u16 = 0xFFFF;
/// Other controls are numbered from here, in the description's order.
const FIRST_ID: u16 = 100;
/// Radio buttons are numbered from here, a block of ids for each group.
const FIRST_RADIO_ID: u16 = 1000;
const RADIO_STRIDE: u16 = 16;

/// Dialog units.
const MARGIN: i16 = 7;
const WIDTH: i16 = 280;
const LABEL_WIDTH: i16 = 84;
const ROW_GAP: i16 = 4;
const BUTTON_WIDTH: i16 = 50;
const BUTTON_HEIGHT: i16 = 14;
/// A list dialog's list: eight rows.
const LIST_HEIGHT: i16 = 86;

/// The font JAWS's and Windows's dialogs use.
const FONT: &str = "Segoe UI";
const FONT_POINTS: u16 = 9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class {
    Button,
    Static,
    Edit,
    ComboBox,
    Named(&'static str),
}

/// One child window of the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub class: Class,
    pub style: u32,
    pub x: i16,
    pub y: i16,
    pub cx: i16,
    pub cy: i16,
    pub id: u16,
    pub text: String,
}

/// A laid-out dialog: its size and items, and which item stands for which control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub width: i16,
    pub height: i16,
    pub items: Vec<Item>,
    /// The window id of each control of the description, by its index there.
    pub ids: Vec<u16>,
}

/// The window id of the control at `index` with `id` in the description.
pub fn window_id(index: usize, id: &str) -> u16 {
    match id {
        "ok" => IDOK,
        "cancel" => IDCANCEL,
        _ => FIRST_ID.saturating_add(u16::try_from(index).unwrap_or(u16::MAX - FIRST_ID)),
    }
}

/// The window id of option `option` of the radio group at `index` in the description.
pub fn radio_id(index: usize, option: usize) -> u16 {
    let index = u16::try_from(index).unwrap_or(0);
    let option = u16::try_from(option).unwrap_or(0);
    FIRST_RADIO_ID
        .saturating_add(index.saturating_mul(RADIO_STRIDE))
        .saturating_add(option)
}

/// The radio group (its index in the description) and option a window id stands for.
pub fn radio_of(id: u16) -> Option<(usize, usize)> {
    let offset = id.checked_sub(FIRST_RADIO_ID)?;
    Some((
        usize::from(offset / RADIO_STRIDE),
        usize::from(offset % RADIO_STRIDE),
    ))
}

/// Lays out `dialog`: a dialog with a list as JAWS's list dialogs are (the list at the top
/// left, the buttons beside it, the radio groups side by side below), any other as a form.
/// Items are in the description's order, which is the tab order.
pub fn layout(dialog: &DialogSpec) -> Layout {
    if dialog
        .controls
        .iter()
        .any(|c| matches!(c.kind, ControlKind::List { .. } | ControlKind::Tree { .. }))
    {
        list_layout(dialog)
    } else {
        form_layout(dialog)
    }
}

/// A radio group's box and buttons, at `x`, `y`, `width` wide; returns the bottom.
fn push_radios(
    items: &mut Vec<Item>,
    index: usize,
    id: u16,
    label: &str,
    options: &[String],
    (x, y, width): (i16, i16, i16),
) -> i16 {
    let rows = i16::try_from(options.len()).unwrap_or(0);
    let height = 12 + rows * 12 + 2;
    items.push(Item {
        class: Class::Button,
        style: WS_CHILD | WS_VISIBLE | WS_GROUP | BS_GROUPBOX,
        x,
        y,
        cx: width,
        cy: height,
        id,
        text: label.to_owned(),
    });
    for (option, text) in options.iter().enumerate() {
        // The first starts the group and is its tab stop; the arrows move between them.
        let first = if option == 0 {
            WS_TABSTOP | WS_GROUP
        } else {
            0
        };
        items.push(Item {
            class: Class::Button,
            style: WS_CHILD | WS_VISIBLE | BS_AUTORADIOBUTTON | first,
            x: x + 6,
            y: y + 12 + i16::try_from(option).unwrap_or(0) * 12,
            cx: width - 12,
            cy: 10,
            id: radio_id(index, option),
            text: text.clone(),
        });
    }
    y + height
}

fn read_only_text(x: i16, y: i16, cx: i16, id: u16) -> Item {
    Item {
        class: Class::Edit,
        style: WS_CHILD | WS_VISIBLE | WS_GROUP | WS_BORDER | ES_READONLY | ES_AUTOHSCROLL,
        x,
        y,
        cx,
        cy: 12,
        id,
        text: String::new(),
    }
}

/// A push button; `first` starts a group. Buttons side by side are one group, as in JAWS's
/// dialogs: the arrows move between them and wrap [obs, 224/s8, s9].
fn button(id: u16, text: &str, default: bool, first: bool, (x, y): (i16, i16)) -> Item {
    let kind = if default {
        BS_DEFPUSHBUTTON
    } else {
        BS_PUSHBUTTON
    };
    let group = if first { WS_GROUP } else { 0 };
    Item {
        class: Class::Button,
        style: WS_CHILD | WS_VISIBLE | WS_TABSTOP | group | kind,
        x,
        y,
        cx: BUTTON_WIDTH,
        cy: BUTTON_HEIGHT,
        id,
        text: text.to_owned(),
    }
}

fn list_layout(dialog: &DialogSpec) -> Layout {
    let mut items = Vec::new();
    let mut ids = Vec::new();
    let list_width = WIDTH - 2 * MARGIN - BUTTON_WIDTH - 6;
    let button_x = WIDTH - MARGIN - BUTTON_WIDTH;
    let mut button_y = MARGIN;
    // Below the list: radio groups two to a row, then the rest.
    let column_width = (WIDTH - 2 * MARGIN - 6) / 2;
    let mut row_top = MARGIN + LIST_HEIGHT + 6;
    let mut column = 0;
    let mut row_bottom = row_top;
    let mut y = row_top;
    let mut after_button = false;
    for (index, control) in dialog.controls.iter().enumerate() {
        let id = window_id(index, &control.id);
        ids.push(id);
        let first = !after_button;
        after_button = matches!(control.kind, ControlKind::Button { .. });
        match &control.kind {
            ControlKind::List { .. } => items.push(Item {
                class: Class::Named("SysListView32"),
                style: WS_CHILD
                    | WS_VISIBLE
                    | WS_BORDER
                    | WS_TABSTOP
                    | WS_GROUP
                    | LVS_REPORT
                    | LVS_SINGLESEL
                    | LVS_SHOWSELALWAYS
                    | LVS_NOCOLUMNHEADER,
                x: MARGIN,
                y: MARGIN,
                cx: list_width,
                cy: LIST_HEIGHT,
                id,
                // Its name, which JAWS's list dialogs show nowhere ("links").
                text: control.label.clone(),
            }),
            ControlKind::Tree { .. } => items.push(Item {
                class: Class::Named("SysTreeView32"),
                style: WS_CHILD
                    | WS_VISIBLE
                    | WS_BORDER
                    | WS_TABSTOP
                    | WS_GROUP
                    | TVS_HASBUTTONS
                    | TVS_HASLINES
                    | TVS_LINESATROOT
                    | TVS_SHOWSELALWAYS,
                x: MARGIN,
                y: MARGIN,
                cx: list_width,
                cy: LIST_HEIGHT,
                id,
                // Its name, which JAWS's tree dialogs show nowhere ("Regions").
                text: control.label.clone(),
            }),
            ControlKind::Button { default } => {
                items.push(button(
                    id,
                    &control.label,
                    *default,
                    first,
                    (button_x, button_y),
                ));
                button_y += BUTTON_HEIGHT + 4;
            }
            ControlKind::Radios { options, .. } => {
                let x = MARGIN + column * (column_width + 6);
                let bottom = push_radios(
                    &mut items,
                    index,
                    id,
                    &control.label,
                    options,
                    (x, row_top, column_width),
                );
                row_bottom = row_bottom.max(bottom);
                column += 1;
                if column == 2 {
                    column = 0;
                    row_top = row_bottom + 4;
                }
                y = row_bottom + 4;
            }
            ControlKind::Text { .. } => {
                items.push(read_only_text(MARGIN, y, WIDTH - 2 * MARGIN, id));
                y += 12 + ROW_GAP;
            }
            ControlKind::CheckBox { .. } => {
                items.push(Item {
                    class: Class::Button,
                    style: WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_AUTOCHECKBOX,
                    x: MARGIN,
                    y,
                    cx: WIDTH - 2 * MARGIN,
                    cy: 10,
                    id,
                    text: control.label.clone(),
                });
                y += 10 + ROW_GAP;
            }
            // Not used in list dialogs yet.
            ControlKind::Group | ControlKind::Choice { .. } | ControlKind::Slider { .. } => {}
        }
    }
    let height = y.max(button_y).max(MARGIN + LIST_HEIGHT) + MARGIN;
    Layout {
        width: WIDTH,
        height,
        items,
        ids,
    }
}

fn form_layout(dialog: &DialogSpec) -> Layout {
    let mut items = Vec::new();
    let mut ids = Vec::new();
    let mut buttons = Vec::new();
    let mut y = MARGIN;
    // The open group: its item's index, and where it starts.
    let mut group: Option<(usize, i16)> = None;
    let inner_left = MARGIN + 6;
    let inner_right = WIDTH - MARGIN - 6;
    let close = |items: &mut Vec<Item>, group: &mut Option<(usize, i16)>, y: &mut i16| {
        if let Some((index, top)) = group.take() {
            *y += 2;
            if let Some(item) = items.get_mut(index) {
                item.cy = *y - top;
            }
            *y += ROW_GAP + 2;
        }
    };
    for (index, control) in dialog.controls.iter().enumerate() {
        let id = window_id(index, &control.id);
        ids.push(id);
        let (left, right) = if group.is_some() {
            (inner_left, inner_right)
        } else {
            (MARGIN, WIDTH - MARGIN)
        };
        match &control.kind {
            ControlKind::Group => {
                close(&mut items, &mut group, &mut y);
                group = Some((items.len(), y));
                items.push(Item {
                    class: Class::Button,
                    style: WS_CHILD | WS_VISIBLE | WS_GROUP | BS_GROUPBOX,
                    x: MARGIN,
                    y,
                    cx: WIDTH - 2 * MARGIN,
                    cy: 0,
                    id,
                    text: control.label.clone(),
                });
                y += 12;
            }
            ControlKind::CheckBox { .. } => {
                items.push(Item {
                    class: Class::Button,
                    style: WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_AUTOCHECKBOX,
                    x: left,
                    y,
                    cx: right - left,
                    cy: 10,
                    id,
                    text: control.label.clone(),
                });
                y += 10 + ROW_GAP;
            }
            ControlKind::Choice { .. } | ControlKind::Slider { .. } | ControlKind::Text { .. } => {
                // The label first: in the tab order before its control, it names it and
                // gives it its access key.
                items.push(Item {
                    class: Class::Static,
                    style: WS_CHILD | WS_VISIBLE,
                    x: left,
                    y: y + 2,
                    cx: LABEL_WIDTH,
                    cy: 8,
                    id: LABEL_ID,
                    text: control.label.clone(),
                });
                let x = left + LABEL_WIDTH + 4;
                let cx = right - left - LABEL_WIDTH - 4;
                let height = match &control.kind {
                    ControlKind::Choice { .. } => {
                        items.push(Item {
                            class: Class::ComboBox,
                            style: WS_CHILD
                                | WS_VISIBLE
                                | WS_TABSTOP
                                | WS_GROUP
                                | CBS_DROPDOWNLIST
                                | WS_VSCROLL,
                            x,
                            y,
                            cx,
                            // The list's height when it drops down.
                            cy: 120,
                            id,
                            text: String::new(),
                        });
                        12
                    }
                    ControlKind::Slider { .. } => {
                        items.push(Item {
                            class: Class::Named("msctls_trackbar32"),
                            style: WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | TBS_AUTOTICKS,
                            x,
                            y,
                            cx,
                            cy: 16,
                            id,
                            text: String::new(),
                        });
                        16
                    }
                    _ => {
                        items.push(read_only_text(x, y, cx, id));
                        12
                    }
                };
                y += height + ROW_GAP;
            }
            ControlKind::Radios { options, .. } => {
                close(&mut items, &mut group, &mut y);
                y = push_radios(
                    &mut items,
                    index,
                    id,
                    &control.label,
                    options,
                    (MARGIN, y, WIDTH - 2 * MARGIN),
                ) + ROW_GAP
                    + 2;
            }
            ControlKind::List { .. } | ControlKind::Tree { .. } => {}
            ControlKind::Button { default } => {
                buttons.push((id, control.label.clone(), *default));
            }
        }
    }
    close(&mut items, &mut group, &mut y);
    if !buttons.is_empty() {
        y += 3;
        let count = i16::try_from(buttons.len()).unwrap_or(1);
        let mut x = WIDTH - MARGIN - count * BUTTON_WIDTH - (count - 1) * 4;
        for (n, (id, text, default)) in buttons.into_iter().enumerate() {
            items.push(button(id, &text, default, n == 0, (x, y)));
            x += BUTTON_WIDTH + 4;
        }
        y += BUTTON_HEIGHT;
    }
    Layout {
        width: WIDTH,
        height: y + MARGIN,
        items,
        ids,
    }
}

fn push_dword(words: &mut Vec<u16>, value: u32) {
    words.push((value & 0xFFFF) as u16);
    words.push((value >> 16) as u16);
}

fn push_text(words: &mut Vec<u16>, text: &str) {
    words.extend(text.encode_utf16());
    words.push(0);
}

/// `layout` as a `DLGTEMPLATEEX` titled `title`, in 32-bit words: Windows wants the
/// template and each item aligned on four bytes.
pub fn encode(title: &str, layout: &Layout) -> Vec<u32> {
    let mut words: Vec<u16> = vec![1, 0xFFFF];
    push_dword(&mut words, 0); // help id
    push_dword(&mut words, WS_EX_TOPMOST | WS_EX_CONTROLPARENT);
    push_dword(
        &mut words,
        WS_POPUP | WS_CAPTION | WS_SYSMENU | DS_SETFONT | DS_MODALFRAME | DS_CENTER,
    );
    words.push(u16::try_from(layout.items.len()).unwrap_or(0));
    for v in [0, 0, layout.width, layout.height] {
        words.push(v as u16);
    }
    words.push(0); // no menu
    words.push(0); // the standard dialog class
    push_text(&mut words, title);
    words.push(FONT_POINTS);
    words.push(400); // weight
    words.push(0x0100); // not italic; DEFAULT_CHARSET
    push_text(&mut words, FONT);
    for item in &layout.items {
        if words.len() % 2 == 1 {
            words.push(0);
        }
        push_dword(&mut words, 0); // help id
        push_dword(&mut words, 0); // extended style
        push_dword(&mut words, item.style);
        for v in [item.x, item.y, item.cx, item.cy] {
            words.push(v as u16);
        }
        push_dword(&mut words, u32::from(item.id));
        match item.class {
            Class::Button => words.extend([0xFFFF, 0x0080]),
            Class::Static => words.extend([0xFFFF, 0x0082]),
            Class::Edit => words.extend([0xFFFF, 0x0081]),
            Class::ComboBox => words.extend([0xFFFF, 0x0085]),
            Class::Named(name) => push_text(&mut words, name),
        }
        push_text(&mut words, &item.text);
        words.push(0); // no creation data
    }
    if words.len() % 2 == 1 {
        words.push(0);
    }
    words
        .chunks(2)
        .map(|pair| {
            let low = u32::from(pair.first().copied().unwrap_or(0));
            let high = u32::from(pair.get(1).copied().unwrap_or(0));
            low | (high << 16)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use okur_ui_model::Control;

    fn control(id: &str, label: &str, kind: ControlKind) -> Control {
        Control {
            id: id.into(),
            label: label.into(),
            kind,
            enabled: true,
        }
    }

    fn sample() -> DialogSpec {
        DialogSpec {
            id: 1,
            title: "Okur Settings".into(),
            controls: vec![
                control("speech", "Speech", ControlKind::Group),
                control(
                    "rate",
                    "&Rate:",
                    ControlKind::Slider {
                        min: -10,
                        max: 10,
                        value: 0,
                    },
                ),
                control(
                    "echo",
                    "Typing &Echo:",
                    ControlKind::Choice {
                        options: vec!["Characters".into(), "None".into()],
                        selected: 0,
                    },
                ),
                control("viewer", "&Show", ControlKind::CheckBox { checked: true }),
                control("ok", "OK", ControlKind::Button { default: true }),
                control("cancel", "Cancel", ControlKind::Button { default: false }),
            ],
        }
    }

    #[test]
    fn controls_are_laid_out_in_order_with_labels_before_their_controls() {
        let layout = layout(&sample());
        assert_eq!(layout.ids, [100, 101, 102, 103, IDOK, IDCANCEL]);
        let kinds: Vec<(&Class, u16, &str)> = layout
            .items
            .iter()
            .map(|i| (&i.class, i.id, i.text.as_str()))
            .collect();
        assert_eq!(
            kinds,
            [
                (&Class::Button, 100, "Speech"),
                (&Class::Static, LABEL_ID, "&Rate:"),
                (&Class::Named("msctls_trackbar32"), 101, ""),
                (&Class::Static, LABEL_ID, "Typing &Echo:"),
                (&Class::ComboBox, 102, ""),
                (&Class::Button, 103, "&Show"),
                (&Class::Button, IDOK, "OK"),
                (&Class::Button, IDCANCEL, "Cancel"),
            ]
        );
        // The group box holds its rows; the buttons sit below it, on one row, at the right,
        // one group that the arrows move in.
        let group = layout.items.first().map(|g| g.y + g.cy).unwrap_or(0);
        let ok = layout.items.get(6).cloned();
        let cancel = layout.items.get(7).cloned();
        assert!(ok.as_ref().is_some_and(|ok| ok.y > group));
        assert!(ok.zip(cancel).is_some_and(|(ok, cancel)| ok.y == cancel.y
            && cancel.x + cancel.cx == WIDTH - MARGIN
            && ok.style & BS_DEFPUSHBUTTON != 0
            && ok.style & WS_GROUP != 0
            && cancel.style & WS_GROUP == 0));
        assert!(layout.height > group);
    }

    #[test]
    fn the_template_is_a_dlgtemplateex_with_aligned_items() {
        let layout = layout(&sample());
        let dwords = encode("Okur Settings", &layout);
        let words: Vec<u16> = dwords
            .iter()
            .flat_map(|d| [(d & 0xFFFF) as u16, (d >> 16) as u16])
            .collect();
        // dlgVer 1, signature 0xFFFF, then help id, extended style, style.
        assert_eq!(words.get(..2), Some(&[1u16, 0xFFFF][..]));
        assert_eq!(words.get(8), Some(&8u16), "eight items");
        // The title follows the menu and class words.
        let title: Vec<u16> = "Okur Settings".encode_utf16().collect();
        assert_eq!(words.get(15..15 + title.len()), Some(&title[..]));
        // The first item (the group box) starts on a four-byte boundary, with the button
        // class's ordinal after its id.
        let font_end = 15 + title.len() + 1 + 3 + FONT.len() + 1;
        let first = font_end + font_end % 2;
        assert_eq!(
            words.get(first + 10..first + 14),
            Some(&[100u16, 0, 0xFFFF, 0x0080][..])
        );
    }
}
