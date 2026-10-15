use crate::aom::{AomNode, Role};
use crate::buffer::{BufferLine, VirtualBuffer};
use crate::speech::SpeechFormatter;

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

    /// Extends selection to next character (`Shift+{RIGHT}`).
    pub fn select_next_char(&mut self) -> Option<String> {
        let max_offset = if let Some((_, max_off)) = self.active_modal_range() {
            max_off
        } else {
            self.buffer.len()
        };

        if self.offset >= max_offset {
            return None;
        }

        let current_char = self.buffer.char_at(self.offset)?;
        let next_offset = (self.offset + current_char.len_utf8()).min(max_offset);
        if next_offset == self.offset {
            return None;
        }

        let mut sel = self
            .selection
            .unwrap_or(TextSelection::new(self.offset, self.offset));
        let prior_active = sel.active;
        sel.active = next_offset;
        self.selection = Some(sel);
        self.offset = next_offset;

        if sel.active > sel.anchor {
            Some(SpeechFormatter::format_selection(
                true,
                &current_char.to_string(),
            ))
        } else {
            let unsel_slice = self
                .buffer
                .text
                .get(prior_active..next_offset)
                .unwrap_or_default();
            Some(SpeechFormatter::format_selection(false, unsel_slice))
        }
    }

    /// Extends selection to prior character (`Shift+{LEFT}`).
    pub fn select_prior_char(&mut self) -> Option<String> {
        let min_offset = if let Some((min_off, _)) = self.active_modal_range() {
            min_off
        } else {
            0
        };

        if self.offset <= min_offset {
            return None;
        }

        let mut prev = self.offset - 1;
        while prev > min_offset && !self.buffer.text.is_char_boundary(prev) {
            prev -= 1;
        }

        let ch = self.buffer.char_at(prev)?;
        let mut sel = self
            .selection
            .unwrap_or(TextSelection::new(self.offset, self.offset));
        let prior_active = sel.active;
        sel.active = prev;
        self.selection = Some(sel);
        self.offset = prev;

        if sel.active < sel.anchor {
            Some(SpeechFormatter::format_selection(true, &ch.to_string()))
        } else {
            let unsel_slice = self.buffer.text.get(prev..prior_active).unwrap_or_default();
            Some(SpeechFormatter::format_selection(false, unsel_slice))
        }
    }

    /// Extends selection to next line (`Shift+{DOWN}`).
    pub fn select_next_line(&mut self) -> Option<String> {
        let max_offset = if let Some((_, max_off)) = self.active_modal_range() {
            max_off
        } else {
            self.buffer.len()
        };

        let next_line = self.buffer.next_line_from(self.offset)?;
        if next_line.start_offset > max_offset {
            return None;
        }

        let mut sel = self
            .selection
            .unwrap_or(TextSelection::new(self.offset, self.offset));
        let prior_active = sel.active;
        let new_offset = next_line.start_offset;
        sel.active = new_offset;
        self.selection = Some(sel);
        self.offset = new_offset;
        self.current_node_id = Some(next_line.node_id);

        let line_text = &next_line.text;
        if sel.active >= sel.anchor {
            Some(SpeechFormatter::format_selection(true, line_text))
        } else {
            let unsel = if prior_active < new_offset {
                self.buffer
                    .text
                    .get(prior_active..new_offset)
                    .unwrap_or_default()
            } else {
                line_text
            };
            Some(SpeechFormatter::format_selection(false, unsel))
        }
    }

    /// Extends selection to prior line (`Shift+{UP}`).
    pub fn select_prior_line(&mut self) -> Option<String> {
        let min_offset = if let Some((min_off, _)) = self.active_modal_range() {
            min_off
        } else {
            0
        };

        let prior_line = self.buffer.prior_line_from(self.offset)?;
        if prior_line.start_offset < min_offset {
            return None;
        }

        let mut sel = self
            .selection
            .unwrap_or(TextSelection::new(self.offset, self.offset));
        let prior_active = sel.active;
        let new_offset = prior_line.start_offset;
        sel.active = new_offset;
        self.selection = Some(sel);
        self.offset = new_offset;
        self.current_node_id = Some(prior_line.node_id);

        let line_text = &prior_line.text;
        if sel.active <= sel.anchor {
            Some(SpeechFormatter::format_selection(true, line_text))
        } else {
            let unsel = if prior_active > new_offset {
                self.buffer
                    .text
                    .get(new_offset..prior_active)
                    .unwrap_or_default()
            } else {
                line_text
            };
            Some(SpeechFormatter::format_selection(false, unsel))
        }
    }

    /// Speaks the currently selected text (`SaySelection`).
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
        "No selection".to_string()
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
                    if line.text.ends_with("frame end") || line.text.ends_with("frame") {
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
            .map(|l| self.format_line_announcement(l))
            .collect();

        lines.join("\n")
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
                None => (in_range.first().copied()?, Some("wrapping to top")),
            }
        } else {
            match in_range.iter().copied().rfind(|&o| o < current_line_start) {
                Some(o) => (o, None),
                None => (in_range.last().copied()?, Some("wrapping to bottom")),
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
        let previous = self.offset;
        self.move_to_node_id(node_id);
        let node = self.buffer.node(node_id)?;
        let attr = |k: &str| node.attributes.get(k).map(String::as_str);
        let name = node.name.as_deref().unwrap_or_default().trim();
        let mut speech = match &node.role {
            // Unlike the C quick key, Tab omits the collapsed state.
            // One typed in keeps its state: "Fruit edit combo collapsed | blank" (JAWS on
            // the dev box).
            Role::ComboBox if attr("editable") == Some("true") => {
                let state = if node.states.expanded == Some(true) {
                    "expanded"
                } else {
                    "collapsed"
                };
                let value = node.value.as_deref().filter(|v| !v.is_empty());
                format!(
                    "{name}  \u{1d} edit combo \u{1e}   \u{1d}{state}\u{1e} \n{}",
                    value.unwrap_or("blank")
                )
            }
            Role::ComboBox => format!(
                "{name}  \u{1d} Combo box \u{1e}  \n{}",
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
                    lines.push(format!("{group} \u{1d} group \u{1e} \n"));
                }
                lines.push(format!("{name}  \u{1d} Tab \u{1e} "));
                if let Some(i) = info {
                    lines.push(format!("{} of {}", i.index, i.total));
                }
                if node.states.selected == Some(true) {
                    lines.push(" \u{1d}selected\u{1e}".to_owned());
                }
                lines.join("\n")
            }
            Role::Edit if attr("password") == Some("true") => format!(
                "{}  \u{1d}\u{1e}",
                self.control_focus_speech(self.offset, previous)
            ),
            _ => self.control_focus_speech(self.offset, previous),
        };
        if speech.trim().is_empty() {
            speech = name.to_owned();
        }
        if let Some(hint) = SpeechFormatter::web_focus_hint(node) {
            speech.push('\n');
            speech.push_str(hint);
        }
        Some(speech)
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
        let text = self
            .buffer
            .line_at_offset(offset)
            .map(|l| l.text.clone())
            .unwrap_or_default();
        SpeechFormatter::format_heading(level, &text)
    }

    /// Moves cursor to next heading across all levels (`H` / `MoveToNextHeading`).
    pub fn move_next_heading(&mut self) -> Option<String> {
        let targets: Vec<usize> = self.buffer.headings.iter().map(|h| h.0).collect();
        self.quick_nav(
            &targets,
            true,
            "There are no headings on this page.",
            |c, o, _| c.heading_speech_at(o),
        )
    }

    /// Moves cursor to prior heading across all levels (`Shift+H`).
    pub fn move_prior_heading(&mut self) -> Option<String> {
        let targets: Vec<usize> = self.buffer.headings.iter().map(|h| h.0).collect();
        self.quick_nav(
            &targets,
            false,
            "There are no headings on this page.",
            |c, o, _| c.heading_speech_at(o),
        )
    }

    /// Heading level keys do not wrap (§12).
    fn no_more_headings(level: u32) -> String {
        if level == 1 {
            "No more headings at level1.".to_owned()
        } else {
            format!("No more headings at level{level} in this section.")
        }
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
        match target {
            Some(off) => {
                self.offset = off;
                self.current_node_id = self.buffer.line_at_offset(off).map(|l| l.node_id);
                Some(self.heading_speech_at(off))
            }
            None => Some(Self::no_more_headings(level)),
        }
    }

    /// Moves cursor to prior heading of `level` (`Shift+1`–`Shift+6`).
    pub fn move_prior_heading_level(&mut self, level: u32) -> Option<String> {
        self.selection = None;
        let current_line_start = self
            .buffer
            .line_at_offset(self.offset)
            .map(|l| l.start_offset)
            .unwrap_or(self.offset);
        let target = self
            .buffer
            .headings
            .iter()
            .rfind(|&&(off, l)| l == level && off < current_line_start)
            .map(|h| h.0);
        match target {
            Some(off) => {
                self.offset = off;
                self.current_node_id = self.buffer.line_at_offset(off).map(|l| l.node_id);
                Some(self.heading_speech_at(off))
            }
            None => Some(Self::no_more_headings(level)),
        }
    }

    /// Moves cursor to next link (`U` / `MoveToNextLink`).
    pub fn move_next_link(&mut self) -> Option<String> {
        let targets = self.buffer.links.clone();
        self.quick_nav(
            &targets,
            true,
            "There are no links on this page.",
            |c, o, _| c.line_speech_at(o),
        )
    }

    /// Moves cursor to prior link (`Shift+U`).
    pub fn move_prior_link(&mut self) -> Option<String> {
        let targets = self.buffer.links.clone();
        self.quick_nav(
            &targets,
            false,
            "There are no links on this page.",
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
            return Some("There are no frames on this page".to_string());
        }

        if let Some(frame) = self.buffer.next_frame_from(self.offset) {
            self.offset = frame.start_offset;
            self.current_node_id = Some(frame.node_id);
            Some(SpeechFormatter::format_frame(&frame.name))
        } else if let Some(frame) = self.buffer.first_frame() {
            self.offset = frame.start_offset;
            self.current_node_id = Some(frame.node_id);
            Some(format!(
                "wrapping to top\n{}",
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
            return Some("There are no frames on this page".to_string());
        }

        if let Some(frame) = self.buffer.prior_frame_from(self.offset) {
            self.offset = frame.start_offset;
            self.current_node_id = Some(frame.node_id);
            Some(SpeechFormatter::format_frame(&frame.name))
        } else if let Some(frame) = self.buffer.last_frame() {
            self.offset = frame.start_offset;
            self.current_node_id = Some(frame.node_id);
            Some(format!(
                "wrapping to bottom\n{}",
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

    /// Moves cursor to next list container (`L` / `MoveToNextList`).
    pub fn move_next_list(&mut self) -> Option<String> {
        let t = self.buffer.lists.clone();
        self.line_nav(t, true, "There are no lists on this page.")
    }

    /// Moves cursor to prior list container (`Shift+L`).
    pub fn move_prior_list(&mut self) -> Option<String> {
        let t = self.buffer.lists.clone();
        self.line_nav(t, false, "There are no lists on this page.")
    }

    /// Moves cursor to next list item (`I`).
    pub fn move_next_list_item(&mut self) -> Option<String> {
        let t = self.buffer.list_items.clone();
        self.line_nav(t, true, "There are no list items on this page.")
    }

    /// Moves cursor to prior list item (`Shift+I`).
    pub fn move_prior_list_item(&mut self) -> Option<String> {
        let t = self.buffer.list_items.clone();
        self.line_nav(t, false, "There are no list items on this page.")
    }

    /// Moves cursor to next region (`R`, §15).
    pub fn move_next_landmark(&mut self) -> Option<String> {
        let t = self.buffer.landmarks.clone();
        self.line_nav(t, true, "There are no regions on this page.")
    }

    /// Moves cursor to prior region (`Shift+R`).
    pub fn move_prior_landmark(&mut self) -> Option<String> {
        let t = self.buffer.landmarks.clone();
        self.line_nav(t, false, "There are no regions on this page.")
    }

    /// Moves cursor to next graphic (`G`, §16).
    pub fn move_next_graphic(&mut self) -> Option<String> {
        let t = self.buffer.graphics.clone();
        self.line_nav(t, true, "There are no graphics on this page.")
    }

    /// Moves cursor to prior graphic (`Shift+G`).
    pub fn move_prior_graphic(&mut self) -> Option<String> {
        let t = self.buffer.graphics.clone();
        self.line_nav(t, false, "There are no graphics on this page.")
    }

    /// Moves cursor to next article (`O`).
    pub fn move_next_article(&mut self) -> Option<String> {
        let t = self.buffer.articles.clone();
        self.line_nav(t, true, "There are no articles on this page.")
    }

    /// Moves cursor to prior article (`Shift+O`).
    pub fn move_prior_article(&mut self) -> Option<String> {
        let t = self.buffer.articles.clone();
        self.line_nav(t, false, "There are no articles on this page.")
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
        self.line_nav(t, true, "There are no onMouseOver elements on this page.")
    }

    /// Moves cursor to prior onMouseOver element (`Shift+;`).
    pub fn move_prior_on_mouse_over(&mut self) -> Option<String> {
        let t = self.buffer.on_mouse_over.clone();
        self.line_nav(t, false, "There are no onMouseOver elements on this page.")
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
        self.control_nav(t, true, "There are no form fields on this page.")
    }

    /// Moves cursor to prior form field (`Shift+F`).
    pub fn move_prior_form_control(&mut self) -> Option<String> {
        let t = self.buffer.form_controls.clone();
        self.control_nav(t, false, "There are no form fields on this page.")
    }

    /// Moves cursor to next button (`B`).
    pub fn move_next_button(&mut self) -> Option<String> {
        let t = self.buffer.buttons.clone();
        self.control_nav(t, true, "There are no buttons on this page.")
    }

    /// Moves cursor to prior button (`Shift+B`).
    pub fn move_prior_button(&mut self) -> Option<String> {
        let t = self.buffer.buttons.clone();
        self.control_nav(t, false, "There are no buttons on this page.")
    }

    /// Moves cursor to next edit field (`E`).
    pub fn move_next_edit(&mut self) -> Option<String> {
        let t = self.buffer.edits.clone();
        self.control_nav(t, true, "There are no edit fields on this page.")
    }

    /// Moves cursor to prior edit field (`Shift+E`).
    pub fn move_prior_edit(&mut self) -> Option<String> {
        let t = self.buffer.edits.clone();
        self.control_nav(t, false, "There are no edit fields on this page.")
    }

    /// Moves cursor to next check box (`X`).
    pub fn move_next_checkbox(&mut self) -> Option<String> {
        let t = self.buffer.checkboxes.clone();
        self.control_nav(t, true, "There are no check boxes on this page.")
    }

    /// Moves cursor to prior check box (`Shift+X`).
    pub fn move_prior_checkbox(&mut self) -> Option<String> {
        let t = self.buffer.checkboxes.clone();
        self.control_nav(t, false, "There are no check boxes on this page.")
    }

    /// Moves cursor to next radio button (`A`).
    pub fn move_next_radio_button(&mut self) -> Option<String> {
        let t = self.buffer.radio_buttons.clone();
        self.control_nav(t, true, "There are no radio buttons on this page.")
    }

    /// Moves cursor to prior radio button (`Shift+A`).
    pub fn move_prior_radio_button(&mut self) -> Option<String> {
        let t = self.buffer.radio_buttons.clone();
        self.control_nav(t, false, "There are no radio buttons on this page.")
    }

    /// Moves cursor to next combo box (`C`).
    pub fn move_next_combobox(&mut self) -> Option<String> {
        let t = self.buffer.comboboxes.clone();
        self.control_nav(t, true, "There are no combo boxes on this page.")
    }

    /// Moves cursor to prior combo box (`Shift+C`).
    pub fn move_prior_combobox(&mut self) -> Option<String> {
        let t = self.buffer.comboboxes.clone();
        self.control_nav(t, false, "There are no combo boxes on this page.")
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
                Some(" \u{001d}selected\u{001e}".to_string())
            }
            Role::ComboBox => {
                if let Some(node) = self.buffer.node_mut(node_id) {
                    let is_exp = node.states.expanded.unwrap_or(false);
                    node.states.expanded = Some(!is_exp);
                    if !is_exp {
                        Some("expanded".to_string())
                    } else {
                        Some("collapsed".to_string())
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
            Some("expanded".to_string())
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
            Some("collapsed".to_string())
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
                Some("expanded".to_string())
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
            Some("collapsed".to_string())
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
        if let Some(span) = self.buffer.spans.iter().find(|s| s.node_id == node_id) {
            self.offset = span.start_offset;
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
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
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
        Some("end of row".to_string())
    }

    /// Moves to cell to the left within the active row (`Ctrl+Alt+LeftArrow`).
    pub fn move_table_cell_left(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
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
        Some("edge of table".to_string())
    }

    /// Moves to cell directly below in the active column (`Ctrl+Alt+DownArrow`).
    pub fn move_table_cell_down(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
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
        Some("bottom of column".to_string())
    }

    /// Moves to cell directly above in the active column (`Ctrl+Alt+UpArrow`).
    pub fn move_table_cell_up(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
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
        Some("edge of table".to_string())
    }

    /// Moves to the first cell of the current table (`Ctrl+Alt+Home`).
    pub fn move_table_first_cell(&mut self) -> Option<String> {
        let (t_idx, _, _) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
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
        Some("Not in a table.".to_string())
    }

    /// Moves to the last cell of the current table (`Ctrl+Alt+End`).
    pub fn move_table_last_cell(&mut self) -> Option<String> {
        let (t_idx, _, _) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
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
        Some("Not in a table.".to_string())
    }

    /// Moves to the first cell in the active row (`Ctrl+Alt+Shift+LeftArrow`).
    pub fn move_table_row_start(&mut self) -> Option<String> {
        let (t_idx, r_idx, _) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
        };
        if let Some(cell) = table.row_start(r_idx + 1) {
            self.current_table_pos = Some((t_idx, r_idx, 0));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
            return Some(SpeechFormatter::format_table_row_start(&cell.text));
        }
        Some("Not in a table.".to_string())
    }

    /// Moves to the last cell in the active row (`Ctrl+Alt+Shift+RightArrow`).
    pub fn move_table_row_end(&mut self) -> Option<String> {
        let (t_idx, r_idx, _) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
        };
        if let Some(cell) = table.row_end(r_idx + 1) {
            self.current_table_pos = Some((t_idx, r_idx, table.cols.saturating_sub(1)));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
            return Some(SpeechFormatter::format_table_row_end(&cell.text));
        }
        Some("Not in a table.".to_string())
    }

    /// Moves to the top cell in the active column (`Ctrl+Alt+Shift+UpArrow`).
    pub fn move_table_col_top(&mut self) -> Option<String> {
        let (t_idx, _, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
        };
        if let Some(cell) = table.col_top(c_idx + 1) {
            self.current_table_pos = Some((t_idx, 0, c_idx));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
            return Some(SpeechFormatter::format_table_col_top(&cell.text));
        }
        Some("Not in a table.".to_string())
    }

    /// Moves to the bottom cell in the active column (`Ctrl+Alt+Shift+DownArrow`).
    pub fn move_table_col_bottom(&mut self) -> Option<String> {
        let (t_idx, _, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
        };
        if let Some(cell) = table.col_bottom(c_idx + 1) {
            self.current_table_pos = Some((t_idx, table.rows.saturating_sub(1), c_idx));
            self.offset = cell.offset;
            self.current_node_id = Some(cell.node_id);
            return Some(SpeechFormatter::format_table_col_bottom(&cell.text));
        }
        Some("Not in a table.".to_string())
    }

    /// Reads the current cell with coordinates and headers (`Ctrl+Alt+NumPad5` / `SayCurrentCell`).
    pub fn read_current_cell(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
        };
        if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(c_idx)) {
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
        Some("Not in a table.".to_string())
    }

    /// Reads the column header of the current cell (`Ctrl+Alt+Shift+H`).
    pub fn read_current_col_header(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
        };
        if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(c_idx)) {
            return Some(SpeechFormatter::format_column_header(
                cell.col_header.as_deref(),
                cell.col,
            ));
        }
        Some("Not in a table.".to_string())
    }

    /// Reads the row header of the current cell (`Ctrl+Alt+Shift+R`).
    pub fn read_current_row_header(&mut self) -> Option<String> {
        let (t_idx, r_idx, c_idx) = match self.resolve_table_pos() {
            Some(pos) => pos,
            None => return Some("Not in a table.".to_string()),
        };
        let table = match self.buffer.tables.get(t_idx) {
            Some(t) => t,
            None => return Some("Not in a table.".to_string()),
        };
        if let Some(cell) = table.cells.get(r_idx).and_then(|r| r.get(c_idx)) {
            return Some(SpeechFormatter::format_row_header(
                cell.row_header.as_deref(),
                cell.row,
            ));
        }
        Some("Not in a table.".to_string())
    }

    /// Moves to the next math element (`O`).
    pub fn move_next_math(&mut self) -> Option<String> {
        if self.buffer.math_blocks.is_empty() {
            return Some("There are no math equations on this page.".to_string());
        }
        let maybe_target = self.buffer.next_math_from(self.offset);
        let fallback = self.buffer.math_blocks.first().copied().unwrap_or(0);
        let target = maybe_target.unwrap_or(fallback);
        let wrapped = maybe_target.is_none();
        self.offset = target;
        if let Some(line) = self.buffer.line_at_offset(target) {
            self.current_node_id = Some(line.node_id);
            if wrapped {
                Some(format!("wrapping to top\n{}", line.text))
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
            return Some("There are no math equations on this page.".to_string());
        }
        let maybe_target = self.buffer.prior_math_from(self.offset);
        let fallback = self.buffer.math_blocks.last().copied().unwrap_or(0);
        let target = maybe_target.unwrap_or(fallback);
        let wrapped = maybe_target.is_none();
        self.offset = target;
        if let Some(line) = self.buffer.line_at_offset(target) {
            self.current_node_id = Some(line.node_id);
            if wrapped {
                Some(format!("wrapping to bottom\n{}", line.text))
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
                return Some("Math Viewer. Press Escape to exit.".to_string());
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
            Some(format!("wrapping to top\n{}", speech))
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
            Some(format!("wrapping to bottom\n{}", speech))
        }
    }
}
