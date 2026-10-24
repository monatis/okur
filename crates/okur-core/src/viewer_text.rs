//! The Virtual Viewer's text for a window or control (Virtualize Window, Insert+Alt+W;
//! Virtualize Current Control, Insert+Shift+V), from the elements the platform finds on
//! screen.
//!
//! JAWS 2025 shows a window two ways (dev box, 2026-10-15):
//! - **Classic windows** (Win32 dialogs, WinForms, File Explorer) as drawn on screen, by rows:
//!   the title, then each row's text left to right, with labels beside their values, tabs and
//!   buttons on their rows, and text wrapped where the control wraps it. For Local Disk (C:)
//!   Properties: "Type: Local Disk", "Used space: 440,534,675,456 bytes 410 GB", ...,
//!   "OK Cancel Apply".
//! - **Modern apps** (XAML, such as Calculator) as their elements, one per line in tree order,
//!   with the role and state: "Memory recall Button  unavailable", "Display controls group".
//!
//! Okur keeps both, and leaves out what JAWS's screen capture gets wrong: separate controls
//! stay separate words ("Details Drive C:", where JAWS joins "DetailsDrive C:"), a row reads
//! left to right, accelerator marks are not shown, and unnamed groups are not listed.

use crate::aom::Role;
use crate::screen_model::{ScreenRun, Source};

/// How a window's elements become the viewer's lines, and the Okur cursor's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewerLayout {
    /// As drawn on screen, by rows (classic windows).
    #[default]
    Rows,
    /// One element per line in tree order, with role and state (modern apps).
    Elements,
}

/// An element with text, where it is on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenItem {
    pub role: Role,
    /// Its name, or its value for a field.
    pub text: String,
    /// The text as the control draws it, one entry per line, when its wrapping is known.
    pub lines: Vec<String>,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    /// A group box: only its caption, at its top, is text.
    pub caption: bool,
    pub unavailable: bool,
    /// For a check box or radio button.
    pub checked: Option<bool>,
    /// It has the keyboard focus.
    pub focused: bool,
}

impl ScreenItem {
    pub fn new(role: Role, text: impl Into<String>, rect: (i32, i32, i32, i32)) -> Self {
        let (left, top, right, bottom) = rect;
        Self {
            role,
            text: text.into(),
            lines: Vec::new(),
            left,
            top,
            right,
            bottom,
            caption: false,
            unavailable: false,
            checked: None,
            focused: false,
        }
    }

    fn height(&self) -> i32 {
        (self.bottom - self.top).max(1)
    }
}

/// `text` without the invisible marks that steer its direction or joins (Explorer wraps
/// dates in left-to-right marks), which a viewer line would show as stray characters.
fn visible(text: &str) -> String {
    without_marks(text).trim().to_owned()
}

/// `text` without the invisible marks that steer its direction or joins. Windows wraps dates
/// in them (a file's Properties: "\u{200E}Friday, \u{200E}January ..."); JAWS's speech has
/// none [obs, 323/s2, s7].
pub fn without_marks(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !matches!(
                c,
                '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}'
            )
        })
        .collect()
}

/// The height of one line of text in the window: the median height of its one-line items.
fn line_height(items: &[ScreenItem]) -> i32 {
    let mut heights: Vec<i32> = items
        .iter()
        .filter(|i| i.lines.len() <= 1 && !i.caption)
        .map(ScreenItem::height)
        .filter(|h| *h >= 8)
        .collect();
    heights.sort_unstable();
    heights.get(heights.len() / 2).copied().unwrap_or(20)
}

/// The window's text by screen rows: each item's lines placed where they are drawn, rows
/// top to bottom, each row's text left to right ([`crate::screen_model::lines`]).
pub fn rows(items: &[ScreenItem]) -> Vec<String> {
    crate::screen_model::lines(&runs(items))
        .into_iter()
        .map(|line| line.text)
        .collect()
}

/// Each item's lines where they are drawn: a group box's caption at its top, a wrapped text
/// or a multi-line field's lines one under another, a label's line at its top.
pub fn runs(items: &[ScreenItem]) -> Vec<ScreenRun> {
    let line = line_height(items);
    let mut runs: Vec<ScreenRun> = Vec::new();
    for item in items {
        let text = visible(&item.text);
        if text.is_empty() {
            continue;
        }
        if item.caption {
            runs.push(ScreenRun::new(
                text,
                (item.left, item.top, item.right, item.top + line),
                Source::Accessible,
            ));
            continue;
        }
        let first = runs.len();
        // Wrapped as the control wraps it, or a multi-line field's lines.
        let lines: Vec<String> = if item.lines.len() > 1 {
            item.lines.iter().map(|l| visible(l)).collect()
        } else {
            text.lines()
                .map(visible)
                .filter(|l| !l.is_empty())
                .collect()
        };
        let count = i32::try_from(lines.len()).unwrap_or(1).max(1);
        let each = if lines.len() > 1 {
            item.height() / count
        } else if item.role == Role::Text {
            // A label draws its line at its top, however tall the label is ("Drive C:"
            // beside the Details button).
            item.height().min(line)
        } else {
            item.height().min(line * 2)
        };
        for (i, part) in lines.into_iter().enumerate() {
            let top = item.top + each * i32::try_from(i).unwrap_or(0);
            runs.push(ScreenRun::new(
                part,
                (item.left, top, item.right, top + each),
                Source::Accessible,
            ));
        }
        if let Some(run) = runs.get_mut(first).filter(|_| item.focused) {
            run.focused = true;
        }
    }
    runs
}

