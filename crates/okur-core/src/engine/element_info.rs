//! Element Information (Insert+Shift+F1, JAWS's DisplayBasicElementInfo) and Advanced Element
//! Information (Insert+Ctrl+Shift+F1, DisplayAdvancedElementInfo) in a page: the element at the
//! virtual cursor and each element above it to the document, read from the platform, in a
//! message of the Virtual Viewer: "Element Information: | Tag A has 5 parameters: |
//! display=inline | href=... | MSAA Role=1E | Tag HEADER has 5 parameters: | ... | Press ESC to
//! close this message." [obs, 1210/s2, s11].

use super::{AppMode, ScreenReaderEngine};
use crate::event::ElementDetails;
use crate::messages::msg;

impl ScreenReaderEngine {
    /// Insert+Shift+F1 (`advanced` false) or Insert+Ctrl+Shift+F1: asks the platform for the
    /// element at the virtual cursor and those above it; the message opens when they come.
    /// Nothing is said outside a page, or where the platform cannot read them.
    pub(super) fn ask_element_information(&mut self, advanced: bool) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let id = self.element_at_cursor()?;
        let asked = self
            .actions
            .as_ref()
            .is_some_and(|actions| actions.read_element_information(&id));
        if asked {
            self.element_information_asked = Some(advanced);
        }
        None
    }

    /// The platform's id of the element at the virtual cursor: its node's, or the nearest
    /// node's above it that has one (a link's text is the link's).
    fn element_at_cursor(&self) -> Option<String> {
        let buffer = &self.cursor.buffer;
        let id = self.cursor.current_node_id?;
        let own = buffer
            .node(id)
            .and_then(|node| node.attributes.get(crate::aom::UNIQUE_ID_ATTR).cloned());
        own.or_else(|| {
            buffer
                .ancestors(id)
                .into_iter()
                .find_map(|node| node.attributes.get(crate::aom::UNIQUE_ID_ATTR).cloned())
        })
    }

    /// The elements read for Element Information: its message opens, read whole, as JAWS reads
    /// it [obs, 1210/s2]; the keys then move through its lines [obs, 1210/s3-s9]. Nothing when
    /// none was asked for, or none was read.
    pub(super) fn element_information(&mut self, elements: &[ElementDetails]) -> Option<String> {
        let advanced = self.element_information_asked.take()?;
        if elements.is_empty() {
            return None;
        }
        let viewer = crate::viewer::VirtualViewer::message(element_lines(advanced, elements));
        let speech = viewer.message_speech();
        self.viewer = Some(viewer);
        self.show_viewer_window();
        self.speech.speak(&speech, true);
        Some(speech)
    }
}

/// The message's lines: its title and a blank line, then each element's tag with its
/// parameters' count, its parameters by name and its role's number in hexadecimal, a blank
/// line after each, then how to close it [obs, 1213/s3-s28].
pub(crate) fn element_lines(advanced: bool, elements: &[ElementDetails]) -> Vec<String> {
    let title = if advanced {
        msg::element_information_advanced_title()
    } else {
        msg::element_information_title()
    };
    let mut lines = vec![title.to_string(), String::new()];
    for element in elements {
        lines.push(
            msg::element_information_tag(
                element.tag.to_uppercase(),
                element.parameters.len().to_string(),
            )
            .to_string(),
        );
        lines.extend(
            element
                .parameters
                .iter()
                .map(|(name, value)| format!("{name}={value}")),
        );
        if let Some(role) = element.role {
            lines.push(msg::element_information_role(format!("{role:X}")).to_string());
        }
        lines.push(String::new());
    }
    lines.push(msg::element_information_close().to_string());
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JAWS's message for a link in a page's banner [obs, 1210/s2]: the link, its header and
    /// the document, each with its parameters and its role's number.
    #[test]
    fn the_message_is_jaws_element_information() {
        let element = |tag: &str, parameters: &[(&str, &str)], role: u32| ElementDetails {
            tag: tag.to_owned(),
            parameters: parameters
                .iter()
                .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
                .collect(),
            role: Some(role),
        };
        let elements = [
            element(
                "a",
                &[
                    ("display", "inline"),
                    ("href", "file:///news-article.html#top"),
                    ("name-from", "contents"),
                    ("tag", "a"),
                    ("text-align", "left"),
                ],
                0x1e,
            ),
            element("#document", &[("tag", "#document")], 0xf),
        ];
        let lines = element_lines(false, &elements);
        assert_eq!(
            lines,
            [
                "Element Information:",
                "",
                "Tag A has 5 parameters:",
                "display=inline",
                "href=file:///news-article.html#top",
                "name-from=contents",
                "tag=a",
                "text-align=left",
                "MSAA Role=1E",
                "",
                "Tag #DOCUMENT has 1 parameters:",
                "tag=#document",
                "MSAA Role=F",
                "",
                "Press ESC to close this message.",
            ]
        );
        assert_eq!(
            element_lines(true, &elements).first().map(String::as_str),
            Some("Advanced Element Information:")
        );
    }
}
