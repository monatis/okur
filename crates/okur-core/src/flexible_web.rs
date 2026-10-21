//! Flexible Web's customizations (JAWS 2025: Surf's Up, chapter 16, "Take Control of Web Pages
//! with Flexible Web"; [obs, 705, 708, 716, 720]): an element of the page, chosen among the
//! ones around the virtual cursor, is hidden or reading starts at it. A customization names
//! the element by its kind, its text and its place among the elements of its kind, so that it
//! finds the element again when the page is built anew: "Hide ComplementaryRegion 1 from the
//! top with text: "Advertisement"", "Hide all MainRegions".
//!
//! This module is the customizations alone, on a page's tree; the wizard that makes them is
//! [`crate::ui::flexible_web`], and the engine keeps them for the page in front.

use serde::{Deserialize, Serialize};

use crate::aom::{AomNode, Role};
use crate::messages::msg;

/// What a customization does to the element it finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// "Hide an element".
    Hide,
    /// "Start reading at an element": where the page is read from when it loads.
    StartAt,
}

/// A kind of element Flexible Web finds again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A region (landmark), by its type: "main", "banner", "complementary".
    Region(String),
    /// A heading, by its level.
    Heading(u32),
    List,
    Frame,
}

impl Kind {
    /// The kind of `node`, when Flexible Web offers it: regions, headings, lists and frames.
    /// A paragraph is not offered: in the advertisement only its region is [obs, 720/s8].
    pub fn of(node: &AomNode) -> Option<Self> {
        match &node.role {
            Role::Landmark { landmark_type } => region_name(landmark_type)
                .is_some()
                .then(|| Self::Region(landmark_type.clone())),
            Role::Heading { level } => Some(Self::Heading(*level)),
            Role::List => Some(Self::List),
            Role::IFrame => Some(Self::Frame),
            _ => None,
        }
    }

    /// The kind as the wizard names it: "MainRegion", "heading level 1", "list" [obs, 705/s15,
    /// 716/s7, s8].
    pub fn name(&self) -> String {
        match self {
            Self::Region(kind) => region_name(kind).map(|n| n.to_string()).unwrap_or_default(),
            Self::Heading(level) => msg::flexible_web_heading(*level).to_string(),
            Self::List => msg::role_list().to_string(),
            Self::Frame => msg::flexible_web_frame().to_string(),
        }
    }

    /// The kind's plural, for "Hide all": "MainRegions" [obs, 708/s17], "frames" (the guide).
    fn plural(&self) -> String {
        format!("{}s", self.name())
    }

    fn matches(&self, node: &AomNode) -> bool {
        Self::of(node).as_ref() == Some(self)
    }
}

/// A region's name, one word, as JAWS writes it.
fn region_name(kind: &str) -> Option<crate::messages::StaticMessage> {
    Some(match kind {
        "main" => msg::role_region_main(),
        "banner" => msg::role_region_banner(),
        "complementary" => msg::role_region_complementary(),
        "navigation" => msg::role_region_navigation(),
        "contentinfo" => msg::role_region_content_info(),
        "search" => msg::role_region_search(),
        "form" => msg::role_region_form(),
        "region" => msg::role_region(),
        _ => return None,
    })
}

/// An element's text, as a customization matches it: its name, or the text inside it.
fn text_of(node: &AomNode) -> String {
    fn gather(node: &AomNode, out: &mut Vec<String>) {
        if let Some(text) = node
            .text
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            out.push(text.to_owned());
        }
        for child in &node.children {
            gather(child, out);
        }
    }
    if let Some(name) = node
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        return name.to_owned();
    }
    let mut parts = Vec::new();
    gather(node, &mut parts);
    parts.join(" ")
}

/// An element's text as Flexible Web gives it: a region's or a frame's name, not the words in
/// it ("MainRegion" [obs, 705/s15], "ComplementaryRegion Advertisement" [obs, 720/s8]); a
/// heading's words; a list has none ("list" [obs, 716/s7]).
fn text_for(kind: &Kind, node: &AomNode) -> String {
    match kind {
        Kind::Heading(_) => text_of(node),
        Kind::List => String::new(),
        Kind::Region(_) | Kind::Frame => node
            .name
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_owned(),
    }
}

