//! Accessibility event traces.
//!
//! `okur-lab trace` records the system-wide accessibility event stream while a JAWS trial runs
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
pub const UIA_DOCUMENT: i32 = 50030;
/// Window classes that host web page content: Chromium (Chrome, Edge, Electron) and Firefox.
const WEB_CONTENT_WINDOW_CLASSES: [&str; 2] = ["Chrome_RenderWidgetHostHWND", "MozillaWindowClass"];
/// The top-level window class of Chromium browsers and Electron apps.
const CHROMIUM_TOP_WINDOW: &str = "Chrome_WidgetWin_1";
/// The window of the Windows 10 taskbar's news and weather widget (its title is the weather).
pub const NEWS_AND_WEATHER_CLASS: &str = "DynamicContent2";
const UIA_TREEITEM: i32 = 50024;
const UIA_TREE: i32 = 50023;
const UIA_LIST: i32 = 50008;
const UIA_TEXT: i32 = 50020;
const UIA_BUTTON: i32 = 50000;
const UIA_EDIT: i32 = 50004;
const UIA_COMBOBOX: i32 = 50003;
const UIA_MENUITEM: i32 = 50011;
const UIA_TOOLBAR: i32 = 50021;
const UIA_HEADER: i32 = 50034;
/// The window class of Win32 dialog boxes (message boxes, common dialogs).
pub const WIN32_DIALOG_CLASS: &str = "#32770";
/// A Win32 popup menu's window.
const WIN32_MENU_CLASS: &str = "#32768";
/// The window kind of focus on the desktop ([`okur_core::FocusEvent::window_kind`]).
pub const DESKTOP_KIND: &str = "Progman";
/// MSAA `ROLE_SYSTEM_MENUITEM`.
const MSAA_MENUITEM: i32 = 0x0C;
/// MSAA `ROLE_SYSTEM_BUTTONMENU`.
const MSAA_BUTTONMENU: i32 = 0x39;
/// MSAA `ROLE_SYSTEM_PROPERTYPAGE`: a NetUI ribbon itself, or one of its tabs' pages.
const MSAA_PROPERTYPAGE: i32 = 0x26;
/// The object id of a window's client area in a WinEvent (`OBJID_CLIENT`).
const OBJID_CLIENT: i32 = -4;
/// UIA's class of a group of Office's ribbon (its "chunk": Word's Font group).
const NETUI_RIBBON_GROUP_CLASS: &str = "NetUIChunk";
/// The class of Office's panes as UI Automation reaches them: the row of ribbon tabs, where its
/// search box is, is the one that is a tab control.
const NETUI_RIBBON_TABS_CLASS: &str = "NetUIPanViewer";

/// Bumped whenever the recorded fields or their meaning change.
pub const TRACE_FORMAT_VERSION: u32 = 10;

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

/// Whether a WinEvent focus had moved on by the time it was read: the keyboard focus was then
/// outside the event's window (`TraceEvent::focus_elsewhere`), and MSAA no longer says its
/// object is focused. File Explorer's list as Tab passes it on the way to a link [obs,
/// 3156/s6]. A window whose child has the focus is not one: Notepad's window coming to the
/// front with the focus in its edit, a dialog with the focus in its list.
fn left_before_read(event: &TraceEvent) -> bool {
    const STATE_FOCUSED: u32 = 0x4;
    event.event == "focus"
        && event.focus_elsewhere
        && event
            .object
            .as_ref()
            .is_some_and(|o| o.states & STATE_FOCUSED == 0)
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
    /// MSAA's keyboard shortcut (`accKeyboardShortcut`): a Win32 dialog's controls give their
    /// access key here only ("Alt+r" in Notepad's Font dialog) [obs, 541/s8].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
}

/// What Office's own UI Automation element says of a NetUI control beside its MSAA object:
/// UIA's focus stops at the ribbon's window, and MSAA has neither the control's ScreenTip title
/// nor its toggle state, which JAWS says: "Grow Font Button | Increase Font Size
/// (Alt+Ctrl+!)", "Format Painter Button not checked" [obs, 1045/s9, s5].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetuiControl {
    /// Its rectangle from MSAA, `[left, top, right, bottom]` in screen pixels: the PC
    /// cursor's place, where UIA gives only the ribbon's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[i32; 4]>,
    /// UIA's help text, its ScreenTip's title: "Increase Font Size (Alt+Ctrl+!)".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_text: Option<String>,
    /// UIA's toggle state, for a button that has one: 0 off, 1 on, 2 neither.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toggle_state: Option<i32>,
    /// UIA's control type (from format 9): a drop-down that is a menu item (Change Case) or a
    /// button (Styles), which MSAA calls alike [obs, 1051/s17].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_type: Option<i32>,
}

/// An MSAA object with its children, as a NetUI window gives them (WordPad's ribbon, Office's).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MsaaNode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub states: u32,
    /// Its value and description, read for a NetUI control's ancestors only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<MsaaNode>,
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
    /// For a focus: the window with the keyboard focus, once the event was read, was neither
    /// the event's window nor inside it: the focus had moved on (File Explorer's list as Tab
    /// passes it, 3156/s6). Recorded from trace format 10.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub focus_elsewhere: bool,
    /// For a focus: the window with the keyboard focus is an up-down control's buddy (an edit
    /// with arrows beside it, a spin box).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub spin_box: bool,
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
    /// The process's executable's product name and version, from its version resource
    /// (Ctrl+Insert+V: "Microsoft® Windows® Operating System Version 10.0.19041.1" [obs,
    /// 513/s4]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_product: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_version: Option<String>,
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
    /// Whether that window's owner is available while it is open: a modal dialog keeps its
    /// owner disabled (false), a modeless one does not (true: Word's Find and Replace, which
    /// Ctrl+Tab leaves for the document [obs, 1065/s6]). None for a window without an owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_window_owner_enabled: Option<bool>,
    /// When the top-level window is the desktop (`Progman`, `WorkerW`): the current virtual
    /// desktop's name, which JAWS gives as the desktop's title ("Desktop 1").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virtual_desktop: Option<String>,
    /// For focus in a dialog box (`#32770`) in front: its text by screen rows
    /// (okur-windows' `screen_lines`), for the Virtual Viewer.
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
    /// For a `mouse_move`: where the mouse pointer is now, `[x, y]` in screen pixels, which
    /// the JAWS cursor follows (Alt+Delete, "JAWS Cursor active | 1834, 235" [obs, 516/s2]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mouse: Option<[i32; 2]>,
    /// For a `mouse_point`, traced where the mouse came to rest: the point and the text there
    /// by unit; the element there and its ancestors are the `focus_chain` (mouse echo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point: Option<TracePoint>,
    /// For a focus in a NetUI window (`NetUIHWND`: WordPad's ribbon, Office's): the event's
    /// MSAA object with its children, to depth 3. UI Automation stops at the ribbon's host
    /// pane, and Alt gives the focus to the ribbon itself, whose selected tab JAWS reads
    /// ("Ribbon tabs | Home Tab selected" [obs, 523/s1]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msaa_tree: Option<MsaaNode>,
    /// For that focus: the MSAA object's siblings, itself included, without their children
    /// (a ribbon tab's place among the tabs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub msaa_siblings: Vec<MsaaNode>,
    /// For a focus on a NetUI control: its MSAA ancestors inside the NetUI window, outermost
    /// first, without their children (the lower ribbon, the tab's page, the group: "Lower Ribbon
    /// | Home | Ribbon tab | Clipboard Tool bar" [obs, 970/s4]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub msaa_ancestors: Vec<MsaaNode>,
    /// For that focus: the control's MSAA description ("Click here for more options such as
    /// pasting only plain text." [obs, 970/s4]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msaa_description: Option<String>,
    /// For that focus: Office's own UIA element for the control ([`NetuiControl`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub netui_control: Option<NetuiControl>,
    /// For a focus in a page's dialog: the name of the modal dialog (`aria-modal`) it is in,
    /// read through IA2, which UI Automation does not say. A focus that opens the dialog
    /// comes before the page is captured again, and is said with it [obs, round-aria23,
    /// 2668/s1].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modal_dialog: Option<String>,
}

/// Where the mouse came to rest and the text there by unit (okur-windows' `mouse_point`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TracePoint {
    pub at: [i32; 2],
    #[serde(default)]
    pub texts: okur_core::UnitTexts,
}

