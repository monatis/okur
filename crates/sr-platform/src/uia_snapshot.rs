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

fn map_role(node: &UiaNode) -> sr_core::Role {
    use sr_core::Role;
    let aria = node.aria_role.as_deref().unwrap_or_default();
    if let Some(level) = heading_level(node) {
        return Role::Heading { level };
    }
    match aria {
        "dialog" => return Role::Dialog,
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
        CT_GROUP => Role::Group,
        _ => Role::Generic,
    }
}

/// Raw-view-only text leaves duplicate their parent's name (e.g. a heading's inner text);
/// the control view excludes them, and so do we.
fn is_redundant_text(node: &UiaNode) -> bool {
    node.control_type == CT_TEXT && !node.is_control_element && node.children.is_empty()
}

/// Converts a captured UIA tree into the engine's AOM.
///
/// Pure and platform-independent: the live screen reader and the hermetic corpus tests
/// share this mapping, so tests exercise production code on data captured from Chrome.
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
        _ => {}
    }
    aom.level = node.level.and_then(|l| u32::try_from(l).ok());
    aom.value_now = node.range_value;
    aom.value_min = node.range_minimum;
    aom.value_max = node.range_maximum;
    aom.states = NodeStates {
        checked: match node.toggle_state {
            Some(0) => Some(false),
            Some(1) => Some(true),
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
    if node.is_required_for_form {
        aom.attributes
            .insert("required".to_owned(), "true".to_owned());
    }

    aom.children = node
        .children
        .iter()
        .filter(|c| !is_redundant_text(c))
        .map(|c| convert(c, next_id))
        .collect();
    aom
}

#[cfg(windows)]
pub use capture::capture_foreground_document;

#[cfg(windows)]
mod capture {
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

    fn window_text(hwnd: windows::Win32::Foundation::HWND) -> (String, String) {
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

    fn os_version() -> String {
        // `ver` output, e.g. "Microsoft Windows [Version 10.0.19045.5737]".
        std::process::Command::new("cmd")
            .args(["/C", "ver"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_default()
    }
}
