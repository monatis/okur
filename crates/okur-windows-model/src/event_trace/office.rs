//! Office's own controls, which MSAA knows better than UI Automation: the ribbon (NetUI) and
//! its tabs, groups and galleries, the command bars and the task panes.

use super::*;

/// The focus in a NetUI window (WordPad's and Paint's ribbon, `NetUIHWND`), read from the
/// event's MSAA object: UI Automation stops at the ribbon's host pane, so every tab and button
/// came as the same "Ribbon" pane and was said once (523/s2, s3), where the MSAA object names
/// the tab ("View", a page tab, selected). `None` where UI Automation has the element itself.
pub(super) fn netui_focus(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
    use okur_core::Role;
    const PAGETAB: i32 = 0x25;
    const PUSHBUTTON: i32 = 0x2B;
    const CHECKBUTTON: i32 = 0x2C;
    const RADIOBUTTON: i32 = 0x2D;
    const COMBOBOX: i32 = 0x2E;
    const BUTTONDROPDOWN: i32 = 0x38;
    // A drop-down that opens a grid: Word's "Cover Page", a sub menu to JAWS [obs, 1043/s5].
    const BUTTONDROPDOWNGRID: i32 = 0x3A;
    const SPLITBUTTON: i32 = 0x3E;
    const SELECTED: u32 = 0x2;
    if event.window_class.as_deref() != Some("NetUIHWND") {
        return None;
    }
    let object = event.object.as_ref()?;
    // Alt or F10 gives the focus to the ribbon itself: its selected tab is the focus.
    if let Some(tab) = ribbon_tab(event) {
        return Some(tab);
    }
    // The ribbon itself, with no tab focused: Word's, on the way from its search box to
    // Collapse the Ribbon, which JAWS names, "Ribbon | Collapse the Ribbon Button" [obs,
    // 1051/s12].
    if let Some(page) = ribbon_page(event) {
        return Some(page);
    }
    let name = object
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())?;
    let uia_has_it = event
        .focus_chain
        .last()
        .is_some_and(|f| f.name.as_deref() == Some(name));
    if uia_has_it {
        return None;
    }
    // A gallery's item: Word's Styles gallery, an MSAA list in the tool window it opens.
    const LIST: i32 = 0x21;
    const LISTITEM: i32 = 0x22;
    if object.role == Some(LISTITEM)
        && event.msaa_ancestors.first().and_then(|a| a.role) == Some(LIST)
    {
        return Some(gallery_item(event, object, name));
    }
    // A list's item in an Office task pane: the Spelling pane's suggestions, "the | 1 of 5"
    // [obs, 1069/s4].
    if object.role == Some(LISTITEM)
        && in_task_pane(event)
        && event.msaa_ancestors.iter().any(|a| a.role == Some(LIST))
    {
        return Some(task_pane_item(event, object, name));
    }
    const UIA_BUTTON: i32 = 50000;
    const UIA_MENU_ITEM: i32 = 50011;
    const LINK: i32 = 0x1E;
    let uia_type = event.netui_control.as_ref().and_then(|c| c.control_type);
    let uia_button = uia_type == Some(UIA_BUTTON);
    let role = match object.role? {
        PAGETAB => Role::Tab,
        // A button that UIA calls a menu's item opens one: Word's Customize Toolbar, "Customize
        // Toolbar sub menu" [obs, 1051/s4].
        PUSHBUTTON if uia_type == Some(UIA_MENU_ITEM) => Role::MenuButton,
        PUSHBUTTON => Role::Button,
        // Office's "Sign in" beside the ribbon's tabs [obs, 1051/s10].
        LINK => Role::Link { href: None },
        CHECKBUTTON => Role::CheckBox,
        RADIOBUTTON => Role::RadioButton,
        COMBOBOX => Role::ComboBox,
        // A button that drops a list or a grid down opens it as a menu does: Word's Change
        // Case and Cover Page, each a "sub menu" to JAWS, where Paste, a split button, is one
        // [obs, 1045/s2, s11; 1043/s5]. One that UIA calls a button, not a menu's item, is a
        // button that opens a menu: Styles, "sub menu | Styles Button" [obs, 1051/s17].
        BUTTONDROPDOWN | BUTTONDROPDOWNGRID if uia_button => Role::Button,
        BUTTONDROPDOWN | BUTTONDROPDOWNGRID => Role::MenuButton,
        SPLITBUTTON => Role::SplitButton,
        _ => return None,
    };
    let tab = role == Role::Tab;
    let mut focus = okur_core::FocusEvent::new(role).with_name(name.to_owned());
    focus.opens_menu =
        uia_button && matches!(object.role, Some(BUTTONDROPDOWN | BUTTONDROPDOWNGRID));
    // Its value: a split button's "Paste (Ctrl+V)" [obs, 970/s4].
    focus.value = object.value.clone().filter(|v| !v.trim().is_empty());
    if tab {
        focus.states.selected = Some(object.states & SELECTED != 0);
        // Its place among the tabs shown: "Home Tab selected 1 of 2" (JAWS's own places lag a
        // key behind: "View 1 of 2", then "Home 2 of 2" [obs, 970/s2, s3]).
        let shown: Vec<&MsaaNode> = event
            .msaa_siblings
            .iter()
            .filter(|s| s.role == Some(PAGETAB) && s.states & MSAA_INVISIBLE == 0)
            .collect();
        focus.item_index = shown
            .iter()
            .position(|s| s.name.as_deref() == Some(name))
            .map(|i| i + 1);
        focus.item_count = focus.item_index.map(|_| shown.len());
    } else if event.msaa_ancestors.last().and_then(|a| a.role) == Some(TOOLBAR) {
        // A command in one of the ribbon's groups: its place among the group's commands shown,
        // "Copy 3 of 3" (JAWS's lags a key behind, "1 of 3" [obs, 970/s5]).
        let shown: Vec<&MsaaNode> = event
            .msaa_siblings
            .iter()
            .filter(|s| s.states & MSAA_INVISIBLE == 0)
            .collect();
        focus.item_index = shown
            .iter()
            .position(|s| s.name.as_deref() == Some(name) && s.role == object.role)
            .map(|i| i + 1);
        focus.item_count = focus.item_index.map(|_| shown.len());
    }
    apply_msaa_states(&mut focus, object.states);
    // Unavailable, as MSAA says: "Copy Button Unavailable" [obs, 970/s5].
    if object.states & MSAA_UNAVAILABLE != 0 {
        focus.states.disabled = true;
    }
    // A task pane's controls are said without the dock and pane around them: "Ignore All
    // Button" [obs, 1069/s2].
    if !in_task_pane(event) {
        focus.groups = ribbon_groups(event);
    }
    focus.app = event.process.clone();
    focus.app_product = event.process_product.clone();
    focus.app_version = event.process_version.clone();
    // Office's own UIA element: the control's rectangle, its ScreenTip's title, and its
    // toggle state, a toggle button's "not checked" [obs, 1045/s5, s9].
    let control = event.netui_control.as_ref();
    focus.bounds = control
        .and_then(|c| c.bounds)
        .or_else(|| event.focus_chain.last().and_then(|n| n.bounds));
    focus.help_text = control.and_then(|c| c.help_text.clone());
    if let Some(toggle) = control.and_then(|c| c.toggle_state) {
        focus.states.checked = Some(toggle == 1);
    }
    // An Office task pane's controls (the Spelling pane) are no ribbon's.
    focus.in_ribbon = !in_task_pane(event);
    // The ribbon's own name: its outermost property page's, "Ribbon".
    focus.ribbon_name = event
        .msaa_ancestors
        .iter()
        .find(|a| a.role == Some(0x26))
        .and_then(|a| a.name.clone());
    // On the ribbon's menus: a tab, or a command on a tab's page (a property page inside the
    // ribbon's own, the lower ribbon's "Home"), not Minimize the Ribbon or the Quick Access
    // Toolbar beside them.
    const PROPERTYPAGE: i32 = 0x26;
    let pages = event
        .msaa_ancestors
        .iter()
        .filter(|a| a.role == Some(PROPERTYPAGE))
        .count();
    focus.ribbon_menu = tab || pages >= 2;
    Some(focus)
}