/// An element of the page Flexible Web offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub node: usize,
    pub kind: Kind,
    /// Its text: a region's name, a heading's words; a list has none.
    pub text: String,
    /// Its id in the page ("events").
    pub id: Option<String>,
}

fn dom_id(node: &AomNode) -> Option<String> {
    node.attributes
        .get("id")
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

impl Element {
    fn new(node: &AomNode) -> Option<Self> {
        let kind = Kind::of(node)?;
        let text = text_for(&kind, node);
        Some(Self {
            node: node.id,
            kind,
            text,
            id: dom_id(node),
        })
    }

    /// As the wizard's list names it: "heading level 1 Weekend events", "MainRegion",
    /// "ComplementaryRegion Advertisement", "list" [obs, 705/s15, 720/s8, 716/s7].
    pub fn label(&self) -> String {
        if self.text.is_empty() {
            self.kind.name()
        } else {
            format!("{} {}", self.kind.name(), self.text)
        }
    }
}

/// The elements Flexible Web offers at node `id`: the node and the ones around it, innermost
/// first: on a heading, the heading and its main region [obs, 705/s15]; on a list in a
/// navigation region inside the banner, the list, the region and the banner [obs, 716/s7,
/// s8].
pub fn elements_at(page: &AomNode, id: usize) -> Vec<Element> {
    fn path<'a>(node: &'a AomNode, id: usize, out: &mut Vec<&'a AomNode>) -> bool {
        out.push(node);
        if node.id == id || node.children.iter().any(|c| path(c, id, out)) {
            return true;
        }
        out.pop();
        false
    }
    let mut nodes = Vec::new();
    if !path(page, id, &mut nodes) {
        return Vec::new();
    }
    nodes.iter().rev().filter_map(|n| Element::new(n)).collect()
}

/// Which of the elements of its kind a customization finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// The nth from the page's top, 1 the first.
    FromTop(usize),
    /// The nth from the page's bottom, 1 the last.
    FromBottom(usize),
    /// All of them.
    All,
}

/// A customization: what to do to which element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Customization {
    pub action: Action,
    pub kind: Kind,
    /// Only the elements with this text.
    pub text: Option<String>,
    /// Only the elements with this id.
    #[serde(default)]
    pub id: Option<String>,
    pub place: Place,
}

impl Customization {
    /// As the wizard and its first page name it: what it does and where, then what the element
    /// has: "Hide heading level 1 1 from the top with text: "Weekend events", and with id =
    /// "events"", "Hide all heading level 1s with id = "events"", "Start at any heading level
    /// 1" [obs, 708/s15-s17, 720/s12-s14, 732/s7-s12, 733/s9-s13, 734/s6-s11].
    pub fn label(&self) -> String {
        let kind = self.kind.name();
        let place = match (self.action, self.place) {
            (Action::Hide, Place::FromTop(n)) => msg::flexible_web_hide_from_top(kind, n),
            (Action::Hide, Place::FromBottom(n)) => msg::flexible_web_hide_from_bottom(kind, n),
            (Action::Hide, Place::All) => msg::flexible_web_hide_all(self.kind.plural()),
            (Action::StartAt, Place::FromTop(n)) => msg::flexible_web_start_from_top(kind, n),
            (Action::StartAt, Place::FromBottom(n)) => msg::flexible_web_start_from_bottom(kind, n),
            (Action::StartAt, Place::All) => msg::flexible_web_start_any(kind),
        }
        .to_string();
        let has = match (&self.text, &self.id) {
            (Some(text), Some(id)) => {
                Some(msg::flexible_web_with_text_and_id(text.clone(), id.clone()))
            }
            (Some(text), None) => Some(msg::flexible_web_with_text(text.clone())),
            (None, Some(id)) => Some(msg::flexible_web_with_id(id.clone())),
            (None, None) => None,
        };
        match has {
            Some(has) => format!("{place} {has}"),
            None => place,
        }
    }

