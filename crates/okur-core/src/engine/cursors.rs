//! The cursors (JAWS 2025 help, Keystrokes for the Cursors): the PC cursor follows the focus,
//! the JAWS cursor is the mouse pointer. Numpad Minus makes the JAWS cursor the active one and
//! Numpad Plus the PC cursor; Insert+Numpad Minus routes the JAWS cursor to the PC cursor,
//! moving the mouse there; Alt+Delete says which cursor is active and where it is [obs, 516,
//! 513/s2].
//!
//! Okur has no model of the screen's pixels. Routed to the PC cursor, the Okur cursor reads the
//! text there: a text field's lines, or a page's as its screen shows them, by line, character
//! and word, from the caret's or the virtual cursor's line, leaving the virtual cursor where it
//! is [obs, 690, 752, 753]. In the dialog fixture JAWS's cursor read nothing at the mouse [obs,
//! 516/s4-s6].

use super::ScreenReaderEngine;
use crate::messages::msg;

/// Where the Okur cursor is in what it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkurPlace {
    /// The window's top, Page Up's: its title bar [obs, 752/s15, s16].
    Top,
    /// A line, and the character on it; none before the line's first, where Up and Down leave
    /// it [obs, 752/s9, 753/s10].
    Text { line: usize, column: Option<usize> },
}

