//! W and Shift+W (MoveToNextWordFromList, MoveToPriorWordFromList): the word chosen in the Word
//! List (Ctrl+Insert+W, `crate::ui::lists::ListKind::Words`) found again in the page.
//!
//! JAWS finds the word in the virtual cursor's lines as it finds text: within other words,
//! whatever its capitals, and in what the cursor says of lists and regions. Choosing "m" in the news article moves to
//! "Primary navigation region", W then to "list of 4 items" and "Primary navigation region
//! end" [obs, 560/s6-s10].

use super::{AppMode, ScreenReaderEngine};
use crate::messages::msg;

impl ScreenReaderEngine {
    /// The line with the chosen word after the virtual cursor's (before it with `forward`
    /// false), or from its own with `here`, whatever its capitals: the cursor goes there, its
    /// line is returned. Past the page's end the search goes on from its top, said "wrapping to
    /// top", and back from its bottom: the word's only line is found again [obs, 764/s8, 766/s5,
    /// s7]. None without one.
    pub(super) fn word_from_list_line(&mut self, forward: bool, here: bool) -> Option<String> {
        let word = self.word_from_list.as_deref()?.to_lowercase();
        let lines = &self.cursor.buffer.lines;
        let current = self.cursor.current_line_index();
        let has_word = |i: &usize| {
            lines
                .get(*i)
                .is_some_and(|line| line.text.to_lowercase().contains(&word))
        };
        let (found, wrapped) = if forward {
            let from = if here { current } else { current + 1 };
            match (from..lines.len()).find(has_word) {
                Some(index) => (index, None),
                None => (
                    (0..=current).find(has_word)?,
                    Some(msg::quicknav_wrap_top()),
                ),
            }
        } else {
            match (0..current).rev().find(has_word) {
                Some(index) => (index, None),
                None => (
                    (current..lines.len()).rev().find(has_word)?,
                    Some(msg::quicknav_wrap_bottom()),
                ),
            }
        };
        self.cursor.move_to_line(found);
        let line = self.cursor.current_line();
        Some(match wrapped {
            Some(wrapped) => format!(
                "{wrapped}
{line}"
            ),
            None => line,
        })
    }

    /// W or Shift+W in a page. Before a word was chosen, JAWS says how to choose one [obs,
    /// 560/s3].
    pub(super) fn move_to_word_from_list(&mut self, forward: bool) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let said = if self.word_from_list.is_none() {
            format!("{}\n{}", msg::word_list_none(), msg::word_list_none_hint())
        } else {
            self.word_from_list_line(forward, false)?
        };
        self.speech.speak(&said, true);
        Some(said)
    }
}
