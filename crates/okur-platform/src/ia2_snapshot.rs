//! IAccessible2 snapshots of browser documents.
//!
//! Captured alongside UIA snapshots (`okur snapshot --api ia2`) from the same fixture load,
//! with the same provenance block. IA2 adds what UIA lacks for web content: HTML tags and CSS
//! display (`tag`, `display` attributes), visited state, hypertext with embedded objects, and
//! the browser's rendered line ranges per text object.

use crate::uia_snapshot::SnapshotProvenance;
use serde::{Deserialize, Serialize};

pub use xa11y_windows::ia2::Ia2Node;

/// A captured IA2 document tree plus its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ia2Snapshot {
    pub provenance: SnapshotProvenance,
    pub root: Ia2Node,
}

impl Ia2Snapshot {
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
        serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("{path:?}: {e}"))
    }
}

/// Captures the document of the foreground Chromium window through MSAA + IA2.
#[cfg(windows)]
pub fn capture_foreground_document(
    fixture_sha256: Option<String>,
    process_version: Option<String>,
) -> Result<Ia2Snapshot, crate::PlatformError> {
    use crate::uia_snapshot::capture::{os_version, window_text};
    use crate::uia_snapshot::SNAPSHOT_FORMAT_VERSION;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let started = std::time::Instant::now();
    // SAFETY: standard COM initialization; repeated initialization is tolerated.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    // SAFETY: plain Win32 call with no preconditions.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return Err(crate::PlatformError::NoWindowFound);
    }
    let (window_title, window_class) = window_text(hwnd);
    let root = xa11y_windows::ia2::capture_chromium_document(hwnd)
        .map_err(|e| crate::PlatformError::EventListenerError(e.to_string()))?;
    Ok(Ia2Snapshot {
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

/// MSAA roles (`ROLE_SYSTEM_*` in `oleacc.h`) that Chromium reports for web content.
mod msaa_role {
    pub const DOCUMENT: i32 = 0x0F;
    pub const DIALOG: i32 = 0x12;
    pub const GROUPING: i32 = 0x14;
    pub const TABLE: i32 = 0x18;
    pub const COLUMNHEADER: i32 = 0x19;
    pub const ROWHEADER: i32 = 0x1A;
    pub const ROW: i32 = 0x1C;
    pub const CELL: i32 = 0x1D;
    pub const LINK: i32 = 0x1E;
    pub const LIST: i32 = 0x21;
    pub const LISTITEM: i32 = 0x22;
    pub const OUTLINE: i32 = 0x23;
    pub const OUTLINEITEM: i32 = 0x24;
    pub const PAGETAB: i32 = 0x25;
    pub const PROPERTYPAGE: i32 = 0x26;
    pub const GRAPHIC: i32 = 0x28;
    pub const STATICTEXT: i32 = 0x29;
    pub const TEXT: i32 = 0x2A;
    pub const PUSHBUTTON: i32 = 0x2B;
    pub const CHECKBUTTON: i32 = 0x2C;
    pub const RADIOBUTTON: i32 = 0x2D;
    pub const COMBOBOX: i32 = 0x2E;
    pub const PROGRESSBAR: i32 = 0x30;
    pub const SLIDER: i32 = 0x33;
    pub const EQUATION: i32 = 0x37;
    pub const PAGETABLIST: i32 = 0x3C;
}

/// IA2 state bits (`AccessibleStates.idl`).
mod ia2_state {
    pub const EDITABLE: i32 = 0x8;
    pub const MODAL: i32 = 0x100;
    pub const MULTI_LINE: i32 = 0x200;
    pub const REQUIRED: i32 = 0x800;
}

/// ARIA landmark roles, as Chromium reports them in `xml-roles`.
const LANDMARKS: [&str; 8] = [
    "banner",
    "complementary",
    "contentinfo",
    "form",
    "main",
    "navigation",
    "region",
    "search",
];

/// Whether a CSS `display` value makes a block-level box (anything but inline content).
fn is_block_display(display: Option<&str>) -> bool {
    display.is_some_and(|d| !d.is_empty() && !d.starts_with("inline") && d != "contents")
}

/// A font size from IA2 text attributes in points: IA2 specifies points (`12pt`), and CSS
/// pixels (`16px`) are three quarters of a point.
fn font_size_points(raw: &str) -> Option<f32> {
    let raw = raw.trim();
    if let Some(px) = raw.strip_suffix("px") {
        return px.trim().parse::<f32>().ok().map(|px| px * 0.75);
    }
    raw.strip_suffix("pt").unwrap_or(raw).trim().parse().ok()
}

/// Say Font's formatting from an IA2 text run (`font-family`, `font-size`, `font-weight`,
/// `font-style`, `text-underline-style`).
fn text_formatting(
    attributes: &std::collections::BTreeMap<String, String>,
) -> Option<okur_core::aom::TextFormatting> {
    let get = |k: &str| attributes.get(k).map(|v| v.trim().trim_matches('"'));
    let font_family = get("font-family")
        .filter(|f| !f.is_empty())
        .map(str::to_owned);
    let font_size_pt = get("font-size").and_then(font_size_points);
    if font_family.is_none() && font_size_pt.is_none() {
        return None;
    }
    let bold =
        get("font-weight").is_some_and(|w| w == "bold" || w.parse::<u32>().is_ok_and(|n| n >= 600));
    Some(okur_core::aom::TextFormatting {
        font_family,
        font_size_pt,
        bold,
        italic: get("font-style").is_some_and(|s| s == "italic" || s == "oblique"),
        underline: get("text-underline-style").is_some_and(|s| !s.is_empty() && s != "none"),
        alignment: None,
    })
}

fn map_role(node: &Ia2Node) -> okur_core::Role {
    use okur_core::Role;
    use xa11y_windows::ia2::msaa_states::STATE_SYSTEM_FOCUSABLE as FOCUSABLE;
    use xa11y_windows::ia2::roles::{IA2_ROLE_BLOCK_QUOTE, IA2_ROLE_HEADING};

    let attr = |k: &str| node.attributes.get(k).map(String::as_str);
    let xml_role = attr("xml-roles")
        .and_then(|r| r.split_whitespace().next())
        .unwrap_or_default();
    let named = node.name.as_deref().is_some_and(|n| !n.trim().is_empty());

    if node.ia2_role == Some(IA2_ROLE_HEADING) {
        let level = attr("level")
            .and_then(|l| l.parse().ok())
            .or_else(|| {
                node.group_position
                    .and_then(|[l, _, _]| u32::try_from(l).ok())
            })
            .unwrap_or(2);
        return Role::Heading { level };
    }
    if node.ia2_role == Some(IA2_ROLE_BLOCK_QUOTE) {
        return Role::BlockQuote;
    }
    match xml_role {
        "dialog" => return Role::Dialog,
        "alertdialog" => return Role::AlertDialog,
        "tabpanel" => return Role::TabPanel,
        // Forms and generic regions are landmarks only when named.
        "form" | "region" if !named => {}
        r if LANDMARKS.contains(&r) => {
            return Role::Landmark {
                landmark_type: r.to_owned(),
            }
        }
        _ => {}
    }
    let editable = node.ia2_states & ia2_state::EDITABLE != 0;
    match node.msaa_role.unwrap_or_default() {
        msaa_role::DOCUMENT if attr("tag") == Some("article") => Role::Group,
        msaa_role::DOCUMENT => Role::Document,
        msaa_role::DIALOG => Role::Dialog,
        msaa_role::LINK => Role::Link {
            href: node.value.clone(),
        },
        msaa_role::GRAPHIC => Role::Image,
        msaa_role::STATICTEXT => Role::Text,
        msaa_role::TEXT if editable => Role::Edit,
        msaa_role::PUSHBUTTON => Role::Button,
        msaa_role::CHECKBUTTON => Role::CheckBox,
        msaa_role::RADIOBUTTON => Role::RadioButton,
        msaa_role::COMBOBOX => Role::ComboBox,
        msaa_role::PROGRESSBAR => Role::ProgressBar,
        msaa_role::SLIDER => Role::Slider,
        msaa_role::LIST => Role::List,
        msaa_role::LISTITEM => Role::ListItem,
        msaa_role::OUTLINE => Role::Tree,
        msaa_role::OUTLINEITEM => Role::TreeItem,
        msaa_role::PAGETABLIST => Role::TabList,
        msaa_role::PAGETAB => Role::Tab,
        msaa_role::PROPERTYPAGE => Role::TabPanel,
        msaa_role::TABLE => Role::Table,
        msaa_role::ROW => Role::TableRow,
        msaa_role::CELL | msaa_role::COLUMNHEADER | msaa_role::ROWHEADER => Role::TableCell,
        msaa_role::EQUATION => Role::Math,
        // A fieldset; its name is the legend.
        // An editing host (contenteditable without a role): editable and focusable.
        // Its text children are editable too, but not focusable.
        _ if editable && node.msaa_states & FOCUSABLE != 0 => Role::Edit,
        msaa_role::GROUPING if named => Role::Group,
        // Any other element reads by its CSS box: block boxes break lines, inline
        // elements (strong, em, time, label) flow with the surrounding text.
        _ if is_block_display(attr("display")) => Role::Group,
        _ => Role::Text,
    }
}

/// Converts a captured IA2 tree into the engine's AOM.
///
/// Pure and platform-independent like [`crate::uia_snapshot::to_aom`], and with the same AOM
/// conventions: text leaves carry `text`, inline elements are `Text` nodes with children,
/// articles and fieldsets are `Group` with an `aria_role`, and header cells carry
/// `tag`=`th` plus `scope`.
///
/// IA2 is what JAWS reads in Chromium. It carries semantics UIA lacks or only localizes:
/// the HTML `tag` (block quotes), the CSS `display` (block versus inline), visited links.
pub fn ia2_to_aom(root: &Ia2Node) -> okur_core::AomNode {
    let mut next_id = 0;
    convert(root, &mut next_id)
}

fn convert(node: &Ia2Node, next_id: &mut usize) -> okur_core::AomNode {
    use okur_core::{AomNode, NodeStates, Role};
    use xa11y_windows::ia2::msaa_states::{
        STATE_SYSTEM_CHECKED, STATE_SYSTEM_COLLAPSED, STATE_SYSTEM_EXPANDED, STATE_SYSTEM_FOCUSED,
        STATE_SYSTEM_INVISIBLE, STATE_SYSTEM_PROTECTED, STATE_SYSTEM_SELECTED,
        STATE_SYSTEM_TRAVERSED, STATE_SYSTEM_UNAVAILABLE,
    };

    let id = *next_id;
    *next_id += 1;
    let role = map_role(node);
    let attr = |k: &str| node.attributes.get(k).cloned();
    let has = |bit: u32| node.msaa_states & bit != 0;

    let mut aom = AomNode::new(id, role.clone());
    aom.name = node.name.clone().filter(|n| !n.is_empty());
    match role {
        Role::Text if node.msaa_role == Some(msaa_role::STATICTEXT) => {
            aom.text = node.name.clone();
        }
        Role::Edit | Role::ComboBox => aom.value = node.value.clone(),
        // The document value is its URL; same-page links are resolved against it.
        Role::Document => {
            if let Some(url) = &node.value {
                aom.attributes.insert("url".to_owned(), url.clone());
            }
        }
        _ => {}
    }
    aom.level = attr("level").and_then(|l| l.parse().ok());
    aom.formatting = node
        .text_runs
        .first()
        .and_then(|run| text_formatting(&run.attributes));
    aom.states = NodeStates {
        checked: matches!(role, Role::CheckBox | Role::RadioButton)
            .then(|| has(STATE_SYSTEM_CHECKED)),
        selected: has(STATE_SYSTEM_SELECTED).then_some(true),
        expanded: if has(STATE_SYSTEM_EXPANDED) {
            Some(true)
        } else if has(STATE_SYSTEM_COLLAPSED) {
            Some(false)
        } else {
            None
        },
        focused: has(STATE_SYSTEM_FOCUSED),
        disabled: has(STATE_SYSTEM_UNAVAILABLE),
        hidden: has(STATE_SYSTEM_INVISIBLE),
        modal: node.ia2_states & ia2_state::MODAL != 0,
    };

    let mut set_attr = |key: &str, value: String| {
        aom.attributes.insert(key.to_owned(), value);
    };
    match (&role, node.msaa_role) {
        (Role::Group, Some(msaa_role::DOCUMENT)) => set_attr("aria_role", "article".to_owned()),
        // A fieldset: a named grouping. An unnamed one (a form, a section) keeps its xml-roles.
        (Role::Group, Some(msaa_role::GROUPING)) if aom.name.is_some() => {
            set_attr("aria_role", "group".to_owned())
        }
        (_, Some(msaa_role::COLUMNHEADER)) => {
            set_attr("tag", "th".to_owned());
            set_attr("scope", "col".to_owned());
        }
        (_, Some(msaa_role::ROWHEADER)) => {
            set_attr("tag", "th".to_owned());
            set_attr("scope", "row".to_owned());
        }
        _ => {
            if let Some(r) = attr("xml-roles") {
                set_attr("aria_role", r);
            }
        }
    }
    if node.ia2_states & ia2_state::REQUIRED != 0 {
        set_attr("required", "true".to_owned());
    }
    // Chrome keeps the whitespace of preformatted elements in their text (tabs, runs of
    // spaces, line breaks), and so does the virtual buffer.
    if matches!(
        attr("tag").as_deref(),
        Some("pre" | "listing" | "xmp" | "plaintext")
    ) {
        set_attr("preformatted", "true".to_owned());
    }
    if has(STATE_SYSTEM_PROTECTED) {
        set_attr("password", "true".to_owned());
    }
    if node.ia2_states & ia2_state::MULTI_LINE != 0 {
        set_attr("multiline", "true".to_owned());
    }
    if has(STATE_SYSTEM_TRAVERSED) {
        set_attr("visited", "true".to_owned());
    }
    // aria-describedby text, spoken after the value in focus phrasing.
    if let Some(d) = node.description.as_deref().filter(|d| !d.is_empty()) {
        set_attr("description", d.to_owned());
    }
    if let Some(dom_id) = attr("id") {
        set_attr("id", dom_id);
    }
    // The live object behind the node, for actions (focus, default action).
    if let Some(unique_id) = node.unique_id {
        set_attr(crate::actions::UNIQUE_ID_ATTR, unique_id.to_string());
    }
    if let (Some(pos), Some(size)) = (attr("posinset"), attr("setsize")) {
        if size != "0" {
            set_attr("posinset", pos);
            set_attr("setsize", size);
        }
    }

    aom.children = node.children.iter().map(|c| convert(c, next_id)).collect();
    aom
}
