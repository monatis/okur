//! Accessibility event traces.
//!
//! `okur trace` records the system-wide accessibility event stream while a JAWS trial runs
//! on the reference machine: one JSON object per line, each event carrying the MSAA view of
//! its object and, for focus and foreground changes, a UIA snapshot of the focused element
//! and its ancestors. Steps in the evidence carry start/end timestamps, so a step's events
//! can be replayed into the engine hermetically and its speech compared with JAWS's: the
//! verification path for focus-driven behavior (Tab order, forms mode, dialogs, menus,
//! desktop apps) that static document snapshots cannot reach.

use crate::uia_snapshot::UiaNode;
use serde::{Deserialize, Serialize};

/// UIA control type ids used to read focus context.
const UIA_LISTITEM: i32 = 50007;
const UIA_WINDOW: i32 = 50032;
const UIA_DOCUMENT: i32 = 50030;
/// Window classes that host web page content: Chromium (Chrome, Edge, Electron) and Firefox.
const WEB_CONTENT_WINDOW_CLASSES: [&str; 2] = ["Chrome_RenderWidgetHostHWND", "MozillaWindowClass"];
/// The top-level window class of Chromium browsers and Electron apps.
const CHROMIUM_TOP_WINDOW: &str = "Chrome_WidgetWin_1";
/// The window of the Windows 10 taskbar's news and weather widget (its title is the weather).
const NEWS_AND_WEATHER_CLASS: &str = "DynamicContent2";
const UIA_TREEITEM: i32 = 50024;
const UIA_TREE: i32 = 50023;
const UIA_LIST: i32 = 50008;
const UIA_TEXT: i32 = 50020;
const UIA_BUTTON: i32 = 50000;
const UIA_EDIT: i32 = 50004;
const UIA_COMBOBOX: i32 = 50003;
const UIA_MENUITEM: i32 = 50011;
/// The window class of Win32 dialog boxes (message boxes, common dialogs).
const WIN32_DIALOG_CLASS: &str = "#32770";
/// A Win32 popup menu's window.
const WIN32_MENU_CLASS: &str = "#32768";
/// The window kind of focus on the desktop ([`okur_core::FocusEvent::window_kind`]).
pub const DESKTOP_KIND: &str = "Progman";
/// MSAA `ROLE_SYSTEM_MENUITEM`.
const MSAA_MENUITEM: i32 = 0x0C;
/// MSAA `ROLE_SYSTEM_BUTTONMENU`.
const MSAA_BUTTONMENU: i32 = 0x39;

/// Bumped whenever the recorded fields or their meaning change.
pub const TRACE_FORMAT_VERSION: u32 = 7;

/// Foreground windows while focus moves only through UIA: no WinEvent reports it. The task
/// switcher (Alt+Tab) focuses each window's item as Tab is pressed with Alt held (the same
/// items NVDA's explorer module handles: `MultitaskingViewFrame` on Windows 10,
/// `XamlExplorerHostIslandWindow` on Windows 11, which also hosts Task View). On Windows 10
/// the switcher itself is never foreground: the shell's invisible `ForegroundStaging` window
/// is, and no WinEvent names the switcher, though JAWS speaks it [obs, 306/s7 trial 1:
/// "Task Switching" with only ForegroundStaging events traced].
pub const UIA_FOCUS_WINDOWS: [&str; 3] = [
    "MultitaskingViewFrame",
    "XamlExplorerHostIslandWindow",
    "ForegroundStaging",
];

/// Processes in front whose list selections are read while focus stays in a field: Windows
/// 11 search selects a result as the search box's text changes, and moves the selection on
/// Down, with UIA ElementSelected events only (probed on the dev box).
/// Windows Settings' search box does the same with its suggestions; Settings is in front as
/// its frame's process (JAWS: "Display settings | 1 of 25", dev box 2026-10-16).
pub const UIA_SELECTION_PROCESSES: [&str; 2] = ["SearchHost.exe", "ApplicationFrameHost.exe"];

/// Whether a recorded event is a focus change, from a WinEvent or from UIA.
pub fn is_focus(event: &TraceEvent) -> bool {
    matches!(event.event.as_str(), "focus" | "uia_focus")
}

/// Whether a value change is a choice's, not text's: its object has a value (a select's new
/// option, "Eastgate") while the focused element has no text. A text field's value is its
/// text.
fn is_choice_without_text(event: &TraceEvent) -> bool {
    let value = event
        .object
        .as_ref()
        .and_then(|o| o.value.as_deref())
        .is_some_and(|v| !v.trim().is_empty());
    value
        && event
            .text_context
            .as_ref()
            .is_none_or(|t| t.text.trim().is_empty())
}

/// The MSAA view of an event's object (`AccessibleObjectFromEvent`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TraceObject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub states: u32,
}

/// One recorded accessibility event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Unix time in milliseconds when the event was received.
    pub t_ms: u128,
    /// Short event name (`focus`, `foreground`, `name_change`, …). `uia_focus` is a UIA
    /// focus change in a window whose focus no WinEvent reports ([`UIA_FOCUS_WINDOWS`]).
    pub event: String,
    /// Raw `EVENT_*` id; for `uia_focus`, the UIA event id.
    pub win_event: u32,
    pub hwnd: isize,
    pub id_object: i32,
    pub id_child: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_class: Option<String>,
    /// For a focus: the style (`GWL_STYLE`, raw) and class of the window with the keyboard
    /// focus. An edit's ES_MULTILINE says it has several lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_style: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_window_class: Option<String>,
    /// For a focus: its top-level window's menu bar items, as written ("&File"), and its
    /// status bar's parts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub menu_bar: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_bar: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<TraceObject>,
    /// For focus and foreground events: the focused element and its ancestors, outermost
    /// first, read through UIA with the snapshot property set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus_chain: Vec<UiaNode>,
    /// For focus events: the focused element's control-view siblings, itself included,
    /// without their children.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus_siblings: Vec<UiaNode>,
    /// For focus events: the focused element with its control-view subtree to depth 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_subtree: Option<UiaNode>,
    /// For focus on a list view's item: its column headers in order ("Name", "Size"), which
    /// JAWS says with a column (Ctrl+Insert+1) [obs, 514/s5].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_headers: Vec<String>,
    /// For focus on a Win32 combo box or list box: the selection as `[index, count]`,
    /// with a 0-based index, asked of the control itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_position: Option<[i32; 2]>,
    /// For focus, caret and change events: the focused text field's text, caret and
    /// selection (`text_context::read_focused`), from which edit keys are spoken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_context: Option<okur_core::TextContext>,
    /// For a live region change in a Chromium page: the region with its subtree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<crate::ia2_snapshot::Ia2Node>,
    /// For a live region change in a Chromium page: the page after the change, from which
    /// the virtual buffer is refreshed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<crate::ia2_snapshot::Ia2Node>,
    /// For focus and foreground events: the caption of the window in front when the event was
    /// read, as JAWS names the window (Chrome's names the profile; the desktop's list view has
    /// no UIA window around it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_window_title: Option<String>,
    /// The class of that window (`XamlExplorerHostIslandWindow` while the task switcher is up).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_window_class: Option<String>,
    /// When the top-level window is the desktop (`Progman`, `WorkerW`): the current virtual
    /// desktop's name, which JAWS gives as the desktop's title ("Desktop 1").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virtual_desktop: Option<String>,
    /// For focus in a dialog box (`#32770`) in front: its text by screen rows
    /// ([`crate::screen_lines`]), for the Virtual Viewer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dialog_lines: Vec<String>,
    /// For focus in that dialog box: its static text that labels no field, top to bottom.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dialog_text: Vec<String>,
    /// For focus on a property sheet's page in that dialog box: the page's name ("General").
    /// UIA leaves the page out of the focus chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog_page: Option<String>,
    /// For focus in that dialog box: the group box around the focused control, by its text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_box: Option<String>,
    /// For focus in that dialog box: its default button's text, without its ampersand
    /// (DM_GETDEFID), which Insert+E says [obs, 540/s2].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_button: Option<String>,
    /// For focus in that dialog box: its control tree, without the title bar (Say Window).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog_tree: Option<UiaNode>,
    /// For a UIA notification from the application in front: what it says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notification: Option<TraceNotification>,
}

/// A UIA notification (`UiaRaiseNotificationEvent`): Windows Settings' search box says
/// "Suggestions available" as it is typed in (JAWS reads it; dev box, 2026-10-16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceNotification {
    pub text: String,
    /// `NotificationProcessing`: 0 and 1 are important (they interrupt), 2 to 4 are not.
    pub processing: i32,
}

/// A dialog's tree without the items inside its tree views' items: a file dialog's navigation
/// pane lists the account's pinned and recent folders under Quick access (525, round 35), which
/// Say Window does not read (JAWS names the tree's item and its count) and a public corpus must
/// not hold. The tree's own items stay ("Quick access", "This PC").
pub(crate) fn without_nested_tree_items(mut tree: UiaNode) -> UiaNode {
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
fn window_items(tree: &UiaNode) -> Vec<okur_core::WindowItem> {
    fn walk(node: &UiaNode, items: &mut Vec<okur_core::WindowItem>) {
        for child in &node.children {
            let mut leaf = child.clone();
            leaf.children.clear();
            let aom = crate::uia_snapshot::to_aom(&leaf);
            let name = child.name.clone().unwrap_or_default();
            if !name.trim().is_empty() || aom.value.is_some() {
                items.push(okur_core::WindowItem {
                    role: aom.role.clone(),
                    name: okur_core::viewer_text::without_marks(&name),
                    value: aom
                        .value
                        .as_deref()
                        .map(okur_core::viewer_text::without_marks)
                        .filter(|v| !v.is_empty()),
                    expanded: aom.states.expanded,
                    checked: aom.states.checked,
                    read_only: child.value_is_read_only == Some(true),
                    group: child.group_box.clone(),
                });
            }
            walk(child, items);
        }
    }
    let mut items = Vec::new();
    walk(tree, &mut items);
    items
}

/// Whether a window is an edit of several lines: an edit window class (Win32's Edit, WinForms'
/// `WindowsForms10.EDIT…`, RichEdit) with ES_MULTILINE in its style. The bit means something
/// else for other classes.
/// Whether a window is a plain Win32 edit (Win32's Edit, WinForms' `WindowsForms10.EDIT…`), not
/// RichEdit.
fn plain_edit(class: Option<&str>) -> bool {
    class.is_some_and(|c| {
        let c = c.to_ascii_uppercase();
        c == "EDIT" || c.contains(".EDIT.")
    })
}

fn multi_line_edit(class: Option<&str>, style: Option<u32>) -> bool {
    const ES_MULTILINE: u32 = 0x0004;
    let edit = class.is_some_and(|c| {
        let c = c.to_ascii_uppercase();
        c == "EDIT" || c.contains(".EDIT.") || c.starts_with("RICHEDIT")
    });
    edit && style.is_some_and(|s| s & ES_MULTILINE != 0)
}

/// The first line of a trace file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceHeader {
    pub trace_format_version: u32,
    pub tool_version: String,
    pub host: String,
    pub started_unix_ms: u128,
}

/// Reads a trace file: the header line, then one event per line (malformed lines skipped).
pub fn read_trace(
    path: impl AsRef<std::path::Path>,
) -> Result<(TraceHeader, Vec<TraceEvent>), String> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
    let mut lines = text.lines();
    let header: TraceHeader = lines
        .next()
        .ok_or_else(|| format!("{path:?}: empty trace"))
        .and_then(|l| {
            serde_json::from_str(l.trim_start_matches('\u{feff}'))
                .map_err(|e| format!("{path:?}: header: {e}"))
        })?;
    let events = lines.filter_map(|l| serde_json::from_str(l).ok()).collect();
    Ok((header, events))
}

/// Name of a recorded `EVENT_*` id, or `None` for events the tracer ignores.
pub fn event_name(win_event: u32, id_object: i32) -> Option<&'static str> {
    const OBJID_CARET: i32 = -8;
    // A window's scroll bars (OBJID_VSCROLL, OBJID_HSCROLL): nothing about them is said. A
    // combo box's list scrolling sent 264 state changes as the Run dialog opened, and reading
    // each held the focus behind them for 5 s (old-pc, 319).
    const OBJID_VSCROLL: i32 = -5;
    const OBJID_HSCROLL: i32 = -6;
    if matches!(id_object, OBJID_VSCROLL | OBJID_HSCROLL) {
        return None;
    }
    Some(match win_event {
        0x0002 => "alert",
        0x0003 => "foreground",
        0x0004 => "menu_start",
        0x0005 => "menu_end",
        0x0006 => "menu_popup_start",
        0x0007 => "menu_popup_end",
        0x0010 => "dialog_start",
        0x0011 => "dialog_end",
        0x8005 => "focus",
        0x8006 => "selection",
        0x8007 => "selection_add",
        0x8008 => "selection_remove",
        0x8009 => "selection_within",
        0x800A => "state_change",
        0x800B if id_object == OBJID_CARET => "caret",
        0x800C => "name_change",
        0x800D => "description_change",
        0x800E => "value_change",
        0x8014 => "text_selection_change",
        0x8019 => "live_region_change",
        _ => return None,
    })
}

/// Builds the engine's focus event from a focus chain (outermost first), using the same
/// UIA-to-AOM mapping as document snapshots. Used by trace replay, and by the live listener,
/// so both exercise one conversion.
/// Whether the focus is in a dialog box (the innermost window of its chain is a Win32
/// `#32770`), whose static text and buttons are read from the focus's siblings.
pub fn in_dialog_box(chain: &[UiaNode]) -> bool {
    chain
        .iter()
        .rev()
        .find(|w| w.control_type == UIA_WINDOW)
        .is_some_and(|w| w.class_name.as_deref() == Some(WIN32_DIALOG_CLASS))
}

/// Whether the focused element reports its own position ("2 of 5"), so that its siblings
/// need not be counted.
pub fn reports_position(chain: &[UiaNode]) -> bool {
    chain
        .last()
        .is_some_and(|f| f.position_in_set.is_some() && f.size_of_set.is_some())
}

