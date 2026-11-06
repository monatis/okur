//! Raw UI Automation snapshots.
//!
//! A [`UiaSnapshot`] is a faithful, serializable copy of the UIA properties a screen reader
//! consumes, captured from a live application in a single cached cross-process request.
//! Converting a snapshot into the engine's model is a pure function, so the exact data
//! Chrome exposed on the reference machine can drive hermetic tests through production
//! mapping code, and the live screen reader can use the same conversion.
//!
//! The data types compile on every platform; only capture is Windows-specific.

use serde::{Deserialize, Serialize};

/// Bumped whenever the captured property set or its meaning changes.
pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// One UIA element with the properties the screen reader uses.
///
/// Optional fields are `None` when the provider does not support the property, which is
/// distinct from a supported property holding a default value.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiaNode {
    /// `UIA_*ControlTypeId` value.
    pub control_type: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub localized_control_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `ValuePattern.Value` (edit contents; Chrome exposes link URLs here).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_is_read_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aria_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aria_properties: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation_id: Option<String>,
    /// The element's RuntimeId, by which an action finds it again (a page read through UIA has
    /// no IA2 uniqueIDs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<Vec<i32>>,
    /// Raw `HeadingLevel` value (`HeadingLevel1` = 80051 .. `HeadingLevel9` = 80059).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading_level: Option<i32>,
    /// Raw `LandmarkType` value (`UIA_*LandmarkTypeId`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landmark_type: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub localized_landmark_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_text: Option<String>,
    /// `ItemType`: what kind of item it is (File Explorer's "File folder", "Text Document").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    /// `AcceleratorKey`: the shortcut that runs the command (a menu item's "Ctrl+X").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accelerator_key: Option<String>,
    /// `AccessKey`: the keys that reach the control ("Alt+o"), from its label's accelerator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_key: Option<String>,
    /// `ToggleState`: 0 off, 1 on, 2 indeterminate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toggle_state: Option<i32>,
    /// `ExpandCollapseState`: 0 collapsed, 1 expanded, 2 partially expanded, 3 leaf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expand_collapse_state: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_selected: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_in_set: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_of_set: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_row: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_column: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_row_span: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_column_span: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_row_count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_column_count: Option<i32>,
    /// `LiveSetting`: 0 off, 1 polite, 2 assertive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_setting: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_dialog: Option<bool>,
    /// For an unnamed list in a focus chain: the text just before it, which names it as JAWS
    /// names it (Windows Settings' "98 apps found List box", "Recent and commonly used
    /// settings List box"; dev box, 2026-10-17).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// For a control of a dialog's tree: the group box it sits in, by their rectangles (a
    /// Win32 dialog's tree is flat, its group boxes beside their controls). JAWS's Say Window
    /// names a control after its group box ("Direction Up radio button"; 540/s4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_box: Option<String>,
    /// For the focused element of a traced focus: its bounding rectangle on the screen,
    /// `[left, top, right, bottom]` in pixels, where JAWS places the PC cursor (Alt+Delete,
    /// "PC Cursor active | 1045, 544" [obs, 513/s2]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[i32; 4]>,
    pub is_enabled: bool,
    pub is_offscreen: bool,
    pub is_password: bool,
    pub is_required_for_form: bool,
    pub is_keyboard_focusable: bool,
    pub has_keyboard_focus: bool,
    pub is_control_element: bool,
    pub is_content_element: bool,
    /// A Win32 static or button its application paints itself: its window text is not what
    /// it shows, and Say Window does not read it (the Font dialog's sample, "AaBbYyZz"
    /// [obs, 665/s3]).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub owner_drawn: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<UiaNode>,
}

impl UiaNode {
    /// Total number of nodes in this subtree, including `self`.
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(UiaNode::node_count).sum::<usize>()
    }
}

