use crate::aom::{AomNode, Role};
use crate::buffer::VirtualBuffer;
use crate::speech::SpeechFormatter;

/// The Virtual Cursor providing reading navigation and quick key primitives over a VirtualBuffer.
#[derive(Debug, Clone)]
pub struct VirtualCursor {
    pub buffer: VirtualBuffer,
    pub offset: usize,
    pub current_node_id: Option<usize>,
    pub forms_mode_active: bool,
    pub current_table_pos: Option<(usize, usize, usize)>,
}

impl VirtualCursor {
    pub fn new(buffer: VirtualBuffer) -> Self {
        Self {
            buffer,
            offset: 0,
            current_node_id: None,
            forms_mode_active: false,
            current_table_pos: None,
        }
    }

    /// Returns the active AOM node at the cursor's current position.
    pub fn current_node(&self) -> Option<&AomNode> {
        if let Some(id) = self.current_node_id {
            if let Some(node) = self.buffer.node_map.get(&id) {
                return Some(node);
            }
        }
        self.buffer.node_at_offset(self.offset)
    }

    /// Resets the cursor to the top of document (`^{HOME}`).
    pub fn move_to_start(&mut self) -> String {
        self.offset = 0;
        self.current_node_id = self.buffer.spans.first().map(|s| s.node_id);
        self.current_table_pos = None;
        self.current_line()
    }

    /// Moves cursor to the end of the document (`^{END}`).
    pub fn move_to_end(&mut self) -> String {
        self.offset = self.buffer.len();
        self.current_node_id = self.buffer.spans.last().map(|s| s.node_id);
        self.current_table_pos = None;
        self.current_line()
    }

    /// Moves cursor down to next line (`{DOWN}`).
    pub fn move_next_line(&mut self) -> Option<String> {
        if let Some(next_line) = self.buffer.next_line_from(self.offset) {
            self.offset = next_line.start_offset;
            self.current_node_id = Some(next_line.node_id);
            Some(SpeechFormatter::format_line(next_line))
        } else {
            None
        }
    }

    /// Moves cursor up to prior line (`{UP}`).
    pub fn move_prior_line(&mut self) -> Option<String> {
        if let Some(prior_line) = self.buffer.prior_line_from(self.offset) {
            self.offset = prior_line.start_offset;
            self.current_node_id = Some(prior_line.node_id);
            Some(SpeechFormatter::format_line(prior_line))
        } else {
            None
        }
    }

    /// Speaks current line without moving cursor (`SayLine`).
    pub fn current_line(&self) -> String {
        if let Some(line) = self.buffer.line_at_offset(self.offset) {
            SpeechFormatter::format_line(line)
        } else {
            String::new()
        }
    }

    /// Moves cursor to next word (`^{RIGHT}`).
    pub fn move_next_word(&mut self) -> Option<String> {
        if let Some(next_word) = self.buffer.next_word_from(self.offset) {
            self.offset = next_word.start_offset;
            Some(SpeechFormatter::format_word(&next_word.text))
        } else {
            None
        }
    }

    /// Moves cursor to prior word (`^{LEFT}`).
    pub fn move_prior_word(&mut self) -> Option<String> {
        if let Some(prior_word) = self.buffer.prior_word_from(self.offset) {
            self.offset = prior_word.start_offset;
            Some(SpeechFormatter::format_word(&prior_word.text))
        } else {
            None
        }
    }

    /// Speaks current word without moving cursor (`SayWord`).
    pub fn current_word(&self) -> String {
        if let Some(word) = self.buffer.word_at_offset(self.offset) {
            SpeechFormatter::format_word(&word.text)
        } else {
            String::new()
        }
    }

    /// Moves cursor to next character (`{RIGHT}`).
    pub fn move_next_char(&mut self) -> Option<String> {
        if self.offset >= self.buffer.len() {
            return None;
        }

        let current_char = self.buffer.char_at(self.offset)?;
        let next_offset = self.offset + current_char.len_utf8();

        if next_offset < self.buffer.len() {
            self.offset = next_offset;
            let ch = self.buffer.char_at(self.offset)?;
            Some(SpeechFormatter::format_char(ch))
        } else {
            self.offset = self.buffer.len();
            None
        }
    }

