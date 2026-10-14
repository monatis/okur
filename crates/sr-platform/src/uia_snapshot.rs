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
    pub is_enabled: bool,
    pub is_offscreen: bool,
    pub is_password: bool,
    pub is_required_for_form: bool,
    pub is_keyboard_focusable: bool,
    pub has_keyboard_focus: bool,
    pub is_control_element: bool,
    pub is_content_element: bool,
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
    /// `sr-platform` crate version that performed the capture.
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
const CT_CHECKBOX: i32 = 50002;
const CT_COMBOBOX: i32 = 50003;
const CT_EDIT: i32 = 50004;
const CT_HYPERLINK: i32 = 50005;
const CT_IMAGE: i32 = 50006;
const CT_LISTITEM: i32 = 50007;
const CT_LIST: i32 = 50008;
const CT_PROGRESSBAR: i32 = 50012;
const CT_RADIOBUTTON: i32 = 50013;
const CT_SLIDER: i32 = 50015;
const CT_TAB: i32 = 50018;
const CT_TABITEM: i32 = 50019;
const CT_TEXT: i32 = 50020;
const CT_TREE: i32 = 50023;
const CT_TREEITEM: i32 = 50024;
const CT_GROUP: i32 = 50026;
const CT_DATAGRID: i32 = 50028;
const CT_DATAITEM: i32 = 50029;
const CT_DOCUMENT: i32 = 50030;
const CT_WINDOW: i32 = 50032;
const CT_TABLE: i32 = 50036;

/// `HeadingLevel1`..`HeadingLevel9` are 80051..80059; 80050 is `HeadingLevel_None`.
fn heading_level(node: &UiaNode) -> Option<u32> {
    node.heading_level
        .filter(|l| (80051..=80059).contains(l))
        .and_then(|l| u32::try_from(l - 80050).ok())
}

fn aria_property<'a>(node: &'a UiaNode, key: &str) -> Option<&'a str> {
    node.aria_properties.as_deref().and_then(|props| {
        props.split(';').find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            (k.trim() == key).then_some(v.trim())
        })
    })
}

/// A focusable element whose ARIA properties say it is writable (Chromium reports
/// `readonly=true` for all other content).
fn is_editing_host(node: &UiaNode) -> bool {
    node.is_keyboard_focusable
        && node.aria_properties.is_some()
        && aria_property(node, "readonly") != Some("true")
}