/// Where and how a snapshot was captured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotProvenance {
    pub format_version: u32,
    /// `okur-windows` crate version that performed the capture.
    pub tool_version: String,
    pub host: String,
    pub os_version: String,
    pub window_title: String,
    pub window_class: String,
    /// Executable file version of the captured process (e.g. Chrome's version).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_version: Option<String>,
    /// Fixture the page was loaded from, when the capture was made for the corpus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture_sha256: Option<String>,
    pub captured_at_unix_ms: u128,
    pub node_count: usize,
    pub capture_ms: u128,
}

/// A captured document tree plus its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiaSnapshot {
    pub provenance: SnapshotProvenance,
    pub root: UiaNode,
}

impl UiaSnapshot {
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
        serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("{path:?}: {e}"))
    }
}

// UIA control type ids (UIAutomationClient.h). Kept as plain numbers so mapping compiles on
// every platform.
const CT_BUTTON: i32 = 50000;
const CT_SPLITBUTTON: i32 = 50031;
const CT_CHECKBOX: i32 = 50002;
pub const CT_COMBOBOX: i32 = 50003;
const CT_EDIT: i32 = 50004;
const CT_HYPERLINK: i32 = 50005;
const CT_IMAGE: i32 = 50006;
pub const CT_LISTITEM: i32 = 50007;
pub const CT_LIST: i32 = 50008;
const CT_PROGRESSBAR: i32 = 50012;
const CT_RADIOBUTTON: i32 = 50013;
const CT_SLIDER: i32 = 50015;
const CT_TAB: i32 = 50018;
const CT_TABITEM: i32 = 50019;
const CT_TEXT: i32 = 50020;
const CT_TREE: i32 = 50023;
pub const CT_TREEITEM: i32 = 50024;
pub const CT_GROUP: i32 = 50026;

/// A WPF label's text without its access-key mark: "_Received Notification" is "Received
/// Notification", the underscore marking R; a doubled underscore is one.
pub fn without_access_mark(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut marked = false;
    while let Some(c) = chars.next() {
        if c == '_' {
            if chars.peek() == Some(&'_') {
                out.push('_');
                chars.next();
                continue;
            }
            if !marked {
                marked = true;
                continue;
            }
        }
        out.push(c);
    }
    out
}
const CT_DATAGRID: i32 = 50028;
pub const CT_DATAITEM: i32 = 50029;
const CT_DOCUMENT: i32 = 50030;
const CT_WINDOW: i32 = 50032;
const CT_TABLE: i32 = 50036;

/// `HeadingLevel1`..`HeadingLevel9` are 80051..80059; 80050 is `HeadingLevel_None`.
fn heading_level(node: &UiaNode) -> Option<u32> {
    node.heading_level
        .filter(|l| (80051..=80059).contains(l))
        .and_then(|l| u32::try_from(l - 80050).ok())
}

pub(crate) fn aria_property<'a>(node: &'a UiaNode, key: &str) -> Option<&'a str> {
    node.aria_properties.as_deref().and_then(|props| {
        props.split(';').find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            (k.trim() == key).then_some(v.trim())
        })
    })
}

/// A focusable element whose ARIA properties say it is writable (Chromium reports
/// `readonly=true` for all other content), with no role of its own: a role that may be read-only
/// (a radio group, a list box) leaves `readonly` out unless the page sets it
/// [obs, round-aria16 snapshots].
fn is_editing_host(node: &UiaNode) -> bool {
    node.is_keyboard_focusable
        && matches!(node.aria_role.as_deref(), None | Some("" | "group"))
        && node.aria_properties.is_some()
        && aria_property(node, "readonly") != Some("true")
}

