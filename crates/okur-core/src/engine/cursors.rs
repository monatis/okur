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

/// Where the Okur cursor may go, which Insert+R changes in turn, back to the application's
/// after none [obs, 757, 759].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Restriction {
    /// The application's window, as the cursor starts.
    #[default]
    Application,
    /// The real window around the focus.
    Real,
    /// The window the focus is in.
    Current,
    /// The focused control: Page Up goes to its first line [obs, 757/s14].
    Focus,
    /// The frame the focus is in.
    Frame,
    /// Nowhere it may not go.
    Unrestricted,
}

impl Restriction {
    /// The one after it.
    fn next(self) -> Self {
        match self {
            Restriction::Application => Restriction::Real,
            Restriction::Real => Restriction::Current,
            Restriction::Current => Restriction::Focus,
            Restriction::Focus => Restriction::Frame,
            Restriction::Frame => Restriction::Unrestricted,
            Restriction::Unrestricted => Restriction::Application,
        }
    }

    fn message(self) -> String {
        match self {
            Restriction::Application => msg::cursor_restriction_application(),
            Restriction::Real => msg::cursor_restriction_real(),
            Restriction::Current => msg::cursor_restriction_current(),
            Restriction::Focus => msg::cursor_restriction_focus(),
            Restriction::Frame => msg::cursor_restriction_frame(),
            Restriction::Unrestricted => msg::cursor_unrestricted(),
        }
        .to_string()
    }
}

/// Where the Okur cursor is in what it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkurPlace {
    /// The window's top, Page Up's: its title bar [obs, 752/s15, s16].
    Top,
    /// The window's bottom, Page Down's, below the text: "blank" [obs, 754/s12, s13].
    Bottom,
    /// The empty line above the window's bottom: "blank" too [obs, 754/s14, s15].
    AboveBottom,
    /// A line, and the character on it; none before the line's first, where Up and Down leave
    /// it [obs, 752/s9, 753/s10].
    Text { line: usize, column: Option<usize> },
}

/// Where the words of `text` start, as JAWS splits words: letters and digits apart from
/// other marks, "main" and "()" [obs, 754/s11] ([`crate::editing::web_word_at`]).
fn word_starts(text: &[char]) -> Vec<usize> {
    let line: String = text.iter().collect();
    (0..text.len())
        .filter(|&i| crate::editing::web_word_at(&line, i).is_some_and(|(start, _)| start == i))
        .collect()
}

