//! A dialog description laid out and encoded as a Windows dialog template (`DLGTEMPLATEEX`)
//! in memory, so that Windows creates it as it creates a dialog from a resource: a `#32770`
//! window whose standard controls the dialog manager runs (Tab, access keys, the default
//! button, Escape) and UI Automation exposes as any application's. JAWS's own dialogs are
//! built this way.
//!
//! Layout, in dialog units: groups one under the other, each control on a row of its own,
//! labels left of their drop-down lists and sliders, and the buttons in a row at the bottom
//! right. Pure, so it is tested anywhere.

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
const TBS_AUTOTICKS: u32 = 1;

/// The dialog manager's ids for OK and Cancel: Enter without a default button presses OK,
/// and Escape and the close box press Cancel.
pub const IDOK: u16 = 1;
pub const IDCANCEL: u16 = 2;
/// Labels have no id of their own.
const LABEL_ID: u16 = 0xFFFF;
/// Other controls are numbered from here, in the description's order.
const FIRST_ID: u16 = 100;

/// Dialog units.
const MARGIN: i16 = 7;
const WIDTH: i16 = 280;
const LABEL_WIDTH: i16 = 84;
const ROW_GAP: i16 = 4;
const BUTTON_WIDTH: i16 = 50;
const BUTTON_HEIGHT: i16 = 14;

/// The font JAWS's and Windows's dialogs use.
const FONT: &str = "Segoe UI";
const FONT_POINTS: u16 = 9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class {
    Button,
    Static,
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

/// Lays out `dialog`.
pub fn layout(dialog: &DialogSpec) -> Layout {
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
                    style: WS_CHILD | WS_VISIBLE | BS_GROUPBOX,
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
            ControlKind::Choice { .. } | ControlKind::Slider { .. } => {
                let (class, style, cy, height) = match &control.kind {
                    ControlKind::Choice { .. } => (
                        Class::ComboBox,
                        CBS_DROPDOWNLIST | WS_VSCROLL,
                        // The list's height when it drops down.
                        120,
                        12,
                    ),
                    _ => (Class::Named("msctls_trackbar32"), TBS_AUTOTICKS, 16, 16),
                };
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
                items.push(Item {
                    class,
                    style: WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | style,
                    x: left + LABEL_WIDTH + 4,
                    y,
                    cx: right - left - LABEL_WIDTH - 4,
                    cy,
                    id,
                    text: String::new(),
                });
                y += height + ROW_GAP;
            }
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
        for (id, text, default) in buttons {
            let kind = if default {
                BS_DEFPUSHBUTTON
            } else {
                BS_PUSHBUTTON
            };
            items.push(Item {
                class: Class::Button,
                style: WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | kind,
                x,
                y,
                cx: BUTTON_WIDTH,
                cy: BUTTON_HEIGHT,
                id,
                text,
            });
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
        // The group box holds its rows; the buttons sit below it, on one row, at the right.
        let group = layout.items.first().map(|g| g.y + g.cy).unwrap_or(0);
        let ok = layout.items.get(6).cloned();
        let cancel = layout.items.get(7).cloned();
        assert!(ok.as_ref().is_some_and(|ok| ok.y > group));
        assert!(ok.zip(cancel).is_some_and(|(ok, cancel)| ok.y == cancel.y
            && cancel.x + cancel.cx == WIDTH - MARGIN
            && ok.style & BS_DEFPUSHBUTTON != 0));
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
