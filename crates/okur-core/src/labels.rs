//! Custom labels: names the user gives a page's elements, read in place of the page's own
//! (JAWS's Custom Labels, Surf's Up "Using Custom Labels"; [obs, 688]). JAWS keeps them per
//! domain, and applies a label to the same element wherever it appears on the domain's pages.
//! Okur keeps them per site, and per page address for a page without a site (a `file:` page).
//! An element is known by its DOM id, else by its role and the name it had.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::aom::{AomNode, Role};

/// The attribute keeping a labelled node's own name.
const OWN_NAME: &str = "own_name";
/// The attribute keeping whether a labelled link was named by its page (aria-label) before.
const OWN_LABELLED: &str = "own_labelled";

/// Every place's labels: by place, then by element.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CustomLabels {
    pub places: BTreeMap<String, BTreeMap<String, String>>,
}

/// Where custom labels are kept, so that they hold the next time Okur starts.
pub trait LabelStore: Send {
    fn save(&self, labels: &CustomLabels);
}

/// The place a page's labels belong to: its site, or its address without its fragment.
pub fn place_of(url: &str) -> String {
    crate::settings_layers::site_of(url)
        .unwrap_or_else(|| url.split('#').next().unwrap_or(url).to_owned())
}

/// How an element is known again: its DOM id, else its role and the name it had.
pub fn element_key(node: &AomNode) -> Option<String> {
    if let Some(id) = node.attributes.get("id").filter(|id| !id.is_empty()) {
        return Some(format!("id {id}"));
    }
    // A labelled node is known by the name it had.
    let name = node
        .attributes
        .get(OWN_NAME)
        .map(String::as_str)
        .or(node.name.as_deref())
        .or(node.text.as_deref())
        .map(str::trim)
        .filter(|n| !n.is_empty())?;
    Some(format!("{} {name}", role_key(&node.role)))
}

/// A role as an element's key names it.
fn role_key(role: &Role) -> &'static str {
    match role {
        Role::Link { .. } => "link",
        Role::Button | Role::MenuButton | Role::SplitButton => "button",
        Role::Edit => "edit",
        Role::CheckBox => "checkbox",
        Role::RadioButton => "radio",
        Role::ComboBox => "combobox",
        Role::Image => "graphic",
        Role::Heading { .. } => "heading",
        _ => "element",
    }
}

impl CustomLabels {
    /// Whether `place` has any label.
    pub fn has(&self, place: &str) -> bool {
        self.places
            .get(place)
            .is_some_and(|labels| !labels.is_empty())
    }

    /// Gives `node` of `place` the label `label`, or takes its label away when there is none.
    pub fn set(&mut self, place: &str, node: &AomNode, label: Option<&str>) -> bool {
        let Some(key) = element_key(node) else {
            return false;
        };
        match label.map(str::trim).filter(|l| !l.is_empty()) {
            Some(label) => {
                self.places
                    .entry(place.to_owned())
                    .or_default()
                    .insert(key, label.to_owned());
            }
            None => {
                if let Some(labels) = self.places.get_mut(place) {
                    labels.remove(&key);
                    if labels.is_empty() {
                        self.places.remove(place);
                    }
                }
            }
        }
        true
    }

    /// `page` with the labels of `place` in its elements' names, a link's read in place of its
    /// text.
    pub fn apply(&self, place: &str, page: &AomNode) -> AomNode {
        let mut page = page.clone();
        let none = BTreeMap::new();
        label_tree(&mut page, self.places.get(place).unwrap_or(&none));
        page
    }
}

