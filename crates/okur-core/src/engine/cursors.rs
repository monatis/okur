//! The cursors (JAWS 2025 help, Keystrokes for the Cursors): the PC cursor follows the focus,
//! the JAWS cursor is the mouse pointer. Numpad Minus makes the JAWS cursor the active one and
//! Numpad Plus the PC cursor; Insert+Numpad Minus routes the JAWS cursor to the PC cursor,
//! moving the mouse there; Alt+Delete says which cursor is active and where it is [obs, 516,
//! 513/s2].
//!
//! Okur has no model of the screen's text. In a text field the Okur cursor moves over the
//! field's lines, from the caret's line once routed there: JAWS's cursor in Notepad read the
//! line at the mouse with Insert+Up and Down, without its indentation, and said nothing for
//! Right, Left, the word, End and Home [obs, 690]. In a dialog it read nothing at the mouse
//! [obs, 516/s4-s6].

use super::ScreenReaderEngine;
use crate::messages::msg;

/// The cursor the reading keys act on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ActiveCursor {
    /// The focus (or the virtual cursor in a page).
    #[default]
    Pc,
    /// The mouse pointer.
    Okur,
}

impl ScreenReaderEngine {
    /// A cursor message, its short form with short messages (Intermediate, Advanced): "JAWS",
    /// "JAWS to PC", "PC" [obs, 697, 698].
    fn cursor_message(&self, long: impl ToString, short: impl ToString) -> String {
        match self.settings.verbosity.message_length() {
            crate::settings::MessageLength::Long => long.to_string(),
            crate::settings::MessageLength::Short => short.to_string(),
        }
    }

    /// Numpad Minus: "Okur Cursor", where JAWS says "JAWS Cursor" [obs, 516/s1].
    pub(super) fn activate_okur_cursor(&mut self) -> Option<String> {
        self.active_cursor = ActiveCursor::Okur;
        let said = self.cursor_message(msg::cursor_okur(), msg::cursor_okur_short());
        self.say(said)
    }

    /// Numpad Plus with the JAWS cursor active: "PC Cursor" [obs, 516/s7].
    pub(super) fn activate_pc_cursor(&mut self) -> Option<String> {
        self.active_cursor = ActiveCursor::Pc;
        // In a page the PC cursor is the virtual one: "virtual PC Cursor" [obs, 564/s19].
        if self.app_mode == super::AppMode::VirtualBuffer {
            let said = self.virtual_pc_message();
            return self.say(said);
        }
        let said = self.cursor_message(msg::cursor_pc(), msg::cursor_pc_short());
        self.say(said)
    }

    /// Insert+Numpad Minus: the mouse goes to the PC cursor, "Route Okur to PC" (JAWS: "Route
    /// JAWS to PC") [obs, 516/s3].
    pub(super) fn route_okur_to_pc(&mut self) -> Option<String> {
        if let Some((x, y)) = self.pc_cursor_point() {
            if let Some(actions) = &self.actions {
                actions.move_mouse(x, y);
            }
            self.mouse = Some((x, y));
        }
        // In a text field the Okur cursor is now on the caret's line.
        self.okur_line = self
            .last_text
            .as_ref()
            .filter(|_| self.app_mode != super::AppMode::VirtualBuffer)
            .map(|text| crate::editing::lines_and_caret_line(text).1);
        let said = self.cursor_message(
            msg::cursor_route_okur_to_pc(),
            msg::cursor_route_okur_to_pc_short(),
        );
        self.say(said)
    }

    /// Whether the reading keys read the Okur cursor's line in a text field.
    pub(super) fn okur_line_active(&self) -> bool {
        self.active_cursor == ActiveCursor::Okur
            && self.app_mode != super::AppMode::VirtualBuffer
            && self.okur_line.is_some()
    }

    /// The Okur cursor's line in the text field, without its indentation; an empty one is
    /// "blank".
    pub(super) fn okur_cursor_line(&self) -> Option<String> {
        let index = self.okur_line?;
        let (lines, _) = crate::editing::lines_and_caret_line(self.last_text.as_ref()?);
        let line = lines.get(index)?.trim().to_owned();
        Some(if line.is_empty() {
            msg::blank().to_string()
        } else {
            line
        })
    }

    /// Up or Down with the Okur cursor in a text field: the line above or below, read; at the
    /// text's first or last line it stays there.
    pub(super) fn move_okur_line(&mut self, down: bool) -> Option<String> {
        let index = self.okur_line?;
        let (lines, _) = crate::editing::lines_and_caret_line(self.last_text.as_ref()?);
        let last = lines.len().saturating_sub(1);
        let next = if down {
            (index + 1).min(last)
        } else {
            index.saturating_sub(1)
        };
        self.okur_line = Some(next);
        let said = self.okur_cursor_line()?;
        self.say(said)
    }

    /// Alt+Delete: the active cursor and where it is, "PC Cursor active | 1045, 544"; the JAWS
    /// cursor adds its restriction, "Application window restriction" [obs, 513/s2, 516/s2,
    /// s8]. JAWS counts the screen's pixels from 1: the mouse at 835, 435 is "836, 436" [obs,
    /// 516/s2 and its trace].
    pub(super) fn say_active_cursor(&mut self) -> Option<String> {
        let (name, at) = match self.active_cursor {
            ActiveCursor::Pc => (msg::cursor_pc_active(), self.pc_cursor_point()),
            ActiveCursor::Okur => (msg::cursor_okur_active(), self.mouse),
        };
        let mut said = vec![name.to_string()];
        said.extend(at.map(|(x, y)| {
            msg::cursor_position((x + 1).to_string(), (y + 1).to_string()).to_string()
        }));
        if self.active_cursor == ActiveCursor::Okur {
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