fn map_role(node: &UiaNode) -> okur_core::Role {
    use okur_core::Role;
    let aria = node.aria_role.as_deref().unwrap_or_default();
    if let Some(level) = heading_level(node) {
        return Role::Heading { level };
    }
    match aria {
        "dialog" => return Role::Dialog,
        // An ARIA switch, a toggle button to UI Automation, is a check box as IA2 has it, so
        // that its focus meets its line [obs, round-aria8, 2255/s1].
        "switch" => return Role::CheckBox,
        // Chrome exposes tab panels as Pane or Group; the ARIA role is authoritative.
        "tabpanel" => return Role::TabPanel,
        "alertdialog" => return Role::AlertDialog,
        _ => {}
    }
    if node.landmark_type.is_some() {
        return Role::Landmark {
            landmark_type: aria.to_owned(),
        };
    }
    match node.control_type {
        // An iframe: Chrome gives a document holding its page's own document [obs, 1175's
        // snapshot].
        CT_DOCUMENT if node.children.iter().any(|c| c.control_type == CT_DOCUMENT) => Role::IFrame,
        CT_DOCUMENT => Role::Document,
        CT_BUTTON => Role::Button,
        CT_SPLITBUTTON => Role::SplitButton,
        CT_CHECKBOX => Role::CheckBox,
        CT_RADIOBUTTON => Role::RadioButton,
        CT_COMBOBOX => Role::ComboBox,
        CT_EDIT => Role::Edit,
        CT_HYPERLINK => Role::Link {
            href: node.value.clone(),
        },
        CT_IMAGE => Role::Image,
        CT_LIST => Role::List,
        CT_LISTITEM => Role::ListItem,
        CT_PROGRESSBAR => Role::ProgressBar,
        CT_SLIDER => Role::Slider,
        CT_TAB => Role::TabList,
        CT_TABITEM => Role::Tab,
        CT_TEXT => Role::Text,
        CT_TREE => Role::Tree,
        CT_TREEITEM => Role::TreeItem,
        CT_TABLE | CT_DATAGRID => Role::Table,
        CT_DATAITEM if aria == "row" => Role::TableRow,
        CT_DATAITEM => Role::TableCell,
        CT_WINDOW => Role::Window,
        CT_GROUP if aria == "tabpanel" => Role::TabPanel,
        // An editing host (contenteditable without a role): Chromium reports a focusable
        // group without `readonly=true` among its ARIA properties.
        CT_GROUP if is_editing_host(node) => Role::Edit,
        CT_GROUP => Role::Group,
        _ => Role::Generic,
    }
}

/// Converts a captured UIA tree into the engine's AOM.
///
/// Pure and platform-independent: the live screen reader and the hermetic corpus tests
/// share this mapping, so tests exercise production code on data captured from Chrome.
///
/// Every text leaf is kept, including raw-view-only ones: Chrome exposes visible content
/// there (list markers, the text around links, cell text), and the virtual buffer renders
/// content from text leaves rather than from element names.
pub fn to_aom(root: &UiaNode) -> okur_core::AomNode {
    let mut next_id = 0;
    convert(root, &mut next_id)
}

/// The page in the UIA snapshot at `path` (`okur-lab snapshot`'s output), through the same
/// mapping to the AOM as a live page.
pub fn load_aom(path: impl AsRef<std::path::Path>) -> Result<okur_core::AomNode, String> {
    Ok(to_aom(&UiaSnapshot::load(path)?.root))
}

