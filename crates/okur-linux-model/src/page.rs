//! A page's AT-SPI tree as data, and as the engine's AOM: what the Linux driver reads when the
//! focus enters a page, and what snapshots of the fixtures keep for replay on every OS.
//!
//! Chrome builds its AT-SPI objects and its IA2 objects (what JAWS reads on Windows) from the
//! same node of its accessibility tree, with the same object attributes (`tag`, `display`,
//! `xml-roles`, `level`): news-article.html over AT-SPI (the Linux lab's
//! `probe-atspi-tree.sh`) and over IA2 (`corpus/snapshots/news-article.chrome.ia2.json`) hold
//! the same elements. So a page read over AT-SPI is given, object by object, as the IA2 objects
//! Chrome gives on Windows, to the conversion Windows parity checks
//! (`okur_windows_model::ia2_snapshot::ia2_to_aom`): block and inline elements, landmarks,
//! articles, header cells, same-page links read on Linux as on Windows.

use std::collections::BTreeMap;

use okur_core::aom::AomNode;
use okur_windows_model::ia2_snapshot::{ia2_state, msaa_role};
use serde::{Deserialize, Serialize};
use xa11y_windows::ia2::{msaa_states as msaa, roles as ia2_role, Ia2Node};

/// AT-SPI's `StateType` numbering: a state is its bit in the state set.
pub mod state {
    pub const BUSY: u32 = 3;
    pub const CHECKED: u32 = 4;
    pub const EDITABLE: u32 = 7;
    pub const ENABLED: u32 = 8;
    pub const EXPANDABLE: u32 = 9;
    pub const EXPANDED: u32 = 10;
    pub const FOCUSABLE: u32 = 11;
    pub const FOCUSED: u32 = 12;
    pub const HORIZONTAL: u32 = 14;
    pub const MODAL: u32 = 16;
    pub const MULTI_LINE: u32 = 17;
    pub const PRESSED: u32 = 20;
    pub const SELECTED: u32 = 23;
    pub const VERTICAL: u32 = 29;
    pub const INDETERMINATE: u32 = 32;
    pub const REQUIRED: u32 = 33;
    pub const INVALID_ENTRY: u32 = 36;
    pub const SUPPORTS_AUTOCOMPLETION: u32 = 37;
    pub const VISITED: u32 = 40;
    pub const CHECKABLE: u32 = 41;
    pub const READ_ONLY: u32 = 43;
}

/// One object of a page's AT-SPI tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtspiNode {
    /// The role's name ("document web", "heading", "static").
    pub role: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, String>,
    /// The state bits, by [`state`]'s numbering.
    #[serde(default)]
    pub states: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// An address: a link's, or the document's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<AtspiNode>,
}

impl AtspiNode {
    pub fn has(&self, state: u32) -> bool {
        1u64.checked_shl(state)
            .is_some_and(|bit| self.states & bit != 0)
    }

    fn attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(String::as_str)
    }
}

