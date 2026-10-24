//! What Okur and `okur-ui`, the process that draws Okur's windows, say to each other.
//!
//! Okur decides what is shown; `okur-ui` only draws it, natively on each platform, and tells
//! Okur what the user did to a window (see `docs/references/helper-ui-architecture.md`).
//! Messages are JSON, one per line: Okur writes them to the process's standard input, and
//! reads its answers from its standard output.

use serde::{Deserialize, Serialize};

/// A span of the viewer's text: `length` characters from `column` of `line`. An empty span
/// marks a position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSpan {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

/// A dialog: standard controls in tab order, with the exact labels the user hears. An "&" in
/// a label marks its access key, as in Windows dialog resources. Layout is the renderer's:
/// a description never says where a control goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogSpec {
    /// Okur's number for this dialog; events carry it back.
    pub id: u32,
    pub title: String,
    pub controls: Vec<Control>,
    /// Keys of the whole dialog, wherever its focus is: JAWS's Settings Center's Ctrl+E to its
    /// search box, Ctrl+Shift+D to the Default settings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<DialogShortcut>,
}

/// A key of a whole dialog ("Ctrl+E", "Ctrl+Shift+D"): it moves the focus to `control`, or,
/// where no control has that id, or it is a button, reports it pressed
/// ([`FromUi::DialogPressed`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogShortcut {
    pub key: String,
    pub control: String,
    /// Only while the focus is in this control (Settings Center's Down from its search box);
    /// none, anywhere in the dialog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

/// One control of a dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Control {
    /// Names the control in events and updates. The buttons "ok" and "cancel" are the
    /// dialog's own: Escape and the close box press "cancel".
    pub id: String,
    pub label: String,
    pub kind: ControlKind,
    #[serde(default = "enabled")]
    pub enabled: bool,
}

fn enabled() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ControlKind {
    /// A group box around the controls that follow it, up to the next group or the buttons.
    Group,
    CheckBox {
        checked: bool,
    },
    /// A drop-down list of `options`, one selected.
    Choice {
        options: Vec<String>,
        selected: usize,
    },
    /// An editable combo box: a text typed in, its earlier values in its list (JAWS Find's
    /// "Find what"). Its value is the text.
    ComboEdit {
        text: String,
        history: Vec<String>,
    },
    /// A slider from `min` to `max`.
    Slider {
        min: i32,
        max: i32,
        value: i32,
    },
    /// A push button; Enter presses the `default` one.
    Button {
        default: bool,
    },
    /// A list of items, one selected. Its label names it and is not shown: JAWS's "links
    /// List view".
    List {
        items: Vec<String>,
        selected: usize,
    },
    /// A list of rows, each with a check box, one selected. Its label names it and is not
    /// shown; under column headers, or one column without them: JAWS's Save Temporary
    /// Customizations, " List view | Hide ComplementaryRegion 1 from the top with text:
    /// "Advertisement" checked | 1 of 1" [obs, 742/s18]; its rules list, Rule and Set for,
    /// "Okur lab rule | Page checked" [obs, 748/s12].
    CheckList {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        checked: Vec<bool>,
        selected: usize,
        /// F2 edits the selected row's first cell, its name, in place: JAWS's rules list,
        /// "To edit the selected item press f2." [obs, 751/s18-s20].
        #[serde(default)]
        renamable: bool,
    },
    /// A read-only text the user can Tab to and read with the reading keys, named by its
    /// label shown above it: JAWS's rules dialog, "View or Change Where Rules are Applied read
    /// only edit" [obs, 748/s17].
    Note {
        text: String,
    },
    /// A list of rows under column headers, named by its label (not shown), with none
    /// selected until the user picks one: JAWS's PlaceMarker List, "PlaceMarker List view |
    /// Not Selected Water in a closed bottle is allowed; meals are not. | Document | 1 of 1"
    /// [obs, 702/s18]. With rows and none selected, the first has the focus.
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        selected: Option<usize>,
    },
    /// A list box of items, one selected, named by its label shown above it: JAWS's Window
    /// List, "Running Applications List box" [obs, 511/s1].
    ListBox {
        items: Vec<String>,
        selected: usize,
    },
    /// A tree of items, one selected, every item open. Its label names it and is not shown:
    /// JAWS's "Regions Tree view". The items are in order, each at its level (0 at the top),
    /// an item's children right after it; `selected` counts in that order.
    Tree {
        items: Vec<TreeItem>,
        selected: usize,
    },
    /// Radio buttons in a group box that the label names; each option's label has its own
    /// access key.
    Radios {
        options: Vec<String>,
        selected: usize,
    },
    /// Radio buttons standing alone, with no group box, the label not shown; those not yet
    /// possible are unavailable (`enabled`, one per option): a wizard's choices, as JAWS's
    /// Flexible Web offers "Create a new customization" alone at first [obs, 705/s3].
    RadioButtons {
        options: Vec<String>,
        selected: usize,
        enabled: Vec<bool>,
    },
    /// A read-only line of text, outside the tab order: the Links List's link address.
    Text {
        text: String,
    },
    /// A text field named by its label (JAWS's Jump To Line, "Enter Line Number (1 to 73):
    /// edit" [obs, 559/s1]). Its value is the text, told as it is typed.
    Edit {
        text: String,
    },
    /// The dialog's own text, as a message box shows it: a question it asks (the question
    /// before Okur exits). Static text, outside the tab order, the whole dialog wide; the
    /// label is not shown.
    Message {
        text: String,
    },
}

