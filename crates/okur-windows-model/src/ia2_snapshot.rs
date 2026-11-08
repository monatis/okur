//! IAccessible2 snapshots of browser documents.
//!
//! Captured alongside UIA snapshots (`okur-lab snapshot --api ia2`) from the same fixture load,
//! with the same provenance block. IA2 adds what UIA lacks for web content: HTML tags and CSS
//! display (`tag`, `display` attributes), visited state, hypertext with embedded objects, and
//! the browser's rendered line ranges per text object.

use okur_model::snapshot::SnapshotProvenance;
use okur_model::web::{web_to_aom, WebMark, WebNode, WebRole, WebStates};
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

/// MSAA roles (`ROLE_SYSTEM_*` in `oleacc.h`) that Chromium reports for web content.
pub mod msaa_role {
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
pub mod ia2_state {
    pub const EDITABLE: i32 = 0x8;
    pub const HORIZONTAL: i32 = 0x10;
    pub const INVALID_ENTRY: i32 = 0x40;
    pub const VERTICAL: i32 = 0x20000;
    pub const MODAL: i32 = 0x100;
    pub const MULTI_LINE: i32 = 0x200;
    pub const REQUIRED: i32 = 0x800;
    pub const SUPPORTS_AUTOCOMPLETION: i32 = 0x8000;
}

/// An IA2 object as the web node okur-model converts (okur_model::web): its MSAA role as the
/// main role, its IA2 role as the finer one, and its MSAA and IA2 state bits, each by name.
pub fn ia2_to_web(node: &Ia2Node) -> WebNode {
    use xa11y_windows::ia2::msaa_states::{
        STATE_SYSTEM_CHECKED, STATE_SYSTEM_COLLAPSED, STATE_SYSTEM_EXPANDED,
        STATE_SYSTEM_FOCUSABLE, STATE_SYSTEM_FOCUSED, STATE_SYSTEM_INVISIBLE, STATE_SYSTEM_MIXED,
        STATE_SYSTEM_PRESSED, STATE_SYSTEM_PROTECTED, STATE_SYSTEM_READONLY, STATE_SYSTEM_SELECTED,
        STATE_SYSTEM_TRAVERSED, STATE_SYSTEM_UNAVAILABLE,
    };
    use xa11y_windows::ia2::roles::{
        IA2_ROLE_BLOCK_QUOTE, IA2_ROLE_CONTENT_DELETION, IA2_ROLE_CONTENT_INSERTION,
        IA2_ROLE_HEADING, IA2_ROLE_MARK, IA2_ROLE_SECTION, IA2_ROLE_TOGGLE_BUTTON,
    };
    let msaa = |bit: u32| node.msaa_states & bit != 0;
    let ia2 = |bit: i32| node.ia2_states & bit != 0;
    let role = match node.msaa_role.unwrap_or_default() {
        msaa_role::DOCUMENT => WebRole::Document,
        msaa_role::DIALOG => WebRole::Dialog,
        msaa_role::MENUBAR => WebRole::MenuBar,
        msaa_role::MENUITEM => WebRole::MenuItem,
        msaa_role::LINK => WebRole::Link,
        msaa_role::GRAPHIC => WebRole::Image,
        msaa_role::SEPARATOR => WebRole::Separator,
        msaa_role::STATICTEXT => WebRole::StaticText,
        msaa_role::TEXT => WebRole::Text,
        msaa_role::PUSHBUTTON => WebRole::Button,
        msaa_role::BUTTONMENU => WebRole::MenuButton,
        msaa_role::CHECKBUTTON => WebRole::CheckBox,
        msaa_role::RADIOBUTTON => WebRole::RadioButton,
        msaa_role::COMBOBOX => WebRole::ComboBox,
        msaa_role::PROGRESSBAR => WebRole::ProgressBar,
        msaa_role::SLIDER => WebRole::Slider,
        msaa_role::LIST => WebRole::List,
        msaa_role::LISTITEM => WebRole::ListItem,
        msaa_role::OUTLINE => WebRole::Tree,
        msaa_role::OUTLINEITEM => WebRole::TreeItem,
        msaa_role::PAGETABLIST => WebRole::TabList,
        msaa_role::PAGETAB => WebRole::Tab,
        msaa_role::PROPERTYPAGE => WebRole::TabPanel,
        msaa_role::TABLE => WebRole::Table,
        msaa_role::ROW => WebRole::Row,
        msaa_role::CELL => WebRole::Cell,
        msaa_role::COLUMNHEADER => WebRole::ColumnHeader,
        msaa_role::ROWHEADER => WebRole::RowHeader,
        msaa_role::EQUATION => WebRole::Math,
        msaa_role::GROUPING => WebRole::Grouping,
        _ => WebRole::Other,
    };
    let mark = match node.ia2_role {
        Some(IA2_ROLE_HEADING) => Some(WebMark::Heading),
        Some(IA2_ROLE_BLOCK_QUOTE) => Some(WebMark::BlockQuote),
        Some(IA2_ROLE_TOGGLE_BUTTON) => Some(WebMark::ToggleButton),
        Some(IA2_ROLE_SECTION) => Some(WebMark::Section),
        Some(IA2_ROLE_CONTENT_DELETION) => Some(WebMark::ContentDeletion),
        Some(IA2_ROLE_CONTENT_INSERTION) => Some(WebMark::ContentInsertion),
        Some(IA2_ROLE_MARK) => Some(WebMark::Mark),
        _ => None,
    };
    WebNode {
        role,
        mark,
        name: node.name.clone(),
        value: node.value.clone(),
        description: node.description.clone(),
        default_action: node.default_action.clone(),
        value_min: node.value_min,
        value_max: node.value_max,
        states: WebStates {
            checked: msaa(STATE_SYSTEM_CHECKED),
            mixed: msaa(STATE_SYSTEM_MIXED),
            pressed: msaa(STATE_SYSTEM_PRESSED),
            selected: msaa(STATE_SYSTEM_SELECTED),
            expanded: msaa(STATE_SYSTEM_EXPANDED),
            collapsed: msaa(STATE_SYSTEM_COLLAPSED),
            focused: msaa(STATE_SYSTEM_FOCUSED),
            focusable: msaa(STATE_SYSTEM_FOCUSABLE),
            unavailable: msaa(STATE_SYSTEM_UNAVAILABLE),
            invisible: msaa(STATE_SYSTEM_INVISIBLE),
            protected: msaa(STATE_SYSTEM_PROTECTED),
            readonly: msaa(STATE_SYSTEM_READONLY),
            traversed: msaa(STATE_SYSTEM_TRAVERSED),
            editable: ia2(ia2_state::EDITABLE),
            horizontal: ia2(ia2_state::HORIZONTAL),
            vertical: ia2(ia2_state::VERTICAL),
            invalid_entry: ia2(ia2_state::INVALID_ENTRY),
            modal: ia2(ia2_state::MODAL),
            multi_line: ia2(ia2_state::MULTI_LINE),
            required: ia2(ia2_state::REQUIRED),
            supports_autocompletion: ia2(ia2_state::SUPPORTS_AUTOCOMPLETION),
        },
        unique_id: node.unique_id,
        attributes: node.attributes.clone(),
        group_position: node.group_position,
        text: node.text.clone(),
        text_attributes: node.text_runs.first().map(|run| run.attributes.clone()),
        relations: node.relations.clone(),
        children: node.children.iter().map(ia2_to_web).collect(),
    }
}

/// Converts a captured IA2 tree into the engine's AOM: okur-model's conversion of a page
/// (`okur_model::web::web_to_aom`), from the IA2 objects named (`ia2_to_web`).
///
/// IA2 is what JAWS reads in Chromium. It carries semantics UIA lacks or only localizes:
/// the HTML `tag` (block quotes), the CSS `display` (block versus inline), visited links.
pub fn ia2_to_aom(root: &Ia2Node) -> okur_core::AomNode {
    web_to_aom(&ia2_to_web(root))
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