/// Win32 list-view controls (`SysListView32`, also as WinForms `WindowsForms10.SysListView32...`):
/// JAWS calls their lists "List view".
fn is_list_view(class: Option<&str>) -> bool {
    class.is_some_and(|c| c.contains("SysListView32"))
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
    }
    event.help_text = focused
        .full_description
        .clone()
        .or_else(|| focused.help_text.clone());
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
    // Groups that are a control's content name nothing: not even as the nearest container.
    let (groups, inside_control) = nested_groups(chain);
    let control_content = !event.in_document && inside_control;
    if !event.in_document {
        event.groups = groups;
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
            event.container_name = a.name.clone();
            event.platform.container_class = a.class_name.clone();
            event.container_is_list_view = is_list_view(a.class_name.as_deref());
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
fn nested_groups(chain: &[UiaNode]) -> (Vec<okur_core::GroupEntry>, bool) {
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
            UIA_GROUP | UIA_TEXT => {
                entries.extend(named(ancestor).map(okur_core::GroupEntry::group))
            }
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
        return Some(
            okur_core::FocusEvent::new(okur_core::Role::MenuItem)
                .with_name(object.name.clone().unwrap_or_default()),
        );
    }
    if let Some(focus) = netui_focus(event) {
        return Some(focus);
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
    let mut focus = focus_event_from_chain(
        &event.focus_chain,
        event.window_class.as_deref(),
        top_window,
    )?;
    let focused = event.focus_chain.last()?;
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
    focus.in_dialog_box = focus.platform.top_window_class.as_deref() == Some(WIN32_DIALOG_CLASS);
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
    focus.menu_bar = event
        .menu_bar
        .iter()
        .map(|item| crate::screen_lines::without_accelerators(item))
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
    if active_window.and_then(|w| w.class_name.as_deref()) == Some(WIN32_DIALOG_CLASS) {
        // On a property sheet's page, the text beside the focus is the page's labels ("Type:",
        // "Local Disk"): JAWS 2025 opens Local Disk (C:) Properties naming the page instead,
        // "General page" (dev box, 2026-10-15).
        focus.dialog_page = event.dialog_page.clone();
        focus.group_box = event.group_box.clone();
        focus.default_button = event.default_button.clone();
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
        fn control_names<'a>(node: &'a UiaNode, names: &mut Vec<&'a str>) {
            for child in &node.children {
                if child.control_type != UIA_TEXT {
                    names.extend(child.name.as_deref());
                }
                control_names(child, names);
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
            .filter_map(|s| s.name.as_deref())
            .collect();
        if let Some(tree) = &event.dialog_tree {
            control_names(tree, &mut labels);
        }
        let text: Vec<&str> = event
            .focus_siblings
            .iter()
            .filter(|_| event.dialog_page.is_none())
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
    focus.window_items = event
        .dialog_tree
        .as_ref()
        .map(window_items)
        .unwrap_or_default();
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
        // A list-view item's further columns are its text children after the name.
        if focus.role == okur_core::Role::ListItem {
            focus.columns = subtree
                .children
                .iter()
                .filter_map(|c| c.name.clone())
                .filter(|n| !n.trim().is_empty() && Some(n) != focused.name.as_ref())
                .collect();
            focus.column_headers = event.column_headers.clone();
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
    if focus.item_index.is_none()
        && focus.role != okur_core::Role::ComboBox
        && focus.role != okur_core::Role::Button
        && focus.role != okur_core::Role::Tab
        && focus.role != okur_core::Role::SplitButton
        && focus.role != okur_core::Role::MenuItem
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
    Some(focus)
}

#[cfg(windows)]
pub use capture::{
    run_live, run_trace, PageChange, IA2_EVENT_TEXT_INSERTED, IA2_EVENT_TEXT_REMOVED,
};

/// The menu mode transition a recorded event reports, if any.
pub fn menu_event_from_trace(event: &TraceEvent) -> Option<okur_core::MenuEvent> {
    use okur_core::MenuEvent;
    match event.event.as_str() {
        "menu_start" => Some(MenuEvent::Start),
        "menu_end" => Some(MenuEvent::End),
        "menu_popup_start" => Some(MenuEvent::PopupStart),
        "menu_popup_end" => Some(MenuEvent::PopupEnd),
        _ => None,
    }
}

/// Converts captured events (live or recorded) into engine events. The live listener and
/// parity replay share it, so both feed the engine the same sequence.
///
/// It remembers the focused object: a state, value or selection change is passed on only when
/// it concerns that object (a check box toggled with Space, a tree item expanded, a combo
/// box value changed), as JAWS speaks changes of the focused control and not of others.
#[derive(Debug, Default)]
pub struct EventConverter {
    /// The MSAA identity (`hwnd`, `id_object`, `id_child`) and name of the last focus.
    focused: Option<(isize, i32, i32, Option<String>)>,
    /// The last focus as the engine received it.
    last_focus: Option<okur_core::FocusEvent>,
    /// The window last brought to the front, whose renaming the engine is told of.
    foreground: Option<isize>,
    /// The focused control's own name, from the last focus event's focus chain.
    focused_control: Option<String>,
    /// The last UIA notification and when it came: the same text again within
    /// [`NOTIFICATION_REPEAT_MS`] is said once (Settings' search box says "Suggestions
    /// available" for every letter typed; JAWS said it once).
    last_notification: Option<(String, u128)>,
    /// The last native live region text and when it came, said once within
    /// [`NOTIFICATION_REPEAT_MS`] as notifications are.
    last_live_region: Option<(String, u128)>,
}

/// A window's own name changed (its title), not a control's in it.
pub fn is_window_renamed(event: &TraceEvent) -> bool {
    event.event == "name_change" && event.id_object == OBJID_WINDOW
}

/// `OBJID_WINDOW`: an event about the window itself.
const OBJID_WINDOW: i32 = 0;

/// File Explorer's top-level window class.
const EXPLORER_WINDOW_CLASS: &str = "CabinetWClass";
/// The Windows Shell property of an item's kind, the AutomationId of its cell in a shell view.
const SHELL_ITEM_TYPE: &str = "System.ItemTypeText";
/// The Windows Shell property of an item's size ("1 KB"), likewise.
const SHELL_ITEM_SIZE: &str = "System.Size";

/// How long the same UIA notification again is not said, in milliseconds.
const NOTIFICATION_REPEAT_MS: u128 = 2000;

impl EventConverter {
    /// The engine events for one captured event, in order (usually one, or none).
    pub fn convert(&mut self, event: &TraceEvent) -> Vec<okur_core::AccessibilityEvent> {
        if event.event == "live_region_change" {
            return live_region_events(event);
        }
        if event.event == "uia_live_region" {
            let Some(live) = event.focus_subtree.as_ref().and_then(uia_live_region_event) else {
                return Vec::new();
            };
            let repeat = self.last_live_region.as_ref().is_some_and(|(text, at)| {
                *text == live.text && event.t_ms.saturating_sub(*at) < NOTIFICATION_REPEAT_MS
            });
            if repeat {
                return Vec::new();
            }
            self.last_live_region = Some((live.text.clone(), event.t_ms));
            return vec![okur_core::AccessibilityEvent::LiveRegion(live)];
        }
        if let Some(notification) = &event.notification {
            let repeat = self.last_notification.as_ref().is_some_and(|(text, at)| {
                *text == notification.text
                    && event.t_ms.saturating_sub(*at) < NOTIFICATION_REPEAT_MS
            });
            if repeat {
                return Vec::new();
            }
            self.last_notification = Some((notification.text.clone(), event.t_ms));
            return vec![notification_event(notification)];
        }
        // A window's own menu bar reports the focus as its menus end (MSAA's menu bar object,
        // OBJID_MENU): the focus is back in the window. When UI Automation already gives the
        // focus elsewhere (the dialog a menu item opened), that came after: JAWS says both,
        // "Leaving menus | edit | Type in text. | Page Setup dialog | Paper Size: ..."
        // [obs, 541/s2, 544/s2].
        // A menu item's focus that came after its menu closed (an item chosen): the popup
        // window has no item left to read, and UI Automation's focus is the menu bar. It is
        // the menus ending, said by the events after it [obs, 541/s2, 544/s2].
        if event.event == "focus"
            && event.window_class.as_deref() == Some(WIN32_MENU_CLASS)
            && event.object.is_none()
        {
            return Vec::new();
        }
        if let Some(window) = menu_bar_return(event) {
            let mut events = vec![okur_core::AccessibilityEvent::Focus(window.clone())];
            if let Some(okur_core::AccessibilityEvent::Focus(next)) = self.convert_one(event) {
                if next.window_title != window.window_title {
                    events.push(okur_core::AccessibilityEvent::Focus(next));
                }
            }
            return events;
        }
        self.convert_one(event).into_iter().collect()
    }

    fn convert_one(&mut self, event: &TraceEvent) -> Option<okur_core::AccessibilityEvent> {
        use okur_core::{AccessibilityEvent, ChangeKind};
        if let Some(menu) = menu_event_from_trace(event) {
            return Some(AccessibilityEvent::Menu(menu));
        }
        if event.event == "foreground" {
            self.foreground = Some(event.hwnd);
        }
        if is_window_renamed(event) && self.foreground == Some(event.hwnd) {
            return event
                .window_title
                .clone()
                .map(AccessibilityEvent::WindowRenamed);
        }
        let kind = match event.event.as_str() {
            // A list item selected while the focus stays in a field (Windows search).
            "uia_selected" => {
                return focus_event_from_trace(event).map(AccessibilityEvent::Selected);
            }
            "focus" if is_drop_down_item(event) => {
                // Arrow keys in an open combo box move focus into its drop-down list; JAWS
                // speaks that as the combo box's new value [obs, 304/s12].
                let mut combo = self
                    .last_focus
                    .clone()
                    .filter(|f| f.role == okur_core::Role::ComboBox)?;
                combo.value = event.object.as_ref().and_then(|o| o.name.clone());
                combo.item_index = usize::try_from(event.id_child).ok().filter(|&i| i > 0);
                self.last_focus = Some(combo.clone());
                return Some(AccessibilityEvent::Change {
                    kind: ChangeKind::Value,
                    control: combo,
                });
            }
            "focus" | "uia_focus" => {
                let mut focus = focus_event_from_trace(event)?;
                // UIA repeats a focus change some WinEvent may also have reported.
                if event.event == "uia_focus" && self.last_focus.as_ref() == Some(&focus) {
                    return None;
                }
                // The event's MSAA object can be a window around the focused control: File
                // Explorer's "Explorer Pane" for its view buttons, whose states are not the
                // checked "Details" button's (dev box, 2026-10-17). Only the control's count.
                if let Some(object) = event
                    .object
                    .as_ref()
                    .filter(|o| o.name.is_some() && o.name == focus.name)
                {
                    apply_msaa_states(&mut focus, object.states);
                }
                focus.text = event.text_context.clone();
                let name = event.object.as_ref().and_then(|o| o.name.clone());
                self.focused = Some((event.hwnd, event.id_object, event.id_child, name));
                self.focused_control = focus.name.clone();
                self.last_focus = Some(focus.clone());
                return Some(AccessibilityEvent::Focus(focus));
            }
            // A select's value changed: it has no text, only the new choice as its value. It is
            // said as a value change [obs, 205/s9, s10].
            "value_change" if is_choice_without_text(event) => ChangeKind::Value,
            // The focused text field's caret moved or its text changed.
            "caret" | "text_selection_change" | "value_change" if event.text_context.is_some() => {
                return event
                    .text_context
                    .clone()
                    .map(AccessibilityEvent::TextContext);
            }
            "state_change" => ChangeKind::State,
            "value_change" => ChangeKind::Value,
            "selection" => ChangeKind::Selection,
            _ => return None,
        };
        let (hwnd, id_object, id_child, name) = self.focused.as_ref()?;
        let same_object =
            event.hwnd == *hwnd && event.id_object == *id_object && event.id_child == *id_child;
        // The focus event's MSAA object can be the browser window while the focused control is
        // a page button (focus left on it, then Space expanded it, GitHub): the control's own
        // name, from the focus chain, is compared too.
        let object_name = event.object.as_ref().and_then(|o| o.name.as_ref());
        let control_name = self.focused_control.as_ref();
        let focus_on_window = control_name.is_some() && name.as_ref() != control_name;
        let same_name = object_name.is_some()
            && (object_name == name.as_ref() || (focus_on_window && object_name == control_name));
        if !(same_object || same_name) {
            return None;
        }
        // The control as UIA reports it now; without a focus chain (a failed UIA read), the
        // last focus with the event object's MSAA states.
        let mut control = match focus_event_from_trace(event) {
            Some(control) => control,
            None => self.last_focus.clone()?,
        };
        // A change event reads no dialog: the dialog's controls are the last focus's, in the
        // same window, so Say Window still has them after a button's state changes (537/s8).
        if control.window_items.is_empty() {
            if let Some(last) = self
                .last_focus
                .as_ref()
                .filter(|l| l.window_title == control.window_title)
            {
                control.window_items = last.window_items.clone();
                if control.default_button.is_none() {
                    control.default_button = last.default_button.clone();
                }
            }
        }
        // The event object is the focused control: its MSAA states are current, and they
        // carry what UIA may not report (a WinForms combo box's expanded state, 304/s11).
        if same_object {
            if let Some(states) = event.object.as_ref().map(|o| o.states) {
                apply_msaa_states(&mut control, states);
            }
        }
        // A change event carries no siblings to count a position from; the control is the
        // one last focused, whose position it keeps [obs, 304/s17].
        if control.item_index.is_none() {
            if let Some(last) = self.last_focus.as_ref().filter(|l| l.name == control.name) {
                control.item_index = last.item_index;
                control.item_count = last.item_count;
            }
        }
        self.last_focus = Some(control.clone());
        Some(AccessibilityEvent::Change { kind, control })
    }
}

/// A focus on an item of a Win32 combo box's drop-down list (`ComboLBox`).
fn is_drop_down_item(event: &TraceEvent) -> bool {
    const MSAA_LISTITEM: i32 = 0x22;
    event.window_class.as_deref() == Some("ComboLBox")
        && event.object.as_ref().and_then(|o| o.role) == Some(MSAA_LISTITEM)
}

/// Text lines of an IA2 subtree: its static text runs, in order.
fn text_lines(node: &crate::ia2_snapshot::Ia2Node, out: &mut Vec<String>) {
    const MSAA_STATICTEXT: i32 = 0x29;
    if node.msaa_role == Some(MSAA_STATICTEXT) {
        if let Some(name) = node
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            out.push(name.to_owned());
        }
    }
    for child in &node.children {
        text_lines(child, out);
    }
}

/// A UIA notification, said as a live region's announcement: whole each time (the same text
/// again too), interrupting speech when the application marks it important.
fn notification_event(notification: &TraceNotification) -> okur_core::AccessibilityEvent {
    use okur_core::{LiveRegionEvent, Politeness};
    // NotificationProcessing_ImportantAll and _ImportantMostRecent.
    let politeness = if matches!(notification.processing, 0 | 1) {
        Politeness::Assertive
    } else {
        Politeness::Polite
    };
    let mut live = LiveRegionEvent::new(notification.text.trim(), politeness);
    live.region_id = Some("uia-notification".to_owned());
    live.atomic = true;
    okur_core::AccessibilityEvent::LiveRegion(live)
}

/// A native application's live region changed (UIA): the texts it holds, then its own name
/// where that is not one of them, as JAWS reads Windows Settings' expander button as Alt+Left
/// shows its page again: " Brightness Change brightness for the built-in display | Show more
/// settings" (the slider inside is left out; dev box, 2026-10-17). A text region says its
/// name. A region whose live setting is off says nothing.
fn uia_live_region_event(region: &UiaNode) -> Option<okur_core::LiveRegionEvent> {
    use okur_core::{LiveRegionEvent, Politeness};
    fn texts(node: &UiaNode, out: &mut Vec<String>) {
        for child in &node.children {
            if child.control_type == UIA_TEXT {
                if let Some(name) = child
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                {
                    out.push(name.to_owned());
                }
            }
            texts(child, out);
        }
    }
    let politeness = match region.live_setting {
        Some(0) => return None,
        Some(2) => Politeness::Assertive,
        _ => Politeness::Polite,
    };
    let mut held = Vec::new();
    texts(region, &mut held);
    let name = region
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty() && !held.iter().any(|t| t == n));
    let mut parts = Vec::new();
    if !held.is_empty() {
        parts.push(held.join(" "));
    }
    parts.extend(name.map(str::to_owned));
    if parts.is_empty() {
        return None;
    }
    let mut live = LiveRegionEvent::new(parts.join("\n"), politeness);
    live.region_id = Some("uia-live-region".to_owned());
    live.atomic = true;
    Some(live)
}

/// A live region changed: its announcement, then the page as it is now, which refreshes the
/// buffer [obs, 206/s2-s10].
fn live_region_events(event: &TraceEvent) -> Vec<okur_core::AccessibilityEvent> {
    use okur_core::{AccessibilityEvent, LiveRegionEvent, Politeness};
    let mut events = Vec::new();
    if let Some(region) = &event.region {
        let mut lines = Vec::new();
        text_lines(region, &mut lines);
        // The region's ARIA properties, as Chromium gives them (its own, or inherited from the
        // live region it is in): an assertive region interrupts what is being said (Google
        // Docs' announcements as the caret moves), an atomic one is read whole.
        let attribute = |name: &str, inherited: &str| {
            region
                .attributes
                .get(name)
                .or_else(|| region.attributes.get(inherited))
                .map(String::as_str)
        };
        let politeness = match attribute("live", "container-live") {
            Some("assertive") => Politeness::Assertive,
            Some("off") => Politeness::Off,
            _ => Politeness::Polite,
        };
        let mut live = LiveRegionEvent::new(lines.join("\n"), politeness);
        live.container_name = region.name.clone().filter(|n| !n.trim().is_empty());
        live.region_id = region.attributes.get("id").cloned();
        live.atomic = attribute("atomic", "container-atomic") == Some("true");
        events.push(AccessibilityEvent::LiveRegion(live));
    }
    if let Some(page) = &event.page {
        events.push(AccessibilityEvent::DocumentLoaded {
            title: page.name.clone().unwrap_or_default(),
            root: crate::ia2_snapshot::ia2_to_aom(page),
        });
    }
    events
}

/// The text just before the control with `runtime_id` among its siblings in a dialog's tree:
/// the label that names it.
fn label_before(tree: &UiaNode, runtime_id: Option<&[i32]>) -> Option<String> {
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
            .map(|name| crate::screen_lines::without_accelerators(&name))
            .filter(|name| !name.trim().is_empty()),
        None => tree
            .children
            .iter()
            .find_map(|child| label_before(child, Some(runtime_id))),
    }
}

/// The window a menu bar's focus report gives the focus back to as its menus end (a focus
/// event on OBJID_MENU, MSAA's menu bar object): the window itself, which the engine takes
/// as the control focused before the menus.
fn menu_bar_return(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
    const OBJID_MENU: i32 = -3;
    const ROLE_SYSTEM_MENUBAR: i32 = 0x02;
    if event.event != "focus" || event.id_object != OBJID_MENU {
        return None;
    }
    event
        .object
        .as_ref()
        .filter(|o| o.role == Some(ROLE_SYSTEM_MENUBAR))?;
    let title = event.window_title.clone()?;
    let mut focus = okur_core::FocusEvent::new(okur_core::Role::Window).with_name(title.clone());
    focus.window_title = Some(title);
    focus.platform.window_class = event.window_class.clone();
    focus.app = event.process.clone();
    // The field the window gives the focus back to, as the trace read it.
    focus.text = event.text_context.clone();
    Some(focus)
}

/// The focus in a NetUI window (WordPad's and Paint's ribbon, `NetUIHWND`), read from the
/// event's MSAA object: UI Automation stops at the ribbon's host pane, so every tab and button
/// came as the same "Ribbon" pane and was said once (523/s2, s3), where the MSAA object names
/// the tab ("View", a page tab, selected). `None` where UI Automation has the element itself.
fn netui_focus(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
    use okur_core::Role;
    const PAGETAB: i32 = 0x25;
    const PUSHBUTTON: i32 = 0x2B;
    const CHECKBUTTON: i32 = 0x2C;
    const RADIOBUTTON: i32 = 0x2D;
    const COMBOBOX: i32 = 0x2E;
    const BUTTONDROPDOWN: i32 = 0x38;
    const SPLITBUTTON: i32 = 0x3E;
    const SELECTED: u32 = 0x2;
    if event.window_class.as_deref() != Some("NetUIHWND") {
        return None;
    }
    let object = event.object.as_ref()?;
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
    let role = match object.role? {
        PAGETAB => Role::Tab,
        PUSHBUTTON => Role::Button,
        CHECKBUTTON => Role::CheckBox,
        RADIOBUTTON => Role::RadioButton,
        COMBOBOX => Role::ComboBox,
        BUTTONDROPDOWN | SPLITBUTTON => Role::SplitButton,
        _ => return None,
    };
    let tab = role == Role::Tab;
    let mut focus = okur_core::FocusEvent::new(role).with_name(name.to_owned());
    if tab {
        focus.states.selected = Some(object.states & SELECTED != 0);
    }
    apply_msaa_states(&mut focus, object.states);
    focus.app = event.process.clone();
    focus.in_ribbon = true;
    Some(focus)
}

/// Updates the states a change can carry from MSAA state bits (`STATE_SYSTEM_*`).
fn apply_msaa_states(control: &mut okur_core::FocusEvent, states: u32) {
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

#[cfg(windows)]
mod capture {
    use super::{
        event_name, in_dialog_box, reports_position, TraceEvent, TraceHeader, TraceObject,
        TRACE_FORMAT_VERSION, UIA_FOCUS_WINDOWS, UIA_SELECTION_PROCESSES,
    };
    use crate::uia_snapshot::capture::{
        automation, column_headers_of, focus_chain_from, focus_context_of, focus_subtree_of,
        focused_element, window_subtree, window_text, with_properties,
    };
    use crate::PlatformError;
    use std::cell::RefCell;
    use std::io::Write;
    use windows::core::implement;
    use windows::Win32::Foundation::{CloseHandle, HWND};
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::{
        AccessibleObjectFromEvent, IAccessible, IUIAutomation, SetWinEventHook, UnhookWinEvent,
        HWINEVENTHOOK,
    };
    use windows::Win32::UI::Accessibility::{
        IUIAutomationElement, IUIAutomationEventHandler, IUIAutomationEventHandler_Impl,
        IUIAutomationFocusChangedEventHandler, IUIAutomationFocusChangedEventHandler_Impl,
        IUIAutomationNotificationEventHandler, IUIAutomationNotificationEventHandler_Impl,
        NotificationKind, NotificationProcessing, TreeScope_Subtree, UIA_LiveRegionChangedEventId,
        UIA_SelectionItem_ElementSelectedEventId,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetWindowThreadProcessId, MsgWaitForMultipleObjects, PeekMessageW,
        TranslateMessage, MSG, PM_REMOVE, QS_ALLINPUT,
    };

    const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
    const WINEVENT_SKIPOWNPROCESS: u32 = 0x0002;

    /// How soon a focus event naming the focused element again is a repeat of the last one.
    const REPEATED_FOCUS: std::time::Duration = std::time::Duration::from_millis(1000);
    /// How soon after a focus a selection or value change of the same element is its echo.
    const FOCUS_ECHO: std::time::Duration = std::time::Duration::from_millis(300);
    /// A WinEvent about an object itself rather than one of its children.
    const CHILDID_SELF: i32 = 0;

    /// A focus event that was read: the element it named, when, and its event's window.
    #[derive(Clone)]
    struct SeenFocus {
        element: IUIAutomationElement,
        at: std::time::Instant,
        window: isize,
    }

    impl SeenFocus {
        /// Whether `element` is this focus's element, named again within `within`.
        fn names(
            &self,
            uia: &IUIAutomation,
            element: &IUIAutomationElement,
            within: std::time::Duration,
        ) -> bool {
            self.at.elapsed() < within
                // SAFETY: comparing two valid automation elements.
                && unsafe { uia.CompareElements(element, &self.element) }
                    .is_ok_and(|same| same.as_bool())
        }
    }

    struct TraceState {
        /// Receives every event: the trace file, or the live screen reader.
        sink: Box<dyn FnMut(TraceEvent)>,
        uia: Option<IUIAutomation>,
        count: usize,
        /// Feeding the live screen reader: events it does not use are dropped before any
        /// cross-process read, and live-region pages are captured elsewhere.
        live: bool,
        /// The last focus event about a whole object (not a child item): the same element
        /// named again right after is not read twice.
        last_object_focus: Option<SeenFocus>,
        /// The last focus event read, of any kind: a selection or value change for its
        /// element right after is its echo.
        last_focus_read: Option<SeenFocus>,
        /// Told when the page in front changes (live only), at most every
        /// [`PAGE_CHANGE_INTERVAL`].
        page_changed: Option<Box<dyn FnMut(PageChange)>>,
        page_changed_at: Option<std::time::Instant>,
        /// Page changes not told since the last one, for the debug log.
        page_changes_dropped: usize,
        /// Tracing (not live): every page change is recorded as a `page_change` event, with
        /// its object and nothing read (`okur trace --page-changes`).
        record_page_changes: bool,
    }

    /// The shortest time between two page changes told: a page being built reports its
    /// objects shown and reordered by the hundred.
    const PAGE_CHANGE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);

    thread_local! {
        // The out-of-context hook calls back on the installing thread, so thread-local
        // state is sufficient and needs no locking.
        static STATE: RefCell<Option<TraceState>> = const { RefCell::new(None) };
    }

    fn now_ms() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    }

    fn process_name(hwnd: HWND) -> Option<String> {
        let mut pid = 0u32;
        // SAFETY: valid out-pointer; a stale hwnd yields pid 0.
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        process_name_of(pid)
    }

    /// The executable's file name of process `pid`.
    fn process_name_of(pid: u32) -> Option<String> {
        if pid == 0 {
            return None;
        }
        // SAFETY: limited-information query of another process; the handle is closed below.
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok();
            let _ = CloseHandle(handle);
            if !ok {
                return None;
            }
            let path = String::from_utf16_lossy(buf.get(..len as usize).unwrap_or_default());
            path.rsplit('\\').next().map(str::to_owned)
        }
    }

    /// The selection of a Win32 combo box or list box as `[index, count]`, asked of the
    /// control itself (`CB_GETCURSEL`/`CB_GETCOUNT`, `LB_GETCURSEL`/`LB_GETCOUNT`). A
    /// collapsed WinForms combo box exposes no items through UIA, yet JAWS says "2 of 3".
    fn win32_list_position(hwnd: HWND, class: &str) -> Option<[i32; 2]> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            GetParent, SendMessageTimeoutW, SMTO_ABORTIFHUNG,
        };
        // An edit inside a combo box: the combo box's selection.
        if class.eq_ignore_ascii_case("Edit") {
            // SAFETY: plain Win32 query on a window handle.
            let parent = unsafe { GetParent(hwnd) }.ok()?;
            let (_, parent_class) = window_text(parent);
            return parent_class
                .to_ascii_uppercase()
                .contains("COMBOBOX")
                .then(|| win32_list_position(parent, &parent_class))
                .flatten();
        }
        let upper = class.to_ascii_uppercase();
        // A list view: its focused item and item count (LVM_GETNEXTITEM with LVNI_FOCUSED,
        // searching from the start; LVM_GETITEMCOUNT). Counting its items through UIA took
        // 2 s per item in a 300-item list (dev box, 2026-10-15).
        if upper.contains("SYSLISTVIEW32") {
            const LVM_GETITEMCOUNT: u32 = 0x1004;
            const LVM_GETNEXTITEM: u32 = 0x100C;
            const LVNI_FOCUSED: isize = 0x0001;
            let ask = |msg: u32, wparam: usize, lparam: isize| {
                let mut result = 0usize;
                // SAFETY: read-only integer queries with a timeout, to a window that may be
                // gone.
                let sent = unsafe {
                    SendMessageTimeoutW(
                        hwnd,
                        msg,
                        WPARAM(wparam),
                        LPARAM(lparam),
                        SMTO_ABORTIFHUNG,
                        200,
                        Some(&mut result),
                    )
                };
                (sent.0 != 0).then_some(result as i32)
            };
            let count = ask(LVM_GETITEMCOUNT, 0, 0).filter(|&n| n > 0)?;
            let index = ask(LVM_GETNEXTITEM, usize::MAX, LVNI_FOCUSED).filter(|&i| i >= 0)?;
            return Some([index, count]);
        }
        if upper == "BUTTON" {
            return win32_radio_position(hwnd);
        }
        let (get_sel, get_count) = if upper.contains("COMBOBOX") {
            (0x0147, 0x0146)
        } else if upper.contains("LISTBOX") {
            (0x0188, 0x018B)
        } else {
            return None;
        };
        let ask_with = |msg: u32, wparam: WPARAM, lparam: LPARAM| {
            let mut result = 0usize;
            // SAFETY: a read-only query message with a timeout, to a window that may be gone;
            // a string parameter outlives the call (the system copies it across processes).
            let sent = unsafe {
                SendMessageTimeoutW(
                    hwnd,
                    msg,
                    wparam,
                    lparam,
                    SMTO_ABORTIFHUNG,
                    200,
                    Some(&mut result),
                )
            };
            // CB_ERR/LB_ERR is -1, as the low 32 bits of the result.
            (sent.0 != 0).then_some(result as i32)
        };
        let ask = |msg: u32| ask_with(msg, WPARAM(0), LPARAM(0));
        let count = ask(get_count).filter(|&n| n > 0)?;
        let index = ask(get_sel).filter(|&i| i >= 0).or_else(|| {
            // An editable combo box's typed text selects nothing: its position is that of
            // the list item with the same text (JAWS on the Run dialog: "C: | 1 of 6").
            const CB_FINDSTRINGEXACT: u32 = 0x0158;
            if !upper.contains("COMBOBOX") {
                return None;
            }
            let text = crate::screen_lines::control_text(hwnd);
            let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            ask_with(
                CB_FINDSTRINGEXACT,
                WPARAM(usize::MAX),
                LPARAM(wide.as_ptr() as isize),
            )
            .filter(|&i| i >= 0)
        })?;
        Some([index, count])
    }

    /// A radio button's place among the radio buttons of its group, as Windows groups them
    /// (from a control with `WS_GROUP` to the next, the run its arrow keys move in): JAWS's
    /// Links List says "Sort Links In Tab Order radio button checked 1 of 2" and, in the
    /// group after it, "Display All Links ... 1 of 3" [obs, 222/s5-s6]. Not a radio button:
    /// `None`.
    fn win32_radio_position(hwnd: HWND) -> Option<[i32; 2]> {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindow, GetWindowLongW, GWL_STYLE, GW_HWNDNEXT, GW_HWNDPREV,
        };
        const WS_GROUP: i32 = 0x0002_0000;
        const BS_TYPEMASK: i32 = 0x000F;
        const BS_RADIOBUTTON: i32 = 0x0004;
        const BS_AUTORADIOBUTTON: i32 = 0x0009;
        // Enough for any dialog; a broken window chain cannot loop forever.
        const MOST: usize = 256;
        // SAFETY: plain Win32 queries on window handles that may be gone (they then fail).
        let style = |h: HWND| unsafe { GetWindowLongW(h, GWL_STYLE) };
        let is_radio = |h: HWND| {
            window_text(h).1.eq_ignore_ascii_case("Button")
                && matches!(style(h) & BS_TYPEMASK, BS_RADIOBUTTON | BS_AUTORADIOBUTTON)
        };
        // SAFETY: as above.
        let step = |h: HWND, way| {
            unsafe { GetWindow(h, way) }
                .ok()
                .filter(|n| !n.is_invalid())
        };
        if !is_radio(hwnd) {
            return None;
        }
        let mut first = hwnd;
        for _ in 0..MOST {
            if style(first) & WS_GROUP != 0 {
                break;
            }
            match step(first, GW_HWNDPREV) {
                Some(previous) => first = previous,
                None => break,
            }
        }
        let (mut index, mut count, mut window) = (None, 0, first);
        for _ in 0..MOST {
            if is_radio(window) {
                if window == hwnd {
                    index = Some(count);
                }
                count += 1;
            }
            match step(window, GW_HWNDNEXT).filter(|n| style(*n) & WS_GROUP == 0) {
                Some(next) => window = next,
                None => break,
            }
        }
        Some([index?, count])
    }

    /// A Chromium live region that changed, with its subtree, and the page after the change.
    fn chromium_live_region(
        hwnd: HWND,
        class: &str,
        child_id: i32,
        with_page: bool,
    ) -> (
        Option<crate::ia2_snapshot::Ia2Node>,
        Option<crate::ia2_snapshot::Ia2Node>,
    ) {
        use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT};
        if class != "Chrome_RenderWidgetHostHWND" {
            return (None, None);
        }
        // SAFETY: plain Win32 query on a window handle.
        let top = unsafe { GetAncestor(hwnd, GA_ROOT) };
        (
            xa11y_windows::ia2::capture_chromium_object(top, child_id).ok(),
            with_page
                .then(|| xa11y_windows::ia2::capture_chromium_document(top).ok())
                .flatten(),
        )
    }

    /// The title and class of `hwnd`'s top-level window, and the virtual desktop's name when
    /// that window is the desktop.
    fn top_window(hwnd: HWND) -> (Option<String>, Option<String>, Option<String>) {
        use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT};
        // SAFETY: plain Win32 query on a window handle.
        let top = unsafe { GetAncestor(hwnd, GA_ROOT) };
        if top.is_invalid() {
            return (None, None, None);
        }
        let (title, class) = window_text(top);
        let desktop = matches!(class.as_str(), "Progman" | "WorkerW")
            .then(crate::virtual_desktop::current_name);
        (
            Some(title).filter(|t| !t.is_empty()),
            Some(class).filter(|c| !c.is_empty()),
            desktop,
        )
    }

    /// The events the live screen reader consumes ([`EventConverter`]).
    const LIVE_EVENTS: [&str; 14] = [
        "focus",
        "uia_focus",
        "uia_selected",
        "foreground",
        "caret",
        "text_selection_change",
        "value_change",
        "state_change",
        "selection",
        "live_region_change",
        "menu_start",
        "menu_end",
        "menu_popup_start",
        "menu_popup_end",
    ];

    /// Whether `hwnd` belongs to the process of the foreground window.
    fn in_foreground_process(hwnd: HWND) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };
        let mut event_pid = 0u32;
        let mut foreground_pid = 0u32;
        // SAFETY: plain Win32 queries with valid out-pointers.
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut event_pid));
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_pid));
        }
        event_pid != 0 && event_pid == foreground_pid
    }

    /// Whether `hwnd` is inside a file dialog's navigation pane (Windows'
    /// `NamespaceTreeControl`), a few windows up.
    fn in_namespace_tree(hwnd: HWND) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::GetParent;
        let mut window = hwnd;
        for _ in 0..6 {
            // SAFETY: plain Win32 query on a window handle, which may be gone.
            let Ok(parent) = (unsafe { GetParent(window) }) else {
                return false;
            };
            if window_text(parent).1 == "NamespaceTreeControl" {
                return true;
            }
            window = parent;
        }
        false
    }

    /// Whether `hwnd` belongs to an application other than the one in front, both known (a
    /// window in front is briefly none while windows switch).
    fn in_background_process(hwnd: HWND) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };
        let mut event_pid = 0u32;
        let mut foreground_pid = 0u32;
        // SAFETY: plain Win32 queries with valid out-pointers.
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut event_pid));
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_pid));
        }
        event_pid != 0 && foreground_pid != 0 && event_pid != foreground_pid
    }

    fn msaa_object(hwnd: HWND, id_object: i32, id_child: i32) -> Option<TraceObject> {
        let mut acc: Option<IAccessible> = None;
        let mut child = VARIANT::default();
        // SAFETY: standard MSAA event-object resolution with valid out-pointers.
        unsafe {
            AccessibleObjectFromEvent(
                hwnd,
                id_object as u32,
                id_child as u32,
                &mut acc,
                &mut child,
            )
            .ok()?;
            let acc = acc?;
            Some(TraceObject {
                role: acc
                    .get_accRole(&child)
                    .ok()
                    .and_then(|v| i32::try_from(&v).ok()),
                name: acc.get_accName(&child).ok().map(|b| b.to_string()),
                value: acc
                    .get_accValue(&child)
                    .ok()
                    .map(|b| b.to_string())
                    .filter(|s| !s.is_empty()),
                states: acc
                    .get_accState(&child)
                    .ok()
                    .and_then(|v| i32::try_from(&v).ok())
                    .map(|s| s as u32)
                    .unwrap_or(0),
            })
        }
    }

    unsafe extern "system" fn on_event(
        _hook: HWINEVENTHOOK,
        win_event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        _thread: u32,
        time: u32,
    ) {
        let Some(name) = event_name(win_event, id_object) else {
            return;
        };
        record_at(
            event_ms(time),
            name,
            win_event,
            hwnd,
            id_object,
            id_child,
            None,
        );
    }

    /// When an event happened, in Unix milliseconds, from its `dwmsEventTime` (the tick count
    /// when Windows raised it): events wait in the hook's queue while the ones before them are
    /// read, and a trace places each in the step it belongs to by this time.
    fn event_ms(time: u32) -> u128 {
        use windows::Win32::System::SystemInformation::GetTickCount;
        // SAFETY: plain Win32 call with no preconditions.
        let waited = unsafe { GetTickCount() }.wrapping_sub(time);
        now_ms().saturating_sub(u128::from(waited))
    }

    /// Objects shown, hidden or reordered in the page in front (`EVENT_OBJECT_SHOW`, `_HIDE`,
    /// `_REORDER`): the page changed. Chromium reports its web objects on the page's
    /// "Chrome Legacy Window" by their negative uniqueIDs. Nothing is read across processes:
    /// a page being built sends these by the hundred.
    unsafe extern "system" fn on_page_change(
        _hook: HWINEVENTHOOK,
        win_event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetAncestor, GetClassNameW, GetForegroundWindow, IsWindowVisible, GA_ROOT,
        };
        const OBJID_CLIENT: i32 = -4;
        if id_object != OBJID_CLIENT || id_child >= 0 {
            return;
        }
        let mut class = [0u16; 32];
        // SAFETY: plain Win32 queries on the event's window, with a valid buffer.
        let in_page_in_front = unsafe {
            let length = GetClassNameW(hwnd, &mut class);
            class
                .get(..length as usize)
                .is_some_and(|class| CHROME_PAGE_WINDOW.encode_utf16().eq(class.iter().copied()))
                && IsWindowVisible(hwnd).as_bool()
                && GetAncestor(hwnd, GA_ROOT) == GetForegroundWindow()
        };
        if !in_page_in_front {
            return;
        }
        STATE.with(|s| {
            let Ok(mut state) = s.try_borrow_mut() else {
                return;
            };
            let Some(state) = state.as_mut() else {
                return;
            };
            if state.record_page_changes {
                let event = TraceEvent {
                    t_ms: now_ms(),
                    event: "page_change".to_owned(),
                    win_event,
                    hwnd: hwnd.0 as isize,
                    id_object,
                    id_child,
                    window_class: Some(CHROME_PAGE_WINDOW.to_owned()),
                    window_style: None,
                    focus_window_class: None,
                    menu_bar: Vec::new(),
                    status_bar: Vec::new(),
                    window_title: None,
                    process: None,
                    object: None,
                    focus_chain: Vec::new(),
                    focus_siblings: Vec::new(),
                    focus_subtree: None,
                    list_position: None,
                    column_headers: Vec::new(),
                    text_context: None,
                    region: None,
                    page: None,
                    top_window_title: None,
                    top_window_class: None,
                    virtual_desktop: None,
                    dialog_lines: Vec::new(),
                    dialog_text: Vec::new(),
                    dialog_page: None,
                    group_box: None,
                    default_button: None,
                    dialog_tree: None,
                    notification: None,
                };
                (state.sink)(event);
                state.count += 1;
                return;
            }
            // Every change is passed on with its object; the engine is told at most every
            // PAGE_CHANGE_INTERVAL.
            let notify = !state
                .page_changed_at
                .is_some_and(|at| at.elapsed() < PAGE_CHANGE_INTERVAL);
            if notify {
                tracing::debug!(
                    event = win_event,
                    object = id_child,
                    dropped_before = state.page_changes_dropped,
                    "page changed"
                );
                state.page_changed_at = Some(std::time::Instant::now());
                state.page_changes_dropped = 0;
            } else {
                state.page_changes_dropped += 1;
            }
            if let Some(page_changed) = state.page_changed.as_mut() {
                page_changed(PageChange {
                    event: win_event,
                    object: id_child,
                    notify,
                });
            }
        });
    }

    /// The window class of a Chromium page ("Chrome Legacy Window").
    const CHROME_PAGE_WINDOW: &str = "Chrome_RenderWidgetHostHWND";

    /// IA2 text inserted into and removed from an object's text (`AccessibleEventId.idl`).
    pub const IA2_EVENT_TEXT_INSERTED: u32 = 0x011E;
    pub const IA2_EVENT_TEXT_REMOVED: u32 = 0x011F;

    /// An object shown, hidden or reordered in the page in front (`EVENT_OBJECT_SHOW`,
    /// `_HIDE`, `_REORDER`), or whose text changed (`IA2_EVENT_TEXT_INSERTED`, `_REMOVED`),
    /// by its (negative) IA2 uniqueID. `notify`: the engine is to be
    /// told now (at most every [`PAGE_CHANGE_INTERVAL`]).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageChange {
        pub event: u32,
        pub object: i32,
        pub notify: bool,
    }

    /// UIA's focus-changed event id.
    const UIA_FOCUS_CHANGED: u32 = 20005;
    /// An event read taking longer is logged with its stages.
    const SLOW_EVENT: std::time::Duration = std::time::Duration::from_millis(250);
    /// Posted to the tracer thread by [`UiaFocusHandler`].
    const WM_UIA_FOCUS: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;

    /// A UIA element handed from UIA's event thread to the tracer thread.
    struct MtaElement(IUIAutomationElement);
    // SAFETY: both threads are in the multithreaded apartment, where a COM object may be
    // called from any thread.
    unsafe impl Send for MtaElement {}

    /// The elements of UIA focus changes not yet recorded, oldest first, for the tracer
    /// thread: on Windows 11 the task switcher's items are named only by the event, while
    /// `GetFocusedElement` answers with the switcher's input pane (seen live on the dev box,
    /// 2026-10-15). Each is recorded: keeping only the latest lost the first item to the
    /// switcher window's own focus right after it.
    static UIA_FOCUSED: std::sync::Mutex<std::collections::VecDeque<MtaElement>> =
        std::sync::Mutex::new(std::collections::VecDeque::new());
    /// Focus changes kept for the tracer thread at most (Tab held down repeats fast).
    const MAX_UIA_FOCUSED: usize = 8;

    /// Wakes the tracer thread on UIA focus changes in [`UIA_FOCUS_WINDOWS`]. UIA calls it on
    /// its own threads, and it also reports every WinEvent focus (UIA bridges MSAA), so it
    /// only checks the foreground window's class, a call that crosses no process.
    #[implement(IUIAutomationFocusChangedEventHandler)]
    struct UiaFocusHandler {
        thread: u32,
    }

    impl IUIAutomationFocusChangedEventHandler_Impl for UiaFocusHandler_Impl {
        fn HandleFocusChangedEvent(
            &self,
            sender: windows::core::Ref<IUIAutomationElement>,
        ) -> windows::core::Result<()> {
            use windows::Win32::Foundation::{LPARAM, WPARAM};
            use windows::Win32::UI::WindowsAndMessaging::{
                GetClassNameW, GetForegroundWindow, PostThreadMessageW,
            };
            let mut class = [0u16; 64];
            // SAFETY: plain Win32 queries with a valid buffer; the post targets a thread
            // that pumps messages until the handler is removed.
            unsafe {
                let length = GetClassNameW(GetForegroundWindow(), &mut class);
                let class = String::from_utf16_lossy(class.get(..length as usize).unwrap_or(&[]));
                if UIA_FOCUS_WINDOWS.contains(&class.as_str()) {
                    if let (Some(element), Ok(mut queue)) = (sender.as_ref(), UIA_FOCUSED.lock()) {
                        if queue.len() >= MAX_UIA_FOCUSED {
                            queue.pop_front();
                        }
                        queue.push_back(MtaElement(element.clone()));
                    }
                    let _ = PostThreadMessageW(self.thread, WM_UIA_FOCUS, WPARAM(0), LPARAM(0));
                }
            }
            Ok(())
        }
    }

    /// Records the focus as it is now, as a focus event on the foreground thread's focused
    /// window (the foreground window when it has none).
    fn record_current_focus() {
        const OBJID_CLIENT: i32 = -4;
        const EVENT_OBJECT_FOCUS: u32 = 0x8005;
        let focused = keyboard_focus_window();
        if !focused.is_invalid() {
            record("focus", EVENT_OBJECT_FOCUS, focused, OBJID_CLIENT, 0, None);
        }
    }

    /// The window with the keyboard focus in the foreground thread (the foreground window
    /// when none has it). A focus event can arrive on another window: the dialog box whose
    /// field has the focus.
    fn keyboard_focus_window() -> HWND {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
        };
        // SAFETY: plain Win32 queries; the info struct is sized as the call requires.
        unsafe {
            let foreground = GetForegroundWindow();
            let thread = GetWindowThreadProcessId(foreground, None);
            let mut info = GUITHREADINFO {
                cbSize: u32::try_from(std::mem::size_of::<GUITHREADINFO>()).unwrap_or(0),
                ..Default::default()
            };
            match GetGUIThreadInfo(thread, &mut info) {
                Ok(()) if !info.hwndFocus.is_invalid() => info.hwndFocus,
                _ => foreground,
            }
        }
    }

    /// Records the focus UIA reported last, on the tracer thread.
    fn record_uia_focus() {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let elements: Vec<MtaElement> = UIA_FOCUSED
            .lock()
            .map(|mut queue| queue.drain(..).collect())
            .unwrap_or_default();
        // SAFETY: plain Win32 call with no preconditions.
        let foreground = unsafe { GetForegroundWindow() };
        for element in &elements {
            record(
                "uia_focus",
                UIA_FOCUS_CHANGED,
                foreground,
                0,
                0,
                Some(&element.0),
            );
        }
    }

    /// UIA's element-selected event id.
    const UIA_ELEMENT_SELECTED: u32 = 20012;
    /// Posted to the tracer thread by [`UiaSelectedHandler`].
    const WM_UIA_SELECTED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 2;
    /// Selected elements not yet recorded, oldest first.
    static UIA_SELECTED: std::sync::Mutex<std::collections::VecDeque<MtaElement>> =
        std::sync::Mutex::new(std::collections::VecDeque::new());

    /// Wakes the tracer thread on UIA element selections in [`UIA_SELECTION_PROCESSES`] in
    /// front. Selections everywhere reach it (it is registered on the desktop), so it only
    /// looks up the foreground window's process before anything else.
    #[implement(IUIAutomationEventHandler)]
    struct UiaSelectedHandler {
        thread: u32,
    }

    impl IUIAutomationEventHandler_Impl for UiaSelectedHandler_Impl {
        fn HandleAutomationEvent(
            &self,
            sender: windows::core::Ref<IUIAutomationElement>,
            _event: windows::Win32::UI::Accessibility::UIA_EVENT_ID,
        ) -> windows::core::Result<()> {
            use windows::Win32::Foundation::{LPARAM, WPARAM};
            use windows::Win32::UI::WindowsAndMessaging::{
                GetForegroundWindow, PostThreadMessageW,
            };
            // SAFETY: plain Win32 call with no preconditions.
            let process = process_name(unsafe { GetForegroundWindow() }).unwrap_or_default();
            if !UIA_SELECTION_PROCESSES
                .iter()
                .any(|p| p.eq_ignore_ascii_case(&process))
            {
                return Ok(());
            }
            if let (Some(element), Ok(mut queue)) = (sender.as_ref(), UIA_SELECTED.lock()) {
                if queue.len() >= MAX_UIA_FOCUSED {
                    queue.pop_front();
                }
                queue.push_back(MtaElement(element.clone()));
            }
            // SAFETY: the post targets a thread that pumps messages until the handler is
            // removed.
            let _ =
                unsafe { PostThreadMessageW(self.thread, WM_UIA_SELECTED, WPARAM(0), LPARAM(0)) };
            Ok(())
        }
    }

    /// Records the selections UIA reported, on the tracer thread.
    fn record_uia_selected() {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let elements: Vec<MtaElement> = UIA_SELECTED
            .lock()
            .map(|mut queue| queue.drain(..).collect())
            .unwrap_or_default();
        // SAFETY: plain Win32 call with no preconditions.
        let foreground = unsafe { GetForegroundWindow() };
        for element in &elements {
            record(
                "uia_selected",
                UIA_ELEMENT_SELECTED,
                foreground,
                0,
                0,
                Some(&element.0),
            );
        }
    }

    const WM_UIA_NOTIFICATION: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 3;
    /// UIA notifications not yet recorded, oldest first.
    static UIA_NOTIFICATIONS: std::sync::Mutex<
        std::collections::VecDeque<super::TraceNotification>,
    > = std::sync::Mutex::new(std::collections::VecDeque::new());

    /// The processes of the window in front: its own, and for a UWP app in a frame
    /// (ApplicationFrameHost: Windows Settings) the app's, which owns its core window.
    /// Windows' shell, whose UIA notifications are its toasts ("New notification from Windows
    /// PowerShell, ...", which JAWS says whatever is in front [obs, 533/s1]): the notification
    /// host, or Explorer.
    fn is_notification_host(pid: u32) -> bool {
        process_name_of(pid).is_some_and(|name| {
            name.eq_ignore_ascii_case("ShellExperienceHost.exe")
                || name.eq_ignore_ascii_case("explorer.exe")
        })
    }

    fn foreground_processes() -> [u32; 2] {
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, GetForegroundWindow};
        let mut frame = 0u32;
        let mut app = 0u32;
        // SAFETY: plain Win32 queries with valid out-pointers and a static class name.
        unsafe {
            let foreground = GetForegroundWindow();
            GetWindowThreadProcessId(foreground, Some(&mut frame));
            if let Ok(core) = FindWindowExW(
                Some(foreground),
                None,
                w!("Windows.UI.Core.CoreWindow"),
                None,
            ) {
                GetWindowThreadProcessId(core, Some(&mut app));
            }
        }
        [frame, app]
    }

    /// Wakes the tracer thread on UIA notifications from the application in front: what an
    /// application says without a focus or a live region (Settings' search box: "Suggestions
    /// available", which JAWS reads; dev box, 2026-10-16).
    #[implement(IUIAutomationNotificationEventHandler)]
    struct UiaNotificationHandler {
        thread: u32,
    }

    impl IUIAutomationNotificationEventHandler_Impl for UiaNotificationHandler_Impl {
        fn HandleNotificationEvent(
            &self,
            sender: windows::core::Ref<IUIAutomationElement>,
            _kind: NotificationKind,
            processing: NotificationProcessing,
            display: &windows::core::BSTR,
            _activity: &windows::core::BSTR,
        ) -> windows::core::Result<()> {
            use windows::Win32::Foundation::{LPARAM, WPARAM};
            use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;
            let text = display.to_string();
            if text.trim().is_empty() {
                return Ok(());
            }
            // SAFETY: a property read on the element UIA passed in.
            let pid = sender
                .as_ref()
                .and_then(|e| unsafe { e.CurrentProcessId() }.ok())
                .and_then(|p| u32::try_from(p).ok())
                .unwrap_or(0);
            if pid == 0 || !(foreground_processes().contains(&pid) || is_notification_host(pid)) {
                tracing::debug!(pid, %text, "UIA notification from a process behind, not taken");
                return Ok(());
            }
            if let Ok(mut queue) = UIA_NOTIFICATIONS.lock() {
                if queue.len() >= MAX_UIA_FOCUSED {
                    queue.pop_front();
                }
                queue.push_back(super::TraceNotification {
                    text,
                    processing: processing.0,
                });
            }
            // SAFETY: the post targets a thread that pumps messages until the handler is
            // removed.
            let _ = unsafe {
                PostThreadMessageW(self.thread, WM_UIA_NOTIFICATION, WPARAM(0), LPARAM(0))
            };
            Ok(())
        }
    }

    /// Records the notifications UIA reported, on the tracer thread.
    fn record_uia_notifications() {
        let notifications: Vec<super::TraceNotification> = UIA_NOTIFICATIONS
            .lock()
            .map(|mut queue| queue.drain(..).collect())
            .unwrap_or_default();
        for notification in notifications {
            STATE.with(|state| {
                let mut state = state.borrow_mut();
                let Some(state) = state.as_mut() else { return };
                let event = TraceEvent {
                    t_ms: now_ms(),
                    event: "uia_notification".to_owned(),
                    win_event: UIA_NOTIFICATION_EVENT,
                    hwnd: 0,
                    id_object: 0,
                    id_child: 0,
                    window_class: None,
                    window_style: None,
                    focus_window_class: None,
                    menu_bar: Vec::new(),
                    status_bar: Vec::new(),
                    window_title: None,
                    process: None,
                    object: None,
                    focus_chain: Vec::new(),
                    focus_siblings: Vec::new(),
                    focus_subtree: None,
                    list_position: None,
                    column_headers: Vec::new(),
                    text_context: None,
                    region: None,
                    page: None,
                    top_window_title: None,
                    top_window_class: None,
                    virtual_desktop: None,
                    dialog_lines: Vec::new(),
                    dialog_text: Vec::new(),
                    dialog_page: None,
                    group_box: None,
                    default_button: None,
                    dialog_tree: None,
                    notification: Some(notification),
                };
                (state.sink)(event);
                state.count += 1;
            });
        }
    }

    /// UIA's notification event id.
    const UIA_NOTIFICATION_EVENT: u32 = 20035;

    const WM_UIA_LIVE_REGION: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 4;
    /// Live regions changed and not yet recorded, oldest first.
    static UIA_LIVE_REGIONS: std::sync::Mutex<std::collections::VecDeque<MtaElement>> =
        std::sync::Mutex::new(std::collections::VecDeque::new());
    /// UIA's live region changed event id.
    const UIA_LIVE_REGION_EVENT: u32 = 20024;

    /// Wakes the tracer thread when a live region changes in the application in front: a
    /// native application's (Windows Settings' cards as a page is shown again). A browser's
    /// page regions are read through IA2 from their WinEvent instead.
    #[implement(IUIAutomationEventHandler)]
    struct UiaLiveRegionHandler {
        thread: u32,
    }

    impl IUIAutomationEventHandler_Impl for UiaLiveRegionHandler_Impl {
        fn HandleAutomationEvent(
            &self,
            sender: windows::core::Ref<IUIAutomationElement>,
            _event: windows::Win32::UI::Accessibility::UIA_EVENT_ID,
        ) -> windows::core::Result<()> {
            use windows::Win32::Foundation::{LPARAM, WPARAM};
            use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;
            let Some(element) = sender.as_ref() else {
                return Ok(());
            };
            // SAFETY: property reads on the element UIA passed in.
            let pid = unsafe { element.CurrentProcessId() }
                .ok()
                .and_then(|p| u32::try_from(p).ok())
                .unwrap_or(0);
            if pid == 0 || !foreground_processes().contains(&pid) {
                return Ok(());
            }
            // SAFETY: as above.
            let framework = unsafe { element.CurrentFrameworkId() }
                .map(|f| f.to_string())
                .unwrap_or_default();
            if matches!(framework.as_str(), "Chrome" | "Gecko") {
                return Ok(());
            }
            if let Ok(mut queue) = UIA_LIVE_REGIONS.lock() {
                if queue.len() >= MAX_UIA_FOCUSED {
                    queue.pop_front();
                }
                queue.push_back(MtaElement(element.clone()));
            }
            // SAFETY: the post targets a thread that pumps messages until the handler is
            // removed.
            let _ = unsafe {
                PostThreadMessageW(self.thread, WM_UIA_LIVE_REGION, WPARAM(0), LPARAM(0))
            };
            Ok(())
        }
    }

    /// Records the live regions UIA reported, each with what it holds, on the tracer thread.
    fn record_uia_live_regions() {
        let elements: Vec<MtaElement> = UIA_LIVE_REGIONS
            .lock()
            .map(|mut queue| queue.drain(..).collect())
            .unwrap_or_default();
        for element in elements {
            STATE.with(|state| {
                let mut state = state.borrow_mut();
                let Some(state) = state.as_mut() else { return };
                let Some(region) = state.uia.as_ref().and_then(|uia| {
                    crate::uia_snapshot::capture::element_subtree(uia, &element.0, 2)
                }) else {
                    return;
                };
                let event = TraceEvent {
                    t_ms: now_ms(),
                    event: "uia_live_region".to_owned(),
                    win_event: UIA_LIVE_REGION_EVENT,
                    hwnd: 0,
                    id_object: 0,
                    id_child: 0,
                    window_class: None,
                    window_style: None,
                    focus_window_class: None,
                    menu_bar: Vec::new(),
                    status_bar: Vec::new(),
                    window_title: None,
                    process: None,
                    object: None,
                    focus_chain: Vec::new(),
                    focus_siblings: Vec::new(),
                    focus_subtree: Some(region),
                    list_position: None,
                    column_headers: Vec::new(),
                    text_context: None,
                    region: None,
                    page: None,
                    top_window_title: None,
                    top_window_class: None,
                    virtual_desktop: None,
                    dialog_lines: Vec::new(),
                    dialog_text: Vec::new(),
                    dialog_page: None,
                    group_box: None,
                    default_button: None,
                    dialog_tree: None,
                    notification: None,
                };
                (state.sink)(event);
                state.count += 1;
            });
        }
    }

    /// Records one event through the sink, with what the engine needs read now. `element`
    /// is the focused element when the event names it (UIA); otherwise UIA's focus is read.
    fn record(
        name: &'static str,
        win_event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        element: Option<&IUIAutomationElement>,
    ) {
        record_at(
            now_ms(),
            name,
            win_event,
            hwnd,
            id_object,
            id_child,
            element,
        );
    }

    /// [`record`] for an event raised at `t_ms`.
    fn record_at(
        t_ms: u128,
        name: &'static str,
        win_event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        element: Option<&IUIAutomationElement>,
    ) {
        let (title, class) = if hwnd.is_invalid() {
            (String::new(), String::new())
        } else {
            window_text(hwnd)
        };
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let Some(state) = state.as_mut() else { return };
            // A change to the focused control (check box toggled, tree item expanded, combo
            // box value) is spoken with the same context as a focus landing on it.
            let focused = matches!(name, "focus" | "uia_focus");
            let changes_focus_context =
                matches!(name, "state_change" | "value_change" | "selection");
            // Live, every cross-process read on this thread delays the events behind it: drop
            // what the engine never uses (name and description changes, alerts) and changes
            // in other applications (the taskbar, the tray clock) before reading anything.
            // The window in front renaming itself is kept: its title is read anew.
            let foreground_renamed = name == "name_change" && id_object == super::OBJID_WINDOW && {
                use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                // SAFETY: plain Win32 call with no preconditions.
                let front = unsafe { GetForegroundWindow() };
                front == hwnd
            };
            // A focus in an application not in front is not the user's: Windows Settings,
            // running behind Chrome, reported its search box focused while the page kept the
            // focus, and JAWS said nothing of it (221, old-pc, 2026-10-18).
            if name == "focus" && in_background_process(hwnd) {
                return;
            }
            // Nor are its changes recorded: nothing replays them, and the taskbar's news and
            // weather widget put the reference machine's location into traces (its MSN
            // links, old-pc, 2026-10-18).
            let change = matches!(
                name,
                "name_change"
                    | "description_change"
                    | "value_change"
                    | "state_change"
                    | "selection"
                    | "selection_add"
                    | "selection_remove"
                    | "selection_within"
                    | "text_selection_change"
            );
            if change && in_background_process(hwnd) {
                return;
            }
            // The taskbar's news and weather widget is never recorded, even when Explorer is
            // in front: it put the weather where the reference machine is into a trace (its
            // title "13°C ...", 504, old-pc, 2026-10-19).
            if class == super::NEWS_AND_WEATHER_CLASS {
                return;
            }
            // A file dialog's navigation pane names the account's pinned and recent folders as
            // it fills its tree (Quick access: 524, round 38); only the focus moving into it is
            // recorded.
            if name != "focus"
                && class.eq_ignore_ascii_case("SysTreeView32")
                && in_namespace_tree(hwnd)
            {
                return;
            }
            if state.live
                && ((!LIVE_EVENTS.contains(&name) && !foreground_renamed)
                    || (changes_focus_context && !in_foreground_process(hwnd)))
            {
                return;
            }
            // A live region outside a Chromium page is read from UIA's own event
            // ([`UiaLiveRegionHandler`]); this one has nothing to read, and its MSAA object took
            // 970 ms to answer as Windows Settings opened (dev box, 2026-10-17).
            if name == "live_region_change" && class != "Chrome_RenderWidgetHostHWND" {
                return;
            }
            // Each read's end, in milliseconds since the event arrived, for the debug log when
            // an event holds the thread up (a UIA call waiting out its timeout).
            let clock = std::time::Instant::now();
            let mut stages: Vec<(&str, u128)> = Vec::new();
            // The focused element is asked for once: chain and context are read around the
            // same element. A second GetFocusedElement right after a window switch could
            // block for seconds while UIA's focus settled (7 s into Windows 11 Notepad, dev
            // box), and could answer with another element than the chain's.
            let uia_named = matches!(name, "uia_focus" | "uia_selected");
            let reads_focus =
                focused || name == "foreground" || changes_focus_context || name == "uia_selected";
            let asked = match (element, state.uia.as_ref()) {
                (None, Some(uia)) if reads_focus => focused_element(uia),
                _ => None,
            };
            let element = element.or(asked.as_ref());
            // The same element named by focus events right after one another is read once:
            // Windows Terminal reports one focus five times, once from each of its windows
            // (dev box, 2026-10-15). Any other focus between them makes the next one new. An
            // event about a child object (a WinForms menu item, while UIA still names the
            // control behind the menu [obs, 302]) is always read.
            let window = hwnd.0 as isize;
            if focused && !uia_named {
                if let (Some(uia), Some(element)) = (state.uia.as_ref(), element) {
                    let repeated = id_child == CHILDID_SELF
                        && state
                            .last_object_focus
                            .as_ref()
                            .is_some_and(|last| last.names(uia, element, REPEATED_FOCUS));
                    let seen = SeenFocus {
                        element: element.clone(),
                        at: std::time::Instant::now(),
                        window,
                    };
                    state.last_object_focus = (id_child == CHILDID_SELF).then(|| seen.clone());
                    state.last_focus_read = Some(seen);
                    if repeated {
                        tracing::debug!(event = name, "repeated focus not read");
                        return;
                    }
                }
            }
            // A list's selection and value change for the item just focused (a list view
            // reports both after each arrow key) say nothing the focus did not: 70 ms each,
            // per key (dev box, 2026-10-15). A state change (a check box toggled) is always
            // read.
            if matches!(name, "selection" | "value_change") {
                if let (Some(uia), Some(element)) = (state.uia.as_ref(), element) {
                    let echo = state.last_focus_read.as_ref().is_some_and(|last| {
                        last.window == window && last.names(uia, element, FOCUS_ECHO)
                    });
                    if echo {
                        tracing::debug!(event = name, "focus echo not read");
                        return;
                    }
                }
            }
            stages.push(("element", clock.elapsed().as_millis()));
            // The focused element's properties are fetched once, for the chain and the context.
            let element = match (state.uia.as_ref(), element) {
                (Some(uia), Some(element)) if reads_focus => with_properties(uia, element),
                _ => None,
            };
            stages.push(("properties", clock.elapsed().as_millis()));
            let element = element.as_ref();
            let focus = match (state.uia.as_ref(), element) {
                (Some(uia), Some(element)) if reads_focus => focus_chain_from(uia, element),
                _ => Vec::new(),
            };
            stages.push(("chain", clock.elapsed().as_millis()));
            let list_position = if focused || changes_focus_context {
                win32_list_position(hwnd, &class).or_else(|| {
                    let focus_window = keyboard_focus_window();
                    (focus_window != hwnd)
                        .then(|| win32_list_position(focus_window, &window_text(focus_window).1))
                        .flatten()
                })
            } else {
                None
            };
            stages.push(("position", clock.elapsed().as_millis()));
            // Siblings give a position when nothing else does, and a dialog box's text and
            // buttons. Reading them in a long list took 2 s per item (dev box, 2026-10-15),
            // where the list already gives the position.
            let needs_siblings =
                in_dialog_box(&focus) || (list_position.is_none() && !reports_position(&focus));
            let (focus_siblings, focus_subtree) = match (name, state.uia.as_ref()) {
                // A focus UIA names (the task switcher's items) is read by name alone: on the
                // switcher's first opening its tree answers slowly, and reading around the item
                // held the thread for seconds while the next items went by (dev box).
                (_, Some(_)) if uia_named => (Vec::new(), None),
                (_, Some(uia)) if focused => match element {
                    Some(element) => focus_context_of(uia, element, needs_siblings),
                    None => (Vec::new(), None),
                },
                (_, Some(uia)) if changes_focus_context => (Vec::new(), focus_subtree_of(uia)),
                _ => (Vec::new(), None),
            };
            let column_headers = match (state.uia.as_ref(), element) {
                (Some(uia), Some(element)) if focused && !uia_named => {
                    column_headers_of(uia, element)
                }
                _ => Vec::new(),
            };
            stages.push(("context", clock.elapsed().as_millis()));
            // The window is the one in front now, when the focus chain was read: an event's
            // own window can have lost the foreground meanwhile (the task switcher's last
            // events arrive after the switch back, seen live on the dev box).
            let (top_window_title, top_window_class, virtual_desktop) =
                if focused || name == "foreground" {
                    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                    // SAFETY: plain Win32 call with no preconditions.
                    top_window(unsafe { GetForegroundWindow() })
                } else {
                    (None, None, None)
                };
            let (dialog, dialog_tree, dialog_page, group_box, default_button) =
                if focused && top_window_class.as_deref() == Some("#32770") {
                    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                    // SAFETY: plain Win32 call with no preconditions.
                    let window = unsafe { GetForegroundWindow() };
                    (
                        crate::screen_lines::dialog_layout(window),
                        // Say Window's controls: the page the focus is on, or the dialog.
                        state.uia.as_ref().and_then(|uia| {
                            let page =
                                crate::screen_lines::page_window(keyboard_focus_window(), window);
                            window_subtree(uia, page.unwrap_or(window))
                                .map(super::without_nested_tree_items)
                        }),
                        crate::screen_lines::page_of(keyboard_focus_window(), window),
                        crate::screen_lines::group_box_of(keyboard_focus_window(), window),
                        crate::screen_lines::default_button_of(window),
                    )
                } else {
                    (
                        crate::screen_lines::DialogLayout::default(),
                        None,
                        None,
                        None,
                        None,
                    )
                };
            stages.push(("window", clock.elapsed().as_millis()));
            let (region, page) = if name == "live_region_change" {
                // Live, the page after the change is captured by the listener's page worker.
                chromium_live_region(hwnd, &class, id_child, !state.live)
            } else {
                (None, None)
            };
            // Live, after an edit key the actions worker reads the field once it has settled.
            // Reading here too would race it with a half-updated state (a selection still
            // being extended), and whichever arrived first would be spoken.
            let caret_moved = matches!(name, "caret" | "text_selection_change");
            let text_context = if (focused || changes_focus_context || caret_moved)
                && !uia_named
                && !(state.live && caret_moved)
            {
                // A browser's own controls (its address bar) are read as an application's: the
                // page names a focus of its own, which is not the one typed in.
                let in_page = focus.iter().any(|n| n.control_type == super::UIA_DOCUMENT);
                let read = || {
                    state.uia.as_ref().and_then(|uia| {
                        if in_page {
                            crate::text_context::read_focused(uia)
                        } else {
                            crate::text_context::read_focused_outside_page(uia)
                        }
                    })
                };
                let first = read();
                // Chromium fires the caret event before its IA2 caret offset moves: a trace
                // reads the field again once it has settled (live, the actions worker polls
                // for the change after the key instead).
                if !state.live && matches!(name, "caret" | "text_selection_change") {
                    std::thread::sleep(std::time::Duration::from_millis(40));
                    read().or(first)
                } else {
                    first
                }
            } else {
                None
            };
            stages.push(("text", clock.elapsed().as_millis()));
            // A UIA focus has no MSAA object: the focused item is UIA's.
            let object = (!uia_named)
                .then(|| msaa_object(hwnd, id_object, id_child))
                .flatten();
            stages.push(("msaa", clock.elapsed().as_millis()));
            if clock.elapsed() > SLOW_EVENT {
                tracing::debug!(event = name, %class, ?stages, "slow event read");
            }
            // Every read's time, for `okur latency`.
            tracing::debug!(
                event = name,
                us = clock.elapsed().as_micros(),
                ?stages,
                "event read"
            );
            // The window with the keyboard focus, which a UIA focus event does not name (it
            // reports the top-level window: 316/s3 after Alt+Tab).
            let focus_window = focused.then(keyboard_focus_window);
            let window_style = focus_window.map(|w| {
                use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongW, GWL_STYLE};
                // SAFETY: plain Win32 call; a window that is gone gives 0.
                unsafe { GetWindowLongW(w, GWL_STYLE) as u32 }
            });
            let focus_window_class = focus_window
                .map(|w| window_text(w).1)
                .filter(|c| !c.is_empty());
            let (menu_bar, status_bar) = if focused {
                crate::screen_lines::window_bars(hwnd)
            } else {
                (Vec::new(), Vec::new())
            };
            let event = TraceEvent {
                t_ms,
                event: name.to_owned(),
                win_event,
                hwnd: hwnd.0 as isize,
                id_object,
                id_child,
                window_class: Some(class).filter(|s| !s.is_empty()),
                window_style,
                focus_window_class,
                menu_bar,
                status_bar,
                window_title: Some(title).filter(|s| !s.is_empty()),
                process: process_name(hwnd),
                object,
                focus_chain: focus,
                focus_siblings,
                focus_subtree,
                list_position,
                column_headers,
                text_context,
                region,
                page,
                top_window_title,
                top_window_class,
                virtual_desktop,
                dialog_lines: dialog.rows,
                dialog_text: dialog.text,
                dialog_page,
                group_box,
                default_button,
                dialog_tree,
                notification: None,
            };
            (state.sink)(event);
            state.count += 1;
        });
    }

    /// Records accessibility events to `out` (JSON Lines) until `stop_file` exists or
    /// `max_ms` elapses. Returns the number of events recorded. With `page_changes`, the page
    /// in front's objects shown, hidden and reordered are recorded too, as `page_change`
    /// events with their object ids and nothing read.
    pub fn run_trace(
        out: &std::path::Path,
        stop_file: Option<&std::path::Path>,
        max_ms: u64,
        page_changes: bool,
    ) -> Result<usize, PlatformError> {
        let io_err = |e: std::io::Error| PlatformError::EventListenerError(format!("{out:?}: {e}"));
        let mut file = std::fs::File::create(out).map_err(io_err)?;
        let header = TraceHeader {
            trace_format_version: TRACE_FORMAT_VERSION,
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
            host: std::env::var("COMPUTERNAME").unwrap_or_default(),
            started_unix_ms: now_ms(),
        };
        let header = serde_json::to_string(&header)
            .map_err(|e| PlatformError::EventListenerError(e.to_string()))?;
        file.write_all(format!("{header}\n").as_bytes())
            .map_err(io_err)?;

        // A trace goes into the public corpus: no one's e-mail address goes with it.
        let sink = move |event: TraceEvent| {
            if let Ok(line) = serde_json::to_string(&event) {
                let mut line = crate::privacy::redact_addresses(&line);
                line.push('\n');
                let _ = file.write_all(line.as_bytes());
            }
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(max_ms);
        Ok(run_hooks(Box::new(sink), None, page_changes, || {
            std::time::Instant::now() < deadline && !stop_file.is_some_and(|p| p.exists())
        }))
    }

    /// Delivers accessibility events to `sink` as they happen, with the same capture as
    /// [`run_trace`], until `stop` is set. This is the live screen reader's event source,
    /// so the engine consumes exactly the event stream the harness records and replays.
    /// `page_changed` is told of each change to the page in front outside live regions.
    pub fn run_live(
        sink: Box<dyn FnMut(TraceEvent)>,
        page_changed: Box<dyn FnMut(PageChange)>,
        stop: &std::sync::atomic::AtomicBool,
    ) -> usize {
        run_hooks(sink, Some(page_changed), false, || {
            !stop.load(std::sync::atomic::Ordering::Relaxed)
        })
    }

    /// Installs the WinEvent hooks on this thread, pumps messages while `keep_running`,
    /// and returns the number of events delivered. Live (with `page_changed`), page changes
    /// are hooked too.
    fn run_hooks(
        sink: Box<dyn FnMut(TraceEvent)>,
        page_changed: Option<Box<dyn FnMut(PageChange)>>,
        record_page_changes: bool,
        keep_running: impl Fn() -> bool,
    ) -> usize {
        let live = page_changed.is_some();
        // SAFETY: COM for UIA queries on this thread, which also pumps the hook's messages.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        STATE.with(|s| {
            *s.borrow_mut() = Some(TraceState {
                sink,
                uia: automation().ok(),
                count: 0,
                live,
                last_object_focus: None,
                last_focus_read: None,
                page_changed,
                page_changed_at: None,
                page_changes_dropped: 0,
                record_page_changes,
            })
        });

        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        // SAFETY: out-of-context hooks with a valid callback; unhooked before returning.
        let hooks = unsafe {
            [
                SetWinEventHook(0x0002, 0x0011, None, Some(on_event), 0, 0, flags),
                SetWinEventHook(0x8005, 0x8019, None, Some(on_event), 0, 0, flags),
                // EVENT_OBJECT_SHOW to EVENT_OBJECT_REORDER.
                if live || record_page_changes {
                    SetWinEventHook(0x8002, 0x8004, None, Some(on_page_change), 0, 0, flags)
                } else {
                    HWINEVENTHOOK::default()
                },
                // IA2's own events: text changed in place (a React time stamp) raises no show,
                // hide or reorder, only IA2_EVENT_TEXT_REMOVED and _INSERTED on the block holding
                // it (old-pc, 2026-10-18). Live takes those two; a trace of page changes takes
                // all of IA2's (IA2_EVENT_ACTION_CHANGED to IA2_EVENT_ROLE_CHANGED).
                if record_page_changes {
                    SetWinEventHook(0x0101, 0x0123, None, Some(on_page_change), 0, 0, flags)
                } else if live {
                    SetWinEventHook(
                        IA2_EVENT_TEXT_INSERTED,
                        IA2_EVENT_TEXT_REMOVED,
                        None,
                        Some(on_page_change),
                        0,
                        0,
                        flags,
                    )
                } else {
                    HWINEVENTHOOK::default()
                },
            ]
        };

        // SAFETY: plain Win32 call with no preconditions.
        let thread = unsafe { GetCurrentThreadId() };
        let uia_focus: IUIAutomationFocusChangedEventHandler = UiaFocusHandler { thread }.into();
        let uia = STATE.with(|s| s.borrow().as_ref().and_then(|s| s.uia.clone()));
        let uia_focus_added = uia.as_ref().is_some_and(|uia| {
            // SAFETY: a system-wide UIA handler, removed before this thread stops pumping.
            unsafe { uia.AddFocusChangedEventHandler(None, &uia_focus) }
                .inspect_err(|e| tracing::warn!("UIA focus handler not added: {e}"))
                .is_ok()
        });
        let uia_selected: IUIAutomationEventHandler = UiaSelectedHandler { thread }.into();
        // SAFETY: UIA client query on a valid automation object.
        let root = uia
            .as_ref()
            .and_then(|uia| unsafe { uia.GetRootElement() }.ok());
        let uia_selected_added = match (uia.as_ref(), root.as_ref()) {
            // SAFETY: a desktop-wide UIA handler, removed before this thread stops pumping.
            (Some(uia), Some(root)) => unsafe {
                uia.AddAutomationEventHandler(
                    UIA_SelectionItem_ElementSelectedEventId,
                    root,
                    TreeScope_Subtree,
                    None,
                    &uia_selected,
                )
            }
            .inspect_err(|e| tracing::warn!("UIA selection handler not added: {e}"))
            .is_ok(),
            _ => false,
        };
        let uia_live_region: IUIAutomationEventHandler = UiaLiveRegionHandler { thread }.into();
        let uia_live_region_added = match (uia.as_ref(), root.as_ref()) {
            // SAFETY: a desktop-wide UIA handler, removed before this thread stops pumping.
            (Some(uia), Some(root)) => unsafe {
                uia.AddAutomationEventHandler(
                    UIA_LiveRegionChangedEventId,
                    root,
                    TreeScope_Subtree,
                    None,
                    &uia_live_region,
                )
            }
            .inspect_err(|e| tracing::warn!("UIA live region handler not added: {e}"))
            .is_ok(),
            _ => false,
        };
        let uia_notification: IUIAutomationNotificationEventHandler =
            UiaNotificationHandler { thread }.into();
        let uia5 = uia.as_ref().and_then(|uia| {
            use windows::core::Interface;
            uia.cast::<windows::Win32::UI::Accessibility::IUIAutomation5>()
                .ok()
        });
        let uia_notification_added = match (uia5.as_ref(), root.as_ref()) {
            // SAFETY: a desktop-wide UIA handler, removed before this thread stops pumping.
            (Some(uia5), Some(root)) => unsafe {
                uia5.AddNotificationEventHandler(root, TreeScope_Subtree, None, &uia_notification)
            }
            .inspect_err(|e| tracing::warn!("UIA notification handler not added: {e}"))
            .is_ok(),
            _ => false,
        };
        // Where focus is already, as JAWS reads it when it starts: no event will report it.
        record_current_focus();

        let mut msg = MSG::default();
        while keep_running() {
            // SAFETY: standard message pump; waits at most 50 ms between stop checks.
            unsafe {
                let _ = MsgWaitForMultipleObjects(None, false, 50, QS_ALLINPUT);
                let mut uia_focused = false;
                let mut uia_selected_queued = false;
                let mut uia_notified = false;
                let mut uia_live_region_queued = false;
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.hwnd.is_invalid() && msg.message == WM_UIA_FOCUS {
                        // Several focus changes queued meanwhile: the focus now is read once.
                        uia_focused = true;
                        continue;
                    }
                    if msg.hwnd.is_invalid() && msg.message == WM_UIA_SELECTED {
                        uia_selected_queued = true;
                        continue;
                    }
                    if msg.hwnd.is_invalid() && msg.message == WM_UIA_NOTIFICATION {
                        uia_notified = true;
                        continue;
                    }
                    if msg.hwnd.is_invalid() && msg.message == WM_UIA_LIVE_REGION {
                        uia_live_region_queued = true;
                        continue;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                if uia_focused {
                    record_uia_focus();
                }
                if uia_selected_queued {
                    record_uia_selected();
                }
                if uia_notified {
                    record_uia_notifications();
                }
                if uia_live_region_queued {
                    record_uia_live_regions();
                }
            }
        }
        if let (Some(uia), Some(root), true) = (uia.as_ref(), root.as_ref(), uia_live_region_added)
        {
            // SAFETY: removes the handler added above.
            let _ = unsafe {
                uia.RemoveAutomationEventHandler(
                    UIA_LiveRegionChangedEventId,
                    root,
                    &uia_live_region,
                )
            };
        }
        if let (Some(uia5), Some(root), true) =
            (uia5.as_ref(), root.as_ref(), uia_notification_added)
        {
            // SAFETY: removes the handler added above.
            let _ = unsafe { uia5.RemoveNotificationEventHandler(root, &uia_notification) };
        }
        if let (Some(uia), Some(root), true) = (uia.as_ref(), root.as_ref(), uia_selected_added) {
            // SAFETY: removes the handler added above.
            let _ = unsafe {
                uia.RemoveAutomationEventHandler(
                    UIA_SelectionItem_ElementSelectedEventId,
                    root,
                    &uia_selected,
                )
            };
        }

        if let (Some(uia), true) = (uia.as_ref(), uia_focus_added) {
            // SAFETY: removes the handler added above.
            let _ = unsafe { uia.RemoveFocusChangedEventHandler(&uia_focus) };
        }
        for hook in hooks {
            if !hook.is_invalid() {
                // SAFETY: hooks installed above.
                let _ = unsafe { UnhookWinEvent(hook) };
            }
        }
        STATE.with(|s| s.borrow_mut().take().map(|s| s.count).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RADIO: i32 = 50013;
    const GROUP: i32 = 50026;
    const TEXT: i32 = 50020;

    fn node(control_type: i32, name: &str, focused: bool) -> UiaNode {
        UiaNode {
            control_type,
            name: Some(name.to_owned()),
            has_keyboard_focus: focused,
            is_enabled: true,
            ..UiaNode::default()
        }
    }

    /// A dialog's tree keeps its tree views' own items and drops what is inside them: a file
    /// dialog's Quick access holds the account's recent folders (525, round 35).
    #[test]
    fn a_dialog_tree_keeps_no_nested_tree_items() {
        let mut quick_access = node(UIA_TREEITEM, "Quick access", false);
        quick_access.children = vec![node(UIA_TREEITEM, "recent folder", false)];
        let mut tree = node(50023, "Tree View", false);
        tree.children = vec![quick_access, node(UIA_TREEITEM, "This PC", false)];
        let mut dialog = node(50032, "Open", false);
        dialog.children = vec![tree, node(50000, "Open", false)];
        let pruned = without_nested_tree_items(dialog);
        let names = |n: &UiaNode| -> Vec<String> {
            fn walk(n: &UiaNode, out: &mut Vec<String>) {
                out.extend(n.name.clone());
                for c in &n.children {
                    walk(c, out);
                }
            }
            let mut out = Vec::new();
            walk(n, &mut out);
            out
        };
        assert_eq!(
            names(&pruned),
            ["Open", "Tree View", "Quick access", "This PC", "Open"]
        );
    }

    /// A window's scroll bars are not traced: a scrolling list's state changes held the Run
    /// dialog's focus back 5 s (319). The window's own changes are.
    #[test]
    fn scroll_bar_events_are_ignored() {
        assert_eq!(event_name(0x800A, -5), None);
        assert_eq!(event_name(0x800A, -6), None);
        assert_eq!(event_name(0x800A, -4), Some("state_change"));
        assert_eq!(event_name(0x800C, 0), Some("name_change"));
    }

    #[test]
    fn siblings_are_needed_in_a_dialog_box_or_without_a_reported_position() {
        let window = |class: &str| UiaNode {
            class_name: Some(class.to_owned()),
            ..node(UIA_WINDOW, "Window", false)
        };
        let item = node(RADIO, "Option", true);
        let dialog = [window("#32770"), item.clone()];
        let main = [window("WindowsForms10.Window.8.app"), item.clone()];
        assert!(in_dialog_box(&dialog));
        assert!(!in_dialog_box(&main));

        assert!(!reports_position(&main));
        let placed = UiaNode {
            position_in_set: Some(2),
            size_of_set: Some(5),
            ..item
        };
        assert!(reports_position(&[window("Main"), placed]));
    }

    /// A live region's politeness and atomicity come from its ARIA properties, as Chromium
    /// gives them: Google Docs' announcements are assertive and atomic (dev box, 2026-10-16).
    #[test]
    fn a_live_region_keeps_its_aria_properties() {
        let region = |attributes: &[(&str, &str)]| crate::ia2_snapshot::Ia2Node {
            attributes: attributes
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            children: vec![crate::ia2_snapshot::Ia2Node {
                msaa_role: Some(0x29),
                name: Some("o".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let live = |region| {
            let mut event = focus_event(Vec::new());
            event.event = "live_region_change".to_owned();
            event.region = Some(region);
            match live_region_events(&event).first() {
                Some(okur_core::AccessibilityEvent::LiveRegion(live)) => Some(live.clone()),
                _ => None,
            }
        };
        let docs = live(region(&[
            ("id", "docs-aria-speakable"),
            ("live", "assertive"),
            ("atomic", "true"),
        ]));
        assert_eq!(
            docs.as_ref()
                .map(|l| (l.politeness, l.atomic, l.text.as_str())),
            Some((okur_core::Politeness::Assertive, true, "o"))
        );
        // A region inside a live region takes the container's properties.
        let inner = live(region(&[
            ("container-live", "polite"),
            ("container-atomic", "false"),
        ]));
        assert_eq!(
            inner.map(|l| (l.politeness, l.atomic)),
            Some((okur_core::Politeness::Polite, false))
        );
    }

    /// Windows Settings' Display page (UIA chains read on the dev box, 2026-10-16): the
    /// groups a control is directly in, outermost first, a text counting as one; none when
    /// they are inside a control (the slider in the "Show more settings" button).
    #[test]
    fn a_controls_groups_are_the_ones_it_nests_in() {
        use okur_core::GroupEntry;
        const UIA_GROUP: i32 = 50026;
        const UIA_PANE: i32 = 50033;
        const UIA_SLIDER: i32 = 50015;
        const UIA_LIST: i32 = 50008;
        const UIA_CUSTOM: i32 = 50025;
        let groups = |names: &[&str]| -> Vec<GroupEntry> {
            names.iter().map(|n| GroupEntry::group(*n)).collect()
        };
        let toggle = [
            node(UIA_PANE, "", false),
            node(UIA_GROUP, "Brightness & color", false),
            node(UIA_GROUP, "Night light", false),
            node(UIA_BUTTON, "Night light", true),
        ];
        assert_eq!(
            nested_groups(&toggle),
            (groups(&["Brightness & color", "Night light"]), false)
        );
        let link = [
            node(UIA_PANE, "", false),
            node(UIA_GROUP, "Brightness & color", false),
            node(UIA_GROUP, "HDR", false),
            node(UIA_TEXT, "More about HDR ", false),
            node(50005, "More about HDR", true),
        ];
        assert_eq!(
            nested_groups(&link),
            (
                groups(&["Brightness & color", "HDR", "More about HDR"]),
                false
            )
        );
        // Sound's output device: through its list and the scroll panes, up to the window's
        // navigation view.
        let device = [
            node(UIA_CUSTOM, "", false),
            node(UIA_PANE, "", false),
            node(UIA_GROUP, "Output", false),
            node(UIA_GROUP, "Choose where to play sound.", false),
            node(UIA_LIST, "", false),
            node(UIA_PANE, "", false),
            node(UIA_GROUP, "Hoparlör", false),
            node(UIA_BUTTON, "Hoparlör", true),
        ];
        let mut expected = groups(&["Output", "Choose where to play sound."]);
        expected.push(GroupEntry::list(""));
        expected.extend(groups(&["Hoparlör"]));
        assert_eq!(nested_groups(&device), (expected, false));
        let slider = [
            node(UIA_GROUP, "Brightness, Brightness", false),
            node(UIA_BUTTON, "Show more settings", false),
            node(UIA_GROUP, "Brightness", false),
            node(UIA_SLIDER, "Brightness", true),
        ];
        assert_eq!(nested_groups(&slider), (Vec::new(), true));
        // Sound's "Output devices" link, in its list item: the item's groups.
        let link_in_item = [
            node(UIA_GROUP, "Advanced", false),
            node(UIA_LIST, "", false),
            node(UIA_LISTITEM, "Output devices", false),
            node(50005, "Output devices", true),
        ];
        let mut expected = groups(&["Advanced"]);
        expected.push(GroupEntry::list(""));
        assert_eq!(nested_groups(&link_in_item), (expected, true));
        // Network's Wi-Fi switch, in a "Wi-Fi" group in its list item.
        let switch_in_item = [
            node(UIA_LIST, "", false),
            node(UIA_LISTITEM, "Wi-Fi", false),
            node(UIA_GROUP, "Wi-Fi", false),
            node(UIA_BUTTON, "Wi-Fi", true),
        ];
        assert_eq!(
            nested_groups(&switch_in_item),
            (vec![GroupEntry::list(""), GroupEntry::group("Wi-Fi")], true)
        );
        // Notifications' app switch, in the app's button, its group and the list: the list's
        // entry, as an item.
        let switch_in_entry = [
            node(
                UIA_GROUP,
                "Notifications from apps and other senders",
                false,
            ),
            node(UIA_LIST, "", false),
            node(UIA_PANE, "", false),
            node(UIA_GROUP, "Claude", false),
            node(UIA_BUTTON, "Claude", false),
            node(UIA_BUTTON, "Claude", true),
        ];
        let mut expected = groups(&["Notifications from apps and other senders"]);
        expected.push(GroupEntry::list(""));
        expected.extend(groups(&["Claude"]));
        assert_eq!(nested_groups(&switch_in_entry), (expected, true));
    }

    /// A native live region says the texts it holds, then its name; a text region its name.
    #[test]
    fn a_native_live_region_says_its_texts_then_its_name() {
        // Windows Settings' brightness expander, traced on the dev box (2026-10-17).
        const UIA_GROUP: i32 = 50026;
        const UIA_SLIDER: i32 = 50015;
        let mut group = node(UIA_GROUP, "Brightness", false);
        group.children = vec![
            node(UIA_TEXT, "Brightness", false),
            node(
                UIA_TEXT,
                "Change brightness for the built-in display",
                false,
            ),
            node(UIA_SLIDER, "Brightness", false),
        ];
        let mut expander = node(UIA_BUTTON, "Show more settings", false);
        expander.live_setting = Some(1);
        expander.children = vec![group];
        let live = uia_live_region_event(&expander).map(|l| (l.text, l.politeness));
        assert_eq!(
            live,
            Some((
                "Brightness Change brightness for the built-in display\nShow more settings"
                    .to_owned(),
                okur_core::Politeness::Polite
            ))
        );
        let mut status = node(UIA_TEXT, "Discoverable as \u{201c}PC\u{201d}", false);
        status.live_setting = Some(2);
        assert_eq!(
            uia_live_region_event(&status).map(|l| (l.text, l.politeness)),
            Some((
                "Discoverable as \u{201c}PC\u{201d}".to_owned(),
                okur_core::Politeness::Assertive
            ))
        );
        status.live_setting = Some(0);
        assert!(uia_live_region_event(&status).is_none());
    }

    /// A closed WinUI combo box holds only its selected item: that is its value, not a
    /// position (Windows Settings' "Scale Combo box | 125% (Recommended)").
    #[test]
    fn a_closed_combo_boxs_only_item_is_its_value() {
        let mut event = focus_event(Vec::new());
        let scale = node(UIA_COMBOBOX, "Scale", true);
        event.focus_chain = vec![node(UIA_WINDOW, "Settings", false), scale.clone()];
        event.focus_subtree = Some(UiaNode {
            children: vec![UiaNode {
                is_selected: Some(true),
                ..node(UIA_LISTITEM, "125% (Recommended)", false)
            }],
            ..scale
        });
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus.map(|f| (f.value, f.item_index)),
            Some((Some("125% (Recommended)".to_owned()), None))
        );
    }

    /// A page's menu item focused through its document (Chrome, Google Docs' menus): the menu
    /// item, as its MSAA focus names it.
    #[test]
    fn a_page_menu_item_focused_through_its_document_is_a_menu_item() {
        let mut event = focus_event(Vec::new());
        event.focus_chain = vec![
            node(UIA_DOCUMENT, "Untitled document - Google Docs", false),
            node(UIA_MENUITEM, "Graphics g", true),
        ];
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus.map(|f| (f.role, f.name)),
            Some((okur_core::Role::MenuItem, Some("Graphics g".to_owned())))
        );
    }

    /// Windows Settings' Power page: its first link is in an InfoBar, an unnamed dialog named
    /// by its first text (JAWS: "Power mode is set automatically ... dialog | More about power
    /// mode Link").
    #[test]
    fn a_link_in_an_info_bar_has_the_bar_among_its_groups() {
        let mut event = focus_event(Vec::new());
        let bar = UiaNode {
            is_dialog: Some(true),
            label: Some("Power mode is set automatically.".to_owned()),
            ..node(50017, "", false)
        };
        event.focus_chain = vec![
            node(UIA_WINDOW, "Settings", false),
            node(50033, "", false),
            bar,
            node(50005, "More about power mode", true),
        ];
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus.map(|f| f.groups),
            Some(vec![okur_core::GroupEntry::dialog(
                "Power mode is set automatically."
            )])
        );
    }

    /// A slider says the value it reports, else its range value, as a percentage from 0 to
    /// 100: Windows Settings' brightness "100", volume "34%" (JAWS on the dev box).
    #[test]
    fn a_sliders_value_is_its_reported_value_or_its_percentage() {
        const UIA_SLIDER: i32 = 50015;
        let slider = |value: Option<&str>, now: f64| {
            let mut event = focus_event(Vec::new());
            event.focus_chain = vec![
                node(UIA_WINDOW, "Settings", false),
                UiaNode {
                    value: value.map(str::to_owned),
                    range_value: Some(now),
                    range_minimum: Some(0.0),
                    range_maximum: Some(100.0),
                    ..node(UIA_SLIDER, "Volume", true)
                },
            ];
            focus_event_from_trace(&event).and_then(|f| f.value)
        };
        assert_eq!(slider(Some("100"), 100.0), Some("100".to_owned()));
        assert_eq!(slider(None, 34.0), Some("34%".to_owned()));
    }

    /// A UIA notification is said as an announcement, and the same one again within two
    /// seconds once (Settings' "Suggestions available" for each letter typed).
    #[test]
    fn a_notification_is_said_once_while_it_repeats() {
        let notify = |t_ms: u128| {
            let mut event = focus_event(Vec::new());
            event.event = "uia_notification".to_owned();
            event.t_ms = t_ms;
            event.notification = Some(TraceNotification {
                text: "Suggestions available".to_owned(),
                processing: 3,
            });
            event
        };
        let mut converter = EventConverter::default();
        let said = |events: Vec<okur_core::AccessibilityEvent>| match events.first() {
            Some(okur_core::AccessibilityEvent::LiveRegion(live)) => Some(live.text.clone()),
            _ => None,
        };
        assert_eq!(
            said(converter.convert(&notify(1_000))),
            Some("Suggestions available".to_owned())
        );
        assert_eq!(said(converter.convert(&notify(1_500))), None);
        assert_eq!(
            said(converter.convert(&notify(4_000))),
            Some("Suggestions available".to_owned())
        );
    }

    /// Windows Settings' breadcrumb is a Button with a heading level: a button to focus, with
    /// the position it reports. A button reporting none gets no position counted among its
    /// siblings (a dialog's buttons, where JAWS says none).
    #[test]
    fn a_native_heading_button_is_a_button() {
        let mut event = focus_event(Vec::new());
        let home = UiaNode {
            heading_level: Some(80051),
            position_in_set: Some(1),
            size_of_set: Some(1),
            ..node(UIA_BUTTON, "Home", true)
        };
        event.focus_chain = vec![node(UIA_WINDOW, "Settings", false), home];
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus.map(|f| (f.role, f.item_index, f.item_count)),
            Some((okur_core::Role::Button, Some(1), Some(1)))
        );

        let ok = node(UIA_BUTTON, "OK", true);
        let cancel = node(UIA_BUTTON, "Cancel", false);
        let mut event = focus_event(vec![ok.clone(), cancel]);
        event.focus_chain = vec![node(UIA_WINDOW, "Dialog", false), ok];
        assert_eq!(
            focus_event_from_trace(&event).and_then(|f| f.item_index),
            None
        );
    }

    /// A page's menu item keeps its place: on the menu bar, its position, whether it opens a
    /// menu (Google Docs' "Help", 9 of 10). A menu item reported through MSAA while UIA has
    /// the focus elsewhere in the page is the page moving on, not a menu.
    #[test]
    fn a_page_menu_item_keeps_its_place() {
        const UIA_MENUBAR: i32 = 50010;
        let mut event = focus_event(Vec::new());
        let help = UiaNode {
            position_in_set: Some(9),
            size_of_set: Some(10),
            aria_properties: Some("haspopup=menu".to_owned()),
            ..node(UIA_MENUITEM, "Help", true)
        };
        event.focus_chain = vec![
            node(UIA_DOCUMENT, "Untitled document - Google Docs", false),
            node(UIA_MENUBAR, "", false),
            help,
        ];
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus
                .as_ref()
                .map(|f| (f.in_document, f.in_menu_bar, f.item_index, f.item_count)),
            Some((true, true, Some(9), Some(10)))
        );
        assert_eq!(focus.and_then(|f| f.states.expanded), Some(false));

        event.window_class = Some("Chrome_RenderWidgetHostHWND".to_owned());
        event.object = Some(TraceObject {
            role: Some(MSAA_MENUITEM),
            name: Some("Help".to_owned()),
            value: None,
            states: 0,
        });
        event.focus_chain = vec![
            node(UIA_DOCUMENT, "Untitled document - Google Docs", false),
            node(UIA_EDIT, "Document content", true),
        ];
        assert_eq!(focus_event_from_trace(&event), None);
    }

    fn focus_event(siblings: Vec<UiaNode>) -> TraceEvent {
        TraceEvent {
            t_ms: 0,
            event: "focus".to_owned(),
            win_event: 0x8005,
            hwnd: 0,
            id_object: -4,
            id_child: 0,
            window_class: None,
            window_style: None,
            focus_window_class: None,
            menu_bar: Vec::new(),
            status_bar: Vec::new(),
            window_title: None,
            process: None,
            object: None,
            focus_chain: vec![node(GROUP, "Plan", false), node(RADIO, "Pro", true)],
            focus_siblings: siblings,
            focus_subtree: None,
            list_position: None,
            column_headers: Vec::new(),
            text_context: None,
            region: None,
            page: None,
            top_window_title: None,
            top_window_class: None,
            virtual_desktop: None,
            dialog_lines: Vec::new(),
            dialog_text: Vec::new(),
            dialog_page: None,
            group_box: None,
            default_button: None,
            dialog_tree: None,
            notification: None,
        }
    }

    /// WinForms radio buttons report no PositionInSet; JAWS still says "2 of 3" [obs, 301/s4].
    #[test]
    fn position_is_counted_among_same_type_siblings() {
        let event = focus_event(vec![
            node(TEXT, "Choose a plan", false),
            node(RADIO, "Basic", false),
            node(RADIO, "Pro", true),
            node(RADIO, "Team", false),
        ]);
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus.map(|f| (f.item_index, f.item_count)),
            Some((Some(2), Some(3)))
        );
    }

    #[test]
    fn no_siblings_means_no_position() {
        let focus = focus_event_from_trace(&focus_event(Vec::new()));
        assert_eq!(focus.map(|f| f.item_index), Some(None));
    }

    fn dialog_node(control_type: i32, name: &str) -> UiaNode {
        UiaNode {
            class_name: Some("#32770".to_owned()),
            ..node(control_type, name, false)
        }
    }

    /// Local Disk (C:) Properties opening, focus on the General page's Label field, as traced
    /// on the dev box: the page is not in the focus chain, and the siblings are the page's.
    fn property_page_event() -> TraceEvent {
        TraceEvent {
            focus_chain: vec![
                dialog_node(UIA_WINDOW, "Local Disk (C:) Properties"),
                node(UIA_EDIT, "Label:", true),
            ],
            focus_siblings: vec![
                node(UIA_EDIT, "Label:", true),
                node(TEXT, "Type:", false),
                node(TEXT, "Local Disk", false),
            ],
            top_window_title: Some("Local Disk (C:) Properties".to_owned()),
            top_window_class: Some("#32770".to_owned()),
            dialog_page: Some("General".to_owned()),
            ..focus_event(Vec::new())
        }
    }

    /// A text that names a control beside it is that control's label, not the dialog's text;
    /// the group box comes from the trace (JAWS on Okur's settings dialog, dev box,
    /// 2026-10-17: "Okur Settings dialog | Speech Synthesizer: Combo box").
    #[test]
    fn a_dialogs_labels_are_not_its_text() {
        let settings = TraceEvent {
            focus_chain: vec![
                dialog_node(UIA_WINDOW, "Okur Settings"),
                node(UIA_COMBOBOX, "Synthesizer:", true),
            ],
            focus_siblings: vec![
                node(TEXT, "Synthesizer:", false),
                node(UIA_COMBOBOX, "Synthesizer:", true),
                node(TEXT, "Voice:", false),
                node(UIA_COMBOBOX, "Voice:", false),
                node(TEXT, "Changes apply when you press OK.", false),
            ],
            group_box: Some("Speech".to_owned()),
            ..focus_event(Vec::new())
        };
        let focus = focus_event_from_trace(&settings);
        assert_eq!(
            focus.as_ref().and_then(|f| f.dialog_text.clone()),
            Some("Changes apply when you press OK.".to_owned())
        );
        assert_eq!(focus.and_then(|f| f.group_box), Some("Speech".to_owned()));
    }

    /// A text that names the control the focus is inside is that control's label, not the
    /// dialog's text: Document Regions' "Regions" names its tree [obs, 274/s1].
    #[test]
    fn the_label_of_the_control_around_the_focus_is_not_the_dialogs_text() {
        const UIA_TREE: i32 = 50023;
        let regions = TraceEvent {
            focus_chain: vec![
                dialog_node(UIA_WINDOW, "Document Regions"),
                node(UIA_TREE, "Regions", false),
                node(UIA_TREEITEM, "Banner", true),
            ],
            focus_siblings: vec![node(UIA_TREEITEM, "Main", false)],
            dialog_text: vec!["Regions".to_owned()],
            ..focus_event(Vec::new())
        };
        let focus = focus_event_from_trace(&regions);
        assert_eq!(focus.and_then(|f| f.dialog_text), Some(String::new()));
    }

    /// A message box's text is beside its buttons; a property page's labels are the page's,
    /// not the dialog's (JAWS 2025 on the dev box: "Local Disk (C:) Properties dialog",
    /// "General page", "Label: Edit").
    #[test]
    fn a_property_pages_labels_are_not_the_dialogs_text() {
        let message_box = TraceEvent {
            focus_chain: vec![
                dialog_node(UIA_WINDOW, "SR Dialog Fixture"),
                node(UIA_BUTTON, "Yes", true),
            ],
            focus_siblings: vec![
                node(TEXT, "Save changes to the document?", false),
                node(UIA_BUTTON, "Yes", true),
            ],
            ..focus_event(Vec::new())
        };
        assert_eq!(
            focus_event_from_trace(&message_box).and_then(|f| f.dialog_text),
            Some("Save changes to the document?".to_owned())
        );
        assert_eq!(
            focus_event_from_trace(&property_page_event()).and_then(|f| f.dialog_text),
            Some(String::new())
        );
    }

    /// JAWS 2025 names the page a property sheet opens on (dev box, 2026-10-15).
    #[test]
    fn a_property_sheet_opens_on_its_page() {
        use okur_core::{AomNode, MockSpeechOutput, Role, ScreenReaderEngine, VirtualBuffer};
        let cursor = okur_core::VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(
            0,
            Role::Document,
        )));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        let spoken = focus_event_from_trace(&property_page_event())
            .and_then(|focus| engine.handle_focus_event(&focus))
            .unwrap_or_default();
        // Lines as heard: role marks and spacing aside.
        let lines: Vec<String> = spoken
            .lines()
            .map(|l| {
                l.replace(['\u{1d}', '\u{1e}'], " ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        assert_eq!(
            lines,
            vec![
                "Local Disk (C:) Properties dialog",
                "General page",
                "Label: Edit",
                "Type in text."
            ],
            "{spoken:?}"
        );
    }

    /// The Run dialog's Open field: an edit inside a combo box, whose access key and position
    /// are the combo box's.
    #[test]
    fn an_edit_in_a_combo_box_is_its_field() {
        let combo = UiaNode {
            access_key: Some("Alt+o".to_owned()),
            ..node(UIA_COMBOBOX, "Open:", false)
        };
        let event = TraceEvent {
            focus_chain: vec![combo, node(UIA_EDIT, "Open:", true)],
            focus_siblings: vec![node(UIA_EDIT, "Open:", true)],
            list_position: Some([0, 6]),
            ..focus_event(Vec::new())
        };
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus.as_ref().map(|f| (
                f.container_role.clone(),
                f.access_key.as_deref(),
                f.item_index,
                f.item_count
            )),
            Some((
                Some(okur_core::Role::ComboBox),
                Some("Alt+o"),
                Some(1),
                Some(6)
            ))
        );
    }

    /// Windows search's result, selected while focus stays in the search box.
    #[test]
    fn a_uia_selection_is_a_selected_item() {
        let result = UiaNode {
            position_in_set: Some(1),
            size_of_set: Some(5),
            ..node(
                UIA_LISTITEM,
                "Notepad, App, Press right to view options",
                false,
            )
        };
        let event = TraceEvent {
            event: "uia_selected".to_owned(),
            win_event: 20012,
            focus_chain: vec![result],
            ..focus_event(Vec::new())
        };
        let events = EventConverter::default().convert(&event);
        assert!(
            matches!(events.as_slice(), [okur_core::AccessibilityEvent::Selected(item)]
                if item.item_index == Some(1) && item.item_count == Some(5)),
            "{events:?}"
        );
    }

    /// Focus reported on the browser window while a page button is focused, then the button
    /// expanded with Space (GitHub's "Solutions", dev box): the change is the focused
    /// control's, so it is passed on. Another object's change is not.
    #[test]
    fn a_change_of_the_control_focused_inside_a_window_is_passed_on() {
        const BUTTON: i32 = 50000;
        let object = |name: &str| {
            Some(TraceObject {
                role: None,
                name: Some(name.to_owned()),
                value: None,
                states: 0,
            })
        };
        let solutions = UiaNode {
            expand_collapse_state: Some(1),
            ..node(BUTTON, "Solutions", true)
        };
        let focus = TraceEvent {
            object: object("Page - Google Chrome"),
            focus_chain: vec![solutions.clone()],
            ..focus_event(Vec::new())
        };
        let change = |name: &str| TraceEvent {
            event: "state_change".to_owned(),
            hwnd: 7,
            id_child: -16936,
            object: object(name),
            focus_chain: vec![solutions.clone()],
            ..focus_event(Vec::new())
        };
        let mut converter = EventConverter::default();
        assert_eq!(converter.convert(&focus).len(), 1);
        assert!(
            matches!(
                converter.convert(&change("Solutions")).as_slice(),
                [okur_core::AccessibilityEvent::Change { .. }]
            ),
            "the focused button's change"
        );
        assert!(converter.convert(&change("Pricing")).is_empty());
    }

    /// Down in a select: the value change carries the focus's text, which a select has none
    /// of, and its new choice as its value. It is a value change, not a text change
    /// [obs, 205/s9]; a text field's value change stays a text change.
    #[test]
    fn a_selects_value_change_is_a_change_not_text() {
        const COMBO_BOX: i32 = 50003;
        let select = node(COMBO_BOX, "Home branch", true);
        let object = |value: &str| {
            Some(TraceObject {
                role: Some(46),
                name: Some("Home branch".to_owned()),
                value: Some(value.to_owned()),
                states: 0,
            })
        };
        let focus = TraceEvent {
            object: object("Northside"),
            focus_chain: vec![select.clone()],
            ..focus_event(Vec::new())
        };
        let value_change = |text: &str| TraceEvent {
            event: "value_change".to_owned(),
            object: object("Eastgate"),
            focus_chain: vec![select.clone()],
            text_context: Some(okur_core::TextContext {
                text: text.to_owned(),
                caret: 0,
                selection: None,
                font: None,
                word: None,
            }),
            ..focus_event(Vec::new())
        };
        let mut converter = EventConverter::default();
        assert_eq!(converter.convert(&focus).len(), 1);
        assert!(
            matches!(
                converter.convert(&value_change("")).as_slice(),
                [okur_core::AccessibilityEvent::Change { .. }]
            ),
            "the select's new choice"
        );
        assert!(matches!(
            converter.convert(&value_change("Eastgate")).as_slice(),
            [okur_core::AccessibilityEvent::TextContext(_)]
        ));
    }

    /// Chrome's caption names the profile; its UIA window name does not.
    #[test]
    fn window_is_named_by_its_caption() {
        const WINDOW: i32 = 50032;
        const DOCUMENT: i32 = 50030;
        let event = TraceEvent {
            focus_chain: vec![
                node(WINDOW, "Page - Google Chrome", false),
                node(DOCUMENT, "Page", true),
            ],
            top_window_title: Some("Page - Google Chrome - Profile".to_owned()),
            ..focus_event(Vec::new())
        };
        let focus = focus_event_from_trace(&event).and_then(|f| f.window_title);
        assert_eq!(focus.as_deref(), Some("Page - Google Chrome - Profile"));
        // Older traces: the UIA window's name.
        let older = TraceEvent {
            top_window_title: None,
            top_window_class: None,
            ..event
        };
        let focus = focus_event_from_trace(&older).and_then(|f| f.window_title);
        assert_eq!(focus.as_deref(), Some("Page - Google Chrome"));
    }

    /// The task switcher's items take focus through UIA only (Alt+Tab with Alt held).
    #[test]
    fn uia_focus_is_a_focus_once() {
        const LIST_ITEM: i32 = 50007;
        let switcher = |name: &str| TraceEvent {
            event: "uia_focus".to_owned(),
            win_event: 20005,
            window_class: Some("XamlExplorerHostIslandWindow".to_owned()),
            focus_chain: vec![node(LIST_ITEM, name, true)],
            ..focus_event(Vec::new())
        };
        let mut converter = EventConverter::default();
        let first = converter.convert(&switcher("Untitled - Notepad"));
        assert!(
            matches!(first.as_slice(), [okur_core::AccessibilityEvent::Focus(f)]
                if f.name.as_deref() == Some("Untitled - Notepad")),
            "{first:?}"
        );
        // A focus UIA repeats changes nothing.
        assert!(converter
            .convert(&switcher("Untitled - Notepad"))
            .is_empty());
        assert_eq!(converter.convert(&switcher("Downloads")).len(), 1);
    }
}
