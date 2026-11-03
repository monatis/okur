//! Win32's common controls read through MSAA: an address band's toolbar, a toolbar's
//! buttons, a list view's column titles, and MSAA's states on a control.

use super::*;

/// A common file dialog's folder: its address bar's toolbar (`ToolbarWindow32`, control id
/// 1001, in the address band) is named for it, "Address: C:\Users\…" [obs, 524/s3].
pub(super) fn address_band(node: &UiaNode) -> Option<String> {
    let named = node
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty());
    if node.class_name.as_deref() == Some("ToolbarWindow32")
        && node.automation_id.as_deref() == Some("1001")
    {
        if let Some(name) = named {
            return Some(name.to_owned());
        }
    }
    node.children.iter().find_map(address_band)
}

/// A Win32 toolbar's button with the focus: UI Automation's focus stays on the toolbar
/// (`ToolbarWindow32`), and the MSAA event's object is the button, a child of it. The focus is
/// the button, in the toolbar: "Tool bar | Hide Folders Button | To activate press spacebar."
/// [obs, 524/s6]. The toolbar is said by its role alone, its window's name unsaid: Programs
/// and Features' "Tool bar Up to "All Control Panel Items" (Alt + Up Arrow) Button", in its
/// "Up band toolbar" [obs, 3156/s3].
pub(super) fn toolbar_button(
    event: &TraceEvent,
    toolbar: &UiaNode,
    focus: &mut okur_core::FocusEvent,
) {
    use okur_core::Role;
    const PUSHBUTTON: i32 = 0x2B;
    const CHECKBUTTON: i32 = 0x2C;
    const BUTTONDROPDOWN: i32 = 0x38;
    // A drop-down that opens a grid: Word's "Cover Page", a sub menu to JAWS [obs, 1043/s5].
    const BUTTONDROPDOWNGRID: i32 = 0x3A;
    const SPLITBUTTON: i32 = 0x3E;
    if toolbar.control_type != UIA_TOOLBAR || event.id_child <= 0 {
        return;
    }
    let Some(object) = event.object.as_ref() else {
        return;
    };
    let Some(name) = object
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    else {
        return;
    };
    let role = match object.role {
        Some(PUSHBUTTON) => Role::Button,
        Some(CHECKBUTTON) => Role::CheckBox,
        Some(BUTTONDROPDOWN | BUTTONDROPDOWNGRID | SPLITBUTTON) => Role::SplitButton,
        _ => return,
    };
    focus.container_role = Some(Role::ToolBar);
    focus.container_name = None;
    focus.platform.container_class = toolbar.class_name.clone();
    focus.role = role;
    focus.name = Some(name.to_owned());
    focus.value = None;
    focus.states = okur_core::aom::NodeStates::default();
    apply_msaa_states(focus, object.states);
    focus.item_index = None;
    focus.item_count = None;
}

/// A Win32 header control's item with the focus (`SysHeader32`: a list view's column title):
/// JAWS's header bar, at its place among the titles, without the list around it: "Name Header
/// bar | 1 of 5 | Select column title to sort by that category." [obs, 3156/s2].
pub(super) fn header_bar(event: &TraceEvent, focused: &UiaNode, focus: &mut okur_core::FocusEvent) {
    let in_header = event.focus_chain.iter().rev().nth(1).is_some_and(|parent| {
        parent.control_type == UIA_HEADER && parent.class_name.as_deref() == Some("SysHeader32")
    });
    if !in_header {
        return;
    }
    focus.header_bar = true;
    focus.container_role = None;
    focus.container_name = None;
    focus.container_is_list_view = false;
    let titles: Vec<&UiaNode> = event
        .focus_siblings
        .iter()
        .filter(|s| s.control_type == focused.control_type)
        .collect();
    let index = titles
        .iter()
        .position(|s| s.has_keyboard_focus)
        .or_else(|| titles.iter().position(|s| s.name == focused.name));
    if let Some(index) = index {
        focus.item_index = Some(index + 1);
        focus.item_count = Some(titles.len());
    }
}

/// Updates the states a change can carry from MSAA state bits (`STATE_SYSTEM_*`).
pub(super) fn apply_msaa_states(control: &mut okur_core::FocusEvent, states: u32) {
    const SELECTED: u32 = 0x2;
    const CHECKED: u32 = 0x10;
    const EXPANDED: u32 = 0x200;
    const COLLAPSED: u32 = 0x400;
    if matches!(
        control.role,
        okur_core::Role::CheckBox | okur_core::Role::RadioButton
    ) {
        control.states.checked = Some(states & CHECKED != 0);
    }
    // A grid's cell says whether it is selected: a WinForms DataGridView's cells report it
    // through MSAA only ("Item Row 0 | selected | Pens" [obs, 545/s1]).
    if control.role == okur_core::Role::TableCell {
        control.states.selected = Some(states & SELECTED != 0);
    }
    if states & EXPANDED != 0 {
        control.states.expanded = Some(true);
    } else if states & COLLAPSED != 0 {
        control.states.expanded = Some(false);
    }
}
