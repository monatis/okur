//! What is spoken, as data: lines made of parts, turned into text only at the end.
//!
//! JAWS speaks a control's type and states in a voice of their own, and its speech history
//! marks them (`‹ Button ›`, `‹checked›`); Okur's speech text marks them the same way
//! (`\u{1d}` … `\u{1e}`), and synthesizers drop the marks. Keeping an announcement as parts
//! until it is rendered lets code change one part (an application module's own wording,
//! verbosity settings or braille) without editing strings.
//!
//! Every part has a kind ([`PartKind`]): what it says about the object (its name, its role,
//! its position, a usage hint…). Verbosity filters parts by kind ([`Announcement::retain`]);
//! the words themselves come from the catalog ([`crate::messages`]) or from the application.

use crate::linearize::{role_marker, state_marker};

/// What a part of an announcement says (proposal §6.1). Verbosity decides by kind which parts
/// are said; only roles and states are rendered with JAWS's marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartKind {
    /// The object's name: "Remember me".
    Name,
    /// Its value or text: "Germany", "50 percent", "Contains text".
    Value,
    /// Its type: "check box", "List view".
    Role,
    /// A state: "checked", "expanded".
    State,
    /// Its place among its siblings: "2 of 5".
    Position,
    /// An item's place as the arrows move within its list: "3 of 3", which verbosity can leave
    /// out ([`crate::settings::OutputType::ListPosition`]).
    ListPosition,
    /// Its depth: a tree item's "level 2".
    Level,
    /// Its description (aria-description, help text).
    Description,
    /// What a key does on it: "To activate press Enter."
    Hint,
    /// Its keyboard shortcut: "Ctrl+L".
    Shortcut,
    /// Its access key: "alt+S", "Alt followed by I".
    AccessKey,
    /// A container entered on the way to it: a named group, a list, a dialog.
    Container,
    /// One of Okur's own messages about it: "Open List Box".
    Message,
    /// Words with no kind of their own, or several kinds already joined.
    Text,
}

/// One part of a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub kind: PartKind,
    /// The words, without marks: the application's, or rendered from the catalog.
    pub text: String,
    /// What JAWS says instead with short messages (Intermediate and Advanced verbosity): "1"
    /// for "level 1"; empty when the short form says nothing.
    pub short: Option<String>,
}

impl Part {
    pub fn new(kind: PartKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            short: None,
        }
    }

    /// Words with no kind of their own (spacing, words joined already).
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(PartKind::Text, text)
    }

    /// A control's type: "Button", "check box", "List view".
    pub fn role(role: impl Into<String>) -> Self {
        Self::new(PartKind::Role, role)
    }

    /// A state: "checked", "open", "selected".
    pub fn state(state: impl Into<String>) -> Self {
        Self::new(PartKind::State, state)
    }

    /// The part as speech text, with JAWS's marks around a role or state.
    pub fn render(&self) -> String {
        match self.kind {
            PartKind::Role => role_marker(&self.text),
            PartKind::State => state_marker(&self.text),
            _ => self.text.clone(),
        }
    }
}

/// One line: an entry of JAWS's speech history.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Line {
    pub parts: Vec<Part>,
    /// Said only as focus arrives: Say Current Focus (Insert+Tab) leaves it out.
    pub on_arrival: bool,
}

impl Line {
    pub fn new(parts: Vec<Part>) -> Self {
        Self {
            parts,
            on_arrival: false,
        }
    }

    /// A line of words with no kind of their own.
    pub fn text(text: impl Into<String>) -> Self {
        Self::of(PartKind::Text, text)
    }

    /// A line of one part of `kind`: `Line::of(PartKind::Hint, "To activate press Enter.")`.
    pub fn of(kind: PartKind, text: impl Into<String>) -> Self {
        Self::new(vec![Part::new(kind, text)])
    }

    /// The same line, said only as focus arrives.
    pub fn on_arrival(mut self) -> Self {
        self.on_arrival = true;
        self
    }