/// The window class of Office's classic command bars, a toolbar of buttons inside some of its
/// dialogs (Insert Hyperlink's "Link to:").
pub const COMMAND_BAR_CLASS: &str = "MsoCommandBar";

/// A button of an Office command bar (`MsoCommandBar`), known only to MSAA: UI Automation's
/// focus stays on the dialog around it. Read from its MSAA object, held down where MSAA says
/// so, its description after: "Existing File or Web Page Button Pressed | Existing File or Web
/// Page button" [obs, 1076/s5]. The bar's own focus, a
/// moment before (an internal name, "MSO Generic Control Container"), is no focus.
pub(super) fn command_bar_focus(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
    const PUSHBUTTON: i32 = 0x2B;
    const PRESSED: u32 = 0x8;
    if event.window_class.as_deref() != Some(COMMAND_BAR_CLASS) {
        return None;
    }
    let object = event
        .object
        .as_ref()
        .filter(|o| o.role == Some(PUSHBUTTON))?;
    let name = object
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())?;
    let mut focus = okur_core::FocusEvent::new(okur_core::Role::Button).with_name(name.to_owned());
    focus.states.pressed = object.states & PRESSED != 0;
    focus.states.disabled = object.states & MSAA_UNAVAILABLE != 0;
    focus.description = event
        .msaa_description
        .clone()
        .filter(|d| !d.trim().is_empty());
    focus.access_key = object.shortcut.clone().filter(|k| !k.trim().is_empty());
    focus.window_title = event.top_window_title.clone();
    focus.app = event.process.clone();
    focus.app_product = event.process_product.clone();
    focus.app_version = event.process_version.clone();
    focus.platform.window_class = event.window_class.clone();
    focus.platform.top_window_class = event.top_window_class.clone();
    Some(focus)
}

