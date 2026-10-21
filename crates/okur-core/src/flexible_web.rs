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
}

impl Element {
    fn new(node: &AomNode) -> Option<Self> {
        let kind = Kind::of(node)?;
        let text = text_for(&kind, node);
        Some(Self {
            node: node.id,
            kind,
            text,
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
    pub place: Place,
}

impl Customization {
    /// As the wizard and its first page name it: "Hide MainRegion 1 from the top", "Hide all
    /// ComplementaryRegions with text: "Advertisement"" [obs, 708/s15-s17, 720/s12-s14].
    pub fn label(&self) -> String {
        let kind = self.kind.name();
        let text = self.text.clone().unwrap_or_default();
        let hide = self.action == Action::Hide;
        match (self.place, self.text.is_some(), hide) {
            (Place::FromTop(n), false, true) => msg::flexible_web_hide_from_top(kind, n),
            (Place::FromTop(n), true, true) => {
                msg::flexible_web_hide_from_top_with_text(kind, n, text)
            }
            (Place::FromBottom(n), _, true) => msg::flexible_web_hide_from_bottom(kind, n),
            (Place::All, false, true) => msg::flexible_web_hide_all(self.kind.plural()),
            (Place::All, true, true) => {
                msg::flexible_web_hide_all_with_text(self.kind.plural(), text)
            }
            (Place::FromTop(n), false, false) => msg::flexible_web_start_from_top(kind, n),
            (Place::FromTop(n), true, false) => {
                msg::flexible_web_start_from_top_with_text(kind, n, text)
            }
            (Place::FromBottom(n), _, false) => msg::flexible_web_start_from_bottom(kind, n),
            (Place::All, _, false) => msg::flexible_web_start_from_top(kind, 1),
        }
        .to_string()
    }

    /// The page's elements this customization finds, in the page's order.
    pub fn find(&self, page: &AomNode) -> Vec<usize> {
        let mut found = Vec::new();
        collect(page, &self.kind, self.text.as_deref(), &mut found);
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

/// The page's elements of `kind` (with `text`), in its order.
fn collect(node: &AomNode, kind: &Kind, text: Option<&str>, out: &mut Vec<usize>) {
    if kind.matches(node) && text.is_none_or(|t| text_for(kind, node) == t) {
        out.push(node.id);
    }
    for child in &node.children {
        collect(child, kind, text, out);
    }
}

/// The customizations the wizard offers for `element`: with a text, the nth from the top with
/// that text, all with that text, the nth from the top, the nth from the bottom and all; with
/// none, the last three [obs, 720/s12-s14; 708/s15-s17; the guide's frames]. Reading starts at
/// one element, so its "all" is not offered.
pub fn customizations(page: &AomNode, element: &Element, action: Action) -> Vec<Customization> {
    let mut all = Vec::new();
    collect(page, &element.kind, None, &mut all);
    let mut with_text = Vec::new();
    collect(page, &element.kind, Some(&element.text), &mut with_text);
    let place_in = |list: &[usize]| list.iter().position(|&id| id == element.node);
    let Some(index) = place_in(&all) else {
        return Vec::new();
    };
    let make = |text: Option<String>, place: Place| Customization {
        action,
        kind: element.kind.clone(),
        text,
        place,
    };
    let mut offered = Vec::new();
    if !element.text.is_empty() {
        if let Some(i) = place_in(&with_text) {
            offered.push(make(Some(element.text.clone()), Place::FromTop(i + 1)));
            if action == Action::Hide {
                offered.push(make(Some(element.text.clone()), Place::All));
            }
        }
    }
    offered.push(make(None, Place::FromTop(index + 1)));
    offered.push(make(None, Place::FromBottom(all.len() - index)));
    if action == Action::Hide {
        offered.push(make(None, Place::All));
    }
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
    /// list, the advertisement, and the main region with its heading.
    fn page() -> AomNode {
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
            region(
                7,
                "main",
                None,
                vec![AomNode::new(8, Role::Heading { level: 1 }).with_text("Weekend events")],
            ),
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

    #[test]
    fn the_customizations_offered_and_what_they_find() {
        let page = page();
        let labels = |id: usize| -> Vec<String> {
            let element = elements_at(&page, id).into_iter().last();
            element
                .map(|e| customizations(&page, &e, Action::Hide))
                .unwrap_or_default()
                .iter()
                .map(Customization::label)
                .collect()
        };
        // [obs, 708/s15-s17]
        assert_eq!(
            labels(8),
            [
                "Hide MainRegion 1 from the top",
                "Hide MainRegion 1 from the bottom",
                "Hide all MainRegions"
            ]
        );
        // [obs, 720/s12-s14], and the two after them as the guide's frames have them.
        assert_eq!(
            labels(6),
            [
                "Hide ComplementaryRegion 1 from the top with text: \"Advertisement\"",
                "Hide all ComplementaryRegions with text: \"Advertisement\"",
                "Hide ComplementaryRegion 1 from the top",
                "Hide ComplementaryRegion 1 from the bottom",
                "Hide all ComplementaryRegions"
            ]
        );
        let advertisement = elements_at(&page, 6).remove(0);
        let first = customizations(&page, &advertisement, Action::Hide).remove(0);
        assert_eq!(first.find(&page), [5]);
        let hidden = without(&page, &first.find(&page));
        assert!(elements_at(&hidden, 6).is_empty());
        assert_eq!(elements_at(&hidden, 8).len(), 2);
        // Reading starts at one element: no "all" [the guide: "Start at heading level 2 1
        // from the top"].
        let heading = elements_at(&page, 8).remove(0);
        let start: Vec<String> = customizations(&page, &heading, Action::StartAt)
            .iter()
            .map(Customization::label)
            .collect();
        assert_eq!(
            start.first().map(String::as_str),
            Some("Start at heading level 1 1 from the top with text: \"Weekend events\"")
        );
        assert!(start.iter().all(|s| !s.contains("all")));
    }
}
