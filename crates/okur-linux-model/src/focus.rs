//! An AT-SPI element as the engine's focus event: okur-model's mapping of an xa11y element, with
//! what only AT-SPI tells (a toggle button by its role's name), and a native control read with
//! what surrounds it (`native_focus`). How each GTK and Qt control is said is the plan's
//! invariants to settle (docs/plans/2026-11-05-linux-first.md §2.4).

use crate::page::AtspiNode;
use okur_core::aom::Role;
use okur_core::event::FocusEvent;
use xa11y_core::ElementData;

/// The focus event for the element that took the keyboard focus.
pub fn focus_event(element: &ElementData) -> FocusEvent {
    let mut event = okur_model::element::focus_event(element);
    // xa11y gives a toggle button (GTK's GtkToggleButton) the Switch role, a switch's too:
    // AT-SPI's own role name tells them apart. A toggle button is Okur's button whose
    // checked state is its pressed one, "Toggle button Pressed", as Windows' pages give it.
    if element.raw.get("atspi_role").and_then(|r| r.as_str()) == Some("toggle button") {
        event.role = Role::Button;
        event.states.checked = Some(event.states.checked == Some(true));
    }
    event
}

/// The most objects a native focus's subtree, and each surrounding subtree read whole, is read
/// with ([`ancestor_limit`]).
pub const CONTROL_LIMIT: usize = 200;

/// How many of a native focus's ancestors are read, at most, on the way to its window.
pub const WINDOW_DEPTH: usize = 12;

/// Whether an AT-SPI role is a window's: where a native focus's read of its ancestors ends.
pub fn is_window(role: &str) -> bool {
    matches!(role, "frame" | "dialog" | "window" | "alert")
}

/// The read plan for a native focus's ancestors, what [`native_focus`] needs of each: the
/// parent whole (its peers, for their place), and above it a list's, a column view's or a
/// group box's (their items, columns, label); every other ancestor alone. A list's item or a
/// column view's row that says its own place (GTK's `posinset` and `setsize`) needs its
/// parent alone, for its name (the code review of 2026-11-08, 3d). The most objects read for
/// the ancestor `index` above `focus` (0 its parent) of AT-SPI role `role`.
pub fn ancestor_limit(index: usize, role: &str, focus: &AtspiNode) -> usize {
    let own_place =
        own_place(focus).is_some() && matches!(focus.role.as_str(), "list item" | "table row");
    let whole = (index == 0 && !own_place)
        || (index == 1 && matches!(role, "list" | "tree table" | "grouping"));
    if whole {
        CONTROL_LIMIT
    } else {
        1
    }
}

/// Whether a focus read with its ancestors (`native_focus`'s) is in the active window: a focus
/// moving in a window behind (a chat application in the background) is not said, as JAWS says
/// the window in front's. A focus whose window was not reached is taken as active. GTK's frames
/// carry the state (the lab's 301, 303 and 304, 2026-11-08).
pub fn in_active_window(ancestors: &[AtspiNode]) -> bool {
    use crate::page::state;
    ancestors
        .iter()
        .find(|a| is_window(&a.role))
        .is_none_or(|window| window.has(state::ACTIVE))
}