/// Whether a NetUI focus is in one of Office's task panes (the Spelling pane, in a dock of
/// Office's window beside the document, `MsoDockRight`), not in its ribbon, docked at the top
/// (`MsoDockTop`) [obs, 1069].
pub(super) fn in_task_pane(event: &TraceEvent) -> bool {
    event.focus_chain.iter().any(|n| {
        n.class_name.as_deref() == Some("MsoCommandBarDock")
            && n.name.as_deref().is_some_and(|dock| dock != "MsoDockTop")
    })
}

/// A list's item in an Office task pane, placed among the list's items: the Spelling pane's
/// suggestions, "the | 1 of 5" [obs, 1069/s4].
pub(super) fn task_pane_item(
    event: &TraceEvent,
    object: &TraceObject,
    name: &str,
) -> okur_core::FocusEvent {
    const LISTITEM: i32 = 0x22;
    let mut focus =
        okur_core::FocusEvent::new(okur_core::Role::ListItem).with_name(name.to_owned());
    let items: Vec<&MsaaNode> = event
        .msaa_siblings
        .iter()
        .filter(|s| s.role == Some(LISTITEM))
        .collect();
    focus.item_index = items
        .iter()
        .position(|s| s.name.as_deref() == Some(name))
        .map(|i| i + 1);
    focus.item_count = focus.item_index.map(|_| items.len());
    apply_msaa_states(&mut focus, object.states);
    focus.app = event.process.clone();
    focus.app_product = event.process_product.clone();
    focus.app_version = event.process_version.clone();
    focus.bounds = event
        .netui_control
        .as_ref()
        .and_then(|c| c.bounds)
        .or_else(|| event.focus_chain.last().and_then(|n| n.bounds));
    focus
}