    /// Moves cursor to prior character (`{LEFT}`).
    pub fn move_prior_char(&mut self) -> Option<String> {
        if self.offset == 0 {
            return None;
        }

        // Find preceding UTF-8 character boundary
        let mut prev = self.offset - 1;
        while prev > 0 && !self.buffer.text.is_char_boundary(prev) {
            prev -= 1;
        }

        self.offset = prev;
        let ch = self.buffer.char_at(self.offset)?;
        Some(SpeechFormatter::format_char(ch))
    }

    /// Speaks current character without moving cursor (`SayCharacter`).
    pub fn current_char(&self) -> String {
        if let Some(ch) = self.buffer.char_at(self.offset) {
            SpeechFormatter::format_char(ch)
        } else {
            String::new()
        }
    }

    /// Moves cursor down to next paragraph / block element (`^{DOWN}`).
    pub fn move_next_paragraph(&mut self) -> Option<String> {
        if let Some(next_p) = self.buffer.next_paragraph_from(self.offset) {
            self.offset = next_p.start_offset;
            self.current_node_id = Some(next_p.node_id);
            Some(SpeechFormatter::format_role_and_text(
                &next_p.role,
                &next_p.text,
            ))
        } else {
            None
        }
    }

    /// Moves cursor up to prior paragraph / block element (`^{UP}`).
    pub fn move_prior_paragraph(&mut self) -> Option<String> {
        if let Some(prior_p) = self.buffer.prior_paragraph_from(self.offset) {
            self.offset = prior_p.start_offset;
            self.current_node_id = Some(prior_p.node_id);
            Some(SpeechFormatter::format_role_and_text(
                &prior_p.role,
                &prior_p.text,
            ))
        } else {
            None
        }
    }

    /// Speaks entire document from current cursor position (`SayAll`).
    pub fn say_all(&self) -> String {
        let current_line_idx = self
            .buffer
            .line_at_offset(self.offset)
            .map(|l| l.line_index)
            .unwrap_or(0);

        let lines: Vec<String> = self
            .buffer
            .lines
            .iter()
            .skip(current_line_idx)
            .map(SpeechFormatter::format_line)
            .collect();

        lines.join("\n")
    }

    /// Moves cursor to next heading across all levels (`H` / `MoveToNextHeading`).
    pub fn move_next_heading(&mut self) -> Option<String> {
        if self.buffer.headings.is_empty() {
            return Some("There are no headings on this page.".to_string());
        }
        let (target_offset, level, wrapped) = match self.buffer.next_heading_from(self.offset) {
            Some((off, lvl)) => (off, lvl, false),
            None => {
                let (off, lvl) = self.buffer.headings.first().copied()?;
                (off, lvl, true)
            }
        };
        self.offset = target_offset;
        let node = self.buffer.find_heading_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let text = node
            .and_then(|n| n.text.as_deref().or(n.name.as_deref()))
            .or_else(|| {
                self.buffer
                    .line_at_offset(target_offset)
                    .map(|l| l.text.as_str())
            })
            .unwrap_or("");
        let heading_speech = SpeechFormatter::format_heading(level, text);
        if wrapped {
            Some(format!("wrapping to top\n{}", heading_speech))
        } else {
            Some(heading_speech)
        }
    }

    /// Moves cursor to prior heading across all levels (`+H` / `MoveToPriorHeading`).
    pub fn move_prior_heading(&mut self) -> Option<String> {
        if self.buffer.headings.is_empty() {
            return Some("There are no headings on this page.".to_string());
        }
        let (target_offset, level, wrapped) = match self.buffer.prior_heading_from(self.offset) {
            Some((off, lvl)) => (off, lvl, false),
            None => {
                let (off, lvl) = self.buffer.headings.last().copied()?;
                (off, lvl, true)
            }
        };
        self.offset = target_offset;
        let node = self.buffer.find_heading_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let text = node
            .and_then(|n| n.text.as_deref().or(n.name.as_deref()))
            .or_else(|| {
                self.buffer
                    .line_at_offset(target_offset)
                    .map(|l| l.text.as_str())
            })
            .unwrap_or("");
        let heading_speech = SpeechFormatter::format_heading(level, text);
        if wrapped {
            Some(format!("wrapping to bottom\n{}", heading_speech))
        } else {
            Some(heading_speech)
        }
    }

