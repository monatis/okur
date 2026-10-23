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
        }
    }

    fn middle(&self) -> (i32, i32) {
        (
            self.left + (self.right - self.left) / 2,
            self.top + (self.bottom - self.top) / 2,
        )
    }

    fn contains(&self, (x, y): (i32, i32)) -> bool {
        (self.left..self.right.max(self.left + 1)).contains(&x)
            && (self.top..self.bottom.max(self.top + 1)).contains(&y)
    }
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
}

impl ScreenLine {
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
        let width = piece.right - piece.left;
        let x = piece.left + width * index / chars + width / chars / 2;
        Some((x, piece.top + (piece.bottom - piece.top) / 2))
    }

    /// Whether character `column` was recognized from the screen's pixels.
    pub fn recognized_at(&self, column: usize) -> bool {
        self.piece_at(column)
            .is_some_and(|piece| piece.source == Source::Recognized)
    }

    /// Whether the line has a piece where `point` is.
    pub fn holds(&self, (x, y): (i32, i32)) -> bool {
        self.pieces.iter().any(|piece| {
            (piece.left..piece.right.max(piece.left + 1)).contains(&x)
                && (piece.top..piece.bottom.max(piece.top + 1)).contains(&y)
        })
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
            None => rows.push((run.top, run.bottom.max(run.top + 1), vec![run])),
        }
    }
    rows.into_iter()
        .map(|(_, _, mut row)| {
            row.sort_by_key(|r| r.left);
            let mut line = ScreenLine {
                text: String::new(),
                pieces: Vec::with_capacity(row.len()),
            };
            for run in row.into_iter().filter(|r| !r.text.is_empty()) {
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

/// A window's model: what the accessibility interfaces place on it, and what was recognized
/// from its pixels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScreenModel {
    /// The window it is of, by its title, as the engine knows the window in front.
    pub window: Option<String>,
    pub accessible: Vec<ScreenRun>,
    pub recognized: Vec<ScreenRun>,
}

impl ScreenModel {
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

    /// The model's lines.
    pub fn lines(&self) -> Vec<ScreenLine> {
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
        let model = ScreenModel {
            window: Some("SR Drawn Text Fixture".to_owned()),
            accessible: vec![
                accessible("Label drawn by GDI", 20, 16, 240, 48),
                accessible("Focus here", 20, 340, 320, 380),
            ],
            recognized: vec![
                recognized("Label", 22, 20, 80, 44),
                recognized("Painted", 20, 80, 110, 108),
                recognized("text", 120, 80, 170, 108),
                recognized("Picture", 24, 200, 110, 230),
                recognized("Focus", 24, 346, 90, 372),
                recognized("here", 96, 346, 150, 372),
            ],
        };
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
