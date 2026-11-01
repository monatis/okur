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

/// A page's list box option as its focus says it, from the focus itself: a combo box's list
/// may not be in the buffer yet when the arrow that opens it moves to its first option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBoxOption {
    /// The list box's name: "States".
    pub list: String,
    pub option: String,
    pub index: Option<usize>,
    pub count: Option<usize>,
}

impl ListBoxOption {
    /// Coming into the list box, and on Insert+Tab: "States List box | Alabama | 1 of 56 | To
    /// move to an item press the Arrow keys."; moving in it: "Alaska | 2 of 56" [obs,
    /// round-aria32, 2967/s1, 2975/s1, 2977/s1].
    pub fn speech(&self, entering: bool) -> String {
        let mut lines = Vec::new();
        if entering {
            lines.push(
                format!(
                    "{}{}",
                    self.list,
                    crate::linearize::role_marker(msg::role_list_box().as_str())
                )
                .trim()
                .to_owned(),
            );
        }
        lines.push(self.option.clone());
        if let (Some(index), Some(count)) = (self.index, self.count) {
            lines.push(msg::position(index.to_string(), count.to_string()).render());
        }
        if entering {
            lines.push(msg::hint_move_to_an_item_arrows().to_string());
        }
        lines.join("\n")
    }
}

