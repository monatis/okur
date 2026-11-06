//! How a focus is read: a recorded event's focus chain, MSAA object and context into the
//! engine's `FocusEvent`, for every kind of control (`focus_event_from_trace`).

use super::*;

/// A dialog's tree without the items inside its tree views' items: a file dialog's navigation
/// pane lists the account's pinned and recent folders under Quick access (525, round 35), which
/// Say Window does not read (JAWS names the tree's item and its count) and a public corpus must
/// not hold. The tree's own items stay ("Quick access", "This PC").
pub fn without_nested_tree_items(mut tree: UiaNode) -> UiaNode {
    fn prune(node: &mut UiaNode) {
        if node.control_type == UIA_TREEITEM {
            node.children.clear();
            return;
        }
        for child in &mut node.children {
            prune(child);
        }
    }
    prune(&mut tree);
    tree
}

/// A dialog's controls in reading order (depth first), below the dialog itself: what Say
/// Window reads. Nameless containers (panes, groups) are passed through.
pub(super) fn window_items(tree: &UiaNode) -> Vec<okur_core::WindowItem> {
    const UIA_COMBOBOX: i32 = 50003;
    const UIA_EDIT: i32 = 50004;
    const UIA_LINK: i32 = 50005;
    const UIA_SCROLLBAR: i32 = 50014;
    const UIA_PANE: i32 = 50033;
    /// A control as Say Window reads it, without what is inside it.
    fn item(node: &UiaNode) -> Option<okur_core::WindowItem> {
        let mut leaf = node.clone();
        leaf.children.clear();
        let aom = crate::uia_snapshot::to_aom(&leaf);
        let name = node.name.clone().unwrap_or_default();
        (!name.trim().is_empty() || aom.value.is_some()).then(|| okur_core::WindowItem {
            role: aom.role.clone(),
            name: okur_core::viewer_text::without_marks(&name),
            value: aom
                .value
                .as_deref()
                .map(okur_core::viewer_text::without_marks)
                .filter(|v| !v.is_empty()),
            expanded: aom.states.expanded,
            checked: aom.states.checked,
            read_only: node.value_is_read_only == Some(true),
            group: node.group_box.clone(),
            place: None,
            in_combo_box: false,
        })
    }
    /// A Win32 combo box as JAWS's Say Window reads it [obs, 665/s3, 544/s13]: the box, its
    /// value with its list's shown items and its place, then its list (its selected item)
    /// and its field, the value and place again; a drop-down list is the box alone. Its
    /// scroll bar and drop-down button are not read.
    fn combo_box(node: &UiaNode, items: &mut Vec<okur_core::WindowItem>) {
        let place = node
            .position_in_set
            .zip(node.size_of_set)
            .and_then(|(i, n)| Some((usize::try_from(i).ok()?, usize::try_from(n).ok()?)));
        let list = node.children.iter().find(|c| c.control_type == UIA_LIST);
        let field = node.children.iter().find(|c| c.control_type == UIA_EDIT);
        let shown: Vec<&str> = list
            .map(|l| {
                l.children
                    .iter()
                    .filter(|c| c.control_type == UIA_LISTITEM)
                    .filter_map(|c| c.name.as_deref())
                    .collect()
            })
            .unwrap_or_default();
        let button = node.children.iter().find(|c| c.control_type == UIA_BUTTON);
        // A drop-down combo box with a field of its own, its list closed: the box, collapsed,
        // with its value; its field, an edit; its button; and no place. "Open: combo box
        // collapsed | C: | Open: edit | C: | Open button" in the Run dialog [obs, 3159/s9].
        if field.is_some() && shown.is_empty() && button.is_some() {
            items.extend(item(node));
            items.extend(field.and_then(item));
            items.extend(button.and_then(item));
            return;
        }
        let Some(mut combo) = item(node) else {
            return;
        };
        combo.place = place;
        combo.value = match (combo.value.take(), shown.is_empty()) {
            (Some(value), false) => Some(format!("{value} {}", shown.join(" "))),
            (value, _) => value,
        };
        items.push(combo);
        if let Some(list) = list {
            let selected = list
                .children
                .iter()
                .find(|c| c.is_selected == Some(true))
                .and_then(|c| c.name.clone());
            items.push(okur_core::WindowItem {
                role: okur_core::Role::List,
                name: String::new(),
                value: selected,
                expanded: None,
                checked: None,
                read_only: false,
                group: None,
                place: None,
                in_combo_box: true,
            });
        }
        if let Some(mut edit) = field.and_then(item) {
            edit.place = place;
            edit.in_combo_box = true;
            items.push(edit);
        }
    }
    fn walk(node: &UiaNode, font_dialog: bool, items: &mut Vec<okur_core::WindowItem>) {
        for child in &node.children {
            match child.control_type {
                UIA_SCROLLBAR => continue,
                UIA_TEXT if child.owner_drawn => continue,
                // The common Font dialog's sample, its text in the chosen font (stc5 and stc6 of
                // the Windows SDK's dlgs.h, in the Sample box): JAWS reads the box's name alone
                // [obs, 665/s3].
                UIA_TEXT
                    if font_dialog
                        && matches!(child.automation_id.as_deref(), Some("1092" | "1093")) =>
                {
                    continue
                }
                UIA_COMBOBOX if child.position_in_set.is_some() => {
                    combo_box(child, items);
                    continue;
                }
                // A link in a pane of its own name (a SysLink): the link alone [obs, 665/s3].
                UIA_PANE
                    if child.children.len() == 1
                        && child
                            .children
                            .iter()
                            .all(|c| c.control_type == UIA_LINK && c.name == child.name) =>
                {
                    walk(child, font_dialog, items);
                    continue;
                }
                _ => {}
            }
            items.extend(item(child));
            walk(child, font_dialog, items);
        }
    }
    // The common Font dialog, known by its font name combo box (cmb1 of dlgs.h).
    let font_dialog = tree
        .children
        .iter()
        .any(|c| c.control_type == UIA_COMBOBOX && c.automation_id.as_deref() == Some("1136"));
    let mut items = Vec::new();
    walk(tree, font_dialog, &mut items);
    items
}

/// Whether a window is an edit of several lines: an edit window class (Win32's Edit, WinForms'
/// `WindowsForms10.EDIT…`, RichEdit) with ES_MULTILINE in its style. The bit means something
/// else for other classes.
/// Whether a window is a classic RichEdit (`RichEdit`, `RichEdit20W`, `RICHEDIT50W`), not
/// Windows 11 Notepad's `RichEditD2DPT`, which names itself.
pub(super) fn rich_edit(class: Option<&str>) -> bool {
    class.is_some_and(|c| {
        let c = c.to_ascii_uppercase();
        c.strip_prefix("RICHEDIT").is_some_and(|version| {
            version
                .trim_end_matches(['A', 'W'])
                .chars()
                .all(|d| d.is_ascii_digit())
        })
    })
}

/// Whether a window is a plain Win32 edit (Win32's Edit, WinForms' `WindowsForms10.EDIT…`), not
/// RichEdit.
pub(super) fn plain_edit(class: Option<&str>) -> bool {
    class.is_some_and(|c| {
        let c = c.to_ascii_uppercase();
        c == "EDIT" || c.contains(".EDIT.")
    })
}

pub(super) fn multi_line_edit(class: Option<&str>, style: Option<u32>) -> bool {
    const ES_MULTILINE: u32 = 0x0004;
    let edit = class.is_some_and(|c| {
        let c = c.to_ascii_uppercase();
        c == "EDIT" || c.contains(".EDIT.") || c.starts_with("RICHEDIT")
    });
    edit && style.is_some_and(|s| s & ES_MULTILINE != 0)
}

