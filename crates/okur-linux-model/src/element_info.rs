//! A page's element as Element Information shows it (Insert+Shift+F1), from its AT-SPI object:
//! its object attributes, Chromium's same as over IA2, with its address where it is a link or
//! the document and its ARIA role where Chromium names one (`xml-roles`), sorted by name, as
//! `okur_windows_model::element_info` makes them from IA2 [obs, 1210/s2]. AT-SPI has no MSAA
//! role: an element's block ends with its parameters.

use crate::page::AtspiNode;
use okur_core::event::ElementDetails;

/// `node` as Element Information shows it; none for an object with no tag (a text leaf).
pub fn details(node: &AtspiNode) -> Option<ElementDetails> {
    let tag = node
        .attributes
        .get("tag")
        .filter(|t| !t.is_empty())?
        .clone();
    let mut parameters: Vec<(String, String)> = node
        .attributes
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    let addressed = matches!(node.role.as_str(), "link" | "document web");
    if let (true, Some(href)) = (addressed, node.uri.as_ref().filter(|u| !u.is_empty())) {
        parameters.push(("href".to_owned(), href.clone()));
    }
    if let Some(role) = node.attributes.get("xml-roles").filter(|r| !r.is_empty()) {
        if !node.attributes.contains_key("role") {
            parameters.push(("role".to_owned(), role.clone()));
        }
    }
    parameters.sort();
    Some(ElementDetails {
        tag,
        parameters,
        role: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(role: &str, tag: &str, attributes: &[(&str, &str)]) -> AtspiNode {
        let mut node = AtspiNode {
            role: role.to_owned(),
            ..AtspiNode::default()
        };
        node.attributes.insert("tag".to_owned(), tag.to_owned());
        for (name, value) in attributes {
            node.attributes
                .insert((*name).to_owned(), (*value).to_owned());
        }
        node
    }

    /// news-article.html's banner link over AT-SPI: its attributes and its address, its
    /// header's role, as the IA2 ones list them; a text leaf has none.
    #[test]
    fn a_links_details_are_its_attributes_and_address() {
        let mut link = node("link", "a", &[("display", "inline")]);
        link.uri = Some("file:///news-article.html#top".to_owned());
        let names = |node: &AtspiNode| -> Vec<String> {
            details(node)
                .map(|d| {
                    d.parameters
                        .iter()
                        .map(|(n, v)| format!("{n}={v}"))
                        .collect()
                })
                .unwrap_or_default()
        };
        assert_eq!(
            names(&link),
            [
                "display=inline",
                "href=file:///news-article.html#top",
                "tag=a"
            ]
        );
        let header = node("landmark", "header", &[("xml-roles", "banner")]);
        assert_eq!(
            names(&header),
            ["role=banner", "tag=header", "xml-roles=banner"]
        );
        assert_eq!(details(&header).and_then(|d| d.role), None);
        assert!(details(&AtspiNode::default()).is_none());
    }
}
