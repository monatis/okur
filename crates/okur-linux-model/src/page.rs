//! A page's AT-SPI tree as data, and as the engine's AOM: what the Linux driver reads when the
//! focus enters a page, and what snapshots of the fixtures keep for replay on every OS.
//!
//! Chrome builds its AT-SPI objects and its IA2 objects (what JAWS reads on Windows) from the
//! same node of its accessibility tree, with the same object attributes (`tag`, `display`,
//! `xml-roles`, `level`): news-article.html over AT-SPI (the Linux lab's
//! `probe-atspi-tree.sh`) and over IA2 (`corpus/snapshots/news-article.chrome.ia2.json`) hold
//! the same elements. So a page read over AT-SPI is given, object by object, as okur-model's web
//! nodes, which Windows' IA2 objects become too, to the conversion Windows parity checks
//! (`okur_model::web::web_to_aom`): block and inline elements, landmarks, articles, header
//! cells, same-page links read on Linux as on Windows.

use std::collections::{BTreeMap, HashMap};

use okur_core::aom::AomNode;
use okur_model::web::{web_to_aom, WebMark, WebNode, WebRole, WebStates};
use serde::{Deserialize, Serialize};

/// AT-SPI's `StateType` numbering: a state is its bit in the state set.
pub mod state {
    pub const ACTIVE: u32 = 1;
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
    /// The text attributes of its text's first run, as Chrome gives them over AT-SPI.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub text_attributes: BTreeMap<String, String>,
    /// Its description (aria-describedby, aria-description, title), as Chrome gives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Its object's path, which relations name their targets by.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    /// Its relations: each one's AT-SPI type (its number) and its targets' paths.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<(u32, Vec<String>)>,
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

/// The main role and the finer one Chrome gives an element whose AT-SPI role is `node`'s, as
/// news-article.html's AT-SPI and IA2 snapshots pair them.
fn web_roles(node: &AtspiNode) -> (WebRole, Option<WebMark>) {
    let grouping = |mark: Option<WebMark>| (WebRole::Grouping, mark);
    match node.role.as_str() {
        // An iframe is an internal frame to Chrome's AT-SPI, a document with its tag to the
        // conversion, which says it as a frame (web-structures-2's "Weather frame").
        "document web" | "document frame" | "article" | "internal frame" => {
            (WebRole::Document, None)
        }
        "landmark" | "form" | "paragraph" | "caption" | "label" | "grouping" => grouping(None),
        // A group (a fieldset, role="group") and a log are panels to Chrome's AT-SPI, a generic
        // container a section: a grouping of its own, as IA2's grouping without its section role
        // (signup-form's "Membership type", web-structures' "Messages").
        "panel" | "log" => grouping(None),
        "heading" => grouping(Some(WebMark::Heading)),
        "block quote" => grouping(Some(WebMark::BlockQuote)),
        "content deletion" => grouping(Some(WebMark::ContentDeletion)),
        "content insertion" => grouping(Some(WebMark::ContentInsertion)),
        "mark" => grouping(Some(WebMark::Mark)),
        // Highlighted text (<mark>) is a static element to Chrome's AT-SPI, IA2's mark role on
        // Windows (edits.html's "library").
        "static" if node.attribute("tag") == Some("mark") => (WebRole::Text, Some(WebMark::Mark)),
        // A text leaf; an inline element holding text (strong, em) is a text object.
        "static" if node.children.is_empty() => (WebRole::StaticText, None),
        "static" => (WebRole::Text, None),
        "link" => (WebRole::Link, None),
        "push button" | "button" if node.attribute("haspopup").is_some() => {
            (WebRole::MenuButton, None)
        }
        "push button" | "button" => (WebRole::Button, None),
        "toggle button" => (WebRole::Button, Some(WebMark::ToggleButton)),
        "check box" | "check menu item" | "switch" => (WebRole::CheckBox, None),
        "radio button" | "radio menu item" => (WebRole::RadioButton, None),
        "entry" | "password text" | "text" | "spin button" => (WebRole::Text, None),
        "combo box" => (WebRole::ComboBox, None),
        "list" | "list box" | "description list" => (WebRole::List, None),
        "list item" | "description term" | "description value" => (WebRole::ListItem, None),
        "table" => (WebRole::Table, None),
        "table row" => (WebRole::Row, None),
        "table cell" => (WebRole::Cell, None),
        "column header" | "table column header" => (WebRole::ColumnHeader, None),
        "row header" | "table row header" => (WebRole::RowHeader, None),
        "page tab list" => (WebRole::TabList, None),
        "page tab" => (WebRole::Tab, None),
        "dialog" | "alert" => (WebRole::Dialog, None),
        "tree" | "tree table" => (WebRole::Tree, None),
        "tree item" => (WebRole::TreeItem, None),
        "slider" => (WebRole::Slider, None),
        "progress bar" => (WebRole::ProgressBar, None),
        "image" => (WebRole::Image, None),
        "separator" => (WebRole::Separator, None),
        "menu bar" => (WebRole::MenuBar, None),
        "menu item" => (WebRole::MenuItem, None),
        "math" => (WebRole::Math, None),
        // A generic element (section, panel, frame): its CSS display makes it a block or not.
        _ => grouping(Some(WebMark::Section)),
    }
}

/// `node`'s AT-SPI states by name.
fn web_states(node: &AtspiNode) -> WebStates {
    let has = |bit: u32| node.has(bit);
    WebStates {
        checked: has(state::CHECKED),
        mixed: has(state::INDETERMINATE),
        pressed: has(state::PRESSED),
        selected: has(state::SELECTED),
        expanded: has(state::EXPANDED),
        collapsed: has(state::EXPANDABLE) && !has(state::EXPANDED),
        focused: has(state::FOCUSED),
        focusable: has(state::FOCUSABLE),
        unavailable: !has(state::ENABLED),
        invisible: false,
        protected: node.role == "password text",
        readonly: has(state::READ_ONLY),
        traversed: has(state::VISITED),
        editable: has(state::EDITABLE),
        horizontal: has(state::HORIZONTAL),
        vertical: has(state::VERTICAL),
        invalid_entry: has(state::INVALID_ENTRY),
        modal: has(state::MODAL),
        multi_line: has(state::MULTI_LINE),
        required: has(state::REQUIRED),
        supports_autocompletion: has(state::SUPPORTS_AUTOCOMPLETION),
    }
}

/// The name of the item chosen in a combo box's own menu or list (selected), as its value.
fn chosen_item(node: &AtspiNode) -> Option<String> {
    node.children.iter().find_map(|child| {
        let items = matches!(child.role.as_str(), "menu" | "list box" | "list");
        items
            .then(|| {
                child
                    .children
                    .iter()
                    .find(|item| item.has(state::SELECTED) && !item.name.is_empty())
                    .map(|item| item.name.clone())
            })
            .flatten()
    })
}

/// `node` and its subtree as okur-model's web nodes, each with its place in the tree, depth
/// first from 1, as its unique id: what acting on a page's node finds its object by (the Linux
/// driver numbers the objects it read the same way).
pub fn atspi_to_web(node: &AtspiNode) -> WebNode {
    atspi_to_web_from(node, 0)
}

/// [`atspi_to_web`], numbered from `base` + 1: each page the live driver reads numbers its
/// objects apart from the pages before it, so an action finds its own page's object even after
/// another page was read (the code review of 2026-11-08, finding 9).
pub fn atspi_to_web_from(node: &AtspiNode, base: i32) -> WebNode {
    let mut ids = HashMap::new();
    let mut next = base;
    number_paths(node, &mut next, &mut ids);
    let mut next = base;
    numbered(node, &mut next, &ids)
}

/// Each object's id by its path, numbered as `numbered` numbers them, for relations' targets.
/// The unique ids [`atspi_to_web_from`] gives a page's objects, by their paths: what a focus
/// in the page names its node by, as Windows' focus names it by IA2's unique id.
pub fn page_ids(root: &AtspiNode, base: i32) -> HashMap<String, i32> {
    let mut ids = HashMap::new();
    let mut next = base;
    number_paths(root, &mut next, &mut ids);
    ids.into_iter()
        .map(|(path, id)| (path.to_owned(), id))
        .collect()
}

fn number_paths<'a>(node: &'a AtspiNode, next: &mut i32, ids: &mut HashMap<&'a str, i32>) {
    *next += 1;
    if !node.path.is_empty() {
        ids.insert(node.path.as_str(), *next);
    }
    for child in &node.children {
        number_paths(child, next, ids);
    }
}