    /// The page's elements this customization finds, in the page's order.
    pub fn find(&self, page: &AomNode) -> Vec<usize> {
        let mut found = Vec::new();
        collect(
            page,
            &self.kind,
            self.text.as_deref(),
            self.id.as_deref(),
            &mut found,
        );
        match self.place {
            Place::All => found,
            Place::FromTop(n) => found.get(n.wrapping_sub(1)).copied().into_iter().collect(),
            Place::FromBottom(n) => found
                .len()
                .checked_sub(n)
                .and_then(|i| found.get(i))
                .copied()
                .into_iter()
                .collect(),
        }
    }
}

/// The page's elements of `kind` (with `text`, with `id`), in its order.
fn collect(
    node: &AomNode,
    kind: &Kind,
    text: Option<&str>,
    id: Option<&str>,
    out: &mut Vec<usize>,
) {
    if kind.matches(node)
        && text.is_none_or(|t| text_for(kind, node) == t)
        && id.is_none_or(|i| dom_id(node).as_deref() == Some(i))
    {
        out.push(node.id);
    }
    for child in &node.children {
        collect(child, kind, text, id, out);
    }
}

/// The customizations the wizard offers for `element`, for hiding it or starting to read at
/// it alike [obs, 732-734]: what the element has first (the nth from the top with its text
/// and its id, all with its text, all with its id; with a text alone, the nth from the top
/// with it and all with it [obs, 733/s9-s10]); then the nth from the top, the nth from the
/// bottom and all [obs, 708/s15-s17].
pub fn customizations(page: &AomNode, element: &Element, action: Action) -> Vec<Customization> {
    let text = Some(element.text.clone()).filter(|t| !t.is_empty());
    let id = element.id.clone();
    let make = |text: Option<String>, id: Option<String>, place: Place| Customization {
        action,
        kind: element.kind.clone(),
        text,
        id,
        place,
    };
    // The element's place among those with `text` and `id`, and how many they are.
    let place_among = |text: Option<&str>, id: Option<&str>| {
        let mut found = Vec::new();
        collect(page, &element.kind, text, id, &mut found);
        found
            .iter()
            .position(|&n| n == element.node)
            .map(|i| (i, found.len()))
    };
    let Some((index, count)) = place_among(None, None) else {
        return Vec::new();
    };
    let mut offered = Vec::new();
    if text.is_some() || id.is_some() {
        if let Some((i, _)) = place_among(text.as_deref(), id.as_deref()) {
            offered.push(make(text.clone(), id.clone(), Place::FromTop(i + 1)));
        }
        if text.is_some() {
            offered.push(make(text.clone(), None, Place::All));
        }
        if id.is_some() {
            offered.push(make(None, id.clone(), Place::All));
        }
    }
    offered.push(make(None, None, Place::FromTop(index + 1)));
    offered.push(make(None, None, Place::FromBottom(count - index)));
    offered.push(make(None, None, Place::All));
    offered
}

