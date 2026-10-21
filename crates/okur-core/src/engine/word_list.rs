//! W and Shift+W (MoveToNextWordFromList, MoveToPriorWordFromList): the word chosen in the Word
//! List (Ctrl+Insert+W, `crate::ui::lists::ListKind::Words`) found again in the page.
//!
//! JAWS finds the word in the virtual cursor's lines as it finds text: within other words and
//! in what the cursor says of lists and regions. Choosing "m" in the news article moves to
//! "Primary navigation region", W then to "list of 4 items" and "Primary navigation region
//! end" [obs, 560/s6-s10].

use super::{AppMode, ScreenReaderEngine};
use crate::messages::msg;

impl ScreenReaderEngine {
    /// The line with the chosen word after the virtual cursor's (before it with `forward`
    /// false), or from its own with `here`: the cursor goes there, its line is returned. None
    /// without one.
    pub(super) fn word_from_list_line(&mut self, forward: bool, here: bool) -> Option<String> {
        let word = self.word_from_list.as_deref()?;
        let lines = &self.cursor.buffer.lines;
        let current = self.cursor.current_line_index();
        let has_word = |i: &usize| lines.get(*i).is_some_and(|line| line.text.contains(word));
        let index = if forward {
            let from = if here { current } else { current + 1 };
            (from..lines.len()).find(has_word)
        } else {
            (0..current).rev().find(has_word)
        }?;
        self.cursor.move_to_line(index);
        Some(self.cursor.current_line())
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