/// A native control's focus (GTK's, a toolkit's outside a page) read with what surrounds it, as
/// Windows reads the same control, so the engine says it alike (the plan's §2.4: SR Desktop
/// Fixture's GTK replica beside JAWS's Windows evidence, probe-gtk-tree.sh). `focus` is the
/// focused element's AT-SPI subtree, `ancestors` its ancestors' subtrees from its parent up,
/// as far as its window. Fills in:
/// - the window, the nearest frame or dialog's name;
/// - a name GTK leaves empty (a list's item) from the first label within, and the mnemonic's
///   underscore GTK leaves in a check box's or a button's name ("_Subscribe to updates");
/// - the place among its peers of the same role, "2 of 3";
/// - a list's item: its list, by the list's name ("Fruit:");
/// - a radio button's group, by the label its group box shows ("Plan");
/// - GTK's drop-down, focused on its toggle button: the combo box, its name and the item shown;
/// - GTK's tree (a list view whose rows have an expander): the expander's row as a tree's item,
///   open or closed, said as a toolkit's own tree (`classic_tree`);
/// - a column view's row as a list view's item, its first cell its name and the others its
///   columns;
/// - a notebook focused itself: its selected page's tab.
pub fn native_focus(
    mut event: FocusEvent,
    focus: &AtspiNode,
    ancestors: &[AtspiNode],
) -> FocusEvent {
    use crate::page::state;
    let parent = ancestors.first();
    let grandparent = ancestors.get(1);
    event.window_title = ancestors
        .iter()
        .find(|a| matches!(a.role.as_str(), "frame" | "dialog" | "window" | "alert"))
        .map(|a| a.name.clone())
        .filter(|n| !n.is_empty());
    // The control as read now, not as its event's snapshot had it: read again as something
    // within it changed, it may have another name or state ("Play" become "Pause"; the code
    // review of 2026-11-08).
    if !focus.name.is_empty() {
        event.name = Some(focus.name.clone());
    }
    if event.states.checked.is_some() {
        let pressed = focus.role == "toggle button" && focus.has(state::PRESSED);
        event.states.checked = Some(focus.has(state::CHECKED) || pressed);
        event.states.mixed = focus.has(state::INDETERMINATE);
    }
    event.states.selected = focus.has(state::SELECTED).then_some(true);
    if focus.has(state::EXPANDABLE) {
        event.states.expanded = Some(focus.has(state::EXPANDED));
    }
    // GTK's mnemonic mark, where the control's own label shows the name without it.
    if let Some(name) = event.name.clone().filter(|n| n.contains('_')) {
        let plain = without_mnemonic(&name);
        if first_label(focus).as_deref() == Some(plain.as_str()) {
            event.name = Some(plain);
        }
    }
    if event.name.is_none() {
        event.name = first_label(focus);
    }
    // A notebook focused itself: its selected page's tab, "General Tab selected".
    if focus.role == "grouping" {
        if let Some(tab) = find(focus, &|n| n.role == "page tab" && n.has(state::SELECTED)) {
            event.role = Role::Tab;
            event.name = Some(tab.name.clone()).filter(|n| !n.is_empty());
            event.states.selected = Some(true);
            event.container_role = Some(Role::TabList);
            event.container_name = None;
            return event;
        }
    }
    // GTK's drop-down: its toggle button has the focus, its label shows the item chosen.
    if focus.role == "toggle button" && parent.is_some_and(|p| p.role == "combo box") {
        event.role = Role::ComboBox;
        event.name = parent.map(|p| p.name.clone()).filter(|n| !n.is_empty());
        event.value = first_label(focus);
        event.states.checked = None;
        return event;
    }
    // GTK's tree: a list's item holding an expander, the expander focused.
    let tree_row = focus.role == "button"
        && parent.is_some_and(|p| p.role == "list item")
        && grandparent.is_some_and(|list| {
            list.role == "list"
                && list.children.iter().any(|item| {
                    item.children
                        .iter()
                        .any(|c| c.role == "button" && c.has(state::EXPANDABLE))
                })
        });
    if tree_row {
        event.role = Role::TreeItem;
        // A toolkit's own tree, said as JAWS says WinForms' [obs, 303/s4].
        event.classic_tree = true;
        event.states.expanded = focus
            .has(state::EXPANDABLE)
            .then(|| focus.has(state::EXPANDED));
        event.states.selected = parent.map(|p| p.has(state::SELECTED));
        if let (Some(item), Some(list)) = (parent, grandparent) {
            place(&mut event, item, list);
        }
        event.container_role = Some(Role::Tree);
        event.container_name = grandparent.map(|l| l.name.clone());
        return event;
    }
    // A column view's row: a list view's item, its cells its columns.
    if focus.role == "table row" && grandparent.is_some_and(|t| t.role == "tree table") {
        let cells: Vec<String> = focus
            .children
            .iter()
            .filter(|c| c.role == "table cell")
            .map(|c| c.name.clone())
            .collect();
        event.role = Role::ListItem;
        event.name = cells.first().cloned().filter(|n| !n.is_empty());
        event.columns = cells
            .into_iter()
            .skip(1)
            .filter(|c| !c.is_empty())
            .collect();
        event.container_is_list_view = true;
        if let Some(rows) = parent {
            place(&mut event, focus, rows);
        }
        event.container_role = Some(Role::List);
        event.container_name = grandparent.map(|t| t.name.clone());
        event.groups = vec![okur_core::GroupEntry::list(
            event.container_name.clone().unwrap_or_default(),
        )];
        return event;
    }
    // A list's item, its list named.
    if focus.role == "list item" {
        if let Some(list) = parent.filter(|p| p.role == "list") {
            place(&mut event, focus, list);
            event.container_role = Some(Role::List);
            event.container_name = Some(list.name.clone());
            event.groups = vec![okur_core::GroupEntry::list(list.name.clone())];
        }
        return event;
    }
    // A radio button: its place in its group, the group by the label its box shows.
    if focus.role == "radio button" {
        if let Some(radios) = parent {
            place(&mut event, focus, radios);
        }
        let group = ancestors
            .iter()
            .take(3)
            .find(|a| a.role == "grouping")
            .and_then(group_name);
        if let Some(group) = group {
            event.container_role = Some(Role::Group);
            event.container_name = Some(group.clone());
            event.groups = vec![okur_core::GroupEntry::group(group)];
        }
    }
    event
}