/// The MSAA role and the IA2 role Chrome gives on Windows the element it gives over AT-SPI with
/// `node`'s role, as news-article.html's two snapshots pair them; the IA2 role is the MSAA one
/// where Chrome gives no other.
fn ia2_roles(node: &AtspiNode) -> (i32, i32) {
    let same = |role: i32| (role, role);
    let section = (msaa_role::GROUPING, ia2_role::IA2_ROLE_SECTION);
    match node.role.as_str() {
        "document web" | "document frame" | "article" => same(msaa_role::DOCUMENT),
        "landmark" => (msaa_role::GROUPING, ia2_role::IA2_ROLE_LANDMARK),
        "form" => (msaa_role::GROUPING, ia2_role::IA2_ROLE_FORM),
        "heading" => (msaa_role::GROUPING, ia2_role::IA2_ROLE_HEADING),
        "paragraph" => (msaa_role::GROUPING, ia2_role::IA2_ROLE_PARAGRAPH),
        "block quote" => (msaa_role::GROUPING, ia2_role::IA2_ROLE_BLOCK_QUOTE),
        "caption" => (msaa_role::GROUPING, ia2_role::IA2_ROLE_CAPTION),
        "label" => (msaa_role::GROUPING, ia2_role::IA2_ROLE_LABEL),
        "content deletion" => (msaa_role::GROUPING, ia2_role::IA2_ROLE_CONTENT_DELETION),
        "content insertion" => (msaa_role::GROUPING, ia2_role::IA2_ROLE_CONTENT_INSERTION),
        "mark" => (msaa_role::GROUPING, ia2_role::IA2_ROLE_MARK),
        "grouping" => same(msaa_role::GROUPING),
        // A text leaf; an inline element holding text (strong, em) is a text frame.
        "static" if node.children.is_empty() => same(msaa_role::STATICTEXT),
        "static" => (msaa_role::TEXT, IA2_ROLE_TEXT_FRAME),
        "link" => same(msaa_role::LINK),
        "push button" | "button" if node.attribute("haspopup").is_some() => {
            same(msaa_role::BUTTONMENU)
        }
        "push button" | "button" => same(msaa_role::PUSHBUTTON),
        "toggle button" => (msaa_role::PUSHBUTTON, ia2_role::IA2_ROLE_TOGGLE_BUTTON),
        "check box" | "check menu item" | "switch" => same(msaa_role::CHECKBUTTON),
        "radio button" | "radio menu item" => same(msaa_role::RADIOBUTTON),
        "entry" | "password text" | "text" | "spin button" => same(msaa_role::TEXT),
        "combo box" => same(msaa_role::COMBOBOX),
        "list" | "list box" | "description list" => same(msaa_role::LIST),
        "list item" | "description term" | "description value" => same(msaa_role::LISTITEM),
        "table" => same(msaa_role::TABLE),
        "table row" => same(msaa_role::ROW),
        "table cell" => same(msaa_role::CELL),
        "column header" | "table column header" => same(msaa_role::COLUMNHEADER),
        "row header" | "table row header" => same(msaa_role::ROWHEADER),
        "page tab list" => same(msaa_role::PAGETABLIST),
        "page tab" => same(msaa_role::PAGETAB),
        "dialog" | "alert" => same(msaa_role::DIALOG),
        "tree" | "tree table" => same(msaa_role::OUTLINE),
        "tree item" => same(msaa_role::OUTLINEITEM),
        "slider" => same(msaa_role::SLIDER),
        "progress bar" => same(msaa_role::PROGRESSBAR),
        "image" => same(msaa_role::GRAPHIC),
        "separator" => same(msaa_role::SEPARATOR),
        "menu bar" => same(msaa_role::MENUBAR),
        "menu item" => same(msaa_role::MENUITEM),
        "math" => same(msaa_role::EQUATION),
        // A generic element (section, panel, frame): its CSS display makes it a block or not.
        _ => section,
    }
}

/// IA2's role for a text frame (`IA2_ROLE_TEXT_FRAME`): Chrome's inline element holding text.
const IA2_ROLE_TEXT_FRAME: i32 = 0x429;

/// The MSAA and IA2 state bits for `node`'s AT-SPI states.
fn ia2_states(node: &AtspiNode) -> (u32, i32) {
    let mut m = 0;
    let msaa_bits = [
        (state::SELECTED, msaa::STATE_SYSTEM_SELECTED),
        (state::FOCUSED, msaa::STATE_SYSTEM_FOCUSED),
        (state::PRESSED, msaa::STATE_SYSTEM_PRESSED),
        (state::CHECKED, msaa::STATE_SYSTEM_CHECKED),
        (state::INDETERMINATE, msaa::STATE_SYSTEM_MIXED),
        (state::READ_ONLY, msaa::STATE_SYSTEM_READONLY),
        (state::EXPANDED, msaa::STATE_SYSTEM_EXPANDED),
        (state::BUSY, msaa::STATE_SYSTEM_BUSY),
        (state::FOCUSABLE, msaa::STATE_SYSTEM_FOCUSABLE),
        (state::VISITED, msaa::STATE_SYSTEM_TRAVERSED),
    ];
    for (at, bit) in msaa_bits {
        if node.has(at) {
            m |= bit;
        }
    }
    if !node.has(state::ENABLED) {
        m |= msaa::STATE_SYSTEM_UNAVAILABLE;
    }
    if node.has(state::EXPANDABLE) && !node.has(state::EXPANDED) {
        m |= msaa::STATE_SYSTEM_COLLAPSED;
    }
    if node.role == "link" {
        m |= msaa::STATE_SYSTEM_LINKED;
    }
    if node.role == "password text" {
        m |= msaa::STATE_SYSTEM_PROTECTED;
    }
    let mut i = 0;
    let ia2_bits = [
        (state::EDITABLE, ia2_state::EDITABLE),
        (state::HORIZONTAL, ia2_state::HORIZONTAL),
        (state::INVALID_ENTRY, ia2_state::INVALID_ENTRY),
        (state::VERTICAL, ia2_state::VERTICAL),
        (state::MODAL, ia2_state::MODAL),
        (state::MULTI_LINE, ia2_state::MULTI_LINE),
        (state::REQUIRED, ia2_state::REQUIRED),
        (
            state::SUPPORTS_AUTOCOMPLETION,
            ia2_state::SUPPORTS_AUTOCOMPLETION,
        ),
    ];
    for (at, bit) in ia2_bits {
        if node.has(at) {
            i |= bit;
        }
    }
    (m, i)
}