/// A text run's attributes by the names the conversion reads (Chromium's IA2 names), from the
/// ATK names Chrome gives over AT-SPI, as the lab's capture of activation.html has them
/// (`family-name`, `size` "24pt", `weight` "700", `fg-color` and `bg-color` "0,0,0"); the
/// colors as CSS gives them on Windows, `rgb(0, 0, 0)`. Others are left out.
fn text_attributes(atk: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let rgb = |value: &str| {
        let parts: Vec<&str> = value.split(',').map(str::trim).collect();
        (parts.len() == 3).then(|| format!("rgb({})", parts.join(", ")))
    };
    let mut out = BTreeMap::new();
    for (name, value) in atk {
        let renamed = match name.as_str() {
            "family-name" => Some(("font-family", value.clone())),
            "size" => Some(("font-size", value.clone())),
            "weight" => Some(("font-weight", value.clone())),
            "style" => Some(("font-style", value.clone())),
            "underline" => Some(("text-underline-style", value.clone())),
            "strikethrough" => Some((
                "text-line-through-style",
                if value == "true" { "solid" } else { "none" }.to_owned(),
            )),
            "fg-color" => rgb(value).map(|v| ("color", v)),
            "bg-color" => rgb(value).map(|v| ("background-color", v)),
            // Raised and lowered text, named as IA2 names it ("super", "sub": edits.html's 23).
            "text-position" => Some(("text-position", value.clone())),
            _ => None,
        };
        if let Some((key, value)) = renamed {
            out.insert(key.to_owned(), value);
        }
    }
    out
}

