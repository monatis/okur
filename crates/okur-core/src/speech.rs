use crate::announcement::{Announcement, Line, Part, PartKind};
use crate::aom::{AomNode, Role, TextFormatting};
use crate::buffer::BufferLine;
use crate::messages::msg;

/// A name, a space, and a role: "Name  ‹ check box ›", JAWS's spacing on focus.
fn name_and_role(name: &str, role: &str) -> Vec<Part> {
    vec![
        Part::new(PartKind::Name, name),
        Part::text(" "),
        Part::role(role),
    ]
}

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
                format!(
                    " \u{001d} {} \u{001e} {}",
                    msg::role_heading_level(*level),
                    text
                )
            }
            Role::Link { href } => {
                let is_same_page = href.as_deref().map(|h| h.starts_with('#')).unwrap_or(false);
                if is_same_page {
                    format!(" \u{001d} {} \u{001e} {}", msg::role_link_same_page(), text)
                } else {
                    format!(" \u{001d} {} \u{001e} {}", msg::role_link(), text)
                }
            }
            Role::Button => format!("{} {}", msg::role_button_line(), text),
            Role::CheckBox => format!("{} {}", msg::role_check_box_line(), text),
            Role::Edit => format!("{} {}", msg::role_edit_multiline(), text),
            Role::ComboBox => format!("{} {}", msg::role_combo_box(), text),
            Role::TreeItem => format!("{} {}", msg::role_tree_item(), text),
            Role::Slider => format!("{} {}", msg::role_slider(), text),
            Role::IFrame => {
                if Self::is_frame_line(text) {
                    text.to_string()
                } else {
                    Self::format_frame(text)
                }
            }
            _ => text.to_string(),
        }
    }

    /// Whether a line's text already says it is a frame's ("Comments frame", "frame end").
    pub fn is_frame_line(text: &str) -> bool {
        text.ends_with(msg::frame_end().as_str()) || text.ends_with(msg::frame().as_str())
    }

    /// Formats an inline frame entry announcement matching JAWS conventions: `"<Name> frame"`.
    pub fn format_frame(name: &str) -> String {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            msg::frame().to_string()
        } else {
            msg::frame_named(trimmed).render()
        }
    }

    /// Formats page title update or SPA route transition matching JAWS.
    pub fn format_page_title_change(title: &str) -> String {
        msg::page_loaded(title.trim()).render()
    }

    fn format_number(val: f64) -> String {
        if val.fract() == 0.0 {
            format!("{:.0}", val)
        } else {
            format!("{}", val)
        }
    }

    /// Formats an ARIA combobox announcement matching JAWS conventions.
    /// Canonical JAWS pattern: `"<Label>  \u{001d} combo box \u{001e} <Value> \u{001d} <collapsed|expanded> \u{001e}"`
    pub fn format_combobox(name: &str, value: Option<&str>, expanded: bool) -> String {
        let state_str = if expanded {
            msg::state_expanded()
        } else {
            msg::state_collapsed()
        };
        let role = msg::role_combo_box();
        let val_str = value.unwrap_or("").trim();
        if val_str.is_empty() {
            format!(
                "{}  \u{001d} {} \u{001e} \u{001d} {} \u{001e}",
                name, role, state_str
            )
        } else {
            format!(
                "{}  \u{001d} {} \u{001e} {} \u{001d} {} \u{001e}",
                name, role, val_str, state_str
            )
        }
    }

    /// Formats an ARIA treeitem announcement matching JAWS conventions.
    /// Canonical JAWS pattern: `"<Name>  \u{001d} level <N> \u{001e} <collapsed|expanded>"`
    pub fn format_tree_item(name: &str, level: u32, expanded: Option<bool>) -> String {
        let level = msg::role_tree_level(level);
        match expanded {
            Some(true) => format!(
                "{}  \u{001d} {} \u{001e} {}",
                name,
                level,
                msg::state_expanded()
            ),
            Some(false) => format!(
                "{}  \u{001d} {} \u{001e} {}",
                name,
                level,
                msg::state_collapsed()
            ),
            None => format!("{}  \u{001d} {} \u{001e}", name, level),
        }
    }

    /// Formats an ARIA slider announcement matching JAWS conventions.
    /// Canonical JAWS pattern: `"<Name>  \u{001d} slider \u{001e} <value>"`
    pub fn format_slider(name: &str, value: f64) -> String {
        format!(
            "{}  \u{001d} {} \u{001e} {}",
            name,
            msg::role_slider(),
            Self::format_number(value)
        )
    }

    /// Formats an ARIA progress bar announcement matching JAWS conventions.
    /// Canonical JAWS pattern: `"<Name>  \u{001d} progress bar \u{001e} <value>%"`
    pub fn format_progressbar(name: &str, value: f64) -> String {
        format!(
            "{}  \u{001d} {} \u{001e} {}",
            name,
            msg::role_progress_bar(),
            msg::progress_percent(Self::format_number(value))
        )
    }

    /// Formats an edit control announcement.
    pub fn format_edit(name: &str, value: Option<&str>) -> String {
        let val_str = value.unwrap_or("");
        let role = msg::role_edit();
        if val_str.is_empty() {
            format!("{}  \u{001d} {} \u{001e}", name, role)
        } else {
            format!("{}  \u{001d} {} \u{001e}  \n{}", name, role, val_str)
        }
    }

    /// Formats a push button announcement.
    pub fn format_button(name: &str) -> String {
        format!("{}  \u{001d} {} \u{001e}", name, msg::role_button())
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
        let state = match expanded {
            Some(true) => Some(msg::state_expanded()),
            Some(false) => Some(msg::state_collapsed()),
            None => None,
        };
        let exp_str = state
            .map(|state| {
                format!(
                    "   \u{001d}{}\u{001e}  \n{}",
                    state,
                    msg::hint_activate_enter()
                )
            })
            .unwrap_or_default();
        parts.push(format!(
            "{}  \u{001d} {} \u{001e}{}",
            name,
            msg::role_button(),
            exp_str
        ));
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
                    parts.push(format!("{} \u{001d} {} \u{001e}", grp, msg::role_group()));
                }
            }
        }
        parts.push(format!("{}  \u{001d} {} \u{001e}", name, msg::role_tab()));
        parts.push(msg::position(index, total).render());
        if selected {
            parts.push(format!(" \u{001d}{}\u{001e}", msg::state_selected()));
        }
        parts.join("\n")
    }

    /// Formats a checkbox announcement.
    pub fn format_checkbox(name: &str, checked: bool) -> String {
        let state_str = if checked {
            msg::state_checked()
        } else {
            msg::state_not_checked()
        };
        format!(
            " {}  \u{001d} {} \u{001e}   \u{001d}{}\u{001e}",
            name,
            msg::role_check_box(),
            state_str
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
                if node.attributes.get("editable").is_some_and(|e| e == "true") {
                    return Self::format_edit_combo(name, node);
                }
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
        let edit = msg::role_edit();
        let hint = msg::hint_type_in_text();
        let region = msg::role_region_form();
        if doc_title.is_empty() {
            if val_str.is_empty() {
                format!("{field_name}  \u{001d} {edit} \u{001e}  \n{hint}")
            } else {
                format!("{field_name}  \u{001d} {edit} \u{001e}   \n{val_str}\n{hint}")
            }
        } else if val_str.is_empty() {
            format!(
                "{doc_title} \n \u{001d} {region} \u{001e} \n{field_name}  \u{001d} {edit} \u{001e}\n{hint}"
            )
        } else {
            format!(
                "{doc_title} \n \u{001d} {region} \u{001e} \n{field_name}  \u{001d} {edit} \u{001e}   \n{val_str}\n{hint}"
            )
        }
    }

    /// Formats announcement when exiting Forms Mode back to Virtual PC Cursor.
    pub fn format_forms_mode_exit() -> String {
        msg::cursor_virtual_pc().to_string()
    }

    /// Formats announcement when toggling a checkbox via Spacebar.
    pub fn format_checkbox_toggle(doc_title: &str, name: &str, checked: bool) -> String {
        let state_str = if checked {
            msg::state_checked()
        } else {
            msg::state_not_checked()
        };
        let action_hint = if checked {
            msg::hint_clear_checkmark_spacebar()
        } else {
            msg::hint_check_spacebar()
        };
        let role = msg::role_check_box();
        if doc_title.is_empty() {
            format!(
                "{name}  \u{001d} {role} \u{001e}   \u{001d}{state_str}\u{001e}  \n{action_hint}"
            )
        } else {
            format!(
                "{doc_title} \n \u{001d} {} \u{001e} \n{name}  \u{001d} {role} \u{001e}   \u{001d}{state_str}\u{001e}  \n{action_hint}",
                msg::role_region_form()
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
        Self::control_focus(node, was_inside).render_all()
    }

    /// [`Self::format_control_focus`] as parts with their kinds. A line whose spacing parts
    /// cannot give (a check box's, a button's) is one part of kind `Text`.
    pub fn control_focus(
        node: &crate::aom::AomNode,
        was_inside: impl Fn(usize) -> bool,
    ) -> Announcement {
        use crate::aom::Role;
        let attr = |k: &str| node.attributes.get(k).map(String::as_str);
        let name = node.name.as_deref().unwrap_or_default().trim();
        let required = attr("required") == Some("true");
        let checked = |c: Option<bool>| {
            if c == Some(true) {
                msg::state_checked()
            } else {
                msg::state_not_checked()
            }
        };
        match &node.role {
            Role::Edit => {
                let password = attr("password") == Some("true");
                let kind = if password {
                    msg::role_edit_password().as_str()
                } else if let Some(description) = node.role_description() {
                    description
                } else if attr("readonly") == Some("true") {
                    msg::role_edit_read_only().as_str()
                } else if attr("multiline") == Some("true") {
                    msg::role_edit_multiline().as_str()
                } else {
                    msg::role_edit().as_str()
                };
                let mut lines = vec![Line::new(vec![
                    Part::new(PartKind::Name, name),
                    Part::role(kind),
                ])];
                if required {
                    lines.push(Line::new(vec![Part::state(msg::state_required())]));
                }
                // A multi-line field with text says so, not its text (a text area, a web code
                // editor): "Notes edit | Contains text" (JAWS on the dev box). A field holding
                // only a line break is blank: WhatsApp's empty composer (JAWS on the dev box).
                if !password {
                    lines.push(Line::of(
                        PartKind::Value,
                        match node.value.as_deref().filter(|v| !v.trim().is_empty()) {
                            Some(_) if attr("multiline") == Some("true") => {
                                msg::field_contains_text().as_str()
                            }
                            Some(v) => v,
                            None => msg::blank().as_str(),
                        },
                    ));
                }
                if let Some(d) = attr("description") {
                    lines.push(Line::of(PartKind::Description, d));
                }
                Announcement::new(lines)
            }
            Role::RadioButton => {
                let mut lines = Vec::new();
                if let (Some(group), Some(group_id)) = (attr("group_name"), attr("group_id")) {
                    let inside = group_id.parse().is_ok_and(&was_inside);
                    if !inside {
                        lines.push(Line::new(vec![
                            Part::new(PartKind::Container, group),
                            Part::role(msg::role_group()),
                        ]));
                    }
                }
                let mut parts = name_and_role(name, msg::role_radio_button().as_str());
                parts.push(Part::text(" "));
                parts.push(Part::state(checked(node.states.checked)));
                lines.push(Line::new(parts));
                if let (Some(p), Some(n)) = (attr("posinset"), attr("setsize")) {
                    lines.push(Line::of(PartKind::Position, msg::position(p, n)));
                }
                Announcement::new(lines)
            }
            Role::ComboBox if attr("editable") == Some("true") => Self::edit_combo(name, node),
            Role::ComboBox => {
                let state = if node.states.expanded == Some(true) {
                    msg::state_expanded()
                } else {
                    msg::state_collapsed()
                };
                let mut parts = name_and_role(name, msg::role_combo_box_focus().as_str());
                parts.push(Part::text(" "));
                parts.push(Part::state(state));
                let mut lines = vec![Line::new(parts)];
                if let Some(v) = node.value.as_deref() {
                    lines.push(Line::of(PartKind::Value, v));
                }
                Announcement::new(lines)
            }
            Role::CheckBox => Announcement::from_text(&format!(
                "{name}  \u{1d} {} \u{1e}   \u{1d}{}\u{1e}",
                node.role_description()
                    .unwrap_or(msg::role_check_box().as_str()),
                checked(node.states.checked)
            )),
            // A toggle button (`aria-pressed`): "Sohbetler Toggle button Pressed", "👍 Toggle
            // button Not pressed" (WhatsApp, JAWS on the dev box, 2026-10-17).
            Role::Button if node.states.checked.is_some() => {
                let pressed = if node.states.checked == Some(true) {
                    msg::state_pressed()
                } else {
                    msg::state_not_pressed()
                };
                let mut parts = name_and_role(name, msg::role_toggle_button().as_str());
                parts.push(Part::state(pressed));
                Announcement::new(vec![Line::new(parts)])
            }
            Role::Button => {
                // A disclosure button speaks its state [obs, 29/s1].
                let state = match node.states.expanded {
                    Some(true) => format!(" \u{1d}{}\u{1e}", msg::state_expanded()),
                    Some(false) => format!(" \u{1d}{}\u{1e}", msg::state_collapsed()),
                    None => String::new(),
                };
                let current = node
                    .current_phrase()
                    .map(|c| format!(" \u{1d}{c}\u{1e}"))
                    .unwrap_or_default();
                let popup = node
                    .attributes
                    .get("haspopup")
                    .map(|p| format!(" \u{1d}{}\u{1e} {p}", msg::state_has_popup()))
                    .unwrap_or_default();
                let role = node
                    .role_description()
                    .unwrap_or(msg::role_button().as_str());
                Announcement::from_text(&format!(
                    "{name}  \u{1d} {role} \u{1e}{state}{popup}{current}"
                ))
            }
            // On focus a menu button says no state and no hint (JAWS on the dev box).
            Role::MenuButton => {
                let role = node
                    .role_description()
                    .unwrap_or(msg::role_button_menu().as_str());
                Announcement::new(vec![Line::new(name_and_role(name, role))])
            }
            _ => Announcement::new(vec![Line::of(PartKind::Name, name)]),
        }
    }

    /// The usage hint JAWS adds when focus lands on a page control [obs, 204/s7–s10].
    /// Edits, radio buttons and combo boxes get none.
    pub fn web_focus_hint(node: &crate::aom::AomNode) -> Option<&'static str> {
        use crate::aom::Role;
        match node.role {
            Role::CheckBox if node.states.checked == Some(true) => {
                Some(msg::hint_clear_checkmark_spacebar().as_str())
            }
            Role::CheckBox => Some(msg::hint_check_spacebar().as_str()),
            Role::Button if node.states.checked.is_some() => {
                Some(msg::hint_toggle_spacebar().as_str())
            }
            Role::Link { .. } | Role::Button => Some(msg::hint_activate_enter().as_str()),
            _ => None,
        }
    }

    /// Formats a heading announcement for quick navigation (`H` / `1`–`6`).
    pub fn format_heading(level: u32, text: &str) -> String {
        format!(
            "{}\n \u{001d} {} \u{001e}",
            text,
            msg::role_heading_level(level)
        )
    }

    /// A higher heading a level key passes, said on one line before the heading it reaches:
    /// "Reference ‹ heading level 1 ›" [obs, 275/s3].
    pub fn format_passed_heading(level: u32, text: &str) -> String {
        format!(
            "{text}{}",
            crate::linearize::role_marker(&msg::role_heading_level(level).render())
        )
    }

    /// A combo box one types in (an autocomplete): "Fruit edit combo collapsed | blank" (JAWS
    /// on the dev box), on focus and at E.
    pub fn format_edit_combo(name: &str, node: &AomNode) -> String {
        Self::edit_combo(name, node).render_all()
    }

    /// [`Self::format_edit_combo`] as parts with their kinds.
    pub fn edit_combo(name: &str, node: &AomNode) -> Announcement {
        let mut parts = name_and_role(name, msg::role_edit_combo().as_str());
        // One that opens nothing (Google Sheets' cell editor: no expanded state) has no state
        // to say, and JAWS says none (dev box, 2026-10-16).
        let state = match node.states.expanded {
            Some(true) => Some(msg::state_expanded()),
            Some(false) => Some(msg::state_collapsed()),
            None => None,
        };
        if let Some(state) = state {
            parts.push(Part::text(" "));
            parts.push(Part::state(state));
        }
        // Google Sheets' cell editor holds a zero-width no-break space and a line break
        // around its text.
        let value = node
            .value
            .as_deref()
            .map(|v| v.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}'))
            .filter(|v| !v.is_empty());
        Announcement::new(vec![
            Line::new(parts),
            Line::of(PartKind::Value, value.unwrap_or(msg::blank().as_str())),
        ])
    }

    /// A link taking focus: "Page one ‹ same page link › ‹Current Page›". aria-current comes
    /// before a plain link's role: "README ‹Current Page› ‹ Link ›".
    pub fn format_link_focus(name: &str, node: &AomNode) -> String {
        Self::link_focus(name, node).render_all()
    }

    /// [`Self::format_link_focus`] as parts with their kinds.
    pub fn link_focus(name: &str, node: &AomNode) -> Announcement {
        let current = node.current_phrase().map(Part::state);
        let parts = if node
            .attributes
            .get("same_page")
            .is_some_and(|v| v == "true")
        {
            let mut parts = name_and_role(name, msg::role_link_same_page().as_str());
            parts.extend(current);
            parts
        } else {
            let mut parts = vec![Part::new(PartKind::Name, name)];
            parts.extend(current);
            parts.push(Part::text(" "));
            parts.push(Part::role(
                if matches!(&node.role, Role::Link { href: Some(h) } if h.starts_with("mailto:")) {
                    msg::role_link_mailto()
                } else {
                    msg::role_link()
                },
            ));
            parts
        };
        let mut lines = vec![Line::new(parts)];
        // Its description follows (GitHub's commit links: the whole message, dev box).
        if let Some(description) = node.attributes.get("description") {
            lines.push(Line::of(PartKind::Description, description.as_str()));
        }
        Announcement::new(lines)
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
    /// Format: "table with {cols} columns and {rows} rows\n{caption}\nColumn 1, Row 1\n{FirstCellText}";
    /// with short messages "{cols} columns and {rows} rows" [obs, 578/s1].
    pub fn format_table_entry(
        cols: usize,
        rows: usize,
        caption: Option<&str>,
        col: usize,
        row: usize,
        cell_text: &str,
        short: bool,
    ) -> String {
        let mut parts = Vec::new();
        parts.push(if short {
            msg::table_entry_short(cols, rows).render()
        } else {
            msg::table_entry(cols, rows).render()
        });
        if let Some(cap) = caption {
            if !cap.is_empty() {
                parts.push(cap.to_string());
            }
        }
        parts.push(msg::table_cell(col, row).render());
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
            parts.push(msg::table_spans_columns(col_span).render());
        }
        parts.push(msg::table_column_of(col, total_cols).render());
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
            parts.push(msg::table_cell_of_spanning_row(col, row_span).render());
        }
        parts.push(msg::table_row_of(row, total_rows).render());
        parts.join("\n")
    }

    /// Formats full cell coordinates and headers when jumping to a cell or querying it (`Ctrl+Alt+Home`, `Ctrl+Alt+End`, `Ctrl+Alt+NumPad5`).
    /// Matches JAWS format: "{ColHdr}  {RowHdr}\n{CellText}\nColumn {col}, Row {row}".
    /// Say Current Cell: the cell's place first, then its headers and its text, "Column 4, Row
    /// 2 | Profit  Q1 | $3,000.00" [obs, 535/s8]; a move to the first or last cell says the
    /// place last ([`Self::format_table_cell_coordinates`]).
    pub fn format_current_cell(
        text: &str,
        col: usize,
        row: usize,
        col_header: Option<&str>,
        row_header: Option<&str>,
        col_span: usize,
        row_span: usize,
    ) -> String {
        let said = Self::format_table_cell_coordinates(
            text, col, row, col_header, row_header, col_span, row_span,
        );
        match said.rsplit_once('\n') {
            Some((rest, place)) => format!("{place}\n{rest}"),
            None => said,
        }
    }

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
            parts.push(msg::table_cell_of_spanning_row(col, row_span).render());
        }
        if col_span > 1 {
            parts.push(msg::table_spans_columns(col_span).render());
        }
        parts.push(msg::table_cell(col, row).render());
        parts.join("\n")
    }

    /// Formats beginning of row announcement (`Ctrl+Alt+Shift+LeftArrow`).
    pub fn format_table_row_start(text: &str) -> String {
        format!("{}\n{}", msg::table_row_start(), text)
    }

    /// Formats end of row announcement (`Ctrl+Alt+Shift+RightArrow`).
    pub fn format_table_row_end(text: &str) -> String {
        format!("{}\n{}", msg::table_row_end(), text)
    }

    /// Formats top of column announcement (`Ctrl+Alt+Shift+UpArrow`).
    pub fn format_table_col_top(text: &str) -> String {
        format!("{}\n{}", msg::table_column_top(), text)
    }

    /// Formats bottom of column announcement (`Ctrl+Alt+Shift+DownArrow`).
    pub fn format_table_col_bottom(text: &str) -> String {
        format!("{}\n{}", msg::table_column_bottom(), text)
    }

    /// Formats column header direct query (`Ctrl+Alt+Shift+H`).
    pub fn format_column_header(col_hdr: Option<&str>, col: usize) -> String {
        if let Some(hdr) = col_hdr {
            if !hdr.is_empty() {
                return hdr.to_string();
            }
        }
        msg::table_column(col).render()
    }

    /// Formats row header direct query (`Ctrl+Alt+Shift+R`).
    pub fn format_row_header(row_hdr: Option<&str>, row: usize) -> String {
        if let Some(hdr) = row_hdr {
            if !hdr.is_empty() {
                return hdr.to_string();
            }
        }
        msg::table_row(row).render()
    }

    /// The font part of Say Font: attributes, then family and size, e.g. `bold  Times New
    /// Roman 24 point` or `Times New Roman 12 point` [obs, 209/s3, s5, s7].
    pub fn font_description(formatting: &TextFormatting) -> Option<String> {
        let mut attributes = Vec::new();
        if formatting.bold {
            attributes.push(msg::font_bold().as_str());
        }
        if formatting.italic {
            attributes.push(msg::font_italic().as_str());
        }
        if formatting.underline {
            attributes.push(msg::font_underline().as_str());
        }
        let mut font = Vec::new();
        font.extend(formatting.font_family.clone());
        font.extend(
            formatting
                .font_size_pt
                .map(|size| msg::font_size(format!("{size:.0}")).render()),
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
        formatting.and_then(Self::font_description).map_or_else(
            || msg::font_none().to_string(),
            |f| msg::font_is(f).render(),
        )
    }

    /// A text field's alignment as Say Font words it: "Paragraph formatting left justified"
    /// (observed; the other alignments' words are assumed to follow it).
    pub fn paragraph_formatting(alignment: &str) -> String {
        let words = match alignment {
            "left" => msg::paragraph_left_justified().as_str(),
            "right" => msg::paragraph_right_justified().as_str(),
            other => other,
        };
        msg::paragraph_formatting(words).render()
    }

    /// Formats modal dialog entry announcement.
    pub fn format_dialog_entry(title: &str) -> String {
        if title.is_empty() {
            msg::dialog_entered().to_string()
        } else {
            msg::dialog_entered_named(title).render()
        }
    }

    /// Formats modal dialog exit announcement.
    pub fn format_dialog_exit() -> String {
        msg::dialog_left().to_string()
    }

    /// A form field as Select a Form Field lists it: its name, its type, then its value or
    /// state ("Wrap check box not checked", "Fruit edit combo", "Notes Edit the lazy dog")
    /// [obs, 227]. A button of any kind is a "Button" ("Outline Button" for a button menu).
    pub fn form_field_list_entry(item: &crate::buffer::FormFieldListItem) -> String {
        let kind = match item.role {
            Role::Button | Role::MenuButton => msg::role_button(),
            Role::Edit => msg::role_edit(),
            Role::CheckBox => msg::role_check_box(),
            Role::RadioButton => msg::role_radio_button(),
            Role::ComboBox if item.editable => msg::role_edit_combo(),
            Role::ComboBox => msg::role_combo_box(),
            Role::Slider => msg::role_slider(),
            _ => msg::role_field(),
        };
        let state = match (&item.role, item.checked) {
            (Role::CheckBox | Role::RadioButton, Some(true)) => Some(msg::state_checked()),
            (Role::CheckBox | Role::RadioButton, Some(false)) => Some(msg::state_not_checked()),
            _ => None,
        };
        [
            Some(item.name.as_str()),
            Some(kind.as_str()),
            item.value.as_deref(),
            state.map(|s| s.as_str()),
        ]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
    }

    /// Formats negative text search feedback matching JAWS.
    pub fn format_text_not_found(query: &str) -> String {
        msg::find_not_found(query).render()
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

    /// Speaks text with every symbol in it named, whatever the punctuation level: a selection
    /// and a word read on its own, as JAWS reads them ("Friday comma, January 2 comma," after
    /// Ctrl+A [obs, 323/s7]; "dog comma," by word [obs, 213/s11]).
    fn speak_symbols(&mut self, text: &str, interrupt: bool) {
        self.speak(text, interrupt);
    }

    /// Speaks a character being read on its own (Left/Right, Delete). `text` is the character
    /// as JAWS's speech history records it; the synthesizer is given its name, as JAWS names
    /// spaces and symbols whatever its punctuation level ([`crate::pronounce`]). A synthesizer
    /// raises a capital's pitch ([`crate::pronounce::character_pitch`]).
    fn speak_character(&mut self, text: &str, interrupt: bool) {
        self.speak(&crate::pronounce::character(text), interrupt);
    }

    /// Speaks a character spelled phonetically (Say Character twice): the history keeps the
    /// character, the synthesizer says its word ([`crate::pronounce::phonetic`]), a capital's
    /// at a raised pitch [obs, 401/s2: "J" in the history, "juliett" at pitch 4 spoken].
    fn speak_phonetic(&mut self, text: &str, interrupt: bool) {
        self.speak(&crate::pronounce::phonetic(text), interrupt);
    }

    /// Speaks a text spelled (Say Word or Say Line twice): the history keeps its characters one
    /// by one ([`crate::pronounce::spelled`]); the synthesizer is given them in parts, each
    /// capital alone at a raised pitch ([`crate::pronounce::spelled_parts`]).
    fn speak_spelled(&mut self, text: &str, interrupt: bool) {
        self.speak(&crate::pronounce::spelled(text), interrupt);
    }

    /// Spells a selection (Say Selected Text twice quickly): spoken as
    /// [`SpeechOutput::speak_spelled`] is, but JAWS's history keeps the text as it is, "Th"
    /// [obs, 501/s4].
    fn speak_spelled_selection(&mut self, text: &str, interrupt: bool) {
        self.speak_spelled(text, interrupt);
    }

    /// Speaks one part of a continuous reading (Say All) behind the parts before it. When
    /// [`SpeechOutput::reports_marks`], `mark` comes back to the engine as the synthesizer
    /// starts the part (`ScreenReaderEngine::speech_reached`).
    fn speak_marked(&mut self, text: &str, mark: u64, interrupt: bool) {
        let _ = mark;
        self.speak(text, interrupt);
    }

    /// Whether the marks of [`SpeechOutput::speak_marked`] come back as their parts are
    /// spoken. Without them a Say All hands over every line at once.
    fn reports_marks(&self) -> bool {
        false
    }

    /// Returns whether speech output is currently active.
    fn is_speaking(&self) -> bool {
        false
    }

    /// The voices the synthesizer offers, by name, for the settings dialog.
    fn voices(&self) -> Vec<String> {
        Vec::new()
    }

    /// Speaks from now on with `voice`: its synthesizer, voice, rate and volume.
    fn configure(&mut self, voice: &crate::settings::VoiceSettings) {
        let _ = voice;
    }

    /// Speech muted (Insert+Space, S): nothing is spoken, nor kept in the history, until it is
    /// unmuted [obs, 517/s7].
    fn set_muted(&mut self, muted: bool) {
        let _ = muted;
    }

    /// What was said lately, the oldest first: the Speech History viewer's lines.
    fn history(&self) -> Vec<String> {
        Vec::new()
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

#[cfg(test)]
mod kind_tests {
    use super::*;

    fn with(mut node: AomNode, attributes: &[(&str, &str)]) -> AomNode {
        for (key, value) in attributes {
            node.attributes
                .insert((*key).to_owned(), (*value).to_owned());
        }
        node
    }

    /// A page control's focus phrasing says what each part is, and renders as before.
    #[test]
    fn web_focus_phrasing_has_kinds() {
        let mut radio = with(
            AomNode::new(5, Role::RadioButton).with_name("Small"),
            &[
                ("group_name", "Size"),
                ("group_id", "4"),
                ("posinset", "1"),
                ("setsize", "3"),
            ],
        );
        radio.states.checked = Some(true);
        let said = SpeechFormatter::control_focus(&radio, |_| false);
        assert_eq!(
            said.kinds(),
            [
                PartKind::Container,
                PartKind::Role,
                PartKind::Name,
                PartKind::Text,
                PartKind::Role,
                PartKind::Text,
                PartKind::State,
                PartKind::Position
            ]
        );
        assert_eq!(
            said.render_all(),
            "Size \u{1d} group \u{1e} \nSmall  \u{1d} radio button \u{1e}   \u{1d}checked\u{1e} \n1 of 3"
        );

        let edit = with(
            AomNode::new(6, Role::Edit)
                .with_name("Email")
                .with_value("a@b.c"),
            &[("required", "true"), ("description", "Work address")],
        );
        let said = SpeechFormatter::control_focus(&edit, |_| false);
        assert_eq!(
            said.kinds(),
            [
                PartKind::Name,
                PartKind::Role,
                PartKind::State,
                PartKind::Value,
                PartKind::Description
            ]
        );
        assert_eq!(
            said.render_all(),
            "Email \u{1d} Edit \u{1e} \n \u{1d}Required\u{1e} \na@b.c\nWork address"
        );
    }
}
