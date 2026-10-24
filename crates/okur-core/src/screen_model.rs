//! The screen's text where it is drawn, as lines from the places of its pieces: the Virtual
//! Viewer's rows of a classic window ([`crate::viewer_text`]), and the Okur cursor's model of a
//! window, Okur's own feature
//! (`docs/plans/2026-10-23-okur-cursor-screen-text.md`).
//!
//! JAWS's cursor reads a model of what applications draw, and reads nothing where they draw
//! with what it cannot catch (WPF, DirectWrite, pictures) [obs, 994, 995]. Okur's model holds
//! the text the accessibility interfaces place on screen, and the text recognized from the
//! screen's pixels (OCR) where no accessible text lies, as JAWS's Convenient OCR puts its
//! recognized words among its model's lines without saying any twice [obs, 996, 997].

use crate::aom::Role;
use crate::viewer_text::ViewerLayout;
use serde::{Deserialize, Serialize};

/// Where a piece of the screen's text came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    /// An accessibility interface's text, with its element's place.
    Accessible,
    /// Recognized from the screen's pixels.
    Recognized,
}

/// Text at its place on the screen, in desktop pixels; `right` and `bottom` lie outside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenRun {
    pub text: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub source: Source,
    /// The element's role, for a window read element by element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// A check box's or radio button's state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// The element with the keyboard focus, where routing the Okur cursor to the PC cursor
    /// takes it: a trace can lack the field's own focus event, and the PC cursor's point is
    /// then the window's middle (1018/s3).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub focused: bool,
}

impl ScreenRun {
    pub fn new(text: impl Into<String>, rect: (i32, i32, i32, i32), source: Source) -> Self {
        let (left, top, right, bottom) = rect;
        Self {
            text: text.into(),
            left,
            top,
            right,
            bottom,
            source,
            role: None,
            checked: None,
            focused: false,
        }
    }

    /// This run, an element with `role`.
    pub fn with_role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    fn middle(&self) -> (i32, i32) {
        (middle(self.left, self.right), middle(self.top, self.bottom))
    }

    fn contains(&self, (x, y): (i32, i32)) -> bool {
        spans(self.left, self.right, x) && spans(self.top, self.bottom, y)
    }
}

// The places are an application's word: odd ones must not overflow the arithmetic, which the
// dev build checks (performance review 2026-10-24, 5).

/// The middle of `from` to `to`.
fn middle(from: i32, to: i32) -> i32 {
    let middle = (i64::from(from) + i64::from(to)) / 2;
    i32::try_from(middle).unwrap_or(from)
}

/// Whether `at` lies from `from` to `to`, which takes one pixel at least.
fn spans(from: i32, to: i32, at: i32) -> bool {
    (from..to.max(from.saturating_add(1))).contains(&at)
}

/// A run's part of a line: its characters and its place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    /// The line's character its text starts at.
    pub start: usize,
    /// Its characters.
    pub chars: usize,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub source: Source,
}

/// A line of the screen: its text, and the pieces it is made of, left to right, a space
/// between each two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenLine {
    pub text: String,
    pub pieces: Vec<Piece>,
    /// An element's line: its role's word, said after its text.
    pub role: Option<&'static str>,
    /// An element's state, said after its role ("not checked").
    pub state: Option<&'static str>,
    /// It holds the element with the keyboard focus.
    pub focused: bool,
}

impl ScreenLine {
    /// The line as the Okur cursor says it: its text, then an element's role ("WPF ‹Button›"
    /// [obs, 1002/s11]).
    pub fn spoken(&self) -> String {
        let text = self.text.trim();
        let mut said = match self.role {
            // An unnamed element is its role, said as text: "combo box" [obs, 1016/s13].
            Some(role) if text.is_empty() => role.to_owned(),
            Some(role) => format!("{text}{}", crate::linearize::role_marker(role)),
            None => self.text.clone(),
        };
        if let Some(state) = self.state {
            said.push_str(&crate::linearize::state_marker(state));
        }
        said
    }

    /// The piece character `column` is in; a space between two pieces is the earlier one's.
    fn piece_at(&self, column: usize) -> Option<&Piece> {
        self.pieces
            .iter()
            .rev()
            .find(|piece| piece.start <= column)
            .or_else(|| self.pieces.first())
    }

