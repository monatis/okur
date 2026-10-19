//! The page keys that tie the virtual cursor and the focus together, and tell about the page:
//! the link's address (Ctrl+Insert+F7), the virtual cursor routed to the focus (Insert+Delete)
//! and the focus to the virtual cursor (Ctrl+Insert+Delete), the page's frames (Insert+F9),
//! and the left mouse button (Numpad Slash) (Default.JKM: IndicateLinkSourceURL,
//! RouteVirtualToPc, RoutePCToVirtual, SelectAFrame, LeftMouseButton).

use super::{AppMode, ScreenReaderEngine};
use crate::aom::Role;
use crate::messages::msg;

impl ScreenReaderEngine {
    /// The address of the link at the virtual cursor, as the page gives it:
    /// "file:///C:/…/news-article.html#top" [obs, 536/s2]. Nothing elsewhere.
    pub(super) fn say_link_address(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let address = match &self.node_under_cursor()?.role {
            Role::Link { href } => href.clone()?,
            _ => return None,
        };
        self.say(address)
    }

    /// The virtual cursor goes to the focused element and reads its line: "Route Virtual to
    /// PC", "same page link Riverside Gazette" [obs, 536/s6].
    pub(super) fn route_virtual_to_pc(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        if let Some(id) = self.last_page_focus {
            self.cursor.move_to_node_id(id);
        }
        let said = [
            msg::route_virtual_to_pc().to_string(),
            self.cursor.current_line(),
        ];
        for (i, part) in said.iter().enumerate() {
            self.speech.speak(part, i == 0);
        }
        Some(said.join("\n"))
    }

    /// The focus goes to the element at the virtual cursor, which the application then reports
    /// focused: "Route PC to Virtual Cursor", then the element [obs, 536/s10].
    pub(super) fn route_pc_to_virtual(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        if let (Some(actions), Some(node)) = (&self.actions, self.node_under_cursor()) {
            actions.focus(node);
        }
        self.say(msg::route_pc_to_virtual().to_string())
    }

    /// Insert+F9: on a page without frames, "No frames found in page" [obs, 536/s12]. A page
    /// with frames waits for JAWS's Frames list.
    pub(super) fn select_a_frame(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let frames = self
            .cursor
            .buffer
            .nodes()
            .any(|node| node.role == Role::IFrame);
        if frames {
            return None;
        }
        self.say(msg::no_frames_in_page().to_string())
    }

    /// Numpad Slash in a page: "left mouse button", and the element at the virtual cursor is
    /// clicked; the page then reports what the click did, the check box focused and unchecked,
    /// then checked [obs, 534/s2, s4, s9]. Elsewhere JAWS clicks at the mouse pointer, which
    /// Okur does not move yet.
    pub(super) fn left_mouse_click(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer || self.forms_mode {
            return None;
        }
        let said = self.say(msg::left_mouse_button().to_string());
        if let (Some(actions), Some(node)) = (&self.actions, self.node_under_cursor()) {
            actions.activate(node);
        }
        said
    }
}