    /// Moves cursor to next heading of a specific level (`1`–`6` / `MoveToNextHeadingLevelX`).
    pub fn move_next_heading_level(&mut self, level: u32) -> Option<String> {
        let (target_offset, lvl) = self.buffer.next_heading_level_from(self.offset, level)?;
        self.offset = target_offset;
        let node = self.buffer.find_heading_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let text = node
            .and_then(|n| n.text.as_deref().or(n.name.as_deref()))
            .or_else(|| {
                self.buffer
                    .line_at_offset(target_offset)
                    .map(|l| l.text.as_str())
            })
            .unwrap_or("");
        Some(SpeechFormatter::format_heading(lvl, text))
    }

    /// Moves cursor to prior heading of a specific level (`+1`–`+6` / `MoveToPriorHeadingLevelX`).
    pub fn move_prior_heading_level(&mut self, level: u32) -> Option<String> {
        let (target_offset, lvl) = self.buffer.prior_heading_level_from(self.offset, level)?;
        self.offset = target_offset;
        let node = self.buffer.find_heading_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let text = node
            .and_then(|n| n.text.as_deref().or(n.name.as_deref()))
            .or_else(|| {
                self.buffer
                    .line_at_offset(target_offset)
                    .map(|l| l.text.as_str())
            })
            .unwrap_or("");
        Some(SpeechFormatter::format_heading(lvl, text))
    }

    /// Moves cursor to next link (`U` / `K` / `MoveToNextLink`).
    pub fn move_next_link(&mut self) -> Option<String> {
        let target_offset = self.buffer.next_link_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_link_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let is_same_page = match node.map(|n| &n.role) {
            Some(Role::Link { href: Some(h) }) => h.starts_with('#'),
            _ => false,
        };
        let text = node
            .and_then(|n| n.name.as_deref().or(n.text.as_deref()))
            .or_else(|| self.buffer.span_text_at_offset(target_offset))
            .unwrap_or("");
        Some(SpeechFormatter::format_link(text, is_same_page))
    }

    /// Moves cursor to prior link (`+U` / `+K` / `MoveToPriorLink`).
    pub fn move_prior_link(&mut self) -> Option<String> {
        let target_offset = self.buffer.prior_link_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_link_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let is_same_page = match node.map(|n| &n.role) {
            Some(Role::Link { href: Some(h) }) => h.starts_with('#'),
            _ => false,
        };
        let text = node
            .and_then(|n| n.name.as_deref().or(n.text.as_deref()))
            .or_else(|| self.buffer.span_text_at_offset(target_offset))
            .unwrap_or("");
        Some(SpeechFormatter::format_link(text, is_same_page))
    }

    /// Moves cursor to next list container (`L` / `MoveToNextList`).
    pub fn move_next_list(&mut self) -> Option<String> {
        let target_offset = self.buffer.next_list_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_list_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let count = node.map(|n| n.children.len()).unwrap_or(0);
        Some(SpeechFormatter::format_list(count))
    }

    /// Moves cursor to prior list container (`+L` / `MoveToPriorList`).
    pub fn move_prior_list(&mut self) -> Option<String> {
        let target_offset = self.buffer.prior_list_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_list_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let count = node.map(|n| n.children.len()).unwrap_or(0);
        Some(SpeechFormatter::format_list(count))
    }

    /// Moves cursor to next list item (`I` / `MoveToNextListItem`).
    pub fn move_next_list_item(&mut self) -> Option<String> {
        let is_at_list = self
            .current_node()
            .map(|n| matches!(n.role, Role::List))
            .unwrap_or(false);

        let target_offset = if is_at_list {
            self.buffer
                .list_items
                .iter()
                .find(|&&l| l >= self.offset)
                .copied()?
        } else {
            self.buffer.next_list_item_from(self.offset)?
        };
        self.offset = target_offset;
        let node = self.buffer.find_list_item_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let text = node
            .map(|n| n.collect_text())
            .filter(|t| !t.is_empty())
            .or_else(|| {
                self.buffer
                    .line_at_offset(target_offset)
                    .map(|l| l.text.clone())
            })
            .unwrap_or_default();
        let has_link = node
            .map(|n| {
                n.children
                    .iter()
                    .any(|c| matches!(c.role, Role::Link { .. }))
            })
            .unwrap_or(false);
        Some(SpeechFormatter::format_list_item(&text, has_link))
    }

