//! What is spoken, as data: lines made of parts, turned into text only at the end.
//!
//! JAWS speaks a control's type and states in a voice of their own, and its speech history
//! marks them (`‹ Button ›`, `‹checked›`); Okur's speech text marks them the same way
//! (`\u{1d}` … `\u{1e}`), and synthesizers drop the marks. Keeping an announcement as parts
//! until it is rendered lets code change one part (an application module's own wording,
//! later verbosity settings or braille) without editing strings.

use crate::linearize::{role_marker, state_marker};

/// One part of a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    /// Words: a name, a value, a position, a hint.
    Text(String),
    /// A control's type: "Button", "check box", "List view".
    Role(String),
    /// A state: "checked", "open", "selected".
    State(String),
}

impl Part {
    /// The part as speech text, with JAWS's marks around a role or state.
    pub fn render(&self) -> String {
        match self {
            Part::Text(text) => text.clone(),
            Part::Role(role) => role_marker(role),
            Part::State(state) => state_marker(state),
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

    /// A line of words.
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(vec![Part::Text(text.into())])
    }

    /// The same line, said only as focus arrives.
    pub fn on_arrival(mut self) -> Self {
        self.on_arrival = true;
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

    /// The speech text: the lines that say something, one per line.
    pub fn render(&self) -> String {
        self.lines
            .iter()
            .map(Line::render)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_render_with_jaws_marks_and_empty_lines_are_dropped() {
        let announcement = Announcement::new(vec![
            Line::new(vec![
                Part::Text("Remember me".to_owned()),
                Part::Role("check box".to_owned()),
                Part::State("not checked".to_owned()),
            ]),
            Line::text(""),
            Line::text("To check press Spacebar.").on_arrival(),
        ]);
        assert_eq!(
            announcement.render(),
            "Remember me \u{1d} check box \u{1e}  \u{1d}not checked\u{1e} \nTo check press Spacebar."
        );
    }
}
