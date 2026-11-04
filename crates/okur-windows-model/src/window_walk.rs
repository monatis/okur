//! A window's UI Automation tree as Say Window reads a window that is not a dialog box
//! ([`okur_core::window_walk`]): the window and its controls in the tree's order, without
//! what JAWS leaves out of it [obs, 548/s14]: a control off the screen (a closed menu's
//! items, a combo box's own text) or unavailable (a disabled button), a list item's parts (a
//! list view's cells), and the title bar and scroll bars, which Say Window does not read in a
//! dialog box either.

use okur_core::window_walk::{ControlKind, WindowControl};

use crate::uia_snapshot::UiaNode;

/// The controls of the window `tree` is, the window first.
pub fn window_controls(tree: &UiaNode) -> Vec<WindowControl> {
    let mut controls = Vec::new();
    walk(tree, &mut controls);
    controls
}

fn walk(node: &UiaNode, controls: &mut Vec<WindowControl>) {
    if node.is_offscreen || !node.is_enabled {
        return;
    }
    let Some(kind) = kind(node.control_type) else {
        return;
    };
    if matches!(kind, ControlKind::TitleBar | ControlKind::ScrollBar) {
        return;
    }
    controls.push(control(node, kind));
    // A list item is its name: a list view's row is not read again cell by cell.
    if kind == ControlKind::ListItem {
        return;
    }
    for child in &node.children {
        walk(child, controls);
    }
}

/// The control `node` is, as Say Window says it.
fn control(node: &UiaNode, kind: ControlKind) -> WindowControl {
    let mut control = WindowControl::new(
        kind,
        okur_core::viewer_text::without_marks(node.name.as_deref().unwrap_or_default()),
    );
    control.value = value(node, kind);
    if kind == ControlKind::CheckBox {
        control.checked = match node.toggle_state {
            Some(0) => Some(false),
            Some(1) => Some(true),
            _ => None,
        };
    }
    // `ExpandCollapseState`: collapsed, expanded, partly expanded, a leaf.
    match node.expand_collapse_state {
        Some(0) => control.expanded = Some(false),
        Some(1) => control.expanded = Some(true),
        Some(3) if kind == ControlKind::TreeItem => control.leaf = true,
        _ => {}
    }
    control.selected = node.is_selected == Some(true);
    // A list with column headers is a grid of its headers' columns and its items' rows: "list
    // | grid with 2 columns and 3 rows" (a WinForms list view in details view, [obs, 548/s14]).
    if kind == ControlKind::List {
        let header = node
            .children
            .iter()
            .find(|c| kind_of(c) == Some(ControlKind::Header));
        if let Some(header) = header {
            let columns = header
                .children
                .iter()
                .filter(|c| kind_of(c) == Some(ControlKind::HeaderItem))
                .count();
            let rows = node
                .children
                .iter()
                .filter(|c| kind_of(c) == Some(ControlKind::ListItem))
                .count();
            control.grid = Some((
                node.grid_column_count
                    .and_then(|n| usize::try_from(n).ok())
                    .unwrap_or(columns),
                node.grid_row_count
                    .and_then(|n| usize::try_from(n).ok())
                    .unwrap_or(rows),
            ));
        }
    }
    control
}

/// Its value: what its Value pattern says, or a progress bar's place in its range as a
/// percentage, "40%" [obs, 1136/s3].
fn value(node: &UiaNode, kind: ControlKind) -> Option<String> {
    if let Some(value) = node.value.as_deref() {
        let value = okur_core::viewer_text::without_marks(value);
        return (!value.trim().is_empty()).then_some(value);
    }
    if kind != ControlKind::ProgressBar {
        return None;
    }
    let now = node.range_value?;
    let (min, max) = (
        node.range_minimum.unwrap_or(0.0),
        node.range_maximum.unwrap_or(100.0),
    );
    let percent = if max > min {
        (now - min) / (max - min) * 100.0
    } else {
        now
    };
    Some(format!("{}%", percent.round()))
}

fn kind_of(node: &UiaNode) -> Option<ControlKind> {
    kind(node.control_type)
}

