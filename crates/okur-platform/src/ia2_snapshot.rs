//! IAccessible2 snapshots of browser documents.
//!
//! Captured alongside UIA snapshots (`okur-lab snapshot --api ia2`) from the same fixture load,
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
    pub const MENUBAR: i32 = 0x02;
    pub const MENUITEM: i32 = 0x0C;
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
    pub const BUTTONMENU: i32 = 0x39;
    pub const SEPARATOR: i32 = 0x15;
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

/// Whether Chromium reports `node` as a grouping of its own (a fieldset, `role=group`, a
/// log): MSAA calls every generic `div` and `span` a grouping too, and only IA2's role
/// (`IA2_ROLE_SECTION`) tells them apart. WhatsApp names its message wrappers with
/// `aria-label`, and JAWS reads them as plain text, not as groups.
fn is_grouping(node: &Ia2Node) -> bool {
    use xa11y_windows::ia2::roles::IA2_ROLE_SECTION;
    node.msaa_role == Some(msaa_role::GROUPING) && node.ia2_role != Some(IA2_ROLE_SECTION)
}

/// Whether `node` is a generic element (`div`, `span`) with an `aria-label` and nothing
/// inside: it reads as its label (WhatsApp's "Siz:" before each of the user's messages,
/// which JAWS reads as a line of its own).
fn is_labelled_empty_generic(node: &Ia2Node) -> bool {
    use xa11y_windows::ia2::roles::IA2_ROLE_SECTION;
    node.ia2_role == Some(IA2_ROLE_SECTION)
        && node.children.is_empty()
        && node.attributes.get("name-from").map(String::as_str) == Some("attribute")
        && node.name.as_deref().is_some_and(|n| !n.trim().is_empty())
}

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