/// `node` and its subtree as the IA2 objects Chrome gives on Windows for the same elements, each
/// with its place in the tree, depth first from 1, as its unique id: what acting on a page's node
/// finds its object by (the Linux driver numbers the objects it read the same way).
pub fn to_ia2(node: &AtspiNode) -> Ia2Node {
    let mut next = 0;
    numbered(node, &mut next)
}

fn numbered(node: &AtspiNode, next: &mut i32) -> Ia2Node {
    *next += 1;
    let unique_id = *next;
    let (msaa_role, ia2_role) = ia2_roles(node);
    let (msaa_states, ia2_states) = ia2_states(node);
    // A field's value is its text; a link's and the document's, their address.
    let value = match msaa_role {
        msaa_role::TEXT if node.has(state::EDITABLE) => node.text.clone(),
        msaa_role::COMBOBOX => node.text.clone().filter(|t| !t.contains('\u{fffc}')),
        msaa_role::LINK | msaa_role::DOCUMENT => node.uri.clone(),
        _ => None,
    };
    let number = |key: &str| node.attribute(key).and_then(|v| v.parse::<i32>().ok());
    let group_position = match (number("level"), number("setsize"), number("posinset")) {
        (None, None, None) => None,
        (level, size, position) => {
            Some([level.unwrap_or(0), size.unwrap_or(0), position.unwrap_or(0)])
        }
    };
    Ia2Node {
        msaa_role: Some(msaa_role),
        ia2_role: Some(ia2_role),
        name: Some(node.name.clone()).filter(|n| !n.is_empty()),
        value,
        msaa_states,
        ia2_states,
        attributes: node.attributes.clone(),
        group_position,
        text: node.text.clone(),
        unique_id: Some(unique_id),
        children: node
            .children
            .iter()
            .map(|child| numbered(child, next))
            .collect(),
        ..Ia2Node::default()
    }
}

/// A page's AT-SPI tree as the Linux lab captured it (`corpus/snapshots/<fixture>.chrome.atspi.json`,
/// the plan's §2.3): Chrome's document read through xa11y-linux, with the provenance Windows'
/// snapshots carry, replayed on every OS through [`to_aom`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtspiSnapshot {
    pub provenance: okur_model::snapshot::SnapshotProvenance,
    pub root: AtspiNode,
}

