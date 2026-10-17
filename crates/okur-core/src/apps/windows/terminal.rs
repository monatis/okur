//! Consoles: Windows Terminal and the classic console window (Command Prompt, PowerShell).
//!
//! Their text area holds the console's whole buffer, read through its text pattern. JAWS's
//! Virtual Viewer shows that text (JAWS 2025 on the dev box, Windows Terminal, 2026-10-15):
//! - opening speaks "Virtualizing current window" and the line the cursor is on (the
//!   prompt);
//! - Ctrl+Home reads the console's first line: there is no window title line above it;
//! - Ctrl+End reads the last line with text (the prompt), not the empty rows below it.

use crate::aom::Role;
use crate::apps::AppModule;
use crate::editing::EditKey;
use crate::event::FocusEvent;
use crate::input::{Key, KeyEvent};

/// Windows Terminal's text area (its UIA class name).
const TERMINAL_TEXT_AREA: &str = "TermControl";
/// The classic console's window class; its text area is the document in it.
const CONSOLE_WINDOW: &str = "ConsoleWindowClass";

/// Whether the focus is a console's text area.
pub fn owns(focus: &FocusEvent) -> bool {
    focus.platform.element_class.as_deref() == Some(TERMINAL_TEXT_AREA)
        || (focus.platform.top_window_class.as_deref() == Some(CONSOLE_WINDOW)
            && focus.role == Role::Document)
}

/// The Virtual Viewer's lines for a console's text, and the line holding `caret` (a
/// character offset). Rows lose the spaces that pad them to the console's width, and the
/// empty rows below the last text are dropped.
pub fn viewer_lines(text: &str, caret: usize) -> (Vec<String>, usize) {
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|row| row.trim_end().to_owned())
        .collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    let before_caret: String = text.chars().take(caret).collect();
    let caret_line = before_caret.matches('\n').count();
    let last = lines.len().saturating_sub(1);
    (lines, caret_line.min(last))
}

/// The line holding `caret`, without the row's padding.
pub fn cursor_line(text: &str, caret: usize) -> String {
    let (lines, line) = viewer_lines(text, caret);
    lines.into_iter().nth(line).unwrap_or_default()
}

/// Consoles.
pub struct Terminal;

impl AppModule for Terminal {
    fn owns(&self, focus: &FocusEvent) -> bool {
        owns(focus)
    }

    fn virtualizes_text(&self) -> bool {
        true
    }

    fn viewer_lines(&self, text: &str, caret: usize) -> Option<(Vec<String>, usize)> {
        Some(viewer_lines(text, caret))
    }

    fn line_after_viewer(&self, text: &str, caret: usize) -> Option<String> {
        Some(cursor_line(text, caret))
    }

    fn types_into(&self, field: &FocusEvent) -> bool {
        owns(field)
    }

    /// Escape clears the input line (PowerShell's and Command Prompt's), and JAWS reads the
    /// line then: the prompt (Windows Terminal, JAWS on the dev box, 2026-10-17).
    fn edit_key(&self, _field: &FocusEvent, key: &KeyEvent) -> Option<EditKey> {
        (key.key == Key::Escape && key.modifiers.is_empty()).then_some(EditKey::Line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_in_a_console_reads_its_line() {
        use crate::input::Modifiers;
        let field = FocusEvent::new(Role::Text);
        let escape = KeyEvent::new(Key::Escape, Modifiers::none(), true);
        assert_eq!(Terminal.edit_key(&field, &escape), Some(EditKey::Line));
        let mut shift_escape = escape.clone();
        shift_escape.modifiers.shift = true;
        assert_eq!(Terminal.edit_key(&field, &shift_escape), None);
    }

    #[test]
    fn a_terminal_text_area_and_a_classic_console_are_consoles() {
        let mut terminal = FocusEvent::new(Role::Text);
        terminal.platform.element_class = Some("TermControl".to_owned());
        assert!(owns(&terminal));

        let mut classic = FocusEvent::new(Role::Document).with_name("Text Area");
        classic.platform.top_window_class = Some("ConsoleWindowClass".to_owned());
        assert!(owns(&classic));

        let mut notepad = FocusEvent::new(Role::Document);
        notepad.platform.top_window_class = Some("Notepad".to_owned());
        assert!(!owns(&notepad));
    }

    #[test]
    fn rows_lose_their_padding_and_the_viewer_starts_at_the_prompt() {
        // As Windows Terminal's text pattern gives it: rows padded to the width, CRLF.
        let text = "Hello from the console        \r\n\
                    Second line                   \r\n\
                    \x20                             \r\n\
                    C:\\Users\\Gigabyte>            \r\n\
                    \x20                             \r\n";
        let caret = text.find("C:\\").unwrap_or_default() + "C:\\Users\\Gigabyte>".len();
        let (lines, line) = viewer_lines(text, caret);
        assert_eq!(
            lines,
            vec![
                "Hello from the console",
                "Second line",
                "",
                "C:\\Users\\Gigabyte>"
            ]
        );
        assert_eq!(line, 3);
    }
}