/// Gives each node its label, and a node whose label was taken away its own name back.
fn label_tree(node: &mut AomNode, labels: &BTreeMap<String, String>) {
    match element_key(node).and_then(|key| labels.get(&key)) {
        Some(label) => {
            if !node.attributes.contains_key(OWN_NAME) {
                let own = node.name.clone().or_else(|| node.text.clone());
                node.attributes
                    .insert(OWN_NAME.to_owned(), own.unwrap_or_default());
                if let Some(labelled) = node.attributes.get("labelled").cloned() {
                    node.attributes.insert(OWN_LABELLED.to_owned(), labelled);
                }
            }
            node.name = Some(label.clone());
            if matches!(node.role, Role::Link { .. }) {
                node.attributes
                    .insert("labelled".to_owned(), "true".to_owned());
            }
        }
        None => {
            if let Some(own) = node.attributes.remove(OWN_NAME) {
                node.name = (!own.is_empty()).then_some(own);
                match node.attributes.remove(OWN_LABELLED) {
                    Some(labelled) => node.attributes.insert("labelled".to_owned(), labelled),
                    None => node.attributes.remove("labelled"),
                };
            }
        }
    }
    for child in &mut node.children {
        label_tree(child, labels);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field() -> AomNode {
        let mut field = AomNode::new(2, Role::Edit).with_name("Branch");
        field
            .attributes
            .insert("id".to_owned(), "pickup-branch".to_owned());
        field
    }

    /// A label takes the element's name on the page it was given on, and on another page of
    /// the same site; a page without a site keeps its own.
    #[test]
    fn a_label_names_its_element_on_its_sites_pages() {
        let mut labels = CustomLabels::default();
        let place = place_of("https://library.example/pickup#form");
        assert_eq!(place, "library.example");
        assert!(labels.set(&place, &field(), Some(" lamp ")));
        let page = AomNode::new(0, Role::Document).with_children(vec![field()]);
        let labelled = labels.apply(&place_of("https://library.example/other"), &page);
        assert_eq!(
            labelled.children.first().and_then(|f| f.name.as_deref()),
            Some("lamp")
        );
        assert_eq!(
            place_of("file:///C:/fixtures/custom-label.html#top"),
            "file:///C:/fixtures/custom-label.html"
        );
        // An empty label takes it away.
        assert!(labels.set(&place, &field(), Some("")));
        assert!(!labels.has(&place));
    }

    /// A label given again or taken away from the page as labelled before: the element keeps
    /// its key, and gets its own name back.
    #[test]
    fn a_label_taken_away_gives_the_name_back() {
        let mut labels = CustomLabels::default();
        let mut named = AomNode::new(2, Role::Button).with_name("Reserve");
        named.attributes.remove("id");
        let page = AomNode::new(0, Role::Document).with_children(vec![named]);
        assert!(labels.set("site", page.children.first().unwrap_or(&page), Some("Hold")));
        let labelled = labels.apply("site", &page);
        let button = labelled.children.first().unwrap_or(&labelled);
        assert_eq!(button.name.as_deref(), Some("Hold"));
        // Known by the name it had, though it now reads its label.
        assert_eq!(element_key(button).as_deref(), Some("button Reserve"));
        assert!(labels.set("site", button, None));
        let restored = labels.apply("site", &labelled);
        assert_eq!(
            restored.children.first().and_then(|b| b.name.as_deref()),
            Some("Reserve")
        );
    }

    /// A link's label is read in place of its text.
    #[test]
    fn a_links_label_takes_its_texts_place() {
        let mut labels = CustomLabels::default();
        let link = AomNode::new(
            3,
            Role::Link {
                href: Some("mailto:web@example.org".to_owned()),
            },
        )
        .with_name("click here");
        assert!(labels.set("example.org", &link, Some("Send the Webmaster e-mail")));
        let page = AomNode::new(0, Role::Document).with_children(vec![link]);
        let labelled = labels.apply("example.org", &page);
        let link = labelled.children.first();
        assert_eq!(
            link.and_then(|l| l.name.as_deref()),
            Some("Send the Webmaster e-mail")
        );
        assert_eq!(
            link.and_then(|l| l.attributes.get("labelled"))
                .map(String::as_str),
            Some("true")
        );
    }
}