/// The word an element list gives an element's role ("Button", "group"); text has none.
pub(crate) fn role_word(role: &Role) -> Option<&'static str> {
    use crate::messages::msg;
    Some(
        match role {
            Role::Button => msg::role_button(),
            Role::MenuButton => msg::role_button_menu(),
            Role::CheckBox => msg::role_check_box(),
            Role::RadioButton => msg::role_radio_button(),
            Role::ComboBox => msg::role_combo_box_focus(),
            Role::Edit => msg::role_edit(),
            Role::Tab => msg::role_tab(),
            Role::Group => msg::role_group(),
            Role::Link { .. } => msg::role_link_viewer(),
            Role::MenuItem => msg::role_menu_item(),
            Role::Slider => msg::role_slider(),
            Role::ProgressBar => msg::role_progress_bar(),
            Role::List => msg::role_list(),
            Role::ListItem => msg::role_list_item(),
            Role::Tree => msg::role_tree_view_viewer(),
            Role::TreeItem => msg::role_tree_item(),
            _ => return None,
        }
        .as_str(),
    )
}

/// Each item with text as a run of its own, in the window's order, with its role: a modern
/// app's elements for the Okur cursor, which reads them one a line as JAWS's Scan Cursor does
/// [obs, 1002/s7-s11].
pub fn element_runs(items: &[ScreenItem]) -> Vec<ScreenRun> {
    items
        .iter()
        .filter_map(|item| {
            let text = visible(&item.text);
            // An unnamed combo box or list is said by its role [obs, 1016/s13, s14].
            let unnamed = matches!(item.role, Role::ComboBox | Role::List);
            (!text.is_empty() || unnamed).then(|| {
                let mut run = ScreenRun::new(
                    text,
                    (item.left, item.top, item.right, item.bottom),
                    Source::Accessible,
                )
                .with_role(item.role.clone());
                run.checked = item.checked;
                run.focused = item.focused;
                run
            })
        })
        .collect()
}

/// The window's elements one per line in tree order, as JAWS lists a modern app's: its name,
/// its role, and its state ("Memory recall Button  unavailable"). Unnamed groups, which JAWS
/// lists as a bare "group", are left out.
pub fn elements(items: &[ScreenItem]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| {
            let mut line = visible(&item.text);
            if line.is_empty() {
                return None;
            }
            if let Some(word) = role_word(&item.role) {
                line.push(' ');
                line.push_str(word);
            }
            use crate::messages::msg;
            let state = match item.checked {
                Some(true) => Some(msg::state_checked()),
                Some(false) if matches!(item.role, Role::CheckBox) => {
                    Some(msg::state_not_checked())
                }
                _ => None,
            };
            if let Some(state) = state {
                line.push_str("  ");
                line.push_str(state.as_str());
            }
            if item.unavailable {
                line.push_str("  ");
                line.push_str(msg::state_unavailable_viewer().as_str());
            }
            Some(line)
        })
        .collect()
}