/// An item of a [`ControlKind::Tree`]: its text and its level, 0 at the top.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeItem {
    pub text: String,
    pub level: usize,
    /// Its items shown; a closed item opens with Right Arrow: JAWS's "Current Site:
    /// localhost Closed" [obs, 748/s9, s10].
    #[serde(default = "opened")]
    pub open: bool,
    /// A check box's state, for an item that is one (a setting turned on or off: JAWS's Quick
    /// Settings, "Automatic Activation not checked"); `None` for any other item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
}

fn opened() -> bool {
    true
}

/// A control's value, as the user set it or Okur sets it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Value {
    Checked(bool),
    Selected(usize),
    /// A check list's item checked or not, by its index.
    ItemChecked(usize, bool),
    /// A check list's item renamed in place (F2), by its index.
    ItemRenamed(usize, String),
    /// A tree's item opened (`true`) or closed, by its index; and from Okur, a tree item's
    /// text and check box changed in place, by its index, the tree otherwise as it is.
    ItemOpened(usize, bool),
    TreeItem(usize, TreeItem),
    Number(i32),
    Text(String),
}

/// A wizard's buttons available on its page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WizardButtons {
    pub back: bool,
    /// Next; without it, Finish takes its place.
    pub next: bool,
    /// Finish available; without Next it is shown either way, unavailable until it can finish
    /// (JAWS's Save Temporary Customizations, until the rule has a name [obs, 740]).
    pub finish: bool,
}

/// A wizard: one page at a time under Back, Next (or Finish) and Cancel, the window titled
/// with the page's title, as a Windows property-sheet wizard is (JAWS's Flexible Web [obs,
/// 705/s3, s9]). The page is a dialog description without buttons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WizardSpec {
    /// Okur's number for the wizard; events carry it back, as a dialog's.
    pub id: u32,
    pub page: DialogSpec,
    pub buttons: WizardButtons,
}

/// What Okur asks `okur-ui` to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToUi {
    /// Show a dialog in front, with the focus on its first control; one already showing
    /// with this id comes to the front instead.
    DialogShow { dialog: DialogSpec },
    /// Set a control's value.
    DialogSet {
        id: u32,
        control: String,
        value: Value,
    },
    /// Replace a list's items, and select one; an editable combo box's history, its text the
    /// item selected (none past the end).
    DialogItems {
        id: u32,
        control: String,
        items: Vec<String>,
        selected: usize,
    },
    /// Show Okur's main window in front, made or remade as described.
    MainWindowShow { window: MainWindowSpec },
    /// Show a context menu on a control's selected item (Shift+F10 on a tree's): its items,
    /// one checked. The choice comes back as [`FromUi::DialogMenuChosen`].
    DialogMenu {
        id: u32,
        control: String,
        items: Vec<MenuEntry>,
    },
    /// Replace a tree's items, and select one: the settings a search found.
    DialogTree {
        id: u32,
        control: String,
        items: Vec<TreeItem>,
        selected: usize,
    },
    /// Replace a check list's rows and their checks, and select one: the rules of the area
    /// chosen in JAWS's rules dialog [obs, 748/s12, s15].
    DialogRows {
        id: u32,
        control: String,
        rows: Vec<Vec<String>>,
        checked: Vec<bool>,
        selected: usize,
    },
    /// Make a control available or not, as a button is while the list has no item chosen.
    DialogEnable {
        id: u32,
        control: String,
        enabled: bool,
    },
    /// Make a button the dialog's default, which Enter presses: JAWS's PlaceMarker List
    /// makes MoveTo the default once a PlaceMarker is chosen, Add before.
    DialogDefault { id: u32, control: String },
    /// Move the dialog's focus to a control: the rules' list after Disable, as JAWS's
    /// Manage Notification Rules gives it back the focus.
    DialogFocus { id: u32, control: String },
    /// Show the system's Open dialog over dialog `id` for a file, as JAWS's Select Sound...
    /// opens Windows' (802/s12): the file types offered as (name, pattern) pairs, and the
    /// folder it opens in (none: the system's sounds). The path chosen is put in the edit
    /// `control` and reported as its change; cancelled, nothing is.
    FileOpen {
        id: u32,
        control: String,
        filter: Vec<(String, String)>,
        folder: Option<String>,
    },
    /// Close a dialog, or a wizard.
    DialogClose { id: u32 },
    /// Show a wizard in front on its first page, the focus on the page's first control.
    WizardShow { wizard: WizardSpec },
    /// Turn wizard `id` to another page, the focus on its first control. Its buttons
    /// ("back", "next", "finish") come back as [`FromUi::DialogPressed`].
    WizardPage {
        id: u32,
        page: DialogSpec,
        buttons: WizardButtons,
    },
    /// Wizard `id`'s buttons on the page shown: Finish once the rule has a name [obs, 742/s26].
    WizardButtons { id: u32, buttons: WizardButtons },
    /// Show the Virtual Viewer's text on screen, without taking the focus, with `mark` on
    /// what the virtual cursor read last. Okur keeps the keys: the window only shows them
    /// to someone watching, as JAWS's does.
    ViewerShow {
        title: String,
        lines: Vec<String>,
        mark: TextSpan,
    },
    /// Move the viewer's mark.
    ViewerMark { mark: TextSpan },
    /// Close the viewer's window.
    ViewerClose,
}

