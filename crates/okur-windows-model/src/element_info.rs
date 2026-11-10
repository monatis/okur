//! A page's element as Element Information shows it (Insert+Shift+F1), from its IA2 object:
//! its object attributes, with its address where it is a link or the document (its IA2 value)
//! and its ARIA role where Chromium names one (`xml-roles`), sorted by name, and its IA2 role:
//! "Tag A has 5 parameters: | display=inline | href=file:///...#top | name-from=contents |
//! tag=a | text-align=left | MSAA Role=1E" [obs, 1210/s2]. The live driver reads the objects,
//! parity replay takes them from a page's IA2 snapshot.

use okur_core::event::ElementDetails;
use xa11y_windows::ia2::Ia2Node;

/// MSAA's link and document roles, whose value is their address.
const ROLE_SYSTEM_LINK: i32 = 0x1e;
const ROLE_SYSTEM_DOCUMENT: i32 = 0xf;

/// `node` as Element Information shows it; none for an object with no tag (a text leaf).
pub fn details(node: &Ia2Node) -> Option<ElementDetails> {
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
    let addressed = matches!(
        node.msaa_role,
        Some(ROLE_SYSTEM_LINK | ROLE_SYSTEM_DOCUMENT)
    );
    if let (true, Some(href)) = (addressed, node.value.as_ref().filter(|v| !v.is_empty())) {
        parameters.push(("href".to_owned(), href.clone()));
    }
    // The landmark's role as JAWS gives it beside Chromium's xml-roles: "role=banner"
    // [obs, 1210/s2].
    if let Some(role) = node.attributes.get("xml-roles").filter(|r| !r.is_empty()) {
        if !node.attributes.contains_key("role") {
            parameters.push(("role".to_owned(), role.clone()));
        }
    }
    parameters.sort();
    Some(ElementDetails {
        tag,
        parameters,
        role: node
            .ia2_role
            .or(node.msaa_role)
            .and_then(|r| u32::try_from(r).ok()),
    })
}

/// The elements from the object `unique_id` of the page `root` up to the document, as
/// Element Information lists them; empty where the page has no such object.
pub fn chain(root: &Ia2Node, unique_id: i32) -> Vec<ElementDetails> {
    let mut path = Vec::new();
    if !find(root, unique_id, &mut path) {
        return Vec::new();
    }
    path.iter().rev().filter_map(|node| details(node)).collect()
}

/// Whether `node` holds the object `unique_id`, with `path` the nodes from `node` to it.
fn find<'a>(node: &'a Ia2Node, unique_id: i32, path: &mut Vec<&'a Ia2Node>) -> bool {
    path.push(node);
    if node.unique_id == Some(unique_id) {
        return true;
    }
    if node
        .children
        .iter()
        .any(|child| find(child, unique_id, path))
    {
        return true;
    }
    path.pop();
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(tag: &str, attributes: &[(&str, &str)], role: i32, id: i32) -> Ia2Node {
        let mut node = Ia2Node {
            msaa_role: Some(role),
            ia2_role: Some(role),
            unique_id: Some(id),
            ..Ia2Node::default()
        };
        node.attributes.insert("tag".to_owned(), tag.to_owned());
        for (name, value) in attributes {
            node.attributes
                .insert((*name).to_owned(), (*value).to_owned());
        }
        node
    }

    /// news-article.html's banner link, as its IA2 snapshot holds it and JAWS lists it: the
    /// link with its address, the header with its role, the document with its address
    /// [obs, 1210/s2].
    #[test]
    fn a_links_elements_are_jaws_parameters() {
        let mut link = node(
            "a",
            &[("display", "inline"), ("name-from", "contents")],
            0x1e,
            -3,
        );
        link.value = Some("file:///news-article.html#top".to_owned());
        // A text leaf of the link has no tag, and is not listed.
        let mut leaf = Ia2Node {
            unique_id: Some(-4),
            ..Ia2Node::default()
        };
        leaf.name = Some("Riverside Gazette".to_owned());
        link.children.push(leaf);
        let mut header = node("header", &[("xml-roles", "banner")], 1069, -2);
        header.msaa_role = Some(20);
        header.children.push(link);
        let mut document = node("#document", &[("explicit-name", "true")], 0xf, -1);
        document.value = Some("file:///news-article.html".to_owned());
        document.children.push(header);
        let elements = chain(&document, -4);
        let tags: Vec<&str> = elements.iter().map(|e| e.tag.as_str()).collect();
        assert_eq!(tags, ["a", "header", "#document"]);
        let names = |at: usize| -> Vec<String> {
            elements
                .get(at)
                .map(|e| {
                    e.parameters
                        .iter()
                        .map(|(n, v)| format!("{n}={v}"))
                        .collect()
                })
                .unwrap_or_default()
        };
        assert_eq!(
            names(0),
            [
                "display=inline",
                "href=file:///news-article.html#top",
                "name-from=contents",
                "tag=a"
            ]
        );
        assert_eq!(names(1), ["role=banner", "tag=header", "xml-roles=banner"]);
        assert_eq!(elements.get(1).and_then(|e| e.role), Some(1069));
        assert_eq!(
            names(2),
            [
                "explicit-name=true",
                "href=file:///news-article.html",
                "tag=#document"
            ]
        );
        assert!(chain(&document, -99).is_empty());
    }
}