/// AT-SPI's relation types (libatspi 2.60.4's `AtspiRelationType`, listed in the Linux lab,
/// 2026-11-08) by the names the conversion reads, as IA2's are.
fn relation_name(kind: u32) -> Option<&'static str> {
    match kind {
        1 => Some("labelFor"),
        10 => Some("flowsTo"),
        17 => Some("descriptionFor"),
        // An error message's target, the field's `aria-errormessage`.
        21 => Some("error"),
        _ => None,
    }
}

fn numbered(node: &AtspiNode, next: &mut i32, ids: &HashMap<&str, i32>) -> WebNode {
    *next += 1;
    let unique_id = *next;
    let relations = node
        .relations
        .iter()
        .filter_map(|(kind, paths)| {
            let targets: Vec<i32> = paths
                .iter()
                .filter_map(|path| ids.get(path.as_str()).copied())
                .collect();
            relation_name(*kind)
                .filter(|_| !targets.is_empty())
                .map(|name| (name.to_owned(), targets))
        })
        .collect();
    let (role, mark) = web_roles(node);
    // A field's value is its text; a link's and the document's, their address.
    let value = match role {
        WebRole::Text if node.has(state::EDITABLE) => node.text.clone(),
        // A select's text is its menu's mark: its value is the item chosen in it ("Northside").
        WebRole::ComboBox => node
            .text
            .clone()
            .filter(|t| !t.is_empty() && !t.contains('\u{fffc}'))
            .or_else(|| chosen_item(node)),
        WebRole::Link | WebRole::Document => node.uri.clone(),
        _ => None,
    };
    let number = |key: &str| node.attribute(key).and_then(|v| v.parse::<i32>().ok());
    let group_position = match (number("level"), number("setsize"), number("posinset")) {
        (None, None, None) => None,
        (level, size, position) => {
            Some([level.unwrap_or(0), size.unwrap_or(0), position.unwrap_or(0)])
        }
    };
    WebNode {
        role,
        mark,
        name: Some(node.name.clone()).filter(|n| !n.is_empty()),
        value,
        states: web_states(node),
        attributes: node.attributes.clone(),
        group_position,
        text: node.text.clone(),
        text_attributes: Some(text_attributes(&node.text_attributes)).filter(|a| !a.is_empty()),
        unique_id: Some(unique_id),
        description: node.description.clone(),
        relations,
        children: node
            .children
            .iter()
            .map(|child| numbered(child, next, ids))
            .collect(),
        ..WebNode::default()
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

/// A page's AT-SPI tree as the engine's AOM: okur-model's conversion of a page's web nodes.
pub fn to_aom(root: &AtspiNode) -> AomNode {
    web_to_aom(&atspi_to_web(root))
}

/// [`to_aom`], its objects numbered from `base` + 1 ([`atspi_to_web_from`]).
pub fn to_aom_from(root: &AtspiNode, base: i32) -> AomNode {
    web_to_aom(&atspi_to_web_from(root, base))
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

    /// A fieldset is a panel to Chrome's AT-SPI, a group to the engine; a select's value is the
    /// item chosen in its menu (signup-form's AT-SPI snapshot).
    #[test]
    fn a_fieldset_is_a_group_and_a_select_says_its_item() {
        let fieldset = block(
            node(
                "panel",
                "Membership type",
                vec![node("radio button", "Adult", vec![])],
            ),
            "fieldset",
        );
        let item = |name: &str| node("menu item", name, vec![]);
        let mut chosen = item("Northside");
        chosen.states |= 1 << state::SELECTED;
        let mut select = node(
            "combo box",
            "Home branch",
            vec![node(
                "menu",
                "",
                vec![item("Central"), chosen, item("Eastgate")],
            )],
        );
        select.text = Some("\u{fffc}".to_owned());
        let page = node("document web", "Sign up", vec![fieldset, select]);
        let aom = to_aom(&page);
        let group = aom.children.first();
        assert_eq!(group.map(|g| g.role.clone()), Some(Role::Group));
        assert_eq!(
            group
                .and_then(|g| g.attributes.get("aria_role"))
                .map(String::as_str),
            Some("group")
        );
        assert_eq!(
            aom.children.get(1).and_then(|c| c.value.clone()).as_deref(),
            Some("Northside")
        );
    }

    /// A relation names its target by the object's path; the web node names it by the target's
    /// numbered id, as IA2's relations name uniqueIDs ("Has Flows To", 563's page).
    #[test]
    fn a_relation_names_its_target_by_its_number() {
        let mut part = node(
            "paragraph",
            "",
            vec![node("static", "This part flows", vec![])],
        );
        part.path = "/a11y/2".to_owned();
        part.relations = vec![
            (10, vec!["/a11y/4".to_owned()]),
            (99, vec!["/a11y/4".to_owned()]),
        ];
        let mut list = node("list", "", vec![]);
        list.path = "/a11y/4".to_owned();
        let page = node("document web", "Flow", vec![part, list]);
        let web = atspi_to_web(&page);
        let flows = web.children.first().map(|p| p.relations.clone());
        assert_eq!(
            flows,
            Some(BTreeMap::from([("flowsTo".to_owned(), vec![4])]))
        );
    }

    /// A text run's ATK attributes become the names Say Font and Say Color read, its colors CSS's.
    #[test]
    fn a_runs_attributes_are_named_as_windows_names_them() {
        let atk: BTreeMap<String, String> = [
            ("family-name", "Liberation Serif"),
            ("size", "24pt"),
            ("weight", "700"),
            ("fg-color", "0,0,238"),
            ("bg-color", "255,255,255"),
            ("strikethrough", "true"),
            ("language", "en"),
        ]
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
        let named = text_attributes(&atk);
        assert_eq!(
            named.get("font-family").map(String::as_str),
            Some("Liberation Serif")
        );
        assert_eq!(named.get("font-size").map(String::as_str), Some("24pt"));
        assert_eq!(named.get("font-weight").map(String::as_str), Some("700"));
        assert_eq!(
            named.get("color").map(String::as_str),
            Some("rgb(0, 0, 238)")
        );
        assert_eq!(
            named.get("background-color").map(String::as_str),
            Some("rgb(255, 255, 255)")
        );
        assert_eq!(
            named.get("text-line-through-style").map(String::as_str),
            Some("solid")
        );
        assert!(!named.contains_key("language"));
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