    /// Moves cursor to prior list item (`+I` / `MoveToPriorListItem`).
    pub fn move_prior_list_item(&mut self) -> Option<String> {
        let target_offset = self.buffer.prior_list_item_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_list_item_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let text = node
            .map(|n| n.collect_text())
            .filter(|t| !t.is_empty())
            .or_else(|| {
                self.buffer
                    .line_at_offset(target_offset)
                    .map(|l| l.text.clone())
            })
            .unwrap_or_default();
        let has_link = node
            .map(|n| {
                n.children
                    .iter()
                    .any(|c| matches!(c.role, Role::Link { .. }))
            })
            .unwrap_or(false);
        Some(SpeechFormatter::format_list_item(&text, has_link))
    }

    /// Moves cursor to next landmark / region (`R` / `;` / `MoveToNextRegion`).
    pub fn move_next_landmark(&mut self) -> Option<String> {
        let target_offset = self.buffer.next_landmark_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_landmark_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        if let Some(n) = node {
            if let Role::Landmark { ref landmark_type } = n.role {
                return Some(SpeechFormatter::format_landmark(
                    landmark_type,
                    n.name.as_deref(),
                ));
            }
        }
        Some(SpeechFormatter::format_landmark("region", None))
    }

    /// Moves cursor to prior landmark / region (`+R` / `+;` / `MoveToPriorRegion`).
    pub fn move_prior_landmark(&mut self) -> Option<String> {
        let target_offset = self.buffer.prior_landmark_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_landmark_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        if let Some(n) = node {
            if let Role::Landmark { ref landmark_type } = n.role {
                return Some(SpeechFormatter::format_landmark(
                    landmark_type,
                    n.name.as_deref(),
                ));
            }
        }
        Some(SpeechFormatter::format_landmark("region", None))
    }

    /// Moves cursor to next form control (`F` / `FocusToNextField`).
    pub fn move_next_form_control(&mut self) -> Option<String> {
        let target_offset = self.buffer.next_form_control_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_form_control_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        node.map(SpeechFormatter::format_form_control_node)
    }

    /// Moves cursor to prior form control (`+F` / `FocusToPriorField`).
    pub fn move_prior_form_control(&mut self) -> Option<String> {
        let target_offset = self.buffer.prior_form_control_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_form_control_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        node.map(SpeechFormatter::format_form_control_node)
    }

    /// Moves cursor to next button (`B` / `MoveToNextButton`).
    pub fn move_next_button(&mut self) -> Option<String> {
        let target_offset = self.buffer.next_button_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_button_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let name = node
            .and_then(|n| n.name.as_deref().or(n.text.as_deref()))
            .unwrap_or("");
        Some(SpeechFormatter::format_button(name))
    }

    /// Moves cursor to prior button (`+B` / `MoveToPriorButton`).
    pub fn move_prior_button(&mut self) -> Option<String> {
        let target_offset = self.buffer.prior_button_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_button_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let name = node
            .and_then(|n| n.name.as_deref().or(n.text.as_deref()))
            .unwrap_or("");
        Some(SpeechFormatter::format_button(name))
    }

    /// Moves cursor to next edit field (`E` / `MoveToNextEdit`).
    pub fn move_next_edit(&mut self) -> Option<String> {
        let target_offset = self.buffer.next_edit_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_edit_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let name = node.and_then(|n| n.name.as_deref()).unwrap_or("");
        let value = node.and_then(|n| n.value.as_deref());
        Some(SpeechFormatter::format_edit(name, value))
    }

    /// Moves cursor to prior edit field (`+E` / `MoveToPriorEdit`).
    pub fn move_prior_edit(&mut self) -> Option<String> {
        let target_offset = self.buffer.prior_edit_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_edit_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let name = node.and_then(|n| n.name.as_deref()).unwrap_or("");
        let value = node.and_then(|n| n.value.as_deref());
        Some(SpeechFormatter::format_edit(name, value))
    }

