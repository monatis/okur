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

    /// Backslash and Shift+Backslash: the next or prior link that writes an e-mail. JAWS gives
    /// it the focus, and the focus is all it says ("... | help@example.org Send Mail Link | To
    /// activate press Enter."); on the link that has the focus already it says the link
    /// ("help@example.org Send Mail Link") [obs, 293/s1-s3].
    pub(super) fn move_to_mail_link(&mut self, forward: bool) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let said = self.cursor.move_mail_link(forward)?;
        let to_focus = self
            .node_under_cursor()
            .filter(|node| matches!(node.role, Role::Link { .. }))
            .filter(|node| Some(node.id) != self.last_page_focus);
        if let (Some(node), Some(actions)) = (to_focus, &self.actions) {
            actions.focus(node);
            return Some(String::new());
        }
        self.say(said)
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
        self.routed = true;
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

    /// The radio button at the virtual cursor, when checking it unchecks another of its group:
    /// the page then reports both changes, and JAWS says the radio button checked without its
    /// position [obs, 205/s6, 534/s9]; checking one in a group with none checked, it keeps the
    /// position [obs, 229/s10].
    pub(super) fn radio_switching_here(&self) -> Option<usize> {
        let node = self.node_under_cursor()?;
        if node.role != crate::aom::Role::RadioButton || node.states.checked == Some(true) {
            return None;
        }
        let group = node.attributes.get("group_id")?;
        let other_checked = self.cursor.buffer.nodes().any(|other| {
            other.id != node.id
                && other.role == crate::aom::Role::RadioButton
                && other.attributes.get("group_id") == Some(group)
                && other.states.checked == Some(true)
        });
        other_checked.then_some(node.id)
    }

    /// Numpad Slash in a page: "left mouse button", and the element at the virtual cursor is
    /// clicked; the page then reports what the click did, the check box focused and unchecked,
    /// then checked [obs, 534/s2, s4, s9]. Elsewhere JAWS clicks at the mouse pointer, which
    /// Okur does not move yet.
    pub(super) fn left_mouse_click(&mut self) -> Option<String> {
        // Outside a page, at the mouse: "left mouse button", the caret going where it clicks
        // [obs, 756/s6, s9].
        if self.app_mode != AppMode::VirtualBuffer {
            let said = self.say(msg::left_mouse_button().to_string());
            self.click_at_mouse(crate::actions::MouseClick::LeftSingle);
            return said;
        }
        if self.forms_mode {
            return None;
        }
        let said = self.say(msg::left_mouse_button().to_string());
        self.clicked = self.node_under_cursor().map(|node| node.id);
        self.radio_switched = self.radio_switching_here();
        if let (Some(actions), Some(node)) = (&self.actions, self.node_under_cursor()) {
            actions.activate(node);
        }
        said
    }

    /// Numpad Star: "right mouse button", clicked at the mouse; the application's context
    /// menu opens there [obs, 756/s15].
    pub(super) fn right_mouse_click(&mut self) -> Option<String> {
        let said = self.say(msg::right_mouse_button().to_string());
        self.click_at_mouse(crate::actions::MouseClick::RightSingle);
        said
    }

    /// A click where the mouse is, or at the PC cursor when Okur has not placed it.
    fn click_at_mouse(&self, click: crate::actions::MouseClick) {
        if let (Some((x, y)), Some(actions)) =
            (self.mouse.or(self.pc_cursor_point()), &self.actions)
        {
            actions.click_mouse(x, y, click);
        }
    }
}