    /// The same line with `short` as its last part's short form ([`Part::short`]).
    pub fn short(mut self, short: impl Into<String>) -> Self {
        if let Some(part) = self.parts.last_mut() {
            part.short = Some(short.into());
        }
        self
    }

    pub fn render(&self) -> String {
        self.parts.iter().map(Part::render).collect()
    }
}

/// What is spoken for one event, line by line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Announcement {
    pub lines: Vec<Line>,
}

impl Announcement {
    pub fn new(lines: Vec<Line>) -> Self {
        Self { lines }
    }

    /// Text as lines of words, one per line of `text`.
    pub fn from_text(text: &str) -> Self {
        Self::new(text.split('\n').map(Line::text).collect())
    }

    /// The speech text: the lines that say something, one per line.
    pub fn render(&self) -> String {
        self.lines
            .iter()
            .map(Line::render)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Every line joined as it is, empty ones included: text built line by line, as a string
    /// joined with `\n` would be.
    pub fn render_all(&self) -> String {
        self.lines
            .iter()
            .map(Line::render)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Keeps the parts whose kind `keep` accepts; a line left without parts goes.
    pub fn retain(&mut self, keep: impl Fn(PartKind) -> bool) {
        for line in &mut self.lines {
            line.parts.retain(|part| keep(part.kind));
        }
        self.lines.retain(|line| !line.parts.is_empty());
    }

    /// Says the parts that have a short form by it ([`Part::short`]); a part whose short form
    /// is empty goes, and a line left without parts.
    pub fn shorten(&mut self) {
        for line in &mut self.lines {
            line.parts.retain_mut(|part| match part.short.take() {
                Some(short) => {
                    part.text = short;
                    !part.text.is_empty()
                }
                None => true,
            });
        }
        self.lines.retain(|line| !line.parts.is_empty());
    }

    /// The kinds of its parts, in order.
    pub fn kinds(&self) -> Vec<PartKind> {
        self.lines
            .iter()
            .flat_map(|line| line.parts.iter().map(|part| part.kind))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_render_with_jaws_marks_and_empty_lines_are_dropped() {
        let announcement = Announcement::new(vec![
            Line::new(vec![
                Part::new(PartKind::Name, "Remember me"),
                Part::role("check box"),
                Part::state("not checked"),
            ]),
            Line::text(""),
            Line::of(PartKind::Hint, "To check press Spacebar.").on_arrival(),
        ]);
        assert_eq!(
            announcement.render(),
            "Remember me \u{1d} check box \u{1e}  \u{1d}not checked\u{1e} \nTo check press Spacebar."
        );
        assert_eq!(
            announcement.render_all(),
            "Remember me \u{1d} check box \u{1e}  \u{1d}not checked\u{1e} \n\nTo check press Spacebar."
        );
    }

    #[test]
    fn parts_are_kept_or_dropped_by_kind() {
        let mut announcement = Announcement::new(vec![
            Line::new(vec![Part::new(PartKind::Name, "Yes"), Part::role("Button")]),
            Line::of(PartKind::Hint, "To activate press spacebar."),
            Line::of(PartKind::AccessKey, "alt+Y").on_arrival(),
        ]);
        assert_eq!(
            announcement.kinds(),
            [
                PartKind::Name,
                PartKind::Role,
                PartKind::Hint,
                PartKind::AccessKey
            ]
        );
        announcement.retain(|kind| kind != PartKind::AccessKey);
        assert_eq!(
            announcement.render(),
            "Yes \u{1d} Button \u{1e} \nTo activate press spacebar."
        );
        assert_eq!(announcement.lines.len(), 2);
    }

    #[test]
    fn short_messages_say_a_parts_short_form_or_leave_it_out() {
        let mut announcement = Announcement::new(vec![
            Line::of(PartKind::Message, "Open List Box").short(""),
            Line::of(PartKind::Level, "level 1").short("1"),
            Line::text("Reports"),
        ]);
        let long = announcement.render();
        announcement.shorten();
        assert_eq!(long, "Open List Box\nlevel 1\nReports");
        assert_eq!(announcement.render(), "1\nReports");
    }
}
