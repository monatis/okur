//! JAWS's element lists ("Select a Button", "Select a List", ...): which of a page's elements
//! each one holds and how it shows them, as JAWS 2025 does on old-pc [obs, 231-245]. Every
//! list is a dialog of the same shape (list "List1", OK, Cancel); this module gives each its
//! title and its items, in page order.
//!
//! An item is the element as JAWS lists it:
//! - a control: its name, its type, then its value or state ("Email address Edit
//!   sam@example.com", "Save draft Button Unavailable"); a radio button after its group
//!   ("Membership type Adult radio button checked"), a tab after its tab list ("Settings General
//!   Tab");
//! - a block (a list, an article, a paragraph, a block quote, a division): its text, the lines
//!   of one paragraph joined by spaces and the paragraphs run together ("• News• Sports"),
//!   without the structure's own lines ("list of 4 items");
//! - a graphic: its text; a table: its caption and size ("Branch hours after the change 3x3").
//!
//! JAWS cuts an item at 254 characters [obs, 238].

use crate::aom::{AomNode, Role};
use crate::buffer::VirtualBuffer;
use crate::messages::{msg, StaticMessage};

/// The longest item JAWS shows [obs, 238/s1: the article's text ends after 254 characters].
const ITEM_LENGTH: usize = 254;

/// One of JAWS's element lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementList {
    RadioButtons,
    Buttons,
    /// Combo boxes, list boxes and trees: "Selectable ARIA controls, comboboxes, listboxes or
    /// treeviews".
    Controls,
    Edits,
    CheckBoxes,
    Graphics,
    Lists,
    Articles,
    Paragraphs,
    BlockQuotes,
    Tables,
    Divisions,
    Tabs,
}

/// One element as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementItem {
    pub text: String,
    pub node_id: usize,
    /// Where the element starts in the buffer.
    pub offset: usize,
}

impl ElementList {
    /// The dialog's title, as JAWS's.
    pub fn title(self) -> StaticMessage {
        match self {
            Self::RadioButtons => msg::list_dialog_radio_buttons(),
            Self::Buttons => msg::list_dialog_buttons(),
            Self::Controls => msg::list_dialog_controls(),
            Self::Edits => msg::list_dialog_edits(),
            Self::CheckBoxes => msg::list_dialog_check_boxes(),
            Self::Graphics => msg::list_dialog_graphics(),
            Self::Lists => msg::list_dialog_lists(),
            Self::Articles => msg::list_dialog_articles(),
            Self::Paragraphs => msg::list_dialog_paragraphs(),
            Self::BlockQuotes => msg::list_dialog_block_quotes(),
            Self::Tables => msg::list_dialog_tables(),
            Self::Divisions => msg::list_dialog_divisions(),
            Self::Tabs => msg::list_dialog_tabs(),
        }
    }

    /// What JAWS says when the page has none [obs, 261-273].
    pub fn none(self) -> StaticMessage {
        match self {
            Self::RadioButtons => msg::list_none_radio_buttons(),
            Self::Buttons => msg::list_none_buttons(),
            Self::Controls => msg::list_none_controls(),
            Self::Edits => msg::list_none_edits(),
            Self::CheckBoxes => msg::list_none_check_boxes(),
            Self::Graphics => msg::list_none_graphics(),
            Self::Lists => msg::list_none_lists(),
            Self::Articles => msg::list_none_articles(),
            Self::Paragraphs => msg::list_none_paragraphs(),
            Self::BlockQuotes => msg::list_none_block_quotes(),
            Self::Tables => msg::list_none_tables(),
            Self::Divisions => msg::list_none_divisions(),
            Self::Tabs => msg::list_none_tabs(),
        }
    }

    /// Whether OK gives the element the focus, as JAWS does for form controls (an edit then
    /// takes typing); the other lists move the virtual cursor to the element [obs, 246-260].
    pub fn focuses(self) -> bool {
        matches!(
            self,
            Self::RadioButtons | Self::Buttons | Self::Edits | Self::CheckBoxes | Self::Tabs
        )
    }

    /// Whether `node` is one of the list's elements.
    fn holds(self, node: &AomNode) -> bool {
        match self {
            Self::RadioButtons => node.role == Role::RadioButton,
            Self::Buttons => matches!(node.role, Role::Button | Role::MenuButton),
            Self::Controls => matches!(node.role, Role::ComboBox | Role::Tree),
            Self::Edits => node.role == Role::Edit,
            Self::CheckBoxes => node.role == Role::CheckBox,
            Self::Graphics => node.role == Role::Image,
            Self::Lists => node.role == Role::List,
            Self::Articles => {
                node.role == Role::Group
                    && node
                        .attributes
                        .get("aria_role")
                        .is_some_and(|r| r == "article")
            }
            // Paragraphs and divisions are blocks like any other in the page's tree; the HTML
            // tag IA2 reports tells them apart.
            Self::Paragraphs => node.role == Role::Paragraph || tag(node) == Some("p"),
            Self::BlockQuotes => node.role == Role::BlockQuote,
            Self::Tables => node.role == Role::Table,
            Self::Divisions => tag(node) == Some("div"),
            Self::Tabs => node.role == Role::Tab,
        }
    }