    /// Where character `column` is drawn: its share of its piece's width, at the piece's
    /// middle height. The accessibility interfaces place an element, not its characters.
    pub fn point(&self, column: usize) -> Option<(i32, i32)> {
        let piece = self.piece_at(column)?;
        let chars = i32::try_from(piece.chars.max(1)).unwrap_or(i32::MAX);
        let index = i32::try_from(column.saturating_sub(piece.start))
            .unwrap_or(i32::MAX)
            .min(chars - 1);
        let (left, width) = (
            i64::from(piece.left),
            i64::from(piece.right) - i64::from(piece.left),
        );
        let (index, chars) = (i64::from(index), i64::from(chars));
        let x = left + width * index / chars + width / chars / 2;
        Some((
            i32::try_from(x).unwrap_or(piece.left),
            middle(piece.top, piece.bottom),
        ))
    }

    /// Whether character `column` was recognized from the screen's pixels.
    pub fn recognized_at(&self, column: usize) -> bool {
        self.piece_at(column)
            .is_some_and(|piece| piece.source == Source::Recognized)
    }

    /// Whether the line has a piece where `point` is.
    pub fn holds(&self, (x, y): (i32, i32)) -> bool {
        self.pieces
            .iter()
            .any(|piece| spans(piece.left, piece.right, x) && spans(piece.top, piece.bottom, y))
    }
}

/// The lines of `runs`: rows top to bottom, each row's runs left to right, joined by a space.
/// A run joins the first row whose band holds its middle; a row's band is its first run's, so
/// a tall run never pulls the rows beside it into one. A run without text keeps its band, and
/// is left out of its row's text; a row with no text is no line.
pub fn lines(runs: &[ScreenRun]) -> Vec<ScreenLine> {
    let mut sorted: Vec<&ScreenRun> = runs.iter().collect();
    sorted.sort_by_key(|r| (r.top, r.left));
    let mut rows: Vec<(i32, i32, Vec<&ScreenRun>)> = Vec::new();
    for run in sorted {
        let middle = run.middle().1;
        match rows
            .iter_mut()
            .find(|(top, bottom, _)| (*top..*bottom).contains(&middle))
        {
            Some((_, _, row)) => row.push(run),
            None => rows.push((
                run.top,
                run.bottom.max(run.top.saturating_add(1)),
                vec![run],
            )),
        }
    }
    rows.into_iter()
        .map(|(_, _, mut row)| {
            row.sort_by_key(|r| r.left);
            let mut line = ScreenLine {
                text: String::new(),
                pieces: Vec::with_capacity(row.len()),
                role: None,
                state: None,
                focused: false,
            };
            for run in row.into_iter().filter(|r| !r.text.is_empty()) {
                line.focused |= run.focused;
                if !line.text.is_empty() {
                    line.text.push(' ');
                }
                let start = line.text.chars().count();
                line.text.push_str(&run.text);
                line.pieces.push(Piece {
                    start,
                    chars: run.text.chars().count(),
                    left: run.left,
                    top: run.top,
                    right: run.right,
                    bottom: run.bottom,
                    source: run.source,
                });
            }
            line
        })
        .filter(|line| !line.pieces.is_empty())
        .collect()
}

/// The word JAWS's Scan Cursor says after an element of `role`: none after text, a list's
/// items and a menu item ("System", "First item"), "list view" after a list [obs, 1016/s6-s17].
fn element_role_word(role: &Role) -> Option<&'static str> {
    use crate::messages::msg;
    match role {
        Role::Text | Role::ListItem | Role::MenuItem => None,
        Role::List => Some(msg::role_list_view_element().as_str()),
        Role::ComboBox => Some(msg::role_combo_box().as_str()),
        Role::Edit => Some(msg::role_edit().as_str()),
        other => crate::viewer_text::role_word(other),
    }
}

/// Each element its own line, in the window's order: its name, without its role's word where
/// the name ends with it ("WPF button" is "WPF", said with the role: "WPF ‹Button›" [obs,
/// 1002/s11]), its role and its state; an element without a name is its role alone.
pub fn element_lines(runs: &[ScreenRun]) -> Vec<ScreenLine> {
    use crate::messages::msg;
    runs.iter()
        .filter(|run| !run.text.trim().is_empty() || run.role.is_some())
        .map(|run| {
            let role = run.role.as_ref().and_then(element_role_word);
            let state = run.checked.map(|checked| {
                if checked {
                    msg::state_checked().as_str()
                } else {
                    msg::state_not_checked().as_str()
                }
            });
            let mut text = run.text.trim().to_owned();
            if let Some(word) = role {
                if let Some((name, last)) = text.rsplit_once(' ') {
                    if last.eq_ignore_ascii_case(word) && !name.trim().is_empty() {
                        text = name.trim_end().to_owned();
                    }
                }
            }
            ScreenLine {
                pieces: vec![Piece {
                    start: 0,
                    chars: text.chars().count(),
                    left: run.left,
                    top: run.top,
                    right: run.right,
                    bottom: run.bottom,
                    source: run.source,
                }],
                text,
                role,
                state,
                focused: run.focused,
            }
        })
        .filter(|line| !line.text.is_empty() || line.role.is_some())
        .collect()
}

