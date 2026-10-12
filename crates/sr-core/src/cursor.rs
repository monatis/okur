use crate::aom::{AomNode, Role};
use crate::buffer::VirtualBuffer;
use crate::speech::SpeechFormatter;

/// The Virtual Cursor providing reading navigation and quick key primitives over a VirtualBuffer.
#[derive(Debug, Clone)]
pub struct VirtualCursor {
    pub buffer: VirtualBuffer,
    pub offset: usize,
    pub current_node_id: Option<usize>,
}

impl VirtualCursor {
    pub fn new(buffer: VirtualBuffer) -> Self {
        Self {
            buffer,
            offset: 0,
            current_node_id: None,
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
        self.current_line()
    }

    /// Moves cursor to the end of the document (`^{END}`).
    pub fn move_to_end(&mut self) -> String {
        self.offset = self.buffer.len();
        self.current_node_id = self.buffer.spans.last().map(|s| s.node_id);
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
        let (target_offset, level) = self.buffer.next_heading_from(self.offset)?;
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
        Some(SpeechFormatter::format_heading(level, text))
    }

    /// Moves cursor to prior heading across all levels (`+H` / `MoveToPriorHeading`).
    pub fn move_prior_heading(&mut self) -> Option<String> {
        let (target_offset, level) = self.buffer.prior_heading_from(self.offset)?;
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
        Some(SpeechFormatter::format_heading(level, text))
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
}