    /// Moves cursor to next checkbox (`X` / `MoveToNextCheckbox`).
    pub fn move_next_checkbox(&mut self) -> Option<String> {
        let target_offset = self.buffer.next_checkbox_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_checkbox_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let name = node
            .and_then(|n| n.name.as_deref().or(n.text.as_deref()))
            .unwrap_or("");
        let checked = node.and_then(|n| n.states.checked).unwrap_or(false);
        Some(SpeechFormatter::format_checkbox(name, checked))
    }

    /// Moves cursor to prior checkbox (`+X` / `MoveToPriorCheckbox`).
    pub fn move_prior_checkbox(&mut self) -> Option<String> {
        let target_offset = self.buffer.prior_checkbox_from(self.offset)?;
        self.offset = target_offset;
        let node = self.buffer.find_checkbox_node(target_offset);
        self.current_node_id = node.map(|n| n.id);
        let name = node
            .and_then(|n| n.name.as_deref().or(n.text.as_deref()))
            .unwrap_or("");
        let checked = node.and_then(|n| n.states.checked).unwrap_or(false);
        Some(SpeechFormatter::format_checkbox(name, checked))
    }

    /// Enters Forms Mode (`{ENTER}` on an Edit field or `EnterFormsMode`).
    pub fn enter_forms_mode(&mut self) -> Option<String> {
        let (role, name, value) = {
            let node = self.current_node()?;
            (
                node.role.clone(),
                node.name.clone().unwrap_or_default(),
                node.value.clone(),
            )
        };

        if matches!(role, Role::Edit) {
            self.forms_mode_active = true;
            let doc_title = self
                .buffer
                .node_map
                .values()
                .find(|n| n.role == Role::Document)
                .and_then(|n| n.name.as_deref())
                .unwrap_or("");
            Some(SpeechFormatter::format_forms_mode_entry(
                doc_title,
                &name,
                value.as_deref(),
            ))
        } else {
            None
        }
    }

    /// Exits Forms Mode back to Virtual PC Cursor (`{ESC}` or `ExitFormsMode`).
    pub fn exit_forms_mode(&mut self) -> Option<String> {
        self.forms_mode_active = false;
        Some(SpeechFormatter::format_forms_mode_exit())
    }

    /// Toggles active control state (`{SPACE}` on CheckBox or Tab).
    pub fn toggle_active_control(&mut self) -> Option<String> {
        let (node_id, role, name, current_checked) = {
            let node = self.current_node()?;
            (
                node.id,
                node.role.clone(),
                node.name.clone().unwrap_or_default(),
                node.states.checked.unwrap_or(false),
            )
        };

        match role {
            Role::CheckBox => {
                let new_checked = !current_checked;
                if let Some(node) = self.buffer.node_map.get_mut(&node_id) {
                    node.states.checked = Some(new_checked);
                }
                let doc_title = self
                    .buffer
                    .node_map
                    .values()
                    .find(|n| n.role == Role::Document)
                    .and_then(|n| n.name.as_deref())
                    .unwrap_or("");
                Some(SpeechFormatter::format_checkbox_toggle(
                    doc_title,
                    &name,
                    new_checked,
                ))
            }
            Role::Tab => {
                for node in self.buffer.node_map.values_mut() {
                    if node.role == Role::Tab {
                        if node.id == node_id {
                            node.states.selected = Some(true);
                        } else {
                            node.states.selected = Some(false);
                        }
                    }
                }
                Some(" \u{001d}selected\u{001e}".to_string())
            }
            Role::Button => Some(SpeechFormatter::format_button(&name)),
            _ => None,
        }
    }

    /// Moves cursor to next focusable control / tab stop (`{TAB}`).
    pub fn move_next_tab_stop(&mut self) -> Option<String> {
        if self.buffer.tab_stops.is_empty() {
            return None;
        }
        let is_first_tab_from_start = self.offset == 0;
        let prev_node_id = self.current_node_id;

        let maybe_next = self.buffer.next_tab_stop_from(self.offset);
        let target_offset = match maybe_next {
            Some(off) => off,
            None => *self.buffer.tab_stops.first()?,
        };
        self.offset = target_offset;
        let node = self.buffer.node_at_offset(target_offset)?.clone();
        self.current_node_id = Some(node.id);

        let doc_title = if is_first_tab_from_start {
            self.buffer.document_title()
        } else {
            None
        };

        Some(self.format_tab_stop_node(&node, prev_node_id, doc_title))
    }