fn convert(node: &UiaNode, next_id: &mut usize) -> okur_core::AomNode {
    use okur_core::{AomNode, NodeStates, Role};
    let id = *next_id;
    *next_id += 1;
    let role = map_role(node);

    let mut aom = AomNode::new(id, role.clone());
    aom.name = node.name.clone();
    if let Some(runtime_id) = &node.runtime_id {
        let joined: Vec<String> = runtime_id.iter().map(i32::to_string).collect();
        aom.attributes.insert(
            crate::node_ids::UIA_RUNTIME_ID_ATTR.to_owned(),
            joined.join(","),
        );
    }
    match role {
        Role::Text => aom.text = node.name.clone(),
        Role::Edit | Role::ComboBox | Role::Slider => aom.value = node.value.clone(),
        // The document value is its URL; same-page links are resolved against it.
        Role::Document => {
            if let Some(url) = &node.value {
                aom.attributes.insert("url".to_owned(), url.clone());
            }
        }
        _ => {}
    }
    aom.level = node.level.and_then(|l| u32::try_from(l).ok());
    aom.value_now = node.range_value;
    aom.value_min = node.range_minimum;
    aom.value_max = node.range_maximum;
    aom.states = NodeStates {
        // Radio buttons report "checked" through SelectionItem, not Toggle.
        checked: match (&role, node.toggle_state) {
            (Role::RadioButton, _) => node.is_selected,
            (_, Some(0)) => Some(false),
            (_, Some(1)) => Some(true),
            _ => None,
        },
        // `ToggleState_Indeterminate`: a tri-state check box between checked and not.
        mixed: role != Role::RadioButton && node.toggle_state == Some(2),
        selected: node.is_selected,
        expanded: match node.expand_collapse_state {
            Some(0) => Some(false),
            Some(1) | Some(2) => Some(true),
            _ => None,
        },
        focused: node.has_keyboard_focus,
        disabled: !node.is_enabled,
        hidden: false,
        modal: aria_property(node, "modal") == Some("true"),
        focusable: Some(node.is_keyboard_focusable),
        pressed: false,
    };
    // Chrome gives a page's plain table UI Automation's Table type with the ARIA role "grid"
    // (its cells "gridcell"), and an ARIA grid the DataGrid type: the role is a grid's on a
    // DataGrid alone. JAWS says "table with 4 columns and 4 rows" for the first [obs, 21/s1],
    // "grid with 5 columns and 7 rows" for the second [obs, 2694/s1].
    let aria_role = node
        .aria_role
        .as_deref()
        .filter(|aria| !(*aria == "grid" && node.control_type == CT_TABLE));
    if let Some(aria) = aria_role {
        aom.attributes
            .insert("aria_role".to_owned(), aria.to_owned());
        // The AOM's table model identifies header cells by `tag`/`scope`.
        match aria {
            "columnheader" => {
                aom.attributes.insert("tag".to_owned(), "th".to_owned());
                aom.attributes.insert("scope".to_owned(), "col".to_owned());
            }
            "rowheader" => {
                aom.attributes.insert("tag".to_owned(), "th".to_owned());
                aom.attributes.insert("scope".to_owned(), "row".to_owned());
            }
            // An ARIA spin button, as IA2's mapping marks it.
            "spinbutton" if aom.role == okur_core::Role::Edit => {
                aom.attributes
                    .insert("spinbutton".to_owned(), "true".to_owned());
            }
            _ => {}
        }
    }
    let mut set_attr = |key: &str, value: String| {
        aom.attributes.insert(key.to_owned(), value);
    };
    if node.is_required_for_form {
        set_attr("required", "true".to_owned());
    }
    if node.is_password {
        set_attr("password", "true".to_owned());
    }
    // A read-only field (GitHub's code view is a read-only text area).
    if aom.role == okur_core::Role::Edit && node.value_is_read_only == Some(true) {
        set_attr("readonly", "true".to_owned());
    }
    // A combo box one types in (an autocomplete), JAWS's "edit combo": its value is writable,
    // and its popup is not a menu (Chrome's UIA gives a select a writable value too, with
    // "haspopup=menu").
    if aom.role == okur_core::Role::ComboBox
        && node.value_is_read_only == Some(false)
        && aria_property(node, "haspopup").is_some_and(|p| p != "menu")
    {
        set_attr("editable", "true".to_owned());
    }
    if aria_property(node, "multiline") == Some("true") {
        set_attr("multiline", "true".to_owned());
    }
    // An editing host (contenteditable), which JAWS reads in the page as a section: "Notes field |
    // Editable Section Start | Editable text | Editable Section End" [obs, 1172/s23-s26].
    if node.control_type == CT_GROUP && is_editing_host(node) {
        set_attr("editing_host", "true".to_owned());
    }
    // aria-describedby text, spoken after the value in focus phrasing.
    if let Some(d) = node
        .full_description
        .as_deref()
        .or(node.help_text.as_deref())
    {
        set_attr("description", d.to_owned());
    }
    // Chrome reports the DOM id as AutomationId; same-page link targets resolve against it.
    if let Some(id) = &node.automation_id {
        set_attr("id", id.clone());
    }
    // aria-current: "page", "step", "location", "date", "time" or "true".
    if let Some(current) = aria_property(node, "current").filter(|c| *c != "false") {
        set_attr("current", current.to_owned());
    }
    if let (Some(pos), Some(size)) = (node.position_in_set, node.size_of_set) {
        set_attr("posinset", pos.to_string());
        set_attr("setsize", size.to_string());
    }

    aom.children = node.children.iter().map(|c| convert(c, next_id)).collect();
    // Chromium gives an editing host (contenteditable) no UIA value; its value is its text.
    if role == Role::Edit && aom.value.is_none() {
        let text: String = aom
            .children
            .iter()
            .filter_map(|c| c.text.as_deref())
            .collect();
        aom.value = Some(text).filter(|t| !t.is_empty());
    }
    aom
}