/// What `okur-ui` tells Okur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromUi {
    /// The viewer's window took the focus (a click, Alt+Tab) or was closed by its close
    /// button. JAWS ends its viewer then, and so does Okur.
    ViewerDismissed,
    /// The user changed a control's value.
    DialogChanged {
        id: u32,
        control: String,
        value: Value,
    },
    /// The user pressed a button other than "cancel"; a wizard's are "back", "next" and
    /// "finish".
    DialogPressed { id: u32, control: String },
    /// The dialog was cancelled (its Cancel button, Escape, its close box) and is gone.
    DialogClosed { id: u32 },
    /// One of the main window's menu items chosen, by its id; its closing, "exit".
    MainMenuCommand { id: String },
    /// A context menu's item chosen, by its index; none when it was closed without a choice.
    DialogMenuChosen {
        id: u32,
        control: String,
        index: Option<usize>,
    },
    /// A key pressed on a list or tree that does something to its selected item: Delete, F2,
    /// Space.
    DialogKey {
        id: u32,
        control: String,
        key: DialogKey,
    },
}

/// Okur's main window (Insert+J), as JAWS's: its title, its text (Okur's version), and its
/// menu bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainWindowSpec {
    pub title: String,
    pub lines: Vec<String>,
    pub menus: Vec<MenuSpec>,
}

/// A menu of the menu bar: its label, an "&" before its access key, and its items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuSpec {
    pub label: String,
    pub items: Vec<MenuItemSpec>,
}

/// A menu's item: a command told back by its id, or with items of its own, a submenu. Its
/// key, shown after it, as JAWS's menus show theirs ("Notification History INSERT+SPACE, N").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuItemSpec {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<MenuItemSpec>,
}

/// The command the main window's closing asks for (Alt+F4, its close box).
pub const MAIN_WINDOW_EXIT: &str = "exit";

/// An item of a context menu ([`ToUi::DialogMenu`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuEntry {
    pub text: String,
    #[serde(default)]
    pub checked: bool,
}

/// A key on a list's item ([`FromUi::DialogKey`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogKey {
    /// Delete: remove it.
    Delete,
    /// F2: rename it.
    Rename,
    /// Space on a tree's item: change its value to the next one (JAWS's Settings Center).
    Change,
    /// Shift+Space: to the one before.
    ChangeBack,
}

/// One message as a line of JSON, without the line break.
pub fn encode<T: Serialize>(message: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(message)
}

/// The message on one line of JSON.
pub fn decode<'a, T: Deserialize<'a>>(line: &'a str) -> Result<T, serde_json::Error> {
    serde_json::from_str(line.trim_end())
}

/// Where `column` (in characters) of `line` falls in `lines` joined by one line break each,
/// counted in UTF-16 units: the positions of a Windows rich edit control holding them.
/// Positions past a line's end stop at its end; past the last line, at the text's end.
pub fn utf16_offset(lines: &[String], line: usize, column: usize) -> usize {
    let before: usize = lines
        .iter()
        .take(line)
        .map(|l| l.encode_utf16().count() + 1)
        .sum();
    let within = lines.get(line).map_or(0, |l| {
        l.chars().take(column).map(char::len_utf16).sum::<usize>()
    });
    if line >= lines.len() {
        before.saturating_sub(1)
    } else {
        before + within
    }
}