/// Say Font's and Say Color's formatting from an IA2 text run (`font-family`, `font-size`,
/// `font-weight`, `font-style`, `text-underline-style`, `color`, `background-color`), and
/// what skim reading finds text by (`text-line-through-style`, `text-position`).
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
        strikeout: get("text-line-through-style").is_some_and(|s| !s.is_empty() && s != "none"),
        superscript: get("text-position") == Some("super"),
        subscript: get("text-position") == Some("sub"),
        alignment: None,
        color: get("color").and_then(okur_core::color::Rgb::parse_css),
        background: get("background-color").and_then(okur_core::color::Rgb::parse_css),
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
        msaa_role::MENUBAR => Role::MenuBar,
        msaa_role::MENUITEM => Role::MenuItem,
        msaa_role::LINK => Role::Link {
            href: node.value.clone(),
        },
        msaa_role::GRAPHIC => Role::Image,
        msaa_role::SEPARATOR => Role::Separator,
        msaa_role::STATICTEXT => Role::Text,
        msaa_role::TEXT if editable => Role::Edit,
        msaa_role::PUSHBUTTON => Role::Button,
        // Chromium makes a button with any popup but a dialog a "button menu"; JAWS says
        // "button menu" only for a menu ("Pick one Button Has Popup listbox", dev box).
        msaa_role::BUTTONMENU if matches!(attr("haspopup"), None | Some("menu")) => {
            Role::MenuButton
        }
        msaa_role::BUTTONMENU => Role::Button,
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
        msaa_role::GROUPING if named && is_grouping(node) => Role::Group,
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
        STATE_SYSTEM_CHECKED, STATE_SYSTEM_COLLAPSED, STATE_SYSTEM_EXPANDED,
        STATE_SYSTEM_FOCUSABLE, STATE_SYSTEM_FOCUSED, STATE_SYSTEM_INVISIBLE, STATE_SYSTEM_PRESSED,
        STATE_SYSTEM_PROTECTED, STATE_SYSTEM_READONLY, STATE_SYSTEM_SELECTED,
        STATE_SYSTEM_TRAVERSED, STATE_SYSTEM_UNAVAILABLE,
    };

    use xa11y_windows::ia2::roles::IA2_ROLE_TOGGLE_BUTTON;
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
        Role::Text | Role::Group if is_labelled_empty_generic(node) => {
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
        // A toggle button (`aria-pressed`) is checked when pressed.
        checked: if node.ia2_role == Some(IA2_ROLE_TOGGLE_BUTTON) {
            Some(has(STATE_SYSTEM_PRESSED))
        } else {
            matches!(role, Role::CheckBox | Role::RadioButton).then(|| has(STATE_SYSTEM_CHECKED))
        },
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
        focusable: Some(has(STATE_SYSTEM_FOCUSABLE)),
    };

    let mut set_attr = |key: &str, value: String| {
        aom.attributes.insert(key.to_owned(), value);
    };
    match (&role, node.msaa_role) {
        (Role::Group, Some(msaa_role::DOCUMENT)) => set_attr("aria_role", "article".to_owned()),
        // A fieldset: a named grouping. An unnamed one (a form, a section) keeps its xml-roles.
        (Role::Group, Some(msaa_role::GROUPING)) if aom.name.is_some() && is_grouping(node) => {
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
    // Struck or deleted text (<s>, <del>), which JAWS reads between "Deletion" and "end
    // Deletion" [obs, 814/s3].
    if node.ia2_role == Some(xa11y_windows::ia2::roles::IA2_ROLE_CONTENT_DELETION) {
        set_attr("aria_role", "deletion".to_owned());
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
    // Paragraphs and divisions are blocks like any other in the AOM; JAWS lists them apart
    // (Select a Paragraph, Select a Division) by their tag [obs, 239, 244].
    if let Some(tag) = attr("tag").filter(|t| matches!(t.as_str(), "p" | "div")) {
        set_attr("tag", tag);
    }
    // A read-only field (GitHub's code view is a read-only text area).
    if role == Role::Edit && has(STATE_SYSTEM_READONLY) {
        set_attr("readonly", "true".to_owned());
    }
    if has(STATE_SYSTEM_PROTECTED) {
        set_attr("password", "true".to_owned());
    }
    // A combo box one types in (an autocomplete), JAWS's "edit combo". Chrome's UIA reports
    // a select's value as writable too, so only IA2's editable state tells them apart.
    if role == Role::ComboBox && node.ia2_states & ia2_state::EDITABLE != 0 {
        set_attr("editable", "true".to_owned());
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
    // aria-current: "page", "step", "location", "date", "time" or "true".
    if let Some(current) = attr("current").filter(|c| c != "false") {
        set_attr("current", current);
    }
    // A menu item that opens a menu of its own: "File menu sub menu collapsed" [obs, 218/s1].
    if role == Role::MenuItem && attr("haspopup").is_some_and(|p| p != "false") {
        set_attr("submenu", "true".to_owned());
    }
    // A button's popup other than a menu ("listbox", "dialog"); a menu makes a menu button.
    if let Some(popup) =
        attr("haspopup").filter(|p| !matches!(p.as_str(), "false" | "true" | "menu"))
    {
        set_attr("haspopup", popup);
    }
    // A name given by aria-label (or title), which a link says in place of its text.
    if attr("name-from").as_deref() == Some("attribute") {
        set_attr("labelled", "true".to_owned());
    }
    // aria-keyshortcuts, said after a link's or graphic's name ("Alt+ArrowUp").
    if let Some(keys) = attr("keyshortcuts").filter(|k| !k.trim().is_empty()) {
        set_attr("keyshortcuts", keys);
    }
    // aria-roledescription, said in place of some roles.
    if let Some(description) = attr("roledescription") {
        set_attr("roledescription", description);
    }
    // aria-flowto: the objects this one flows to, by their IA2 uniqueIDs (Equals follows them).
    if let Some(targets) = node.relations.get("flowsTo").filter(|t| !t.is_empty()) {
        let ids: Vec<String> = targets.iter().map(i32::to_string).collect();
        set_attr("flows-to", ids.join(" "));
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

#[cfg(test)]
mod tests {
    use super::{ia2_to_aom, msaa_role, Ia2Node};
    use okur_core::Role;
    use xa11y_windows::ia2::msaa_states::{STATE_SYSTEM_FOCUSABLE, STATE_SYSTEM_PRESSED};
    use xa11y_windows::ia2::roles::{IA2_ROLE_SECTION, IA2_ROLE_TOGGLE_BUTTON};

    fn node(msaa: i32, ia2: i32, name: &str, attributes: &[(&str, &str)]) -> Ia2Node {
        Ia2Node {
            msaa_role: Some(msaa),
            ia2_role: Some(ia2),
            name: Some(name.to_owned()).filter(|n| !n.is_empty()),
            attributes: attributes
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            ..Default::default()
        }
    }

    /// Struck-through, raised and lowered text, as Chromium marks its runs
    /// (`text-line-through-style:solid`, `text-position:super` or `sub`): what skim reading's
    /// Strikeout, Superscript and Subscript find.
    #[test]
    fn a_run_says_struck_raised_and_lowered_text() {
        let run = |pairs: &[(&str, &str)]| {
            let attributes = [("font-family", "Times New Roman"), ("font-size", "12pt")]
                .iter()
                .chain(pairs)
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect();
            super::text_formatting(&attributes).unwrap_or_default()
        };
        assert!(run(&[("text-line-through-style", "solid")]).strikeout);
        assert!(!run(&[("text-line-through-style", "none")]).strikeout);
        let raised = run(&[("text-position", "super")]);
        assert!(raised.superscript && !raised.subscript);
        let lowered = run(&[("text-position", "sub")]);
        assert!(lowered.subscript && !lowered.superscript);
        assert!(!run(&[("text-position", "baseline")]).superscript);
    }

    /// Chromium calls every `div` and `span` an MSAA grouping; only IA2's role tells a
    /// generic one (a section) from a fieldset. WhatsApp names its messages' wrappers with
    /// `aria-label` ("Siz test 14:13 Okundu"), and JAWS reads them as plain text, with no
    /// "group start" (dev box, 2026-10-17); a fieldset stays a group.
    #[test]
    fn a_labelled_div_is_not_a_group() {
        let text = Ia2Node {
            msaa_role: Some(msaa_role::STATICTEXT),
            name: Some("test".to_owned()),
            ..Default::default()
        };
        let mut wrapper = node(
            msaa_role::GROUPING,
            IA2_ROLE_SECTION,
            "Siz test 14:13 Okundu",
            &[("display", "flex"), ("name-from", "attribute")],
        );
        wrapper.children = vec![text];
        let aom = ia2_to_aom(&wrapper);
        assert_eq!(aom.role, Role::Group);
        assert_eq!(aom.attributes.get("aria_role"), None);

        let fieldset = node(
            msaa_role::GROUPING,
            msaa_role::GROUPING,
            "Membership type",
            &[("display", "block"), ("tag", "fieldset")],
        );
        let aom = ia2_to_aom(&fieldset);
        assert_eq!(aom.role, Role::Group);
        assert_eq!(
            aom.attributes.get("aria_role").map(String::as_str),
            Some("group")
        );
    }

    /// An empty `span` with an `aria-label` reads as its label: WhatsApp's "Siz:" before the
    /// first of the user's messages, a line of its own for JAWS (dev box, 2026-10-17).
    #[test]
    fn an_empty_labelled_span_reads_its_label() {
        let label = node(
            msaa_role::GROUPING,
            IA2_ROLE_SECTION,
            "Siz:",
            &[("display", "inline"), ("name-from", "attribute")],
        );
        let aom = ia2_to_aom(&label);
        assert_eq!(aom.role, Role::Text);
        assert_eq!(aom.text.as_deref(), Some("Siz:"));
    }

    /// A toggle button (`aria-pressed`) is checked when pressed (WhatsApp's "Sohbetler"), and
    /// a capture says which elements can take the focus.
    #[test]
    fn a_toggle_button_is_checked_when_pressed() {
        let mut pressed = node(
            msaa_role::PUSHBUTTON,
            IA2_ROLE_TOGGLE_BUTTON,
            "Sohbetler",
            &[],
        );
        pressed.msaa_states = STATE_SYSTEM_PRESSED | STATE_SYSTEM_FOCUSABLE;
        let aom = ia2_to_aom(&pressed);
        assert_eq!(aom.role, Role::Button);
        assert_eq!(aom.states.checked, Some(true));
        assert_eq!(aom.states.focusable, Some(true));

        let plain = node(
            msaa_role::PUSHBUTTON,
            msaa_role::PUSHBUTTON,
            "14:13 Okundu",
            &[],
        );
        let aom = ia2_to_aom(&plain);
        assert_eq!(aom.states.checked, None);
        assert_eq!(aom.states.focusable, Some(false));
    }
}
