//! A page's live regions on Linux (ARIA `aria-live`, and the roles live by themselves: a log, a
//! status, an alert), as Chrome gives them over AT-SPI: a region's root has its own `live`
//! attribute, every object inside it `container-live`. Chrome tells a region's change by its
//! objects' children and text changing, which the driver hears for the objects of known regions
//! alone ([`live_objects`]) and answers by reading the region again; the converter compares the
//! region's text with its text before and says what changed, then tells the engine the page
//! changed, which reads the page again once its changes settle, as Windows' live region event
//! carries its region and its page is captured again later [obs, 206/s2-s10]. A page read again
//! for any reason is compared the same way.

use std::collections::HashMap;

use okur_core::aom::{AomNode, UNIQUE_ID_ATTR};
use okur_core::event::{LiveRegionEvent, Politeness};

use crate::page::AtspiNode;

/// A live region of a page, as its root's attributes give it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// The key the engine knows the region by: its DOM id, else its object's path (a region
    /// without an id is known by its text in no buffer, and its text is given to the engine
    /// when it is first read).
    pub key: String,
    /// Whether the key is its DOM id.
    pub dom_id: bool,
    /// Its object's path.
    pub path: String,
    pub name: Option<String>,
    pub politeness: Politeness,
    pub atomic: bool,
}

/// The live regions of a page: the objects with a `live` attribute of their own, outermost
/// first.
pub fn regions(root: &AtspiNode) -> Vec<Region> {
    let mut regions = Vec::new();
    collect_regions(root, &mut regions);
    regions
}

fn collect_regions(node: &AtspiNode, regions: &mut Vec<Region>) {
    let attribute = |name: &str| node.attributes.get(name).map(String::as_str);
    if let (Some(live), false) = (attribute("live"), node.path.is_empty()) {
        let politeness = match live {
            "assertive" => Politeness::Assertive,
            "off" => Politeness::Off,
            _ => Politeness::Polite,
        };
        let id = attribute("id").filter(|id| !id.is_empty());
        regions.push(Region {
            key: id.map_or_else(|| format!("atspi:{}", node.path), str::to_owned),
            dom_id: id.is_some(),
            path: node.path.clone(),
            name: Some(node.name.trim())
                .filter(|n| !n.is_empty())
                .map(str::to_owned),
            politeness,
            atomic: attribute("atomic") == Some("true"),
        });
    }
    for child in &node.children {
        collect_regions(child, regions);
    }
}

/// The objects of a page's live regions by their paths, each with its region's path (a region's
/// own object is its own region): the objects whose children or text changing tells a region's
/// change, and the region read again for it. A region whose politeness is off is not heard; a
/// region inside another is its own.
pub fn live_objects(root: &AtspiNode) -> HashMap<String, String> {
    let mut objects = HashMap::new();
    collect_objects(root, None, &mut objects);
    objects
}

fn collect_objects(node: &AtspiNode, region: Option<&str>, objects: &mut HashMap<String, String>) {
    let attribute = |name: &str| node.attributes.get(name).map(String::as_str);
    let own = attribute("live").filter(|live| !live.is_empty());
    let region = match own {
        Some("off") => None,
        Some(_) if !node.path.is_empty() => Some(node.path.as_str()),
        _ => region,
    };
    if let Some(region) = region.filter(|_| !node.path.is_empty()) {
        objects.insert(node.path.clone(), region.to_owned());
    }
    for child in &node.children {
        collect_objects(child, region, objects);
    }
}

/// Each region's text lines in the page's AOM, by the region's key: the texts within it, as the
/// engine reads a region's lines from its buffer. `ids` gives each object's unique id by its
/// path ([`crate::page::page_ids`]).
pub fn region_lines(
    aom: &AomNode,
    regions: &[Region],
    ids: &HashMap<String, i32>,
) -> HashMap<String, Vec<String>> {
    let by_id: HashMap<String, &str> = regions
        .iter()
        .filter_map(|r| Some((ids.get(&r.path)?.to_string(), r.key.as_str())))
        .collect();
    let mut lines = HashMap::new();
    collect_lines(aom, &by_id, &mut lines);
    lines
}

fn collect_lines(
    node: &AomNode,
    by_id: &HashMap<String, &str>,
    out: &mut HashMap<String, Vec<String>>,
) {
    if let Some(key) = node
        .attributes
        .get(UNIQUE_ID_ATTR)
        .and_then(|id| by_id.get(id))
    {
        let mut texts = Vec::new();
        node_texts(node, &mut texts);
        out.insert((*key).to_owned(), texts);
    }
    for child in &node.children {
        collect_lines(child, by_id, out);
    }
}

fn node_texts(node: &AomNode, out: &mut Vec<String>) {
    if let Some(text) = node
        .text
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        out.push(text.to_owned());
    }
    for child in &node.children {
        node_texts(child, out);
    }
}

