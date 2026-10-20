//! Google Docs' document editor (docs.google.com/document). Docs draws the document on a
//! canvas: its text is not in the page's accessibility tree, and what the caret passes over
//! reaches screen readers as Docs' own announcements (a live region). Keys go to an empty
//! element Docs focuses for typing, so a font cannot be taken from the focus's text, and
//! JAWS's Say Font says "No font" there (JAWS 2025 on the dev box, 2026-10-16).
//!
//! The toolbar shows the formatting at the caret: the Bold, Italic and Underline buttons are
//! pressed over such text, the size field holds the size, and the font list has the font
//! selected. Say Font reads them, by their DOM ids, as JAWS reads a font in Notepad.

use crate::aom::{AomNode, Role, TextFormatting};
use crate::apps::AppModule;

/// Docs' host.
const HOST: &str = "docs.google.com";
/// The element Docs focuses for typing into the document.
const TEXT_TARGET: &str = "docs-texteventtarget-descendant";
const FONT_LIST: &str = "docs-font-family";
const SIZE_LIST: &str = "fontSizeSelect";
const BOLD: &str = "boldButton";
const ITALIC: &str = "italicButton";
const UNDERLINE: &str = "underlineButton";
/// The toolbar's formatting controls, by DOM id.
const FORMATTING: &[&str] = &[FONT_LIST, SIZE_LIST, BOLD, ITALIC, UNDERLINE];

/// Whether `url` is a Google Docs document.
fn is_document(url: &str) -> bool {
    let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return false;
    };
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    host.eq_ignore_ascii_case(HOST) && path.starts_with("document/")
}

/// The first node in `node`'s subtree, itself included, that `matches`.
fn find<'a>(node: &'a AomNode, matches: &dyn Fn(&AomNode) -> bool) -> Option<&'a AomNode> {
    if matches(node) {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, matches))
}

/// The size in the size list's field ("11").
fn size_text(size_list: &AomNode) -> Option<&str> {
    find(size_list, &|n| n.role == Role::Edit && n.value.is_some())
        .and_then(|field| field.value.as_deref())
        .map(str::trim)
        .filter(|size| !size.is_empty())
}

/// The font selected in the font list. Docs names a list's selection only in a sentence made
/// of the list's name and the selection ("Font list. Arial selected."), in the document's
/// language. The size list says its size the same way ("Font size list. 11 selected."), and
/// its field holds the size, which gives the sentence's words around the selection.
fn font_family(font_list: &AomNode, size_list: &AomNode) -> Option<String> {
    let size = size_text(size_list)?;
    let size_sentence = find(size_list, &|n| {
        n.role != Role::Edit && n.name.as_deref().is_some_and(|name| name.contains(size))
    })?
    .name
    .as_deref()?;
    let (before, after) = size_sentence.split_once(size)?;
    let joint = before.strip_prefix(size_list.name.as_deref()?)?;
    let selected = find(font_list, &|n| n.states.selected == Some(true))?
        .name
        .as_deref()?;
    let family = selected
        .strip_prefix(font_list.name.as_deref()?)?
        .strip_prefix(joint)?
        .strip_suffix(after)?
        .trim();
    (!family.is_empty()).then(|| family.to_owned())
}

/// Google Docs.
pub struct GoogleDocs;

impl AppModule for GoogleDocs {
    fn owns_page(&self, url: &str) -> bool {
        is_document(url)
    }