/// Win32 list-view controls (`SysListView32`, also as WinForms `WindowsForms10.SysListView32...`):
/// JAWS calls their lists "List view".
/// A list view: Win32's, or File Explorer's items view, whose unselected item JAWS reads as
/// "Not Selected SR OCR Sign.png" [obs, 1026/s5].
pub(super) fn is_list_view(class: Option<&str>) -> bool {
    class.is_some_and(|c| c.contains("SysListView32") || c == "UIItemsView")
}

pub fn focus_event_from_chain(
    chain: &[UiaNode],
    window_class: Option<&str>,
    window_title: Option<&str>,
) -> Option<okur_core::FocusEvent> {
    use okur_core::Role;
    let focused = chain.last()?;
    let mut leaf = focused.clone();
    leaf.children.clear();
    let aom = crate::uia_snapshot::to_aom(&leaf);
    // A native button that is also a heading (Windows Settings' breadcrumb, a Button with
    // HeadingLevel1) takes the focus as a button: JAWS says "Home Button | 1 of 1" (dev box,
    // 2026-10-16).
    let native = !chain.iter().any(|a| a.control_type == UIA_DOCUMENT);
    let role = match aom.role {
        Role::Heading { .. } if native && focused.control_type == UIA_BUTTON => Role::Button,
        // A WinUI menu's item (File Explorer's context menu): UIA reports it, where a Win32
        // menu's comes through MSAA (`focus_event_from_trace`). Pages' menu items stay out
        // of the snapshot's roles, which the UIA parity reads.
        _ if native && focused.control_type == UIA_MENUITEM => Role::MenuItem,
        ref role => role.clone(),
    };

    let mut event = okur_core::FocusEvent::new(role);
    if event.role == Role::MenuItem {
        const UIA_MENUBAR: i32 = 50010;
        event.in_menu_bar = chain
            .iter()
            .rev()
            .nth(1)
            .is_some_and(|parent| parent.control_type == UIA_MENUBAR);
    }
    event.name = aom.name.clone();
    event.value = aom.value.clone();
    // An application grid's cell has its text as its value (a WinForms DataGridView's "Item
    // Row 0" is "Pens" [obs, 545/s1]).
    if native && event.role == Role::TableCell {
        event.value = focused.value.clone().filter(|v| !v.trim().is_empty());
    }
    // A slider's value is the one it reports ("Brightness left right slider | 100"), else its
    // range value, a percentage when it runs from 0 to 100 ("Adjust the output volume left
    // right slider | 34%"): Windows Settings, JAWS on the dev box (2026-10-16).
    if aom.role == Role::Slider {
        if let Some(value) = focused.value.clone().filter(|v| !v.trim().is_empty()) {
            event.value = Some(value);
        } else if let Some(now) = aom.value_now {
            let number = if now.fract() == 0.0 {
                format!("{now:.0}")
            } else {
                format!("{now}")
            };
            event.value = Some(match (aom.value_min, aom.value_max) {
                (Some(min), Some(max)) if min == 0.0 && max == 100.0 => format!("{number}%"),
                _ => number,
            });
        }
    }
    event.states = aom.states.clone();
    event.item_index = focused
        .position_in_set
        .and_then(|p| usize::try_from(p).ok());
    event.item_count = focused.size_of_set.and_then(|s| usize::try_from(s).ok());
    if focused.control_type == UIA_TREEITEM {
        // Nesting level: tree items above this one in the control view.
        event.tree_level = Some(
            chain
                .iter()
                .rev()
                .skip(1)
                .filter(|a| a.control_type == UIA_TREEITEM)
                .count(),
        );
        // The item it is in, which JAWS says after a level change: "level 1 | Primary
        // Navigation | 1 of 1 | Banner" [obs, 241/s2]; not in a WinForms tree, "level 1 |
        // Reports | 1 of 2" [obs, 303/s8].
        if !window_class.is_some_and(|c| c.starts_with("WindowsForms")) {
            event.item_parent = chain
                .iter()
                .rev()
                .skip(1)
                .find(|a| a.control_type == UIA_TREEITEM)
                .and_then(|a| a.name.clone())
                .filter(|n| !n.trim().is_empty());
        }
    }
    event.help_text = focused
        .full_description
        .clone()
        .or_else(|| focused.help_text.clone());
    // File Explorer's search box (its Control Panel pages' too) says its prompt, which UI
    // Automation gives as its help: "Search Box Edit | Search Programs and Features | Type in
    // text." [obs, 3156/s5]. Another field's help is not said: Chrome's address bar [obs,
    // 204/s13], Excel's cell editor.
    if focused.class_name.as_deref() == Some("SearchEditBox") {
        event.description.clone_from(&event.help_text);
    }
    event.item_type = focused.item_type.clone().filter(|t| !t.trim().is_empty());
    event.platform.window_class = window_class.map(str::to_owned);
    event.window_title = window_title.map(str::to_owned);
    // Inside a browser document, Chrome reports the DOM id as AutomationId.
    // A page: a document in a browser's web content window. Documents elsewhere (Windows 11
    // Notepad, Word) are native text controls, read by caret, not by the virtual cursor. A
    // browser coming to the front reports its focus first from its top-level window
    // (Chromium's `Chrome_WidgetWin_1`, Electron apps alike), then from the page's window;
    // both are the page (seen live on the dev box).
    let in_browser = window_class
        .is_some_and(|c| c == CHROMIUM_TOP_WINDOW || WEB_CONTENT_WINDOW_CLASSES.contains(&c));
    event.in_document = in_browser && chain.iter().any(|a| a.control_type == UIA_DOCUMENT);
    // Inside a page's menu without being one of its items: WhatsApp's reactions, buttons in
    // its message menu.
    const UIA_MENU: i32 = 50009;
    event.in_menu = event.in_document
        && chain
            .iter()
            .rev()
            .skip(1)
            .take_while(|a| a.control_type != UIA_DOCUMENT)
            .any(|a| a.control_type == UIA_MENU);
    event.menu_name = chain
        .iter()
        .rev()
        .skip(1)
        .take_while(|a| a.control_type != UIA_DOCUMENT)
        .find(|a| a.control_type == UIA_MENU)
        .and_then(|a| a.name.as_deref())
        .map(str::trim)
        .filter(|n| event.in_document && !n.is_empty())
        .map(str::to_owned);
    // Groups that are a control's content name nothing: not even as the nearest container.
    let (groups, inside_control) = nested_groups(chain);
    let control_content = !event.in_document && inside_control;
    if !event.in_document {
        event.groups = groups;
    }
    // A control in a group of Office's ribbon that UIA reaches, a window of its own (Word's
    // Font box, a RichEdit): in the ribbon's menus as its NetUI commands are, its group the
    // ribbon's tool bar (`netui_focus`) [obs, 1045/s7].
    if let Some(chunk) = chain
        .iter()
        .rev()
        .skip(1)
        .find(|a| a.class_name.as_deref() == Some(NETUI_RIBBON_GROUP_CLASS))
    {
        event.in_ribbon = true;
        event.ribbon_menu = true;
        if let Some(name) = chunk
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            match event.groups.iter_mut().find(|g| g.name.trim() == name) {
                Some(group) => group.tool_bar = true,
                None => event.groups.push(okur_core::GroupEntry::tool_bar(name)),
            }
        }
    }
    // Office's search box beside the ribbon's tabs (Word's "Tell me what you want to do..."):
    // in the ribbon, not on its menus, and in the menu item that holds it, which JAWS says
    // first, "Tell me what you want to do... sub menu" [obs, 1051/s11].
    const UIA_TAB: i32 = 50018;
    if chain.iter().rev().skip(1).any(|a| {
        a.control_type == UIA_TAB && a.class_name.as_deref() == Some(NETUI_RIBBON_TABS_CLASS)
    }) {
        event.in_ribbon = true;
    }
    const UIA_MENU_ITEM: i32 = 50011;
    if focused.control_type == UIA_EDIT {
        if let Some(item) = chain
            .iter()
            .rev()
            .nth(1)
            .filter(|p| p.control_type == UIA_MENU_ITEM)
        {
            event.container_role = Some(Role::MenuItem);
            event.container_name = item.name.clone().filter(|n| !n.trim().is_empty());
            return Some(event);
        }
    }
    event.read_only = focused.value_is_read_only;
    event.platform.element_class = focused.class_name.clone().filter(|c| !c.is_empty());
    event.platform.automation_id = focused.automation_id.clone().filter(|i| !i.is_empty());
    if event.in_document {
        event.dom_id = focused.automation_id.clone().filter(|id| !id.is_empty());
        event.web_application = chain
            .iter()
            .rev()
            .skip(1)
            .find(|a| a.aria_role.as_deref() == Some("application"))
            .map(|a| a.name.clone().unwrap_or_default());
    }

    event.access_key = focused.access_key.clone().filter(|k| !k.is_empty());
    event.accelerator_key = focused
        .accelerator_key
        .clone()
        .filter(|k| !k.trim().is_empty());
    // An edit inside a combo box is the combo box's text field (an "edit combo").
    let parent = chain.iter().rev().nth(1);
    if focused.control_type == UIA_EDIT && parent.is_some_and(|p| p.control_type == UIA_COMBOBOX) {
        event.container_role = Some(Role::ComboBox);
        event.container_name = parent.and_then(|p| p.name.clone());
        event.platform.container_class = parent.and_then(|p| p.class_name.clone());
        // The access key belongs to the combo box; the position is its list's selection
        // (from the Win32 combo box), not the edit's own "1 of 1".
        event.access_key = event.access_key.or_else(|| {
            parent
                .and_then(|p| p.access_key.clone())
                .filter(|k| !k.is_empty())
        });
        event.item_index = None;
        event.item_count = None;
        return Some(event);
    }
    // A list view's edit renaming its item (F2) is not in the list [obs, 751/s18, s20].
    if focused.control_type == UIA_EDIT
        && parent.is_some_and(|p| is_list_view(p.class_name.as_deref()))
    {
        event.list_label_edit = true;
        event.groups.clear();
        return Some(event);
    }
    // The nearest structural container (list, group, tab list, tree, table, dialog, landmark).
    for ancestor in chain.iter().rev().skip(1) {
        let mut a = ancestor.clone();
        a.children.clear();
        let role = crate::uia_snapshot::to_aom(&a).role;
        let is_container = matches!(
            role,
            Role::List
                | Role::TabList
                | Role::Tree
                | Role::Table
                | Role::Dialog
                | Role::AlertDialog
                | Role::Landmark { .. }
        ) || (role == Role::Group
            && a.name.as_deref().is_some_and(|n| !n.trim().is_empty()));
        if is_container {
            if matches!(role, Role::Dialog | Role::AlertDialog) {
                event.container_description =
                    a.full_description.clone().filter(|d| !d.trim().is_empty());
            }
            event.container_name = a.name.clone();
            event.platform.container_class = a.class_name.clone();
            event.container_is_list_view = is_list_view(a.class_name.as_deref());
            event.container_is_list_box =
                role == Role::List && a.aria_role.as_deref() == Some("listbox");
            event.container_role = Some(role);
            break;
        }
    }
    if control_content && event.container_role == Some(Role::Group) {
        event.container_name = None;
    }
    Some(event)
}