/// `page` without the elements `hidden` names, and what is inside them.
pub fn without(page: &AomNode, hidden: &[usize]) -> AomNode {
    let mut kept = page.clone();
    fn prune(node: &mut AomNode, hidden: &[usize]) {
        node.children.retain(|c| !hidden.contains(&c.id));
        for child in &mut node.children {
            prune(child, hidden);
        }
    }
    prune(&mut kept, hidden);
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(id: usize, kind: &str, name: Option<&str>, children: Vec<AomNode>) -> AomNode {
        let mut node = AomNode::new(
            id,
            Role::Landmark {
                landmark_type: kind.to_owned(),
            },
        )
        .with_children(children);
        node.name = name.map(str::to_owned);
        node
    }

    /// The Flexible Web fixture's page: a banner holding the sections' navigation and its
    /// list, the advertisement, and the main region with its heading, whose id is "events".
    fn page() -> AomNode {
        let mut heading = AomNode::new(8, Role::Heading { level: 1 }).with_text("Weekend events");
        heading
            .attributes
            .insert("id".to_owned(), "events".to_owned());
        AomNode::new(0, Role::Document).with_children(vec![
            region(
                1,
                "banner",
                None,
                vec![region(
                    2,
                    "navigation",
                    Some("Sections"),
                    vec![AomNode::new(3, Role::List).with_children(vec![AomNode::new(
                        4,
                        Role::ListItem,
                    )
                    .with_text("News")])],
                )],
            ),
            region(
                5,
                "complementary",
                Some("Advertisement"),
                vec![AomNode::new(6, Role::Paragraph).with_text("Spring sale")],
            ),
            region(7, "main", None, vec![heading]),
        ])
    }

    #[test]
    fn the_elements_around_the_cursor_innermost_first() {
        let labels = |id| -> Vec<String> {
            elements_at(&page(), id)
                .iter()
                .map(Element::label)
                .collect()
        };
        // [obs, 705/s15]
        assert_eq!(labels(8), ["heading level 1 Weekend events", "MainRegion"]);
        // [obs, 716/s7, s8]: the list, its region, the banner.
        assert_eq!(
            labels(3),
            ["list", "NavigationRegion Sections", "BannerRegion"]
        );
        // [obs, 720/s8]: the paragraph is not offered.
        assert_eq!(labels(6), ["ComplementaryRegion Advertisement"]);
    }

    fn offered(page: &AomNode, id: usize, innermost: bool, action: Action) -> Vec<String> {
        let elements = elements_at(page, id);
        let element = if innermost {
            elements.first()
        } else {
            elements.last()
        };
        element
            .map(|e| customizations(page, e, action))
            .unwrap_or_default()
            .iter()
            .map(Customization::label)
            .collect()
    }

    #[test]
    fn the_customizations_offered_as_jaws_lists_them() {
        let page = page();
        // [obs, 708/s15-s17]
        assert_eq!(
            offered(&page, 8, false, Action::Hide),
            [
                "Hide MainRegion 1 from the top",
                "Hide MainRegion 1 from the bottom",
                "Hide all MainRegions"
            ]
        );
        // [obs, 733/s9-s13]
        assert_eq!(
            offered(&page, 6, true, Action::Hide),
            [
                "Hide ComplementaryRegion 1 from the top with text: \"Advertisement\"",
                "Hide all ComplementaryRegions with text: \"Advertisement\"",
                "Hide ComplementaryRegion 1 from the top",
                "Hide ComplementaryRegion 1 from the bottom",
                "Hide all ComplementaryRegions"
            ]
        );
        // [obs, 734/s6-s11]
        assert_eq!(
            offered(&page, 8, true, Action::Hide),
            [
                "Hide heading level 1 1 from the top with text: \"Weekend events\", and with id = \"events\"",
                "Hide all heading level 1s with text: \"Weekend events\"",
                "Hide all heading level 1s with id = \"events\"",
                "Hide heading level 1 1 from the top",
                "Hide heading level 1 1 from the bottom",
                "Hide all heading level 1s"
            ]
        );
        // [obs, 732/s7-s12]
        assert_eq!(
            offered(&page, 8, true, Action::StartAt),
            [
                "Start at heading level 1 1 from the top with text: \"Weekend events\", and with id = \"events\"",
                "Start at any heading level 1 with text: \"Weekend events\"",
                "Start at any heading level 1 with id = \"events\"",
                "Start at heading level 1 1 from the top",
                "Start at heading level 1 1 from the bottom",
                "Start at any heading level 1"
            ]
        );
    }

    #[test]
    fn a_customization_finds_its_element_and_hides_it() {
        let page = page();
        let advertisement = elements_at(&page, 6).remove(0);
        let first = customizations(&page, &advertisement, Action::Hide).remove(0);
        assert_eq!(first.find(&page), [5]);
        let hidden = without(&page, &first.find(&page));
        assert!(elements_at(&hidden, 6).is_empty());
        assert_eq!(elements_at(&hidden, 8).len(), 2);
        let heading = elements_at(&page, 8).remove(0);
        let by_id = customizations(&page, &heading, Action::Hide).remove(2);
        assert_eq!(by_id.find(&page), [8]);
    }
}