/// A ribbon gallery's item (Word's Styles gallery): named by its description, the name without
/// the preview mark before it ("¶ Heading 2", "Heading 2"), placed among the gallery's items,
/// "2 of 17" [obs, 1064/s4-s7]. The gallery is the lower ribbon's, on the ribbon's menus.
pub(super) fn gallery_item(
    event: &TraceEvent,
    object: &TraceObject,
    name: &str,
) -> okur_core::FocusEvent {
    const LISTITEM: i32 = 0x22;
    let plain = event
        .msaa_description
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty() && name.ends_with(d))
        .unwrap_or(name);
    let mut focus =
        okur_core::FocusEvent::new(okur_core::Role::ListItem).with_name(plain.to_owned());
    let items: Vec<&MsaaNode> = event
        .msaa_siblings
        .iter()
        .filter(|s| s.role == Some(LISTITEM))
        .collect();
    focus.item_index = items
        .iter()
        .position(|s| s.name.as_deref() == Some(name))
        .map(|i| i + 1);
    focus.item_count = focus.item_index.map(|_| items.len());
    apply_msaa_states(&mut focus, object.states);
    focus.container_role = Some(okur_core::Role::List);
    focus.container_name = event.msaa_ancestors.first().and_then(|a| a.name.clone());
    focus.app = event.process.clone();
    focus.app_product = event.process_product.clone();
    focus.app_version = event.process_version.clone();
    let control = event.netui_control.as_ref();
    focus.bounds = control
        .and_then(|c| c.bounds)
        .or_else(|| event.focus_chain.last().and_then(|n| n.bounds));
    focus.help_text = control.and_then(|c| c.help_text.clone());
    focus.in_ribbon = true;
    focus.ribbon_menu = true;
    focus
}

/// The ribbon's own property page with the focus and no tab focused in it (`ribbon_tab` finds
/// none): a tab panel named as the ribbon, "Ribbon", which the focus passes on its way between
/// two of the ribbon's commands; the menus' own, so that passing it leaves them not.
pub(super) fn ribbon_page(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
    const PROPERTYPAGE: i32 = 0x26;
    let object = event.object.as_ref()?;
    if object.role != Some(PROPERTYPAGE) || !event.msaa_ancestors.is_empty() {
        return None;
    }
    let name = object
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())?;
    let mut focus = okur_core::FocusEvent::new(okur_core::Role::TabPanel).with_name(name);
    focus.app = event.process.clone();
    focus.app_product = event.process_product.clone();
    focus.app_version = event.process_version.clone();
    focus.groups = ribbon_panes(event).collect();
    focus.in_ribbon = true;
    focus.ribbon_menu = true;
    focus.ribbon_name = Some(name.to_owned());
    Some(focus)
}

/// MSAA's invisible state: a ribbon's hidden tab ("Print preview").
pub(super) const MSAA_INVISIBLE: u32 = 0x8000;
/// MSAA's unavailable state.
pub(super) const MSAA_UNAVAILABLE: u32 = 0x1;
/// MSAA's editable text role.
pub(super) const MSAA_TEXT: i32 = 0x2A;
/// MSAA's spin button role: a RichEdit with arrows that step its value.
pub(super) const MSAA_SPIN_BUTTON: i32 = 0x34;
/// MSAA's tool bar role: a ribbon's group of commands.
pub(super) const TOOLBAR: i32 = 0x16;

/// The ribbon's panes, as UI Automation names them (the window itself is not one of them).
pub(super) fn ribbon_panes(event: &TraceEvent) -> impl Iterator<Item = okur_core::GroupEntry> + '_ {
    const PANE: i32 = 50033;
    event
        .focus_chain
        .iter()
        .skip(1)
        .filter(|n| n.control_type == PANE)
        .filter_map(|n| n.name.as_deref().map(str::trim).filter(|n| !n.is_empty()))
        .map(okur_core::GroupEntry::pane)
}