/// The named groups and the lists above the focused element (`chain`'s last), outermost first,
/// up to its window. Windows Settings' toggle switch is in its "Night light" card, in the
/// "Brightness & color" section, and JAWS names both on the way in; a list item of Sound's
/// "Troubleshoot common sound problems" list is in that list, its card and the "Advanced"
/// section, all named (dev box, 2026-10-16). A text a control is in counts as a group
/// (Settings' "More about HDR" link, in a text of that name). Scroll panes are passed through.
/// Groups that are a control's content (Settings' brightness slider, in a group inside the
/// "Show more settings" button) are left out, JAWS names none there: the groups are a list
/// item's, or none inside another control. The second value says the focus is a control's
/// content.
pub(super) fn nested_groups(chain: &[UiaNode]) -> (Vec<okur_core::GroupEntry>, bool) {
    const UIA_GROUP: i32 = 50026;
    const UIA_PANE: i32 = 50033;
    const UIA_LIST: i32 = 50008;
    let named = |node: &UiaNode| {
        node.name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_owned)
    };
    // Whether a list is above, through groups, texts and panes: a control there is the list's
    // entry, as an item is.
    let in_list = |above: &[UiaNode]| {
        above
            .iter()
            .rev()
            .take_while(|a| matches!(a.control_type, UIA_GROUP | UIA_TEXT | UIA_PANE | UIA_LIST))
            .any(|a| a.control_type == UIA_LIST)
    };
    let mut entries = Vec::new();
    let mut inside_control = false;
    let ancestors = chain.len().saturating_sub(1);
    for (i, ancestor) in chain.iter().enumerate().take(ancestors).rev() {
        match ancestor.control_type {
            // An unnamed WPF group box by its header (`UiaNode::label`).
            UIA_GROUP | UIA_TEXT => entries.extend(
                named(ancestor)
                    .or_else(|| ancestor.label.clone())
                    .map(okur_core::GroupEntry::group),
            ),
            // An unnamed list is named by the text just before it (`UiaNode::label`).
            UIA_LIST => entries.push(okur_core::GroupEntry::list(
                named(ancestor)
                    .or_else(|| ancestor.label.clone())
                    .unwrap_or_default(),
            )),
            UIA_PANE => {}
            // A dialog inside a window that the control is in (Settings' InfoBar): its name, or
            // its first text. A dialog window is named as its window opens.
            _ if ancestor.is_dialog == Some(true) && ancestor.control_type != UIA_WINDOW => entries
                .push(okur_core::GroupEntry::dialog(
                    named(ancestor)
                        .or_else(|| ancestor.label.clone())
                        .unwrap_or_default(),
                )),
            // Inside a control: a button, a check box, a combo box, an edit, a link, a list or
            // tree item, a menu item, a radio button, a slider, a tab, a data item, a split
            // button.
            50000 | 50002 | 50003 | 50004 | 50005 | 50007 | 50011 | 50013 | 50015 | 50019
            | 50024 | 50029 | 50031 => {
                inside_control = true;
                // Inside a list, tree or data item the groups go on through the item (Sound's
                // "Output devices" link, in its list item: JAWS names "Advanced" no more after
                // it; Network's Wi-Fi switch, in a "Wi-Fi" group in its list item: JAWS names
                // "Wi-Fi"); inside another control there are none (Display's brightness
                // slider, in a button: JAWS names "Brightness & color" again after it). A
                // control that is a list's entry is its item (Notifications' app switch, in
                // the app's button in the list: JAWS names "Notifications from apps and other
                // senders" no more after it).
                let item = matches!(ancestor.control_type, 50007 | 50024 | 50029)
                    || chain.get(..i).is_some_and(in_list);
                if !item {
                    return (Vec::new(), true);
                }
            }
            // Any other container (the window, a tab control, a toolbar) ends the groups.
            _ => break,
        }
    }
    entries.reverse();
    (entries, inside_control)
}