/// The place a node says itself, from 1, and its set's size: GTK's list items and rows carry
/// `posinset` and `setsize` (the lab's traces at f0b3a66d).
fn own_place(node: &AtspiNode) -> Option<(usize, usize)> {
    let number = |name: &str| node.attributes.get(name)?.parse::<usize>().ok();
    Some((number("posinset")?, number("setsize")?)).filter(|(at, of)| *at >= 1 && at <= of)
}

/// `node`'s place among its parent's children of its role, from 1, and their count: the one it
/// says itself where it says one, its parent then read alone. A peer is
/// `node` by its object's path where the read has one (two items of one name and state are two
/// places: the code review of 2026-11-08, finding 11), else by its name, states and label.
fn place(event: &mut FocusEvent, node: &AtspiNode, parent: &AtspiNode) {
    if let Some((at, of)) = own_place(node) {
        event.item_index = Some(at);
        event.item_count = Some(of);
        return;
    }
    let peers: Vec<&AtspiNode> = parent
        .children
        .iter()
        .filter(|c| c.role == node.role)
        .collect();
    let at = peers.iter().position(|p| {
        if !node.path.is_empty() && !p.path.is_empty() {
            return p.path == node.path;
        }
        p.name == node.name && p.states == node.states && first_label(p) == first_label(node)
    });
    if let Some(at) = at {
        event.item_index = Some(at + 1);
        event.item_count = Some(peers.len());
    }
}

/// A group box's name: its own, else the label it shows first ("Plan").
fn group_name(group: &AtspiNode) -> Option<String> {
    Some(group.name.clone())
        .filter(|n| !n.is_empty())
        .or_else(|| {
            group
                .children
                .iter()
                .find(|c| c.role == "label")
                .map(|label| label.name.clone())
        })
        .filter(|n| !n.is_empty())
}

/// The first label's text within `node`, depth first.
fn first_label(node: &AtspiNode) -> Option<String> {
    find(node, &|n| n.role == "label" && !n.name.is_empty()).map(|label| label.name.clone())
}

