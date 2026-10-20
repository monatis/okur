use crate::aom::{AomNode, Role};
use crate::buffer::{BufferLine, TableCellInfo, VirtualBuffer};
use crate::messages::msg;
use crate::speech::SpeechFormatter;

/// Lines a Page Down or Page Up moves the virtual cursor: JAWS's Lines Per Page, 24 by default
/// (JAWS 2025 Settings Center help, HTML Options) [obs, 502/s7-s9].
pub const LINES_PER_PAGE: usize = 24;

/// Active text selection range within the virtual buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextSelection {
    pub anchor: usize,
    pub active: usize,
}

impl TextSelection {
    pub fn new(anchor: usize, active: usize) -> Self {
        Self { anchor, active }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.active
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.active)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.active)
    }
}

/// Active in-buffer search state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchState {
    pub query: Option<String>,
    pub case_sensitive: bool,
}

/// Where a focus is judged to come from, for the table position JAWS says with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusFrom {
    /// Where the virtual cursor was before the focus moved it.
    Cursor,
    /// Another place in the page: the virtual cursor, when it is not the previous focus.
    Offset(usize),
    /// Outside the page's tables: the focus before it was said without its table, so the
    /// next one says header and full position, "Message | ... | column 2 row 2" [obs, 216/s11].
    Outside,
}

impl FocusFrom {
    fn offset(self, cursor: usize) -> Option<usize> {
        match self {
            Self::Cursor => Some(cursor),
            Self::Offset(offset) => Some(offset),
            Self::Outside => None,
        }
    }
}

/// The Virtual Cursor providing reading navigation and quick key primitives over a VirtualBuffer.
#[derive(Debug, Clone)]
pub struct VirtualCursor {
    pub buffer: VirtualBuffer,
    pub offset: usize,
    pub current_node_id: Option<usize>,
    pub forms_mode_active: bool,
    pub current_table_pos: Option<(usize, usize, usize)>,
    pub selection: Option<TextSelection>,
    pub active_modal_root: Option<usize>,
    pub search: SearchState,
}

impl VirtualCursor {
    pub fn new(buffer: VirtualBuffer) -> Self {
        Self {
            buffer,
            offset: 0,
            current_node_id: None,
            forms_mode_active: false,
            current_table_pos: None,
            selection: None,
            active_modal_root: None,
            search: SearchState::default(),
        }
    }

    /// Returns the active AOM node at the cursor's current position.
    pub fn current_node(&self) -> Option<&AomNode> {
        if let Some(id) = self.current_node_id {
            if let Some(node) = self.buffer.node(id) {
                return Some(node);
            }
        }
        self.buffer.node_at_offset(self.offset)
    }

    /// Checks whether cursor is currently inside a modal dialog.
    pub fn is_in_modal(&self) -> bool {
        self.active_modal_root.is_some()
    }

    /// Returns the offset boundary (start, end) of the active modal dialog if any.
    pub fn active_modal_range(&self) -> Option<(usize, usize)> {
        let node_id = self.active_modal_root?;
        let dialog = self.buffer.dialog_for_node(node_id)?;
        Some((dialog.start_offset, dialog.end_offset))
    }

    /// Activates modal confinement for a dialog node and emits announcement.
    pub fn enter_modal(&mut self, node_id: usize) -> Option<String> {
        let dialog = self.buffer.dialog_for_node(node_id)?;
        self.active_modal_root = Some(node_id);
        self.offset = dialog.start_offset;
        self.current_node_id = Some(node_id);
        self.selection = None;
        Some(SpeechFormatter::format_dialog_entry(&dialog.name))
    }

    /// Exits modal confinement and returns speech announcement.
    pub fn exit_modal(&mut self) -> Option<String> {
        if self.active_modal_root.is_some() {
            self.active_modal_root = None;
            self.selection = None;
            Some(SpeechFormatter::format_dialog_exit())
        } else {
            None
        }
    }

