//! Accessibility event traces.
//!
//! `sr-cli trace` records the system-wide accessibility event stream while a JAWS trial runs
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
const UIA_TREEITEM: i32 = 50024;
const UIA_TEXT: i32 = 50020;
const UIA_BUTTON: i32 = 50000;
/// The window class of Win32 dialog boxes (message boxes, common dialogs).
const WIN32_DIALOG_CLASS: &str = "#32770";
/// The window kind of focus on the desktop ([`sr_core::FocusEvent::window_kind`]).
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

/// Whether a recorded event is a focus change, from a WinEvent or from UIA.
pub fn is_focus(event: &TraceEvent) -> bool {
    matches!(event.event.as_str(), "focus" | "uia_focus")
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
    /// For focus on a Win32 combo box or list box: the selection as `[index, count]`,
    /// with a 0-based index, asked of the control itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_position: Option<[i32; 2]>,
    /// For focus, caret and change events: the focused text field's text, caret and
    /// selection (`text_context::read_focused`), from which edit keys are spoken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_context: Option<sr_core::TextContext>,
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
pub fn focus_event_from_chain(
    chain: &[UiaNode],
    window_class: Option<&str>,
    window_title: Option<&str>,
) -> Option<sr_core::FocusEvent> {
    use sr_core::Role;
    let focused = chain.last()?;
    let mut leaf = focused.clone();
    leaf.children.clear();
    let aom = crate::uia_snapshot::to_aom(&leaf);

    let mut event = sr_core::FocusEvent::new(aom.role.clone());
    event.name = aom.name.clone();
    event.value = aom.value.clone();
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
    event.window_class = window_class.map(str::to_owned);
    event.window_title = window_title.map(str::to_owned);
    // Inside a browser document, Chrome reports the DOM id as AutomationId.
    // A page: a document in a browser's web content window. Documents elsewhere (Windows 11
    // Notepad, Word) are native text controls, read by caret, not by the virtual cursor.
    event.in_document = window_class.is_some_and(|c| WEB_CONTENT_WINDOW_CLASSES.contains(&c))
        && chain.iter().any(|a| a.control_type == UIA_DOCUMENT);
    if event.in_document {
        event.dom_id = focused.automation_id.clone().filter(|id| !id.is_empty());
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
            event.container_class = a.class_name.clone();
            event.container_role = Some(role);
            break;
        }
    }
    Some(event)
}