/// The word of `text` starting at `start`.
fn word_at(text: &[char], start: usize) -> String {
    let line: String = text.iter().collect();
    let end = crate::editing::web_word_at(&line, start).map_or(start, |(_, end)| end);
    text.get(start..end).unwrap_or_default().iter().collect()
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
            self.note_own_mouse_move();
        }
        // The Okur cursor is now on the caret's line, or the virtual cursor's.
        self.okur_place = self.routed_place();
        let said = self.cursor_message(
            msg::cursor_route_okur_to_pc(),
            msg::cursor_route_okur_to_pc_short(),
        );
        self.say(said)
    }

    /// Insert+R: the next restriction, said: "Real window restriction" after the application
    /// window's [obs, 757/s3, s7, s11].
    pub(super) fn restrict_okur_cursor(&mut self) -> Option<String> {
        self.okur_restriction = self.okur_restriction.next();
        let said = self.okur_restriction.message();
        self.say(said)
    }

    /// Insert+Numpad Plus: the PC cursor to the Okur cursor, "Route PC to Okur" (JAWS: "Route
    /// PC to JAWS") [obs, 756/s13, s14]. In a page the virtual cursor goes to the Okur cursor's
    /// line; elsewhere a click at the mouse brings the caret there.
    pub(super) fn route_pc_to_okur(&mut self) -> Option<String> {
        if self.app_mode == super::AppMode::VirtualBuffer {
            if let Some(OkurPlace::Text { line, .. }) = self.okur_place {
                let start = self
                    .cursor
                    .buffer
                    .lines
                    .iter()
                    .filter(|l| screen_line(l))
                    .nth(line)
                    .map(|l| l.start_offset);
                if let Some(start) = start {
                    self.cursor.move_to(start);
                }
            }
        } else if let (Some((x, y)), Some(actions)) = (self.mouse, &self.actions) {
            actions.click_mouse(x, y, crate::actions::MouseClick::LeftSingle);
        }
        let said = msg::cursor_route_pc_to_okur().to_string();
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
        if let Some(rows) = self.okur_dialog_rows() {
            return rows;
        }
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

    /// A dialog box's rows as the Okur cursor reads them, below its title: each control on its
    /// own, a field by its text, a label by its words, buttons side by side on one: "Line
    /// number:", "1", "Go To Button Cancel Button" [obs, 755/s5, s10]. The trace gives the
    /// controls without their places, so the rows follow the dialog's order.
    fn okur_dialog_rows(&self) -> Option<Vec<String>> {
        let focus = self.last_native_focus.as_ref()?;
        if self.app_mode == super::AppMode::VirtualBuffer
            || !focus.in_dialog_box
            || focus.window_items.is_empty()
        {
            return None;
        }
        use crate::aom::Role;
        let mut rows: Vec<String> = Vec::new();
        let mut buttons: Vec<String> = Vec::new();
        for item in &focus.window_items {
            if item.role == Role::Button {
                buttons.push(format!(
                    "{}  {}",
                    item.name,
                    crate::linearize::role_marker(msg::role_button().as_str())
                ));
                continue;
            }
            if !buttons.is_empty() {
                rows.push(std::mem::take(&mut buttons).join(" "));
            }
            let text = match item.role {
                Role::Edit => item.value.clone().unwrap_or_default(),
                _ => item.name.clone(),
            };
            if !text.trim().is_empty() {
                rows.push(text);
            }
        }
        if !buttons.is_empty() {
            rows.push(buttons.join(" "));
        }
        Some(rows)
    }

    /// The row of the dialog the focus is on: the field's, by its text.
    fn focused_dialog_row(&self) -> Option<usize> {
        let focus = self.last_native_focus.as_ref()?;
        let rows = self.okur_dialog_rows()?;
        let text = focus
            .text
            .as_ref()
            .map(|t| t.text.clone())
            .or_else(|| focus.value.clone())
            .or_else(|| focus.name.clone())?;
        rows.iter()
            .position(|row| row == &text || row.starts_with(&text))
    }

    /// Where the Okur cursor goes as it is routed to the PC cursor: the caret's line in a text
    /// field, the focused control's row in a dialog box, the virtual cursor's line in a page,
    /// before its first character.
    fn routed_place(&self) -> Option<OkurPlace> {
        if self.okur_dialog_rows().is_some() {
            return Some(OkurPlace::Text {
                line: self.focused_dialog_row().unwrap_or(0),
                column: None,
            });
        }
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
        Some(match self.okur_place? {
            OkurPlace::Top => self.okur_top_line()?,
            OkurPlace::Bottom | OkurPlace::AboveBottom => String::new(),
            OkurPlace::Text { line, .. } => self.okur_lines().get(line)?.trim().to_owned(),
        })
    }

    /// The Okur cursor's line, said: an empty one is "blank", which JAWS speaks but keeps
    /// nothing of in its history, as for any empty line [obs, 516/s4, 754/s13-s15; 315/s2].
    fn say_okur_line(&mut self) -> Option<String> {
        let line = self.okur_cursor_line()?;
        if line.is_empty() {
            self.speech.speak_character(
                "
", true,
            );
            return Some(msg::blank().to_string());
        }
        self.say(line)
    }

    /// The window's top as the Okur cursor reads it: a page's first line, as JAWS reads the
    /// screen there ("Riverside Gazette" [obs, 753/s17]); an application's title bar.
    fn okur_top_line(&self) -> Option<String> {
        // A dialog box's title bar, with its role: "Go To Line dialog" [obs, 755/s9].
        if self.okur_dialog_rows().is_some() {
            let title = self.last_window_title.clone()?;
            return Some(format!(
                "{title} {}",
                crate::linearize::role_marker(msg::role_dialog().as_str())
            ));
        }
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
        if !self.okur_place_active() {
            return None;
        }
        let moved = self.move_okur_cursor(event);
        if moved.is_some() {
            self.mouse_to_okur_place();
        }
        moved
    }

    /// The Okur cursor is the mouse: in a text field, the mouse goes onto the character the
    /// Okur cursor is at, or the line's first when it is before it, as JAWS's cursor moves the
    /// mouse (the traces of 752-754 follow it). Echo does not read this move.
    fn mouse_to_okur_place(&mut self) {
        if self.app_mode == super::AppMode::VirtualBuffer || self.okur_dialog_rows().is_some() {
            return;
        }
        let Some(OkurPlace::Text { line, column }) = self.okur_place else {
            return;
        };
        let Some(context) = self.last_text.as_ref() else {
            return;
        };
        let Some(start) = crate::editing::line_starts(context).get(line).copied() else {
            return;
        };
        let offset = start + column.unwrap_or(0);
        if let Some(actions) = &self.actions {
            if actions.move_mouse_to_text(context, offset) {
                self.note_own_mouse_move();
            }
        }
    }

    /// The Okur cursor's movement for `event`, without the mouse.
    fn move_okur_cursor(&mut self, event: &crate::input::KeyEvent) -> Option<()> {
        use crate::input::Key;
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
            // Restricted to the focused control, its first line [obs, 757/s14].
            (Key::PageUp, true, _) if self.okur_restriction == Restriction::Focus => {
                self.okur_place = Some(OkurPlace::Text {
                    line: 0,
                    column: None,
                });
            }
            (Key::PageUp, true, _) => self.okur_place = Some(OkurPlace::Top),
            // A page's bottom is its last line; an application's, below its text.
            (Key::PageDown, true, _) => {
                self.okur_place = Some(if self.app_mode == super::AppMode::VirtualBuffer {
                    OkurPlace::Text {
                        line: self.okur_lines().len().saturating_sub(1),
                        column: None,
                    }
                } else {
                    OkurPlace::Bottom
                });
            }
            _ => return None,
        }
        Some(())
    }

    /// Up or Down: the line above or below, read; at the first or last line it stays there.
    /// From the window's top, Down goes to the first line.
    fn move_okur_line(&mut self, down: bool) {
        let last = self.okur_lines().len().saturating_sub(1);
        let next = match (self.okur_place, down) {
            (Some(OkurPlace::Top), true) => OkurPlace::Text {
                line: 0,
                column: None,
            },
            (Some(OkurPlace::Top), false) => OkurPlace::Top,
            // From the bottom, Up says the empty line above it, then the text's last line.
            (Some(OkurPlace::Bottom), false) => OkurPlace::AboveBottom,
            (Some(OkurPlace::Bottom | OkurPlace::AboveBottom), true) => OkurPlace::Bottom,
            (Some(OkurPlace::AboveBottom), false) => OkurPlace::Text {
                line: last,
                column: None,
            },
            (Some(OkurPlace::Text { line, .. }), true) => OkurPlace::Text {
                line: (line + 1).min(last),
                column: None,
            },
            (Some(OkurPlace::Text { line, .. }), false) => OkurPlace::Text {
                line: line.saturating_sub(1),
                column: None,
            },
            (None, _) => return,
        };
        self.okur_place = Some(next);
        let _ = self.say_okur_line();
    }

    /// The line the Okur cursor is on, and the character it is at.
    fn okur_text_place(&self) -> Option<(usize, Option<usize>, Vec<char>)> {
        match self.okur_place? {
            OkurPlace::Text { line, column } => {
                let text = self.okur_lines().get(line)?.chars().collect();
                Some((line, column, text))
            }
            OkurPlace::Top | OkurPlace::Bottom | OkurPlace::AboveBottom => None,
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

    /// Ctrl+Right or Ctrl+Left: the next or previous word, said, going on to the next or
    /// previous line: "let", then "{" at the end of the line above [obs, 752/s11, 754/s8, s9].
    fn move_okur_word(&mut self, right: bool) {
        let Some((mut line, column, text)) = self.okur_text_place() else {
            return;
        };
        let lines = self.okur_lines();
        let mut found = word_starts(&text)
            .into_iter()
            .filter(|&start| match (right, column) {
                (true, None) => true,
                (true, Some(c)) => start > c,
                (false, None) => false,
                (false, Some(c)) => start < c,
            })
            .reduce(|a, b| if right { a.min(b) } else { a.max(b) });
        let mut words_of = text;
        while found.is_none() {
            let next = if right {
                line + 1
            } else {
                match line.checked_sub(1) {
                    Some(up) => up,
                    None => return,
                }
            };
            let Some(other) = lines.get(next) else {
                return;
            };
            line = next;
            words_of = other.chars().collect();
            let starts = word_starts(&words_of);
            found = if right {
                starts.first().copied()
            } else {
                starts.last().copied()
            };
        }
        let Some(start) = found else {
            return;
        };
        self.okur_place = Some(OkurPlace::Text {
            line,
            column: Some(start),
        });
        let _ = self.say(word_at(&words_of, start));
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
            Command::SayLine => Some(self.say_okur_line()),
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
        let start = word_starts(&text)
            .into_iter()
            .filter(|&start| start <= c)
            .max()
            .unwrap_or(c);
        let _ = self.say(word_at(&text, start));
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
            said.push(self.okur_restriction.message());
        }
        for (i, line) in said.iter().enumerate() {
            self.speech.speak(line, i == 0);
        }
        Some(said.join("\n"))
    }

    /// Where the PC cursor is on the screen: the middle pixel of the focused element (its
    /// right and bottom edges are outside it): the mouse Route JAWS to PC put on a button
    /// 1000 to 1090 wide was at 1044 [obs, 516 traces].
    pub(super) fn pc_cursor_point(&self) -> Option<(i32, i32)> {
        let [left, top, right, bottom] = self.focus_bounds?;
        Some(((left + right - 1) / 2, (top + bottom - 1) / 2))
    }
}
