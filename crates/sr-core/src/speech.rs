use crate::aom::{AomNode, Role, TextFormatting};
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
            Role::ComboBox => format!("combo box {}", text),
            Role::TreeItem => format!("tree item {}", text),
            Role::Slider => format!("slider {}", text),
            Role::ProgressBar => format!("progress bar {}", text),
            _ => text.to_string(),
        }
    }

    fn format_number(val: f64) -> String {
        if val.fract() == 0.0 {
            format!("{:.0}", val)
        } else {
            format!("{}", val)
        }
    }

    /// Formats an ARIA combobox announcement matching JAWS conventions.
    /// Canonical JAWS pattern: "<Label>  \u{001d} combo box \u{001e} <Value> \u{001d} <collapsed|expanded> \u{001e}"
    pub fn format_combobox(name: &str, value: Option<&str>, expanded: bool) -> String {
        let state_str = if expanded { "expanded" } else { "collapsed" };
        let val_str = value.unwrap_or("").trim();
        if val_str.is_empty() {
            format!(
                "{}  \u{001d} combo box \u{001e} \u{001d} {} \u{001e}",
                name, state_str
            )
        } else {
            format!(
                "{}  \u{001d} combo box \u{001e} {} \u{001d} {} \u{001e}",
                name, val_str, state_str
            )
        }
    }

    /// Formats an ARIA treeitem announcement matching JAWS conventions.
    /// Canonical JAWS pattern: "<Name>  \u{001d} level <N> \u{001e} <collapsed|expanded>"
    pub fn format_tree_item(name: &str, level: u32, expanded: Option<bool>) -> String {
        match expanded {
            Some(true) => format!("{}  \u{001d} level {} \u{001e} expanded", name, level),
            Some(false) => format!("{}  \u{001d} level {} \u{001e} collapsed", name, level),
            None => format!("{}  \u{001d} level {} \u{001e}", name, level),
        }
    }

    /// Formats an ARIA slider announcement matching JAWS conventions.
    /// Canonical JAWS pattern: "<Name>  \u{001d} slider \u{001e} <value>"
    pub fn format_slider(name: &str, value: f64) -> String {
        format!(
            "{}  \u{001d} slider \u{001e} {}",
            name,
            Self::format_number(value)
        )
    }

    /// Formats an ARIA progress bar announcement matching JAWS conventions.
    /// Canonical JAWS pattern: "<Name>  \u{001d} progress bar \u{001e} <value>%"
    pub fn format_progressbar(name: &str, value: f64) -> String {
        format!(
            "{}  \u{001d} progress bar \u{001e} {}%",
            name,
            Self::format_number(value)
        )
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

    /// Formats a push button with expansion state and optional document title.
    pub fn format_button_interactive(
        name: &str,
        expanded: Option<bool>,
        doc_title: Option<&str>,
    ) -> String {
        let mut parts = Vec::new();
        if let Some(title) = doc_title {
            if !title.is_empty() {
                parts.push(title.to_string());
            }
        }
        let exp_str = match expanded {
            Some(true) => "   \u{001d}expanded\u{001e}  \nTo activate press Enter.",
            Some(false) => "   \u{001d}collapsed\u{001e}  \nTo activate press Enter.",
            None => "",
        };
        parts.push(format!("{}  \u{001d} Button \u{001e}{}", name, exp_str));
        parts.join("\n")
    }

    /// Formats an ARIA tab announcement matching JAWS conventions.
    pub fn format_tab(
        name: &str,
        selected: bool,
        index: usize,
        total: usize,
        group_name: Option<&str>,
        announce_group: bool,
    ) -> String {
        let mut parts = Vec::new();
        if announce_group {
            if let Some(grp) = group_name {
                if !grp.is_empty() {
                    parts.push(format!("{} \u{001d} group \u{001e}", grp));
                }
            }
        }
        parts.push(format!("{}  \u{001d} Tab \u{001e}", name));
        parts.push(format!("{} of {}", index, total));
        if selected {
            parts.push(" \u{001d}selected\u{001e}".to_string());
        }
        parts.join("\n")
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
            Role::ComboBox => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let value = node.value.as_deref();
                let expanded = node.states.expanded.unwrap_or(false);
                Self::format_combobox(name, value, expanded)
            }
            Role::Slider => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let value = node.value_now.unwrap_or(0.0);
                Self::format_slider(name, value)
            }
            Role::ProgressBar => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let value = node.value_now.unwrap_or(0.0);
                Self::format_progressbar(name, value)
            }
            Role::TreeItem => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let level = node.level.unwrap_or(1);
                Self::format_tree_item(name, level, node.states.expanded)
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

    /// Formats font formatting attributes into canonical JAWS description string.
    /// Format: "{size} pt, {styles}, {family} on default background, {alignment} aligned"
    pub fn format_font(formatting: &TextFormatting) -> String {
        let mut parts = Vec::new();

        if let Some(size) = formatting.font_size_pt {
            parts.push(format!("{:.0} pt", size));
        }

        if formatting.bold {
            parts.push("bold".to_string());
        }
        if formatting.italic {
            parts.push("italic".to_string());
        }
        if formatting.underline {
            parts.push("underline".to_string());
        }

        if let Some(ref family) = formatting.font_family {
            parts.push(format!("{} on default background", family));
        }

        if let Some(ref align) = formatting.alignment {
            let align_str = if align.to_lowercase().contains("align") {
                align.clone()
            } else {
                format!("{} aligned", align)
            };
            parts.push(align_str);
        }

        if parts.is_empty() {
            "No font".to_string()
        } else {
            parts.join(", ")
        }
    }

    /// Formats selection action announcement matching JAWS conventions.
    pub fn format_selection(selected: bool, text: &str) -> String {
        if selected {
            format!("selected {}", text)
        } else {
            format!("unselected {}", text)
        }
    }

    /// Formats modal dialog entry announcement.
    pub fn format_dialog_entry(title: &str) -> String {
        if title.is_empty() {
            "Dialog".to_string()
        } else {
            format!("Dialog {}", title)
        }
    }

    /// Formats modal dialog exit announcement.
    pub fn format_dialog_exit() -> String {
        "Leaving Dialog".to_string()
    }
}

/// Trait for routing spoken utterances from the screen reader engine to audio output.
pub trait SpeechOutput: Send + Sync {
    /// Speaks the given text utterance.
    /// If `interrupt` is true, stops any ongoing speech immediately before outputting.
    fn speak(&mut self, text: &str, interrupt: bool);

    /// Immediately cancels ongoing speech.
    fn stop(&mut self);

    /// Returns whether speech output is currently active.
    fn is_speaking(&self) -> bool {
        false
    }
}

/// Mock speech synthesizer output for testing and headless execution.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MockSpeechOutput {
    pub utterances: Vec<String>,
}

impl MockSpeechOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last_spoken(&self) -> Option<&str> {
        self.utterances.last().map(|s| s.as_str())
    }

    pub fn clear(&mut self) {
        self.utterances.clear();
    }
}

impl SpeechOutput for MockSpeechOutput {
    fn speak(&mut self, text: &str, _interrupt: bool) {
        self.utterances.push(text.to_string());
    }

    fn stop(&mut self) {}
}