/// The groups a ribbon's control is in: the ribbon's panes, then its MSAA ancestors inside
/// the ribbon's own page, each named one: the row of tabs, the lower ribbon, a tab's page with
/// its value, a tool bar, a grouping with its description: "Lower Ribbon | Home | Ribbon tab |
/// Clipboard Tool bar | Paste Click here for more options such as pasting only plain text."
/// [obs, 970/s4]. Without its ancestors (older traces), none.
pub(super) fn ribbon_groups(event: &TraceEvent) -> Vec<okur_core::GroupEntry> {
    use okur_core::GroupEntry;
    const PAGETABLIST: i32 = 0x3C;
    if event.msaa_ancestors.is_empty() {
        return Vec::new();
    }
    let inner = event.msaa_ancestors.iter().skip(1).filter_map(|a| {
        let name = a.name.as_deref().map(str::trim).filter(|n| !n.is_empty())?;
        Some(match a.role {
            Some(PAGETABLIST) => GroupEntry::tab_list(name),
            Some(TOOLBAR) => GroupEntry::tool_bar(name),
            _ => {
                let value = a.value.as_deref().map(str::trim).filter(|v| *v != name);
                let detail = a.description.as_deref().or(value).map(str::to_owned);
                GroupEntry {
                    disabled: a.states & MSAA_UNAVAILABLE != 0,
                    ..GroupEntry::group(name).with_detail(detail)
                }
            }
        })
    });
    ribbon_panes(event).chain(inner).collect()
}

/// The tab a ribbon holds focused when the ribbon itself takes the focus (Alt, F10), from the
/// ribbon's MSAA objects: "UIRibbonDockTop | Ribbon | Ribbon | Ribbon | Ribbon tabs Tab | Home
/// Tab selected | 1 of 2" [obs, 970/s1, 972/s1], the ribbon's named panes and its row of tabs
/// as the groups entered.
pub(super) fn ribbon_tab(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
    use okur_core::{GroupEntry, Role};
    const PROPERTYPAGE: i32 = 0x26;
    const PAGETABLIST: i32 = 0x3C;
    const PAGETAB: i32 = 0x25;
    const FOCUSED: u32 = 0x4;
    const SELECTED: u32 = 0x2;
    fn tab_list(node: &MsaaNode) -> Option<&MsaaNode> {
        if node.role == Some(PAGETABLIST) {
            return Some(node);
        }
        node.children.iter().find_map(tab_list)
    }
    fn tabs<'a>(node: &'a MsaaNode, out: &mut Vec<&'a MsaaNode>) {
        for child in &node.children {
            if child.role == Some(PAGETAB) {
                out.push(child);
            } else {
                tabs(child, out);
            }
        }
    }
    if event.object.as_ref()?.role != Some(PROPERTYPAGE) {
        return None;
    }
    let list = tab_list(event.msaa_tree.as_ref()?)?;
    let mut all = Vec::new();
    tabs(list, &mut all);
    let shown: Vec<&MsaaNode> = all
        .into_iter()
        .filter(|t| t.states & MSAA_INVISIBLE == 0)
        .collect();
    let index = shown.iter().position(|t| t.states & FOCUSED != 0)?;
    let tab = shown.get(index)?;
    let mut focus = okur_core::FocusEvent::new(Role::Tab).with_name(tab.name.clone()?);
    focus.states.selected = Some(tab.states & SELECTED != 0);
    focus.item_index = Some(index + 1);
    focus.item_count = Some(shown.len());
    // The ribbon's panes, then its row of tabs.
    focus.groups = ribbon_panes(event)
        .chain(list.name.clone().map(GroupEntry::tab_list))
        .collect();
    focus.app = event.process.clone();
    focus.app_product = event.process_product.clone();
    focus.app_version = event.process_version.clone();
    focus.bounds = event.focus_chain.last().and_then(|n| n.bounds);
    focus.in_ribbon = true;
    // The ribbon's own name: the property page that took the focus, "Ribbon".
    focus.ribbon_name = event.object.as_ref().and_then(|o| o.name.clone());
    focus.ribbon_menu = true;
    Some(focus)
}