#[cfg(test)]
mod tests {
    use super::without_access_mark;

    /// WPF's access-key mark goes, a doubled underscore stays one [obs, 775/s6].
    #[test]
    fn a_wpf_label_loses_its_access_key_mark() {
        assert_eq!(
            without_access_mark("_Received Notification"),
            "Received Notification"
        );
        assert_eq!(without_access_mark("Speech or _Sound"), "Speech or Sound");
        assert_eq!(without_access_mark("snake__case"), "snake_case");
        assert_eq!(without_access_mark("No mark"), "No mark");
    }

    /// A focusable group without `readonly=true` is an editing host only without a role of its
    /// own: a radio group taking the focus itself (`aria-activedescendant`) is a group, its radio
    /// buttons in the buffer [obs, round-aria16 snapshots].
    #[test]
    fn a_focusable_radio_group_is_no_editing_host() {
        let group = |aria: &str| super::UiaNode {
            control_type: super::CT_GROUP,
            aria_role: Some(aria.to_owned()),
            aria_properties: Some("expanded=false;multiline=false;required=false".to_owned()),
            is_keyboard_focusable: true,
            ..Default::default()
        };
        assert_eq!(super::map_role(&group("group")), okur_core::Role::Edit);
        assert_eq!(
            super::map_role(&group("radiogroup")),
            okur_core::Role::Group
        );
    }

    /// Chrome gives an iframe as a document holding its page's own document: a frame, where the
    /// page's documents stay documents [obs, 1175's snapshot].
    #[test]
    fn a_document_holding_a_document_is_a_frame() {
        let document = |children: Vec<super::UiaNode>| super::UiaNode {
            control_type: super::CT_DOCUMENT,
            children,
            ..Default::default()
        };
        assert_eq!(
            super::map_role(&document(vec![document(Vec::new())])),
            okur_core::Role::IFrame
        );
        assert_eq!(
            super::map_role(&document(Vec::new())),
            okur_core::Role::Document
        );
    }

    /// Chrome's plain table is UI Automation's Table with the ARIA role "grid": a table, which
    /// JAWS calls a table [obs, 21/s1]; an ARIA grid is a DataGrid, a grid [obs, 2694/s1].
    #[test]
    fn a_plain_table_is_no_grid() {
        let table = |control_type: i32| super::UiaNode {
            control_type,
            aria_role: Some("grid".to_owned()),
            ..Default::default()
        };
        let role = |control_type: i32| {
            super::to_aom(&table(control_type))
                .attributes
                .get("aria_role")
                .cloned()
        };
        assert_eq!(role(super::CT_TABLE), None);
        assert_eq!(role(super::CT_DATAGRID).as_deref(), Some("grid"));
    }
}
