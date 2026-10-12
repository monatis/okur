use crate::aom::{AomNode, Role};
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
            Role::Edit => format!("edit {}", text),
            _ => text.to_string(),
        }
    }

    /// Formats an edit control announcement.
    pub fn format_edit(name: &str, value: Option<&str>) -> String {
        let val_str = value.unwrap_or("");
        if val_str.is_empty() {
            format!("{}  \u{001d} Edit \u{001e}", name)
        } else {
            format!("{}  \u{001d} Edit \u{001e}  \n{}", name, val_str)
        }
    }

    /// Formats a push button announcement.
    pub fn format_button(name: &str) -> String {
        format!("{}  \u{001d} Button \u{001e}", name)
    }

    /// Formats a checkbox announcement.
    pub fn format_checkbox(name: &str, checked: bool) -> String {
        let state_str = if checked { "checked" } else { "not checked" };
        format!(
            " {}  \u{001d} check box \u{001e}   \u{001d}{}\u{001e}",
            name, state_str
        )
    }

    /// Formats speech for a generic form control AOM node.
    pub fn format_form_control_node(node: &AomNode) -> String {
        match node.role {
            Role::Edit => {
                let name = node.name.as_deref().unwrap_or("");
                let value = node.value.as_deref();
                Self::format_edit(name, value)
            }
            Role::Button => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                Self::format_button(name)
            }
            Role::CheckBox => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let checked = node.states.checked.unwrap_or(false);
                Self::format_checkbox(name, checked)
            }
            _ => String::new(),
        }
    }

    /// Formats announcement when entering Forms Mode on an edit field.
    pub fn format_forms_mode_entry(
        doc_title: &str,
        field_name: &str,
        value: Option<&str>,
    ) -> String {
        let val_str = value.unwrap_or("");
        if doc_title.is_empty() {
            if val_str.is_empty() {
                format!("{}  \u{001d} Edit \u{001e}  \nType in text.", field_name)
            } else {
                format!(
                    "{}  \u{001d} Edit \u{001e}   \n{}\nType in text.",
                    field_name, val_str
                )
            }
        } else if val_str.is_empty() {
            format!(
                "{} \n \u{001d} FormRegion \u{001e} \n{}  \u{001d} Edit \u{001e}\nType in text.",
                doc_title, field_name
            )
        } else {
            format!(
                "{} \n \u{001d} FormRegion \u{001e} \n{}  \u{001d} Edit \u{001e}   \n{}\nType in text.",
                doc_title, field_name, val_str
            )
        }
    }

    /// Formats announcement when exiting Forms Mode back to Virtual PC Cursor.
    pub fn format_forms_mode_exit() -> String {
        "virtual PC Cursor".to_string()
    }

    /// Formats announcement when toggling a checkbox via Spacebar.
    pub fn format_checkbox_toggle(doc_title: &str, name: &str, checked: bool) -> String {
        let state_str = if checked { "checked" } else { "not checked" };
        let action_hint = if checked {
            "To clear checkmark press Spacebar."
        } else {
            "To check press Spacebar."
        };
        if doc_title.is_empty() {
            format!(
                "{}  \u{001d} check box \u{001e}   \u{001d}{}\u{001e}  \n{}",
                name, state_str, action_hint
            )
        } else {
            format!(
                "{} \n \u{001d} FormRegion \u{001e} \n{}  \u{001d} check box \u{001e}   \u{001d}{}\u{001e}  \n{}",
                doc_title, name, state_str, action_hint
            )
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

    /// Formats a table entry announcement (`T`).
    /// Format: "table with {cols} columns and {rows} rows\n{caption}\nColumn 1, Row 1\n{FirstCellText}"
    pub fn format_table_entry(
        cols: usize,
        rows: usize,
        caption: Option<&str>,
        col: usize,
        row: usize,
        cell_text: &str,
    ) -> String {
        let mut parts = Vec::new();
        parts.push(format!("table with {} columns and {} rows", cols, rows));
        if let Some(cap) = caption {
            if !cap.is_empty() {
                parts.push(cap.to_string());
            }
        }
        parts.push(format!("Column {}, Row {}", col, row));
        if !cell_text.is_empty() {
            parts.push(cell_text.to_string());
        }
        parts.join("\n")
    }

    /// Formats speech when moving horizontally to a cell in the same row (`Ctrl+Alt+Left/Right`).
    /// Format: "{CellText}\nColumn {col} of {total_cols}"
    pub fn format_table_cell_col(text: &str, col: usize, total_cols: usize) -> String {
        format!("{}\nColumn {} of {}", text, col, total_cols)
    }

    /// Formats speech when moving vertically to a cell in the same column (`Ctrl+Alt+Up/Down`).
    /// Format: "{CellText}\nRow {row} of {total_rows}"
    pub fn format_table_cell_row(text: &str, row: usize, total_rows: usize) -> String {
        format!("{}\nRow {} of {}", text, row, total_rows)
    }
}