    /// The page's elements of the list, in page order. An element the buffer shows nothing of
    /// is left out.
    pub fn items(self, buffer: &VirtualBuffer) -> Vec<ElementItem> {
        buffer
            .page_nodes()
            .filter(|node| self.holds(node))
            .filter_map(|node| {
                let offset = buffer.start_of_subtree(node)?;
                // A block without text is listed by its name: JAWS counts the chat's message
                // box, a division with only a line break in it [obs, 244/s1: 17 divisions];
                // that it shows the name is inferred.
                let text = Some(self.text(buffer, node))
                    .filter(|t| !t.trim().is_empty())
                    .or_else(|| node.name.clone())
                    .unwrap_or_default();
                (!text.trim().is_empty()).then(|| ElementItem {
                    text: text.chars().take(ITEM_LENGTH).collect(),
                    node_id: node.id,
                    offset,
                })
            })
            .collect()
    }

    fn text(self, buffer: &VirtualBuffer, node: &AomNode) -> String {
        let name = node
            .name
            .as_deref()
            .or(node.text.as_deref())
            .unwrap_or_default()
            .trim();
        let checked = || {
            if node.states.checked == Some(true) {
                msg::state_checked()
            } else {
                msg::state_not_checked()
            }
        };
        let join = |parts: &[&str]| {
            parts
                .iter()
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        };
        // The nearest group around the element with a name: a radio button's fieldset, a
        // tab's tab list.
        let around = |roles: &[Role]| {
            buffer
                .ancestors(node.id)
                .into_iter()
                .find(|a| roles.contains(&a.role))
                .and_then(|a| a.name.clone())
                .unwrap_or_default()
        };
        match self {
            Self::RadioButtons => join(&[
                &around(&[Role::Group]),
                name,
                msg::role_radio_button().as_str(),
                checked().as_str(),
            ]),
            Self::Buttons => {
                let unavailable = node.states.disabled.then(msg::state_unavailable);
                join(&[
                    name,
                    msg::role_button().as_str(),
                    unavailable.map_or("", |s| s.as_str()),
                ])
            }
            Self::Controls | Self::Graphics => name.to_owned(),
            Self::Edits => join(&[
                name,
                msg::role_edit().as_str(),
                node.value.as_deref().unwrap_or_default(),
            ]),
            Self::CheckBoxes => join(&[name, msg::role_check_box().as_str(), checked().as_str()]),
            Self::Tabs => join(&[&around(&[Role::TabList]), name, msg::role_tab().as_str()]),
            Self::Tables => {
                let grid = buffer.tables.iter().find(|t| t.table_id == node.id);
                let (columns, rows) = grid.map_or((0, 0), |g| (g.cols, g.rows));
                let caption = grid.and_then(|g| g.caption.clone()).unwrap_or_default();
                msg::element_list_table(caption.trim(), columns, rows).to_string()
            }
            Self::Lists
            | Self::Articles
            | Self::Paragraphs
            | Self::BlockQuotes
            | Self::Divisions => block_text(node),
        }
    }
}

/// The element's HTML tag, as IA2 reports it.
fn tag(node: &AomNode) -> Option<&str> {
    node.attributes.get("tag").map(String::as_str)
}

/// A block's text as JAWS lists it: its text pieces as the page has them, run together. Inline
/// pieces keep their own spaces ("• New study rooms open in May."), blocks get none between
/// them ("9 p.m.• Weekend hours", "You:Good morning"), and an element without text gives
/// its name (an image's text, an empty span's label) [obs, 237, 238, 244].
fn block_text(node: &AomNode) -> String {
    fn pieces(node: &AomNode, out: &mut String) {
        if node.states.hidden {
            return;
        }
        if node.children.is_empty() {
            let piece = node
                .text
                .as_deref()
                .filter(|t| !t.is_empty())
                .or(node.name.as_deref())
                .unwrap_or_default();
            out.push_str(piece);
            return;
        }
        for child in &node.children {
            pieces(child, out);
        }
    }
    let mut text = String::new();
    pieces(node, &mut text);
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