/// The focus event a recorded `focus` trace event delivers to the engine: the chain mapping
/// of [`focus_event_from_chain`], plus the context a live listener would query.
///
/// Without a provider PositionInSet (WinForms and most Win32 controls report none), the
/// position is counted among the focused element's siblings of the same control type.
pub fn focus_event_from_trace(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
    // A page's menu item (an ARIA menu): Chrome reports the focus on the menu item and on its
    // document, both with the item focused through UIA, which gives its place: on the menu
    // bar or in a menu, its position, and whether it opens a menu (Google Docs' menus; JAWS
    // says "Help | 9 of 10" for a bar item, "Verbalize to screen reader s ► sub menu" for an
    // item that opens one; dev box, 2026-10-16).
    if let Some(item) = event
        .focus_chain
        .last()
        .filter(|f| f.control_type == UIA_MENUITEM)
        .filter(|_| {
            event
                .focus_chain
                .iter()
                .any(|n| n.control_type == UIA_DOCUMENT)
        })
    {
        const UIA_MENUBAR: i32 = 50010;
        let mut focus = okur_core::FocusEvent::new(okur_core::Role::MenuItem)
            .with_name(item.name.clone().unwrap_or_default());
        focus.in_document = true;
        focus.item_index = item.position_in_set.and_then(|p| usize::try_from(p).ok());
        focus.item_count = item.size_of_set.and_then(|s| usize::try_from(s).ok());
        // An item that opens a menu reports no expand state through Chromium's UIA when its
        // menu is closed, only its popup (`haspopup=menu`).
        let opens_menu =
            crate::uia_snapshot::aria_property(item, "haspopup").is_some_and(|p| p != "false");
        focus.states.expanded = match item.expand_collapse_state {
            Some(0) => Some(false),
            Some(1) | Some(2) => Some(true),
            _ => opens_menu.then_some(false),
        };
        focus.in_menu_bar = event
            .focus_chain
            .iter()
            .rev()
            .nth(1)
            .is_some_and(|parent| parent.control_type == UIA_MENUBAR);
        focus.under_menu_bar = event
            .focus_chain
            .iter()
            .any(|n| n.control_type == UIA_MENUBAR);
        // The menu's own name, where its menu button gives it one ("Actions").
        const UIA_MENU: i32 = 50009;
        focus.menu_name = event
            .focus_chain
            .iter()
            .rev()
            .nth(1)
            .filter(|parent| parent.control_type == UIA_MENU)
            .and_then(|menu| menu.name.as_deref())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned);
        // Without a reported position, the item's place among its menu's items: "Mesaj
        // bilgisi | 1 of 9" (WhatsApp's message menu, JAWS on the dev box, 2026-10-17).
        if focus.item_index.is_none() {
            let peers: Vec<&UiaNode> = event
                .focus_siblings
                .iter()
                .filter(|s| s.control_type == UIA_MENUITEM)
                .collect();
            let index = peers
                .iter()
                .position(|s| s.has_keyboard_focus)
                .or_else(|| peers.iter().position(|s| s.name == item.name));
            if let Some(index) = index {
                focus.item_index = Some(index + 1);
                focus.item_count = Some(peers.len());
            }
        }
        return Some(focus);
    }
    // A page's menu item focused through MSAA when UIA has the focus elsewhere: the page
    // moved on (Google Docs puts the focus on a bar item for a moment as Escape closes its
    // menu, then back in the document); not a menu to enter.
    let page_window = event.window_class.as_deref() == Some("Chrome_RenderWidgetHostHWND");
    if page_window
        && event.object.as_ref().and_then(|o| o.role) == Some(MSAA_MENUITEM)
        && event
            .focus_chain
            .last()
            .is_some_and(|f| f.control_type != UIA_MENUITEM)
    {
        return None;
    }
    // Menus: UIA keeps reporting the control focused before the menu (WinForms MenuStrip),
    // while the MSAA focus event names the menu item, as JAWS reads it [obs, 302].
    // Where UIA has the menu item itself (Chrome's app menu), it is read from UIA, which gives
    // its position and keys.
    let uia_has_item = event.focus_chain.last().is_some_and(|f| {
        f.control_type == UIA_MENUITEM
            && f.name.is_some()
            && f.name == event.object.as_ref().and_then(|o| o.name.clone())
    });
    if let Some(object) = event
        .object
        .as_ref()
        .filter(|o| o.role == Some(MSAA_MENUITEM) && !uia_has_item)
    {
        // Unavailable, as MSAA says (STATE_SYSTEM_UNAVAILABLE): "Undo Unavailable" [obs,
        // 756/s15].
        const STATE_SYSTEM_UNAVAILABLE: u32 = 0x1;
        let mut item = okur_core::FocusEvent::new(okur_core::Role::MenuItem)
            .with_name(object.name.clone().unwrap_or_default());
        item.states.disabled = object.states & STATE_SYSTEM_UNAVAILABLE != 0;
        // Its access key, MSAA's ("n", the menu bar's "Alt+f"), said last where access keys
        // are said in menus [obs, 1084/s1-s3].
        item.access_key = object.shortcut.clone().filter(|k| !k.trim().is_empty());
        return Some(item);
    }
    if let Some(mut focus) = netui_focus(event) {
        // Its window's class: Word's task panes are NetUI windows outside the ribbon.
        focus.platform.window_class = event.window_class.clone();
        return Some(focus);
    }
    if let Some(focus) = command_bar_focus(event) {
        return Some(focus);
    }
    // Word's ribbon taking the focus back from a box of its own (the Font box, a RichEdit
    // window) reports the ribbon itself, then the command that has the focus [obs, 1045/s9]:
    // the ribbon with none of its tabs focused is no focus of its own.
    if event.window_class.as_deref() == Some("NetUIHWND")
        && event.id_object == OBJID_CLIENT
        && event.object.as_ref().and_then(|o| o.role) == Some(MSAA_PROPERTYPAGE)
    {
        return None;
    }
    // A NetUI window's focus is its MSAA object's (`netui_focus`): a focus event naming none
    // is UI Automation's own, passed on as a WinEvent, and UIA reaches only the window: it
    // said "Leaving menus" after each of Word's ribbon commands [obs, 1045/s2-s12].
    if event.event == "focus"
        && event.window_class.as_deref() == Some("NetUIHWND")
        && event.object.is_none()
    {
        return None;
    }
    // An Office dialog's field reports the focus from its own window too, between its dialog's
    // two reports of it: MSAA gives the window's client, UI Automation a pane right in the
    // dialog, both named by the field's text or by nothing. The dialog's report is the field
    // (MSAA's editable text, below); this one names nothing of it [obs, 1124/s1, s8].
    const MSAA_CLIENT: i32 = 0x0A;
    if let Some(client) = event
        .object
        .as_ref()
        .filter(|o| event.event == "focus" && event.id_child == 0 && o.role == Some(MSAA_CLIENT))
    {
        const UIA_PANE: i32 = 50033;
        let text = client.name.as_deref().map(str::trim).unwrap_or_default();
        let mut chain = event.focus_chain.iter().rev();
        let (field, around) = (chain.next(), chain.next());
        if field.is_some_and(|f| {
            f.control_type == UIA_PANE
                && f.name.as_deref().map(str::trim).unwrap_or_default() == text
                && !f.is_control_element
        }) && around.is_some_and(|w| w.control_type == UIA_WINDOW && w.is_dialog == Some(true))
        {
            return None;
        }
    }
    // The recorded window text belongs to the focused HWND (an edit's text is its content);
    // the window the user is in is the outermost element of the focus chain.
    // The active window is the innermost window in the chain (the focus itself, when a window
    // has focus): UIA nests a dialog under the
    // window that owns it [obs, 306/s1, s10].
    let active_window = event
        .focus_chain
        .iter()
        .rev()
        .find(|w| w.control_type == UIA_WINDOW);
    // The window is named as JAWS names it: the desktop by its virtual desktop ("Desktop 1"),
    // anything else by its top-level window's caption, which for Chrome names the profile the
    // UIA name leaves out [obs, 209/s1: "... - Google Chrome - <profile>"]. Traces from before
    // the caption was recorded (format 7) fall back to the UIA window's name.
    // A dialog around the focus is the window, whichever window is in front as the event
    // comes: JAWS's list dialogs take the focus while its hidden main window is still in
    // front, and the dialog comes to the front a moment later [obs, 224/s1, round 52].
    let dialog = active_window.filter(|w| w.is_dialog == Some(true));
    let top_window = event
        .virtual_desktop
        .as_deref()
        .or(dialog.and_then(|w| w.name.as_deref()))
        .or(event.top_window_title.as_deref())
        .or(active_window.and_then(|w| w.name.as_deref()));
    // A focus event from a window already gone (Okur's own list dialog as it closes) has no
    // class: the focus it reads is UIA's, in the window with the keyboard focus. Taken as
    // not the page's, it left Okur out of the virtual cursor after Enter in the Regions list
    // (live11, 256/s4).
    let mut focus = focus_event_from_chain(
        &event.focus_chain,
        event
            .window_class
            .as_deref()
            .or(event.focus_window_class.as_deref()),
        top_window,
    )?;
    focus.container_modal = event.modal_dialog.is_some()
        && matches!(
            focus.container_role,
            Some(okur_core::Role::Dialog | okur_core::Role::AlertDialog)
        );
    let focused = event.focus_chain.last()?;
    // A WPF dialog's window takes the focus a moment before its first control does, with
    // nothing of the dialog's text; the control's focus brings it, said with the dialog:
    // "Name Your Rule dialog | Please enter a name for the rule. | Rule Name: Edit" [obs,
    // 783/s7]. The window's own focus is not one.
    if let Some(window) = dialog.filter(|w| w.class_name.as_deref() != Some(WIN32_DIALOG_CLASS)) {
        if focused.control_type == UIA_WINDOW && focused.runtime_id == window.runtime_id {
            return None;
        }
    }
    toolbar_button(event, focused, &mut focus);
    // A Win32 toolbar's own focus, which its hot button's follows at once: JAWS says the
    // button alone, in its toolbar [obs, 3156/s3, s4]. Said, the toolbar's name ("Up band
    // toolbar") was cut off by the button's a moment later.
    if focused.control_type == UIA_TOOLBAR
        && event.window_class.as_deref() == Some("ToolbarWindow32")
        && event.id_child == 0
    {
        return None;
    }
    header_bar(event, focused, &mut focus);
    // A Win32 combo box's own focus while UI Automation's chain is still on the edit before
    // it: Tab from Save As's File name to Save as type gives the MSAA object "Save as type:"
    // with the chain on File name's edit, and JAWS reads the combo box, "Save as type: Combo
    // box | Word Document (*.docx) | 1 of 17" [obs, 1077/s2]. The combo box is the focus, as
    // MSAA has it, its place in its list asked of the control.
    const MSAA_COMBOBOX: i32 = 0x2E;
    if let Some(object) = event.object.as_ref().filter(|o| {
        o.role == Some(MSAA_COMBOBOX)
            && focused.control_type == UIA_EDIT
            && o.name
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .is_some_and(|n| focused.name.as_deref().map(str::trim) != Some(n))
    }) {
        focus.role = okur_core::Role::ComboBox;
        focus.name = object.name.clone();
        focus.value = object.value.clone().filter(|v| !v.trim().is_empty());
        focus.text = None;
        focus.container_role = None;
        focus.container_name = None;
        focus.item_index = None;
        focus.item_count = None;
    }
    // An Office dialog's field (Excel's Go To, a `bosa_sdm` window): UI Automation reaches only
    // the field's window, a pane named by nothing or by the field's text, where the dialog's
    // MSAA names the field by its label as editable text, its value the text. JAWS reads MSAA's: "Reference: Edit | A1 |
    // Type in text." [obs, 1124/s1, s8].
    const UIA_PANE: i32 = 50033;
    if let Some(object) = event.object.as_ref().filter(|o| {
        o.role == Some(MSAA_TEXT)
            && event.id_child > 0
            && focused.control_type == UIA_PANE
            && focused.name.as_deref().map(str::trim).unwrap_or_default()
                == o.value.as_deref().map(str::trim).unwrap_or_default()
            && o.name.as_deref().is_some_and(|n| !n.trim().is_empty())
    }) {
        focus.role = okur_core::Role::Edit;
        focus.name = object.name.clone();
        focus.value = object.value.clone().filter(|v| !v.trim().is_empty());
        focus.text = None;
        focus.item_index = None;
        focus.item_count = None;
    }
    // A Win32 dialog's control names its access key through MSAA only: the focus event's
    // object is the control ("Script: Combo box … Alt+r" [obs, 541/s8]).
    if focus.access_key.is_none() {
        focus.access_key = event.object.as_ref().and_then(|o| o.shortcut.clone());
    }
    // A page's window reports its focus by the object's IA2 uniqueID (negative), the identity
    // its captures give each node: WhatsApp renames a message as it takes the focus.
    if focus.in_document
        && event.id_child < 0
        && event
            .window_class
            .as_deref()
            .is_some_and(|c| WEB_CONTENT_WINDOW_CLASSES.contains(&c))
    {
        focus.unique_id = Some(event.id_child.to_string());
    }
    // A modeless dialog in front: its owner stays available [obs, 1065/s6].
    focus.modeless = event.top_window_owner_enabled == Some(true);
    focus.app_window_texts.clone_from(&event.app_window_texts);
    // The window's kind is its class: the desktop's, the window in front, or (older traces)
    // the UIA window around the focus.
    focus.platform.top_window_class = if event.virtual_desktop.is_some() {
        Some(DESKTOP_KIND.to_owned())
    } else {
        dialog
            .and_then(|w| w.class_name.clone())
            .or_else(|| event.top_window_class.clone())
            .or_else(|| active_window.and_then(|w| w.class_name.clone()))
    };
    // A modern app's window is read element by element, as JAWS's Scan Cursor walks it. Its
    // window's class tells, not a dialog's UI Automation class: a WPF window is a dialog of
    // class "Window" when the focus is inside it (1022).
    focus.read_by_elements = [
        focus.platform.top_window_class.as_deref(),
        event.top_window_class.as_deref(),
        event.window_class.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|class| crate::classes::reads_by_elements(class, ""));
    // A window of files: Convenient OCR's F reads the file selected in it.
    focus.in_file_view = [
        focus.platform.top_window_class.as_deref(),
        event.top_window_class.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(crate::classes::is_file_view);
    // A window UI Automation calls a dialog is one too, whatever draws it: JAWS's WPF windows,
    // "Name Your Rule dialog" [obs, 783/s7].
    focus.in_dialog_box =
        focus.platform.top_window_class.as_deref() == Some(WIN32_DIALOG_CLASS) || dialog.is_some();
    // A Win32 list view outside a dialog box is named by its window's text: Programs and
    // Features' "FolderView List view" [obs, 3156/s1], where UI Automation and MSAA say "Folder
    // View". In a dialog box it is named by its label, as its accessible name has it: the
    // Print dialog's "Folder View List view" [obs, 546].
    let list_view_text = event
        .window_title
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .filter(|_| !focus.in_dialog_box && event.window_class.as_deref() == Some("SysListView32"))
        .map(str::to_owned);
    if let Some(text) = list_view_text
        .as_ref()
        .filter(|_| focus.container_is_list_view)
    {
        focus.container_name = Some(text.clone());
        if let Some(list) = focus.groups.iter_mut().rev().find(|g| g.list) {
            list.name.clone_from(text);
        }
    }
    focus.multi_line = multi_line_edit(
        event
            .focus_window_class
            .as_deref()
            .or(event.window_class.as_deref()),
        event.window_style,
    );
    focus.plain_edit = plain_edit(
        event
            .focus_window_class
            .as_deref()
            .or(event.window_class.as_deref()),
    );
    focus.rich_edit = rich_edit(
        event
            .focus_window_class
            .as_deref()
            .or(event.window_class.as_deref()),
    );
    // A classic RichEdit field is known by its MSAA name (a ribbon's "Font family", "Font
    // size"), where UI Automation's proxy names every one "RichEdit Control"; the name tells
    // two fields apart, though neither is said [obs, 970/s9, s10]. A spin button's too: Word's
    // Paragraph dialog, "Left: edit combo" [obs, 1060/s6].
    if focus.rich_edit && focus.role == okur_core::Role::Edit {
        if let Some(name) = event
            .object
            .as_ref()
            .filter(|o| matches!(o.role, Some(MSAA_TEXT | MSAA_SPIN_BUTTON)))
            .and_then(|o| o.name.clone())
        {
            focus.name = Some(name);
        }
        // Its value as MSAA gives it: the text read with the focus can still be the field
        // before's, "Calibri" in the size field where JAWS says "16" [obs, 970/s9].
        if !focus.multi_line {
            if let Some(value) = event.object.as_ref().and_then(|o| o.value.clone()) {
                if focus.text.as_ref().is_some_and(|t| t.text != value) {
                    focus.text = None;
                }
                focus.value = Some(value);
            }
        }
    }
    focus.spin_box = event.spin_box && focus.role == okur_core::Role::Edit;
    focus.password = focus.role == okur_core::Role::Edit
        && event
            .focus_chain
            .last()
            .is_some_and(|node| node.is_password);
    focus.menu_bar = event
        .menu_bar
        .iter()
        .map(|item| crate::text::without_accelerators(item))
        .filter(|item| !item.trim().is_empty())
        .collect();
    focus.status_bar = event
        .status_bar
        .iter()
        .map(|part| {
            okur_core::viewer_text::without_marks(part)
                .trim()
                .to_owned()
        })
        .filter(|part| !part.is_empty())
        .collect();
    // A dialog box (Win32 #32770, as a message box is) is announced with its static text,
    // which sits beside its buttons [obs, 306/s1]. Other windows are announced by title only
    // (306/s10).
    if in_dialog_box(&event.focus_chain) {
        // On a property sheet's page, the text beside the focus is the page's labels ("Type:",
        // "Local Disk"): JAWS 2025 opens Local Disk (C:) Properties naming the page instead,
        // "General page" (dev box, 2026-10-15). A wizard shows no tabs and takes its page's
        // title as its own: its page is not named, and the page's text is the dialog's,
        // "Flexible Web dialog | What would you like to do?" [obs, 705/s3].
        let wizard_page = event
            .dialog_page
            .clone()
            .filter(|page| event.top_window_title.as_deref() == Some(page.as_str()));
        let tabbed_page = event.dialog_page.is_some() && wizard_page.is_none();
        focus.dialog_page = event.dialog_page.clone().filter(|_| tabbed_page);
        focus.wizard_page = wizard_page;
        // The dialog boxes in the focus's chain over which the active one opened: owned by
        // them, UI Automation puts it inside them.
        let win32 = |n: &&UiaNode| {
            n.control_type == UIA_WINDOW && n.class_name.as_deref() == Some(WIN32_DIALOG_CLASS)
        };
        let dialogs: Vec<&UiaNode> = event.focus_chain.iter().filter(win32).collect();
        // Over a Win32 message box, a WPF window UI Automation calls a dialog is named too,
        // plainly: JAWS says "Manage Notification Rules" before the "Confirm Delete" box it
        // owns [obs, 787/s17]; a WPF dialog over another names only itself (787/s5).
        let over_win32 = event
            .focus_chain
            .iter()
            .rev()
            .find(|n| n.control_type == UIA_WINDOW)
            .is_some_and(|n| win32(&n));
        if over_win32 {
            focus.outer_windows = event
                .focus_chain
                .iter()
                .filter(|n| n.control_type == UIA_WINDOW && n.is_dialog == Some(true) && !win32(n))
                .filter_map(|n| n.name.clone())
                .collect();
        }
        focus.outer_dialogs = dialogs
            .split_last()
            .map(|(_, outer)| outer.iter().filter_map(|n| n.name.clone()).collect())
            .unwrap_or_default();
        focus.group_box = event.group_box.clone();
        focus.default_button = event.default_button.clone();
        // An unnamed field is named by the text just before it, as a Win32 dialog's static
        // text names the field after it; UI Automation gives a WPF dialog's field no name:
        // "Please enter a name for the rule. | Rule Name: Edit" [obs, 783/s7].
        let field = matches!(focused.control_type, UIA_EDIT | UIA_COMBOBOX)
            && focused.name.as_deref().is_none_or(|n| n.trim().is_empty());
        let label_before = field
            .then(|| {
                let at = event.focus_siblings.iter().position(|s| {
                    s.control_type == focused.control_type
                        && s.name.as_deref().is_none_or(|n| n.trim().is_empty())
                        && (s.runtime_id.is_none() || s.runtime_id == focused.runtime_id)
                })?;
                let before = event.focus_siblings.get(at.checked_sub(1)?)?;
                (before.control_type == UIA_TEXT)
                    .then(|| before.name.clone())
                    .flatten()
                    .filter(|n| !n.trim().is_empty())
            })
            .flatten();
        if let Some(label) = &label_before {
            focus.name = Some(label.clone());
        }
        // A WPF group box's line is its header and the texts before the control in it, said
        // again when the line changes: "Speech or Sound Action Path to Sound File:" before
        // "Path to Sound File: Edit", after the combo box in the same group [obs, 801/s11,
        // 798/s9].
        let in_group_box = event.focus_chain.iter().rev().nth(1).is_some_and(|parent| {
            parent.control_type == 50026 && parent.class_name.as_deref() == Some("GroupBox")
        });
        if in_group_box && focused.runtime_id.is_some() {
            let at = event
                .focus_siblings
                .iter()
                .position(|s| s.runtime_id == focused.runtime_id);
            let texts: Vec<String> = at
                .and_then(|at| event.focus_siblings.get(..at))
                .unwrap_or_default()
                .iter()
                .filter(|s| s.control_type == UIA_TEXT)
                .filter_map(|s| s.name.as_deref().map(str::trim))
                .filter(|n| !n.is_empty() && !n.starts_with('_'))
                .map(str::to_owned)
                .collect();
            if let (false, Some(group)) = (texts.is_empty(), focus.groups.last_mut()) {
                group.name = format!("{} {}", group.name, texts.join(" "));
            }
        }
        // A text that names a control beside it is that control's label, said with the
        // control, not as the dialog's text: JAWS opened Okur's settings with "Okur Settings
        // dialog | Speech Synthesizer: Combo box", without "Voice:", "Rate:" (dev box,
        // 2026-10-17).
        // So is a text naming a control the focus is inside: "Regions" of Okur's Document
        // Regions tree, whose item has the focus (JAWS: "Document Regions dialog | Regions
        // Tree view | Banner Opened" [obs, 274/s1]).
        // And so is a text naming any control of the dialog: the Font dialog's "Script:", on
        // a row of its own above its combo box (JAWS: "Font dialog | Font: edit combo"
        // [obs, 541/s2]).
        // But a text naming the focused field that takes no access key from it is the
        // dialog's prompt, said before the field: "Custom Label dialog | Enter a custom label
        // for this item | Enter a custom label for this item edit" [obs, 688/s9; 559/s1],
        // where the Font dialog's "&Font:" gives its combo box Alt+F.
        let focused = event.focus_chain.last();
        let prompt = |node: &UiaNode| {
            focused.is_some_and(|f| {
                f.control_type != UIA_TEXT
                    && f.access_key.as_deref().is_none_or(str::is_empty)
                    && node.name == f.name
                    && (node.runtime_id.is_none() || node.runtime_id == f.runtime_id)
            })
        };
        fn control_names<'a>(
            node: &'a UiaNode,
            prompt: &dyn Fn(&UiaNode) -> bool,
            names: &mut Vec<&'a str>,
        ) {
            for child in &node.children {
                if child.control_type != UIA_TEXT && !prompt(child) {
                    names.extend(child.name.as_deref());
                }
                control_names(child, prompt, names);
            }
        }
        let mut labels: Vec<&str> = event
            .focus_siblings
            .iter()
            .filter(|s| s.control_type != UIA_TEXT)
            .chain(
                event
                    .focus_chain
                    .iter()
                    .filter(|n| n.control_type != UIA_WINDOW),
            )
            .filter(|n| !prompt(n))
            .filter_map(|s| s.name.as_deref())
            .collect();
        if let Some(tree) = &event.dialog_tree {
            control_names(tree, &prompt, &mut labels);
        }
        labels.extend(label_before.as_deref());
        let text: Vec<&str> = event
            .focus_siblings
            .iter()
            .filter(|_| !tabbed_page)
            .filter(|s| s.control_type == UIA_TEXT)
            .filter_map(|s| s.name.as_deref())
            .filter(|n| !n.trim().is_empty() && !labels.contains(n))
            .collect();
        // Focus inside a field (the Run dialog's combo box) has no text beside it: the
        // dialog's own static text is read from its window [JAWS on Windows 11, dev box:
        // "Run dialog | Type the name of a program, …"].
        focus.dialog_text = Some(if text.is_empty() {
            event
                .dialog_text
                .iter()
                .filter(|t| !labels.contains(&t.as_str()))
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            text.join("\n")
        });
        focus.dialog_buttons = event
            .focus_siblings
            .iter()
            .filter(|s| s.control_type == UIA_BUTTON)
            .filter_map(|s| s.name.clone())
            .collect();
    }
    // A File Explorer window is titled "<folder> - <File Explorer, localized>": JAWS names the
    // folder after the title on entering it ("backlog - File Explorer | backlog"; dev box,
    // 2026-10-17).
    if focus.platform.top_window_class.as_deref() == Some(EXPLORER_WINDOW_CLASS) {
        focus.folder_name = focus
            .window_title
            .as_deref()
            .and_then(|t| t.rsplit_once(" - "))
            .map(|(folder, _)| folder.to_owned());
    }
    focus.dialog_lines = event
        .dialog_lines
        .iter()
        .map(|line| okur_core::viewer_text::without_marks(line))
        .collect();
    // Windows wraps dates in direction marks; JAWS says none [obs, 323/s2, s7].
    focus.name = focus
        .name
        .map(|n| okur_core::viewer_text::without_marks(&n));
    focus.value = focus
        .value
        .map(|v| okur_core::viewer_text::without_marks(&v));
    focus.app = event.process.clone();
    focus.app_product = event.process_product.clone();
    focus.app_version = event.process_version.clone();
    focus.bounds = event.focus_chain.last().and_then(|n| n.bounds);
    focus.window_items = event
        .dialog_tree
        .as_ref()
        .map(window_items)
        .unwrap_or_default();
    focus.file_dialog_address = event.dialog_tree.as_ref().and_then(address_band);
    // An unnamed list or tree in a dialog is named by the text just before it, as MSAA names
    // a control by the label before it: "Regions Tree view" (JAWS's Document Regions
    // [obs, 241/s1, s6]).
    if matches!(
        focus.container_role,
        Some(okur_core::Role::Tree | okur_core::Role::List)
    ) && focus
        .container_name
        .as_deref()
        .is_none_or(|n| n.trim().is_empty())
    {
        let container = event
            .focus_chain
            .iter()
            .rev()
            .skip(1)
            .find(|n| matches!(n.control_type, UIA_TREE | UIA_LIST) && n.runtime_id.is_some());
        if let (Some(container), Some(tree)) = (container, &event.dialog_tree) {
            focus.container_name = label_before(tree, container.runtime_id.as_deref());
        }
        // With no text before it, a list box is named by its dialog: "Select a System Tray
        // Icon List box" (JAWS's own dialog, its only list unlabelled [obs, 511/s4]); a list
        // view is not: " List view" [obs, 742/s18, 748/s7].
        if focus
            .container_name
            .as_deref()
            .is_none_or(|n| n.trim().is_empty())
            && focus.container_role == Some(okur_core::Role::List)
            && !focus.container_is_list_view
        {
            focus.container_name = event
                .dialog_tree
                .as_ref()
                .and_then(|tree| tree.name.clone())
                .filter(|n| !n.trim().is_empty());
            // The list is named as it is entered, too.
            if let (Some(name), Some(list)) = (
                &focus.container_name,
                focus.groups.iter_mut().rev().find(|g| g.list),
            ) {
                if list.name.trim().is_empty() {
                    list.name = name.clone();
                }
            }
        }
    }
    // JAWS names buttons by their MSAA role: ROLE_SYSTEM_BUTTONMENU is a "button menu"
    // even when UIA reports a plain button [obs, 204/s11–s12].
    if focus.role == okur_core::Role::Button
        && event.object.as_ref().and_then(|o| o.role) == Some(MSAA_BUTTONMENU)
    {
        focus.role = okur_core::Role::MenuButton;
    }
    if let Some(subtree) = &event.focus_subtree {
        if focus.role == okur_core::Role::TreeItem {
            let children = subtree
                .children
                .iter()
                .filter(|c| c.control_type == UIA_TREEITEM)
                .count();
            focus.child_count = (children > 0).then_some(children);
        }
        // A shell item's kind is its type cell's value where UIA gives no ItemType: File
        // Explorer's "File folder" (the Windows property `System.ItemTypeText`).
        let cell = |id: &str| {
            subtree
                .children
                .iter()
                .find(|c| c.automation_id.as_deref() == Some(id))
                .and_then(|c| c.value.clone())
                .filter(|t| !t.trim().is_empty())
        };
        if focus.item_type.is_none() {
            focus.item_type = cell(SHELL_ITEM_TYPE);
        }
        focus.item_size = cell(SHELL_ITEM_SIZE);
        // A shell item's whole row, its date as well: "SR OCR Sign.png | 1/2/2026 3:04 AM PNG
        // File 11 KB" [obs, 1024/s11].
        focus.item_details = subtree
            .children
            .iter()
            .filter(|c| {
                c.automation_id
                    .as_deref()
                    .is_some_and(|id| id.starts_with(SHELL_PROPERTY) && id != SHELL_ITEM_NAME)
            })
            .filter_map(|c| c.value.clone())
            .filter(|v| !v.trim().is_empty())
            .collect();
        // A list-view item's further columns are its text children after the name.
        if focus.role == okur_core::Role::ListItem {
            // Without the marks that steer a date's direction, which JAWS's speech has none of:
            // "Igor Pavlov | 11/7/2020" [obs, 3156/s11]. Its texts only: the buttons an opened
            // item shows (Settings' "Advanced options", "Uninstall") are controls of their own
            // [obs, 3155/s11].
            // The item's own text, its first column, is its name; a later column with the same
            // text is said: Manage Key Labels' "Zoom | Zoom | Speak" [obs, 1169/s21].
            let mut own_text = focused.name.as_ref();
            focus.columns = subtree
                .children
                .iter()
                .filter(|c| matches!(c.control_type, UIA_TEXT | UIA_EDIT))
                .filter_map(|c| c.name.clone())
                .filter(|n| {
                    if own_text == Some(n) {
                        own_text = None;
                        return false;
                    }
                    !n.trim().is_empty()
                })
                .map(|n| okur_core::viewer_text::without_marks(&n))
                .collect();
            // A XAML list item is named after its texts, joined: Windows Settings' "7-Zip 18.05
            // (x64) 4.93 MB 11/7/2020" over three texts, which JAWS says once [obs, 3155/s7].
            if focused
                .name
                .as_deref()
                .is_some_and(|name| names_its_texts(name, &focus.columns))
            {
                focus.columns.clear();
            }
            focus.column_headers = event.column_headers.clone();
            // A WPF list item with no name of its own is named after its data object's type
            // ("NotificationViewer.Services.NotificationData"), which JAWS leaves out: it reads
            // the item's texts [obs, 774/s3].
            if focused.class_name.as_deref() == Some(WPF_LIST_ITEM)
                && focus.name.as_deref().is_some_and(is_type_name)
                && !focus.columns.is_empty()
            {
                focus.name = Some(focus.columns.remove(0));
            } else if !focus.container_is_list_view && !focus.columns.is_empty() {
                // An item with a name of its own, outside a list view, has its further texts
                // as its description, said after its position: Windows Settings' home,
                // "Devices | 2 of 13 | Bluetooth, printers, mouse" [obs, 3157/s4].
                focus.description = Some(focus.columns.join(" "));
                focus.columns.clear();
            }
        }
        // A shell item's columns are its cells' values, not their headers' names: "SR OCR
        // Sign.png | 1/2/2026 3:04 AM PNG File 11 KB | 3 of 3" [obs, 1029/s4].
        if !focus.item_details.is_empty() {
            focus.columns.clone_from(&focus.item_details);
        }
        // A combo box's position is its selected item's, among its list items. A closed WinUI
        // combo box holds only its selected item, which is its value, and no position:
        // Windows Settings' "Scale Combo box | 125% (Recommended)" (JAWS on the dev box,
        // 2026-10-16).
        if focus.role == okur_core::Role::ComboBox && focus.item_index.is_none() {
            let items: Vec<&UiaNode> = subtree
                .children
                .iter()
                .flat_map(|c| std::iter::once(c).chain(c.children.iter()))
                .filter(|n| n.control_type == UIA_LISTITEM)
                .collect();
            let selected = items.iter().position(|n| n.is_selected == Some(true));
            match (selected, items.as_slice()) {
                (Some(_), [only]) => {
                    if focus.value.is_none() {
                        focus.value = only.name.clone();
                    }
                }
                (Some(i), _) => {
                    focus.item_index = Some(i + 1);
                    focus.item_count = Some(items.len());
                }
                (None, _) => {}
            }
        }
    }
    if let (None, Some([index, count])) = (focus.item_index, event.list_position) {
        focus.item_index = usize::try_from(index).ok().map(|i| i + 1);
        focus.item_count = usize::try_from(count).ok();
    }
    // Siblings count positions among peers; a combo box's field has its list's position. A
    // button has only the position its application reports: JAWS says none for a dialog's
    // buttons (306), and "1 of 1" for Windows Settings' breadcrumb, which reports it (dev box,
    // 2026-10-16). A tab likewise: none for a WinForms tab (303/s12), "1 of 1" for File
    // Explorer's, which reports it (dev box, 2026-10-17). A menu item too: a reported position
    // is what tells a modern menu from a Win32 one (302 says none).
    // A grid's cell neither: its name says its column and row, and its place among the cells
    // read beside it was said nowhere (dropping it changed no step of the corpus, 2026-11-04).
    if focus.item_index.is_none()
        && focus.role != okur_core::Role::ComboBox
        && focus.role != okur_core::Role::Button
        && focus.role != okur_core::Role::Tab
        && focus.role != okur_core::Role::SplitButton
        && focus.role != okur_core::Role::MenuItem
        && focus.role != okur_core::Role::TableCell
        && focus.container_role != Some(okur_core::Role::ComboBox)
    {
        let peers: Vec<&UiaNode> = event
            .focus_siblings
            .iter()
            .filter(|s| s.control_type == focused.control_type)
            .collect();
        let index = peers
            .iter()
            .position(|s| s.has_keyboard_focus)
            .or_else(|| peers.iter().position(|s| s.name == focused.name));
        if let Some(index) = index {
            focus.item_index = Some(index + 1);
            focus.item_count = Some(peers.len());
        }
    }
    // A list view whose items are renamed in place (LVS_EDITLABELS) [obs, 748/s7].
    const LVS_EDITLABELS: i64 = 0x200;
    focus.list_edits_labels = event
        .window_class
        .as_deref()
        .or(event.focus_window_class.as_deref())
        == Some("SysListView32")
        && event
            .window_style
            .is_some_and(|style| i64::from(style) & LVS_EDITLABELS != 0);
    // A list view with the focus itself, as one without items has it: named by the text
    // before it (its window's), and its count, not its place among its peers: "PlaceMarker
    // List view | 0 items" [obs, 699/s6, 702/s4].
    if focused.control_type == UIA_LIST && is_list_view(focused.class_name.as_deref()) {
        focus.list_view = true;
        if focus.name.as_deref().is_none_or(|n| n.trim().is_empty()) {
            focus.name.clone_from(&focused.label);
        }
        // Outside a dialog box, by its window's text, as its items' list is [obs, 3156/s1].
        if list_view_text.is_some() {
            focus.name.clone_from(&list_view_text);
        }
        focus.item_index = None;
        focus.item_count = focused
            .grid_row_count
            .and_then(|count| usize::try_from(count).ok());
    }
    Some(focus)
}