    /// Moves cursor to prior focusable control / tab stop (`Shift+{TAB}`).
    pub fn move_prior_tab_stop(&mut self) -> Option<String> {
        if self.buffer.tab_stops.is_empty() {
            return None;
        }
        let prev_node_id = self.current_node_id;

        let maybe_prior = self.buffer.prior_tab_stop_from(self.offset);
        let target_offset = match maybe_prior {
            Some(off) => off,
            None => *self.buffer.tab_stops.last()?,
        };
        self.offset = target_offset;
        let node = self.buffer.node_at_offset(target_offset)?.clone();
        self.current_node_id = Some(node.id);

        Some(self.format_tab_stop_node(&node, prev_node_id, None))
    }

    fn format_tab_stop_node(
        &self,
        node: &AomNode,
        prev_node_id: Option<usize>,
        doc_title: Option<&str>,
    ) -> String {
        match node.role {
            Role::Button => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                SpeechFormatter::format_button_interactive(name, node.states.expanded, doc_title)
            }
            Role::Tab => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let selected = node.states.selected.unwrap_or(false);
                let (index, total, group_name) =
                    if let Some(info) = self.buffer.tab_info_for_node(node.id) {
                        (info.index, info.total, info.group_name.as_deref())
                    } else {
                        (1, 1, None)
                    };
                let prev_was_tab = prev_node_id
                    .and_then(|id| self.buffer.node_map.get(&id))
                    .map(|n| n.role == Role::Tab)
                    .unwrap_or(false);
                let announce_group = !prev_was_tab;
                SpeechFormatter::format_tab(
                    name,
                    selected,
                    index,
                    total,
                    group_name,
                    announce_group,
                )
            }
            Role::Edit => {
                let name = node.name.as_deref().unwrap_or("");
                let value = node.value.as_deref();
                SpeechFormatter::format_edit(name, value)
            }
            Role::CheckBox => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let checked = node.states.checked.unwrap_or(false);
                SpeechFormatter::format_checkbox(name, checked)
            }
            Role::Link { .. } => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                SpeechFormatter::format_link(name, false)
            }
            _ => node
                .name
                .clone()
                .or_else(|| node.text.clone())
                .unwrap_or_default(),
        }
    }

    /// Returns the active table column (1-indexed), if inside a table.
    pub fn current_table_col(&self) -> Option<usize> {
        self.current_table_pos.map(|(_, _, c)| c + 1)
    }

    /// Returns the active table row (1-indexed), if inside a table.
    pub fn current_table_row(&self) -> Option<usize> {
        self.current_table_pos.map(|(_, r, _)| r + 1)
    }

    /// Resolves the current table and cell coordinates based on `current_table_pos` or `offset`.
    fn resolve_table_pos(&mut self) -> Option<(usize, usize, usize)> {
        if let Some(pos) = self.current_table_pos {
            return Some(pos);
        }
        for (t_idx, table) in self.buffer.tables.iter().enumerate() {
            if self.offset >= table.start_offset && self.offset <= table.end_offset {
                for (r_idx, row) in table.cells.iter().enumerate() {
                    for (c_idx, cell) in row.iter().enumerate() {
                        if self.offset >= cell.offset {
                            self.current_table_pos = Some((t_idx, r_idx, c_idx));
                        }
                    }
                }
                if self.current_table_pos.is_none() {
                    self.current_table_pos = Some((t_idx, 0, 0));
                }
                return self.current_table_pos;
            }
        }
        None
    }

    /// Moves to the next data table (`T`).
    pub fn move_next_table(&mut self) -> Option<String> {
        if self.buffer.tables.is_empty() {
            return Some("There are no tables on this page.".to_string());
        }
        let maybe_idx = self
            .buffer
            .tables
            .iter()
            .position(|t| t.start_offset > self.offset);
        let table_idx = maybe_idx.unwrap_or(0);
        let table = &self.buffer.tables[table_idx];
        self.offset = table.start_offset;
        self.current_node_id = Some(table.table_id);
        self.current_table_pos = Some((table_idx, 0, 0));

        let first_cell_text = table
            .cells
            .first()
            .and_then(|r| r.first())
            .map(|c| c.text.as_str())
            .unwrap_or("");

        let speech = SpeechFormatter::format_table_entry(
            table.cols,
            table.rows,
            table.caption.as_deref(),
            1,
            1,
            first_cell_text,
        );
        Some(speech)
    }

    /// Moves to the prior data table (`Shift+T`).
    pub fn move_prior_table(&mut self) -> Option<String> {
        if self.buffer.tables.is_empty() {
            return Some("There are no tables on this page.".to_string());
        }
        let current_start = self
            .buffer
            .tables
            .iter()
            .rfind(|t| t.start_offset <= self.offset)
            .map(|t| t.start_offset)
            .unwrap_or(self.offset);
        let maybe_idx = self
            .buffer
            .tables
            .iter()
            .rposition(|t| t.start_offset < current_start);
        let table_idx = maybe_idx.unwrap_or(self.buffer.tables.len() - 1);
        let table = &self.buffer.tables[table_idx];
        self.offset = table.start_offset;
        self.current_node_id = Some(table.table_id);
        self.current_table_pos = Some((table_idx, 0, 0));

        let first_cell_text = table
            .cells
            .first()
            .and_then(|r| r.first())
            .map(|c| c.text.as_str())
            .unwrap_or("");

        let speech = SpeechFormatter::format_table_entry(
            table.cols,
            table.rows,
            table.caption.as_deref(),
            1,
            1,
            first_cell_text,
        );
        Some(speech)
    }

    /// Moves to cell to the right within the active row (`Ctrl+Alt+RightArrow`).
    pub fn move_table_cell_right(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = &self.buffer.tables[t_idx];
        if c_idx + 1 < table.cols {
            let new_c = c_idx + 1;
            self.current_table_pos = Some((t_idx, r_idx, new_c));
            if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(new_c)) {
                self.offset = cell.offset;
                self.current_node_id = Some(cell.node_id);
                return Some(SpeechFormatter::format_table_cell_col(
                    &cell.text,
                    new_c + 1,
                    table.cols,
                ));
            }
        }
        Some("edge of table".to_string())
    }

    /// Moves to cell to the left within the active row (`Ctrl+Alt+LeftArrow`).
    pub fn move_table_cell_left(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = &self.buffer.tables[t_idx];
        if c_idx > 0 {
            let new_c = c_idx - 1;
            self.current_table_pos = Some((t_idx, r_idx, new_c));
            if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(new_c)) {
                self.offset = cell.offset;
                self.current_node_id = Some(cell.node_id);
                return Some(SpeechFormatter::format_table_cell_col(
                    &cell.text,
                    new_c + 1,
                    table.cols,
                ));
            }
        }
        Some("edge of table".to_string())
    }

    /// Moves to cell directly below in the active column (`Ctrl+Alt+DownArrow`).
    pub fn move_table_cell_down(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = &self.buffer.tables[t_idx];
        if r_idx + 1 < table.rows {
            let new_r = r_idx + 1;
            self.current_table_pos = Some((t_idx, new_r, c_idx));
            if let Some(cell) = table.cells.get(new_r).and_then(|r| r.get(c_idx)) {
                self.offset = cell.offset;
                self.current_node_id = Some(cell.node_id);
                return Some(SpeechFormatter::format_table_cell_row(
                    &cell.text,
                    new_r + 1,
                    table.rows,
                ));
            }
        }
        Some("edge of table".to_string())
    }

    /// Moves to cell directly above in the active column (`Ctrl+Alt+UpArrow`).
    pub fn move_table_cell_up(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = &self.buffer.tables[t_idx];
        if r_idx > 0 {
            let new_r = r_idx - 1;
            self.current_table_pos = Some((t_idx, new_r, c_idx));
            if let Some(cell) = table.cells.get(new_r).and_then(|r| r.get(c_idx)) {
                self.offset = cell.offset;
                self.current_node_id = Some(cell.node_id);
                return Some(SpeechFormatter::format_table_cell_row(
                    &cell.text,
                    new_r + 1,
                    table.rows,
                ));
            }
        }
        Some("edge of table".to_string())
    }
}