    /// Clears active text selection.
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Selecting with the virtual cursor (JAWS's Select scripts): the selection runs from where
    /// it began to `target`, and what changed is read in JAWS's way for the key [obs, 501].
    fn select_to(&mut self, target: usize, reading: SelectionReading) -> Option<String> {
        let (min, max) = self.active_modal_range().unwrap_or((0, self.buffer.len()));
        let target = target.clamp(min, max);
        if target == self.offset {
            return None;
        }
        let anchor = self.selection.map_or(self.offset, |s| s.anchor);
        let before = self
            .selection
            .filter(|s| !s.is_empty())
            .map_or((anchor, anchor), |s| (s.start(), s.end()));
        let left_line = self.current_line_index();
        self.offset = target;
        self.current_node_id = self.buffer.line_at_offset(target).map(|l| l.node_id);
        let after = TextSelection::new(anchor, target);
        self.selection = (!after.is_empty()).then_some(after);
        let after = (after.start(), after.end());
        let added = range_gained(before, after);
        let removed = range_gained(after, before);
        let said = match reading {
            SelectionReading::Run => {
                let (range, word) = match (added, removed) {
                    (Some(range), _) => (range, msg::selection_added()),
                    (None, Some(range)) => (range, msg::selection_removed()),
                    (None, None) => return None,
                };
                let line = self.buffer.line_at_offset(range.0)?;
                let text = self.buffer.text.get(range.0..range.1).unwrap_or_default();
                format!("{}{text}\n{word}", line_marks(line))
            }
            SelectionReading::Lines => {
                let mut said = Vec::new();
                if let Some(range) = removed {
                    said.push(msg::selection_removed_lines().to_string());
                    said.extend(self.selected_lines(range));
                }
                if let Some(range) = added {
                    said.push(msg::selection_added().to_string());
                    said.extend(self.selected_lines(range));
                }
                said.join("\n")
            }
            SelectionReading::LineLeft => {
                let word = if added.is_some() {
                    msg::selection_added()
                } else {
                    msg::selection_removed()
                };
                let line = self.buffer.lines.get(left_line)?;
                format!("{word}\n{}", self.format_line_announcement(line))
            }
        };
        Some(said)
    }

    /// The lines `range` covers, each as the virtual cursor reads the part of it in `range`:
    /// its marks, then the part [obs, 501/s6, s12].
    fn selected_lines(&self, range: (usize, usize)) -> Vec<String> {
        self.buffer
            .lines
            .iter()
            .filter(|line| line.end_offset > range.0 && line.start_offset < range.1)
            .filter_map(|line| {
                let part = self
                    .buffer
                    .text
                    .get(range.0.max(line.start_offset)..range.1.min(line.end_offset))?;
                (!part.trim().is_empty()).then(|| format!("{}{part}", line_marks(line)))
            })
            .collect()
    }

    /// Extends selection to next character (`Shift+Right`): " heading level 1 T | selected"
    /// [obs, 501/s2].
    pub fn select_next_char(&mut self) -> Option<String> {
        let ch = self.buffer.char_at(self.offset)?;
        self.select_to(self.offset + ch.len_utf8(), SelectionReading::Run)
    }

    /// Extends selection to prior character (`Shift+Left`).
    pub fn select_prior_char(&mut self) -> Option<String> {
        let mut prev = self.offset.checked_sub(1)?;
        while prev > 0 && !self.buffer.text.is_char_boundary(prev) {
            prev -= 1;
        }
        self.select_to(prev, SelectionReading::Run)
    }

    /// Extends selection to the next word's start (`Ctrl+Shift+Right`): " heading level 1 e |
    /// selected" [obs, 501/s5].
    pub fn select_next_word(&mut self) -> Option<String> {
        let line_end = self
            .buffer
            .line_at_offset(self.offset)
            .map(|l| l.end_offset);
        let target = self
            .buffer
            .next_word_from(self.offset)
            .map(|w| w.start_offset)
            .or(line_end)?;
        self.select_to(target, SelectionReading::Run)
    }

    /// Extends selection to the prior word's start (`Ctrl+Shift+Left`).
    pub fn select_prior_word(&mut self) -> Option<String> {
        let target = self
            .buffer
            .prior_word_from(self.offset)
            .map_or(0, |w| w.start_offset);
        self.select_to(target, SelectionReading::Run)
    }

    /// Extends selection to the line's end or start (`Shift+End`, `Shift+Home`): "selected |
    /// heading level 1 Accessibility Engine Specification" [obs, 501/s6, s11].
    pub fn select_to_line_edge(&mut self, end: bool) -> Option<String> {
        let line = self.buffer.line_at_offset(self.offset)?;
        let target = if end {
            line.end_offset
        } else {
            line.start_offset
        };
        self.select_to(target, SelectionReading::Lines)
    }

    /// Extends selection to the next line's start (`Shift+Down`), reading the line it leaves
    /// [obs, 501/s7].
    pub fn select_next_line(&mut self) -> Option<String> {
        let target = self.buffer.next_line_from(self.offset)?.start_offset;
        self.select_to(target, SelectionReading::LineLeft)
    }

    /// Extends selection to the prior line's start (`Shift+Up`).
    pub fn select_prior_line(&mut self) -> Option<String> {
        let target = self.buffer.prior_line_from(self.offset)?.start_offset;
        self.select_to(target, SelectionReading::Lines)
    }

    /// Extends selection to the page's top or bottom (`Ctrl+Shift+Home`, `Ctrl+Shift+End`)
    /// [obs, 501/s12].
    pub fn select_to_page_edge(&mut self, end: bool) -> Option<String> {
        let target = if end { self.buffer.len() } else { 0 };
        self.select_to(target, SelectionReading::Lines)
    }

    /// Extends selection by [`LINES_PER_PAGE`] lines (`Shift+PageDown`, `Shift+PageUp`).
    pub fn select_page(&mut self, down: bool) -> Option<String> {
        let here = self.current_line_index();
        let target_line = if down {
            here.saturating_add(LINES_PER_PAGE)
        } else {
            here.saturating_sub(LINES_PER_PAGE)
        };
        let target = match self.buffer.lines.get(target_line) {
            Some(line) => line.start_offset,
            None => self.buffer.len(),
        };
        self.select_to(target, SelectionReading::Lines)
    }

    /// Ctrl+A with the virtual cursor: what was selected is unselected, and the page from where
    /// the selection began (the cursor, without one) to its end is selected. JAWS, with a
    /// selection begun at the third paragraph and extended to the top: "Unselected | the
    /// title, heading and paragraph | selected | the last two paragraphs" [obs, 501/s13].
    pub fn select_all(&mut self) -> Option<String> {
        let anchor = self.selection.map_or(self.offset, |s| s.anchor);
        let before = self
            .selection
            .filter(|s| !s.is_empty())
            .map(|s| (s.start(), s.end()));
        let end = self.buffer.len();
        self.selection = Some(TextSelection::new(anchor, end));
        self.offset = end;
        let mut said = Vec::new();
        if let Some(range) = before {
            said.push(msg::selection_removed_lines().to_string());
            said.extend(self.selected_lines(range));
        }
        said.push(msg::selection_added().to_string());
        said.extend(self.selected_lines((anchor, end)));
        Some(said.join("\n"))
    }

    /// Selects the element at the virtual cursor (F8, JAWS's SelectHTMLElement): its lines
    /// around the cursor, said as a selection is: in a table's cell, "selected | Branch"
    /// [obs, 515/s5].
    pub fn select_current_element(&mut self) -> Option<String> {
        let line = self.buffer.line_at_offset(self.offset)?;
        let (node, index) = (line.node_id, line.line_index);
        let lines = &self.buffer.lines;
        let first = lines
            .get(..=index)?
            .iter()
            .rev()
            .take_while(|l| l.node_id == node)
            .last()?;
        let last = lines
            .get(index..)?
            .iter()
            .take_while(|l| l.node_id == node)
            .last()?;
        let range = (first.start_offset, last.end_offset);
        self.selection = Some(TextSelection::new(range.0, range.1));
        let mut said = vec![msg::selection_added().to_string()];
        said.extend(self.selected_lines(range));
        Some(said.join("\n"))
    }

    /// The selection as Ctrl+C puts it on the clipboard: each line's selected part, paragraphs
    /// apart ("...paramount.\r\n\r\nRust provides..." [obs, 501/s14]); empty without one.
    pub fn selection_for_clipboard(&self) -> String {
        let Some(sel) = self.selection.filter(|s| !s.is_empty()) else {
            return String::new();
        };
        self.buffer
            .lines
            .iter()
            .filter(|line| line.end_offset > sel.start() && line.start_offset < sel.end())
            .filter_map(|line| {
                let part = self
                    .buffer
                    .text
                    .get(sel.start().max(line.start_offset)..sel.end().min(line.end_offset))?;
                let part = part.trim();
                (!part.is_empty()).then_some(part)
            })
            .collect::<Vec<_>>()
            .join("\r\n\r\n")
    }

    /// Speaks the currently selected text (`SaySelection`).
    /// The text selected with the virtual cursor, empty without a selection.
    pub fn selected_text(&self) -> String {
        let Some(sel) = self.selection.filter(|s| !s.is_empty()) else {
            return String::new();
        };
        let end = sel.end().min(self.buffer.len());
        self.buffer
            .text
            .get(sel.start().min(end)..end)
            .unwrap_or_default()
            .trim()
            .to_owned()
    }

    /// The cursor's line before the cursor, and from it to the line's end, each with the
    /// line's marks: "• same page link P" and " same page link aragraphs" with the cursor on
    /// the link's "a" [obs, 291/s14, s15].
    pub fn line_around_cursor(&self) -> (String, String) {
        let Some(line) = self.buffer.line_at_offset(self.offset) else {
            return (String::new(), String::new());
        };
        let (mark, text, speech) = split_list_mark(line);
        let marks = speech.strip_suffix(text).unwrap_or_default();
        let text_start = line.start_offset + line.text.len().saturating_sub(text.len());
        let at = self.offset.clamp(text_start, line.end_offset) - text_start;
        let before = text.get(..at).unwrap_or_default();
        let after = text.get(at..).unwrap_or_default();
        let mark = mark.map(|m| format!("{m} ")).unwrap_or_default();
        (format!("{mark}{marks}{before}"), format!("{marks}{after}"))
    }

    pub fn say_selection(&self) -> String {
        if let Some(sel) = self.selection {
            if !sel.is_empty() {
                let start = sel.start();
                let end = sel.end();
                if start < self.buffer.len() {
                    let end_bounded = end.min(self.buffer.len());
                    let text = self
                        .buffer
                        .text
                        .get(start..end_bounded)
                        .unwrap_or_default()
                        .trim();
                    if !text.is_empty() {
                        return text.to_string();
                    }
                }
            }
        }
        msg::selection_none().to_string()
    }

    /// Resets the cursor to the top of document (`^{HOME}`).
    pub fn move_to_start(&mut self) -> String {
        self.selection = None;
        if let Some((start, _)) = self.active_modal_range() {
            self.offset = start;
            self.current_node_id = self
                .buffer
                .spans
                .iter()
                .find(|s| s.start_offset >= start)
                .map(|s| s.node_id);
        } else {
            self.offset = 0;
            self.current_node_id = self.buffer.spans.first().map(|s| s.node_id);
        }
        self.current_table_pos = None;
        self.current_line()
    }

    /// Moves cursor to the end of the document (`^{END}`).
    pub fn move_to_end(&mut self) -> String {
        self.selection = None;
        if let Some((_, end)) = self.active_modal_range() {
            self.offset = end;
            self.current_node_id = self
                .buffer
                .spans
                .iter()
                .rfind(|s| s.end_offset <= end)
                .map(|s| s.node_id);
        } else {
            self.offset = self.buffer.len();
            self.current_node_id = self.buffer.spans.last().map(|s| s.node_id);
        }
        self.current_table_pos = None;
        self.current_line()
    }

    /// Formats the announcement for a line, taking into account specialized widget roles.
    pub fn format_line_announcement(&self, line: &BufferLine) -> String {
        // Lines carry their exact JAWS speech from linearization.
        if !line.speech.is_empty() || line.text.is_empty() {
            return line.speech.clone();
        }
        if let Some(node) = self.buffer.node(line.node_id) {
            match node.role {
                Role::TreeItem => {
                    let name = node
                        .name
                        .as_deref()
                        .or(node.text.as_deref())
                        .unwrap_or(&line.text);
                    let level = node.level.unwrap_or(1);
                    return SpeechFormatter::format_tree_item(name, level, node.states.expanded);
                }
                Role::ComboBox => {
                    let name = node.name.as_deref().unwrap_or(&line.text);
                    let value = node.value.as_deref();
                    let expanded = node.states.expanded.unwrap_or(false);
                    return SpeechFormatter::format_combobox(name, value, expanded);
                }
                Role::Slider => {
                    let name = node.name.as_deref().unwrap_or(&line.text);
                    let value = node.value_now.unwrap_or(0.0);
                    return SpeechFormatter::format_slider(name, value);
                }
                Role::ProgressBar => {
                    let name = node.name.as_deref().unwrap_or(&line.text);
                    let value = node.value_now.unwrap_or(0.0);
                    return SpeechFormatter::format_progressbar(name, value);
                }
                Role::IFrame => {
                    if SpeechFormatter::is_frame_line(&line.text) {
                        return line.text.clone();
                    }
                    let name = node.name.as_deref().unwrap_or(&line.text);
                    return SpeechFormatter::format_frame(name);
                }
                _ => {}
            }
        }
        SpeechFormatter::format_line(line)
    }

    /// Moves cursor down to next line (`{DOWN}`).
    pub fn move_next_line(&mut self) -> Option<String> {
        self.selection = None;
        let max_offset = if let Some((_, max_off)) = self.active_modal_range() {
            max_off
        } else {
            self.buffer.len()
        };

        if let Some(next_line) = self.buffer.next_line_from(self.offset) {
            if next_line.start_offset > max_offset {
                return None;
            }
            self.offset = next_line.start_offset;
            self.current_node_id = Some(next_line.node_id);
            Some(self.format_line_announcement(next_line))
        } else {
            // JAWS re-speaks the last line at the bottom; there is no "bottom" message (§2).
            Some(self.current_line())
        }
    }

    /// Moves cursor up to prior line (`{UP}`).
    pub fn move_prior_line(&mut self) -> Option<String> {
        self.selection = None;
        let min_offset = if let Some((min_off, _)) = self.active_modal_range() {
            min_off
        } else {
            0
        };

        if let Some(prior_line) = self.buffer.prior_line_from(self.offset) {
            if prior_line.start_offset < min_offset {
                return None;
            }
            self.offset = prior_line.start_offset;
            self.current_node_id = Some(prior_line.node_id);
            Some(self.format_line_announcement(prior_line))
        } else {
            // JAWS re-speaks the title line at the top (§2).
            Some(self.current_line())
        }
    }

    /// N and Shift+N (Insert+Enter, Insert+Shift+Enter): to text that follows a link, past the
    /// links. Text before a link in the same paragraph belongs to the links, and the page's
    /// structure (region and list lines) is passed over: JAWS stops on the heading after a
    /// navigation list, on " and more text after it." past "Some text with" and its link, and
    /// on " or call." past "Write to" and its link [obs, 289/s1-s3]. With no further text, N
    /// reads the line it is on again [obs, 289/s4-s6]; Shift+N before the first goes to the top
    /// [obs, 289/s9].
    pub fn move_to_non_link_text(&mut self, forward: bool) -> String {
        self.selection = None;
        let here = self.current_line_index();
        let targets = self.non_link_text_lines();
        let target = if forward {
            targets.iter().find(|&&i| i > here)
        } else {
            targets.iter().rfind(|&&i| i < here)
        };
        match (target, forward) {
            (Some(&line), _) => self.move_to_line(line),
            (None, false) => self.move_to_line(0),
            (None, true) => {}
        }
        self.current_line()
    }

    /// The lines N stops on, in order.
    fn non_link_text_lines(&self) -> Vec<usize> {
        let lines = &self.buffer.lines;
        let parent = |line: &BufferLine| {
            self.buffer
                .ancestors(line.node_id)
                .first()
                .map(|parent| parent.id)
        };
        let mut targets = Vec::new();
        let mut after_link = false;
        for (index, line) in lines.iter().enumerate() {
            match line.role {
                Role::Link { .. } => after_link = true,
                // The page's own line and the lines that open and close regions, lists and
                // tables are not text.
                Role::Document | Role::Landmark { .. } | Role::List | Role::Table => {}
                _ if after_link => {
                    let before_a_link = lines.get(index + 1).is_some_and(|next| {
                        matches!(next.role, Role::Link { .. }) && parent(next) == parent(line)
                    });
                    if !before_a_link {
                        targets.push(index);
                        after_link = false;
                    }
                }
                _ => {}
            }
        }
        targets
    }

    /// Speaks current line without moving cursor (`SayLine`).
    pub fn current_line(&self) -> String {
        if let Some(line) = self.buffer.line_at_offset(self.offset) {
            self.format_line_announcement(line)
        } else {
            String::new()
        }
    }

    /// Moves cursor to next word (`^{RIGHT}`).
    pub fn move_next_word(&mut self) -> Option<String> {
        self.selection = None;
        let max_offset = if let Some((_, max_off)) = self.active_modal_range() {
            max_off
        } else {
            self.buffer.len()
        };

        if let Some(next_word) = self.buffer.next_word_from(self.offset) {
            if next_word.start_offset > max_offset {
                return None;
            }
            self.offset = next_word.start_offset;
            Some(SpeechFormatter::format_word(&next_word.text))
        } else {
            None
        }
    }

    /// Moves cursor to prior word (`^{LEFT}`).
    pub fn move_prior_word(&mut self) -> Option<String> {
        self.selection = None;
        let min_offset = if let Some((min_off, _)) = self.active_modal_range() {
            min_off
        } else {
            0
        };

        if let Some(prior_word) = self.buffer.prior_word_from(self.offset) {
            if prior_word.start_offset < min_offset {
                return None;
            }
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
        self.selection = None;
        let max_offset = if let Some((_, max_off)) = self.active_modal_range() {
            max_off
        } else {
            self.buffer.len()
        };

        if self.offset >= max_offset {
            return None;
        }

        let current_char = self.buffer.char_at(self.offset)?;
        let next_offset = self.offset + current_char.len_utf8();

        if next_offset <= max_offset && next_offset < self.buffer.len() {
            self.offset = next_offset;
            let ch = self.buffer.char_at(self.offset)?;
            Some(SpeechFormatter::format_char(ch))
        } else if next_offset <= max_offset {
            self.offset = next_offset;
            None
        } else {
            None
        }
    }

    /// Moves cursor to prior character (`{LEFT}`).
    pub fn move_prior_char(&mut self) -> Option<String> {
        self.selection = None;
        let min_offset = if let Some((min_off, _)) = self.active_modal_range() {
            min_off
        } else {
            0
        };

        if self.offset <= min_offset {
            return None;
        }

        // Find preceding UTF-8 character boundary
        let mut prev = self.offset - 1;
        while prev > min_offset && !self.buffer.text.is_char_boundary(prev) {
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
        self.selection = None;
        let max_offset = if let Some((_, max_off)) = self.active_modal_range() {
            max_off
        } else {
            self.buffer.len()
        };

        if let Some(next_p) = self.buffer.next_paragraph_from(self.offset) {
            if next_p.start_offset > max_offset {
                return None;
            }
            self.offset = next_p.start_offset;
            self.current_node_id = Some(next_p.node_id);
            Some(next_p.speech.clone())
        } else {
            None
        }
    }

    /// Moves cursor up to prior paragraph / block element (`^{UP}`).
    pub fn move_prior_paragraph(&mut self) -> Option<String> {
        self.selection = None;
        let min_offset = if let Some((min_off, _)) = self.active_modal_range() {
            min_off
        } else {
            0
        };

        if let Some(prior_p) = self.buffer.prior_paragraph_from(self.offset) {
            if prior_p.start_offset < min_offset {
                return None;
            }
            self.offset = prior_p.start_offset;
            self.current_node_id = Some(prior_p.node_id);
            Some(prior_p.speech.clone())
        } else {
            None
        }
    }

    /// The text from the cursor's line to the end, as Say All reads it.
    pub fn say_all(&self) -> String {
        let lines: Vec<String> = self
            .buffer
            .lines
            .iter()
            .skip(self.current_line_index())
            .map(|l| self.format_line_announcement(l))
            .collect();

        lines.join("\n")
    }

    /// The index of the line the cursor is on.
    pub fn current_line_index(&self) -> usize {
        self.buffer
            .line_at_offset(self.offset)
            .map(|l| l.line_index)
            .unwrap_or(0)
    }

    /// Line `index` as the virtual cursor reads it, or `None` past the last line.
    pub fn line_text(&self, index: usize) -> Option<String> {
        self.buffer
            .lines
            .get(index)
            .map(|line| self.format_line_announcement(line))
    }

    /// Moves to the start of line `index`, as Say All reaches it.
    pub fn move_to_line(&mut self, index: usize) {
        if let Some(line) = self.buffer.lines.get(index) {
            self.offset = line.start_offset;
            self.current_node_id = Some(line.node_id);
            self.selection = None;
        }
    }

    /// Shared quick-navigation movement (JAWS spec §11–§17).
    ///
    /// Moves to the next (or prior) offset in `targets`, wrapping with the JAWS
    /// `wrapping to top` / `wrapping to bottom` prefix, or speaks `none` when the page has no
    /// targets. `speak` renders the target. Respects an active modal dialog's range.
    fn quick_nav(
        &mut self,
        targets: &[usize],
        forward: bool,
        none: &str,
        speak: impl Fn(&Self, usize, usize) -> String,
    ) -> Option<String> {
        self.selection = None;
        let (min_off, max_off) = self.active_modal_range().unwrap_or((0, self.buffer.len()));
        let in_range: Vec<usize> = targets
            .iter()
            .copied()
            .filter(|&o| o >= min_off && o <= max_off)
            .collect();
        if in_range.is_empty() {
            return Some(none.to_owned());
        }
        let current_line_start = self
            .buffer
            .line_at_offset(self.offset)
            .map(|l| l.start_offset)
            .unwrap_or(self.offset);
        let (target, wrapped) = if forward {
            match in_range.iter().copied().find(|&o| o > self.offset) {
                Some(o) => (o, None),
                None => (in_range.first().copied()?, Some(msg::quicknav_wrap_top())),
            }
        } else {
            match in_range.iter().copied().rfind(|&o| o < current_line_start) {
                Some(o) => (o, None),
                None => (in_range.last().copied()?, Some(msg::quicknav_wrap_bottom())),
            }
        };
        let previous = self.offset;
        self.offset = target;
        self.current_node_id = self.buffer.line_at_offset(target).map(|l| l.node_id);
        let speech = speak(self, target, previous);
        Some(match wrapped {
            Some(prefix) => format!("{prefix}\n{speech}"),
            None => speech,
        })
    }

    /// The speech of the line starting at `offset`, as Down Arrow would speak it.
    fn line_speech_at(&self, offset: usize) -> String {
        self.buffer
            .line_at_offset(offset)
            .map(|l| self.format_line_announcement(l))
            .unwrap_or_default()
    }

    /// Form control speech in the JAWS focus phrasing, used by F/E/X/A/C/B (§17). `previous`
    /// is the cursor offset before the move: entering a radio group names the group first.
    /// Focus moved to `node_id` in the page (Tab, or the application moving focus): the
    /// virtual cursor follows, and JAWS speaks the control as the F/E/X quick keys do, plus
    /// the usage hint for check boxes, links and buttons [obs, 204].
    pub fn focus_node(&mut self, node_id: usize) -> Option<String> {
        self.focus_node_from(node_id, FocusFrom::Cursor)
    }

    /// A focus said as the virtual cursor's line it lands on, with its usage hint: the first
    /// focus in a page on a link, "• same page link Current Page README", its list item's
    /// bullet first [obs, 214/s8], with no table position or region [obs, 216/s10, 217/s11].
    /// A node sharing its line with other text is left to the focus phrasing (`None`).
    pub fn focus_line(&mut self, node_id: usize) -> Option<String> {
        self.move_to_node_id(node_id);
        let node = self.buffer.node(node_id)?;
        let line = self
            .buffer
            .line_at_offset(self.offset)
            .filter(|l| l.node_id == node_id)?;
        let mut speech = self.format_line_announcement(line);
        if let Some(hint) = SpeechFormatter::web_focus_hint(node) {
            speech.push('\n');
            speech.push_str(hint);
        }
        Some(speech)
    }

    /// [`Self::focus_node`], with the table position judged from `from`: JAWS says a table's
    /// header and position as entered from its virtual cursor (GitHub's files table from its
    /// heading, dev box).
    pub fn focus_node_from(&mut self, node_id: usize, from: FocusFrom) -> Option<String> {
        let previous = self.offset;
        self.move_to_node_id(node_id);
        let node = self.buffer.node(node_id)?;
        // A focus with nothing of it in the buffer (Google Docs' empty element it types
        // into) is said by its name, not by the cursor's line, which is some other element's;
        // a group adds its role (Google Sheets' grid, back from a cell's editor: JAWS on the
        // dev box said "group", 2026-10-16).
        if !self.buffer.shows(node_id) {
            let name = node.name.as_deref().unwrap_or_default().trim();
            return Some(match node.role {
                Role::Group => format!(
                    "{name}{}",
                    crate::linearize::role_marker(msg::role_group().as_str())
                )
                .trim()
                .to_owned(),
                _ => name.to_owned(),
            });
        }
        let attr = |k: &str| node.attributes.get(k).map(String::as_str);
        let name = node.name.as_deref().unwrap_or_default().trim();
        let mut speech = match &node.role {
            // Unlike the C quick key, Tab omits the collapsed state.
            // One typed in keeps its state: "Fruit edit combo collapsed | blank" (JAWS on
            // the dev box).
            Role::ComboBox if attr("editable") == Some("true") => {
                SpeechFormatter::format_edit_combo(name, node)
            }
            Role::ComboBox => format!(
                "{name}  \u{1d} {} \u{1e}  \n{}",
                msg::role_combo_box_focus(),
                node.value.as_deref().unwrap_or_default()
            ),
            Role::Link { .. } => SpeechFormatter::format_link_focus(name, node),
            // A tab: its tab list as a group when focus enters it, then name, position and
            // selection [obs, 29/s2-s3].
            Role::Tab => {
                let info = self.buffer.tab_info_for_node(node_id);
                let entering = info.is_some_and(|i| {
                    !self
                        .buffer
                        .tabs
                        .iter()
                        .any(|t| t.group_name == i.group_name && t.offset == previous)
                });
                let mut lines = Vec::new();
                if let Some(group) = info
                    .and_then(|i| i.group_name.as_deref())
                    .filter(|_| entering)
                {
                    lines.push(format!("{group} \u{1d} {} \u{1e} \n", msg::role_group()));
                }
                lines.push(format!("{name}  \u{1d} {} \u{1e} ", msg::role_tab()));
                if let Some(i) = info {
                    lines.push(msg::position(i.index, i.total).render());
                }
                if node.states.selected == Some(true) {
                    lines.push(format!(" \u{1d}{}\u{1e}", msg::state_selected()));
                }
                lines.join("\n")
            }
            Role::Edit if attr("password") == Some("true") => format!(
                "{}  \u{1d}\u{1e}",
                self.control_focus_speech(self.offset, previous)
            ),
            // A grid's cell that takes the focus itself is said by its name, all of it, as
            // JAWS says WhatsApp's chats moving through its list (the chat's name, its last
            // message's date and the message; dev box, 2026-10-17).
            Role::TableCell if !name.is_empty() => name.to_owned(),
            _ => self.control_focus_speech(self.offset, previous),
        };
        if speech.trim().is_empty() {
            speech = name.to_owned();
        }
        // Such a cell says its position only entering the grid: JAWS said no row moving on.
        let focused_cell = node.role == Role::TableCell && !name.is_empty();
        if let Some((header, position, whole)) =
            self.table_focus_context(self.offset, from.offset(previous))
        {
            if let Some(header) = header {
                speech = format!("{header}\n{speech}");
            }
            if !focused_cell || whole {
                speech.push('\n');
                speech.push_str(&position);
            }
        }
        if let Some(hint) = SpeechFormatter::web_focus_hint(node) {
            speech.push('\n');
            speech.push_str(hint);
        }
        Some(speech)
    }

    /// A focus in a table cell, as JAWS says it on Tab (dev box, 2026-10-16): the column
    /// header when the column changed, and the position that changed after the control:
    /// "Name | alpha.rs same page link | column 1 row 2", then "Message | first commit ... |
    /// Column 2", or "gamma.rs ... | Row 4" further down the same column. The flag says the
    /// position names both the column and the row.
    fn table_focus_context(
        &self,
        offset: usize,
        previous: Option<usize>,
    ) -> Option<(Option<String>, String, bool)> {
        let cell_at = |at: usize| {
            let table = self.buffer.table_at_offset(at)?;
            let cell = table
                .cells
                .iter()
                .flatten()
                .filter(|c| c.offset <= at)
                .max_by_key(|c| c.offset)?;
            Some((table.table_id, cell.row, cell.col, cell.col_header.clone()))
        };
        let (table, row, col, header) = cell_at(offset)?;
        let before = previous
            .and_then(cell_at)
            .filter(|(t, ..)| *t == table)
            .map(|(_, r, c, _)| (r, c));
        let header = match before {
            Some((_, c)) if c == col => None,
            _ => header.filter(|h| !h.trim().is_empty()),
        };
        let (position, whole) = match before {
            Some((r, _)) if r == row => (msg::table_column(col).render(), false),
            Some((_, c)) if c == col => (msg::table_row(row).render(), false),
            _ => (msg::table_focus_cell(col, row).render(), true),
        };
        Some((header, position, whole))
    }

    /// The control at the virtual cursor as its quick key says it, its label with it; the line
    /// itself where it is not a control.
    pub fn control_speech_here(&self) -> String {
        let line = self.buffer.line_at_offset(self.offset);
        let is_control = line
            .and_then(|l| self.buffer.node(l.node_id))
            .is_some_and(|n| {
                matches!(
                    n.role,
                    Role::ComboBox | Role::Edit | Role::CheckBox | Role::RadioButton | Role::Button
                )
            });
        match line {
            Some(l) if is_control => self.control_focus_speech(l.start_offset, self.offset),
            _ => self.current_line(),
        }
    }

    fn control_focus_speech(&self, offset: usize, previous: usize) -> String {
        let Some(node) = self
            .buffer
            .line_at_offset(offset)
            .and_then(|l| self.buffer.node(l.node_id))
        else {
            return String::new();
        };
        SpeechFormatter::format_control_focus(node, |node_id| {
            // The container's own span is the widest one carrying its id (its start and
            // end lines have narrower spans of their own).
            self.buffer
                .spans
                .iter()
                .filter(|s| s.node_id == node_id)
                .max_by_key(|s| s.end_offset - s.start_offset)
                .is_some_and(|s| previous > s.start_offset && previous < s.end_offset)
        })
    }

    fn heading_speech_at(&self, offset: usize) -> String {
        let level = self
            .buffer
            .headings
            .iter()
            .find(|h| h.0 == offset)
            .map(|h| h.1)
            .unwrap_or(1);
        let line = self.buffer.line_at_offset(offset);
        let text = line.map(|l| l.text.clone()).unwrap_or_default();
        // A heading that is a button says the button after the heading.
        let button = line
            .and_then(|l| self.buffer.node(l.node_id))
            .filter(|n| matches!(n.role, Role::Button | Role::MenuButton))
            .map(crate::linearize::button_suffix)
            .unwrap_or_default();
        format!("{}{button}", SpeechFormatter::format_heading(level, &text))
    }

    /// Moves cursor to next heading across all levels (`H` / `MoveToNextHeading`).
    pub fn move_next_heading(&mut self) -> Option<String> {
        let targets: Vec<usize> = self.buffer.headings.iter().map(|h| h.0).collect();
        self.quick_nav(
            &targets,
            true,
            msg::quicknav_none_headings().as_str(),
            |c, o, _| c.heading_speech_at(o),
        )
    }

    /// Moves cursor to prior heading across all levels (`Shift+H`).
    pub fn move_prior_heading(&mut self) -> Option<String> {
        let targets: Vec<usize> = self.buffer.headings.iter().map(|h| h.0).collect();
        self.quick_nav(
            &targets,
            false,
            msg::quicknav_none_headings().as_str(),
            |c, o, _| c.heading_speech_at(o),
        )
    }

    /// Moves to the page's first (`last` false) or last heading: Alt+Insert+Home and End. The
    /// heading alone is said, as its name and level: "Related stories | heading level 2"
    /// [obs, 295/s1, s2].
    pub fn move_edge_heading(&mut self, last: bool) -> Option<String> {
        self.selection = None;
        let target = if last {
            self.buffer.headings.last()
        } else {
            self.buffer.headings.first()
        }
        .map(|h| h.0);
        let Some(target) = target else {
            return Some(msg::quicknav_none_headings().to_string());
        };
        self.offset = target;
        self.current_node_id = self.buffer.line_at_offset(target).map(|l| l.node_id);
        Some(self.heading_speech_at(target))
    }

    /// Moves to the first or last heading of `level`: Ctrl+Alt+Insert and the level, with
    /// Shift for the last. As a level key does, it says the higher headings it walks past
    /// first: "Opening schedule | heading level 2 | What readers said | heading level 3"
    /// [obs, 295/s3-s6].
    pub fn move_edge_heading_level(&mut self, level: u32, last: bool) -> Option<String> {
        self.selection = None;
        let mut of_level = self.buffer.headings.iter().filter(|&&(_, l)| l == level);
        let target = if last {
            of_level.next_back()
        } else {
            of_level.next()
        }
        .map(|h| h.0);
        Some(match target {
            Some(off) => self.move_to_heading_level(off, level),
            None => Self::no_more_headings(level),
        })
    }

    /// Moves to the page's first or last form field (Ctrl+Insert+Home and End), said as F
    /// says one; the focus stays where it was [obs, 296/s1, s3].
    pub fn move_edge_form_control(&mut self, last: bool) -> Option<String> {
        self.selection = None;
        let target = if last {
            self.buffer.form_controls.last()
        } else {
            self.buffer.form_controls.first()
        }
        .copied();
        let Some(target) = target else {
            return Some(msg::quicknav_none_form_fields().to_string());
        };
        let previous = self.offset;
        self.offset = target;
        self.current_node_id = self.buffer.line_at_offset(target).map(|l| l.node_id);
        Some(self.control_focus_speech(target, previous))
    }

    /// Heading level keys do not wrap (§12).
    fn no_more_headings(level: u32) -> String {
        if level == 1 {
            msg::quicknav_heading_level_none_top().to_string()
        } else {
            msg::quicknav_heading_level_none(level).render()
        }
    }

    /// The headings above `level` that a level key walks past from the cursor's line (`from`,
    /// its start) to the heading at `target`, each on a line of its own, said before it. JAWS
    /// says the ones still open where the walk ends: a heading closes the ones of its level and
    /// below walked before it. So 3 from "Voices" to "Reading" says "Reference" and "Commands",
    /// and Shift+3 back from "Reading" to "Voices" says "Reference" alone [obs, 275/s10, s11].
    /// Walking forward starts after the cursor's line: 2 from "User guide" says "Install" alone
    /// [obs, 277/s2, s8]; from the top of the page, above the first heading, it says "User guide"
    /// too (275/s1). Walking back starts on the cursor's line: Shift+2 from "Reference" says
    /// "Reference", then "Configure" (277/s4, s11).
    fn passed_headings(&self, from: usize, target: usize, level: u32) -> String {
        let forward = target > from;
        let mut passed: Vec<(usize, u32)> = self
            .buffer
            .headings
            .iter()
            .copied()
            .filter(|&(off, _)| {
                if forward {
                    off > from && off < target
                } else {
                    off > target && off <= from
                }
            })
            .collect();
        if !forward {
            passed.reverse();
        }
        let mut open: Vec<(usize, u32)> = Vec::new();
        for (off, l) in passed {
            while open.last().is_some_and(|&(_, top)| top >= l) {
                open.pop();
            }
            open.push((off, l));
        }
        open.iter()
            .filter(|&&(_, l)| l < level)
            .map(|&(off, l)| {
                let text = self
                    .buffer
                    .line_at_offset(off)
                    .map(|line| line.text.as_str())
                    .unwrap_or_default();
                format!("{}\n", SpeechFormatter::format_passed_heading(l, text))
            })
            .collect()
    }

    /// Home or End with the virtual cursor: to the first or last character of the line
    /// (JAWSHome, JAWSEnd), saying nothing [obs, 502/s5, s6].
    pub fn move_to_line_edge(&mut self, end: bool) {
        self.selection = None;
        if let Some(line) = self.buffer.line_at_offset(self.offset) {
            self.offset = if end {
                line.end_offset.saturating_sub(1).max(line.start_offset)
            } else {
                line.start_offset
            };
        }
    }

    /// Page Down or Page Up with the virtual cursor: [`LINES_PER_PAGE`] lines on, or back, and
    /// the line reached is read; at the page's end or top, its last or first line
    /// [obs, 502/s7-s11].
    pub fn move_page(&mut self, down: bool) -> String {
        self.selection = None;
        let here = self.current_line_index();
        let last = self.buffer.lines.len().saturating_sub(1);
        let target = if down {
            here.saturating_add(LINES_PER_PAGE).min(last)
        } else {
            here.saturating_sub(LINES_PER_PAGE)
        };
        self.move_to_line(target);
        self.current_line()
    }

    /// The virtual cursor's sentences, as offsets and what is said: a list item's mark alone,
    /// then each sentence of its line, the line's marks with the first ("•", then " same page
    /// link Paragraphs" [obs, 291/s6-s9]). A line without a sentence is one.
    fn sentence_units(&self) -> Vec<(usize, String)> {
        let mut units = Vec::new();
        for line in &self.buffer.lines {
            let (mark, text, speech) = split_list_mark(line);
            if let Some(mark) = mark {
                units.push((line.start_offset, mark.to_owned()));
            }
            let base = line.start_offset + line.text.len().saturating_sub(text.len());
            let marks = speech.strip_suffix(text).unwrap_or_default();
            let ranges = crate::editing::sentences(text);
            if ranges.is_empty() && mark.is_none() {
                units.push((line.start_offset, line.speech.clone()));
            }
            for (i, (start, end)) in ranges.into_iter().enumerate() {
                let part = text.get(start..end).unwrap_or_default();
                let said = if i == 0 {
                    format!("{marks}{part}")
                } else {
                    part.to_owned()
                };
                units.push((base + start, said));
            }
        }
        units
    }

    /// Alt+NumpadPlus and Alt+NumpadMinus: the next or prior sentence, read; at the page's end
    /// or top, the sentence the cursor is in [obs, 291/s6-s8].
    pub fn move_sentence(&mut self, forward: bool) -> String {
        self.selection = None;
        let units = self.sentence_units();
        let here = self.offset;
        let current = units.iter().rposition(|(offset, _)| *offset <= here);
        let target = if forward {
            units.iter().position(|(offset, _)| *offset > here)
        } else {
            current.and_then(|i| i.checked_sub(1))
        };
        let Some((offset, said)) = target.or(current).and_then(|i| units.get(i)).cloned() else {
            return String::new();
        };
        self.offset = offset;
        self.current_node_id = self.buffer.line_at_offset(offset).map(|l| l.node_id);
        said
    }

    /// Alt+Numpad5: the sentence the cursor is in [obs, 291/s9].
    pub fn current_sentence(&self) -> String {
        let units = self.sentence_units();
        units
            .iter()
            .rev()
            .find(|(offset, _)| *offset <= self.offset)
            .or(units.first())
            .map(|(_, said)| said.clone())
            .unwrap_or_default()
    }

    /// Ctrl+Numpad5: the paragraph the cursor is in [obs, 291/s10].
    pub fn current_paragraph(&self) -> String {
        self.buffer
            .paragraph_at_offset(self.offset)
            .filter(|p| p.start_offset <= self.offset)
            .map_or_else(|| self.current_line(), |p| p.speech.clone())
    }

    /// The start of the cursor's line.
    fn current_line_start(&self) -> usize {
        self.buffer
            .line_at_offset(self.offset)
            .map(|l| l.start_offset)
            .unwrap_or(self.offset)
    }

    /// Moves the cursor to the heading of `level` at `target`, saying the higher headings it
    /// passes first.
    fn move_to_heading_level(&mut self, target: usize, level: u32) -> String {
        let passed = self.passed_headings(self.current_line_start(), target, level);
        self.offset = target;
        self.current_node_id = self.buffer.line_at_offset(target).map(|l| l.node_id);
        format!("{passed}{}", self.heading_speech_at(target))
    }

    /// Moves cursor to next heading of `level` (`1`–`6`).
    pub fn move_next_heading_level(&mut self, level: u32) -> Option<String> {
        self.selection = None;
        let target = self
            .buffer
            .headings
            .iter()
            .find(|&&(off, l)| l == level && off > self.offset)
            .map(|h| h.0);
        Some(match target {
            Some(off) => self.move_to_heading_level(off, level),
            None => Self::no_more_headings(level),
        })
    }

    /// Moves cursor to prior heading of `level` (`Shift+1`–`Shift+6`).
    pub fn move_prior_heading_level(&mut self, level: u32) -> Option<String> {
        self.selection = None;
        let current_line_start = self.current_line_start();
        let target = self
            .buffer
            .headings
            .iter()
            .rfind(|&&(off, l)| l == level && off < current_line_start)
            .map(|h| h.0);
        Some(match target {
            Some(off) => self.move_to_heading_level(off, level),
            None => Self::no_more_headings(level),
        })
    }

    /// Moves cursor to next link (`U` / `MoveToNextLink`).
    pub fn move_next_link(&mut self) -> Option<String> {
        let targets = self.buffer.links.clone();
        self.quick_nav(
            &targets,
            true,
            msg::quicknav_none_links().as_str(),
            |c, o, _| c.line_speech_at(o),
        )
    }

    /// Moves cursor to prior link (`Shift+U`).
    pub fn move_prior_link(&mut self) -> Option<String> {
        let targets = self.buffer.links.clone();
        self.quick_nav(
            &targets,
            false,
            msg::quicknav_none_links().as_str(),
            |c, o, _| c.line_speech_at(o),
        )
    }

    /// Activates an in-page fragment link at the cursor and jumps directly to its anchor target.
    pub fn activate_current_link(&mut self) -> Option<String> {
        let current_offset = self.offset;
        let link_node = self
            .current_node()
            .and_then(|n| match n.role {
                Role::Link { .. } => Some(n),
                _ => None,
            })
            .or_else(|| self.buffer.find_link_node(current_offset))
            .or_else(|| {
                self.buffer
                    .spans
                    .iter()
                    .find(|s| {
                        s.start_offset <= current_offset
                            && s.end_offset >= current_offset
                            && matches!(s.role, Role::Link { .. })
                    })
                    .and_then(|s| self.buffer.node(s.node_id))
            });

        if let Some(node) = link_node {
            if let Role::Link {
                href: Some(ref href),
            } = node.role
            {
                if let Some(target_id) = href.strip_prefix('#') {
                    if let Some(&target_offset) = self.buffer.target_anchors.get(target_id) {
                        self.offset = target_offset;
                        self.current_node_id =
                            self.buffer.node_at_offset(target_offset).map(|n| n.id);
                        self.selection = None;
                        return Some(self.current_line());
                    }
                }
            }
        }
        None
    }

    /// Moves cursor to next inline frame (`M` / `MoveToNextFrame`).
    pub fn move_next_frame(&mut self) -> Option<String> {
        self.selection = None;
        if self.buffer.frames.is_empty() {
            return Some(msg::quicknav_none_frames().to_string());
        }

        if let Some(frame) = self.buffer.next_frame_from(self.offset) {
            self.offset = frame.start_offset;
            self.current_node_id = Some(frame.node_id);
            Some(SpeechFormatter::format_frame(&frame.name))
        } else if let Some(frame) = self.buffer.first_frame() {
            self.offset = frame.start_offset;
            self.current_node_id = Some(frame.node_id);
            Some(format!(
                "{}\n{}",
                msg::quicknav_wrap_top(),
                SpeechFormatter::format_frame(&frame.name)
            ))
        } else {
            None
        }
    }

    /// Moves cursor to prior inline frame (`Shift+M` / `MoveToPriorFrame`).
    pub fn move_prior_frame(&mut self) -> Option<String> {
        self.selection = None;
        if self.buffer.frames.is_empty() {
            return Some(msg::quicknav_none_frames().to_string());
        }

        if let Some(frame) = self.buffer.prior_frame_from(self.offset) {
            self.offset = frame.start_offset;
            self.current_node_id = Some(frame.node_id);
            Some(SpeechFormatter::format_frame(&frame.name))
        } else if let Some(frame) = self.buffer.last_frame() {
            self.offset = frame.start_offset;
            self.current_node_id = Some(frame.node_id);
            Some(format!(
                "{}\n{}",
                msg::quicknav_wrap_bottom(),
                SpeechFormatter::format_frame(&frame.name)
            ))
        } else {
            None
        }
    }

    fn line_nav(&mut self, targets: Vec<usize>, forward: bool, none: &str) -> Option<String> {
        self.quick_nav(&targets, forward, none, |c, o, _| c.line_speech_at(o))
    }

    fn control_nav(&mut self, targets: Vec<usize>, forward: bool, none: &str) -> Option<String> {
        self.quick_nav(&targets, forward, none, |c, o, previous| {
            c.control_focus_speech(o, previous)
        })
    }

    /// Moves to the next or prior division (`Z`, `Shift+Z`): the page's `div` elements the
    /// Divisions list holds, read as their line [obs, 292/s6-s10].
    pub fn move_division(&mut self, forward: bool) -> Option<String> {
        let targets: Vec<usize> = crate::element_lists::ElementList::Divisions
            .items(&self.buffer)
            .iter()
            .map(|item| item.offset)
            .collect();
        self.line_nav(targets, forward, msg::quicknav_none_divisions().as_str())
    }

    /// Moves to the next or prior horizontal rule (`Minus`, `Shift+Minus`): "----- |
    /// separator" [obs, 292/s1-s4].
    pub fn move_separator(&mut self, forward: bool) -> Option<String> {
        let targets = self.buffer.separators.clone();
        self.line_nav(targets, forward, msg::quicknav_none_separators().as_str())
    }

    /// Moves to the next or prior link that writes an e-mail (`Backslash`, `Shift+Backslash`),
    /// said by its text and then "Send Mail Link" [obs, 293/s2, s3].
    pub fn move_mail_link(&mut self, forward: bool) -> Option<String> {
        let mail_link = |c: &Self, offset: usize| {
            c.buffer
                .line_at_offset(offset)
                .and_then(|line| c.buffer.node(line.node_id))
                .is_some_and(|node| matches!(&node.role, Role::Link { href: Some(h) } if h.starts_with("mailto:")))
        };
        let targets: Vec<usize> = self
            .buffer
            .links
            .iter()
            .copied()
            .filter(|&offset| mail_link(self, offset))
            .collect();
        self.quick_nav(
            &targets,
            forward,
            msg::quicknav_none_mail_links().as_str(),
            |c, offset, _| {
                let text = c
                    .buffer
                    .line_at_offset(offset)
                    .map(|line| line.text.trim().to_owned())
                    .unwrap_or_default();
                format!(
                    "{text}{}",
                    crate::linearize::role_marker(msg::role_link_mailto().as_str())
                )
            },
        )
    }

    /// Moves to the start or the end of the element the cursor is in (Shift+Comma and
    /// Shift+Period, Alt+Windows+Home and End): of the elements with a line of their own at
    /// both ends, the innermost, read as that line ("list of 2 items", "list end"); from a
    /// list's item it is the list [obs, 294].
    pub fn move_element_edge(&mut self, end: bool) -> Option<String> {
        self.selection = None;
        let here = self
            .buffer
            .line_at_offset(self.offset)
            .map(|l| l.start_offset)
            .unwrap_or(self.offset);
        let (start, end_line) = self
            .buffer
            .bounded
            .iter()
            .copied()
            .filter(|&(start, end_line)| start <= here && here <= end_line)
            .max_by_key(|&(start, end_line)| (start, std::cmp::Reverse(end_line)))?;
        let target = if end { end_line } else { start };
        self.offset = target;
        self.current_node_id = self.buffer.line_at_offset(target).map(|l| l.node_id);
        Some(self.line_speech_at(target))
    }

    /// Moves cursor to next list container (`L` / `MoveToNextList`).
    pub fn move_next_list(&mut self) -> Option<String> {
        let t = self.buffer.lists.clone();
        self.line_nav(t, true, msg::quicknav_none_lists().as_str())
    }

    /// Moves cursor to prior list container (`Shift+L`).
    pub fn move_prior_list(&mut self) -> Option<String> {
        let t = self.buffer.lists.clone();
        self.line_nav(t, false, msg::quicknav_none_lists().as_str())
    }

    /// Moves cursor to next list item (`I`).
    pub fn move_next_list_item(&mut self) -> Option<String> {
        let t = self.buffer.list_items.clone();
        self.line_nav(t, true, msg::quicknav_none_list_items().as_str())
    }

    /// Moves cursor to prior list item (`Shift+I`).
    pub fn move_prior_list_item(&mut self) -> Option<String> {
        let t = self.buffer.list_items.clone();
        self.line_nav(t, false, msg::quicknav_none_list_items().as_str())
    }

    /// Moves cursor to next region (`R`, §15).
    pub fn move_next_landmark(&mut self) -> Option<String> {
        let t = self.buffer.landmarks.clone();
        self.line_nav(t, true, msg::quicknav_none_regions().as_str())
    }

    /// Moves cursor to prior region (`Shift+R`).
    pub fn move_prior_landmark(&mut self) -> Option<String> {
        let t = self.buffer.landmarks.clone();
        self.line_nav(t, false, msg::quicknav_none_regions().as_str())
    }

    /// Moves cursor to next graphic (`G`, §16).
    pub fn move_next_graphic(&mut self) -> Option<String> {
        let t = self.buffer.graphics.clone();
        self.line_nav(t, true, msg::quicknav_none_graphics().as_str())
    }

    /// Moves cursor to prior graphic (`Shift+G`).
    pub fn move_prior_graphic(&mut self) -> Option<String> {
        let t = self.buffer.graphics.clone();
        self.line_nav(t, false, msg::quicknav_none_graphics().as_str())
    }

    /// Moves cursor to next article (`O`).
    pub fn move_next_article(&mut self) -> Option<String> {
        let t = self.buffer.articles.clone();
        self.line_nav(t, true, msg::quicknav_none_articles().as_str())
    }

    /// Moves cursor to prior article (`Shift+O`).
    pub fn move_prior_article(&mut self) -> Option<String> {
        let t = self.buffer.articles.clone();
        self.line_nav(t, false, msg::quicknav_none_articles().as_str())
    }

    /// Moves cursor to the main region (`Q`). JAWS's message for a page without a main
    /// region has not been recorded, so that case is silent rather than guessed.
    pub fn move_to_main_region(&mut self) -> Option<String> {
        let target = self.buffer.main_region?;
        self.selection = None;
        self.offset = target;
        self.current_node_id = self.buffer.line_at_offset(target).map(|l| l.node_id);
        Some(self.line_speech_at(target))
    }

    /// Moves cursor to next element with an onMouseOver handler (`;`). UIA does not expose
    /// event handlers, so the index stays empty until a source (IAccessible2) provides it.
    pub fn move_next_on_mouse_over(&mut self) -> Option<String> {
        let t = self.buffer.on_mouse_over.clone();
        self.line_nav(t, true, msg::quicknav_none_on_mouse_over().as_str())
    }

    /// Moves cursor to prior onMouseOver element (`Shift+;`).
    pub fn move_prior_on_mouse_over(&mut self) -> Option<String> {
        let t = self.buffer.on_mouse_over.clone();
        self.line_nav(t, false, msg::quicknav_none_on_mouse_over().as_str())
    }

    /// Moves to the next or prior element with a click handler (`/`, `Shift+/`). JAWS in
    /// Chrome found none on a page with an `onclick` division: "There are no clickable
    /// elements on this page." [obs, 293/s4-s6].
    pub fn move_on_click(&mut self, forward: bool) -> Option<String> {
        let t = self.buffer.on_click.clone();
        self.line_nav(t, forward, msg::quicknav_none_clickable().as_str())
    }

    /// Follows aria-flowto from the element at the cursor (`=`), or back to the element that
    /// flows to it (`Shift+=`), reading the line it lands on; without one, "No more flows
    /// found" [obs, 293/s8, s9].
    pub fn move_flow(&mut self, to: bool) -> Option<String> {
        self.selection = None;
        let here = self
            .buffer
            .line_at_offset(self.offset)
            .map_or(self.offset, |l| l.start_offset);
        let target = self.buffer.flows.iter().find_map(|&(from, into)| match to {
            true => (from == here).then_some(into),
            false => (into == here).then_some(from),
        });
        let Some(target) = target else {
            return Some(msg::quicknav_none_flows().to_string());
        };
        self.offset = target;
        self.current_node_id = self.buffer.line_at_offset(target).map(|l| l.node_id);
        Some(self.line_speech_at(target))
    }

    /// Moves cursor to next tab (`'`). The no-tabs message is unrecorded, so it is silent.
    pub fn move_next_tab(&mut self) -> Option<String> {
        let t: Vec<usize> = self.buffer.tabs.iter().map(|t| t.offset).collect();
        if t.is_empty() {
            return None;
        }
        self.line_nav(t, true, "")
    }

    /// Moves cursor to prior tab (`Shift+'`).
    pub fn move_prior_tab(&mut self) -> Option<String> {
        let t: Vec<usize> = self.buffer.tabs.iter().map(|t| t.offset).collect();
        if t.is_empty() {
            return None;
        }
        self.line_nav(t, false, "")
    }

    /// Moves cursor to next form field (`F`).
    pub fn move_next_form_control(&mut self) -> Option<String> {
        let t = self.buffer.form_controls.clone();
        self.control_nav(t, true, msg::quicknav_none_form_fields().as_str())
    }

    /// Moves cursor to prior form field (`Shift+F`).
    pub fn move_prior_form_control(&mut self) -> Option<String> {
        let t = self.buffer.form_controls.clone();
        self.control_nav(t, false, msg::quicknav_none_form_fields().as_str())
    }

    /// Moves cursor to next button (`B`).
    pub fn move_next_button(&mut self) -> Option<String> {
        let t = self.buffer.buttons.clone();
        self.control_nav(t, true, msg::quicknav_none_buttons().as_str())
    }

    /// Moves cursor to prior button (`Shift+B`).
    pub fn move_prior_button(&mut self) -> Option<String> {
        let t = self.buffer.buttons.clone();
        self.control_nav(t, false, msg::quicknav_none_buttons().as_str())
    }

    /// Moves cursor to next edit field (`E`).
    pub fn move_next_edit(&mut self) -> Option<String> {
        let t = self.buffer.edits.clone();
        self.control_nav(t, true, msg::quicknav_none_edit_fields().as_str())
    }

    /// Moves cursor to prior edit field (`Shift+E`).
    pub fn move_prior_edit(&mut self) -> Option<String> {
        let t = self.buffer.edits.clone();
        self.control_nav(t, false, msg::quicknav_none_edit_fields().as_str())
    }

    /// Moves cursor to next check box (`X`).
    pub fn move_next_checkbox(&mut self) -> Option<String> {
        let t = self.buffer.checkboxes.clone();
        self.control_nav(t, true, msg::quicknav_none_check_boxes().as_str())
    }

    /// Moves cursor to prior check box (`Shift+X`).
    pub fn move_prior_checkbox(&mut self) -> Option<String> {
        let t = self.buffer.checkboxes.clone();
        self.control_nav(t, false, msg::quicknav_none_check_boxes().as_str())
    }

    /// Moves cursor to next radio button (`A`).
    pub fn move_next_radio_button(&mut self) -> Option<String> {
        let t = self.buffer.radio_buttons.clone();
        self.control_nav(t, true, msg::quicknav_none_radio_buttons().as_str())
    }

    /// Moves cursor to prior radio button (`Shift+A`).
    pub fn move_prior_radio_button(&mut self) -> Option<String> {
        let t = self.buffer.radio_buttons.clone();
        self.control_nav(t, false, msg::quicknav_none_radio_buttons().as_str())
    }

    /// Moves cursor to next combo box (`C`).
    pub fn move_next_combobox(&mut self) -> Option<String> {
        let t = self.buffer.comboboxes.clone();
        self.control_nav(t, true, msg::quicknav_none_combo_boxes().as_str())
    }

    /// Moves cursor to prior combo box (`Shift+C`).
    pub fn move_prior_combobox(&mut self) -> Option<String> {
        let t = self.buffer.comboboxes.clone();
        self.control_nav(t, false, msg::quicknav_none_combo_boxes().as_str())
    }

    /// Enters Forms Mode (`{ENTER}` on an Edit field or `EnterFormsMode`).
    pub fn enter_forms_mode(&mut self) -> Option<String> {
        self.selection = None;
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
                .nodes()
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
        self.selection = None;
        Some(SpeechFormatter::format_forms_mode_exit())
    }

    /// Toggles active control state (`{SPACE}` on CheckBox or Tab).
    pub fn toggle_active_control(&mut self) -> Option<String> {
        self.selection = None;
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
                if let Some(node) = self.buffer.node_mut(node_id) {
                    node.states.checked = Some(new_checked);
                }
                let doc_title = self
                    .buffer
                    .nodes()
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
                self.buffer.for_each_node_mut(|node| {
                    if node.role == Role::Tab {
                        node.states.selected = Some(node.id == node_id);
                    }
                });
                Some(format!(" \u{001d}{}\u{001e}", msg::state_selected()))
            }
            Role::ComboBox => {
                if let Some(node) = self.buffer.node_mut(node_id) {
                    let is_exp = node.states.expanded.unwrap_or(false);
                    node.states.expanded = Some(!is_exp);
                    if !is_exp {
                        Some(msg::state_expanded().to_string())
                    } else {
                        Some(msg::state_collapsed().to_string())
                    }
                } else {
                    None
                }
            }
            Role::Button => Some(SpeechFormatter::format_button(&name)),
            _ => None,
        }
    }

    /// Expands the active combobox (`Alt+DownArrow`).
    pub fn expand_combobox(&mut self) -> Option<String> {
        let node_id = self.current_node()?.id;
        let node = self.buffer.node_mut(node_id)?;
        if node.role == Role::ComboBox {
            node.states.expanded = Some(true);
            Some(msg::state_expanded().to_string())
        } else {
            None
        }
    }

    /// Collapses the active combobox (`Escape` or `Alt+UpArrow`).
    pub fn collapse_combobox(&mut self) -> Option<String> {
        let node_id = self.current_node()?.id;
        let node = self.buffer.node_mut(node_id)?;
        if node.role == Role::ComboBox {
            node.states.expanded = Some(false);
            Some(msg::state_collapsed().to_string())
        } else {
            None
        }
    }

    /// Returns whether the cursor is on an expanded combobox.
    pub fn is_combobox_expanded(&self) -> bool {
        if let Some(node) = self.current_node() {
            node.role == Role::ComboBox && node.states.expanded == Some(true)
        } else {
            false
        }
    }

    /// Selects the next option in an active combobox popup (`{DOWN}`).
    pub fn combobox_next_option(&mut self) -> Option<String> {
        let node_id = self.current_node()?.id;
        let node = self.buffer.node(node_id)?.clone();
        if node.role != Role::ComboBox || node.children.is_empty() {
            return None;
        }

        let current_val = node.value.as_deref().unwrap_or("");
        let current_idx = node
            .children
            .iter()
            .position(|c| {
                c.name.as_deref().unwrap_or("") == current_val
                    || c.text.as_deref().unwrap_or("") == current_val
            })
            .unwrap_or(0);

        let next_idx = (current_idx + 1).min(node.children.len().saturating_sub(1));
        let next_child = node.children.get(next_idx)?;
        let next_val = next_child
            .name
            .clone()
            .or_else(|| next_child.text.clone())
            .unwrap_or_default();

        if let Some(mut_node) = self.buffer.node_mut(node_id) {
            mut_node.value = Some(next_val.clone());
        }

        Some(next_val)
    }

    /// Selects the prior option in an active combobox popup (`{UP}`).
    pub fn combobox_prior_option(&mut self) -> Option<String> {
        let node_id = self.current_node()?.id;
        let node = self.buffer.node(node_id)?.clone();
        if node.role != Role::ComboBox || node.children.is_empty() {
            return None;
        }

        let current_val = node.value.as_deref().unwrap_or("");
        let current_idx = node
            .children
            .iter()
            .position(|c| {
                c.name.as_deref().unwrap_or("") == current_val
                    || c.text.as_deref().unwrap_or("") == current_val
            })
            .unwrap_or(0);

        let prior_idx = current_idx.saturating_sub(1);
        let prior_child = node.children.get(prior_idx)?;
        let prior_val = prior_child
            .name
            .clone()
            .or_else(|| prior_child.text.clone())
            .unwrap_or_default();

        if let Some(mut_node) = self.buffer.node_mut(node_id) {
            mut_node.value = Some(prior_val.clone());
        }

        Some(prior_val)
    }

    /// Expands a collapsed tree item or jumps to its first child if already expanded (`{RIGHT}`).
    pub fn tree_item_expand_or_next(&mut self) -> Option<String> {
        let node = self.current_node()?.clone();
        if node.role != Role::TreeItem {
            return None;
        }

        match node.states.expanded {
            Some(false) => {
                if let Some(mut_node) = self.buffer.node_mut(node.id) {
                    mut_node.states.expanded = Some(true);
                }
                Some(msg::state_expanded().to_string())
            }
            Some(true) => {
                // Move to first child tree item
                let first_child = node.children.iter().find(|c| c.role == Role::TreeItem);
                if let Some(child) = first_child {
                    self.move_to_node_id(child.id);
                    let name = child
                        .name
                        .as_deref()
                        .or(child.text.as_deref())
                        .unwrap_or("");
                    let level = child.level.unwrap_or(node.level.unwrap_or(1) + 1);
                    Some(SpeechFormatter::format_tree_item(
                        name,
                        level,
                        child.states.expanded,
                    ))
                } else {
                    None
                }
            }
            None => None,
        }
    }

    /// Collapses an expanded tree item or jumps to its parent if collapsed (`{LEFT}`).
    pub fn tree_item_collapse_or_parent(&mut self) -> Option<String> {
        let node = self.current_node()?.clone();
        if node.role != Role::TreeItem {
            return None;
        }

        if node.states.expanded == Some(true) {
            if let Some(mut_node) = self.buffer.node_mut(node.id) {
                mut_node.states.expanded = Some(false);
            }
            Some(msg::state_collapsed().to_string())
        } else {
            // Jump to parent tree item
            let parent_id = self.buffer.tree_parents.get(&node.id).copied()?;
            self.move_to_node_id(parent_id);
            let parent_node = self.buffer.node(parent_id)?.clone();
            let name = parent_node
                .name
                .as_deref()
                .or(parent_node.text.as_deref())
                .unwrap_or("");
            let level = parent_node.level.unwrap_or(1);
            Some(SpeechFormatter::format_tree_item(
                name,
                level,
                parent_node.states.expanded,
            ))
        }
    }

    /// Increases slider value (`{RIGHT}` or `{UP}` on Slider).
    pub fn slider_increase(&mut self) -> Option<String> {
        let node_id = self.current_node()?.id;
        let node = self.buffer.node(node_id)?.clone();
        if node.role != Role::Slider {
            return None;
        }

        let step = 5.0;
        let max = node.value_max.unwrap_or(100.0);
        let current = node.value_now.unwrap_or(0.0);
        let new_val = (current + step).min(max);

        if let Some(mut_node) = self.buffer.node_mut(node_id) {
            mut_node.value_now = Some(new_val);
        }

        Some(format!("{:.0}", new_val))
    }

    /// Decreases slider value (`{LEFT}` or `{DOWN}` on Slider).
    pub fn slider_decrease(&mut self) -> Option<String> {
        let node_id = self.current_node()?.id;
        let node = self.buffer.node(node_id)?.clone();
        if node.role != Role::Slider {
            return None;
        }

        let step = 5.0;
        let min = node.value_min.unwrap_or(0.0);
        let current = node.value_now.unwrap_or(0.0);
        let new_val = (current - step).max(min);

        if let Some(mut_node) = self.buffer.node_mut(node_id) {
            mut_node.value_now = Some(new_val);
        }

        Some(format!("{:.0}", new_val))
    }

    /// Moves cursor to next focusable control / tab stop (`{TAB}`).
    pub fn move_next_tab_stop(&mut self) -> Option<String> {
        self.selection = None;
        if self.buffer.tab_stops.is_empty() {
            return None;
        }
        let is_first_tab_from_start = self.offset == 0;
        let prev_node_id = self.current_node_id;

        let target_offset = if let Some((min_off, max_off)) = self.active_modal_range() {
            let modal_tab_stops: Vec<usize> = self
                .buffer
                .tab_stops
                .iter()
                .copied()
                .filter(|&ts| ts >= min_off && ts <= max_off)
                .collect();
            if modal_tab_stops.is_empty() {
                return None;
            }
            match modal_tab_stops.iter().find(|&&ts| ts > self.offset) {
                Some(&off) => off,
                None => *modal_tab_stops.first()?,
            }
        } else {
            let maybe_next = self.buffer.next_tab_stop_from(self.offset);
            match maybe_next {
                Some(off) => off,
                None => *self.buffer.tab_stops.first()?,
            }
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
        self.selection = None;
        if self.buffer.tab_stops.is_empty() {
            return None;
        }
        let prev_node_id = self.current_node_id;

        let target_offset = if let Some((min_off, max_off)) = self.active_modal_range() {
            let modal_tab_stops: Vec<usize> = self
                .buffer
                .tab_stops
                .iter()
                .copied()
                .filter(|&ts| ts >= min_off && ts <= max_off)
                .collect();
            if modal_tab_stops.is_empty() {
                return None;
            }
            match modal_tab_stops.iter().rfind(|&&ts| ts < self.offset) {
                Some(&off) => off,
                None => *modal_tab_stops.last()?,
            }
        } else {
            let maybe_prior = self.buffer.prior_tab_stop_from(self.offset);
            match maybe_prior {
                Some(off) => off,
                None => *self.buffer.tab_stops.last()?,
            }
        };

        self.offset = target_offset;
        let node = self.buffer.node_at_offset(target_offset)?.clone();
        self.current_node_id = Some(node.id);

        Some(self.format_tab_stop_node(&node, prev_node_id, None))
    }

    /// Aligns the virtual cursor offset with the specified node id.
    pub fn move_to_node_id(&mut self, node_id: usize) -> bool {
        // An element whose lines are its children's (WhatsApp's focused message) is at its
        // first line.
        if let Some(start) = self.buffer.start_of(node_id) {
            self.offset = start;
            self.current_node_id = Some(node_id);
            true
        } else if self.buffer.contains_node(node_id) {
            self.current_node_id = Some(node_id);
            true
        } else {
            false
        }
    }

    pub fn format_tab_stop_node(
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
                    .and_then(|id| self.buffer.node(id))
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
            Role::ComboBox => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let value = node.value.as_deref();
                let expanded = node.states.expanded.unwrap_or(false);
                SpeechFormatter::format_combobox(name, value, expanded)
            }
            Role::Slider => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let value = node.value_now.unwrap_or(0.0);
                SpeechFormatter::format_slider(name, value)
            }
            Role::ProgressBar => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let value = node.value_now.unwrap_or(0.0);
                SpeechFormatter::format_progressbar(name, value)
            }
            Role::TreeItem => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                let level = node.level.unwrap_or(1);
                SpeechFormatter::format_tree_item(name, level, node.states.expanded)
            }
            Role::Link { .. } => {
                let name = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                SpeechFormatter::format_link_focus(name, node)
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
        // The cell the last table move reached, while the cursor is still there: the line
        // keys move it from cell to cell too (Down along a header row before the table
        // layer's Right [obs, 535/s2, s3, s6]).
        if let Some(pos @ (t, r, c)) = self.current_table_pos {
            let at = self
                .buffer
                .tables
                .get(t)
                .and_then(|table| table.cells.get(r))
                .and_then(|row| row.get(c))
                .is_some_and(|cell| cell.offset == self.offset);
            if at {
                return Some(pos);
            }
            self.current_table_pos = None;
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
            return Some(msg::quicknav_none_tables().to_string());
        }
        let maybe_idx = self
            .buffer
            .tables
            .iter()
            .position(|t| t.start_offset > self.offset);
        self.enter_table(maybe_idx.unwrap_or(0))
    }

    /// Enters the table starting at `offset` (Select a Table), as T enters it.
    pub fn enter_table_at(&mut self, offset: usize) -> Option<String> {
        let index = self
            .buffer
            .tables
            .iter()
            .position(|t| t.start_offset == offset)?;
        self.enter_table(index)
    }

    /// Moves to table `table_idx` and says it: its size, caption and first cell. The cursor is
    /// then on that cell: Say Line reads it [obs, 257/s4: "Branch"].
    fn enter_table(&mut self, table_idx: usize) -> Option<String> {
        let table = self.buffer.tables.get(table_idx)?;
        let first_cell = table.cells.first().and_then(|r| r.first());
        self.offset = first_cell.map_or(table.start_offset, |c| c.offset);
        self.current_node_id = Some(first_cell.map_or(table.table_id, |c| c.node_id));
        self.current_table_pos = Some((table_idx, 0, 0));

        let first_cell_text = first_cell.map(|c| c.speech.as_str()).unwrap_or("");

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
            return Some(msg::quicknav_none_tables().to_string());
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
        let table_idx = maybe_idx.unwrap_or(self.buffer.tables.len().saturating_sub(1));
        let table = self.buffer.tables.get(table_idx)?;
        self.offset = table.start_offset;
        self.current_node_id = Some(table.table_id);
        self.current_table_pos = Some((table_idx, 0, 0));

        let first_cell_text = table
            .cells
            .first()
            .and_then(|r| r.first())
            .map(|c| c.speech.as_str())
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
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let cur_cell = table.cells.get(r_idx).and_then(|r| r.get(c_idx));
        let step = cur_cell.map(|c| c.col_span).unwrap_or(1);
        let new_c = c_idx + step;
        if new_c < table.cols {
            self.current_table_pos = Some((t_idx, r_idx, new_c));
            if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(new_c)) {
                self.offset = cell.offset;
                self.current_node_id = Some(cell.node_id);
                return Some(SpeechFormatter::format_table_cell_col(
                    &cell.speech,
                    cell.col,
                    table.cols,
                    cell.col_header.as_deref(),
                    cell.col_span,
                ));
            }
        }
        Some(msg::table_row_end().to_string())
    }

    /// Moves to cell to the left within the active row (`Ctrl+Alt+LeftArrow`).
    pub fn move_table_cell_left(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if c_idx > 0 {
            let prev_c = c_idx - 1;
            let target_c = if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(prev_c)) {
                cell.col - 1
            } else {
                prev_c
            };
            self.current_table_pos = Some((t_idx, r_idx, target_c));
            if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(target_c)) {
                self.offset = cell.offset;
                self.current_node_id = Some(cell.node_id);
                return Some(SpeechFormatter::format_table_cell_col(
                    &cell.speech,
                    cell.col,
                    table.cols,
                    cell.col_header.as_deref(),
                    cell.col_span,
                ));
            }
        }
        Some(msg::table_edge().to_string())
    }

    /// Moves to cell directly below in the active column (`Ctrl+Alt+DownArrow`).
    pub fn move_table_cell_down(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let cur_cell = table.cells.get(r_idx).and_then(|r| r.get(c_idx));
        let step = cur_cell.map(|c| c.row_span).unwrap_or(1);
        let new_r = r_idx + step;
        if new_r < table.rows {
            self.current_table_pos = Some((t_idx, new_r, c_idx));
            if let Some(cell) = table.cells.get(new_r).and_then(|r| r.get(c_idx)) {
                self.offset = cell.offset;
                self.current_node_id = Some(cell.node_id);
                return Some(SpeechFormatter::format_table_cell_row(
                    &cell.speech,
                    cell.row,
                    table.rows,
                    cell.row_header.as_deref(),
                    cell.row_span,
                    cell.col,
                ));
            }
        }
        Some(msg::table_column_bottom().to_string())
    }

    /// Moves to cell directly above in the active column (`Ctrl+Alt+UpArrow`).
    pub fn move_table_cell_up(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if r_idx > 0 {
            let prev_r = r_idx - 1;
            let target_r = if let Some(cell) = table.cells.get(prev_r).and_then(|r| r.get(c_idx)) {
                cell.row - 1
            } else {
                prev_r
            };
            self.current_table_pos = Some((t_idx, target_r, c_idx));
            if let Some(cell) = table.cells.get(target_r).and_then(|r| r.get(c_idx)) {
                self.offset = cell.offset;
                self.current_node_id = Some(cell.node_id);
                return Some(SpeechFormatter::format_table_cell_row(
                    &cell.speech,
                    cell.row,
                    table.rows,
                    cell.row_header.as_deref(),
                    cell.row_span,
                    cell.col,
                ));
            }
        }
        Some(msg::table_edge().to_string())
    }

    /// Moves to the first cell of the current table (`Ctrl+Alt+Home`).
    pub fn move_table_first_cell(&mut self) -> Option<String> {
        let (t_idx, _, _) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if let Some(cell) = table.first_cell() {
            self.current_table_pos = Some((t_idx, 0, 0));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
            return Some(SpeechFormatter::format_table_cell_coordinates(
                &cell.text,
                cell.col,
                cell.row,
                cell.col_header.as_deref(),
                cell.row_header.as_deref(),
                cell.col_span,
                cell.row_span,
            ));
        }
        Some(msg::table_not_in_table().to_string())
    }

    /// Moves to the last cell of the current table (`Ctrl+Alt+End`).
    pub fn move_table_last_cell(&mut self) -> Option<String> {
        let (t_idx, _, _) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if let Some(cell) = table.last_cell() {
            self.current_table_pos = Some((
                t_idx,
                table.rows.saturating_sub(1),
                table.cols.saturating_sub(1),
            ));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
            return Some(SpeechFormatter::format_table_cell_coordinates(
                &cell.text,
                cell.col,
                cell.row,
                cell.col_header.as_deref(),
                cell.row_header.as_deref(),
                cell.col_span,
                cell.row_span,
            ));
        }
        Some(msg::table_not_in_table().to_string())
    }

    /// Moves to the first cell in the active row (`Ctrl+Alt+Shift+LeftArrow`).
    pub fn move_table_row_start(&mut self) -> Option<String> {
        let (t_idx, r_idx, _) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if let Some(cell) = table.row_start(r_idx + 1) {
            self.current_table_pos = Some((t_idx, r_idx, 0));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
            return Some(SpeechFormatter::format_table_row_start(&cell.text));
        }
        Some(msg::table_not_in_table().to_string())
    }

    /// Moves to the last cell in the active row (`Ctrl+Alt+Shift+RightArrow`).
    pub fn move_table_row_end(&mut self) -> Option<String> {
        let (t_idx, r_idx, _) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if let Some(cell) = table.row_end(r_idx + 1) {
            self.current_table_pos = Some((t_idx, r_idx, table.cols.saturating_sub(1)));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
            return Some(SpeechFormatter::format_table_row_end(&cell.text));
        }
        Some(msg::table_not_in_table().to_string())
    }

    /// Moves to the top cell in the active column (`Ctrl+Alt+Shift+UpArrow`).
    pub fn move_table_col_top(&mut self) -> Option<String> {
        let (t_idx, _, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if let Some(cell) = table.col_top(c_idx + 1) {
            self.current_table_pos = Some((t_idx, 0, c_idx));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
            return Some(SpeechFormatter::format_table_col_top(&cell.text));
        }
        Some(msg::table_not_in_table().to_string())
    }

    /// Moves to the bottom cell in the active column (`Ctrl+Alt+Shift+DownArrow`).
    pub fn move_table_col_bottom(&mut self) -> Option<String> {
        let (t_idx, _, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if let Some(cell) = table.col_bottom(c_idx + 1) {
            self.current_table_pos = Some((t_idx, table.rows.saturating_sub(1), c_idx));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
            return Some(SpeechFormatter::format_table_col_bottom(&cell.text));
        }
        Some(msg::table_not_in_table().to_string())
    }

    /// Reads the current cell with coordinates and headers (`Ctrl+Alt+NumPad5` / `SayCurrentCell`).
    pub fn read_current_cell(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(c_idx)) {
            return Some(SpeechFormatter::format_current_cell(
                &cell.text,
                cell.col,
                cell.row,
                cell.col_header.as_deref(),
                cell.row_header.as_deref(),
                cell.col_span,
                cell.row_span,
            ));
        }
        Some(msg::table_not_in_table().to_string())
    }

    /// Reads the column header of the current cell (`Ctrl+Alt+Shift+H`).
    pub fn read_current_col_header(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(c_idx)) {
            return Some(SpeechFormatter::format_column_header(
                cell.col_header.as_deref(),
                cell.col,
            ));
        }
        Some(msg::table_not_in_table().to_string())
    }

    /// Reads the row header of the current cell (`Ctrl+Alt+Shift+R`).
    pub fn read_current_row_header(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some(msg::table_not_in_table().to_string()),
        };
        if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(c_idx)) {
            return Some(SpeechFormatter::format_row_header(
                cell.row_header.as_deref(),
                cell.row,
            ));
        }
        Some(msg::table_not_in_table().to_string())
    }

    /// The current row's cells one after another, as JAWS's speech history keeps them, with
    /// nothing between: "Central9 a.m. to 9 p.m.Directions" (its synthesizer is given the
    /// words apart) [obs, 297/s10, 535/s14].
    pub fn read_current_row(&mut self) -> Option<String> {
        self.read_table_line(TableLine::Row, 0)
    }

    /// The current column's cells, as [`Self::read_current_row`] reads a row: "QuarterQ1Q2Q3"
    /// [obs, 297/s11, 535/s15].
    pub fn read_current_column(&mut self) -> Option<String> {
        self.read_table_line(TableLine::Column, 0)
    }

    /// Moves to the next (`step` 1) or prior (-1) row and reads it whole (Windows+Alt+Down and
    /// Up); with none, "You are on the last row in the table." [obs, 297/s5-s7].
    pub fn read_adjacent_row(&mut self, step: isize) -> Option<String> {
        self.read_table_line(TableLine::Row, step)
    }

    /// Moves to the next or prior column and reads it whole (Windows+Alt+Right and Left)
    /// [obs, 297/s8, s9].
    pub fn read_adjacent_column(&mut self, step: isize) -> Option<String> {
        self.read_table_line(TableLine::Column, step)
    }

    /// Reads a row or a column whole, `step` rows or columns from the current cell's.
    fn read_table_line(&mut self, line: TableLine, step: isize) -> Option<String> {
        let Some((t_idx, r_idx, c_idx)) = self.resolve_table_pos() else {
            return Some(msg::table_not_in_table().to_string());
        };
        let table = self.buffer.tables.get(t_idx)?;
        let (index, count) = match line {
            TableLine::Row => (r_idx, table.rows),
            TableLine::Column => (c_idx, table.cols),
        };
        let Some(target) = index.checked_add_signed(step).filter(|&i| i < count) else {
            return Some(
                match (line, step > 0) {
                    (TableLine::Row, true) => msg::table_on_last_row(),
                    (TableLine::Row, false) => msg::table_on_first_row(),
                    (TableLine::Column, true) => msg::table_on_last_column(),
                    (TableLine::Column, false) => msg::table_on_first_column(),
                }
                .to_string(),
            );
        };
        let cells: Vec<&TableCellInfo> = match line {
            TableLine::Row => table.cells.get(target)?.iter().collect(),
            TableLine::Column => table
                .cells
                .iter()
                .filter_map(|row| row.get(target))
                .collect(),
        };
        // A cell spanning several rows or columns is in the grid at each of them: read once.
        let mut read = Vec::new();
        let mut text = String::new();
        for cell in &cells {
            if !read.contains(&cell.node_id) {
                read.push(cell.node_id);
                text.push_str(cell.text.trim());
            }
        }
        let (row, col) = match line {
            TableLine::Row => (target, c_idx),
            TableLine::Column => (r_idx, target),
        };
        if let Some(cell) = table.cells.get(row).and_then(|r| r.get(col)) {
            self.current_table_pos = Some((t_idx, row, col));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
        }
        Some(text)
    }

    /// Moves to the next math element (`O`).
    pub fn move_next_math(&mut self) -> Option<String> {
        if self.buffer.math_blocks.is_empty() {
            return Some(msg::quicknav_none_math().to_string());
        }
        let maybe_target = self.buffer.next_math_from(self.offset);
        let fallback = self.buffer.math_blocks.first().copied().unwrap_or(0);
        let target = maybe_target.unwrap_or(fallback);
        let wrapped = maybe_target.is_none();
        self.offset = target;
        if let Some(line) = self.buffer.line_at_offset(target) {
            self.current_node_id = Some(line.node_id);
            if wrapped {
                Some(format!("{}\n{}", msg::quicknav_wrap_top(), line.text))
            } else {
                Some(line.text.clone())
            }
        } else {
            None
        }
    }

    /// Moves to the prior math element (`Shift+O`).
    pub fn move_prior_math(&mut self) -> Option<String> {
        if self.buffer.math_blocks.is_empty() {
            return Some(msg::quicknav_none_math().to_string());
        }
        let maybe_target = self.buffer.prior_math_from(self.offset);
        let fallback = self.buffer.math_blocks.last().copied().unwrap_or(0);
        let target = maybe_target.unwrap_or(fallback);
        let wrapped = maybe_target.is_none();
        self.offset = target;
        if let Some(line) = self.buffer.line_at_offset(target) {
            self.current_node_id = Some(line.node_id);
            if wrapped {
                Some(format!("{}\n{}", msg::quicknav_wrap_bottom(), line.text))
            } else {
                Some(line.text.clone())
            }
        } else {
            None
        }
    }

    /// Enters Math Viewer on the current math element (`Enter`).
    pub fn enter_math_viewer(&mut self) -> Option<String> {
        if let Some(node) = self.buffer.node_at_offset(self.offset) {
            if node.role == Role::Math {
                return Some(msg::math_viewer_entered().to_string());
            }
        }
        None
    }

    /// Repositions the virtual cursor to a specific element offset and speaks the landing element announcement.
    pub fn jump_to_element_offset(&mut self, offset: usize) -> String {
        self.selection = None;
        self.offset = offset;
        if let Some(heading) = self.buffer.find_heading_node(offset) {
            self.current_node_id = Some(heading.id);
            let level = match heading.role {
                Role::Heading { level } => level,
                _ => 1,
            };
            let text = heading
                .name
                .as_deref()
                .or(heading.text.as_deref())
                .unwrap_or("");
            return SpeechFormatter::format_heading(level, text);
        }
        if let Some(link) = self.buffer.find_link_node(offset) {
            self.current_node_id = Some(link.id);
            // The link's line, as reading speaks it ("•  ‹ same page link › News").
            if let Some(line) = self.buffer.line_at_offset(offset) {
                return self.format_line_announcement(line);
            }
        }
        if let Some(fc) = self.buffer.find_form_control_node(offset) {
            self.current_node_id = Some(fc.id);
            return SpeechFormatter::format_form_control_node(fc);
        }
        self.current_node_id = self.buffer.node_at_offset(offset).map(|n| n.id);
        self.current_line()
    }

    /// Returns the byte start offsets of all lines matching `query`.
    pub fn find_matching_offsets(&self, query: &str, case_sensitive: bool) -> Vec<usize> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        if case_sensitive {
            self.buffer
                .lines
                .iter()
                .filter(|l| l.text.contains(trimmed))
                .map(|l| l.start_offset)
                .collect()
        } else {
            let q_lower = trimmed.to_lowercase();
            self.buffer
                .lines
                .iter()
                .filter(|l| l.text.to_lowercase().contains(&q_lower))
                .map(|l| l.start_offset)
                .collect()
        }
    }

    /// Searches forward in the virtual buffer for `query` and positions the cursor at the match.
    pub fn find_text(&mut self, query: &str, case_sensitive: bool) -> Option<String> {
        self.selection = None;
        let matches = self.find_matching_offsets(query, case_sensitive);
        if matches.is_empty() {
            return Some(SpeechFormatter::format_text_not_found(query));
        }

        self.search = SearchState {
            query: Some(query.to_string()),
            case_sensitive,
        };

        // Find first match at or after current offset
        let target_offset = if let Some(&off) = matches.iter().find(|&&off| off >= self.offset) {
            off
        } else {
            // Wrap to first match from top
            matches.first().copied().unwrap_or(0)
        };

        Some(self.jump_to_element_offset(target_offset))
    }

    /// Searches backward from the cursor for `query` (JAWS Find with Reverse), which F3 and
    /// Shift+F3 then repeat.
    pub fn find_text_backward(&mut self, query: &str, case_sensitive: bool) -> Option<String> {
        self.search = SearchState {
            query: Some(query.to_string()),
            case_sensitive,
        };
        self.find_prior()
    }

    /// Advances forward to the next match of the active search query (`F3`).
    pub fn find_next(&mut self) -> Option<String> {
        self.selection = None;
        let query = match self.search.query.as_deref() {
            Some(q) if !q.is_empty() => q,
            _ => return None,
        };
        let matches = self.find_matching_offsets(query, self.search.case_sensitive);
        if matches.is_empty() {
            return Some(SpeechFormatter::format_text_not_found(query));
        }

        if let Some(&target_offset) = matches.iter().find(|&&off| off > self.offset) {
            Some(self.jump_to_element_offset(target_offset))
        } else {
            // Wrap to top
            let first_offset = matches.first().copied().unwrap_or(0);
            let speech = self.jump_to_element_offset(first_offset);
            Some(format!("{}\n{}", msg::quicknav_wrap_top(), speech))
        }
    }

    /// Retreats backward to the prior match of the active search query (`Shift+F3`).
    pub fn find_prior(&mut self) -> Option<String> {
        self.selection = None;
        let query = match self.search.query.as_deref() {
            Some(q) if !q.is_empty() => q,
            _ => return None,
        };
        let matches = self.find_matching_offsets(query, self.search.case_sensitive);
        if matches.is_empty() {
            return Some(SpeechFormatter::format_text_not_found(query));
        }

        if let Some(&target_offset) = matches.iter().rfind(|&&off| off < self.offset) {
            Some(self.jump_to_element_offset(target_offset))
        } else {
            // Wrap to bottom
            let last_offset = matches.last().copied().unwrap_or(0);
            let speech = self.jump_to_element_offset(last_offset);
            Some(format!("{}\n{}", msg::quicknav_wrap_bottom(), speech))
        }
    }
}