/// The viewer's lines for a window titled `title` (none for a control). An app that draws
/// its own title bar (Calculator) shows its title again at the top; it is read once.
pub fn lines(title: Option<&str>, layout: ViewerLayout, items: &[ScreenItem]) -> Vec<String> {
    let title = title.map(visible).filter(|t| !t.is_empty());
    let mut body = match layout {
        ViewerLayout::Rows => rows(items),
        ViewerLayout::Elements => elements(items),
    };
    if title.is_some() && body.first() == title.as_ref() {
        body.remove(0);
    }
    title.into_iter().chain(body).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(t: &str, left: i32, top: i32, right: i32, bottom: i32) -> ScreenItem {
        ScreenItem::new(Role::Text, t, (left, top, right, bottom))
    }

    /// Local Disk (C:) Properties on the dev box, as UI Automation places its controls.
    #[test]
    fn a_property_page_reads_by_rows() {
        let tab = |t: &str, left: i32, top: i32, right: i32| {
            ScreenItem::new(Role::Tab, t, (left, top, right, top + 24))
        };
        let button = |t: &str, left: i32, top: i32, right: i32, bottom: i32| {
            ScreenItem::new(Role::Button, t, (left, top, right, bottom))
        };
        let mut allow = ScreenItem::new(
            Role::CheckBox,
            "Allow files on this drive to have contents indexed in addition to file properties",
            (1459, 901, 1875, 941),
        );
        allow.lines = vec![
            "Allow files on this drive to have contents indexed in addition to file".to_owned(),
            "properties".to_owned(),
        ];
        let items = vec![
            tab("Security", 1445, 423, 1520),
            tab("Previous Versions", 1520, 423, 1660),
            tab("Quota", 1660, 423, 1720),
            tab("General", 1445, 447, 1520),
            tab("Tools", 1520, 447, 1580),
            tab("Hardware", 1580, 447, 1660),
            tab("Sharing", 1660, 447, 1730),
            text("Type:", 1459, 567, 1543, 585),
            text("Local Disk", 1557, 567, 1885, 595),
            text("File system:", 1459, 593, 1543, 611),
            text("NTFS", 1557, 593, 1885, 621),
            text("Used space:", 1487, 629, 1579, 647),
            text("440,534,675,456 bytes", 1583, 629, 1791, 647),
            text("410 GB", 1805, 629, 1875, 647),
            text("Drive C:", 1499, 811, 1845, 851),
            button("Details", 1777, 799, 1885, 827),
            ScreenItem::new(
                Role::CheckBox,
                "Compress this drive to save disk space",
                (1459, 873, 1875, 893),
            ),
            allow,
            button("OK", 1587, 971, 1687, 999),
            button("Cancel", 1695, 971, 1795, 999),
            button("Apply", 1803, 971, 1903, 999),
        ];
        assert_eq!(
            lines(
                Some("Local Disk (C:) Properties"),
                ViewerLayout::Rows,
                &items
            ),
            vec![
                "Local Disk (C:) Properties",
                "Security Previous Versions Quota",
                "General Tools Hardware Sharing",
                "Type: Local Disk",
                "File system: NTFS",
                "Used space: 440,534,675,456 bytes 410 GB",
                "Drive C: Details",
                "Compress this drive to save disk space",
                "Allow files on this drive to have contents indexed in addition to file",
                "properties",
                "OK Cancel Apply",
            ]
        );
    }

    /// A group box spans the rows of its options; only its caption is a line.
    #[test]
    fn a_group_box_does_not_join_the_rows_inside_it() {
        let mut plan = ScreenItem::new(Role::Group, "Plan", (12, 144, 312, 200));
        plan.caption = true;
        let items = vec![
            text("Name:", 12, 36, 102, 56),
            plan,
            ScreenItem::new(Role::RadioButton, "Free", (24, 166, 104, 190)),
            ScreenItem::new(Role::RadioButton, "Pro", (112, 166, 192, 190)),
            ScreenItem::new(Role::RadioButton, "Team", (200, 166, 280, 190)),
        ];
        assert_eq!(rows(&items), vec!["Name:", "Plan", "Free Pro Team"]);
    }

    /// A multi-line field shows its lines one under another, beside its label.
    #[test]
    fn a_multi_line_field_reads_line_by_line() {
        let items = vec![
            text("Notes:", 12, 100, 90, 120),
            ScreenItem::new(
                Role::Edit,
                "First line\r\nSecond line",
                (100, 100, 400, 140),
            ),
            text("Country:", 12, 150, 90, 170),
        ];
        assert_eq!(
            rows(&items),
            vec!["Notes: First line", "Second line", "Country:"]
        );
    }

    /// File Explorer wraps a file's date in direction marks; the viewer shows the date.
    #[test]
    fn direction_marks_are_not_shown() {
        let items = vec![
            text("notes.txt", 10, 10, 200, 30),
            text(
                "\u{200F}\u{200E}15.10.2026 \u{200F}\u{200E}14:02",
                210,
                10,
                400,
                30,
            ),
        ];
        assert_eq!(rows(&items), vec!["notes.txt 15.10.2026 14:02"]);
    }

    /// Calculator on Windows 11 (JAWS 2025 on the dev box): one element per line.
    #[test]
    fn a_modern_app_lists_its_elements() {
        let mut recall = ScreenItem::new(Role::Button, "Memory recall", (0, 0, 10, 10));
        recall.unavailable = true;
        let items = vec![
            ScreenItem::new(Role::Text, "Calculator", (0, 0, 10, 10)),
            ScreenItem::new(Role::Button, "Open Navigation", (0, 0, 10, 10)),
            ScreenItem::new(Role::Group, "", (0, 0, 10, 10)),
            ScreenItem::new(Role::Text, "Display is 0", (0, 0, 10, 10)),
            ScreenItem::new(Role::Group, "Memory controls", (0, 0, 10, 10)),
            recall,
        ];
        assert_eq!(
            lines(Some("Calculator"), ViewerLayout::Elements, &items),
            vec![
                "Calculator",
                "Open Navigation Button",
                "Display is 0",
                "Memory controls group",
                "Memory recall Button  unavailable",
            ]
        );
    }
}
