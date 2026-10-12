use crate::aom::Role;
use crate::buffer::BufferLine;

/// Formats virtual buffer elements and reading events into canonical JAWS speech tokens.
pub struct SpeechFormatter;

impl SpeechFormatter {
    /// Formats a line announcement matching JAWS speech history tokens.
    pub fn format_line(line: &BufferLine) -> String {
        Self::format_role_and_text(&line.role, &line.text)
    }

    /// Formats a role prefix and text according to JAWS speech syntax.
    pub fn format_role_and_text(role: &Role, text: &str) -> String {
        match role {
            Role::Heading { level } => {
                // JAWS format: " \u001d heading level {level} \u001e {text}"
                format!(" \u{001d} heading level {} \u{001e} {}", level, text)
            }
            Role::Link { .. } => format!("link {}", text),
            Role::Button => format!("button {}", text),
            Role::CheckBox => format!("checkbox {}", text),
            _ => text.to_string(),
        }
    }

    /// Formats a single word for speech.
    pub fn format_word(word: &str) -> String {
        word.to_string()
    }

    /// Formats a single character for speech.
    pub fn format_char(ch: char) -> String {
        ch.to_string()
    }
}