/// Where each of `lines` starts in them joined by one line break each, in UTF-16 units, and,
/// last, where the text ends: made once for a text shown, so that a mark is placed without
/// counting every line before it again, several times per key in the Virtual Viewer
/// (performance review 2026-10-23, T15).
pub fn utf16_line_starts(lines: &[String]) -> Vec<usize> {
    let mut starts = Vec::with_capacity(lines.len() + 1);
    let mut at = 0;
    for line in lines {
        starts.push(at);
        at += line.encode_utf16().count() + 1;
    }
    starts.push(at.saturating_sub(1));
    starts
}

/// [`utf16_offset`] with the lines' starts made by [`utf16_line_starts`].
pub fn utf16_offset_in(lines: &[String], starts: &[usize], line: usize, column: usize) -> usize {
    match (lines.get(line), starts.get(line)) {
        (Some(text), Some(&start)) => {
            start
                + text
                    .chars()
                    .take(column)
                    .map(char::len_utf16)
                    .sum::<usize>()
        }
        _ => starts.last().copied().unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Offsets by the lines' starts are the offsets counted from the text's start, for every
    /// line and column, past a line's end and past the last line, with characters of two
    /// UTF-16 units.
    #[test]
    fn offsets_by_line_starts_are_offsets_counted_from_the_start() {
        for lines in [
            Vec::new(),
            vec![String::new()],
            vec![
                "ab".to_owned(),
                "𝄞 cd".to_owned(),
                String::new(),
                "é f".to_owned(),
            ],
        ] {
            let starts = utf16_line_starts(&lines);
            for line in 0..lines.len() + 2 {
                for column in 0..8 {
                    assert_eq!(
                        utf16_offset_in(&lines, &starts, line, column),
                        utf16_offset(&lines, line, column),
                        "{lines:?} {line} {column}"
                    );
                }
            }
        }
    }

    #[test]
    fn messages_are_one_line_of_json_each_way() {
        let show = ToUi::ViewerShow {
            title: "Virtual Viewer".into(),
            lines: vec!["About Windows".into(), "OK".into()],
            mark: TextSpan {
                line: 0,
                column: 0,
                length: 13,
            },
        };
        let line = encode(&show).unwrap_or_default();
        assert!(!line.contains('\n'));
        assert!(line.starts_with(r#"{"type":"viewer_show""#), "{line}");
        assert_eq!(decode::<ToUi>(&format!("{line}\r\n")).ok(), Some(show));
        assert_eq!(
            decode::<FromUi>(r#"{"type":"viewer_dismissed"}"#).ok(),
            Some(FromUi::ViewerDismissed)
        );
    }

    #[test]
    fn dialogs_round_trip_with_enabled_by_default() {
        let dialog = DialogSpec {
            id: 3,
            title: "Okur Settings".into(),
            controls: vec![
                Control {
                    id: "speech".into(),
                    label: "Speech".into(),
                    kind: ControlKind::Group,
                    enabled: true,
                },
                Control {
                    id: "rate".into(),
                    label: "&Rate:".into(),
                    kind: ControlKind::Slider {
                        min: -10,
                        max: 10,
                        value: 0,
                    },
                    enabled: true,
                },
            ],
            keys: Vec::new(),
        };
        let show = ToUi::DialogShow { dialog };
        let line = encode(&show).unwrap_or_default();
        assert_eq!(decode::<ToUi>(&line).ok(), Some(show));
        let control: Result<Control, _> =
            decode(r#"{"id":"ok","label":"OK","kind":{"kind":"button","default":true}}"#);
        assert_eq!(control.map(|c| c.enabled).ok(), Some(true));
        let changed = FromUi::DialogChanged {
            id: 3,
            control: "rate".into(),
            value: Value::Number(-2),
        };
        let line = encode(&changed).unwrap_or_default();
        assert_eq!(decode::<FromUi>(&line).ok(), Some(changed));
    }

    #[test]
    fn offsets_count_utf16_units_and_one_break_per_line() {
        let lines = vec!["ab".to_owned(), "çağ 𝄞x".to_owned(), String::new()];
        assert_eq!(utf16_offset(&lines, 0, 0), 0);
        assert_eq!(utf16_offset(&lines, 0, 2), 2);
        // Past the line's end: its end.
        assert_eq!(utf16_offset(&lines, 0, 9), 2);
        assert_eq!(utf16_offset(&lines, 1, 0), 3);
        // "𝄞" is two UTF-16 units.
        assert_eq!(utf16_offset(&lines, 1, 5), 3 + 6);
        assert_eq!(utf16_offset(&lines, 2, 0), 3 + 7 + 1);
        // Past the last line: the text's end.
        assert_eq!(utf16_offset(&lines, 3, 0), 3 + 7 + 1);
    }
}
