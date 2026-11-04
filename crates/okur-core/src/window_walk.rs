//! Say Window (Insert+B) in a window that is not a dialog box. JAWS walks the whole window and
//! says each control in the window's order, a speech history entry for each part: its name,
//! its kind's word, its states, its value. "SR Desktop Fixture | window | menu bar | File |
//! menu item | ... | Name: | Name: | edit | Ada | ... | Documents | tree item | collapsed
//! selected | ... | list | grid with 2 columns and 3 rows | ... | status bar | Ready" [obs,
//! 548/s14, 549/s14, 554/s14, the same at every verbosity level]. A dialog box is read
//! another way ([`crate::native_focus::say_window`]).
//!
//! The platform walks the window ([`crate::actions::ApplicationActions::read_window_controls`]) and
//! gives its controls as [`WindowControl`]s, which leave out what JAWS does not read (a
//! control off the screen or unavailable, a list item's own parts).

use serde::{Deserialize, Serialize};

use crate::announcement::{Announcement, Line, PartKind};
use crate::messages::msg;

/// What kind of control a window's control is. Say Window names each by its own word: the
/// words UI Automation gives its control types, which are what JAWS says ("tab item",
/// "header item" [obs, 548/s14]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlKind {
    AppBar,
    Button,
    Calendar,
    CheckBox,
    ComboBox,
    Custom,
    DataGrid,
    DataItem,
    Document,
    Edit,
    Group,
    Header,
    HeaderItem,
    Image,
    Link,
    List,
    /// An item of a list: its name and states, without a word for its kind [obs, 548/s14:
    /// "Apples | Bananas | Cherries | selected"].
    ListItem,
    Menu,
    MenuBar,
    MenuItem,
    Pane,
    ProgressBar,
    RadioButton,
    ScrollBar,
    SemanticZoom,
    Separator,
    Slider,
    Spinner,
    SplitButton,
    StatusBar,
    /// A tab control, its tabs' container: "tab".
    Tab,
    /// One of its tabs: "tab item".
    TabItem,
    Table,
    /// Text: its words alone [obs, 548/s14: "Name: | Name: | edit"].
    Text,
    Thumb,
    TitleBar,
    ToolBar,
    ToolTip,
    Tree,
    TreeItem,
    Window,
}

impl ControlKind {
    /// The word Say Window says after the control's name; none for text and list items.
    fn word(self) -> Option<String> {
        use ControlKind::*;
        let word = match self {
            Text | ListItem => return None,
            AppBar => msg::control_app_bar(),
            Button => msg::control_button(),
            Calendar => msg::control_calendar(),
            CheckBox => msg::control_check_box(),
            ComboBox => msg::control_combo_box(),
            Custom => msg::control_custom(),
            DataGrid => msg::control_data_grid(),
            DataItem => msg::control_data_item(),
            Document => msg::control_document(),
            Edit => msg::control_edit(),
            Group => msg::control_group(),
            Header => msg::control_header(),
            HeaderItem => msg::control_header_item(),
            Image => msg::control_image(),
            Link => msg::control_link(),
            List => msg::control_list(),
            Menu => msg::control_menu(),
            MenuBar => msg::control_menu_bar(),
            MenuItem => msg::control_menu_item(),
            Pane => msg::control_pane(),
            ProgressBar => msg::control_progress_bar(),
            RadioButton => msg::control_radio_button(),
            ScrollBar => msg::control_scroll_bar(),
            SemanticZoom => msg::control_semantic_zoom(),
            Separator => msg::control_separator(),
            Slider => msg::control_slider(),
            Spinner => msg::control_spinner(),
            SplitButton => msg::control_split_button(),
            StatusBar => msg::control_status_bar(),
            Tab => msg::control_tab(),
            TabItem => msg::control_tab_item(),
            Table => msg::control_table(),
            Thumb => msg::control_thumb(),
            TitleBar => msg::control_title_bar(),
            ToolBar => msg::control_tool_bar(),
            ToolTip => msg::control_tool_tip(),
            Tree => msg::control_tree(),
            TreeItem => msg::control_tree_item(),
            Window => msg::control_window(),
        };
        Some(word.as_str().to_owned())
    }
}

/// One control of a window as Say Window reads it, in the window's order; the window itself
/// is the first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowControl {
    pub kind: ControlKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// A check box's state: "checked", "not checked".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// Expanded or collapsed, where it can be either: "collapsed" [obs, 548/s14].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
    /// A tree item with no items of its own: "leaf node" [obs, 548/s14].
    #[serde(default)]
    pub leaf: bool,
    /// Selected, as a list's or tree's item, a radio button or a tab is: "selected".
    #[serde(default)]
    pub selected: bool,
    /// A list with column headers, as its columns and rows: "list | grid with 2 columns and 3
    /// rows" [obs, 548/s14].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<(usize, usize)>,
}

