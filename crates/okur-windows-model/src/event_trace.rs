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
/// The window class of comctl32's tool tips, balloon tips among them; WinForms' own is named
/// after it ("WindowsForms10.tooltips_class32...").
pub const TOOL_TIP_CLASS: &str = "tooltips_class32";
/// A Win32 popup menu's window.
const WIN32_MENU_CLASS: &str = crate::classes::MENU;
/// The top window class of a focus on the desktop (`PlatformIds::top_window_class`).
pub const DESKTOP_KIND: &str = "Progman";
/// MSAA `ROLE_SYSTEM_MENUITEM`.
const MSAA_MENUITEM: i32 = 0x0C;
/// MSAA `ROLE_SYSTEM_BUTTONMENU`.
const MSAA_BUTTONMENU: i32 = 0x39;
/// MSAA `ROLE_SYSTEM_PROPERTYPAGE`: a NetUI ribbon itself, or one of its tabs' pages.
const MSAA_PROPERTYPAGE: i32 = 0x26;
/// The object id of a window's client area in a WinEvent (`OBJID_CLIENT`).
pub const OBJID_CLIENT: i32 = -4;
/// UIA's class of a group of Office's ribbon (its "chunk": Word's Font group).
const NETUI_RIBBON_GROUP_CLASS: &str = "NetUIChunk";
/// The class of Office's panes as UI Automation reaches them: the row of ribbon tabs, where its
/// search box is, is the one that is a tab control.
const NETUI_RIBBON_TABS_CLASS: &str = "NetUIPanViewer";
/// The pane the Windows Ribbon framework (File Explorer's, WordPad's, Paint's ribbon) puts its
/// ribbon in, where Office's is `MsoWorkPane`.
const WINDOWS_RIBBON_PANE_CLASS: &str = "UIRibbonWorkPane";

/// Bumped whenever the recorded fields or their meaning change.
pub const TRACE_FORMAT_VERSION: u32 = 11;

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

mod controls;
mod converter;
mod focus;
mod office;
#[cfg(test)]
mod tests;

use controls::*;
pub use converter::*;
pub use focus::*;
pub use office::*;

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
    /// For a focus in a Store app's frame: the texts directly in its core window, in order.
    /// Windows Settings gives its own and its page's category ("Settings", "System"), which
    /// JAWS's Insert+T says [obs, 3158/s14]. Recorded from trace format 11.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub app_window_texts: Vec<String>,
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
/// Whether the focus is in a dialog box: the innermost window of its chain is a Win32
/// `#32770`, or one UI Automation calls a dialog (Word's, WPF's). Its static text and buttons
/// are read from the focus's siblings.
pub fn in_dialog_box(chain: &[UiaNode]) -> bool {
    chain
        .iter()
        .rev()
        .find(|w| w.control_type == UIA_WINDOW)
        .is_some_and(|w| {
            w.class_name.as_deref() == Some(WIN32_DIALOG_CLASS) || w.is_dialog == Some(true)
        })
}

/// Whether the focused element reports its own position ("2 of 5"), so that its siblings
/// need not be counted.
pub fn reports_position(chain: &[UiaNode]) -> bool {
    chain
        .last()
        .is_some_and(|f| f.position_in_set.is_some() && f.size_of_set.is_some())
}

/// Whether a focus's siblings are read for it, given its chain (outermost first), whether its
/// Win32 list gave its position, and its MSAA role. The conversion takes from them a dialog box's text and
/// buttons, a group box's texts, the label before an unnamed field, a header's titles, and a
/// place among peers where none is reported; a button, a tab or a split button takes no place
/// from its peers, so outside a dialog box, a group box and a header it needs none of them: a page's button read about
/// twelve siblings at each focus for nothing (second review, 2026-11-03).
/// `siblings_are_unused_where_they_are_not_read` checks this against every traced focus.
pub fn focus_needs_siblings(
    chain: &[UiaNode],
    list_position: bool,
    msaa_role: Option<i32>,
) -> bool {
    if in_dialog_box(chain) {
        return true;
    }
    if list_position || reports_position(chain) {
        return false;
    }
    let Some(focused) = chain.last() else {
        return false;
    };
    // A group box's control is named after its texts; a header's title (a split button in
    // Programs and Features' list) is at its place among the titles [obs, 3156/s2].
    let in_group_box_or_header = chain.iter().rev().nth(1).is_some_and(|parent| {
        (parent.control_type == 50026 && parent.class_name.as_deref() == Some("GroupBox"))
            || parent.control_type == UIA_HEADER
    });
    let mut alone = focused.clone();
    alone.children.clear();
    // A button MSAA calls a button menu is one, which counts its place.
    let placeless = msaa_role != Some(MSAA_BUTTONMENU)
        && matches!(
            crate::uia_snapshot::to_aom(&alone).role,
            okur_core::Role::Button
                | okur_core::Role::Tab
                | okur_core::Role::SplitButton
                | okur_core::Role::TableCell
        );
    in_group_box_or_header || !placeless
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