/// A UIA notification (`UiaRaiseNotificationEvent`): Windows Settings' search box says
/// "Suggestions available" as it is typed in (JAWS reads it; dev box, 2026-10-16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceNotification {
    pub text: String,
    /// `NotificationProcessing`: 0 and 1 are important (they interrupt), 2 to 4 are not.
    pub processing: i32,
    /// The executable of the program that raised it ("powershell.exe"): the Notification
    /// History's sender [obs, 820/s3].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,
}

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
fn window_items(tree: &UiaNode) -> Vec<okur_core::WindowItem> {
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
fn rich_edit(class: Option<&str>) -> bool {
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
/// A list view: Win32's, or File Explorer's items view, whose unselected item JAWS reads as
/// "Not Selected SR OCR Sign.png" [obs, 1026/s5].
fn is_list_view(class: Option<&str>) -> bool {
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
    if active_window.and_then(|w| w.class_name.as_deref()) == Some(WIN32_DIALOG_CLASS)
        || dialog.is_some()
    {
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
            focus.columns = subtree
                .children
                .iter()
                .filter(|c| matches!(c.control_type, UIA_TEXT | UIA_EDIT))
                .filter_map(|c| c.name.clone())
                .filter(|n| !n.trim().is_empty() && Some(n) != focused.name.as_ref())
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

/// The menu mode transition a recorded event reports, if any.
pub fn menu_event_from_trace(event: &TraceEvent) -> Option<okur_core::MenuEvent> {
    use okur_core::MenuEvent;
    match event.event.as_str() {
        "menu_start" => Some(MenuEvent::Start {
            window_class: event.window_class.clone(),
        }),
        "menu_end" => Some(MenuEvent::End),
        "menu_popup_start" => Some(MenuEvent::PopupStart {
            window_class: event.window_class.clone(),
            window_title: event.window_title.clone(),
        }),
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
    /// The last toast and when it showed: the shell reports one toast twice, its view and
    /// the window around it [obs, 770/s1].
    last_toast: Option<(String, u128)>,
}

/// The last focus's MSAA identity (`hwnd`, `id_object`, `id_child`) and name, and the focused
/// control's own name from its focus chain.
type FocusIdentity = ((isize, i32, i32, Option<String>), Option<String>);

thread_local! {
    /// The last focus as the converter on this thread knows it. Live, the WinEvent thread reads
    /// a state or selection change's MSAA object first and skips the rest of the read when the
    /// change is not the focused control's, which the converter would drop: 73% of the state
    /// changes recorded (performance review P5, step 2).
    static CONVERTER_FOCUS: std::cell::RefCell<Option<FocusIdentity>> =
        const { std::cell::RefCell::new(None) };
}

/// The new value of a page's progress bar or meter (MSAA `ROLE_SYSTEM_PROGRESSBAR`, which
/// Chromium gives both) a value change event names.
fn page_progress_value(event: &TraceEvent) -> Option<String> {
    let object = event.object.as_ref()?;
    if !is_page_progress(event.window_class.as_deref()?, Some(object)) {
        return None;
    }
    object
        .value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
}

/// MSAA's `ROLE_SYSTEM_PROGRESSBAR`.
const ROLE_SYSTEM_PROGRESSBAR: i32 = 0x30;

/// Whether an object in a window of this class is a page's progress bar or meter.
fn is_page_progress(window_class: &str, object: Option<&TraceObject>) -> bool {
    WEB_CONTENT_WINDOW_CLASSES.contains(&window_class)
        && object.is_some_and(|o| o.role == Some(ROLE_SYSTEM_PROGRESSBAR))
}

/// Whether a change event, by its MSAA identity and object's name, concerns the focused
/// control, as the converter passes changes on: the focus's own object, or one of its name.
/// The focus event's MSAA object can be the browser window while the focused control is a page
/// button (focus left on it, then Space expanded it, GitHub): the control's own name, from the
/// focus chain, is compared too.
fn concerns_focus(
    (focused, control_name): (&(isize, i32, i32, Option<String>), Option<&String>),
    (hwnd, id_object, id_child): (isize, i32, i32),
    object_name: Option<&String>,
) -> bool {
    let (focus_hwnd, focus_object, focus_child, name) = focused;
    let same_object = hwnd == *focus_hwnd && id_object == *focus_object && id_child == *focus_child;
    let focus_on_window = control_name.is_some() && name.as_ref() != control_name;
    let same_name = object_name.is_some()
        && (object_name == name.as_ref() || (focus_on_window && object_name == control_name));
    same_object || same_name
}

/// Whether the converter on this thread may make something of a state, selection or value
/// change, judged from the event and its MSAA object only, before anything else is read: live,
/// the WinEvent thread reads no further a change this refuses. None before a focus.
///
/// Kept: a change of the focused control, and two value changes that are not the focus's:
/// UI Automation's own, which names no MSAA object, in the window of the last focus or the
/// window with the keyboard focus (`in_focus_window`: Excel's grid after Delete,
/// [`EventConverter::focus_moved_by_change`]; Word's document after Space on a check box form
/// field, the frame having had the last focus event), and a page's progress bar or meter, said
/// without the focus. Every change the
/// converter makes something of in the corpus's traces is kept (the corpus test
/// `live_prefilter_keeps_what_the_converter_uses`); the three it once refused were said only
/// in replay (performance review 2026-11-02, live path finding 1).
pub fn live_change_kept(
    event: &str,
    (hwnd, id_object, id_child): (isize, i32, i32),
    object: Option<&TraceObject>,
    window_class: &str,
    in_focus_window: bool,
) -> bool {
    CONVERTER_FOCUS.with(|focus| {
        let focus = focus.borrow();
        let Some((focused, control)) = focus.as_ref() else {
            return false;
        };
        let object_name = object.and_then(|o| o.name.as_ref());
        if concerns_focus(
            (focused, control.as_ref()),
            (hwnd, id_object, id_child),
            object_name,
        ) {
            return true;
        }
        event == "value_change"
            && ((object.is_none() && (hwnd == focused.0 || in_focus_window))
                || is_page_progress(window_class, object))
    })
}

/// A window's own name changed (its title), not a control's in it.
pub fn is_window_renamed(event: &TraceEvent) -> bool {
    event.event == "name_change" && event.id_object == OBJID_WINDOW
}

/// `OBJID_WINDOW`: an event about the window itself.
pub const OBJID_WINDOW: i32 = 0;

/// File Explorer's top-level window class.
const EXPLORER_WINDOW_CLASS: &str = "CabinetWClass";
/// The Windows Shell property of an item's kind, the AutomationId of its cell in a shell view.
const SHELL_ITEM_TYPE: &str = "System.ItemTypeText";
/// The Windows Shell property of an item's size ("1 KB"), likewise.
const SHELL_ITEM_SIZE: &str = "System.Size";
/// The Windows Shell properties a shell item's cells are named by, and its name's own.
const SHELL_PROPERTY: &str = "System.";
const SHELL_ITEM_NAME: &str = "System.ItemNameDisplay";

/// What the mouse points to, as the engine takes it: the control there described as a focus on
/// it would be, and its text there by unit. Shared by the converter and the live worker.
pub fn mouse_point_event(chain: &[UiaNode], point: &TracePoint) -> okur_core::AccessibilityEvent {
    okur_core::AccessibilityEvent::MousePoint(Box::new(okur_core::MousePoint {
        x: point.at[0],
        y: point.at[1],
        control: focus_event_from_chain(chain, None, None),
        texts: point.texts.clone(),
    }))
}

/// How long the same UIA notification again is not said, in milliseconds.
const NOTIFICATION_REPEAT_MS: u128 = 2000;

impl EventConverter {
    /// The engine events for one captured event, in order (usually one, or none).
    pub fn convert(&mut self, event: &TraceEvent) -> Vec<okur_core::AccessibilityEvent> {
        if let Some(point) = &event.point {
            return vec![mouse_point_event(&event.focus_chain, point)];
        }
        if let Some([x, y]) = event.mouse {
            return vec![okur_core::AccessibilityEvent::MouseMoved { x, y }];
        }
        // A page's alert carries its content as a live region does [obs, round-aria12].
        if event.event == "live_region_change" || (event.event == "alert" && event.region.is_some())
        {
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
        // A console's screen after it wrote to it, for Screen Echo.
        if event.event == "console_update" {
            return event
                .text_context
                .as_ref()
                .map(|context| okur_core::AccessibilityEvent::ConsoleText(context.text.clone()))
                .into_iter()
                .collect();
        }
        if event.event == "uia_window_opened" {
            let Some((text, sender)) = event.focus_subtree.as_ref().and_then(toast_text) else {
                return Vec::new();
            };
            let repeat = self.last_toast.as_ref().is_some_and(|(said, at)| {
                *said == text && event.t_ms.saturating_sub(*at) < NOTIFICATION_REPEAT_MS
            });
            if repeat {
                return Vec::new();
            }
            self.last_toast = Some((text.clone(), event.t_ms));
            return vec![okur_core::AccessibilityEvent::Toast { text, sender }];
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
        // A focus that had moved on before it was read is not said: the focus after it is.
        // Tab from File Explorer's search box passes its list for 30 ms on the way to the
        // first link, and JAWS says only "Control Panel Home Link" [obs, 3156/s6].
        if left_before_read(event) {
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

    /// Whether `event`, UI Automation's own change of a value in the window with the last focus,
    /// leaves the keyboard focus on that window's own element, where the last focus was another.
    fn focus_moved_by_change(&self, event: &TraceEvent) -> bool {
        let Some(focused) = event.focus_chain.last() else {
            return false;
        };
        event.object.is_none()
            && event.text_context.is_none()
            && focused.has_keyboard_focus
            && focused.class_name.is_some()
            && focused.class_name == event.window_class
            && self
                .focused
                .as_ref()
                .is_some_and(|(hwnd, ..)| *hwnd == event.hwnd)
            && self
                .last_focus
                .as_ref()
                .is_some_and(|last| last.name != focused.name)
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
        // UI Automation's keyboard focus moved within the focused window with no focus event:
        // Excel's cell emptied by Delete leaves it on the grid, which reports only the change
        // of its value [obs, 1123/s7]. The focus chain read after the change is the new focus.
        if event.event == "value_change" && self.focus_moved_by_change(event) {
            let mut moved = event.clone();
            moved.event = "focus".to_owned();
            return self.convert_one(&moved);
        }
        let kind = match event.event.as_str() {
            // A list item selected while the focus stays in a field (Windows search).
            "uia_selected" => {
                return focus_event_from_trace(event).map(AccessibilityEvent::Selected);
            }
            // The dialog in front shown, read again: its controls as they are now, for Say
            // Window, after a focus read while it was still hidden (the Font dialog [obs,
            // 665/s3]).
            "dialog_shown" => {
                let shown = focus_event_from_trace(event)?;
                if let Some(last) = self.last_focus.as_mut() {
                    if last.window_title == shown.window_title {
                        last.window_items = shown.window_items.clone();
                    }
                }
                return Some(AccessibilityEvent::WindowShown(shown));
            }
            "focus" if is_drop_down_item(event) => {
                let last = self.last_focus.clone()?;
                let name = event.object.as_ref().and_then(|o| o.name.clone());
                let index = usize::try_from(event.id_child).ok().filter(|&i| i > 0);
                // A combo box whose text field has the focus (the Font dialog's, whose lists
                // stay open): its list's item is selected while the focus stays in the field.
                // The dialog's other lists follow it, selecting their own items; the field's
                // new text tells which item is its own [obs, 541/s3].
                if last.role == okur_core::Role::Edit
                    && last.container_role == Some(okur_core::Role::ComboBox)
                {
                    let mut item = okur_core::FocusEvent::new(okur_core::Role::ListItem);
                    item.name = name;
                    item.item_index = index;
                    item.item_count = last.item_count;
                    return Some(AccessibilityEvent::Selected(item));
                }
                // Arrow keys in an open combo box move focus into its drop-down list; JAWS
                // speaks that as the combo box's new value [obs, 304/s12].
                let mut combo = Some(last).filter(|f| f.role == okur_core::Role::ComboBox)?;
                combo.value = name;
                combo.item_index = index;
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
                // Word names a ribbon command twice: the ribbon's own focus names its tab, then
                // the tab's own event does; Cut's event comes twice [obs, 1045/s1, s3]. The same
                // command again, right after, is the same focus.
                if focus.in_ribbon
                    && self.last_focus.as_ref().is_some_and(|last| {
                        last.in_ribbon && last.role == focus.role && last.name == focus.name
                    })
                {
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
                CONVERTER_FOCUS.with(|known| {
                    *known.borrow_mut() = self
                        .focused
                        .clone()
                        .map(|focused| (focused, self.focused_control.clone()));
                });
                self.last_focus = Some(focus.clone());
                return Some(AccessibilityEvent::Focus(focus));
            }
            // A select's value changed: it has no text, only the new choice as its value. It is
            // said as a value change [obs, 205/s9, s10].
            "value_change" if is_choice_without_text(event) => ChangeKind::Value,
            // A page's progress bar or meter changing without the focus says its new value,
            // "86.0274", the CPU meter as its button changes it [obs, round-aria38, 3113/s1,
            // 3116/s1].
            "value_change"
                if page_progress_value(event).is_some()
                    && !self.focused.as_ref().is_some_and(|focused| {
                        concerns_focus(
                            (focused, self.focused_control.as_ref()),
                            (event.hwnd, event.id_object, event.id_child),
                            event.object.as_ref().and_then(|o| o.name.as_ref()),
                        )
                    }) =>
            {
                return page_progress_value(event).map(|value| {
                    AccessibilityEvent::LiveRegion(okur_core::LiveRegionEvent::new(
                        value,
                        okur_core::Politeness::Polite,
                    ))
                });
            }
            // The focused text field's caret moved or its text changed. A value change of
            // another object is not the field's: Word's ribbon and its Font boxes change as the
            // caret crosses a style, and each was read as the document's text (performance
            // review 2026-10-25, finding 2).
            "caret" | "text_selection_change" | "value_change" if event.text_context.is_some() => {
                let object_name = event.object.as_ref().and_then(|o| o.name.as_ref());
                let concerns = self.focused.as_ref().is_some_and(|focused| {
                    concerns_focus(
                        (focused, self.focused_control.as_ref()),
                        (event.hwnd, event.id_object, event.id_child),
                        object_name,
                    )
                });
                // UI Automation's own change of the text in the window with the keyboard focus
                // names no MSAA object: Word's document after Space on a check box form field,
                // the only event with its new state [1057 traces]. Word's frame can have had the
                // last focus event, so the window is told by UI Automation's focus, its class.
                let own_window = event.object.is_none()
                    && event.window_class.is_some()
                    && event
                        .focus_chain
                        .last()
                        .and_then(|n| n.class_name.as_deref())
                        == event.window_class.as_deref();
                if event.event == "value_change" && !concerns && !own_window {
                    return None;
                }
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
        let focused = self.focused.as_ref()?;
        let (hwnd, id_object, id_child, _) = focused;
        let same_object =
            event.hwnd == *hwnd && event.id_object == *id_object && event.id_child == *id_child;
        let object_name = event.object.as_ref().and_then(|o| o.name.as_ref());
        if !concerns_focus(
            (focused, self.focused_control.as_ref()),
            (event.hwnd, event.id_object, event.id_child),
            object_name,
        ) {
            return None;
        }
        // The control as UIA reports it now; without a focus chain (a failed UIA read), the
        // last focus with the event object's MSAA states. The chain is read after the event,
        // and the focus can have moved on from the control the event is about: a select's
        // list takes the focus as the select expands [obs, 276/s7]. The change is then the
        // last focus's, the event object.
        let chain_control = focus_event_from_trace(event);
        let last_name = self.last_focus.as_ref().and_then(|l| l.name.as_ref());
        let moved_on = same_object
            && object_name.is_some()
            && object_name == last_name
            && chain_control
                .as_ref()
                .is_some_and(|c| c.name.as_ref() != object_name);
        let mut control = match chain_control.filter(|_| !moved_on) {
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

/// A UIA notification, said as an announcement, interrupting speech when the application marks
/// it important, under its program's name without ".exe" ("powershell" [obs, 820/s3]).
fn notification_event(notification: &TraceNotification) -> okur_core::AccessibilityEvent {
    use okur_core::Politeness;
    // NotificationProcessing_ImportantAll and _ImportantMostRecent.
    let politeness = if matches!(notification.processing, 0 | 1) {
        Politeness::Assertive
    } else {
        Politeness::Polite
    };
    let app = notification.process.as_deref().map(|name| {
        name.rsplit_once('.')
            .filter(|(_, extension)| extension.eq_ignore_ascii_case("exe"))
            .map_or(name, |(stem, _)| stem)
            .to_owned()
    });
    okur_core::AccessibilityEvent::AppNotification {
        text: notification.text.trim().to_owned(),
        politeness,
        app,
    }
}

/// A native application's live region changed (UIA): the texts it holds, then its own name
/// where that is not one of them, as JAWS reads Windows Settings' expander button as Alt+Left
/// shows its page again: " Brightness Change brightness for the built-in display | Show more
/// settings" (the slider inside is left out; dev box, 2026-10-17). A text region says its
/// name. A region whose live setting is off says nothing.
/// The UIA class of a WPF list box's items.
const WPF_LIST_ITEM: &str = "ListBoxItem";

/// Whether `name` is a .NET type's full name, as WPF names an item by its data object's type
/// when the item has no name of its own: identifiers that start with a capital, joined by dots
/// ("NotificationViewer.Services.NotificationData").
fn is_type_name(name: &str) -> bool {
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|part| {
            part.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

/// Whether an item's `name` is its `texts` joined by spaces, the marks that steer a date's
/// direction left out of both (XAML puts them in the texts and the name alike).
fn names_its_texts(name: &str, texts: &[String]) -> bool {
    use okur_core::viewer_text::without_marks;
    let words = |text: &str| {
        without_marks(text)
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    !texts.is_empty() && words(name) == words(&texts.join(" "))
}

/// A toast's text and sender, in a window of the shell that opened: its view's name
/// (AutomationId `NormalToastView`, on Windows 10), as JAWS says it, and its `SenderName`'s
/// [obs, 770/s1].
fn toast_text(window: &UiaNode) -> Option<(String, Option<String>)> {
    const TOAST_VIEW: &str = "NormalToastView";
    const SENDER: &str = "SenderName";
    let named = |node: &UiaNode| {
        node.name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_owned)
    };
    if window.automation_id.as_deref() == Some(TOAST_VIEW) {
        let sender = window
            .children
            .iter()
            .find(|c| c.automation_id.as_deref() == Some(SENDER))
            .and_then(named);
        return named(window).map(|text| (text, sender));
    }
    window.children.iter().find_map(toast_text)
}

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
            .map(|name| crate::text::without_accelerators(&name))
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
    focus.app_product = event.process_product.clone();
    focus.app_version = event.process_version.clone();
    focus.bounds = event.focus_chain.last().and_then(|n| n.bounds);
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
fn command_bar_focus(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
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
fn in_task_pane(event: &TraceEvent) -> bool {
    event.focus_chain.iter().any(|n| {
        n.class_name.as_deref() == Some("MsoCommandBarDock")
            && n.name.as_deref().is_some_and(|dock| dock != "MsoDockTop")
    })
}

/// A list's item in an Office task pane, placed among the list's items: the Spelling pane's
/// suggestions, "the | 1 of 5" [obs, 1069/s4].
fn task_pane_item(event: &TraceEvent, object: &TraceObject, name: &str) -> okur_core::FocusEvent {
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
fn gallery_item(event: &TraceEvent, object: &TraceObject, name: &str) -> okur_core::FocusEvent {
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
fn ribbon_page(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
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
const MSAA_INVISIBLE: u32 = 0x8000;
/// MSAA's unavailable state.
const MSAA_UNAVAILABLE: u32 = 0x1;
/// MSAA's editable text role.
const MSAA_TEXT: i32 = 0x2A;
/// MSAA's spin button role: a RichEdit with arrows that step its value.
const MSAA_SPIN_BUTTON: i32 = 0x34;
/// MSAA's tool bar role: a ribbon's group of commands.
const TOOLBAR: i32 = 0x16;

/// The ribbon's panes, as UI Automation names them (the window itself is not one of them).
fn ribbon_panes(event: &TraceEvent) -> impl Iterator<Item = okur_core::GroupEntry> + '_ {
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
fn ribbon_groups(event: &TraceEvent) -> Vec<okur_core::GroupEntry> {
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
fn ribbon_tab(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
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

/// A common file dialog's folder: its address bar's toolbar (`ToolbarWindow32`, control id
/// 1001, in the address band) is named for it, "Address: C:\Users\…" [obs, 524/s3].
fn address_band(node: &UiaNode) -> Option<String> {
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
fn toolbar_button(event: &TraceEvent, toolbar: &UiaNode, focus: &mut okur_core::FocusEvent) {
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
fn header_bar(event: &TraceEvent, focused: &UiaNode, focus: &mut okur_core::FocusEvent) {
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

/// Whether an event in the process of a dialog leaves the dialog's controls as they were last
/// read (performance review P6): the focus moving among them, with the caret and selection put
/// in the field it reaches, and a push button's state, which changes as the default button
/// follows the focus (Find Next, twice per Tab onto it [obs, 540]). Any other event (a check
/// box toggled, a value typed, a list's selection, a text renamed, a control shown or hidden)
/// may be a change the read does not show.
pub fn keeps_dialog_read(event: &str, push_button: bool) -> bool {
    matches!(
        event,
        "focus" | "uia_focus" | "caret" | "text_selection_change"
    ) || (event == "state_change" && push_button)
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

    /// Say Window in Notepad's Font dialog, from its tree as the tracer reads it (a list's
    /// shown items, a combo box's place, the sample by its control id): JAWS's words
    /// [obs, 665/s3], but for the Size list's name, which JAWS says and the other lists'
    /// it does not.
    #[test]
    fn say_window_reads_a_font_dialog_as_jaws_does() {
        const COMBOBOX: i32 = 50003;
        const EDIT: i32 = 50004;
        const LINK: i32 = 50005;
        const LISTITEM: i32 = 50007;
        const LIST: i32 = 50008;
        const SCROLLBAR: i32 = 50014;
        const TEXT: i32 = 50020;
        const GROUP: i32 = 50026;
        const PANE: i32 = 50033;
        let with = |mut n: UiaNode, children: Vec<UiaNode>| {
            n.children = children;
            n
        };
        let valued = |control_type: i32, name: &str, value: &str| UiaNode {
            value: Some(value.to_owned()),
            ..node(control_type, name, false)
        };
        let combo = |label: &str, value: &str, items: &[&str], place: (i32, i32)| {
            let item = |name: &&str| UiaNode {
                is_selected: Some(*name == value),
                ..node(LISTITEM, name, false)
            };
            let list = with(
                node(LIST, label, false),
                std::iter::once(with(
                    node(SCROLLBAR, "Vertical", false),
                    vec![node(50000, "Line down", false)],
                ))
                .chain(items.iter().map(item))
                .collect(),
            );
            UiaNode {
                position_in_set: Some(place.0),
                size_of_set: Some(place.1),
                ..with(
                    valued(COMBOBOX, label, value),
                    vec![list, valued(EDIT, label, value)],
                )
            }
        };
        let fonts = [
            "Consolas",
            "Constantia",
            "Cooper",
            "Copperplate Gothic",
            "Corbel",
            "Courier",
            "Courier New",
        ];
        let script = UiaNode {
            position_in_set: Some(1),
            size_of_set: Some(7),
            ..with(
                valued(COMBOBOX, "Script:", "Western"),
                vec![
                    valued(TEXT, "Script:", "Western"),
                    node(50000, "Open", false),
                ],
            )
        };
        let tree = with(
            node(50032, "Font", false),
            vec![
                node(TEXT, "Font:", false),
                UiaNode {
                    automation_id: Some("1136".to_owned()),
                    ..combo("Font:", "Consolas", &fonts, (38, 187))
                },
                node(TEXT, "Font style:", false),
                combo(
                    "Font style:",
                    "Regular",
                    &["Regular", "Italic", "Bold", "Bold Italic"],
                    (1, 4),
                ),
                node(GROUP, "Sample", false),
                UiaNode {
                    automation_id: Some("1093".to_owned()),
                    ..node(TEXT, "AaBbYyZz", false)
                },
                node(TEXT, "Script:", false),
                script,
                with(
                    node(PANE, "Show more fonts", false),
                    vec![node(LINK, "Show more fonts", false)],
                ),
                node(50000, "OK", false),
            ],
        );
        let said = okur_core::native_focus::say_window("Font", &window_items(&tree)).render();
        let words = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            words(&said),
            words(
                "Font dialog\nFont: \nCombo box\nConsolas Consolas Constantia Cooper \
                 Copperplate Gothic Corbel Courier Courier New\n38 of 187\n \u{1d} List box \
                 \u{1e} \nConsolas\nFont:  \u{1d} edit combo \u{1e} \nConsolas\n38 of 187\n\
                 Font style: \nCombo box\nRegular Regular Italic Bold Bold Italic\n1 of 4\n \
                 \u{1d} List box \u{1e} \nRegular\nFont style:  \u{1d} edit combo \u{1e} \n\
                 Regular\n1 of 4\nSample  \u{1d} group \u{1e} \nScript: \nCombo box\n\
                 Western\n1 of 7\nShow more fonts  \u{1d} Link \u{1e} \nOK  \u{1d} Button \u{1e}"
            )
        );
    }

    /// A dialog's controls read for one focus are used again for the next while only the
    /// focus moves among them; any other change in its process reads them anew.
    #[test]
    fn a_dialog_is_read_again_only_after_a_change() {
        for moved in ["focus", "uia_focus", "caret", "text_selection_change"] {
            assert!(keeps_dialog_read(moved, false), "{moved}");
        }
        // The default button follows the focus onto Find Next [obs, 540].
        assert!(keeps_dialog_read("state_change", true));
        // A check box toggled.
        assert!(!keeps_dialog_read("state_change", false));
        for changed in [
            "value_change",
            "selection",
            "selection_add",
            "name_change",
            "description_change",
            "dialog_start",
            "foreground",
        ] {
            assert!(!keeps_dialog_read(changed, false), "{changed}");
            assert!(!keeps_dialog_read(changed, true), "{changed}");
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
        // An unnamed WPF group box, named by its header [obs, 775/s6].
        let mut group_box = node(UIA_GROUP, "", false);
        group_box.name = None;
        group_box.label = Some("Received Notification".to_owned());
        let wpf = [group_box, node(50003, "Rule Type", true)];
        assert_eq!(
            nested_groups(&wpf),
            (groups(&["Received Notification"]), false)
        );
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

    /// A WPF item named after its data object's type is read by its texts [obs, 774/s3].
    #[test]
    fn a_wpf_items_type_name_is_left_out() {
        assert!(is_type_name("NotificationViewer.Services.NotificationData"));
        assert!(!is_type_name("example.com"));
        assert!(!is_type_name("Report.final"));
        assert!(!is_type_name("Notes"));
        assert!(!is_type_name("Mr. Smith"));
    }

    /// A XAML item named after its texts has no further columns; a list view's row does
    /// [obs, 3155/s7].
    #[test]
    fn an_item_named_after_its_texts() {
        let texts = |t: &[&str]| t.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert!(names_its_texts(
            "7-Zip 18.05 (x64) 4.93 MB \u{200E}11/\u{200E}7/\u{200E}2020",
            &texts(&[
                "7-Zip 18.05 (x64)",
                "4.93 MB",
                "\u{200E}11/\u{200E}7/\u{200E}2020"
            ])
        ));
        assert!(!names_its_texts("notes.txt", &texts(&["2 KB"])));
        assert!(!names_its_texts("notes.txt 2 KB", &texts(&["2 KB"])));
        assert!(!names_its_texts("notes.txt", &[]));
    }

    /// A focus read after the keyboard focus had left its window, its object no longer
    /// focused, is not said; the focus that follows is [obs, 3156/s6]. The same object still
    /// focused is said.
    #[test]
    fn a_focus_that_moved_on_before_its_read_is_not_said() {
        let mut passed = focus_event(Vec::new());
        passed.object = Some(TraceObject {
            role: Some(0x21),
            name: Some("Folder View".to_owned()),
            value: None,
            states: 0x0010_0000,
            shortcut: None,
        });
        passed.focus_elsewhere = true;
        assert!(EventConverter::default().convert(&passed).is_empty());
        if let Some(object) = passed.object.as_mut() {
            object.states |= 0x4;
        }
        assert!(!EventConverter::default().convert(&passed).is_empty());
    }

    /// An item with a name of its own and a further text, outside a list view, has the text
    /// as its description: Windows Settings' home [obs, 3157/s4]. In a list view it is a
    /// column.
    #[test]
    fn an_items_further_text_outside_a_list_view_is_its_description() {
        let mut event = focus_event(Vec::new());
        let devices = node(UIA_LISTITEM, "Devices", true);
        event.focus_chain = vec![
            node(UIA_WINDOW, "Settings", false),
            node(UIA_LIST, "", false),
            devices.clone(),
        ];
        event.focus_subtree = Some(UiaNode {
            children: vec![
                node(UIA_TEXT, "Devices", false),
                node(UIA_TEXT, "Bluetooth, printers, mouse", false),
            ],
            ..devices
        });
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus
                .as_ref()
                .map(|f| (f.description.as_deref(), f.columns.len())),
            Some((Some("Bluetooth, printers, mouse"), 0))
        );
        if let Some(list) = event.focus_chain.get_mut(1) {
            list.class_name = Some("SysListView32".to_owned());
        }
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus.map(|f| (f.description, f.columns)),
            Some((None, vec!["Bluetooth, printers, mouse".to_owned()]))
        );
    }

    /// A toast, as the shell reports it opening: its view, then the window around it, once
    /// said [obs, 770/s1].
    #[test]
    fn a_toast_is_said_once_by_its_views_name() {
        const TOAST: &str =
            "New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1";
        let view = UiaNode {
            automation_id: Some("NormalToastView".to_owned()),
            children: vec![
                UiaNode {
                    automation_id: Some("SenderName".to_owned()),
                    ..node(UIA_TEXT, "Windows PowerShell", false)
                },
                UiaNode {
                    automation_id: Some("TitleText".to_owned()),
                    ..node(UIA_TEXT, "SR Fixture", false)
                },
            ],
            ..node(UIA_WINDOW, TOAST, false)
        };
        let window = UiaNode {
            children: vec![view.clone()],
            ..node(UIA_WINDOW, "New notification", false)
        };
        let mut converter = EventConverter::default();
        let mut opened = focus_event(Vec::new());
        opened.event = "uia_window_opened".to_owned();
        opened.focus_subtree = Some(view);
        assert_eq!(
            converter.convert(&opened),
            vec![okur_core::AccessibilityEvent::Toast {
                text: TOAST.to_owned(),
                sender: Some("Windows PowerShell".to_owned())
            }]
        );
        opened.t_ms += 13;
        opened.focus_subtree = Some(window);
        assert!(converter.convert(&opened).is_empty());
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
                process: Some("SystemSettings.exe".to_owned()),
            });
            event
        };
        let mut converter = EventConverter::default();
        let said = |events: Vec<okur_core::AccessibilityEvent>| match events.first() {
            Some(okur_core::AccessibilityEvent::AppNotification { text, app, .. }) => {
                assert_eq!(app.as_deref(), Some("SystemSettings"));
                Some(text.clone())
            }
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
            shortcut: None,
        });
        event.focus_chain = vec![
            node(UIA_DOCUMENT, "Untitled document - Google Docs", false),
            node(UIA_EDIT, "Document content", true),
        ];
        assert_eq!(focus_event_from_trace(&event), None);
    }

    /// A Win32 dialog's control has its access key from MSAA when UIA names none [obs,
    /// 541/s8]; UIA's comes first.
    #[test]
    fn an_access_key_comes_from_msaa_when_uia_has_none() {
        let mut event = focus_event(Vec::new());
        event.object = Some(TraceObject {
            role: None,
            name: Some("Pro".to_owned()),
            value: None,
            states: 0,
            shortcut: Some("Alt+r".to_owned()),
        });
        assert_eq!(
            focus_event_from_trace(&event).and_then(|f| f.access_key),
            Some("Alt+r".to_owned())
        );
        if let Some(pro) = event.focus_chain.last_mut() {
            pro.access_key = Some("alt+P".to_owned());
        }
        assert_eq!(
            focus_event_from_trace(&event).and_then(|f| f.access_key),
            Some("alt+P".to_owned())
        );
    }

    /// A focus event from a window already gone (Okur's own list dialog as it closes) reads
    /// UIA's focus: the page, in the window with the keyboard focus (live11, 256/s4).
    #[test]
    fn a_focus_from_a_window_gone_is_in_the_focus_window() {
        let mut event = focus_event(Vec::new());
        event.focus_chain = vec![node(UIA_DOCUMENT, "Riverside Gazette", true)];
        assert_eq!(
            focus_event_from_trace(&event).map(|f| f.in_document),
            Some(false)
        );
        event.focus_window_class = Some("Chrome_RenderWidgetHostHWND".to_owned());
        assert_eq!(
            focus_event_from_trace(&event).map(|f| f.in_document),
            Some(true)
        );
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
            spin_box: false,
            focus_window_class: None,
            focus_elsewhere: false,
            menu_bar: Vec::new(),
            status_bar: Vec::new(),
            window_title: None,
            process: None,
            process_product: None,
            process_version: None,
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
            top_window_owner_enabled: None,
            virtual_desktop: None,
            dialog_lines: Vec::new(),
            dialog_text: Vec::new(),
            dialog_page: None,
            group_box: None,
            default_button: None,
            dialog_tree: None,
            notification: None,
            mouse: None,
            point: None,
            msaa_tree: None,
            msaa_siblings: Vec::new(),
            msaa_ancestors: Vec::new(),
            msaa_description: None,
            netui_control: None,
            modal_dialog: None,
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

    /// A Win32 message box over a WPF dialog window: the window is named plainly before the box
    /// ("Manage Notification Rules | Confirm Delete dialog" [obs, 787/s17]); a WPF dialog over
    /// another names no outer window (787/s5).
    #[test]
    fn a_message_box_names_the_wpf_window_it_opened_over() {
        let mut manage = node(UIA_WINDOW, "Manage Notification Rules", false);
        manage.is_dialog = Some(true);
        manage.class_name = Some("Window".to_owned());
        let mut confirm = node(UIA_WINDOW, "Confirm Delete", false);
        confirm.class_name = Some(WIN32_DIALOG_CLASS.to_owned());
        let mut event = focus_event(Vec::new());
        event.focus_chain = vec![manage.clone(), confirm, node(UIA_BUTTON, "Yes", true)];
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus.as_ref().map(|f| f.outer_windows.clone()),
            Some(vec!["Manage Notification Rules".to_owned()])
        );
        let mut name_rule = node(UIA_WINDOW, "Name Your Rule", false);
        name_rule.is_dialog = Some(true);
        event.focus_chain = vec![manage, name_rule, node(UIA_BUTTON, "OK", true)];
        let focus = focus_event_from_trace(&event);
        assert_eq!(focus.map(|f| f.outer_windows), Some(Vec::new()));
    }

    /// A WPF window is read element by element by its window's class, also when the focus is
    /// inside it and UI Automation makes the window a dialog of class "Window" (1022/s1).
    #[test]
    fn a_wpf_window_with_the_focus_inside_is_read_by_elements() {
        let mut window = node(UIA_WINDOW, "SR WPF Controls Fixture", false);
        window.is_dialog = Some(true);
        window.class_name = Some("Window".to_owned());
        let mut event = focus_event(Vec::new());
        event.window_class = Some("HwndWrapper[DefaultDomain;;1]".to_owned());
        event.focus_chain = vec![window, node(UIA_BUTTON, "Save", true)];
        let focus = focus_event_from_trace(&event);
        assert_eq!(focus.map(|f| f.read_by_elements), Some(true));
    }

    /// A field in a WPF group box: named by the text before it, and its group's line is the
    /// header and that text: "Speech or Sound Action Path to Sound File: | Path to Sound File:
    /// Edit" [obs, 801/s11].
    #[test]
    fn a_wpf_group_boxs_line_takes_the_texts_before_the_control() {
        const GROUP: i32 = 50026;
        const EDIT: i32 = 50004;
        const COMBO: i32 = 50003;
        let mut group = node(GROUP, "", false);
        group.name = None;
        group.class_name = Some("GroupBox".to_owned());
        group.label = Some("Speech or Sound Action".to_owned());
        let mut edit = node(EDIT, "", true);
        edit.name = None;
        edit.runtime_id = Some(vec![7, 1, 4]);
        let mut event = focus_event(vec![
            node(TEXT, "_Speech or Sound Action", false),
            node(COMBO, "Speech Action", false),
            node(TEXT, "Path to Sound File:", false),
            edit.clone(),
            node(UIA_BUTTON, "Select Sound...", false),
        ]);
        let mut window = node(UIA_WINDOW, "Create Rule", false);
        window.is_dialog = Some(true);
        event.focus_chain = vec![window, group, edit];
        let focus = focus_event_from_trace(&event);
        assert_eq!(
            focus.as_ref().and_then(|f| f.name.as_deref()),
            Some("Path to Sound File:")
        );
        assert_eq!(
            focus
                .as_ref()
                .and_then(|f| f.groups.last())
                .map(|g| g.name.as_str()),
            Some("Speech or Sound Action Path to Sound File:")
        );
    }

    /// A traced `mouse_point` is what the mouse points to: the control there and its text by
    /// unit, for mouse echo in replay; a `mouse_move` stays a move.
    #[test]
    fn a_mouse_point_is_what_the_mouse_points_to() {
        let mut event = focus_event(Vec::new());
        event.event = "mouse_point".to_owned();
        event.point = Some(TracePoint {
            at: [16, 87],
            texts: okur_core::UnitTexts {
                line: Some(
                    "fn main() {
"
                    .to_owned(),
                ),
                ..Default::default()
            },
        });
        let converted = EventConverter::default().convert(&event);
        let point = match converted.as_slice() {
            [okur_core::AccessibilityEvent::MousePoint(point)] => Some(point),
            _ => None,
        };
        assert_eq!(point.map(|p| (p.x, p.y)), Some((16, 87)), "{converted:?}");
        assert_eq!(
            point.and_then(|p| p.texts.line.as_deref()),
            Some(
                "fn main() {
"
            )
        );
        assert_eq!(
            point.and_then(|p| p.control.as_ref()?.name.as_deref()),
            Some("Pro")
        );
        let mut moved = focus_event(Vec::new());
        moved.event = "mouse_move".to_owned();
        moved.mouse = Some([16, 87]);
        assert_eq!(
            EventConverter::default().convert(&moved),
            [okur_core::AccessibilityEvent::MouseMoved { x: 16, y: 87 }]
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

    /// A button of an Office command bar inside a dialog is read from MSAA, held down where
    /// MSAA says so: Insert Hyperlink's "Existing File or Web Page" [obs, 1076/s5]; the bar's
    /// own focus is none.
    #[test]
    fn a_command_bars_button_is_read_from_msaa() {
        let object = |role: i32, name: &str, states: u32| {
            Some(TraceObject {
                role: Some(role),
                name: Some(name.to_owned()),
                value: None,
                states,
                shortcut: Some("x".to_owned()),
            })
        };
        let button = TraceEvent {
            window_class: Some(COMMAND_BAR_CLASS.to_owned()),
            top_window_title: Some("Insert Hyperlink".to_owned()),
            object: object(0x2B, "Existing File or Web Page", 0x10_000C),
            focus_chain: vec![node(UIA_WINDOW, "Insert Hyperlink", false)],
            ..focus_event(Vec::new())
        };
        let focus = command_bar_focus(&button);
        assert!(focus.is_some());
        let Some(focus) = focus else { return };
        assert_eq!(focus.role, okur_core::Role::Button);
        assert_eq!(focus.name.as_deref(), Some("Existing File or Web Page"));
        assert!(focus.states.pressed);
        assert_eq!(focus.window_title.as_deref(), Some("Insert Hyperlink"));
        let bar = TraceEvent {
            object: object(0x16, "MSO Generic Control Container", 0x1000),
            ..button
        };
        assert!(command_bar_focus(&bar).is_none());
    }

    /// A command in the lower ribbon names what it is in, from its MSAA ancestors: "Lower
    /// Ribbon | Home | Ribbon tab | Clipboard Tool bar | Paste Click here for more options
    /// such as pasting only plain text." [obs, 970/s4], and it is on the ribbon's menus.
    #[test]
    fn a_lower_ribbon_command_names_its_groups() {
        let msaa = |role: i32, name: &str, value: &str, description: &str| MsaaNode {
            role: Some(role),
            name: (!name.is_empty()).then(|| name.to_owned()),
            states: 0,
            value: (!value.is_empty()).then(|| value.to_owned()),
            description: (!description.is_empty()).then(|| description.to_owned()),
            children: Vec::new(),
        };
        let event = TraceEvent {
            window_class: Some("NetUIHWND".to_owned()),
            object: Some(TraceObject {
                role: Some(0x3E),
                name: Some("Paste".to_owned()),
                value: Some("Paste (Ctrl+V)".to_owned()),
                states: 0x10_0004,
                shortcut: None,
            }),
            focus_chain: vec![
                node(UIA_WINDOW, "WordPad", false),
                node(50033, "UIRibbonDockTop", false),
            ],
            msaa_ancestors: vec![
                msaa(0x26, "Ribbon", "Ribbon", ""),
                msaa(0x10, "Lower Ribbon", "Lower Ribbon", ""),
                msaa(0x0A, "", "", ""),
                msaa(0x26, "Home", "Ribbon tab", ""),
                msaa(0x16, "Clipboard", "Group", ""),
                msaa(
                    0x14,
                    "Paste",
                    "",
                    "Click here for more options such as pasting only plain text.",
                ),
            ],
            ..focus_event(Vec::new())
        };
        let focus = netui_focus(&event);
        assert!(focus.is_some());
        let Some(focus) = focus else { return };
        assert_eq!(focus.value.as_deref(), Some("Paste (Ctrl+V)"));
        assert!(focus.ribbon_menu);
        let groups: Vec<(&str, bool, Option<&str>)> = focus
            .groups
            .iter()
            .map(|g| (g.name.as_str(), g.tool_bar, g.detail.as_deref()))
            .collect();
        assert_eq!(
            groups,
            [
                ("UIRibbonDockTop", false, None),
                ("Lower Ribbon", false, None),
                ("Home", false, Some("Ribbon tab")),
                ("Clipboard", true, None),
                (
                    "Paste",
                    false,
                    Some("Click here for more options such as pasting only plain text.")
                ),
            ]
        );
    }

    /// Word's ribbon commands, from MSAA and Office's own UIA element: a toggle button's state
    /// and ScreenTip title and its own rectangle, a drop-down a menu button (a "sub menu" to
    /// JAWS) where Paste is a split button [obs, 1045/s2, s5, s9, s11].
    #[test]
    fn a_word_ribbon_command_has_its_tip_toggle_and_kind() {
        let command = |role: i32, name: &str, control: Option<NetuiControl>| TraceEvent {
            window_class: Some("NetUIHWND".to_owned()),
            id_object: 103_350_513,
            object: Some(TraceObject {
                role: Some(role),
                name: Some(name.to_owned()),
                value: None,
                states: 0x10_0004,
                shortcut: None,
            }),
            focus_chain: vec![node(UIA_WINDOW, "sr-word-fixture.docx - Word", false)],
            netui_control: control,
            ..focus_event(Vec::new())
        };
        let painter = netui_focus(&command(
            0x2B,
            "Format Painter",
            Some(NetuiControl {
                bounds: Some([55, 132, 184, 159]),
                help_text: Some("Format Painter (Ctrl+Shift+C, Ctrl+Shift+V)".to_owned()),
                toggle_state: Some(0),
                control_type: None,
            }),
        ));
        assert!(painter.is_some());
        let Some(painter) = painter else { return };
        assert_eq!(painter.role, okur_core::Role::Button);
        assert_eq!(painter.states.checked, Some(false));
        assert_eq!(
            painter.help_text.as_deref(),
            Some("Format Painter (Ctrl+Shift+C, Ctrl+Shift+V)")
        );
        assert_eq!(painter.bounds, Some([55, 132, 184, 159]));
        // Without Office's element (traces before format 8), no toggle state and no tip.
        let plain = netui_focus(&command(0x2B, "Copy", None));
        assert!(plain.is_some_and(|p| p.states.checked.is_none() && p.help_text.is_none()));
        let roles: Vec<Option<okur_core::Role>> = [(0x38, "Change Case"), (0x3A, "Cover Page")]
            .iter()
            .chain(&[(0x3E, "Paste")])
            .map(|&(role, name)| netui_focus(&command(role, name, None)).map(|f| f.role))
            .collect();
        assert_eq!(
            roles,
            [
                Some(okur_core::Role::MenuButton),
                Some(okur_core::Role::MenuButton),
                Some(okur_core::Role::SplitButton)
            ]
        );
        // A drop-down that UIA calls a button: a button that opens a menu [obs, 1051/s17].
        let styles = netui_focus(&command(
            0x3A,
            "Styles",
            Some(NetuiControl {
                help_text: Some("More".to_owned()),
                control_type: Some(50000),
                ..NetuiControl::default()
            }),
        ));
        assert!(styles.is_some_and(|s| s.role == okur_core::Role::Button && s.opens_menu));
    }

    /// Word's Styles gallery: an MSAA list item in a list, named by its description without
    /// the preview mark, placed among the list's items, on the ribbon's menus [obs, 1064/s4].
    #[test]
    fn a_ribbon_gallerys_item_is_a_list_item_with_its_place() {
        let item = |name: &str| MsaaNode {
            role: Some(0x22),
            name: Some(name.to_owned()),
            states: 0,
            value: None,
            description: None,
            children: Vec::new(),
        };
        let button = MsaaNode {
            role: Some(0x2B),
            ..item("More")
        };
        let event = TraceEvent {
            window_class: Some("NetUIHWND".to_owned()),
            object: Some(TraceObject {
                role: Some(0x22),
                name: Some("¶ Heading 2".to_owned()),
                value: None,
                states: 0x30_0000,
                shortcut: None,
            }),
            focus_chain: vec![node(UIA_WINDOW, "sr-word-fixture.docx - Word", false)],
            msaa_siblings: vec![
                button,
                item("¶ Heading 1"),
                item("¶ Heading 2"),
                item("Subtle Reference"),
            ],
            msaa_ancestors: vec![MsaaNode {
                role: Some(0x21),
                ..item("Styles")
            }],
            msaa_description: Some("Heading 2".to_owned()),
            ..focus_event(Vec::new())
        };
        let focus = netui_focus(&event);
        assert_eq!(
            focus.as_ref().map(|f| (
                f.role.clone(),
                f.name.as_deref(),
                f.item_index,
                f.item_count,
                f.in_ribbon && f.ribbon_menu
            )),
            Some((
                okur_core::Role::ListItem,
                Some("Heading 2"),
                Some(2),
                Some(3),
                true
            ))
        );
    }

    /// A focus in a window whose owner stays available is in a modeless dialog (Word's Find and
    /// Replace); one whose owner it disables, or a window without one, is not [obs, 1065/s6].
    #[test]
    fn a_window_that_leaves_its_owner_available_is_modeless() {
        let modeless = |owner_enabled: Option<bool>| {
            focus_event_from_trace(&TraceEvent {
                top_window_owner_enabled: owner_enabled,
                ..focus_event(Vec::new())
            })
            .map(|f| f.modeless)
        };
        assert_eq!(modeless(Some(true)), Some(true));
        assert_eq!(modeless(Some(false)), Some(false));
        assert_eq!(modeless(None), Some(false));
    }

    /// Word's Font box, a RichEdit window of its own that UIA reaches inside the ribbon's
    /// group: on the ribbon's menus, its group the ribbon's tool bar [obs, 1045/s7].
    #[test]
    fn a_word_ribbon_box_is_on_the_ribbons_menus() {
        let classed = |control_type: i32, name: &str, class: &str| UiaNode {
            class_name: Some(class.to_owned()),
            ..node(control_type, name, false)
        };
        let mut event = TraceEvent {
            window_class: Some("RICHEDIT60W".to_owned()),
            focus_chain: vec![
                node(UIA_WINDOW, "sr-word-fixture.docx - Word", false),
                classed(50033, "Lower Ribbon", "NetUIPanViewer"),
                classed(50025, "Home", "NetUIOrderedGroup"),
                classed(50026, "Font", NETUI_RIBBON_GROUP_CLASS),
                classed(UIA_COMBOBOX, "Font:", "NetUIComboboxAnchor"),
                UiaNode {
                    value: Some("Calibri".to_owned()),
                    ..classed(UIA_EDIT, "Font:", "NetUITextbox")
                },
            ],
            ..focus_event(Vec::new())
        };
        let focus = focus_event_from_trace(&event);
        assert!(focus.is_some());
        let Some(focus) = focus else { return };
        assert_eq!(focus.container_role, Some(okur_core::Role::ComboBox));
        assert!(focus.in_ribbon && focus.ribbon_menu);
        assert!(focus.groups.iter().any(|g| g.name == "Font" && g.tool_bar));
        // Outside a ribbon's group, a combo box's field is no ribbon's.
        if let Some(group) = event.focus_chain.get_mut(3) {
            group.class_name = None;
        }
        assert!(focus_event_from_trace(&event).is_some_and(|f| !f.in_ribbon));
    }

    /// Word's ribbon taking the focus back from its Font box reports the ribbon itself, with
    /// none of its tabs focused, then the command that has the focus: the ribbon is a tab panel
    /// on the ribbon's menus, so that passing it leaves them not ("Leaving menus" was said
    /// there [obs, 1045/s9]); Word's module names it on the way from the search box only
    /// [obs, 1051/s12].
    #[test]
    fn the_ribbon_taking_the_focus_back_is_on_its_menus() {
        let event = TraceEvent {
            window_class: Some("NetUIHWND".to_owned()),
            id_object: OBJID_CLIENT,
            object: Some(TraceObject {
                role: Some(MSAA_PROPERTYPAGE),
                name: Some("Ribbon".to_owned()),
                value: Some("Ribbon".to_owned()),
                states: 0,
                shortcut: None,
            }),
            focus_chain: vec![
                node(UIA_WINDOW, "sr-word-fixture.docx - Word", false),
                node(50033, "", false),
            ],
            ..focus_event(Vec::new())
        };
        let focus = focus_event_from_trace(&event);
        assert!(focus.is_some_and(|f| f.role == okur_core::Role::TabPanel
            && f.name.as_deref() == Some("Ribbon")
            && f.in_ribbon
            && f.ribbon_menu));
    }

    /// Word's search box beside the ribbon's tabs: in the ribbon, not on its menus, in the menu
    /// item that holds it [obs, 1051/s11].
    #[test]
    fn words_search_box_is_in_its_menu_item() {
        let classed = |control_type: i32, name: &str, class: &str| UiaNode {
            class_name: Some(class.to_owned()),
            ..node(control_type, name, false)
        };
        let event = TraceEvent {
            window_class: Some("RICHEDIT60W".to_owned()),
            focus_chain: vec![
                node(UIA_WINDOW, "sr-word-fixture.docx - Word", false),
                classed(50018, "Ribbon Tabs", NETUI_RIBBON_TABS_CLASS),
                classed(
                    50011,
                    "Tell me what you want to do...",
                    "NetUISearchBoxAnchor",
                ),
                classed(UIA_EDIT, "Tell me what you want to do...", "NetUITextbox"),
            ],
            ..focus_event(Vec::new())
        };
        let focus = focus_event_from_trace(&event);
        assert!(focus.is_some());
        let Some(focus) = focus else { return };
        assert!(focus.in_ribbon && !focus.ribbon_menu);
        assert_eq!(focus.container_role, Some(okur_core::Role::MenuItem));
        assert_eq!(
            focus.container_name.as_deref(),
            Some("Tell me what you want to do...")
        );
    }

    /// In Word's ribbon, UI Automation's own focus events (passed on as WinEvents, naming no
    /// MSAA object) and a command named twice by Word say nothing more: each command was said
    /// with "Leaving menus" after it, Cut and the Home tab twice [obs, 1045/s1-s12].
    #[test]
    fn a_word_ribbon_command_is_one_focus() {
        let command = |id_object: i32, name: Option<&str>| TraceEvent {
            window_class: Some("NetUIHWND".to_owned()),
            id_object,
            object: name.map(|name| TraceObject {
                role: Some(0x2B),
                name: Some(name.to_owned()),
                value: None,
                states: 0x10_0005,
                shortcut: None,
            }),
            focus_chain: vec![
                node(UIA_WINDOW, "sr-word-fixture.docx - Word", false),
                node(50033, "", false),
            ],
            ..focus_event(Vec::new())
        };
        let mut converter = EventConverter::default();
        let focuses = |converted: Vec<okur_core::AccessibilityEvent>| {
            converted
                .into_iter()
                .filter_map(|e| match e {
                    okur_core::AccessibilityEvent::Focus(f) => f.name,
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            focuses(converter.convert(&command(101_581_535, Some("Cut")))),
            ["Cut"]
        );
        assert!(focuses(converter.convert(&command(11_080, None))).is_empty());
        assert!(focuses(converter.convert(&command(101_581_535, Some("Cut")))).is_empty());
        assert_eq!(
            focuses(converter.convert(&command(103_350_800, Some("Copy")))),
            ["Copy"]
        );
    }

    /// Classic RichEdit windows, not Windows 11 Notepad's, which names itself, nor plain edits.
    #[test]
    fn classic_rich_edits_are_told_apart() {
        for class in ["RICHEDIT50W", "RichEdit20W", "RichEdit20A", "RichEdit"] {
            assert!(rich_edit(Some(class)), "{class}");
        }
        for class in ["RichEditD2DPT", "Edit", "WindowsForms10.EDIT.app.0.1"] {
            assert!(!rich_edit(Some(class)), "{class}");
        }
    }

    /// WordPad's ribbon given the focus by Alt: its focused tab, with its place among the tabs
    /// shown and the ribbon's panes and row of tabs as the groups entered [obs, 970/s1].
    #[test]
    fn alt_on_a_ribbon_focuses_its_selected_tab() {
        let msaa = |role: i32, name: &str, states: u32, children: Vec<MsaaNode>| MsaaNode {
            role: Some(role),
            name: (!name.is_empty()).then(|| name.to_owned()),
            states,
            value: None,
            description: None,
            children,
        };
        let tabs = msaa(
            0x3C,
            "Ribbon tabs",
            0,
            vec![msaa(
                0x0A,
                "",
                0,
                vec![
                    msaa(0x25, "Home", 0x30_0006, Vec::new()),
                    msaa(0x25, "View", 0x30_0000, Vec::new()),
                    msaa(0x25, "Print preview", 0x8001, Vec::new()),
                ],
            )],
        );
        let pane = |name: &str| node(50033, name, false);
        let event = TraceEvent {
            window_class: Some("NetUIHWND".to_owned()),
            object: Some(TraceObject {
                role: Some(0x26),
                name: Some("Ribbon".to_owned()),
                value: Some("Ribbon".to_owned()),
                states: 0,
                shortcut: None,
            }),
            focus_chain: vec![
                node(UIA_WINDOW, "sr-wordpad-fixture.rtf - WordPad", false),
                pane("UIRibbonDockTop"),
                pane("Ribbon"),
                pane("Ribbon"),
                pane(""),
                pane("Ribbon"),
            ],
            msaa_tree: Some(msaa(0x26, "Ribbon", 0, vec![tabs])),
            ..focus_event(Vec::new())
        };
        let focus = netui_focus(&event);
        assert!(focus.is_some());
        let Some(focus) = focus else { return };
        assert_eq!(focus.role, okur_core::Role::Tab);
        assert_eq!(focus.name.as_deref(), Some("Home"));
        assert_eq!(focus.states.selected, Some(true));
        assert_eq!((focus.item_index, focus.item_count), (Some(1), Some(2)));
        assert!(focus.in_ribbon);
        let groups: Vec<(&str, bool)> = focus
            .groups
            .iter()
            .map(|g| (g.name.as_str(), g.tab_list))
            .collect();
        assert_eq!(
            groups,
            [
                ("UIRibbonDockTop", false),
                ("Ribbon", false),
                ("Ribbon", false),
                ("Ribbon", false),
                ("Ribbon tabs", true)
            ]
        );
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
    /// 2026-10-17: "Okur Settings dialog | Speech Synthesizer: Combo box"). Its labels give
    /// their combo boxes access keys ("&Voice:").
    #[test]
    fn a_dialogs_labels_are_not_its_text() {
        let synthesizer = UiaNode {
            access_key: Some("Alt+s".to_owned()),
            ..node(UIA_COMBOBOX, "Synthesizer:", true)
        };
        let settings = TraceEvent {
            focus_chain: vec![dialog_node(UIA_WINDOW, "Okur Settings"), synthesizer],
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

    /// A text naming the focused field is the dialog's prompt when the field takes no access
    /// key from it ("Custom Label dialog | Enter a custom label for this item | Enter a custom
    /// label for this item edit" [obs, 688/s9]), and the field's label when it does ("Font
    /// dialog | Font: edit combo" [obs, 541/s2]).
    #[test]
    fn a_prompt_naming_the_focused_field_is_the_dialogs_text() {
        let prompt = "Enter a custom label for this item";
        let field = UiaNode {
            runtime_id: Some(vec![42, 7]),
            ..node(UIA_DOCUMENT, prompt, true)
        };
        let tree = UiaNode {
            children: vec![
                node(UIA_TEXT, prompt, false),
                field.clone(),
                node(UIA_BUTTON, "OK", false),
            ],
            ..dialog_node(UIA_WINDOW, "Custom Label")
        };
        let label = TraceEvent {
            focus_chain: vec![dialog_node(UIA_WINDOW, "Custom Label"), field.clone()],
            dialog_text: vec![prompt.to_owned()],
            dialog_tree: Some(tree.clone()),
            ..focus_event(Vec::new())
        };
        assert_eq!(
            focus_event_from_trace(&label).and_then(|f| f.dialog_text),
            Some(prompt.to_owned())
        );
        let keyed = UiaNode {
            access_key: Some("Alt+e".to_owned()),
            ..field
        };
        let labelled = TraceEvent {
            focus_chain: vec![dialog_node(UIA_WINDOW, "Custom Label"), keyed],
            ..label
        };
        assert_eq!(
            focus_event_from_trace(&labelled).and_then(|f| f.dialog_text),
            Some(String::new())
        );
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

    /// A wizard takes its page's title as its own and shows no tabs: its page is not named,
    /// and the page's text is the dialog's ("Flexible Web dialog | What would you like to do?"
    /// [obs, 705/s3]), where a tabbed sheet's page is named and its labels are not the
    /// dialog's text.
    #[test]
    fn a_wizards_page_is_its_dialog() {
        const UIA_RADIOBUTTON: i32 = 50013;
        let wizard = TraceEvent {
            focus_chain: vec![
                dialog_node(UIA_WINDOW, "Flexible Web"),
                node(UIA_RADIOBUTTON, "Create a new customization", true),
            ],
            focus_siblings: vec![
                node(TEXT, "What would you like to do?", false),
                node(UIA_RADIOBUTTON, "Create a new customization", true),
                node(UIA_BUTTON, "Next >", false),
            ],
            top_window_title: Some("Flexible Web".to_owned()),
            top_window_class: Some("#32770".to_owned()),
            dialog_page: Some("Flexible Web".to_owned()),
            ..focus_event(Vec::new())
        };
        let focus = focus_event_from_trace(&wizard);
        assert_eq!(
            focus.as_ref().map(|f| (
                f.dialog_page.clone(),
                f.wizard_page.clone(),
                f.dialog_text.clone()
            )),
            Some((
                None,
                Some("Flexible Web".to_owned()),
                Some("What would you like to do?".to_owned())
            ))
        );
        let tabbed = focus_event_from_trace(&property_page_event());
        assert_eq!(
            tabbed.map(|f| (f.dialog_page, f.wizard_page)),
            Some((Some("General".to_owned()), None))
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
                shortcut: None,
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
        // Live, the reader decides from the same focus before reading the change further
        // (performance review P5, step 2): the converter's focus is the one it checks.
        let named = |n: &str| TraceObject {
            name: Some(n.to_owned()),
            ..TraceObject::default()
        };
        let identity = (7, focus.id_object, -16936);
        let page = "Chrome_RenderWidgetHostHWND";
        let kept = |event: &str, identity, object: Option<&TraceObject>| {
            super::live_change_kept(event, identity, object, page, false)
        };
        assert!(kept("state_change", identity, Some(&named("Solutions"))));
        assert!(!kept("state_change", identity, Some(&named("Pricing"))));
        // The focus's own object, whatever its name now.
        assert!(kept(
            "state_change",
            (focus.hwnd, focus.id_object, focus.id_child),
            None
        ));
        // UI Automation's own value change in the focused window names no MSAA object; one in
        // another window, or a state change, is not kept.
        assert!(kept("value_change", (focus.hwnd, 414, 0), None));
        assert!(!kept("state_change", (focus.hwnd, 414, 0), None));
        assert!(!kept("value_change", (focus.hwnd + 1, 414, 0), None));
        // ... unless that window has the keyboard focus.
        assert!(super::live_change_kept(
            "value_change",
            (focus.hwnd + 1, 414, 0),
            None,
            page,
            true
        ));
        // A page's progress bar or meter, anywhere in the page.
        let meter = TraceObject {
            role: Some(super::ROLE_SYSTEM_PROGRESSBAR),
            ..named("CPU")
        };
        assert!(kept("value_change", identity, Some(&meter)));
        assert!(!kept("state_change", identity, Some(&meter)));
        assert!(!super::live_change_kept(
            "value_change",
            identity,
            Some(&meter),
            "Notepad",
            false
        ));
    }

    /// Space on a select with the focus: its state change is read after its list has taken
    /// the focus, so the focus chain ends in the list's item. The change is the select's, the
    /// event's object, expanded by its MSAA states [obs, 276/s7].
    #[test]
    fn a_change_read_after_the_focus_moved_on_is_the_focused_controls() {
        const COMBO_BOX: i32 = 50003;
        const EXPANDED: u32 = 0x200;
        let select = node(COMBO_BOX, "Home branch", true);
        let object = |states: u32| {
            Some(TraceObject {
                role: Some(46),
                name: Some("Home branch".to_owned()),
                value: Some("Northside".to_owned()),
                states,
                shortcut: None,
            })
        };
        let focus = TraceEvent {
            object: object(0),
            focus_chain: vec![select.clone()],
            ..focus_event(Vec::new())
        };
        let expanded = TraceEvent {
            event: "state_change".to_owned(),
            object: object(EXPANDED),
            focus_chain: vec![select, node(UIA_LISTITEM, "Northside", true)],
            ..focus_event(Vec::new())
        };
        let mut converter = EventConverter::default();
        assert_eq!(converter.convert(&focus).len(), 1);
        assert!(
            matches!(
                converter.convert(&expanded).as_slice(),
                [okur_core::AccessibilityEvent::Change { control, .. }]
                    if control.role == okur_core::Role::ComboBox
                        && control.states.expanded == Some(true)
            ),
            "the select, expanded"
        );
    }

    /// A file dialog's folder is its address bar's toolbar's name [obs, 524/s3].
    #[test]
    fn a_file_dialogs_folder_is_its_address_bars_name() {
        let address = UiaNode {
            class_name: Some("ToolbarWindow32".to_owned()),
            automation_id: Some("1001".to_owned()),
            ..node(50021, "Address: C:\\Temp", false)
        };
        let navigation = UiaNode {
            class_name: Some("ToolbarWindow32".to_owned()),
            automation_id: Some("41477".to_owned()),
            ..node(50021, "Navigation buttons", false)
        };
        let band = UiaNode {
            children: vec![navigation, address],
            ..node(50033, "", false)
        };
        let dialog = UiaNode {
            children: vec![band],
            ..node(UIA_WINDOW, "Save As", false)
        };
        assert_eq!(address_band(&dialog).as_deref(), Some("Address: C:\\Temp"));
        assert_eq!(address_band(&node(UIA_WINDOW, "Find", false)), None);
    }

    /// Tab onto a file dialog's toolbar button: UIA's focus stays on the toolbar, the MSAA
    /// event's object is the button. The focus is the button in a toolbar [obs, 524/s6].
    #[test]
    fn a_toolbar_button_focused_through_msaa_is_the_focus() {
        let toolbar = TraceEvent {
            window_class: Some("ToolbarWindow32".to_owned()),
            id_child: 1,
            object: Some(TraceObject {
                role: Some(0x2B),
                name: Some("Hide Folders".to_owned()),
                value: None,
                states: 0,
                shortcut: None,
            }),
            focus_chain: vec![node(UIA_WINDOW, "Save As", false), node(50021, "", true)],
            ..focus_event(Vec::new())
        };
        let focus = focus_event_from_trace(&toolbar);
        assert!(
            focus
                .as_ref()
                .is_some_and(|f| f.role == okur_core::Role::Button
                    && f.name.as_deref() == Some("Hide Folders")
                    && f.container_role == Some(okur_core::Role::ToolBar)),
            "{focus:?}"
        );
        // The toolbar's own focus (no child), which the button's follows, is not one; the
        // toolbar is said by its role alone [obs, 3156/s3].
        let own = TraceEvent {
            id_child: 0,
            ..toolbar.clone()
        };
        assert!(focus_event_from_trace(&own).is_none());
        let named = TraceEvent {
            focus_chain: vec![
                node(UIA_WINDOW, "Programs and Features", false),
                node(50021, "Up band toolbar", true),
            ],
            ..toolbar
        };
        assert!(focus_event_from_trace(&named).is_some_and(|f| f.container_name.is_none()));
    }

    /// Tab onto a list view's column title: a header bar at its place among the titles, not
    /// in the list [obs, 3156/s2].
    #[test]
    fn a_column_title_is_a_header_bar() {
        const SPLIT_BUTTON: i32 = 50031;
        let titles: Vec<UiaNode> = ["Name", "Publisher", "Installed On"]
            .iter()
            .map(|name| node(SPLIT_BUTTON, name, false))
            .collect();
        let header = UiaNode {
            class_name: Some("SysHeader32".to_owned()),
            ..node(UIA_HEADER, "Header Control", false)
        };
        let list = UiaNode {
            class_name: Some("SysListView32".to_owned()),
            ..node(UIA_LIST, "Folder View", false)
        };
        let event = TraceEvent {
            window_class: Some("SysHeader32".to_owned()),
            focus_chain: vec![
                node(UIA_WINDOW, "Programs and Features", false),
                list,
                header,
                node(SPLIT_BUTTON, "Publisher", true),
            ],
            focus_siblings: titles,
            ..focus_event(Vec::new())
        };
        let focus = focus_event_from_trace(&event);
        assert!(
            focus.as_ref().is_some_and(|f| f.header_bar
                && f.item_index == Some(2)
                && f.item_count == Some(3)
                && f.container_role.is_none()),
            "{focus:?}"
        );
    }

    /// Down in the Font dialog's font field: its open list's item takes the WinEvent focus
    /// while the focus stays in the field. It is a selection, with its index [obs, 541/s3].
    #[test]
    fn an_edit_combos_list_item_is_a_selection() {
        const COMBO_BOX: i32 = 50003;
        const EDIT: i32 = 50004;
        const MSAA_LISTITEM: i32 = 0x22;
        let field = TraceEvent {
            focus_chain: vec![node(COMBO_BOX, "Font:", false), node(EDIT, "Font:", true)],
            ..focus_event(Vec::new())
        };
        let item = TraceEvent {
            window_class: Some("ComboLBox".to_owned()),
            id_child: 39,
            object: Some(TraceObject {
                role: Some(MSAA_LISTITEM),
                name: Some("Constantia".to_owned()),
                value: None,
                states: 0,
                shortcut: None,
            }),
            focus_chain: field.focus_chain.clone(),
            ..focus_event(Vec::new())
        };
        let mut converter = EventConverter::default();
        assert_eq!(converter.convert(&field).len(), 1);
        assert!(
            matches!(
                converter.convert(&item).as_slice(),
                [okur_core::AccessibilityEvent::Selected(selected)]
                    if selected.name.as_deref() == Some("Constantia")
                        && selected.item_index == Some(39)
            ),
            "the font, selected"
        );
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
                shortcut: None,
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
                misspelled: Vec::new(),
                structure: None,
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

    /// A value change with the focused field's text is the field's only when it is about the
    /// focus: Word's ribbon changing its Font box as the caret crosses a style is not the
    /// document's text (performance review 2026-10-25, finding 2).
    #[test]
    fn another_objects_value_change_is_not_the_fields_text() {
        let text = || {
            Some(okur_core::TextContext {
                text: "Library Hours".to_owned(),
                caret: 0,
                selection: None,
                font: None,
                word: None,
                misspelled: Vec::new(),
                structure: None,
            })
        };
        let document = TraceEvent {
            hwnd: 1,
            object: Some(TraceObject {
                role: Some(10),
                name: Some("Microsoft Word Document".to_owned()),
                value: None,
                states: 0,
                shortcut: None,
            }),
            focus_chain: vec![node(UIA_DOCUMENT, "sr-word-fixture.docx", true)],
            text_context: text(),
            ..focus_event(Vec::new())
        };
        let value_change = |hwnd: isize, id_object: i32, name: &str| TraceEvent {
            event: "value_change".to_owned(),
            hwnd,
            id_object,
            object: Some(TraceObject {
                role: Some(42),
                name: Some(name.to_owned()),
                value: Some("Calibri Light".to_owned()),
                states: 0,
                shortcut: None,
            }),
            text_context: text(),
            ..focus_event(Vec::new())
        };
        let mut converter = EventConverter::default();
        assert_eq!(converter.convert(&document).len(), 1);
        assert!(converter
            .convert(&value_change(2, 103_351_241, "Font:"))
            .is_empty());
        assert!(matches!(
            converter
                .convert(&value_change(1, -4, "Microsoft Word Document"))
                .as_slice(),
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

    /// Excel's Go To: the dialog's MSAA names its field, "Reference:", where UI Automation has
    /// only the field's window, a pane named by its text; the field's own report between the
    /// dialog's two is none [obs, 1124/s1, s8].
    #[test]
    fn an_office_dialog_field_is_its_label_as_msaa_has_it() {
        const PANE: i32 = 50033;
        let dialog = UiaNode {
            control_type: UIA_WINDOW,
            name: Some("Go To".to_owned()),
            class_name: Some("bosa_sdm_XL9".to_owned()),
            is_dialog: Some(true),
            ..UiaNode::default()
        };
        let field = |text: &str| UiaNode {
            control_type: PANE,
            name: (!text.is_empty()).then(|| text.to_owned()),
            class_name: Some("EDTBX".to_owned()),
            has_keyboard_focus: true,
            ..UiaNode::default()
        };
        let reports = |text: &str| {
            let chain = vec![dialog.clone(), field(text)];
            let from_dialog = TraceEvent {
                window_class: Some("bosa_sdm_XL9".to_owned()),
                window_title: Some("Go To".to_owned()),
                id_child: 5,
                object: Some(TraceObject {
                    role: Some(MSAA_TEXT),
                    name: Some("Reference:".to_owned()),
                    value: (!text.is_empty()).then(|| text.to_owned()),
                    states: 0,
                    shortcut: Some("Alt+R".to_owned()),
                }),
                focus_chain: chain.clone(),
                ..focus_event(Vec::new())
            };
            let from_field = TraceEvent {
                window_class: Some("EDTBX".to_owned()),
                object: Some(TraceObject {
                    role: Some(0x0A),
                    name: Some(text.to_owned()),
                    value: None,
                    states: 0,
                    shortcut: None,
                }),
                focus_chain: chain,
                ..focus_event(Vec::new())
            };
            (from_dialog, from_field)
        };
        for text in ["", "A1"] {
            let (from_dialog, from_field) = reports(text);
            let converted = EventConverter::default().convert(&from_dialog);
            let value = (!text.is_empty()).then_some(text);
            assert!(
                matches!(converted.as_slice(), [okur_core::AccessibilityEvent::Focus(f)]
                    if f.role == okur_core::Role::Edit
                        && f.name.as_deref() == Some("Reference:")
                        && f.value.as_deref() == value),
                "{converted:?}"
            );
            assert!(EventConverter::default().convert(&from_field).is_empty());
        }
    }

    /// Delete empties Excel's cell: UI Automation's focus is the grid then, which reports only
    /// its value's change; that change is the focus moving to it [obs, 1123/s7].
    #[test]
    fn a_value_change_that_moves_the_focus_is_a_focus() {
        const CELL: i32 = 50029;
        const TAB: i32 = 50018;
        let grid = UiaNode {
            control_type: TAB,
            name: Some("Book1.xlsx".to_owned()),
            class_name: Some("EXCEL7".to_owned()),
            has_keyboard_focus: true,
            ..UiaNode::default()
        };
        let cell = TraceEvent {
            hwnd: 7,
            id_object: 412,
            window_class: Some("EXCEL7".to_owned()),
            focus_chain: vec![node(CELL, "D1", true)],
            ..focus_event(Vec::new())
        };
        let changed = TraceEvent {
            event: "value_change".to_owned(),
            win_event: 0x800E,
            id_object: 414,
            focus_chain: vec![grid.clone()],
            ..cell.clone()
        };
        let mut converter = EventConverter::default();
        assert_eq!(converter.convert(&cell).len(), 1);
        let moved = converter.convert(&changed);
        assert!(
            matches!(moved.as_slice(), [okur_core::AccessibilityEvent::Focus(f)]
                if f.name.as_deref() == Some("Book1.xlsx")),
            "{moved:?}"
        );
        // The grid's own value changing again moves the focus nowhere.
        assert!(!converter
            .convert(&changed)
            .iter()
            .any(|e| matches!(e, okur_core::AccessibilityEvent::Focus(_))));
    }
}