/// The first node within `node` (itself left out), depth first, that `wanted` takes.
fn find<'a>(node: &'a AtspiNode, wanted: &dyn Fn(&AtspiNode) -> bool) -> Option<&'a AtspiNode> {
    node.children.iter().find_map(|child| {
        if wanted(child) {
            Some(child)
        } else {
            find(child, wanted)
        }
    })
}

/// A name without GTK's mnemonic mark: "_Subscribe to updates" is "Subscribe to updates", a
/// doubled one an underscore.
fn without_mnemonic(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' {
            if chars.peek() == Some(&'_') {
                out.push('_');
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two items of one name and state are two places: the one focused is found by its path.
    #[test]
    fn a_place_is_found_by_the_objects_path() {
        let item = |path: &str| AtspiNode {
            role: "list item".to_owned(),
            name: "Copy".to_owned(),
            path: path.to_owned(),
            ..AtspiNode::default()
        };
        let list = AtspiNode {
            role: "list".to_owned(),
            children: vec![item("/a"), item("/b"), item("/c")],
            ..AtspiNode::default()
        };
        let mut event = FocusEvent::new(Role::ListItem);
        place(&mut event, &item("/b"), &list);
        assert_eq!((event.item_index, event.item_count), (Some(2), Some(3)));
    }

    /// The parent is read whole, and a list's, a column view's or a group box's above it; the
    /// rest alone; the read ends at a window, a page's dialog's too.
    #[test]
    fn the_read_plan_for_a_native_focus() {
        let button = AtspiNode {
            role: "push button".to_owned(),
            ..AtspiNode::default()
        };
        assert_eq!(ancestor_limit(0, "panel", &button), CONTROL_LIMIT);
        assert_eq!(ancestor_limit(1, "list", &button), CONTROL_LIMIT);
        assert_eq!(ancestor_limit(1, "grouping", &button), CONTROL_LIMIT);
        assert_eq!(ancestor_limit(1, "panel", &button), 1);
        assert_eq!(ancestor_limit(2, "list", &button), 1);
        // A list's item that says its place: its list alone, for its name.
        let mut item = AtspiNode {
            role: "list item".to_owned(),
            ..AtspiNode::default()
        };
        item.attributes
            .insert("posinset".to_owned(), "500".to_owned());
        item.attributes
            .insert("setsize".to_owned(), "1000".to_owned());
        assert_eq!(ancestor_limit(0, "list", &item), 1);
        let list = AtspiNode {
            role: "list".to_owned(),
            name: "Files".to_owned(),
            ..AtspiNode::default()
        };
        let mut event = FocusEvent::new(Role::ListItem);
        place(&mut event, &item, &list);
        assert_eq!(
            (event.item_index, event.item_count),
            (Some(500), Some(1000))
        );
        assert!(is_window("frame") && is_window("dialog") && !is_window("panel"));
    }

    use xa11y_core::Role as AtRole;

    #[test]
    fn a_toggle_button_is_a_button_pressed_or_not() {
        let mut toggle = ElementData::for_role(AtRole::Switch);
        toggle.raw.insert(
            "atspi_role".to_owned(),
            serde_json::Value::String("toggle button".to_owned()),
        );
        let event = focus_event(&toggle);
        assert_eq!(event.role, Role::Button);
        assert_eq!(event.states.checked, Some(false));
        let switch = ElementData::for_role(AtRole::Switch);
        assert_eq!(focus_event(&switch).role, Role::CheckBox);
    }

    /// An AT-SPI node of `role` named `name`, with `children`.
    fn node(role: &str, name: &str, children: Vec<AtspiNode>) -> AtspiNode {
        AtspiNode {
            role: role.to_owned(),
            name: name.to_owned(),
            children,
            ..AtspiNode::default()
        }
    }

    fn with(mut node: AtspiNode, states: &[u32]) -> AtspiNode {
        for s in states {
            node.states |= 1u64 << s;
        }
        node
    }

    fn label(text: &str) -> AtspiNode {
        node("label", text, Vec::new())
    }

    /// SR Desktop Fixture's GTK replica as probe-gtk-tree.sh read it: its window.
    fn window() -> AtspiNode {
        node("frame", "SR Desktop Fixture", Vec::new())
    }

    /// What the engine says as the focus enters `event`, its spaces folded.
    fn said(event: &FocusEvent) -> String {
        fold(&okur_core::native_focus::focus_speech(event, None).render())
    }

    /// JAWS's speech (`speech_raw` of 301-desktop-tab-order's evidence), its spaces folded.
    fn fold(speech: &str) -> String {
        speech.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// The check box keeps GTK's mnemonic mark in its name; its label shows it without
    /// (probe-gtk-tree.sh); the window is named.
    #[test]
    fn a_check_box_loses_the_mnemonic_mark_and_names_its_window() {
        use crate::page::state;
        let check = with(
            node(
                "check box",
                "_Subscribe to updates",
                vec![node("panel", "", vec![]), label("Subscribe to updates")],
            ),
            &[state::CHECKED],
        );
        let mut event = FocusEvent::new(Role::CheckBox);
        event.name = Some("_Subscribe to updates".into());
        event.states.checked = Some(true);
        let event = native_focus(
            event,
            &check,
            &[
                node("panel", "", vec![]),
                node("panel", "", vec![]),
                window(),
            ],
        );
        assert_eq!(event.name.as_deref(), Some("Subscribe to updates"));
        assert_eq!(event.window_title.as_deref(), Some("SR Desktop Fixture"));
        assert_eq!(
            said(&event),
            fold("Subscribe to updates  \u{1d} check box \u{1e}   \u{1d}checked\u{1e} \nTo clear checkmark press Spacebar.")
        );
        // A name with an underscore of its own keeps it.
        let file = node("button", "my_file.txt", vec![label("my_file.txt")]);
        let mut event = FocusEvent::new(Role::Button);
        event.name = Some("my_file.txt".into());
        assert_eq!(
            native_focus(event, &file, &[]).name.as_deref(),
            Some("my_file.txt")
        );
    }

    /// The radio button in its group box: "Plan | Pro radio button checked | 2 of 3", as
    /// Windows gives it [obs, 301/s4].
    #[test]
    fn a_radio_button_has_its_group_and_place() {
        use crate::page::state;
        let radio = |name: &str, on: bool| {
            let r = node(
                "radio button",
                name,
                vec![node("panel", "", vec![]), label(name)],
            );
            if on {
                with(r, &[state::CHECKED])
            } else {
                r
            }
        };
        let pro = radio("Pro", true);
        let radios = node(
            "panel",
            "",
            vec![radio("Free", false), pro.clone(), radio("Team", false)],
        );
        let group = node("grouping", "", vec![label("Plan"), radios.clone()]);
        let mut event = FocusEvent::new(Role::RadioButton);
        event.name = Some("Pro".into());
        event.states.checked = Some(true);
        let event = native_focus(
            event,
            &pro,
            &[radios, group, node("panel", "", vec![]), window()],
        );
        assert_eq!((event.item_index, event.item_count), (Some(2), Some(3)));
        assert_eq!(event.container_role, Some(Role::Group));
        assert_eq!(event.container_name.as_deref(), Some("Plan"));
        assert_eq!(event.groups, vec![okur_core::GroupEntry::group("Plan")]);
        assert_eq!(
            said(&event),
            fold("Plan \nPro  \u{1d} radio button \u{1e}   \u{1d}checked\u{1e} \n2 of 3\nTo change the selection press Up or Down Arrow.")
        );
    }

    /// A focus in a window behind the active one is not said; one whose window is active, or
    /// whose window was not reached, is.
    #[test]
    fn a_focus_in_a_window_behind_is_not_said() {
        use crate::page::state;
        let panel = node("panel", "", vec![]);
        let active = with(window(), &[state::ACTIVE]);
        assert!(in_active_window(&[panel.clone(), active]));
        assert!(!in_active_window(&[panel.clone(), window()]));
        assert!(in_active_window(&[panel]));
    }

    /// A control read again takes its name and states as read now, not as its focus event's
    /// snapshot had them: a button renamed in place, a check box unchecked.
    #[test]
    fn a_control_is_read_as_it_is_now() {
        use crate::page::state;
        let mut event = FocusEvent::new(Role::Button);
        event.name = Some("Play".into());
        let pause = node("push button", "Pause", vec![label("Pause")]);
        assert_eq!(
            native_focus(event, &pause, &[window()]).name.as_deref(),
            Some("Pause")
        );
        let mut event = FocusEvent::new(Role::CheckBox);
        event.name = Some("Remember me".into());
        event.states.checked = Some(true);
        let unchecked = node("check box", "Remember me", vec![label("Remember me")]);
        let read = native_focus(event.clone(), &unchecked, &[window()]);
        assert_eq!(read.states.checked, Some(false));
        let checked = with(unchecked, &[state::CHECKED]);
        assert_eq!(
            native_focus(event, &checked, &[window()]).states.checked,
            Some(true)
        );
    }

    /// GTK's drop-down focused on its toggle button: the combo box, named, with the item shown
    /// [obs, 301/s5: "Country: Combo box | Germany"].
    #[test]
    fn a_drop_down_is_its_combo_box() {
        let shown = node("panel", "", vec![node("panel", "", vec![label("Germany")])]);
        let toggle = node("toggle button", "Country:", vec![shown]);
        let combo = node("combo box", "Country:", vec![toggle.clone()]);
        let mut event = FocusEvent::new(Role::Button);
        event.name = Some("Country:".into());
        let event = native_focus(event, &toggle, &[combo, window()]);
        assert_eq!(event.role, Role::ComboBox);
        assert_eq!(event.name.as_deref(), Some("Country:"));
        assert_eq!(event.value.as_deref(), Some("Germany"));
    }

    /// JAWS gives the drop-down's place, "2 of 3" [obs, 301/s5]; GTK's closed drop-down shows
    /// only the item chosen, its list built when it opens.
    #[test]
    #[ignore = "known gap: a closed GTK drop-down's place among its items"]
    fn a_drop_down_says_its_place() {
        let shown = node("panel", "", vec![node("panel", "", vec![label("Germany")])]);
        let toggle = node("toggle button", "Country:", vec![shown]);
        let combo = node("combo box", "Country:", vec![toggle.clone()]);
        let mut event = FocusEvent::new(Role::Button);
        event.name = Some("Country:".into());
        let event = native_focus(event, &toggle, &[combo, window()]);
        assert_eq!(
            said(&event),
            fold("Country:  \u{1d} Combo box \u{1e} \nGermany\n2 of 3\nTo change the selection use the Arrow keys.")
        );
    }

    /// A list's item, named by its label, in its named list [obs, 301/s6: "Fruit: List box |
    /// Bananas | 2 of 3"].
    #[test]
    fn a_list_item_is_named_by_its_label_in_its_list() {
        use crate::page::state;
        let item = |text: &str| node("list item", "", vec![label(text)]);
        let bananas = with(item("Bananas"), &[state::SELECTED]);
        let list = node(
            "list",
            "Fruit:",
            vec![item("Apples"), bananas.clone(), item("Cherries")],
        );
        let event = native_focus(FocusEvent::new(Role::ListItem), &bananas, &[list, window()]);
        assert_eq!(event.name.as_deref(), Some("Bananas"));
        assert_eq!((event.item_index, event.item_count), (Some(2), Some(3)));
        assert_eq!(event.container_role, Some(Role::List));
        assert_eq!(event.container_name.as_deref(), Some("Fruit:"));
        assert_eq!(
            said(&event),
            fold("Fruit:  \u{1d} List box \u{1e} \nBananas \n2 of 3\nTo move to an item press the Arrow keys.")
        );
    }

    /// GTK's tree, a list view whose rows have expanders: the row is a tree's item, closed
    /// [obs, 301/s7: "Tree view | Documents closed | 1 of 2"].
    #[test]
    fn an_expanders_row_is_a_trees_item() {
        use crate::page::state;
        let documents = with(
            node("button", "Documents", vec![label("Documents")]),
            &[state::EXPANDABLE],
        );
        let first = with(
            node("list item", "", vec![documents.clone()]),
            &[state::SELECTED],
        );
        let second = node(
            "list item",
            "",
            vec![node("button", "Pictures", vec![label("Pictures")])],
        );
        let list = node("list", "", vec![first.clone(), second]);
        let mut event = FocusEvent::new(Role::Button);
        event.name = Some("Documents".into());
        let event = native_focus(event, &documents, &[first, list, window()]);
        assert_eq!(event.role, Role::TreeItem);
        assert_eq!(event.states.expanded, Some(false));
        assert_eq!((event.item_index, event.item_count), (Some(1), Some(2)));
        assert_eq!(event.container_role, Some(Role::Tree));
        assert_eq!(
            said(&event),
            fold(" \u{1d} Tree view \u{1e} \nDocuments  \u{1d}closed\u{1e} \n1 of 2\nTo move through or expand items use the Arrow keys.")
        );
    }

    /// A column view's row: a list view's item, its other cells its columns [obs, 301/s8:
    /// "List view | budget.xlsx | 24 KB | 1 of 3"].
    #[test]
    fn a_column_views_row_is_a_list_views_item() {
        let row = |a: &str, b: &str| {
            node(
                "table row",
                &format!("{a} {b}"),
                vec![
                    node("table cell", a, vec![label(a)]),
                    node("table cell", b, vec![label(b)]),
                ],
            )
        };
        let first = row("budget.xlsx", "24 KB");
        let rows = node(
            "list",
            "",
            vec![
                first.clone(),
                row("notes.txt", "2 KB"),
                row("photo.jpg", "1.2 MB"),
            ],
        );
        let table = node(
            "tree table",
            "",
            vec![node("table row", "Name Size", vec![]), rows.clone()],
        );
        let mut event = FocusEvent::new(Role::TableRow);
        event.name = Some("budget.xlsx 24 KB".into());
        let event = native_focus(event, &first, &[rows, table, window()]);
        assert_eq!(event.role, Role::ListItem);
        assert_eq!(event.name.as_deref(), Some("budget.xlsx"));
        assert_eq!(event.columns, vec!["24 KB".to_owned()]);
        assert!(event.container_is_list_view);
        assert_eq!((event.item_index, event.item_count), (Some(1), Some(3)));
        assert_eq!(
            said(&event),
            fold(" \u{1d} List view \u{1e} \nbudget.xlsx\n24 KB \n1 of 3\nTo move to items use the Arrow keys.")
        );
    }

    /// A notebook focused itself: its selected tab [obs, 301/s9: "General Tab selected"].
    #[test]
    fn a_notebook_is_its_selected_tab() {
        use crate::page::state;
        let tabs = node(
            "page tab list",
            "",
            vec![
                with(
                    node("page tab", "General", vec![label("General")]),
                    &[state::SELECTED],
                ),
                node("page tab", "Advanced", vec![label("Advanced")]),
            ],
        );
        let notebook = node("grouping", "", vec![node("panel", "", vec![tabs])]);
        let event = native_focus(FocusEvent::new(Role::Group), &notebook, &[window()]);
        assert_eq!(event.role, Role::Tab);
        assert_eq!(event.name.as_deref(), Some("General"));
        assert_eq!(event.states.selected, Some(true));
        assert_eq!(
            said(&event),
            fold("General  \u{1d} Tab \u{1e}   \u{1d}selected\u{1e} \nTo switch pages, press Control+PageDown.")
        );
    }
}