impl WindowControl {
    /// A control of `kind` named `name`, in no state.
    pub fn new(kind: ControlKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            value: None,
            checked: None,
            expanded: None,
            leaf: false,
            selected: false,
            grid: None,
        }
    }

    /// Its states as one entry: "collapsed  selected" is one [obs, 548/s14].
    fn states(&self) -> Option<String> {
        let mut states = Vec::new();
        if let Some(checked) = self.checked {
            states.push(if checked {
                msg::state_checked()
            } else {
                msg::state_not_checked()
            });
        }
        match self.expanded {
            Some(true) => states.push(msg::state_expanded()),
            Some(false) => states.push(msg::state_collapsed()),
            None if self.leaf => states.push(msg::state_leaf_node()),
            None => {}
        }
        if self.selected {
            states.push(msg::state_selected());
        }
        (!states.is_empty()).then(|| {
            states
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
    }
}

/// What Say Window says of a window that is not a dialog box: each of its `controls` as its
/// name, its kind's word, a list's columns and rows, its states and its value, each an entry
/// of its own, in their order.
pub fn say(controls: &[WindowControl]) -> Announcement {
    let mut lines = Vec::new();
    for control in controls {
        let name = control.name.trim();
        if !name.is_empty() {
            lines.push(Line::of(PartKind::Name, name));
        }
        // Plain words, without the marks of a role or a state: the window is read as text.
        lines.extend(control.kind.word().map(|w| Line::of(PartKind::Message, w)));
        if let Some((columns, rows)) = control.grid {
            lines.push(Line::of(
                PartKind::Message,
                msg::grid_entry(columns, rows).render(),
            ));
        }
        lines.extend(control.states().map(|s| Line::of(PartKind::Message, s)));
        if let Some(value) = control.value.as_deref().map(str::trim) {
            if !value.is_empty() {
                lines.push(Line::of(PartKind::Value, value));
            }
        }
    }
    Announcement::new(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn control(kind: ControlKind, name: &str) -> WindowControl {
        WindowControl::new(kind, name)
    }

    /// The forms fixture's window as JAWS reads it at every verbosity level [obs, 548/s14,
    /// 549/s14, 554/s14], from the controls its walk gives.
    #[test]
    fn a_window_is_read_control_by_control_as_jaws_reads_it() {
        use ControlKind::*;
        let with = |mut c: WindowControl, f: &dyn Fn(&mut WindowControl)| {
            f(&mut c);
            c
        };
        let controls = vec![
            control(Window, "SR Desktop Fixture"),
            control(MenuBar, ""),
            control(MenuItem, "File"),
            control(Text, "Name:"),
            with(control(Edit, "Name:"), &|c| c.value = Some("Ada".into())),
            control(Text, "Notes:"),
            control(Edit, "Notes:"),
            with(control(CheckBox, "Subscribe to updates"), &|c| {
                c.checked = Some(true)
            }),
            with(control(CheckBox, "Remember me"), &|c| {
                c.checked = Some(false)
            }),
            control(Group, "Plan"),
            control(RadioButton, "Free"),
            with(control(RadioButton, "Team"), &|c| c.selected = true),
            control(Text, "Country:"),
            with(control(ComboBox, "Country:"), &|c| {
                c.value = Some("Germany".into())
            }),
            control(Button, "Open"),
            control(List, "Fruit:"),
            control(ListItem, "Apples"),
            with(control(ListItem, "Cherries"), &|c| c.selected = true),
            control(Tree, ""),
            with(control(TreeItem, "Documents"), &|c| {
                c.expanded = Some(false);
                c.selected = true;
            }),
            with(control(TreeItem, "Pictures"), &|c| c.leaf = true),
            with(control(List, ""), &|c| c.grid = Some((2, 3))),
            control(Header, "Header Control"),
            control(HeaderItem, "Name"),
            control(Tab, ""),
            control(Pane, "General"),
            with(control(TabItem, "General"), &|c| c.selected = true),
            control(StatusBar, ""),
            control(Text, "Ready"),
            with(control(ProgressBar, "Copying files"), &|c| {
                c.value = Some("40%".into())
            }),
        ];
        assert_eq!(
            say(&controls).render().replace('\n', " | "),
            "SR Desktop Fixture | window | menu bar | File | menu item | Name: | Name: | edit | \
             Ada | Notes: | Notes: | edit | Subscribe to updates | check box | checked | \
             Remember me | check box | not checked | Plan | group | Free | radio button | Team \
             | radio button | selected | Country: | Country: | combo box | Germany | Open | \
             button | Fruit: | list | Apples | Cherries | selected | tree | Documents | tree \
             item | collapsed selected | Pictures | tree item | leaf node | list | grid with 2 \
             columns and 3 rows | Header Control | header | Name | header item | tab | General \
             | pane | General | tab item | selected | status bar | Ready | Copying files | \
             progress bar | 40%"
        );
    }
}