/// The focus event a recorded `focus` trace event delivers to the engine: the chain mapping
/// of [`focus_event_from_chain`], plus the context a live listener would query.
///
/// Without a provider PositionInSet (WinForms and most Win32 controls report none), the
/// position is counted among the focused element's siblings of the same control type.
pub fn focus_event_from_trace(event: &TraceEvent) -> Option<sr_core::FocusEvent> {
    // Menus: UIA keeps reporting the control focused before the menu (WinForms MenuStrip),
    // while the MSAA focus event names the menu item, as JAWS reads it [obs, 302].
    if let Some(object) = event
        .object
        .as_ref()
        .filter(|o| o.role == Some(MSAA_MENUITEM))
    {
        return Some(
            sr_core::FocusEvent::new(sr_core::Role::MenuItem)
                .with_name(object.name.clone().unwrap_or_default()),
        );
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
    let top_window = event
        .virtual_desktop
        .as_deref()
        .or(event.top_window_title.as_deref())
        .or(active_window.and_then(|w| w.name.as_deref()));
    let mut focus = focus_event_from_chain(
        &event.focus_chain,
        event.window_class.as_deref(),
        top_window,
    )?;
    let focused = event.focus_chain.last()?;
    // The window's kind is its class: the desktop's, the window in front, or (older traces)
    // the UIA window around the focus.
    focus.window_kind = if event.virtual_desktop.is_some() {
        Some(DESKTOP_KIND.to_owned())
    } else {
        event
            .top_window_class
            .clone()
            .or_else(|| active_window.and_then(|w| w.class_name.clone()))
    };
    // A dialog box (Win32 #32770, as a message box is) is announced with its static text,
    // which sits beside its buttons [obs, 306/s1]. Other windows are announced by title only
    // (306/s10).
    if active_window.and_then(|w| w.class_name.as_deref()) == Some(WIN32_DIALOG_CLASS) {
        let text: Vec<&str> = event
            .focus_siblings
            .iter()
            .filter(|s| s.control_type == UIA_TEXT)
            .filter_map(|s| s.name.as_deref())
            .filter(|n| !n.trim().is_empty())
            .collect();
        focus.dialog_text = Some(text.join("\n"));
        focus.dialog_buttons = event
            .focus_siblings
            .iter()
            .filter(|s| s.control_type == UIA_BUTTON)
            .filter_map(|s| s.name.clone())
            .collect();
    }
    focus.dialog_lines = event.dialog_lines.clone();
    // JAWS names buttons by their MSAA role: ROLE_SYSTEM_BUTTONMENU is a "button menu"
    // even when UIA reports a plain button [obs, 204/s11–s12].
    if focus.role == sr_core::Role::Button
        && event.object.as_ref().and_then(|o| o.role) == Some(MSAA_BUTTONMENU)
    {
        focus.role = sr_core::Role::MenuButton;
    }
    if let Some(subtree) = &event.focus_subtree {
        if focus.role == sr_core::Role::TreeItem {
            let children = subtree
                .children
                .iter()
                .filter(|c| c.control_type == UIA_TREEITEM)
                .count();
            focus.child_count = (children > 0).then_some(children);
        }
        // A list-view item's further columns are its text children after the name.
        if focus.role == sr_core::Role::ListItem {
            focus.columns = subtree
                .children
                .iter()
                .filter_map(|c| c.name.clone())
                .filter(|n| !n.trim().is_empty() && Some(n) != focused.name.as_ref())
                .collect();
        }
        // A combo box's position is its selected item's, among its list items.
        if focus.role == sr_core::Role::ComboBox && focus.item_index.is_none() {
            let items: Vec<&UiaNode> = subtree
                .children
                .iter()
                .flat_map(|c| std::iter::once(c).chain(c.children.iter()))
                .filter(|n| n.control_type == UIA_LISTITEM)
                .collect();
            if let Some(i) = items.iter().position(|n| n.is_selected == Some(true)) {
                focus.item_index = Some(i + 1);
                focus.item_count = Some(items.len());
            }
        }
    }
    if let (None, Some([index, count])) = (focus.item_index, event.list_position) {
        focus.item_index = usize::try_from(index).ok().map(|i| i + 1);
        focus.item_count = usize::try_from(count).ok();
    }
    if focus.item_index.is_none() && focus.role != sr_core::Role::ComboBox {
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
pub use capture::{run_live, run_trace};

/// The menu mode transition a recorded event reports, if any.
pub fn menu_event_from_trace(event: &TraceEvent) -> Option<sr_core::MenuEvent> {
    use sr_core::MenuEvent;
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
    last_focus: Option<sr_core::FocusEvent>,
}

impl EventConverter {
    /// The engine events for one captured event, in order (usually one, or none).
    pub fn convert(&mut self, event: &TraceEvent) -> Vec<sr_core::AccessibilityEvent> {
        if event.event == "live_region_change" {
            return live_region_events(event);
        }
        self.convert_one(event).into_iter().collect()
    }

    fn convert_one(&mut self, event: &TraceEvent) -> Option<sr_core::AccessibilityEvent> {
        use sr_core::{AccessibilityEvent, ChangeKind};
        if let Some(menu) = menu_event_from_trace(event) {
            return Some(AccessibilityEvent::Menu(menu));
        }
        let kind = match event.event.as_str() {
            "focus" if is_drop_down_item(event) => {
                // Arrow keys in an open combo box move focus into its drop-down list; JAWS
                // speaks that as the combo box's new value [obs, 304/s12].
                let mut combo = self
                    .last_focus
                    .clone()
                    .filter(|f| f.role == sr_core::Role::ComboBox)?;
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
                if let Some(states) = event.object.as_ref().map(|o| o.states) {
                    apply_msaa_states(&mut focus, states);
                }
                focus.text = event.text_context.clone();
                let name = event.object.as_ref().and_then(|o| o.name.clone());
                self.focused = Some((event.hwnd, event.id_object, event.id_child, name));
                self.last_focus = Some(focus.clone());
                return Some(AccessibilityEvent::Focus(focus));
            }
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
        let same_name =
            name.is_some() && event.object.as_ref().and_then(|o| o.name.as_ref()) == name.as_ref();
        if !(same_object || same_name) {
            return None;
        }
        // The control as UIA reports it now; without a focus chain (a failed UIA read), the
        // last focus with the event object's MSAA states.
        let mut control = match focus_event_from_trace(event) {
            Some(control) => control,
            None => self.last_focus.clone()?,
        };
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

/// A live region changed: its announcement, then the page as it is now, which refreshes the
/// buffer [obs, 206/s2-s10].
fn live_region_events(event: &TraceEvent) -> Vec<sr_core::AccessibilityEvent> {
    use sr_core::{AccessibilityEvent, LiveRegionEvent, Politeness};
    let mut events = Vec::new();
    if let Some(region) = &event.region {
        let mut lines = Vec::new();
        text_lines(region, &mut lines);
        let mut live = LiveRegionEvent::new(lines.join("\n"), Politeness::Polite);
        live.container_name = region.name.clone().filter(|n| !n.trim().is_empty());
        live.region_id = region.attributes.get("id").cloned();
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

/// Updates the states a change can carry from MSAA state bits (`STATE_SYSTEM_*`).
fn apply_msaa_states(control: &mut sr_core::FocusEvent, states: u32) {
    const CHECKED: u32 = 0x10;
    const EXPANDED: u32 = 0x200;
    const COLLAPSED: u32 = 0x400;
    if matches!(
        control.role,
        sr_core::Role::CheckBox | sr_core::Role::RadioButton
    ) {
        control.states.checked = Some(states & CHECKED != 0);
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
        event_name, TraceEvent, TraceHeader, TraceObject, TRACE_FORMAT_VERSION, UIA_FOCUS_WINDOWS,
    };
    use crate::uia_snapshot::capture::{
        automation, focus_chain, focus_chain_from, focus_context, focus_context_of,
        focus_subtree_of, window_text,
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
        IUIAutomationElement, IUIAutomationFocusChangedEventHandler,
        IUIAutomationFocusChangedEventHandler_Impl,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetWindowThreadProcessId, MsgWaitForMultipleObjects, PeekMessageW,
        TranslateMessage, MSG, PM_REMOVE, QS_ALLINPUT,
    };

    const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
    const WINEVENT_SKIPOWNPROCESS: u32 = 0x0002;

    struct TraceState {
        /// Receives every event: the trace file, or the live screen reader.
        sink: Box<dyn FnMut(TraceEvent)>,
        uia: Option<IUIAutomation>,
        count: usize,
        /// Feeding the live screen reader: events it does not use are dropped before any
        /// cross-process read, and live-region pages are captured elsewhere.
        live: bool,
    }

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
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG};
        let upper = class.to_ascii_uppercase();
        let (get_sel, get_count) = if upper.contains("COMBOBOX") {
            (0x0147, 0x0146)
        } else if upper.contains("LISTBOX") {
            (0x0188, 0x018B)
        } else {
            return None;
        };
        let ask = |msg: u32| {
            let mut result = 0usize;
            // SAFETY: a read-only query message with a timeout, to a window that may be gone.
            let sent = unsafe {
                SendMessageTimeoutW(
                    hwnd,
                    msg,
                    WPARAM(0),
                    LPARAM(0),
                    SMTO_ABORTIFHUNG,
                    200,
                    Some(&mut result),
                )
            };
            // CB_ERR/LB_ERR is -1, as the low 32 bits of the result.
            (sent.0 != 0).then_some(result as i32)
        };
        let index = ask(get_sel).filter(|&i| i >= 0)?;
        let count = ask(get_count).filter(|&n| n > 0)?;
        Some([index, count])
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
    const LIVE_EVENTS: [&str; 13] = [
        "focus",
        "uia_focus",
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
        _time: u32,
    ) {
        let Some(name) = event_name(win_event, id_object) else {
            return;
        };
        record(name, win_event, hwnd, id_object, id_child, None);
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

    /// The element of the latest UIA focus change, for the tracer thread: on Windows 11 the
    /// task switcher's items are named only by the event, while `GetFocusedElement` answers
    /// with the switcher's input pane (seen live on the dev box, 2026-10-15).
    static UIA_FOCUSED: std::sync::Mutex<Option<MtaElement>> = std::sync::Mutex::new(None);

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
                    if let (Some(element), Ok(mut slot)) = (sender.as_ref(), UIA_FOCUSED.lock()) {
                        *slot = Some(MtaElement(element.clone()));
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
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
        };
        const OBJID_CLIENT: i32 = -4;
        const EVENT_OBJECT_FOCUS: u32 = 0x8005;
        // SAFETY: plain Win32 queries; the info struct is sized as the call requires.
        let focused = unsafe {
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
        };
        if !focused.is_invalid() {
            record("focus", EVENT_OBJECT_FOCUS, focused, OBJID_CLIENT, 0, None);
        }
    }

    /// Records the focus UIA reported last, on the tracer thread.
    fn record_uia_focus() {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let element = UIA_FOCUSED.lock().ok().and_then(|mut slot| slot.take());
        // SAFETY: plain Win32 call with no preconditions.
        let foreground = unsafe { GetForegroundWindow() };
        let element = element.as_ref().map(|e| &e.0);
        record("uia_focus", UIA_FOCUS_CHANGED, foreground, 0, 0, element);
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
        let t_ms = now_ms();
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
            if state.live
                && (!LIVE_EVENTS.contains(&name)
                    || (changes_focus_context && !in_foreground_process(hwnd)))
            {
                return;
            }
            // Each read's end, in milliseconds since the event arrived, for the debug log when
            // an event holds the thread up (a UIA call waiting out its timeout).
            let clock = std::time::Instant::now();
            let mut stages: Vec<(&str, u128)> = Vec::new();
            let focus = if focused || name == "foreground" || changes_focus_context {
                match (state.uia.as_ref(), element) {
                    (Some(uia), Some(element)) => focus_chain_from(uia, element),
                    (Some(uia), None) => focus_chain(uia),
                    (None, _) => Vec::new(),
                }
            } else {
                Vec::new()
            };
            stages.push(("chain", clock.elapsed().as_millis()));
            let (focus_siblings, focus_subtree) = match (name, state.uia.as_ref()) {
                (_, Some(uia)) if focused => match element {
                    Some(element) => focus_context_of(uia, element),
                    None => focus_context(uia),
                },
                (_, Some(uia)) if changes_focus_context => (Vec::new(), focus_subtree_of(uia)),
                _ => (Vec::new(), None),
            };
            stages.push(("context", clock.elapsed().as_millis()));
            let list_position = if focused || changes_focus_context {
                win32_list_position(hwnd, &class)
            } else {
                None
            };
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
            let dialog_lines = if focused && top_window_class.as_deref() == Some("#32770") {
                use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                // SAFETY: plain Win32 call with no preconditions.
                crate::screen_lines::dialog_lines(unsafe { GetForegroundWindow() })
            } else {
                Vec::new()
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
                && !(state.live && caret_moved)
            {
                let read = || {
                    state
                        .uia
                        .as_ref()
                        .and_then(crate::text_context::read_focused)
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
            let object = (name != "uia_focus")
                .then(|| msaa_object(hwnd, id_object, id_child))
                .flatten();
            stages.push(("msaa", clock.elapsed().as_millis()));
            if clock.elapsed() > SLOW_EVENT {
                tracing::debug!(event = name, %class, ?stages, "slow event read");
            }
            let event = TraceEvent {
                t_ms,
                event: name.to_owned(),
                win_event,
                hwnd: hwnd.0 as isize,
                id_object,
                id_child,
                window_class: Some(class).filter(|s| !s.is_empty()),
                window_title: Some(title).filter(|s| !s.is_empty()),
                process: process_name(hwnd),
                object,
                focus_chain: focus,
                focus_siblings,
                focus_subtree,
                list_position,
                text_context,
                region,
                page,
                top_window_title,
                top_window_class,
                virtual_desktop,
                dialog_lines,
            };
            (state.sink)(event);
            state.count += 1;
        });
    }

    /// Records accessibility events to `out` (JSON Lines) until `stop_file` exists or
    /// `max_ms` elapses. Returns the number of events recorded.
    pub fn run_trace(
        out: &std::path::Path,
        stop_file: Option<&std::path::Path>,
        max_ms: u64,
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

        let sink = move |event: TraceEvent| {
            if let Ok(mut line) = serde_json::to_string(&event) {
                line.push('\n');
                let _ = file.write_all(line.as_bytes());
            }
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(max_ms);
        Ok(run_hooks(Box::new(sink), false, || {
            std::time::Instant::now() < deadline && !stop_file.is_some_and(|p| p.exists())
        }))
    }

    /// Delivers accessibility events to `sink` as they happen, with the same capture as
    /// [`run_trace`], until `stop` is set. This is the live screen reader's event source,
    /// so the engine consumes exactly the event stream the harness records and replays.
    pub fn run_live(
        sink: Box<dyn FnMut(TraceEvent)>,
        stop: &std::sync::atomic::AtomicBool,
    ) -> usize {
        run_hooks(sink, true, || {
            !stop.load(std::sync::atomic::Ordering::Relaxed)
        })
    }

    /// Installs the WinEvent hooks on this thread, pumps messages while `keep_running`,
    /// and returns the number of events delivered.
    fn run_hooks(
        sink: Box<dyn FnMut(TraceEvent)>,
        live: bool,
        keep_running: impl Fn() -> bool,
    ) -> usize {
        // SAFETY: COM for UIA queries on this thread, which also pumps the hook's messages.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        STATE.with(|s| {
            *s.borrow_mut() = Some(TraceState {
                sink,
                uia: automation().ok(),
                count: 0,
                live,
            })
        });

        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        // SAFETY: out-of-context hooks with a valid callback; unhooked before returning.
        let hooks = unsafe {
            [
                SetWinEventHook(0x0002, 0x0011, None, Some(on_event), 0, 0, flags),
                SetWinEventHook(0x8005, 0x8019, None, Some(on_event), 0, 0, flags),
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
        // Where focus is already, as JAWS reads it when it starts: no event will report it.
        record_current_focus();

        let mut msg = MSG::default();
        while keep_running() {
            // SAFETY: standard message pump; waits at most 50 ms between stop checks.
            unsafe {
                let _ = MsgWaitForMultipleObjects(None, false, 50, QS_ALLINPUT);
                let mut uia_focused = false;
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.hwnd.is_invalid() && msg.message == WM_UIA_FOCUS {
                        // Several focus changes queued meanwhile: the focus now is read once.
                        uia_focused = true;
                        continue;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                if uia_focused {
                    record_uia_focus();
                }
            }
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

    fn focus_event(siblings: Vec<UiaNode>) -> TraceEvent {
        TraceEvent {
            t_ms: 0,
            event: "focus".to_owned(),
            win_event: 0x8005,
            hwnd: 0,
            id_object: -4,
            id_child: 0,
            window_class: None,
            window_title: None,
            process: None,
            object: None,
            focus_chain: vec![node(GROUP, "Plan", false), node(RADIO, "Pro", true)],
            focus_siblings: siblings,
            focus_subtree: None,
            list_position: None,
            text_context: None,
            region: None,
            page: None,
            top_window_title: None,
            top_window_class: None,
            virtual_desktop: None,
            dialog_lines: Vec::new(),
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
            matches!(first.as_slice(), [sr_core::AccessibilityEvent::Focus(f)]
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