/// A native application's live region changed (UIA): the texts it holds, then its own name
/// where that is not one of them, as JAWS reads Windows Settings' expander button as Alt+Left
/// shows its page again: " Brightness Change brightness for the built-in display | Show more
/// settings" (the slider inside is left out; dev box, 2026-10-17). A text region says its
/// name. A region whose live setting is off says nothing.
/// The UIA class of a WPF list box's items.
pub(super) const WPF_LIST_ITEM: &str = "ListBoxItem";

/// Whether `name` is a .NET type's full name, as WPF names an item by its data object's type
/// when the item has no name of its own: identifiers that start with a capital, joined by dots
/// ("NotificationViewer.Services.NotificationData").
pub(super) fn is_type_name(name: &str) -> bool {
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|part| {
            part.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

/// Whether an item's `name` is its `texts` joined by spaces, the marks that steer a date's
/// direction left out of both (XAML puts them in the texts and the name alike).
pub(super) fn names_its_texts(name: &str, texts: &[String]) -> bool {
    use okur_core::viewer_text::without_marks;
    let words = |text: &str| {
        without_marks(text)
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    !texts.is_empty() && words(name) == words(&texts.join(" "))
}

/// The text just before the control with `runtime_id` among its siblings in a dialog's tree:
/// the label that names it.
pub(super) fn label_before(tree: &UiaNode, runtime_id: Option<&[i32]>) -> Option<String> {
    let runtime_id = runtime_id?;
    let at = tree
        .children
        .iter()
        .position(|c| c.runtime_id.as_deref() == Some(runtime_id));
    match at {
        Some(i) => i
            .checked_sub(1)
            .and_then(|before| tree.children.get(before))
            .filter(|before| before.control_type == UIA_TEXT)
            .and_then(|before| before.name.clone())
            .map(|name| crate::text::without_accelerators(&name))
            .filter(|name| !name.trim().is_empty()),
        None => tree
            .children
            .iter()
            .find_map(|child| label_before(child, Some(runtime_id))),
    }
}