/// A table's row or column, read whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TableLine {
    Row,
    Column,
}

/// How a selection key reads what it changed [obs, 501].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionReading {
    /// A character or a word: its line's marks and the text, then "selected" (" heading level
    /// 1 T | selected").
    Run,
    /// Lines: "selected", then the part of each line ("selected | heading level 1 Accessibility
    /// Engine Specification").
    Lines,
    /// Shift+Down: "selected", then the whole line the cursor leaves.
    LineLeft,
}

/// What `after` covers that `before` does not, as one range: both run from the same anchor.
fn range_gained(before: (usize, usize), after: (usize, usize)) -> Option<(usize, usize)> {
    if after.1 > before.1 {
        Some((before.1.max(after.0), after.1))
    } else if after.0 < before.0 {
        Some((after.0, before.0.min(after.1)))
    } else {
        None
    }
}

/// The marks the virtual cursor says before a line's text: " heading level 1 " for a
/// heading, nothing for plain text.
fn line_marks(line: &BufferLine) -> &str {
    line.speech
        .strip_suffix(line.text.as_str())
        .filter(|marks| marks.contains('\u{1d}'))
        .unwrap_or_default()
}

/// A list item's line split at its mark ("• ", "◦ ", "1. "): the mark, and the line's text and
/// speech after it. Lines without one come back whole.
fn split_list_mark(line: &BufferLine) -> (Option<&str>, &str, &str) {
    let mark_len = ["• ", "◦ ", "▪ "]
        .iter()
        .find(|m| line.text.starts_with(**m))
        .map(|m| m.len())
        .or_else(|| {
            let digits = line.text.chars().take_while(char::is_ascii_digit).count();
            (digits > 0 && line.text.get(digits..).is_some_and(|r| r.starts_with(". ")))
                .then_some(digits + 2)
        });
    let split = mark_len.and_then(|len| {
        let mark = line.text.get(..len)?;
        let speech = line.speech.strip_prefix(mark.trim_end())?;
        Some((mark.trim_end(), line.text.get(len..)?, speech))
    });
    match split {
        Some((mark, text, speech)) => (Some(mark), text, speech),
        None => (None, line.text.as_str(), line.speech.as_str()),
    }
}