fn map_role(node: &UiaNode) -> sr_core::Role {
    use sr_core::Role;
    let aria = node.aria_role.as_deref().unwrap_or_default();
    if let Some(level) = heading_level(node) {
        return Role::Heading { level };
    }
    match aria {
        "dialog" => return Role::Dialog,
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
        CT_DOCUMENT => Role::Document,
        CT_BUTTON => Role::Button,
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
pub fn to_aom(root: &UiaNode) -> sr_core::AomNode {
    let mut next_id = 0;
    convert(root, &mut next_id)
}

fn convert(node: &UiaNode, next_id: &mut usize) -> sr_core::AomNode {
    use sr_core::{AomNode, NodeStates, Role};
    let id = *next_id;
    *next_id += 1;
    let role = map_role(node);

    let mut aom = AomNode::new(id, role.clone());
    aom.name = node.name.clone();
    match role {
        Role::Text => aom.text = node.name.clone(),
        Role::Edit | Role::ComboBox => aom.value = node.value.clone(),
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
    };
    if let Some(aria) = node.aria_role.as_deref() {
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
    if aria_property(node, "multiline") == Some("true") {
        set_attr("multiline", "true".to_owned());
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

#[cfg(windows)]
pub use capture::capture_foreground_document;

/// The element UIA reports as having keyboard focus, as `"name" (control type N)`, for
/// harness checks.
#[cfg(windows)]
pub fn focused_element_summary() -> String {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    // SAFETY: COM initialization; repeated initialization is tolerated.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let Ok(uia) = capture::automation() else {
        return "unknown (UIA unavailable)".to_owned();
    };
    capture::focus_chain(&uia)
        .last()
        .map(|n| format!("{:?} (control type {})", n.name, n.control_type))
        .unwrap_or_else(|| "none".to_owned())
}

#[cfg(windows)]
pub(crate) mod capture {
    use super::{SnapshotProvenance, UiaNode, UiaSnapshot, SNAPSHOT_FORMAT_VERSION};
    use crate::PlatformError;
    use windows::core::BSTR;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowTextW,
    };

    /// Properties fetched for every node in one `BuildUpdatedCache` round trip.
    const PROPERTIES: &[UIA_PROPERTY_ID] = &[
        UIA_ControlTypePropertyId,
        UIA_LocalizedControlTypePropertyId,
        UIA_NamePropertyId,
        UIA_ValueValuePropertyId,
        UIA_ValueIsReadOnlyPropertyId,
        UIA_AriaRolePropertyId,
        UIA_AriaPropertiesPropertyId,
        UIA_ClassNamePropertyId,
        UIA_AutomationIdPropertyId,
        UIA_HeadingLevelPropertyId,
        UIA_LandmarkTypePropertyId,
        UIA_LocalizedLandmarkTypePropertyId,
        UIA_FullDescriptionPropertyId,
        UIA_HelpTextPropertyId,
        UIA_ToggleToggleStatePropertyId,
        UIA_ExpandCollapseExpandCollapseStatePropertyId,
        UIA_SelectionItemIsSelectedPropertyId,
        UIA_RangeValueValuePropertyId,
        UIA_RangeValueMinimumPropertyId,
        UIA_RangeValueMaximumPropertyId,
        UIA_PositionInSetPropertyId,
        UIA_SizeOfSetPropertyId,
        UIA_LevelPropertyId,
        UIA_GridItemRowPropertyId,
        UIA_GridItemColumnPropertyId,
        UIA_GridItemRowSpanPropertyId,
        UIA_GridItemColumnSpanPropertyId,
        UIA_GridRowCountPropertyId,
        UIA_GridColumnCountPropertyId,
        UIA_LiveSettingPropertyId,
        UIA_IsDialogPropertyId,
        UIA_IsEnabledPropertyId,
        UIA_IsOffscreenPropertyId,
        UIA_IsPasswordPropertyId,
        UIA_IsRequiredForFormPropertyId,
        UIA_IsKeyboardFocusablePropertyId,
        UIA_HasKeyboardFocusPropertyId,
        UIA_IsControlElementPropertyId,
        UIA_IsContentElementPropertyId,
    ];

    fn err(context: &str, e: windows::core::Error) -> PlatformError {
        PlatformError::EventListenerError(format!("{context}: {e}"))
    }

    /// Reads a cached property, returning `None` when the provider does not support it.
    fn cached(el: &IUIAutomationElement, id: UIA_PROPERTY_ID) -> Option<VARIANT> {
        // SAFETY: `el` is a valid cached element; the property id is in the cache request.
        unsafe { el.GetCachedPropertyValueEx(id, true) }.ok()
    }

    fn string(el: &IUIAutomationElement, id: UIA_PROPERTY_ID) -> Option<String> {
        cached(el, id)
            .and_then(|v| BSTR::try_from(&v).ok())
            .map(|b| b.to_string())
            .filter(|s| !s.is_empty())
    }

    fn int(el: &IUIAutomationElement, id: UIA_PROPERTY_ID) -> Option<i32> {
        cached(el, id).and_then(|v| i32::try_from(&v).ok())
    }

    fn float(el: &IUIAutomationElement, id: UIA_PROPERTY_ID) -> Option<f64> {
        cached(el, id).and_then(|v| f64::try_from(&v).ok())
    }

    fn boolean(el: &IUIAutomationElement, id: UIA_PROPERTY_ID) -> Option<bool> {
        cached(el, id).and_then(|v| bool::try_from(&v).ok())
    }

    fn read_node(el: &IUIAutomationElement, depth: usize) -> UiaNode {
        let mut node = UiaNode {
            control_type: int(el, UIA_ControlTypePropertyId).unwrap_or_default(),
            localized_control_type: string(el, UIA_LocalizedControlTypePropertyId),
            name: string(el, UIA_NamePropertyId),
            value: string(el, UIA_ValueValuePropertyId),
            value_is_read_only: boolean(el, UIA_ValueIsReadOnlyPropertyId),
            aria_role: string(el, UIA_AriaRolePropertyId),
            aria_properties: string(el, UIA_AriaPropertiesPropertyId),
            class_name: string(el, UIA_ClassNamePropertyId),
            automation_id: string(el, UIA_AutomationIdPropertyId),
            heading_level: int(el, UIA_HeadingLevelPropertyId),
            landmark_type: int(el, UIA_LandmarkTypePropertyId).filter(|&v| v != 0),
            localized_landmark_type: string(el, UIA_LocalizedLandmarkTypePropertyId),
            full_description: string(el, UIA_FullDescriptionPropertyId),
            help_text: string(el, UIA_HelpTextPropertyId),
            toggle_state: int(el, UIA_ToggleToggleStatePropertyId),
            expand_collapse_state: int(el, UIA_ExpandCollapseExpandCollapseStatePropertyId),
            is_selected: boolean(el, UIA_SelectionItemIsSelectedPropertyId),
            range_value: float(el, UIA_RangeValueValuePropertyId),
            range_minimum: float(el, UIA_RangeValueMinimumPropertyId),
            range_maximum: float(el, UIA_RangeValueMaximumPropertyId),
            position_in_set: int(el, UIA_PositionInSetPropertyId).filter(|&v| v > 0),
            size_of_set: int(el, UIA_SizeOfSetPropertyId).filter(|&v| v > 0),
            level: int(el, UIA_LevelPropertyId).filter(|&v| v > 0),
            grid_row: int(el, UIA_GridItemRowPropertyId),
            grid_column: int(el, UIA_GridItemColumnPropertyId),
            grid_row_span: int(el, UIA_GridItemRowSpanPropertyId),
            grid_column_span: int(el, UIA_GridItemColumnSpanPropertyId),
            grid_row_count: int(el, UIA_GridRowCountPropertyId),
            grid_column_count: int(el, UIA_GridColumnCountPropertyId),
            live_setting: int(el, UIA_LiveSettingPropertyId).filter(|&v| v != 0),
            is_dialog: boolean(el, UIA_IsDialogPropertyId).filter(|&v| v),
            is_enabled: boolean(el, UIA_IsEnabledPropertyId).unwrap_or(true),
            is_offscreen: boolean(el, UIA_IsOffscreenPropertyId).unwrap_or(false),
            is_password: boolean(el, UIA_IsPasswordPropertyId).unwrap_or(false),
            is_required_for_form: boolean(el, UIA_IsRequiredForFormPropertyId).unwrap_or(false),
            is_keyboard_focusable: boolean(el, UIA_IsKeyboardFocusablePropertyId).unwrap_or(false),
            has_keyboard_focus: boolean(el, UIA_HasKeyboardFocusPropertyId).unwrap_or(false),
            is_control_element: boolean(el, UIA_IsControlElementPropertyId).unwrap_or(false),
            is_content_element: boolean(el, UIA_IsContentElementPropertyId).unwrap_or(false),
            children: Vec::new(),
        };

        // Bound recursion on pathological trees; Chrome documents are far shallower.
        if depth < 256 {
            // SAFETY: the cache request used TreeScope_Subtree, so cached children exist.
            if let Ok(children) = unsafe { el.GetCachedChildren() } {
                let len = unsafe { children.Length() }.unwrap_or(0);
                for i in 0..len {
                    if let Ok(child) = unsafe { children.GetElement(i) } {
                        node.children.push(read_node(&child, depth + 1));
                    }
                }
            }
        }
        node
    }

    pub(crate) fn window_text(hwnd: windows::Win32::Foundation::HWND) -> (String, String) {
        let mut title = [0u16; 512];
        let mut class = [0u16; 256];
        // SAFETY: buffers are valid for their stated lengths.
        let tlen = unsafe { GetWindowTextW(hwnd, &mut title) }.max(0) as usize;
        let clen = unsafe { GetClassNameW(hwnd, &mut class) }.max(0) as usize;
        (
            String::from_utf16_lossy(title.get(..tlen).unwrap_or_default()),
            String::from_utf16_lossy(class.get(..clen).unwrap_or_default()),
        )
    }

    /// Creates the UIA client object (the caller must have initialized COM). A busy or hung
    /// application fails its calls after a second instead of UIA's long defaults, so one
    /// application cannot stall the reader's event thread.
    pub(crate) fn automation() -> Result<IUIAutomation, PlatformError> {
        use windows::core::Interface;
        use windows::Win32::UI::Accessibility::IUIAutomation2;
        // SAFETY: standard in-process COM activation.
        let uia: IUIAutomation =
            unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) }
                .map_err(|e| err("create CUIAutomation8", e))?;
        if let Ok(uia2) = uia.cast::<IUIAutomation2>() {
            // SAFETY: plain setters on a valid client.
            unsafe {
                let _ = uia2.SetConnectionTimeout(1000);
                let _ = uia2.SetTransactionTimeout(1000);
            }
        }
        Ok(uia)
    }

    /// Reads one element (no children) with the full snapshot property set.
    pub(crate) fn read_element(
        uia: &IUIAutomation,
        element: &IUIAutomationElement,
    ) -> Option<UiaNode> {
        // SAFETY: a cache request for a single element of a valid UIA client.
        unsafe {
            let request = uia.CreateCacheRequest().ok()?;
            for &prop in PROPERTIES {
                request.AddProperty(prop).ok()?;
            }
            request.SetTreeScope(TreeScope_Element).ok()?;
            request
                .SetAutomationElementMode(AutomationElementMode_None)
                .ok()?;
            let cached = element.BuildUpdatedCache(&request).ok()?;
            Some(read_node(&cached, usize::MAX))
        }
    }

    /// The focused element and its control-view ancestors, outermost first (the desktop
    /// root is omitted). This is the context a screen reader announces on focus changes.
    pub(crate) fn focus_chain(uia: &IUIAutomation) -> Vec<UiaNode> {
        // SAFETY: UIA client query on a valid automation object.
        unsafe { uia.GetFocusedElement() }
            .map(|focused| focus_chain_from(uia, &focused))
            .unwrap_or_default()
    }

    /// `element` and its control-view ancestors, outermost first, as [`focus_chain`] reads
    /// the focused element: for a focus that UIA's event names while `GetFocusedElement`
    /// still answers with its host (the Windows 11 task switcher's items).
    pub(crate) fn focus_chain_from(
        uia: &IUIAutomation,
        element: &IUIAutomationElement,
    ) -> Vec<UiaNode> {
        let mut chain = Vec::new();
        // SAFETY: UIA client queries on a valid automation object.
        unsafe {
            let Ok(walker) = uia.ControlViewWalker() else {
                return chain;
            };
            let Ok(root) = uia.GetRootElement() else {
                return chain;
            };
            let mut current = Some(element.clone());
            // Bound the walk: real focus chains are a few dozen levels deep at most.
            for _ in 0..64 {
                let Some(el) = current else { break };
                if uia
                    .CompareElements(&el, &root)
                    .map(|b| b.as_bool())
                    .unwrap_or(false)
                {
                    break;
                }
                if let Some(node) = read_element(uia, &el) {
                    chain.push(node);
                }
                current = walker.GetParentElement(&el).ok();
            }
        }
        chain.reverse();
        chain
    }

    /// Siblings recorded around a focused element, at most.
    const MAX_FOCUS_SIBLINGS: usize = 200;
    /// Nodes recorded below a focused element, at most.
    const MAX_FOCUS_SUBTREE: usize = 200;

    /// What a screen reader reads around the focused element besides its ancestors: the
    /// control-view siblings (without children), from which positions such as "2 of 3" are
    /// counted when the provider reports no PositionInSet, and the element's own control-view
    /// subtree to depth 2 (list-view columns, a combo box's items).
    pub(crate) fn focus_context(uia: &IUIAutomation) -> (Vec<UiaNode>, Option<UiaNode>) {
        // SAFETY: UIA client query on a valid automation object.
        unsafe { uia.GetFocusedElement() }
            .map(|focused| focus_context_of(uia, &focused))
            .unwrap_or_default()
    }

    /// [`focus_context`] around `focused`.
    pub(crate) fn focus_context_of(
        uia: &IUIAutomation,
        focused: &IUIAutomationElement,
    ) -> (Vec<UiaNode>, Option<UiaNode>) {
        // SAFETY: UIA client queries on a valid automation object.
        unsafe {
            let Ok(walker) = uia.ControlViewWalker() else {
                return (Vec::new(), None);
            };
            let mut siblings = Vec::new();
            if let Ok(parent) = walker.GetParentElement(focused) {
                let mut current = walker.GetFirstChildElement(&parent).ok();
                while let Some(el) = current {
                    if siblings.len() >= MAX_FOCUS_SIBLINGS {
                        break;
                    }
                    if let Some(node) = read_element(uia, &el) {
                        siblings.push(node);
                    }
                    current = walker.GetNextSiblingElement(&el).ok();
                }
            }
            let mut budget = MAX_FOCUS_SUBTREE;
            let subtree = read_subtree(uia, &walker, focused, 2, &mut budget);
            (siblings, subtree)
        }
    }

    /// The focused element with its control-view subtree to depth 2.
    pub(crate) fn focus_subtree_of(uia: &IUIAutomation) -> Option<UiaNode> {
        // SAFETY: UIA client queries on a valid automation object.
        unsafe {
            let walker = uia.ControlViewWalker().ok()?;
            let focused = uia.GetFocusedElement().ok()?;
            let mut budget = MAX_FOCUS_SUBTREE;
            read_subtree(uia, &walker, &focused, 2, &mut budget)
        }
    }

    fn read_subtree(
        uia: &IUIAutomation,
        walker: &IUIAutomationTreeWalker,
        element: &IUIAutomationElement,
        depth: usize,
        budget: &mut usize,
    ) -> Option<UiaNode> {
        *budget = budget.checked_sub(1)?;
        let mut node = read_element(uia, element)?;
        if depth == 0 {
            return Some(node);
        }
        // SAFETY: UIA tree walking on valid elements.
        let mut current = unsafe { walker.GetFirstChildElement(element) }.ok();
        while let Some(el) = current {
            match read_subtree(uia, walker, &el, depth - 1, budget) {
                Some(child) => node.children.push(child),
                None => break,
            }
            // SAFETY: as above.
            current = unsafe { walker.GetNextSiblingElement(&el) }.ok();
        }
        Some(node)
    }

    /// Captures the first `Document` element in the foreground window (e.g. a Chrome tab's
    /// web content) with its entire raw-view subtree, in a single cached request.
    pub fn capture_foreground_document(
        fixture_sha256: Option<String>,
        process_version: Option<String>,
    ) -> Result<UiaSnapshot, PlatformError> {
        let started = std::time::Instant::now();
        // SAFETY: standard COM initialization; S_FALSE/RPC_E_CHANGED_MODE are tolerated
        // because UIA client calls work from either apartment for this one-shot capture.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let uia: IUIAutomation =
            unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) }
                .map_err(|e| err("create CUIAutomation8", e))?;

        // SAFETY: plain Win32 call with no preconditions.
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_invalid() {
            return Err(PlatformError::NoWindowFound);
        }
        let (window_title, window_class) = window_text(hwnd);

        let window = unsafe { uia.ElementFromHandle(hwnd) }
            .map_err(|e| err("ElementFromHandle(foreground)", e))?;
        let is_document = unsafe {
            uia.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_DocumentControlTypeId.0),
            )
        }
        .map_err(|e| err("CreatePropertyCondition", e))?;
        let document = unsafe { window.FindFirst(TreeScope_Descendants, &is_document) }
            .map_err(|_| PlatformError::NoDocumentFound)?;

        let request = unsafe { uia.CreateCacheRequest() }.map_err(|e| err("cache", e))?;
        for &prop in PROPERTIES {
            unsafe { request.AddProperty(prop) }.map_err(|e| err("AddProperty", e))?;
        }
        let raw_view = unsafe { uia.RawViewCondition() }.map_err(|e| err("RawView", e))?;
        unsafe {
            request
                .SetTreeFilter(&raw_view)
                .map_err(|e| err("SetTreeFilter", e))?;
            request
                .SetTreeScope(TreeScope_Subtree)
                .map_err(|e| err("SetTreeScope", e))?;
            request
                .SetAutomationElementMode(AutomationElementMode_None)
                .map_err(|e| err("SetAutomationElementMode", e))?;
        }
        let cached_root = unsafe { document.BuildUpdatedCache(&request) }
            .map_err(|e| err("BuildUpdatedCache", e))?;

        let root = read_node(&cached_root, 0);
        Ok(UiaSnapshot {
            provenance: SnapshotProvenance {
                format_version: SNAPSHOT_FORMAT_VERSION,
                tool_version: env!("CARGO_PKG_VERSION").to_owned(),
                host: std::env::var("COMPUTERNAME").unwrap_or_default(),
                os_version: os_version(),
                window_title,
                window_class,
                process_version,
                fixture_sha256,
                captured_at_unix_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0),
                node_count: root.node_count(),
                capture_ms: started.elapsed().as_millis(),
            },
            root,
        })
    }

    pub(crate) fn os_version() -> String {
        // `ver` output, e.g. "Microsoft Windows [Version 10.0.19045.5737]".
        std::process::Command::new("cmd")
            .args(["/C", "ver"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_default()
    }
}
