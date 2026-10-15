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
            Role::Link { href } => {
                let is_same_page = href.as_deref().map(|h| h.starts_with('#')).unwrap_or(false);
                if is_same_page {
                    format!(" \u{001d} same page link \u{001e} {}", text)
                } else {
                    format!("link {}", text)
                }
            }
            Role::Button => format!("button {}", text),
            Role::CheckBox => format!("checkbox {}", text),
            Role::Edit => format!("edit {}", text),
            Role::ComboBox => format!("combo box {}", text),
            Role::TreeItem => format!("tree item {}", text),
            Role::Slider => format!("slider {}", text),
            Role::IFrame => {
                if text.ends_with("frame end") || text.ends_with("frame") {
                    text.to_string()
                } else {
                    Self::format_frame(text)
                }
            }
            _ => text.to_string(),
        }
    }

    /// Formats an inline frame entry announcement matching JAWS conventions: "<Name> frame".
    pub fn format_frame(name: &str) -> String {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            "frame".to_string()
        } else {
            format!("{} frame", trimmed)
        }
    }

    /// Formats an inline frame exit announcement matching JAWS conventions: "<Name> frame end".
    pub fn format_frame_end(name: &str) -> String {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            "frame end".to_string()
        } else {
            format!("{} frame end", trimmed)
        }
    }

    /// Formats page title update or SPA route transition matching JAWS.
    pub fn format_page_title_change(title: &str) -> String {
        format!("Page has loaded: {}", title.trim())
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

    /// Form control speech in JAWS's focus phrasing, as spoken by F/E/X/A/C/B quick keys
    /// (spec §17, corpus/evidence/202-form-fields). `was_inside` reports whether the cursor
    /// was already inside the container with the given node id; entering a radio group
    /// names the group first.
    pub fn format_control_focus(
        node: &crate::aom::AomNode,
        was_inside: impl Fn(usize) -> bool,
    ) -> String {
        use crate::aom::Role;
        let attr = |k: &str| node.attributes.get(k).map(String::as_str);
        let name = node.name.as_deref().unwrap_or_default().trim();
        let required = attr("required") == Some("true");
        let checked = |c: Option<bool>| {
            if c == Some(true) {
                "checked"
            } else {
                "not checked"
            }
        };
        match &node.role {
            Role::Edit => {
                let password = attr("password") == Some("true");
                let kind = if password {
                    "password edit"
                } else if attr("multiline") == Some("true") {
                    "edit"
                } else {
                    "Edit"
                };
                let mut s = format!("{name} \u{1d} {kind} \u{1e} ");
                if required {
                    s.push_str("\n \u{1d}Required\u{1e} ");
                }
                if !password {
                    match node.value.as_deref().filter(|v| !v.is_empty()) {
                        Some(v) => s.push_str(&format!("\n{v}")),
                        None => s.push_str("\nblank"),
                    }
                }
                if let Some(d) = attr("description") {
                    s.push_str(&format!("\n{d}"));
                }
                s
            }
            Role::RadioButton => {
                let mut s = String::new();
                if let (Some(group), Some(group_id)) = (attr("group_name"), attr("group_id")) {
                    let inside = group_id.parse().is_ok_and(&was_inside);
                    if !inside {
                        s.push_str(&format!("{group} \u{1d} group \u{1e} \n"));
                    }
                }
                s.push_str(&format!(
                    "{name}  \u{1d} radio button \u{1e}   \u{1d}{}\u{1e} ",
                    checked(node.states.checked)
                ));
                if let (Some(p), Some(n)) = (attr("posinset"), attr("setsize")) {
                    s.push_str(&format!("\n{p} of {n}"));
                }
                s
            }
            Role::ComboBox => {
                let state = if node.states.expanded == Some(true) {
                    "expanded"
                } else {
                    "collapsed"
                };
                let mut s = format!("{name}  \u{1d} Combo box \u{1e}   \u{1d}{state}\u{1e} ");
                if let Some(v) = node.value.as_deref() {
                    s.push_str(&format!("\n{v}"));
                }
                s
            }
            Role::CheckBox => format!(
                "{name}  \u{1d} check box \u{1e}   \u{1d}{}\u{1e}",
                checked(node.states.checked)
            ),
            Role::Button => {
                // A disclosure button speaks its state [obs, 29/s1].
                let state = match node.states.expanded {
                    Some(true) => " \u{1d}expanded\u{1e}",
                    Some(false) => " \u{1d}collapsed\u{1e}",
                    None => "",
                };
                format!("{name}  \u{1d} Button \u{1e}{state}")
            }
            _ => name.to_owned(),
        }
    }

    /// The usage hint JAWS adds when focus lands on a page control [obs, 204/s7–s10].
    /// Edits, radio buttons and combo boxes get none.
    pub fn web_focus_hint(node: &crate::aom::AomNode) -> Option<&'static str> {
        use crate::aom::Role;
        match node.role {
            Role::CheckBox if node.states.checked == Some(true) => {
                Some("To clear checkmark press Spacebar.")
            }
            Role::CheckBox => Some("To check press Spacebar."),
            Role::Link { .. } | Role::Button => Some("To activate press Enter."),
            _ => None,
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
    /// Matches JAWS format: "{ColHdr} \n{CellText}\nColumn {col} of {total_cols}" or with span announcements.
    pub fn format_table_cell_col(
        text: &str,
        col: usize,
        total_cols: usize,
        col_header: Option<&str>,
        col_span: usize,
    ) -> String {
        let mut parts = Vec::new();
        if let Some(hdr) = col_header {
            if !hdr.is_empty() {
                parts.push(format!("{} ", hdr));
            }
        }
        if !text.is_empty() {
            parts.push(text.to_string());
        }
        if col_span > 1 {
            parts.push(format!("Spans {} columns", col_span));
        }
        parts.push(format!("Column {} of {}", col, total_cols));
        parts.join("\n")
    }

    /// Formats speech when moving vertically to a cell in the same column (`Ctrl+Alt+Up/Down`).
    /// Matches JAWS format: "{RowHdr}\n{CellText}\nRow {row} of {total_rows}" or with span announcements.
    pub fn format_table_cell_row(
        text: &str,
        row: usize,
        total_rows: usize,
        row_header: Option<&str>,
        row_span: usize,
        col: usize,
    ) -> String {
        let mut parts = Vec::new();
        if let Some(hdr) = row_header {
            if !hdr.is_empty() {
                parts.push(hdr.to_string());
            }
        }
        if !text.is_empty() {
            parts.push(text.to_string());
        }
        if row_span > 1 {
            parts.push(format!("cell {} of row spanning {} rows", col, row_span));
        }
        parts.push(format!("Row {} of {}", row, total_rows));
        parts.join("\n")
    }

    /// Formats full cell coordinates and headers when jumping to a cell or querying it (`Ctrl+Alt+Home`, `Ctrl+Alt+End`, `Ctrl+Alt+NumPad5`).
    /// Matches JAWS format: "{ColHdr}  {RowHdr}\n{CellText}\nColumn {col}, Row {row}".
    pub fn format_table_cell_coordinates(
        text: &str,
        col: usize,
        row: usize,
        col_header: Option<&str>,
        row_header: Option<&str>,
        col_span: usize,
        row_span: usize,
    ) -> String {
        let mut parts = Vec::new();
        match (
            col_header.filter(|h| !h.is_empty()),
            row_header.filter(|h| !h.is_empty()),
        ) {
            (Some(ch), Some(rh)) => parts.push(format!("{}  {}", ch, rh)),
            (Some(ch), None) => parts.push(ch.to_string()),
            (None, Some(rh)) => parts.push(rh.to_string()),
            (None, None) => {}
        }
        if !text.is_empty() {
            parts.push(text.to_string());
        }
        if row_span > 1 {
            parts.push(format!("cell {} of row spanning {} rows", col, row_span));
        }
        if col_span > 1 {
            parts.push(format!("Spans {} columns", col_span));
        }
        parts.push(format!("Column {}, Row {}", col, row));
        parts.join("\n")
    }

    /// Formats beginning of row announcement (`Ctrl+Alt+Shift+LeftArrow`).
    pub fn format_table_row_start(text: &str) -> String {
        format!("beginning of row\n{}", text)
    }

    /// Formats end of row announcement (`Ctrl+Alt+Shift+RightArrow`).
    pub fn format_table_row_end(text: &str) -> String {
        format!("end of row\n{}", text)
    }

    /// Formats top of column announcement (`Ctrl+Alt+Shift+UpArrow`).
    pub fn format_table_col_top(text: &str) -> String {
        format!("top of column\n{}", text)
    }

    /// Formats bottom of column announcement (`Ctrl+Alt+Shift+DownArrow`).
    pub fn format_table_col_bottom(text: &str) -> String {
        format!("bottom of column\n{}", text)
    }

    /// Formats column header direct query (`Ctrl+Alt+Shift+H`).
    pub fn format_column_header(col_hdr: Option<&str>, col: usize) -> String {
        if let Some(hdr) = col_hdr {
            if !hdr.is_empty() {
                return hdr.to_string();
            }
        }
        format!("Column {}", col)
    }

    /// Formats row header direct query (`Ctrl+Alt+Shift+R`).
    pub fn format_row_header(row_hdr: Option<&str>, row: usize) -> String {
        if let Some(hdr) = row_hdr {
            if !hdr.is_empty() {
                return hdr.to_string();
            }
        }
        format!("Row {}", row)
    }

    /// Formats mathematical equation announcement matching JAWS conventions.
    pub fn format_math(math_text: &str) -> String {
        format!("math {}", math_text)
    }

    /// The font part of Say Font: attributes, then family and size, e.g. `bold  Times New
    /// Roman 24 point` or `Times New Roman 12 point` [obs, 209/s3, s5, s7].
    pub fn font_description(formatting: &TextFormatting) -> Option<String> {
        let mut attributes = Vec::new();
        if formatting.bold {
            attributes.push("bold");
        }
        if formatting.italic {
            attributes.push("italic");
        }
        if formatting.underline {
            attributes.push("underline");
        }
        let mut font = Vec::new();
        font.extend(formatting.font_family.clone());
        font.extend(
            formatting
                .font_size_pt
                .map(|size| format!("{size:.0} point")),
        );
        if font.is_empty() {
            return None;
        }
        let font = font.join(" ");
        Some(if attributes.is_empty() {
            font
        } else {
            format!("{}  {font}", attributes.join(" "))
        })
    }

    /// Say Font (`Insert+F`): `Font is bold  Times New Roman 24 point`, or `No font` when the
    /// text has none [obs, 209/s3; 313/s1].
    pub fn format_font(formatting: Option<&TextFormatting>) -> String {
        formatting
            .and_then(Self::font_description)
            .map_or_else(|| "No font".to_owned(), |f| format!("Font is {f}"))
    }

    /// A text field's alignment as Say Font words it: "Paragraph formatting left justified"
    /// (observed; the other alignments' words are assumed to follow it).
    pub fn paragraph_formatting(alignment: &str) -> String {
        let words = match alignment {
            "left" => "left justified",
            "right" => "right justified",
            other => other,
        };
        format!("Paragraph formatting {words}")
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

    /// Formats the opening announcement of a Links List dialog matching JAWS.
    pub fn format_links_list_dialog(items: &[crate::buffer::LinkListItem]) -> String {
        let first_text = items.first().map(|i| i.text.as_str()).unwrap_or("");
        format!(
            "Links List  \u{001d} dialog \u{001e} \nlinks  \u{001d} List view \u{001e} \n{}\n1 of {}\nTo move to items use the Arrow keys.",
            first_text,
            items.len()
        )
    }

    /// Formats the opening announcement of a Headings List dialog matching JAWS.
    pub fn format_headings_list_dialog(items: &[crate::buffer::HeadingListItem]) -> String {
        let first_text = items
            .first()
            .map(|i| format!("{} : {}", i.text, i.level))
            .unwrap_or_default();
        format!(
            "Heading List  \u{001d} dialog \u{001e} \nheadings  \u{001d} List view \u{001e} \n{}\n1 of {}\nTo move to items use the Arrow keys.",
            first_text,
            items.len()
        )
    }

    /// Formats the opening announcement of a Form Fields List dialog matching JAWS.
    pub fn format_form_fields_list_dialog(items: &[crate::buffer::FormFieldListItem]) -> String {
        let first_text = items
            .first()
            .map(|i| {
                let role_label = match i.role {
                    Role::Button => "Button",
                    Role::Edit => "Edit",
                    Role::CheckBox => "check box",
                    Role::ComboBox => "combo box",
                    Role::Slider => "slider",
                    _ => "Field",
                };
                format!("{} {}", i.name, role_label)
            })
            .unwrap_or_default();
        format!(
            "Select a Form Field  \u{001d} dialog \u{001e} \nList1  \u{001d} List view \u{001e} \n{}\n1 of {}\nTo move to items use the Arrow keys.",
            first_text,
            items.len()
        )
    }

    /// Formats negative text search feedback matching JAWS.
    pub fn format_text_not_found(query: &str) -> String {
        format!("Text not found: {}", query)
    }

    /// Formats desktop folder view entry announcement. JAWS heads it with "desktop", not the
    /// list's role. JAWS 2025 on Windows 11, observed on the dev box (2026-10-15; research,
    /// scenario 316 records it on old-pc): "desktop | FolderView List view | Dropbox | 7 of 11 |
    /// To move to items use the Arrow keys. | To edit the selected item press f2."
    pub fn format_desktop_entry(
        container_name: &str,
        item_name: &str,
        index: usize,
        total: usize,
    ) -> String {
        format!(
            "desktop\n{}    List view   \n{}\n{} of {}\nTo move to items use the Arrow keys.\nTo edit the selected item press f2.",
            container_name, item_name, index, total
        )
    }

    /// Formats desktop grid item announcement matching JAWS.
    pub fn format_desktop_item(item_name: &str, index: usize, total: usize) -> String {
        format!("{}\n{} of {}", item_name, index, total)
    }

    /// Formats File Explorer list view entry announcement matching JAWS verbatim.
    pub fn format_explorer_entry(
        window_title: Option<&str>,
        folder_name: Option<&str>,
        selected: Option<bool>,
        item_name: &str,
        index: usize,
        total: usize,
    ) -> String {
        let mut lines = Vec::new();
        if let Some(title) = window_title {
            lines.push(title.to_string());
        }
        if let Some(folder) = folder_name {
            lines.push(folder.to_string());
        }
        // JAWS 2025, Windows 11, observed on the dev box (research): "live - File Explorer |
        // Items View MultiSelect ListBox | Not Selected combo.cs | 1 of 8".
        lines.push("Items View  \u{1d} MultiSelect ListBox \u{1e} ".to_string());
        match selected {
            Some(false) => lines.push(format!("Not Selected {item_name}")),
            Some(true) => {
                lines.push("selected".to_string());
                lines.push(item_name.to_string());
            }
            None => lines.push(item_name.to_string()),
        }
        lines.push(format!("{} of {}", index, total));
        lines.join("\n")
    }

    /// Formats File Explorer item navigation announcement matching JAWS.
    pub fn format_explorer_item(item_name: &str, index: usize, total: usize) -> String {
        format!("{}\n{} of {}", item_name, index, total)
    }

    /// Formats shell bar element announcement matching JAWS.
    pub fn format_shell_element(
        container: Option<&str>,
        name: &str,
        role: &str,
        help: Option<&str>,
    ) -> String {
        let mut lines = Vec::new();
        if container == Some("Taskbar") && name == "Start Button" {
            lines.push("Taskbar \n   Start Button   ".to_string());
        } else if let Some(c) = container {
            lines.push(format!("  {}    {}    Button   ", c, name));
        } else if role == "Button" {
            lines.push(format!("{}    Button   ", name));
        } else {
            lines.push(format!("{}    {}", name, role));
        }

        if let Some(h) = help {
            lines.push(h.to_string());
        }
        lines.join("\n")
    }
}

/// Trait for routing spoken utterances from the screen reader engine to audio output.
pub trait SpeechOutput: Send + Sync {
    /// Speaks the given text utterance.
    /// If `interrupt` is true, stops any ongoing speech immediately before outputting.
    fn speak(&mut self, text: &str, interrupt: bool);

    /// Immediately cancels ongoing speech.
    fn stop(&mut self);

    /// Speaks typing echo (the character just typed). A separate channel because JAWS does
    /// not record its echo in the speech history, so parity cannot compare it
    /// [obs, 305/s10–s12: audio without history text].
    fn speak_echo(&mut self, text: &str) {
        self.speak(text, true);
    }

    /// Speaks a character being read on its own (Left/Right, Delete). `text` is the character
    /// as JAWS's speech history records it; the synthesizer is given its name, as JAWS names
    /// spaces and symbols whatever its punctuation level ([`crate::pronounce`]). A synthesizer
    /// raises a capital's pitch ([`crate::pronounce::character_pitch`]).
    fn speak_character(&mut self, text: &str, interrupt: bool) {
        self.speak(&crate::pronounce::character(text), interrupt);
    }

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