/// Whether a page's line is text its screen shows: not the page's title, nor a region's or a
/// list's announcement.
fn screen_line(line: &crate::buffer::BufferLine) -> bool {
    use crate::aom::Role;
    !matches!(
        line.role,
        Role::Document | Role::Landmark { .. } | Role::List | Role::Table
    ) && !line.text.trim().is_empty()
}

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
        // The Okur cursor is now on the caret's line, or the virtual cursor's.
        self.okur_place = self.routed_place();
        let said = self.cursor_message(
            msg::cursor_route_okur_to_pc(),
            msg::cursor_route_okur_to_pc_short(),
        );
        self.say(said)
    }

    /// Whether the reading keys read where the Okur cursor is: it is active, and on a text it
    /// reads (a text field's, a page's).
    pub(super) fn okur_place_active(&self) -> bool {
        self.active_cursor == ActiveCursor::Okur && self.okur_place.is_some()
    }

    /// The lines the Okur cursor reads: a page's as its screen shows them, without the roles
    /// and the region and list announcements ("Weekend events", "Three events open to everyone
    /// this weekend." [obs, 753/s5-s9]); a text field's, indentation and all.
    fn okur_lines(&self) -> Vec<String> {
        if self.app_mode == super::AppMode::VirtualBuffer {
            return self
                .cursor
                .buffer
                .lines
                .iter()
                .filter(|line| screen_line(line))
                .map(|line| line.text.clone())
                .collect();
        }
        self.last_text
            .as_ref()
            .map(|text| crate::editing::lines_and_caret_line(text).0)
            .unwrap_or_default()
    }

    /// Where the Okur cursor goes as it is routed to the PC cursor: the caret's line in a text
    /// field, the virtual cursor's in a page, before its first character.
    fn routed_place(&self) -> Option<OkurPlace> {
        let line = if self.app_mode == super::AppMode::VirtualBuffer {
            let at = self.cursor.current_line_index();
            let screen: Vec<usize> = self
                .cursor
                .buffer
                .lines
                .iter()
                .filter(|line| screen_line(line))
                .map(|line| line.line_index)
                .collect();
            screen
                .iter()
                .position(|&index| index >= at)
                .unwrap_or(screen.len().saturating_sub(1))
        } else {
            crate::editing::lines_and_caret_line(self.last_text.as_ref()?).1
        };
        Some(OkurPlace::Text { line, column: None })
    }

    /// The Okur cursor's line, without its indentation; an empty one is "blank". At the
    /// window's top, its title: "sr-notepad-fixture.txt - Notepad" [obs, 752/s16].
    pub(super) fn okur_cursor_line(&self) -> Option<String> {
        let line = match self.okur_place? {
            OkurPlace::Top => self.okur_top_line()?,
            OkurPlace::Text { line, .. } => self.okur_lines().get(line)?.trim().to_owned(),
        };
        Some(if line.is_empty() {
            msg::blank().to_string()
        } else {
            line
        })
    }

    /// The window's top as the Okur cursor reads it: a page's first line, as JAWS reads the
    /// screen there ("Riverside Gazette" [obs, 753/s17]); an application's title bar.
    fn okur_top_line(&self) -> Option<String> {
        if self.app_mode == super::AppMode::VirtualBuffer {
            return self.okur_lines().first().map(|l| l.trim().to_owned());
        }
        self.last_window_title.clone()
    }

    /// A key that moves the Okur cursor, with it active on a text: Up and Down by line, read;
    /// Right and Left by character, said; Ctrl with them by word, said; Home and End to the
    /// line's ends and Page Up to the window's top, quietly [obs, 752, 753]. None for any other
    /// key.
    pub(super) fn okur_cursor_key(&mut self, event: &crate::input::KeyEvent) -> Option<()> {
        use crate::input::Key;
        if !self.okur_place_active() {
            return None;
        }
        let ctrl = event.modifiers.ctrl
            && !event.modifiers.alt
            && !event.modifiers.shift
            && !event.modifiers.insert;
        match (event.key, event.modifiers.is_empty(), ctrl) {
            (Key::UpArrow | Key::DownArrow, true, _) => {
                self.move_okur_line(event.key == Key::DownArrow);
            }
            (Key::LeftArrow | Key::RightArrow, true, _) => {
                self.move_okur_character(event.key == Key::RightArrow);
            }
            (Key::LeftArrow | Key::RightArrow, _, true) => {
                self.move_okur_word(event.key == Key::RightArrow);
            }
            (Key::Home | Key::End, true, _) => self.okur_line_end(event.key == Key::End),
            (Key::PageUp, true, _) => self.okur_place = Some(OkurPlace::Top),
            _ => return None,
        }
        Some(())
    }

    /// Up or Down: the line above or below, read; at the first or last line it stays there.
    /// From the window's top, Down goes to the first line.
    fn move_okur_line(&mut self, down: bool) {
        let last = self.okur_lines().len().saturating_sub(1);
        let next = match self.okur_place {
            Some(OkurPlace::Top) => 0,
            Some(OkurPlace::Text { line, .. }) if down => (line + 1).min(last),
            Some(OkurPlace::Text { line, .. }) => line.saturating_sub(1),
            None => return,
        };
        self.okur_place = Some(OkurPlace::Text {
            line: next,
            column: None,
        });
        if let Some(said) = self.okur_cursor_line() {
            let _ = self.say(said);
        }
    }

    /// The line the Okur cursor is on, and the character it is at.
    fn okur_text_place(&self) -> Option<(usize, Option<usize>, Vec<char>)> {
        match self.okur_place? {
            OkurPlace::Text { line, column } => {
                let text = self.okur_lines().get(line)?.chars().collect();
                Some((line, column, text))
            }
            OkurPlace::Top => None,
        }
    }

    /// Right or Left: the next or previous character on the line, said ("space" for a space
    /// [obs, 690/s7-s9, 752/s9, s10]); from before the line's first, Right says the first ("T"
    /// [obs, 753/s10]).
    fn move_okur_character(&mut self, right: bool) {
        let Some((line, column, text)) = self.okur_text_place() else {
            return;
        };
        let last = text.len().checked_sub(1);
        let next = match (column, right) {
            (None, true) => last.map(|_| 0),
            (None, false) => None,
            (Some(c), true) => Some(last.map_or(c, |last| (c + 1).min(last))),
            (Some(c), false) => Some(c.saturating_sub(1)),
        };
        self.okur_place = Some(OkurPlace::Text { line, column: next });
        if let Some(c) = next.and_then(|c| text.get(c)) {
            self.speech.speak_character(&c.to_string(), true);
        }
    }

    /// Ctrl+Right or Ctrl+Left: the next or previous word on the line, said ("let" [obs,
    /// 752/s11]).
    fn move_okur_word(&mut self, right: bool) {
        let Some((line, column, text)) = self.okur_text_place() else {
            return;
        };
        let blank = |i: usize| text.get(i).is_none_or(|c| c.is_whitespace());
        let starts: Vec<usize> = (0..text.len())
            .filter(|&i| !blank(i) && i.checked_sub(1).is_none_or(blank))
            .collect();
        let next = if right {
            starts
                .iter()
                .copied()
                .find(|&start| column.is_none_or(|c| start > c))
        } else {
            starts
                .iter()
                .copied()
                .rev()
                .find(|&start| column.is_some_and(|c| start < c))
        };
        let Some(start) = next else {
            return;
        };
        self.okur_place = Some(OkurPlace::Text {
            line,
            column: Some(start),
        });
        let word: String = text
            .iter()
            .skip(start)
            .take_while(|c| !c.is_whitespace())
            .collect();
        let _ = self.say(word);
    }

    /// Home or End: the line's first or last character, quietly [obs, 752/s13, s14].
    fn okur_line_end(&mut self, end: bool) {
        let Some((line, _, text)) = self.okur_text_place() else {
            return;
        };
        let column = if end {
            text.len().checked_sub(1)
        } else {
            (!text.is_empty()).then_some(0)
        };
        self.okur_place = Some(OkurPlace::Text { line, column });
    }

    /// Insert+Right or Insert+Left: the next or previous word, as Ctrl with the arrows
    /// ("total" [obs, 752/s12]).
    pub(super) fn okur_next_word(&mut self, right: bool) {
        self.move_okur_word(right);
    }

    /// A reading command for the Okur cursor, with it active on a text: Say Line its line,
    /// Say Word its word (a space said as one [obs, 690/s10]), Say Character its character,
    /// Say Next and Prior Word the next and previous word. None for any other command, which
    /// runs as without it.
    pub(super) fn okur_cursor_command(
        &mut self,
        command: &crate::command::Command,
    ) -> Option<Option<String>> {
        use crate::command::Command;
        if !self.okur_place_active() {
            return None;
        }
        match command {
            Command::SayLine => {
                let said = self.okur_cursor_line();
                Some(said.and_then(|line| self.say(line)))
            }
            Command::SayWord => {
                self.okur_word();
                Some(None)
            }
            Command::SayCharacter => {
                self.okur_character();
                Some(None)
            }
            Command::SayNextWord | Command::SayPriorWord => {
                self.okur_next_word(*command == Command::SayNextWord);
                Some(None)
            }
            _ => None,
        }
    }

    /// The word the Okur cursor is in, said; at a space, the space.
    fn okur_word(&mut self) {
        let Some((_, Some(c), text)) = self.okur_text_place() else {
            return;
        };
        if text.get(c).is_none_or(|ch| ch.is_whitespace()) {
            self.okur_character();
            return;
        }
        let start = (0..=c)
            .rev()
            .take_while(|&i| text.get(i).is_some_and(|ch| !ch.is_whitespace()))
            .last()
            .unwrap_or(c);
        let word: String = text
            .iter()
            .skip(start)
            .take_while(|ch| !ch.is_whitespace())
            .collect();
        let _ = self.say(word);
    }

    /// The character the Okur cursor is at, said; before the line's first, nothing.
    pub(super) fn okur_character(&mut self) {
        if let Some((_, Some(c), text)) = self.okur_text_place() {
            if let Some(c) = text.get(c) {
                self.speech.speak_character(&c.to_string(), true);
            }
        }
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
