//! A console's new text, read as the console writes it while Screen Echo is on: after Enter,
//! the command's output and the next prompt [obs, 962/s2, 964/s2]; nothing with Screen Echo off
//! [obs, 963/s2] ([`crate::screen_echo`]).

use super::ScreenReaderEngine;
use crate::event::FocusEvent;
use crate::settings::ScreenEcho;

/// How many of a screen's last rows are kept to compare its next read with.
const KEPT_ROWS: usize = 300;

impl ScreenReaderEngine {
    /// The rows of the screen `focus`'s application writes to, from `text`; `None` where it
    /// writes none.
    fn screen_rows_of(&self, focus: &FocusEvent, text: &str) -> Option<Vec<String>> {
        let mut rows = self.app_for(focus)?.screen_rows(focus, text)?;
        let first_kept = rows.len().saturating_sub(KEPT_ROWS);
        rows.drain(..first_kept);
        Some(rows)
    }

    /// A focus arrived: a console's screen as it is now is what its next writing is compared
    /// with; anything else has none.
    pub(super) fn note_screen_rows(&mut self, focus: &FocusEvent) {
        self.screen_rows = focus
            .text
            .as_ref()
            .and_then(|text| self.screen_rows_of(focus, &text.text));
    }

    /// The console in front wrote to its screen, now `text`: its new rows are said while
    /// Screen Echo is on, after what is being said.
    pub(super) fn console_text(&mut self, text: &str) -> Option<String> {
        let focus = self.last_native_focus.as_ref()?;
        let rows = self.screen_rows_of(focus, text)?;
        let before = self.screen_rows.replace(rows)?;
        let after = self.screen_rows.as_deref().unwrap_or_default();
        let new = crate::screen_echo::new_rows(&before, after);
        if new.is_empty() || self.settings.screen_echo == ScreenEcho::None {
            return None;
        }
        let said = new.join("\n");
        self.speech.speak(&said, false);
        Some(said)
    }
}
