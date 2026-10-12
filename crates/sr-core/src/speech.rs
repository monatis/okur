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

    /// Formats a heading announcement for quick navigation (`H` / `1`–`6`).
    pub fn format_heading(level: u32, text: &str) -> String {
        format!("{}\n \u{001d} heading level {} \u{001e}", text, level)
    }

    /// Formats a link announcement for quick navigation (`U` / `K`).
    pub fn format_link(text: &str, is_same_page: bool) -> String {
        if is_same_page {
            format!("•  \u{001d} same page link \u{001e} {}", text)
        } else {
            format!(" \u{001d} link \u{001e} {}", text)
        }
    }

    /// Formats a list container announcement (`L`).
    pub fn format_list(item_count: usize) -> String {
        format!("List of {} items", item_count)
    }

    /// Formats a list item announcement (`I`).
    pub fn format_list_item(text: &str, has_link: bool) -> String {
        if has_link {
            format!("•  \u{001d} same page link \u{001e} {}", text)
        } else {
            text.to_string()
        }
    }

    /// Formats an ARIA landmark / region announcement (`R` / `;`).
    pub fn format_landmark(landmark_type: &str, name: Option<&str>) -> String {
        if let Some(n) = name {
            format!("{} {} region", n, landmark_type)
        } else if landmark_type == "banner" {
            "banner".to_string()
        } else {
            format!("{} region", landmark_type)
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
