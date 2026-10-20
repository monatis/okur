//! The cursors (JAWS 2025 help, Keystrokes for the Cursors): the PC cursor follows the focus,
//! the JAWS cursor is the mouse pointer. Numpad Minus makes the JAWS cursor the active one and
//! Numpad Plus the PC cursor; Insert+Numpad Minus routes the JAWS cursor to the PC cursor,
//! moving the mouse there; Alt+Delete says which cursor is active and where it is [obs, 516,
//! 513/s2].
//!
//! The JAWS cursor reads nothing yet: Okur has no model of the screen's text. In the dialog
//! JAWS was recorded in, it read nothing at the mouse either [obs, 516/s4-s6].

use super::ScreenReaderEngine;
use crate::messages::msg;

/// The cursor the reading keys act on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ActiveCursor {
    /// The focus (or the virtual cursor in a page).
    #[default]
    Pc,
    /// The mouse pointer.
    Jaws,
}

impl ScreenReaderEngine {
    /// Numpad Minus: "JAWS Cursor" [obs, 516/s1].
    pub(super) fn activate_jaws_cursor(&mut self) -> Option<String> {
        self.active_cursor = ActiveCursor::Jaws;
        self.say(msg::cursor_jaws().to_string())
    }

    /// Numpad Plus with the JAWS cursor active: "PC Cursor" [obs, 516/s7].
    pub(super) fn activate_pc_cursor(&mut self) -> Option<String> {
        self.active_cursor = ActiveCursor::Pc;
        // In a page the PC cursor is the virtual one: "virtual PC Cursor" [obs, 564/s19].
        if self.app_mode == super::AppMode::VirtualBuffer {
            let said = self.virtual_pc_message();
            return self.say(said);
        }
        self.say(msg::cursor_pc().to_string())
    }

    /// Insert+Numpad Minus: the mouse goes to the PC cursor, "Route JAWS to PC" [obs, 516/s3].
    pub(super) fn route_jaws_to_pc(&mut self) -> Option<String> {
        if let Some((x, y)) = self.pc_cursor_point() {
            if let Some(actions) = &self.actions {
                actions.move_mouse(x, y);
            }
            self.mouse = Some((x, y));
        }
        self.say(msg::cursor_route_jaws_to_pc().to_string())
    }

    /// Alt+Delete: the active cursor and where it is, "PC Cursor active | 1045, 544"; the JAWS
    /// cursor adds its restriction, "Application window restriction" [obs, 513/s2, 516/s2,
    /// s8]. JAWS counts the screen's pixels from 1: the mouse at 835, 435 is "836, 436" [obs,
    /// 516/s2 and its trace].
    pub(super) fn say_active_cursor(&mut self) -> Option<String> {
        let (name, at) = match self.active_cursor {
            ActiveCursor::Pc => (msg::cursor_pc_active(), self.pc_cursor_point()),
            ActiveCursor::Jaws => (msg::cursor_jaws_active(), self.mouse),
        };
        let mut said = vec![name.to_string()];
        said.extend(at.map(|(x, y)| {
            msg::cursor_position((x + 1).to_string(), (y + 1).to_string()).to_string()
        }));
        if self.active_cursor == ActiveCursor::Jaws {
            said.push(msg::cursor_restriction_application().to_string());
        }
        for (i, line) in said.iter().enumerate() {
            self.speech.speak(line, i == 0);
        }
        Some(said.join("\n"))
    }

    /// Where the PC cursor is on the screen: the middle pixel of the focused element (its
    /// right and bottom edges are outside it): the mouse Route JAWS to PC put on a button
    /// 1000 to 1090 wide was at 1044 [obs, 516 traces].
    fn pc_cursor_point(&self) -> Option<(i32, i32)> {
        let [left, top, right, bottom] = self.focus_bounds?;
        Some(((left + right - 1) / 2, (top + bottom - 1) / 2))
    }
}