/// UI Automation's control types (UIAutomationClient.h), 50000 to 50040, by their names.
fn kind(control_type: i32) -> Option<ControlKind> {
    use ControlKind::*;
    Some(match control_type {
        50000 => Button,
        50001 => Calendar,
        50002 => CheckBox,
        50003 => ComboBox,
        50004 => Edit,
        50005 => Link,
        50006 => Image,
        50007 => ListItem,
        50008 => List,
        50009 => Menu,
        50010 => MenuBar,
        50011 => MenuItem,
        50012 => ProgressBar,
        50013 => RadioButton,
        50014 => ScrollBar,
        50015 => Slider,
        50016 => Spinner,
        50017 => StatusBar,
        50018 => Tab,
        50019 => TabItem,
        50020 => Text,
        50021 => ToolBar,
        50022 => ToolTip,
        50023 => Tree,
        50024 => TreeItem,
        50025 => Custom,
        50026 => Group,
        50027 => Thumb,
        50028 => DataGrid,
        50029 => DataItem,
        50030 => Document,
        50031 => SplitButton,
        50032 => Window,
        50033 => Pane,
        50034 => Header,
        50035 => HeaderItem,
        50036 => Table,
        50037 => TitleBar,
        50038 => Separator,
        50039 => SemanticZoom,
        50040 => AppBar,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(control_type: i32, name: &str, children: Vec<UiaNode>) -> UiaNode {
        UiaNode {
            control_type,
            name: (!name.is_empty()).then(|| name.to_owned()),
            is_enabled: true,
            children,
            ..UiaNode::default()
        }
    }

    /// What JAWS leaves out of a window's walk [obs, 548/s14]: a closed menu's items and a
    /// combo box's own text (off the screen), the disabled Apply button, a list view's
    /// cells; a list view's columns and rows are counted from its header and its items.
    #[test]
    fn a_window_walk_leaves_out_what_jaws_does_not_read() {
        let offscreen = |mut n: UiaNode| {
            n.is_offscreen = true;
            n
        };
        let mut apply = node(50000, "Apply", vec![]);
        apply.is_enabled = false;
        let mut selected = node(
            50007,
            "budget.xlsx",
            vec![
                node(50020, "budget.xlsx", vec![]),
                node(50020, "24 KB", vec![]),
            ],
        );
        selected.is_selected = Some(true);
        let mut leaf = node(50024, "Pictures", vec![]);
        leaf.expand_collapse_state = Some(3);
        let mut progress = node(50012, "Copying files", vec![]);
        progress.range_value = Some(40.0);
        progress.range_minimum = Some(0.0);
        progress.range_maximum = Some(100.0);
        let tree = node(
            50032,
            "SR Desktop Fixture",
            vec![
                node(
                    50010,
                    "",
                    vec![node(
                        50011,
                        "File",
                        vec![offscreen(node(50011, "New", vec![]))],
                    )],
                ),
                node(
                    50003,
                    "Country:",
                    vec![
                        offscreen(node(50020, "Country:", vec![])),
                        node(50000, "Open", vec![]),
                    ],
                ),
                node(50023, "", vec![leaf]),
                node(
                    50008,
                    "",
                    vec![
                        node(
                            50034,
                            "Header Control",
                            vec![node(50035, "Name", vec![]), node(50035, "Size", vec![])],
                        ),
                        selected,
                        node(50007, "notes.txt", vec![node(50020, "2 KB", vec![])]),
                    ],
                ),
                apply,
                progress,
            ],
        );
        let said = okur_core::window_walk::say(&window_controls(&tree))
            .render()
            .replace('\n', " | ");
        assert_eq!(
            said,
            "SR Desktop Fixture | window | menu bar | File | menu item | Country: | combo box | \
             Open | button | tree | Pictures | tree item | leaf node | list | grid with 2 \
             columns and 2 rows | Header Control | header | Name | header item | Size | header \
             item | budget.xlsx | selected | notes.txt | Copying files | progress bar | 40%"
        );
    }
}