/// What the regions' change between two reads of a page says, in the page's order: a region
/// whose lines differ from `before`'s is the engine's live region event with its lines now; a
/// region without a DOM id read for the first time is given to the engine silently (politeness
/// off), for its next change to be compared with it.
pub fn changes(
    regions: &[Region],
    before: Option<&HashMap<String, Vec<String>>>,
    now: &HashMap<String, Vec<String>>,
) -> Vec<LiveRegionEvent> {
    let mut events = Vec::new();
    for region in regions {
        let Some(lines) = now.get(&region.key) else {
            continue;
        };
        let event = |politeness: Politeness| {
            let mut live = LiveRegionEvent::new(lines.join("\n"), politeness);
            live.container_name.clone_from(&region.name);
            live.region_id = Some(region.key.clone());
            live.atomic = region.atomic;
            live
        };
        match before.map(|b| b.get(&region.key)) {
            Some(Some(old)) if old != lines => events.push(event(region.politeness)),
            Some(Some(_)) => {}
            // New to this page's reads: known to the engine through its buffer by its DOM id;
            // else given to it now, unsaid.
            _ if !region.dom_id => events.push(event(Politeness::Off)),
            _ => {}
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use okur_core::aom::Role;

    fn node(role: &str, name: &str, path: &str, attributes: &[(&str, &str)]) -> AtspiNode {
        let mut node = AtspiNode {
            role: role.to_owned(),
            name: name.to_owned(),
            path: path.to_owned(),
            ..AtspiNode::default()
        };
        for (key, value) in attributes {
            node.attributes
                .insert((*key).to_owned(), (*value).to_owned());
        }
        node
    }

    fn page(messages: &[&str]) -> AtspiNode {
        let mut log = node(
            "log",
            "Messages",
            "/log",
            &[
                ("live", "polite"),
                ("container-live", "polite"),
                ("id", "log"),
            ],
        );
        for (i, text) in messages.iter().enumerate() {
            log.children.push(node(
                "static",
                text,
                &format!("/message/{i}"),
                &[("container-live", "polite")],
            ));
        }
        let status = node(
            "status bar",
            "",
            "/status",
            &[("live", "polite"), ("atomic", "true")],
        );
        let mut root = node("document web", "Team Chat", "/doc", &[]);
        root.children = vec![log, status];
        root
    }

    fn aom(messages: &[&str], ids: &HashMap<String, i32>) -> AomNode {
        let with_id = |mut n: AomNode, path: &str| {
            if let Some(id) = ids.get(path) {
                n.attributes
                    .insert(UNIQUE_ID_ATTR.to_owned(), id.to_string());
            }
            n
        };
        let log = with_id(AomNode::new(1, Role::Group), "/log").with_children(
            messages
                .iter()
                .enumerate()
                .map(|(i, m)| AomNode::new(10 + i, Role::Text).with_text(*m))
                .collect(),
        );
        let status = with_id(AomNode::new(2, Role::Group), "/status");
        AomNode::new(0, Role::Document).with_children(vec![log, status])
    }

    /// A log's message added between two reads of the page is its change, with its lines now;
    /// a region without a DOM id is first given to the engine unsaid; nothing changed, nothing
    /// said [obs, 206/s2].
    #[test]
    fn a_regions_change_between_two_reads_is_said() {
        let first = page(&["Ada: Good morning"]);
        let regions_first = regions(&first);
        assert_eq!(regions_first.len(), 2);
        assert_eq!(regions_first.first().map(|r| r.key.as_str()), Some("log"));
        let ids = crate::page::page_ids(&first, 0);
        let lines_first = region_lines(&aom(&["Ada: Good morning"], &ids), &regions_first, &ids);
        let seeded = changes(&regions_first, None, &lines_first);
        assert_eq!(seeded.len(), 1);
        assert!(seeded
            .iter()
            .all(|e| e.politeness == Politeness::Off
                && e.region_id.as_deref() == Some("atspi:/status")));

        let second = page(&["Ada: Good morning", "You: Message 1"]);
        let regions_second = regions(&second);
        let ids = crate::page::page_ids(&second, 0);
        let lines_second = region_lines(
            &aom(&["Ada: Good morning", "You: Message 1"], &ids),
            &regions_second,
            &ids,
        );
        let said = changes(&regions_second, Some(&lines_first), &lines_second);
        assert_eq!(said.len(), 1);
        let message = said.first();
        assert_eq!(
            message.map(|e| e.text.as_str()),
            Some("Ada: Good morning\nYou: Message 1")
        );
        assert_eq!(
            message.and_then(|e| e.container_name.as_deref()),
            Some("Messages")
        );
        assert_eq!(message.map(|e| e.politeness), Some(Politeness::Polite));
        assert!(changes(&regions_second, Some(&lines_second), &lines_second).is_empty());
    }

    /// The objects heard for a region's change are those of live regions, each with its
    /// region; an off region's are not.
    #[test]
    fn the_live_objects_are_the_regions_objects() {
        let mut root = page(&["Ada: Good morning"]);
        root.children
            .push(node("panel", "Quiet", "/quiet", &[("live", "off")]));
        let objects = live_objects(&root);
        assert_eq!(objects.get("/log").map(String::as_str), Some("/log"));
        assert_eq!(objects.get("/message/0").map(String::as_str), Some("/log"));
        assert_eq!(objects.get("/status").map(String::as_str), Some("/status"));
        assert!(!objects.contains_key("/quiet") && !objects.contains_key("/doc"));
    }
}