/// How a page control's focus speech is said: with the virtual cursor off (Insert+Z), by a
/// focus moving onto it (Tab) rather than a quick key, asked for (Insert+Tab), and as its line
/// (Insert+Up).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FocusSaid {
    pub virtual_off: bool,
    pub by_focus: bool,
    pub asked: bool,
    /// Said as the line it is on (Insert+Up), not as a focus.
    pub line: bool,
}

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

    /// Formats a checkbox announcement: its state "checked", "not checked" or "partially
    /// checked", "not checked" when it reports none.
    pub fn format_checkbox(name: &str, states: &crate::aom::NodeStates) -> String {
        let state_str = states.check_word().unwrap_or_else(msg::state_not_checked);
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
                Self::format_checkbox(name, &node.states)
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
        said: FocusSaid,
    ) -> String {
        Self::control_focus(node, was_inside, said).render_all()
    }

    /// [`Self::format_control_focus`] as parts with their kinds. A line whose spacing parts
    /// cannot give (a check box's, a button's) is one part of kind `Text`.
    pub fn control_focus(
        node: &crate::aom::AomNode,
        was_inside: impl Fn(usize) -> bool,
        said: FocusSaid,
    ) -> Announcement {
        use crate::aom::Role;
        let attr = |k: &str| node.attributes.get(k).map(String::as_str);
        let name = node.name.as_deref().unwrap_or_default().trim();
        // Its aria-keyshortcuts follow its name in the virtual cursor, not with it off: "Skip
        // To Content Alt+0 button menu" [obs, round-aria30, 2874/s1, 2876/s1; 2877/s1].
        let keyed;
        let name = match attr("keyshortcuts").filter(|_| !said.virtual_off && !name.is_empty()) {
            Some(keys) => {
                keyed = format!("{name} {keys}");
                keyed.as_str()
            }
            None => name,
        };
        // Its description follows what it says, on a quick key and a focus alike: "Show
        // Headings Button | Visually outline the landmarks…" [obs, round-aria30, 2879/s1,
        // 2880/s1, 2881/s1].
        let described = |mut lines: Vec<Line>| {
            if let Some(d) = attr("description").filter(|d| !d.trim().is_empty()) {
                lines.push(Line::of(PartKind::Description, d));
            }
            Announcement::new(lines)
        };
        let required = attr("required") == Some("true");
        let checked = |states: &crate::aom::NodeStates| {
            states.check_word().unwrap_or_else(msg::state_not_checked)
        };
        // The group a radio button or check box is in, named when the cursor comes into it
        // from outside: "Membership type group" [obs, 202], "Sandwich Condiments group" [obs,
        // round-aria2, 2000/s1].
        // A focus coming into an ARIA radio group says an empty state after it: "Pizza Crust
        // group | | Regular crust radio button" [obs, round-aria4, 2080/s1, 2085/s1]; a quick
        // key does not [obs, 2082/s1], nor a fieldset [obs, round-aria2, 2003/s1].
        // Insert+Tab names a check box's or radio button's group, but not a switch's, in a
        // fieldset as in an ARIA group: "Reduced motion Switch Off", where F coming to it
        // names the group [obs, round-aria9, 2286/s1, 2270/s1; round-aria10, 2318/s1; check
        // boxes and radio buttons: round-aria2, 2010/s1; round-aria6, 2190/s1; round-aria4,
        // 2118/s1].
        let entered_group = || {
            let (group, group_id) = (attr("group_name")?, attr("group_id")?);
            let kind = attr("group_kind");
            if said.asked && crate::linearize::switch_state(node).is_some() {
                return None;
            }
            (!group_id.parse().is_ok_and(&was_inside)).then(|| {
                let mut parts = vec![
                    Part::new(PartKind::Container, group),
                    Part::role(msg::role_group()),
                ];
                if said.by_focus && kind == Some("radiogroup") {
                    parts.push(Part::text(" "));
                    parts.push(Part::state(""));
                }
                Line::new(parts)
            })
        };
        match &node.role {
            Role::Edit => {
                let password = attr("password") == Some("true");
                let kind = if password {
                    msg::role_edit_password().as_str()
                } else if let Some(description) = node.role_description() {
                    description
                } else if attr("spinbutton") == Some("true") {
                    msg::role_edit_spin_box().as_str()
                } else if attr("readonly") == Some("true") {
                    msg::role_edit_read_only().as_str()
                } else if attr("multiline") == Some("true") {
                    msg::role_edit_multiline().as_str()
                } else {
                    msg::role_edit().as_str()
                };
                // A quick key to a spin button names its group, its focus does not: "Guests
                // group | Adults edit spin box | 1" [obs, round-aria25, 2719/s1, 2720/s1]; an
                // edit's quick key and focus both: "Add Contact group | Name Edit | blank" [obs,
                // round-aria36, 3078/s1, 3080/s1].
                let mut lines: Vec<Line> = entered_group()
                    .filter(|_| attr("spinbutton") != Some("true") || !said.by_focus)
                    .into_iter()
                    .collect();
                lines.push(Line::new(vec![
                    Part::new(PartKind::Name, name),
                    Part::role(kind),
                ]));
                // An invalid value: "Has Error" where it has an error message, then "invalid
                // entry" [obs, round-aria25, 2732/s1, 2733/s1].
                if attr("has_error") == Some("true") {
                    lines.push(Line::of(PartKind::Text, msg::state_has_error()));
                }
                if attr("invalid") == Some("true") {
                    lines.push(Line::new(vec![Part::state(msg::state_invalid_entry())]));
                }
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
                // A spin button's limits follow its value, not on Insert+Up: "Adults edit spin
                // box | 1 | min 1 max 8" (a newer JAWS, 2025.2508, ARIA-AT's reports).
                if attr("spinbutton") == Some("true") && !said.line {
                    if let Some(limits) = Self::range_limits(node) {
                        lines.push(Line::of(PartKind::Text, limits));
                    }
                }
                // Its error message after its value, not on Insert+Up: "Adults edit spin box |
                // Has Error | invalid entry | 9 | Must be between 1 and 8". JAWS says it on the
                // PC cursor's Insert+Tab alone [obs, round-aria25, 2734/s1], where ARIA-AT's
                // quantity-spin-button plan requires it (MUST): Okur says it wherever the field
                // is described (the owner's rule for the web, 2026-11-01).
                if let Some(message) = attr("error_message")
                    .filter(|m| !m.trim().is_empty() && attr("has_error") == Some("true"))
                    .filter(|_| !said.line)
                {
                    lines.push(Line::of(PartKind::Text, message));
                }
                if let Some(d) = attr("description") {
                    lines.push(Line::of(PartKind::Description, d));
                }
                Announcement::new(lines)
            }
            Role::RadioButton => {
                let mut lines: Vec<Line> = entered_group().into_iter().collect();
                let mut parts = name_and_role(name, msg::role_radio_button().as_str());
                parts.push(Part::text(" "));
                parts.push(Part::state(checked(&node.states)));
                lines.push(Line::new(parts));
                // Its place in its group, checked or not [obs, round-aria16, 2399/s1; 1126].
                if let (Some(p), Some(n)) = (attr("posinset"), attr("setsize")) {
                    lines.push(Line::of(PartKind::Position, msg::position(p, n)));
                }
                Announcement::new(lines)
            }
            // Insert+Up says that it completes what is typed too: "State edit combo collapsed
            // Supports AutoComplete", where JAWS says its text alone, "blank", and ARIA-AT
            // requires its role, name, state and autocomplete (MUST) [obs, round-aria32,
            // 2926/s1] (the owner's rule for the web, 2026-11-01).
            Role::ComboBox if attr("editable") == Some("true") => {
                let mut combo = Self::edit_combo(name, node);
                if said.line && attr("autocomplete") == Some("true") {
                    if let Some(first) = combo.lines.first_mut() {
                        first.parts.push(Part::text(" "));
                        first
                            .parts
                            .push(Part::state(msg::state_supports_autocomplete()));
                    }
                }
                combo
            }
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
            // In parts, so that verbosity's Control Type and Item State can leave them out
            // [obs, 737/s7, 738/s7].
            Role::CheckBox => {
                let switch = crate::linearize::switch_state(node);
                let role = if switch.is_some() {
                    msg::role_switch().as_str()
                } else {
                    node.role_description()
                        .unwrap_or(msg::role_check_box().as_str())
                };
                let mut parts = name_and_role(name, role);
                parts.push(Part::text(" "));
                parts.push(Part::state(switch.unwrap_or_else(|| checked(&node.states))));
                let mut lines: Vec<Line> = entered_group().into_iter().collect();
                lines.push(Line::new(parts));
                Announcement::new(lines)
            }
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
                described(vec![Line::new(parts)])
            }
            Role::Button => {
                let role = node
                    .role_description()
                    .unwrap_or(msg::role_button().as_str());
                let mut parts = name_and_role(name, role);
                // A button that is its heading's only content says the heading after its name,
                // "Billing Address heading level 3 | Button expanded", in the virtual cursor
                // [obs, round-aria5, 2122/s1, 2126/s1]. With it off JAWS says the button alone
                // (2128/s1), where ARIA-AT requires the heading and its level: Okur says them
                // on a focus too (the owner's rule for the web, 2026-11-01); Insert+Up and
                // Insert+Tab keep JAWS's form.
                let heading = attr("heading_level")
                    .and_then(|l| l.parse::<u32>().ok())
                    .filter(|_| !said.virtual_off || (said.by_focus && !said.asked));
                if let Some(level) = heading {
                    parts.insert(1, Part::role(msg::role_heading_level(level).render()));
                }
                // A disclosure button speaks its state [obs, 29/s1].
                match node.states.expanded {
                    Some(true) => parts.push(Part::state(msg::state_expanded())),
                    Some(false) => parts.push(Part::state(msg::state_collapsed())),
                    None => {}
                }
                if let Some(popup) = node.attributes.get("haspopup") {
                    parts.push(Part::state(msg::state_has_popup()));
                    parts.push(Part::text(" "));
                    parts.push(Part::new(PartKind::Value, popup));
                }
                parts.extend(node.current_phrase().map(Part::state));
                // Its named fieldset as it comes into it: "Add Contact group | Add Contact
                // Button" [obs, round-aria36, 3084/s1].
                let mut lines: Vec<Line> = entered_group().into_iter().collect();
                lines.push(Line::new(parts));
                described(lines)
            }
            // "Red left right slider | 128", its group named by a quick key to it, not by its
            // focus [obs, round-aria26, 2740/s1, 2741/s1].
            Role::Slider => {
                let mut lines: Vec<Line> = entered_group()
                    .filter(|_| !said.by_focus)
                    .into_iter()
                    .collect();
                lines.push(Line::new(name_and_role(
                    name,
                    Self::slider_role(node).as_str(),
                )));
                if let Some(value) = node
                    .value
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                {
                    let mut parts = vec![Part::new(PartKind::Value, value)];
                    if let Some(limits) = Self::range_limits(node) {
                        parts.push(Part::text(" "));
                        parts.push(Part::text(limits));
                    }
                    lines.push(Line::new(parts));
                }
                Announcement::new(lines)
            }
            // A menu button says it collapsed or expanded: "Actions button menu collapsed".
            // JAWS says it on a quick key [obs, round-aria20, 2574/s1, 2578/s1] and not on a
            // focus, where ARIA-AT's menu button plans ask for it (SHOULD): Okur says it on a
            // focus too (the owner's rule for the web, 2026-11-01).
            Role::MenuButton => {
                let role = node
                    .role_description()
                    .unwrap_or(msg::role_button_menu().as_str());
                let mut parts = name_and_role(name, role);
                let state = match node.states.expanded {
                    Some(true) => Some(msg::state_expanded()),
                    Some(false) => Some(msg::state_collapsed()),
                    None => None,
                };
                if let Some(state) = state {
                    parts.push(Part::text(" "));
                    parts.push(Part::state(state));
                }
                described(vec![Line::new(parts)])
            }
            // An ARIA meter by its name, role and value, "Central Processing Unit (CPU) Usage
            // meter | 36.1665", where ARIA-AT's meter plan requires all three (MUST) [obs,
            // round-aria38, 3111/s1].
            Role::ProgressBar if attr("meter") == Some("true") => {
                let mut lines = vec![Line::new(name_and_role(name, msg::role_meter().as_str()))];
                if let Some(value) = node
                    .value
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                {
                    lines.push(Line::of(PartKind::Value, value));
                }
                Announcement::new(lines)
            }
            _ => Announcement::new(vec![Line::of(PartKind::Name, name)]),
        }
    }

    /// A container a move comes into, on its line, as a focus coming into it says it: a
    /// landmark "Repository files ‹ NavigationRegion ›", a list "‹ list › | with 1 items", a
    /// dialog by its name and description, a named tab panel (JAWS's words, dev box
    /// 2026-10-16; [obs, round-aria18, round-aria23]). None for any other node.
    pub fn container_entered(n: &crate::aom::AomNode) -> Option<String> {
        use crate::aom::Role;
        let name = n.name.as_deref().map(str::trim).unwrap_or_default();
        let named = |marker: &str| {
            if name.is_empty() {
                format!(" \u{1d} {marker} \u{1e} \n")
            } else {
                format!("{name} \u{1d} {marker} \u{1e} \n")
            }
        };
        match &n.role {
            Role::Landmark { landmark_type } => match landmark_type.as_str() {
                "main" => Some(named(msg::role_region_main().as_str())),
                "form" => Some(named(msg::role_region_form().as_str())),
                "navigation" => Some(named(msg::role_region_navigation().as_str())),
                "banner" => Some(named(msg::role_region_banner().as_str())),
                "complementary" => Some(named(msg::role_region_complementary().as_str())),
                "search" => Some(named(msg::role_region_search().as_str())),
                "contentinfo" => Some(named(msg::role_region_content_info().as_str())),
                "region" => Some(named(msg::role_region().as_str())),
                _ => None,
            },
            // A dialog by its name, "Modal" where it is one, then its description:
            // "Address Added Modal dialog | The address you provided has been
            // added..." [obs, round-aria23, 2666/s1, 2668/s1, 2685/s1]. JAWS 2025.2505
            // says "Modal" with the virtual cursor off only, a newer JAWS (2025.2508,
            // ARIA-AT's reports) with it on too, as Okur does.
            Role::Dialog | Role::AlertDialog => {
                let mut line = if n.states.modal {
                    format!(
                        "{name} \u{1d}{}\u{1e} \u{1d} {} \u{1e} \n",
                        msg::state_modal(),
                        msg::role_dialog()
                    )
                } else {
                    named(msg::role_dialog().as_str())
                };
                if let Some(text) = n
                    .attributes
                    .get("description")
                    .map(|d| d.trim())
                    .filter(|d| !d.is_empty())
                {
                    line.push_str(text);
                    line.push('\n');
                }
                Some(line)
            }
            // "Peter Müller TabPanel" [obs, round-aria18, 2531/s1].
            Role::TabPanel if !name.is_empty() => Some(named(msg::role_tab_panel().as_str())),
            Role::List => {
                let items = n
                    .children
                    .iter()
                    .filter(|c| c.role == Role::ListItem)
                    .count();
                Some(format!(
                    "{}{}\n",
                    named(msg::role_list().as_str()),
                    msg::list_items_count(items)
                ))
            }
            _ => None,
        }
    }

    /// A slider's or spin button's limits, "min 0 max 255", where its value is a number (no
    /// aria-valuetext) and both limits are known. A newer JAWS (2025.2508, ARIA-AT's reports)
    /// says them after the value, which ARIA-AT asks for (SHOULD); JAWS 2025.2505 does not.
    pub fn range_limits(node: &crate::aom::AomNode) -> Option<String> {
        let number = |n: f64| {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{n:.0}")
            } else {
                format!("{n}")
            }
        };
        node.value.as_deref()?.trim().parse::<f64>().ok()?;
        let (min, max) = (node.value_min?, node.value_max?);
        Some(msg::range_limits(number(min), number(max)).render())
    }

    /// A slider's role word, by its orientation: "left right slider" for a horizontal one
    /// [obs, round-aria26, 2741/s1], "slider" where its orientation is not known.
    pub fn slider_role(node: &crate::aom::AomNode) -> crate::messages::StaticMessage {
        match node.attributes.get("orientation").map(String::as_str) {
            Some("horizontal") => msg::role_slider_horizontal(),
            // "Temperature up down slider" [obs, round-aria28, 2779/s2].
            Some("vertical") => msg::role_slider_vertical(),
            _ => msg::role_slider(),
        }
    }

    /// The usage hint JAWS adds when focus lands on a page control [obs, 204/s7–s10].
    /// Edits, radio buttons and combo boxes get none.
    pub fn web_focus_hint(node: &crate::aom::AomNode, virtual_off: bool) -> Option<&'static str> {
        use crate::aom::Role;
        match node.role {
            // A switch has none [obs, round-aria8, 2240/s1].
            Role::CheckBox if crate::linearize::switch_state(node).is_some() => None,
            Role::CheckBox if node.states.checked == Some(true) => {
                Some(msg::hint_clear_checkmark_spacebar().as_str())
            }
            Role::CheckBox => Some(msg::hint_check_spacebar().as_str()),
            Role::Button if node.states.checked.is_some() => {
                Some(msg::hint_toggle_spacebar().as_str())
            }
            // With the virtual cursor off a button is pressed as a native one [obs,
            // round-aria3, 2035/s1, 2040/s1], and a radio button moved through with the arrows
            // [obs, round-aria4, 2081/s1, 2086/s1].
            Role::Button if virtual_off => Some(msg::hint_activate_spacebar().as_str()),
            // The controls a focus takes the keys to say how to use them, with the virtual
            // cursor on or off. JAWS 2025.2505 says these with it off only; a newer JAWS
            // (2025.2508, ARIA-AT's reports) with it on too, as Okur does.
            // A menu button: "Press space to activate the menu, then navigate with arrow keys"
            // [obs, round-aria20, 2582/s1].
            Role::MenuButton => Some(msg::hint_menu_button().as_str()),
            // A slider [obs, round-aria26, 2742/s1].
            Role::Slider => Some(msg::hint_slider_arrows().as_str()),
            // An ARIA spin button: "To set the value use the Arrow keys or type the value."
            // [obs, round-aria25, 2721/s1, 2730/s1].
            Role::Edit if node.attributes.get("spinbutton").map(String::as_str) == Some("true") => {
                Some(msg::hint_set_value_arrows_or_type().as_str())
            }
            Role::RadioButton if virtual_off => Some(msg::hint_radio_up_down().as_str()),
            // A combo box with the virtual cursor off, Insert+Tab's too: an edit combo's "To set
            // the value use the Arrow keys or type the value.", another's "To change the
            // selection use the Arrow keys." [obs, round-aria32, 2927/s1; round-aria33,
            // 2992/s1, 2996/s1].
            Role::ComboBox
                if virtual_off
                    && node.attributes.get("editable").map(String::as_str) == Some("true") =>
            {
                Some(msg::hint_set_value_arrows_or_type().as_str())
            }
            Role::ComboBox if virtual_off => Some(msg::hint_change_selection_arrows().as_str()),
            Role::Link { .. } | Role::Button => Some(msg::hint_activate_enter().as_str()),
            _ => None,
        }
    }

    /// Formats a heading announcement for quick navigation (`H` / `1`–`6`), its role as
    /// `indication` says it [obs, 730, 731].
    pub fn format_heading(
        level: u32,
        text: &str,
        indication: crate::settings::HeadingIndication,
    ) -> String {
        match indication.role(level) {
            Some(role) => format!("{text}\n \u{001d} {role} \u{001e}"),
            None => text.to_owned(),
        }
    }

    /// A higher heading a level key passes, said on one line before the heading it reaches:
    /// "Reference ‹ heading level 1 ›" [obs, 275/s3].
    pub fn format_passed_heading(
        level: u32,
        text: &str,
        indication: crate::settings::HeadingIndication,
    ) -> String {
        let role = indication
            .role(level)
            .map(|role| crate::linearize::role_marker(&role))
            .unwrap_or_default();
        format!("{text}{role}")
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
        Self::link_focus(name, node, false).render_all()
    }

    /// A combo box taking the focus by Tab in a page: its name, role, state and value. JAWS
    /// leaves the state out, where ARIA-AT requires it (the owner's rule for the web).
    pub fn combo_box_tab_focus(name: &str, node: &AomNode) -> Announcement {
        let mut parts = name_and_role(name, msg::role_combo_box_focus().as_str());
        let state = match node.states.expanded {
            Some(true) => Some(msg::state_expanded()),
            Some(false) => Some(msg::state_collapsed()),
            None => None,
        };
        if let Some(state) = state {
            parts.push(Part::text(" "));
            parts.push(Part::state(state));
        }
        let mut lines = vec![Line::new(parts)];
        if let Some(value) = node.value.as_deref() {
            lines.push(Line::of(PartKind::Value, value));
        }
        Announcement::new(lines)
    }

    /// [`Self::format_link_focus`] as parts with their kinds.
    pub fn link_focus(name: &str, node: &AomNode, virtual_off: bool) -> Announcement {
        let current = node.current_phrase().map(Part::state);
        // With the virtual cursor off a same page link is a "Link" as any [obs, round-aria4,
        // 2090/s1].
        let same_page = !virtual_off
            && node
                .attributes
                .get("same_page")
                .is_some_and(|v| v == "true");
        let parts = if same_page {
            let mut parts = name_and_role(name, msg::role_link_same_page().as_str());
            parts.extend(current);
            parts
        } else {
            // A visited link taking the focus is not said visited: "City Library Expands Evening
            // Hours ‹ Link ›" [obs, 689/s3]; its line says it (`linearize`).
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
        entry: String,
        caption: Option<&str>,
        col: usize,
        row: usize,
        cell_text: &str,
    ) -> String {
        let mut parts = vec![entry];
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

    /// A table's size as T says it entering it: "table with 3 columns and 4 rows", an ARIA
    /// grid's "grid with 5 columns and 7 rows" [obs, round-aria24, 2694/s1], with short
    /// messages "3 columns and 4 rows" [obs, 578/s1].
    pub fn table_entry_line(cols: usize, rows: usize, short: bool, aria_grid: bool) -> String {
        if short {
            msg::table_entry_short(cols, rows).render()
        } else if aria_grid {
            msg::grid_entry(cols, rows).render()
        } else {
            msg::table_entry(cols, rows).render()
        }
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
            (Role::CheckBox, _) if item.mixed => Some(msg::state_partially_checked()),
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

/// The tutor messages: what a key does on the control that has the focus (`hints.ftl`).
fn tutor_messages() -> [crate::messages::StaticMessage; 25] {
    [
        msg::hint_activate_enter(),
        msg::hint_activate_spacebar(),
        msg::hint_change_selection_arrows(),
        msg::hint_select_item_arrows(),
        msg::hint_space_toggle_choices(),
        msg::hint_split_button(),
        msg::hint_check_spacebar(),
        msg::hint_clear_checkmark_spacebar(),
        msg::hint_context_menu_up_down(),
        msg::hint_menu_button(),
        msg::hint_menu_bar_left_right(),
        msg::hint_menu_bar_items_left_right(),
        msg::hint_menu_items_up_down(),
        msg::hint_move_to_an_item_arrows(),
        msg::hint_move_to_items_arrows(),
        msg::hint_radio_up_down(),
        msg::hint_set_value_arrows_or_type(),
        msg::hint_set_value_arrows(),
        msg::hint_slider_arrows(),
        msg::hint_switch_pages(),
        msg::hint_switch_pages_ctrl_tab(),
        msg::hint_toggle_spacebar(),
        msg::hint_tree_arrows(),
        msg::hint_type_in_text(),
        msg::hint_read_only_text(),
    ]
}

/// A tutor message ending `text` on a line of its own, apart from what comes before it (empty
/// when the message is all of it): ("Punctuation: Combo box\nMost\n3 of 4", "To change the
/// selection use the Arrow keys."). JAWS says its tutor messages in the Tutor and Message voice,
/// as Voice Adjustment's Adjust box names it.
pub fn split_tutor_message(text: &str) -> Option<(&str, &str)> {
    let (said, last) = text.rsplit_once('\n').unwrap_or(("", text));
    tutor_messages()
        .iter()
        .any(|message| message.as_str() == last)
        .then_some((said, last))
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

    /// Speaks one of Okur's own messages, in the Tutor and Message voice context.
    fn speak_message(&mut self, text: &str, interrupt: bool) {
        self.speak(text, interrupt);
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

    /// Speaks a character selected or unselected with Shift and an arrow: the line's marks
    /// (`before`), the character named and pitched as when read alone, then the selection word
    /// (`after`): "heading level 1 [pitch +4] T selected" [obs, 501/s2], "space selected"
    /// [obs, 504/s17].
    fn speak_selected_character(
        &mut self,
        before: &str,
        character: &str,
        after: &str,
        interrupt: bool,
    ) {
        let marks = !before.trim().is_empty();
        if marks {
            self.speak(before, interrupt);
        }
        self.speak_character(character, interrupt && !marks);
        if !after.trim().is_empty() {
            self.speak(after, false);
        }
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

    /// Reads from now on in the voice of `context`'s cursor: the PC cursor's, or the Okur
    /// cursor's while it is active (JAWS's JAWS Cursor voice). Typing echo keeps the keyboard's.
    fn set_cursor_voice(&mut self, _context: crate::settings::VoiceContext) {}

    /// Speaks from now on with `voice`: its synthesizer, voice, rate and volume.
    fn configure(&mut self, voice: &crate::settings::VoiceSettings) {
        let _ = voice;
    }

    /// Less speech (Speech On Demand between its commands, or muted): what is said from now on
    /// is kept in the speech history but not given to the synthesizer, until it is spoken
    /// again [obs, 700/s1, s3: Down and H silent, "banner" and the heading in the history;
    /// 707/s4, s5 muted].
    fn set_unspoken(&mut self, unspoken: bool) {
        let _ = unspoken;
    }

    /// What was said lately, the oldest first: the Speech History viewer's lines.
    fn history(&self) -> Vec<String> {
        Vec::new()
    }

    /// Forgets what was said (Insert+Space, Shift+H): the Speech History then holds only what
    /// is said after [obs, 570/s6, s9].
    fn clear_history(&mut self) {}
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
mod tutor_tests {
    use super::*;

    /// Every message of `hints.ftl` is a tutor message, and only the last line is taken.
    #[test]
    fn a_tutor_message_ending_an_utterance_is_split_off() {
        let catalog = include_str!("../locales/en-US/hints.ftl");
        let defined = catalog.lines().filter(|l| l.starts_with("hint-")).count();
        assert_eq!(tutor_messages().len(), defined);
        assert_eq!(
            split_tutor_message(
                "Punctuation: Combo box\nMost\n3 of 4\nTo change the selection use the Arrow keys."
            ),
            Some((
                "Punctuation: Combo box\nMost\n3 of 4",
                "To change the selection use the Arrow keys."
            ))
        );
        assert_eq!(
            split_tutor_message("Type in text."),
            Some(("", "Type in text."))
        );
        assert_eq!(split_tutor_message("Type in text.\nMost"), None);
    }
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
        let said = SpeechFormatter::control_focus(&radio, |_| false, FocusSaid::default());
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
        let said = SpeechFormatter::control_focus(&edit, |_| false, FocusSaid::default());
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

    /// A list box's option names its list box only as the focus comes into it.
    #[test]
    fn a_list_box_option_names_its_list_box_on_the_way_in() {
        let option = ListBoxOption {
            list: "States".to_owned(),
            option: "Alabama".to_owned(),
            index: Some(1),
            count: Some(56),
        };
        assert_eq!(
            option.speech(true),
            "States \u{1d} List box \u{1e}\nAlabama\n1 of 56\nTo move to an item press the Arrow keys."
        );
        assert_eq!(option.speech(false), "Alabama\n1 of 56");
    }

    /// A slider's and a spin button's limits follow a number, never a text; a spin button's
    /// not on Insert+Up (a newer JAWS, 2025.2508, ARIA-AT's reports).
    #[test]
    fn range_limits_follow_a_number() {
        let slider = with(
            AomNode::new(7, Role::Slider)
                .with_name("Red")
                .with_value("128")
                .with_range(0.0, 255.0, 128.0),
            &[("orientation", "horizontal")],
        );
        let said = SpeechFormatter::control_focus(&slider, |_| false, FocusSaid::default());
        assert!(
            said.render_all().ends_with("128 min 0 max 255"),
            "{}",
            said.render_all()
        );
        let text = AomNode::new(8, Role::Slider)
            .with_name("Temperature")
            .with_value("25.0 degrees Celsius")
            .with_range(10.0, 38.0, 25.0);
        assert_eq!(SpeechFormatter::range_limits(&text), None);
        let half = AomNode::new(9, Role::Slider)
            .with_value("2.5")
            .with_range(0.5, 10.0, 2.5);
        assert_eq!(
            SpeechFormatter::range_limits(&half).as_deref(),
            Some("min 0.5 max 10")
        );

        let spin = with(
            AomNode::new(10, Role::Edit)
                .with_name("Adults")
                .with_value("1")
                .with_range(1.0, 8.0, 1.0),
            &[("spinbutton", "true")],
        );
        let focus = SpeechFormatter::control_focus(&spin, |_| false, FocusSaid::default());
        assert!(
            focus.render_all().ends_with("\n1\nmin 1 max 8"),
            "{}",
            focus.render_all()
        );
        let line = FocusSaid {
            line: true,
            ..FocusSaid::default()
        };
        let said = SpeechFormatter::control_focus(&spin, |_| false, line).render_all();
        assert!(!said.contains("min 1"), "{said}");
    }
}