/// A page's AT-SPI tree as the engine's AOM, through the IA2 objects Chrome gives on Windows.
pub fn to_aom(root: &AtspiNode) -> AomNode {
    okur_windows_model::ia2_snapshot::ia2_to_aom(&to_ia2(root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use okur_core::aom::Role;

    fn node(role: &str, name: &str, children: Vec<AtspiNode>) -> AtspiNode {
        AtspiNode {
            role: role.to_owned(),
            name: name.to_owned(),
            states: 1 << state::ENABLED,
            children,
            ..AtspiNode::default()
        }
    }

    fn block(mut node: AtspiNode, tag: &str) -> AtspiNode {
        node.attributes.insert("tag".to_owned(), tag.to_owned());
        node.attributes
            .insert("display".to_owned(), "block".to_owned());
        node
    }

    /// doc-reading.html as Chrome 157 gives it over AT-SPI (the lab's probe-atspi-tree.sh).
    #[test]
    fn a_page_becomes_the_aom_the_buffer_reads() {
        let mut heading = block(
            node(
                "heading",
                "The Accessibility Engine Specification",
                vec![node(
                    "static",
                    "The Accessibility Engine Specification",
                    vec![],
                )],
            ),
            "h1",
        );
        heading
            .attributes
            .insert("level".to_owned(), "1".to_owned());
        let paragraph = block(
            node(
                "paragraph",
                "",
                vec![node(
                    "static",
                    "The screen reader engine parses the document.",
                    vec![],
                )],
            ),
            "p",
        );
        let page = node(
            "document web",
            "Document Reading Reference Fixture",
            vec![heading, paragraph],
        );
        let aom = to_aom(&page);
        assert_eq!(aom.role, Role::Document);
        assert_eq!(
            aom.name.as_deref(),
            Some("Document Reading Reference Fixture")
        );
        let heading = aom.children.first().map(|h| (h.role.clone(), h.level));
        assert_eq!(heading, Some((Role::Heading { level: 1 }, Some(1))));
        let text = aom
            .children
            .get(1)
            .and_then(|p| p.children.first())
            .and_then(|t| t.text.clone());
        assert_eq!(
            text.as_deref(),
            Some("The screen reader engine parses the document.")
        );
    }

    #[test]
    fn states_and_landmarks() {
        let mut check = node("check box", "Subscribe", vec![]);
        check.states |= (1 << state::CHECKABLE) | (1 << state::CHECKED) | (1 << state::FOCUSABLE);
        let aom = to_aom(&check);
        assert_eq!(aom.states.checked, Some(true));
        assert_eq!(aom.states.focusable, Some(true));
        assert!(!aom.states.disabled);
        let mut nav = block(node("landmark", "Primary", vec![]), "nav");
        nav.attributes
            .insert("xml-roles".to_owned(), "navigation".to_owned());
        assert_eq!(
            to_aom(&nav).role,
            Role::Landmark {
                landmark_type: "navigation".to_owned()
            }
        );
    }

    /// news-article.html's parts as Chrome gives them over AT-SPI: a link's address and the
    /// document's (the same-page links JAWS says so), the article, and the table's header cells,
    /// read as Chrome's IA2 objects for them are.
    #[test]
    fn links_articles_and_header_cells_read_as_on_windows() {
        let mut link = node("link", "News", vec![node("static", "News", vec![])]);
        link.uri = Some("file:///fixtures/news-article.html#news".to_owned());
        link.attributes.insert("tag".to_owned(), "a".to_owned());
        let mut header = node(
            "column header",
            "Branch",
            vec![node("static", "Branch", vec![])],
        );
        header.attributes.insert("tag".to_owned(), "th".to_owned());
        let table = node(
            "table",
            "Branch hours",
            vec![node("table row", "", vec![header])],
        );
        let article = block(node("article", "", vec![link, table]), "article");
        let mut page = node("document web", "News", vec![article]);
        page.uri = Some("file:///fixtures/news-article.html".to_owned());
        let aom = to_aom(&page);
        assert_eq!(
            aom.attributes.get("url").map(String::as_str),
            Some("file:///fixtures/news-article.html")
        );
        let article = aom.children.first();
        assert_eq!(article.map(|a| &a.role), Some(&Role::Group));
        assert_eq!(
            article.and_then(|a| a.attributes.get("aria_role").map(String::as_str)),
            Some("article")
        );
        let link = article.and_then(|a| a.children.first());
        assert_eq!(
            link.map(|l| &l.role),
            Some(&Role::Link {
                href: Some("file:///fixtures/news-article.html#news".to_owned())
            })
        );
        let header = article
            .and_then(|a| a.children.get(1))
            .and_then(|t| t.children.first())
            .and_then(|r| r.children.first());
        assert_eq!(header.map(|h| &h.role), Some(&Role::TableCell));
        assert_eq!(
            header.and_then(|h| h.attributes.get("scope").map(String::as_str)),
            Some("col")
        );
    }
}
