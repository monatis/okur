//! Saved PlaceMarkers: places in a page the user named, to come back to with K, Shift+K and the
//! PlaceMarker List (Ctrl+Shift+K), from one visit to the next (JAWS 2025 help, Surf's Up
//! chapter 15). JAWS keeps them per page address; one generalized to the domain is offered on
//! every page of its site. A PlaceMarker returns to the exact character it was set at; one
//! anchored to text (Add's "Anchor to text" [obs, 701]) is found by that text where the
//! page's text moved.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::buffer::VirtualBuffer;

/// One saved PlaceMarker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceMarker {
    /// Its name, as the list shows it: the text at the cursor unless the user named it.
    pub name: String,
    /// The character it was set at, in the page's text.
    pub offset: usize,
    /// The text it is anchored to, found where the page's text moved; none for a PlaceMarker
    /// kept at its character.
    #[serde(default)]
    pub anchor: Option<String>,
    /// Generalized to the domain: offered on every page of the site.
    #[serde(default)]
    pub domain: bool,
}

/// Every page's PlaceMarkers, by page address, in the order they were added.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlaceMarkers {
    pub pages: BTreeMap<String, Vec<PlaceMarker>>,
}

/// Where PlaceMarkers are kept, so that they hold the next time Okur starts.
pub trait PlaceMarkerStore: Send {
    fn save(&self, markers: &PlaceMarkers);
}

/// The page a PlaceMarker belongs to: its address without its fragment.
pub fn page_of(url: &str) -> String {
    url.split('#').next().unwrap_or(url).to_owned()
}

impl PlaceMarkers {
    /// The markers offered on `url`'s page: its own, then those generalized to its site from
    /// its site's other pages.
    pub fn on_page(&self, url: &str) -> Vec<&PlaceMarker> {
        let page = page_of(url);
        let site = crate::settings_layers::site_of(url);
        let mut markers: Vec<&PlaceMarker> = self.pages.get(&page).into_iter().flatten().collect();
        if let Some(site) = site {
            markers.extend(
                self.pages
                    .iter()
                    .filter(|(other, _)| {
                        **other != page
                            && crate::settings_layers::site_of(other).as_ref() == Some(&site)
                    })
                    .flat_map(|(_, list)| list.iter().filter(|m| m.domain)),
            );
        }
        markers
    }

    /// Adds `marker` to `url`'s page.
    pub fn add(&mut self, url: &str, marker: PlaceMarker) {
        self.pages.entry(page_of(url)).or_default().push(marker);
    }

    /// Removes the page's marker `index` (of [`Self::on_page`]'s own, which come first).
    pub fn remove(&mut self, url: &str, index: usize) -> bool {
        let page = page_of(url);
        let Some(list) = self.pages.get_mut(&page) else {
            return false;
        };
        if index >= list.len() {
            return false;
        }
        list.remove(index);
        if list.is_empty() {
            self.pages.remove(&page);
        }
        true
    }

    /// Removes every marker of `url`'s page.
    pub fn remove_all(&mut self, url: &str) -> bool {
        self.pages.remove(&page_of(url)).is_some()
    }
}

/// Where `marker` is in `buffer`: where its anchor text is, nearest its character when the
/// page has it more than once; else at its character, within the page.
pub fn place_in(buffer: &VirtualBuffer, marker: &PlaceMarker) -> usize {
    let at = marker.offset.min(buffer.len().saturating_sub(1));
    let Some(anchor) = marker
        .anchor
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty())
    else {
        return at;
    };
    buffer
        .lines
        .iter()
        .filter_map(|line| {
            let byte = line.text.find(anchor)?;
            let chars = line.text.get(..byte)?.chars().count();
            Some(line.start_offset + chars)
        })
        .min_by_key(|found| found.abs_diff(marker.offset))
        .unwrap_or(at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::{AomNode, Role};

    fn marker(name: &str, offset: usize, domain: bool) -> PlaceMarker {
        PlaceMarker {
            name: name.to_owned(),
            offset,
            anchor: None,
            domain,
        }
    }

    #[test]
    fn a_page_offers_its_own_markers_then_its_sites_generalized_ones() {
        let mut markers = PlaceMarkers::default();
        markers.add("https://example.org/rules#food", marker("Food", 10, false));
        markers.add("https://example.org/hours", marker("Hours", 4, true));
        markers.add("https://example.org/news", marker("News", 2, false));
        markers.add("https://other.net/hours", marker("Other", 1, true));
        let names: Vec<&str> = markers
            .on_page("https://example.org/rules")
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        assert_eq!(names, ["Food", "Hours"]);
        // A file: page has no site: its own markers only.
        markers.add("file:///C:/pages/a.html", marker("Here", 0, true));
        assert_eq!(markers.on_page("file:///C:/pages/a.html").len(), 1);
        assert!(markers.on_page("file:///C:/pages/b.html").is_empty());
    }

    #[test]
    fn markers_are_removed_one_or_all() {
        let mut markers = PlaceMarkers::default();
        let url = "https://example.org/rules";
        markers.add(url, marker("One", 1, false));
        markers.add(url, marker("Two", 2, false));
        assert!(markers.remove(url, 0));
        assert!(!markers.remove(url, 5));
        let names: Vec<&str> = markers
            .on_page(url)
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        assert_eq!(names, ["Two"]);
        assert!(markers.remove_all(url));
        assert!(markers.pages.is_empty());
        assert!(!markers.remove_all(url));
    }

    #[test]
    fn an_anchored_marker_finds_its_text_where_the_text_moved() {
        let page = |first: &str| {
            AomNode::new(0, Role::Document).with_children(vec![
                AomNode::new(1, Role::Paragraph).with_text(first),
                AomNode::new(2, Role::Paragraph).with_text("Water in a closed bottle."),
            ])
        };
        let before = VirtualBuffer::from_aom(&page("Quiet."));
        let start = before
            .lines
            .iter()
            .find(|l| l.text.contains("Water"))
            .map_or(0, |l| l.start_offset);
        let mut saved = PlaceMarker {
            name: "Water".to_owned(),
            offset: start + 6,
            anchor: Some("closed bottle".to_owned()),
            domain: false,
        };
        assert_eq!(place_in(&before, &saved), start + 11);
        // A longer first paragraph moves the text: the anchored one finds it, the other
        // stays at its character.
        let after = VirtualBuffer::from_aom(&page("Quiet study is welcome on every floor."));
        let moved = after
            .lines
            .iter()
            .find(|l| l.text.contains("Water"))
            .map_or(0, |l| l.start_offset);
        assert_ne!(moved, start);
        assert_eq!(place_in(&after, &saved), moved + 11);
        saved.anchor = None;
        assert_eq!(place_in(&after, &saved), start + 6);
    }
}