    fn font_elements(&self, focus_id: &str) -> &'static [&'static str] {
        if focus_id == TEXT_TARGET {
            FORMATTING
        } else {
            &[]
        }
    }

    fn font_from_elements(&self, elements: &[AomNode]) -> Option<TextFormatting> {
        let element = |id: &str| {
            elements
                .iter()
                .find(|e| e.attributes.get("id").is_some_and(|i| i == id))
        };
        let pressed = |id: &str| element(id).is_some_and(|e| e.states.checked == Some(true));
        let size_list = element(SIZE_LIST);
        let formatting = TextFormatting {
            font_family: element(FONT_LIST)
                .zip(size_list)
                .and_then(|(font, size)| font_family(font, size)),
            font_size_pt: size_list
                .and_then(size_text)
                .and_then(|size| size.parse::<f32>().ok()),
            bold: pressed(BOLD),
            italic: pressed(ITALIC),
            underline: pressed(UNDERLINE),
            ..TextFormatting::default()
        };
        (formatting != TextFormatting::default()).then_some(formatting)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::NodeStates;

    fn with_id(mut node: AomNode, id: &str) -> AomNode {
        node.attributes.insert("id".to_owned(), id.to_owned());
        node
    }

    fn toggle(id: &str, name: &str, pressed: bool) -> AomNode {
        let states = NodeStates {
            checked: Some(pressed),
            ..NodeStates::default()
        };
        with_id(AomNode::new(0, Role::Button).with_name(name), id).with_states(states)
    }

    /// A toolbar list as Docs builds it: the list, a group, and the selection's sentence.
    fn font_list(name: &str, sentence: &str) -> AomNode {
        let states = NodeStates {
            selected: Some(true),
            ..NodeStates::default()
        };
        let option = AomNode::new(2, Role::ListItem)
            .with_name(sentence)
            .with_states(states);
        with_id(AomNode::new(0, Role::List).with_name(name), FONT_LIST).with_children(vec![
            AomNode::new(1, Role::Group).with_children(vec![option]),
        ])
    }

    fn size_list(name: &str, sentence: &str, size: &str) -> AomNode {
        let field = AomNode::new(3, Role::Edit).with_name(name).with_value(size);
        let caption = AomNode::new(2, Role::Group)
            .with_name(sentence)
            .with_children(vec![field]);
        with_id(AomNode::new(0, Role::ComboBox).with_name(name), SIZE_LIST).with_children(vec![
            AomNode::new(1, Role::Group).with_children(vec![caption]),
        ])
    }

    /// The toolbar with the caret in a bold word, as Chrome's UIA showed it on the dev box
    /// (2026-10-16).
    fn toolbar(bold: bool) -> Vec<AomNode> {
        vec![
            font_list("Font", "Font list. Arial selected."),
            size_list("Font size", "Font size list. 11 selected.", "11"),
            toggle(BOLD, "Bold (Ctrl+B)", bold),
            toggle(ITALIC, "Italic (Ctrl+I)", false),
            toggle(UNDERLINE, "Underline (Ctrl+U)", false),
        ]
    }

    #[test]
    fn documents_are_known_by_their_address() {
        for url in [
            "https://docs.google.com/document/d/1XYG/edit?tab=t.0",
            "https://docs.google.com/document/u/1/d/1XYG/edit",
        ] {
            assert!(is_document(url), "{url}");
        }
        for url in [
            "https://docs.google.com/spreadsheets/d/1XYG/edit",
            "https://docs.google.com.example.org/document/d/1XYG",
            "https://example.org/docs.google.com/document/",
            "docs.google.com/document/d/1XYG",
        ] {
            assert!(!is_document(url), "{url}");
        }
    }

    #[test]
    fn the_toolbar_gives_the_font_at_the_caret() {
        let docs = GoogleDocs;
        assert_eq!(
            docs.font_from_elements(&toolbar(true)),
            Some(
                TextFormatting::new()
                    .with_family("Arial")
                    .with_size(11.0)
                    .with_bold(true)
            )
        );
        assert_eq!(
            docs.font_from_elements(&toolbar(false))
                .and_then(|f| f.font_family),
            Some("Arial".to_owned())
        );
    }

    /// The font list's sentence is read with the words the size list's sentence puts around
    /// its size, whatever they are (this wording is made up, not one of Docs' languages).
    #[test]
    fn the_font_is_found_in_any_wording_of_the_lists() {
        let elements = vec![
            font_list("Typeface", "Typeface choices: Lato (current)"),
            size_list(
                "Typeface size",
                "Typeface size choices: 10.5 (current)",
                "10.5",
            ),
        ];
        let formatting = GoogleDocs.font_from_elements(&elements);
        assert_eq!(
            formatting.as_ref().and_then(|f| f.font_family.as_deref()),
            Some("Lato")
        );
        assert_eq!(formatting.and_then(|f| f.font_size_pt), Some(10.5));
    }

    /// A font list that does not follow the size list's sentence gives no font, rather than a
    /// wrong one.
    #[test]
    fn an_unknown_sentence_gives_no_font() {
        let elements = vec![
            font_list("Font", "Arial"),
            size_list("Font size", "Font size list. 11 selected.", "11"),
        ];
        let formatting = GoogleDocs.font_from_elements(&elements);
        assert_eq!(
            formatting.as_ref().and_then(|f| f.font_family.clone()),
            None
        );
        assert_eq!(formatting.and_then(|f| f.font_size_pt), Some(11.0));
    }

    #[test]
    fn only_the_documents_text_reads_the_toolbar() {
        let docs = GoogleDocs;
        assert_eq!(docs.font_elements(TEXT_TARGET), FORMATTING);
        assert!(docs.font_elements("fontSizeSelect").is_empty());
        assert_eq!(docs.font_from_elements(&[]), None);
    }
}