/// The text the accessibility interfaces place on a window, and how it is read: by screen rows,
/// or element by element in a modern app's window, as JAWS's Scan Cursor reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowText {
    #[serde(default)]
    pub layout: ViewerLayout,
    pub runs: Vec<ScreenRun>,
}

impl WindowText {
    /// A window read by rows.
    pub fn rows(runs: Vec<ScreenRun>) -> Self {
        Self {
            layout: ViewerLayout::Rows,
            runs,
        }
    }

    /// A window read element by element.
    pub fn elements(runs: Vec<ScreenRun>) -> Self {
        Self {
            layout: ViewerLayout::Elements,
            runs,
        }
    }
}

/// A window's model: what the accessibility interfaces place on it, and what was recognized
/// from its pixels, with the lines they make, built when either changes: the Okur cursor reads
/// them several times a key, and building them took 5 ms with 2,000 runs and 3,000 words
/// (performance review 2026-10-24, 8).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScreenModel {
    /// The window it is of, by its title, as the engine knows the window in front.
    pub window: Option<String>,
    accessible: Vec<ScreenRun>,
    recognized: Vec<ScreenRun>,
    /// How its accessible text is read.
    layout: ViewerLayout,
    lines: std::sync::Arc<[ScreenLine]>,
}

impl ScreenModel {
    /// The model of `window` with its accessible `text`, nothing recognized.
    pub fn new(window: Option<String>, text: WindowText) -> Self {
        let mut model = Self {
            window,
            accessible: text.runs,
            layout: text.layout,
            ..Self::default()
        };
        model.build();
        model
    }

    /// The same window's model with `words` recognized in it, in place of any before.
    pub fn with_recognized(mut self, words: Vec<ScreenRun>) -> Self {
        self.set_recognized(words);
        self
    }

    /// The same window's model with new accessible `text`, its recognized words kept.
    pub fn with_text(mut self, text: WindowText) -> Self {
        self.accessible = text.runs;
        self.layout = text.layout;
        self.build();
        self
    }

    /// `words` recognized in the window, in place of any before.
    pub fn set_recognized(&mut self, words: Vec<ScreenRun>) {
        self.recognized = words;
        self.build();
    }

    pub fn accessible(&self) -> &[ScreenRun] {
        &self.accessible
    }

    pub fn recognized(&self) -> &[ScreenRun] {
        &self.recognized
    }

    pub fn layout(&self) -> ViewerLayout {
        self.layout
    }

    fn build(&mut self) {
        self.lines = self.make_lines().into();
    }

    /// The model's runs: every accessible one, and each recognized one whose middle lies on
    /// no accessible run's place, which the accessibility interfaces already read.
    pub fn runs(&self) -> Vec<ScreenRun> {
        let mut runs = self.accessible.clone();
        runs.extend(
            self.recognized
                .iter()
                .filter(|word| !self.accessible.iter().any(|a| a.contains(word.middle())))
                .cloned(),
        );
        runs
    }

    /// The model's lines, shared.
    pub fn lines(&self) -> std::sync::Arc<[ScreenLine]> {
        self.lines.clone()
    }

