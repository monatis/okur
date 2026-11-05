//! A page's AT-SPI tree as data, and as the engine's AOM: what the Linux driver reads when the
//! focus enters a page, and what snapshots of the fixtures keep for replay on every OS. Chrome's
//! AT-SPI tree of a page has the shape of its UI Automation tree (the Linux lab's
//! `probe-atspi-tree.sh`): "document web" named by the title, "heading" with its `level`
//! attribute, "paragraph", and "static" text leaves holding the text; so the conversion follows
//! `okur-windows-model`'s from UI Automation.

use std::collections::BTreeMap;

use okur_core::aom::{AomNode, NodeStates, Role};
use serde::{Deserialize, Serialize};

/// AT-SPI's `StateType` numbering: a state is its bit in the state set.
pub mod state {
    pub const CHECKED: u32 = 4;
    pub const EDITABLE: u32 = 7;
    pub const ENABLED: u32 = 8;
    pub const EXPANDABLE: u32 = 9;
    pub const EXPANDED: u32 = 10;
    pub const FOCUSABLE: u32 = 11;
    pub const FOCUSED: u32 = 12;
    pub const MODAL: u32 = 16;
    pub const MULTI_LINE: u32 = 17;
    pub const PRESSED: u32 = 20;
    pub const SELECTED: u32 = 23;
    pub const INDETERMINATE: u32 = 32;
    pub const REQUIRED: u32 = 33;
    pub const VISITED: u32 = 40;
    pub const CHECKABLE: u32 = 41;
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

/// Okur's role for an AT-SPI object of a page, by its role's name and its ARIA role
/// (`xml-roles`) for landmarks.
fn role(node: &AtspiNode) -> Role {
    let landmark = |kind: &str| Role::Landmark {
        landmark_type: kind.to_owned(),
    };
    if let Some(
        kind @ ("banner" | "complementary" | "contentinfo" | "main" | "navigation" | "region"
        | "search" | "form"),
    ) = node.attribute("xml-roles")
    {
        return landmark(kind);
    }
    match node.role.as_str() {
        "document web" | "document frame" => Role::Document,
        "heading" => Role::Heading {
            level: node
                .attribute("level")
                .and_then(|l| l.parse().ok())
                .unwrap_or(2),
        },
        "paragraph" => Role::Paragraph,
        "static" | "text" if !node.has(state::EDITABLE) => Role::Text,
        "link" => Role::Link { href: None },
        "push button" | "button" | "toggle button" => Role::Button,
        "check box" | "check menu item" => Role::CheckBox,
        "radio button" | "radio menu item" => Role::RadioButton,
        "entry" | "password text" | "text" | "spin button" => Role::Edit,
        "combo box" => Role::ComboBox,
        "list" | "list box" | "description list" => Role::List,
        "list item" | "description term" | "description value" => Role::ListItem,
        "table" => Role::Table,
        "table row" => Role::TableRow,
        "table cell" | "table column header" | "table row header" => Role::TableCell,
        "page tab list" => Role::TabList,
        "page tab" => Role::Tab,
        "dialog" => Role::Dialog,
        "alert" => Role::AlertDialog,
        "tree" | "tree table" => Role::Tree,
        "tree item" => Role::TreeItem,
        "slider" => Role::Slider,
        "progress bar" => Role::ProgressBar,
        "image" => Role::Image,
        "separator" => Role::Separator,
        "block quote" => Role::BlockQuote,
        "menu bar" => Role::MenuBar,
        "menu item" => Role::MenuItem,
        "frame" | "internal frame" => Role::IFrame,
        "math" => Role::Math,
        "form" => landmark("form"),
        "landmark" => landmark(node.attribute("xml-roles").unwrap_or("region")),
        "section" | "panel" | "grouping" => Role::Group,
        _ => Role::Generic,
    }
}

/// A page's AT-SPI tree as the engine's AOM.
pub fn to_aom(root: &AtspiNode) -> AomNode {
    let mut next_id = 0;
    convert(root, &mut next_id)
}

fn convert(node: &AtspiNode, next_id: &mut usize) -> AomNode {
    let id = *next_id;
    *next_id += 1;
    let role = role(node);
    let mut aom = AomNode::new(id, role.clone());
    aom.name = Some(node.name.clone()).filter(|n| !n.is_empty());
    match role {
        // A text leaf's words are its name, or its text.
        Role::Text => aom.text = aom.name.clone().or_else(|| node.text.clone()),
        Role::Edit | Role::ComboBox => {
            aom.value = node.text.clone().filter(|t| !t.contains('\u{fffc}'))
        }
        Role::Heading { level } => aom.level = Some(level),
        _ => {}
    }
    aom.states = NodeStates {
        checked: (node.has(state::CHECKABLE) || node.has(state::CHECKED))
            .then(|| node.has(state::CHECKED)),
        mixed: node.has(state::INDETERMINATE),
        selected: node.has(state::SELECTED).then_some(true),
        expanded: node
            .has(state::EXPANDABLE)
            .then(|| node.has(state::EXPANDED)),
        focused: node.has(state::FOCUSED),
        disabled: !node.has(state::ENABLED),
        hidden: false,
        modal: node.has(state::MODAL),
        focusable: Some(node.has(state::FOCUSABLE)),
        pressed: node.has(state::PRESSED),
    };
    if let Some(tag) = node.attribute("tag") {
        aom.attributes.insert("tag".to_owned(), tag.to_owned());
    }
    if node.has(state::REQUIRED) {
        aom.attributes
            .insert("required".to_owned(), "true".to_owned());
    }
    if node.has(state::VISITED) {
        aom.attributes
            .insert("visited".to_owned(), "true".to_owned());
    }
    if node.has(state::MULTI_LINE) && role == Role::Edit {
        aom.attributes
            .insert("multiline".to_owned(), "true".to_owned());
    }
    aom.children = node
        .children
        .iter()
        .map(|child| convert(child, next_id))
        .collect();
    aom
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(role: &str, name: &str, children: Vec<AtspiNode>) -> AtspiNode {
        AtspiNode {
            role: role.to_owned(),
            name: name.to_owned(),
            states: 1 << state::ENABLED,
            children,
            ..AtspiNode::default()
        }
    }

    /// doc-reading.html as Chrome 157 gives it over AT-SPI (the lab's probe-atspi-tree.sh).
    #[test]
    fn a_page_becomes_the_aom_the_buffer_reads() {
        let mut heading = node(
            "heading",
            "The Accessibility Engine Specification",
            vec![node(
                "static",
                "The Accessibility Engine Specification",
                vec![],
            )],
        );
        heading
            .attributes
            .insert("level".to_owned(), "1".to_owned());
        heading.attributes.insert("tag".to_owned(), "h1".to_owned());
        let paragraph = node(
            "paragraph",
            "",
            vec![node(
                "static",
                "The screen reader engine parses the document.",
                vec![],
            )],
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
        let mut nav = node("landmark", "Primary", vec![]);
        nav.attributes
            .insert("xml-roles".to_owned(), "navigation".to_owned());
        assert_eq!(
            to_aom(&nav).role,
            Role::Landmark {
                landmark_type: "navigation".to_owned()
            }
        );
    }
}