    /// Its elements one a line in a window read element by element; by rows otherwise, and
    /// wherever words were recognized, which lie where they are drawn.
    fn make_lines(&self) -> Vec<ScreenLine> {
        if self.layout == ViewerLayout::Elements && self.recognized.is_empty() {
            return element_lines(&self.accessible);
        }
        lines(&self.runs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accessible(text: &str, left: i32, top: i32, right: i32, bottom: i32) -> ScreenRun {
        ScreenRun::new(text, (left, top, right, bottom), Source::Accessible)
    }

    fn recognized(text: &str, left: i32, top: i32, right: i32, bottom: i32) -> ScreenRun {
        ScreenRun::new(text, (left, top, right, bottom), Source::Recognized)
    }

    fn texts(lines: &[ScreenLine]) -> Vec<&str> {
        lines.iter().map(|l| l.text.as_str()).collect()
    }

    #[test]
    fn runs_make_rows_left_to_right() {
        let runs = vec![
            accessible("Cancel", 120, 200, 200, 224),
            accessible("Name:", 10, 20, 60, 40),
            accessible("OK", 10, 202, 100, 226),
            accessible("Ada", 70, 18, 300, 44),
        ];
        assert_eq!(texts(&lines(&runs)), ["Name: Ada", "OK Cancel"]);
    }

    #[test]
    fn a_tall_run_does_not_join_the_rows_beside_it() {
        let runs = vec![
            accessible("Notes", 10, 10, 60, 30),
            accessible("Taller", 70, 12, 120, 40),
            accessible("Below", 10, 32, 60, 52),
        ];
        // "Taller" reaches below "Notes", but the row's band is "Notes"'s.
        assert_eq!(texts(&lines(&runs)), ["Notes Taller", "Below"]);
    }

    /// The fixture of 994-997: the panel's painted text and the picture have no accessible
    /// text; recognition adds them among the rest, and says nothing the edit already says.
    #[test]
    fn recognized_words_fill_what_no_accessible_text_covers() {
        let model = ScreenModel::new(
            Some("SR Drawn Text Fixture".to_owned()),
            WindowText::rows(vec![
                accessible("Label drawn by GDI", 20, 16, 240, 48),
                accessible("Focus here", 20, 340, 320, 380),
            ]),
        )
        .with_recognized(vec![
            recognized("Label", 22, 20, 80, 44),
            recognized("Painted", 20, 80, 110, 108),
            recognized("text", 120, 80, 170, 108),
            recognized("Picture", 24, 200, 110, 230),
            recognized("Focus", 24, 346, 90, 372),
            recognized("here", 96, 346, 150, 372),
        ]);
        assert_eq!(
            texts(&model.lines()),
            [
                "Label drawn by GDI",
                "Painted text",
                "Picture",
                "Focus here"
            ]
        );
        let lines = model.lines();
        assert!(lines.get(1).is_some_and(|l| l.recognized_at(0)));
        assert!(lines.first().is_some_and(|l| !l.recognized_at(0)));
    }

    /// A modern app's window, element by element: each its own line in the window's order,
    /// though two lie side by side, each with its role, a name ending with its role's word
    /// without it [obs, 1002/s8-s11].
    #[test]
    fn elements_are_lines_of_their_own_with_their_roles() {
        let model = ScreenModel::new(
            None,
            WindowText::elements(vec![
                accessible("WPF text block", 594, 353, 1326, 388).with_role(Role::Text),
                accessible("Left words", 594, 403, 714, 438).with_role(Role::Text),
                accessible("Right words", 764, 403, 903, 438).with_role(Role::Text),
                accessible("WPF button", 594, 453, 764, 500).with_role(Role::Button),
            ]),
        );
        let spoken: Vec<String> = model.lines().iter().map(ScreenLine::spoken).collect();
        assert_eq!(
            spoken,
            [
                "WPF text block",
                "Left words",
                "Right words",
                "WPF \u{1d} Button \u{1e} "
            ]
        );
    }

    /// The Scan Cursor's other elements: a check box with its state, an unnamed combo box and
    /// list by their roles, a list's items and the title bar's menu without one [obs,
    /// 1016/s6, s10, s13-s16].
    #[test]
    fn elements_say_their_states_and_unnamed_ones_their_roles() {
        let mut check = accessible("Remember me", 0, 0, 10, 10).with_role(Role::CheckBox);
        check.checked = Some(false);
        let spoken: Vec<String> = element_lines(&[
            accessible("System", 0, 0, 10, 10).with_role(Role::MenuItem),
            check,
            accessible("", 0, 0, 10, 10).with_role(Role::ComboBox),
            accessible("", 0, 0, 10, 10).with_role(Role::List),
            accessible("First item", 0, 0, 10, 10).with_role(Role::ListItem),
        ])
        .iter()
        .map(ScreenLine::spoken)
        .collect();
        assert_eq!(
            spoken,
            [
                "System",
                "Remember me \u{1d} check box \u{1e}  \u{1d}not checked\u{1e} ",
                "combo box",
                "list view",
                "First item"
            ]
        );
    }

    #[test]
    fn a_character_is_placed_by_its_share_of_its_piece() {
        let line = lines(&[
            accessible("abcd", 100, 10, 140, 30),
            accessible("ef", 200, 10, 220, 30),
        ]);
        let line = line.first();
        // "abcd ef": a at 100-110, d at 130-140, the space after d is d's piece's, e at 200.
        assert_eq!(line.and_then(|l| l.point(0)), Some((105, 20)));
        assert_eq!(line.and_then(|l| l.point(3)), Some((135, 20)));
        assert_eq!(line.and_then(|l| l.point(5)), Some((205, 20)));
        assert!(line.is_some_and(|l| l.holds((210, 15))));
        assert!(line.is_some_and(|l| !l.holds((160, 15))));
    }
}
