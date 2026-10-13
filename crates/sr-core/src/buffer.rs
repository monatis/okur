use crate::aom::{AomNode, Role, TextFormatting};
use crate::speech::SpeechFormatter;
use std::collections::HashMap;

/// Information about an inline frame or embedded frame container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameInfo {
    pub node_id: usize,
    pub name: String,
    pub start_offset: usize,
    pub end_offset: usize,
}

/// Information about an ARIA or web dialog container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogInfo {
    pub node_id: usize,
    pub name: String,
    pub is_modal: bool,
    pub start_offset: usize,
    pub end_offset: usize,
}

/// A continuous span of text mapped to an AOM node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferSpan {
    pub start_offset: usize,
    pub end_offset: usize,
    pub node_id: usize,
    pub role: Role,
}

/// A line within the virtual buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferLine {
    pub line_index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub node_id: usize,
    pub role: Role,
    pub text: String,
}

/// A tokenized word within the virtual buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferWord {
    pub word_index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub text: String,
}

/// A paragraph / block boundary in the virtual buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferParagraph {
    pub paragraph_index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub node_id: usize,
    pub role: Role,
    pub text: String,
}

/// Detailed information for a table cell in a 2D grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCellInfo {
    pub node_id: usize,
    pub offset: usize,
    pub row: usize,
    pub col: usize,
    pub text: String,
}

/// 2D grid structure representing a data table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableGrid {
    pub table_id: usize,
    pub rows: usize,
    pub cols: usize,
    pub caption: Option<String>,
    pub start_offset: usize,
    pub end_offset: usize,
    pub cells: Vec<Vec<TableCellInfo>>,
}

impl TableGrid {
    pub fn cell(&self, row: usize, col: usize) -> Option<&TableCellInfo> {
        if row == 0 || col == 0 {
            return None;
        }
        self.cells.get(row - 1).and_then(|r| r.get(col - 1))
    }
}

/// Information about an ARIA tab control within a tablist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabInfo {
    pub node_id: usize,
    pub offset: usize,
    pub index: usize,
    pub total: usize,
    pub group_name: Option<String>,
}

/// The linearized Virtual Buffer maintaining document offsets, spans, lines, words,
/// and structural indices for quick navigation keys.
#[derive(Debug, Clone, Default)]
pub struct VirtualBuffer {
    pub text: String,
    pub spans: Vec<BufferSpan>,
    pub lines: Vec<BufferLine>,
    pub words: Vec<BufferWord>,
    pub paragraphs: Vec<BufferParagraph>,
    pub headings: Vec<(usize, u32)>,
    pub links: Vec<usize>,
    pub lists: Vec<usize>,
    pub list_items: Vec<usize>,
    pub landmarks: Vec<usize>,
    pub form_controls: Vec<usize>,
    pub buttons: Vec<usize>,
    pub edits: Vec<usize>,
    pub checkboxes: Vec<usize>,
    pub comboboxes: Vec<usize>,
    pub trees: Vec<usize>,
    pub tree_items: Vec<usize>,
    pub sliders: Vec<usize>,
    pub progress_bars: Vec<usize>,
    pub tree_parents: HashMap<usize, usize>,
    pub tab_stops: Vec<usize>,
    pub tabs: Vec<TabInfo>,
    pub tables: Vec<TableGrid>,
    pub table_offsets: Vec<usize>,
    pub dialogs: Vec<DialogInfo>,
    pub frames: Vec<FrameInfo>,
    pub frame_offsets: Vec<usize>,
    pub target_anchors: HashMap<String, usize>,
    pub node_map: HashMap<usize, AomNode>,
}

impl VirtualBuffer {
    /// Creates a VirtualBuffer from an AOM root node.
    pub fn from_aom(root: &AomNode) -> Self {
        let mut buffer = VirtualBuffer::default();
        let mut builder = BufferBuilder::new();
        builder.build(root, &mut buffer);
        buffer.tokenize_words();
        buffer
    }

    /// Returns the character at the specified byte/char offset.
    pub fn char_at(&self, offset: usize) -> Option<char> {
        self.text[offset..].chars().next()
    }

    /// Total length in bytes of the buffer text.
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Returns the line containing the given offset.
    pub fn line_at_offset(&self, offset: usize) -> Option<&BufferLine> {
        self.lines
            .iter()
            .find(|line| offset >= line.start_offset && offset <= line.end_offset)
            .or_else(|| self.lines.last())
    }

    /// Returns the next line starting after the given offset.
    pub fn next_line_from(&self, offset: usize) -> Option<&BufferLine> {
        self.lines.iter().find(|l| l.start_offset > offset)
    }

    /// Returns the prior line starting before the current line of the given offset.
    pub fn prior_line_from(&self, offset: usize) -> Option<&BufferLine> {
        let current_line_start = self
            .line_at_offset(offset)
            .map(|l| l.start_offset)
            .unwrap_or(offset);
        self.lines
            .iter()
            .rfind(|l| l.start_offset < current_line_start)
    }

    /// Returns the word containing or starting at the given offset.
    pub fn word_at_offset(&self, offset: usize) -> Option<&BufferWord> {
        self.words
            .iter()
            .find(|w| offset >= w.start_offset && offset < w.end_offset)
            .or_else(|| self.words.iter().find(|w| w.start_offset >= offset))
    }

    /// Returns the next word starting after the given offset.
    pub fn next_word_from(&self, offset: usize) -> Option<&BufferWord> {
        self.words.iter().find(|w| w.start_offset > offset)
    }

    /// Returns the prior word starting before the given offset.
    pub fn prior_word_from(&self, offset: usize) -> Option<&BufferWord> {
        self.words.iter().rfind(|w| w.start_offset < offset)
    }

    /// Returns the paragraph containing the given offset.
    pub fn paragraph_at_offset(&self, offset: usize) -> Option<&BufferParagraph> {
        self.paragraphs
            .iter()
            .find(|p| offset >= p.start_offset && offset <= p.end_offset)
            .or_else(|| self.paragraphs.last())
    }

    /// Returns the next paragraph starting after the given offset.
    pub fn next_paragraph_from(&self, offset: usize) -> Option<&BufferParagraph> {
        self.paragraphs.iter().find(|p| p.start_offset > offset)
    }

    /// Returns the prior paragraph starting before the current paragraph.
    pub fn prior_paragraph_from(&self, offset: usize) -> Option<&BufferParagraph> {
        let current_p_start = self
            .paragraph_at_offset(offset)
            .map(|p| p.start_offset)
            .unwrap_or(offset);
        self.paragraphs
            .iter()
            .rfind(|p| p.start_offset < current_p_start)
    }

    /// Returns the next heading offset and level after the given offset.
    pub fn next_heading_from(&self, offset: usize) -> Option<(usize, u32)> {
        self.headings.iter().find(|h| h.0 > offset).copied()
    }

    /// Returns the prior heading offset and level before the given offset.
    pub fn prior_heading_from(&self, offset: usize) -> Option<(usize, u32)> {
        let current_start = self
            .headings
            .iter()
            .rfind(|h| h.0 <= offset)
            .map(|h| h.0)
            .unwrap_or(offset);
        self.headings.iter().rfind(|h| h.0 < current_start).copied()
    }

    /// Returns the next heading offset of a specific level after the given offset.
    pub fn next_heading_level_from(&self, offset: usize, level: u32) -> Option<(usize, u32)> {
        self.headings
            .iter()
            .find(|h| h.0 > offset && h.1 == level)
            .copied()
    }

    /// Returns the prior heading offset of a specific level before the given offset.
    pub fn prior_heading_level_from(&self, offset: usize, level: u32) -> Option<(usize, u32)> {
        let current_start = self
            .headings
            .iter()
            .rfind(|h| h.0 <= offset && h.1 == level)
            .map(|h| h.0)
            .unwrap_or(offset);
        self.headings
            .iter()
            .rfind(|h| h.0 < current_start && h.1 == level)
            .copied()
    }

    /// Returns the next link offset after the given offset.
    pub fn next_link_from(&self, offset: usize) -> Option<usize> {
        self.links.iter().find(|&&l| l > offset).copied()
    }

    /// Returns the prior link offset before the given offset.
    pub fn prior_link_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .links
            .iter()
            .rfind(|&&l| l <= offset)
            .copied()
            .unwrap_or(offset);
        self.links.iter().rfind(|&&l| l < current_start).copied()
    }

    /// Returns the next list offset after the given offset.
    pub fn next_list_from(&self, offset: usize) -> Option<usize> {
        self.lists.iter().find(|&&l| l > offset).copied()
    }

    /// Returns the prior list offset before the given offset.
    pub fn prior_list_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .lists
            .iter()
            .rfind(|&&l| l <= offset)
            .copied()
            .unwrap_or(offset);
        self.lists.iter().rfind(|&&l| l < current_start).copied()
    }

    /// Returns the next list item offset after the given offset.
    pub fn next_list_item_from(&self, offset: usize) -> Option<usize> {
        self.list_items.iter().find(|&&l| l > offset).copied()
    }

    /// Returns the prior list item offset before the given offset.
    pub fn prior_list_item_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .list_items
            .iter()
            .rfind(|&&l| l <= offset)
            .copied()
            .unwrap_or(offset);
        self.list_items
            .iter()
            .rfind(|&&l| l < current_start)
            .copied()
    }

    /// Returns the next landmark offset after the given offset.
    pub fn next_landmark_from(&self, offset: usize) -> Option<usize> {
        self.landmarks.iter().find(|&&l| l > offset).copied()
    }

    /// Returns the prior landmark offset before the given offset.
    pub fn prior_landmark_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .landmarks
            .iter()
            .rfind(|&&l| l <= offset)
            .copied()
            .unwrap_or(offset);
        self.landmarks
            .iter()
            .rfind(|&&l| l < current_start)
            .copied()
    }

    /// Returns the next form control offset after the given offset.
    pub fn next_form_control_from(&self, offset: usize) -> Option<usize> {
        self.form_controls.iter().find(|&&fc| fc > offset).copied()
    }

    /// Returns the prior form control offset before the given offset.
    pub fn prior_form_control_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .form_controls
            .iter()
            .rfind(|&&fc| fc <= offset)
            .copied()
            .unwrap_or(offset);
        self.form_controls
            .iter()
            .rfind(|&&fc| fc < current_start)
            .copied()
    }

    /// Returns the next button offset after the given offset.
    pub fn next_button_from(&self, offset: usize) -> Option<usize> {
        self.buttons.iter().find(|&&b| b > offset).copied()
    }

    /// Returns the prior button offset before the given offset.
    pub fn prior_button_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .buttons
            .iter()
            .rfind(|&&b| b <= offset)
            .copied()
            .unwrap_or(offset);
        self.buttons.iter().rfind(|&&b| b < current_start).copied()
    }

    /// Returns the next edit offset after the given offset.
    pub fn next_edit_from(&self, offset: usize) -> Option<usize> {
        self.edits.iter().find(|&&e| e > offset).copied()
    }

    /// Returns the prior edit offset before the given offset.
    pub fn prior_edit_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .edits
            .iter()
            .rfind(|&&e| e <= offset)
            .copied()
            .unwrap_or(offset);
        self.edits.iter().rfind(|&&e| e < current_start).copied()
    }

    /// Returns the next checkbox offset after the given offset.
    pub fn next_checkbox_from(&self, offset: usize) -> Option<usize> {
        self.checkboxes.iter().find(|&&c| c > offset).copied()
    }

    /// Returns the prior checkbox offset before the given offset.
    pub fn prior_checkbox_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .checkboxes
            .iter()
            .rfind(|&&c| c <= offset)
            .copied()
            .unwrap_or(offset);
        self.checkboxes
            .iter()
            .rfind(|&&c| c < current_start)
            .copied()
    }

    /// Returns the next interactive control / tab stop offset after the given offset.
    pub fn next_tab_stop_from(&self, offset: usize) -> Option<usize> {
        self.tab_stops.iter().find(|&&ts| ts > offset).copied()
    }

    /// Returns the prior interactive control / tab stop offset before the given offset.
    pub fn prior_tab_stop_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .tab_stops
            .iter()
            .rfind(|&&ts| ts <= offset)
            .copied()
            .unwrap_or(offset);
        self.tab_stops
            .iter()
            .rfind(|&&ts| ts < current_start)
            .copied()
    }

    /// Resolves tab info for an AOM node.
    pub fn tab_info_for_node(&self, node_id: usize) -> Option<&TabInfo> {
        self.tabs.iter().find(|t| t.node_id == node_id)
    }

    /// Document title if available.
    pub fn document_title(&self) -> Option<&str> {
        self.lines
            .iter()
            .find(|l| l.role == Role::Document)
            .map(|l| l.text.as_str())
    }

    /// Returns the next combobox offset after the given offset.
    pub fn next_combobox_from(&self, offset: usize) -> Option<usize> {
        self.comboboxes.iter().find(|&&c| c > offset).copied()
    }

    /// Returns the prior combobox offset before the given offset.
    pub fn prior_combobox_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .comboboxes
            .iter()
            .rfind(|&&c| c <= offset)
            .copied()
            .unwrap_or(offset);
        self.comboboxes
            .iter()
            .rfind(|&&c| c < current_start)
            .copied()
    }

    /// Resolves the node for a combobox starting at `offset`.
    pub fn find_combobox_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::ComboBox))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Returns the next slider offset after the given offset.
    pub fn next_slider_from(&self, offset: usize) -> Option<usize> {
        self.sliders.iter().find(|&&s| s > offset).copied()
    }

    /// Returns the prior slider offset before the given offset.
    pub fn prior_slider_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .sliders
            .iter()
            .rfind(|&&s| s <= offset)
            .copied()
            .unwrap_or(offset);
        self.sliders.iter().rfind(|&&s| s < current_start).copied()
    }

    /// Resolves the node for a slider starting at `offset`.
    pub fn find_slider_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Slider))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Returns the next tree item offset after the given offset.
    pub fn next_tree_item_from(&self, offset: usize) -> Option<usize> {
        self.tree_items.iter().find(|&&t| t > offset).copied()
    }

    /// Returns the prior tree item offset before the given offset.
    pub fn prior_tree_item_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .tree_items
            .iter()
            .rfind(|&&t| t <= offset)
            .copied()
            .unwrap_or(offset);
        self.tree_items
            .iter()
            .rfind(|&&t| t < current_start)
            .copied()
    }

    /// Resolves the node for a tree item starting at `offset`.
    pub fn find_tree_item_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::TreeItem))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Resolves the node for a form control starting at `offset`.
    pub fn find_form_control_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| {
                s.start_offset == offset
                    && matches!(
                        s.role,
                        Role::Edit
                            | Role::Button
                            | Role::CheckBox
                            | Role::ComboBox
                            | Role::Slider
                            | Role::ProgressBar
                    )
            })
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Resolves the node for a button starting at `offset`.
    pub fn find_button_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Button))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Resolves the node for an edit box starting at `offset`.
    pub fn find_edit_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Edit))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Resolves the node for a checkbox starting at `offset`.
    pub fn find_checkbox_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::CheckBox))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Resolves the node for a heading starting at `offset`.
    pub fn find_heading_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Heading { .. }))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Resolves the node for a link starting at `offset`.
    pub fn find_link_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Link { .. }))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Resolves the node for a list starting at `offset`.
    pub fn find_list_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::List))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Resolves the node for a list item starting at `offset`.
    pub fn find_list_item_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::ListItem))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Resolves the node for a landmark starting at `offset`.
    pub fn find_landmark_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Landmark { .. }))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Returns the next table starting after the given offset.
    pub fn next_table_from(&self, offset: usize) -> Option<&TableGrid> {
        self.tables.iter().find(|t| t.start_offset > offset)
    }

    /// Returns the prior table starting before the current table of the given offset.
    pub fn prior_table_from(&self, offset: usize) -> Option<&TableGrid> {
        let current_start = self
            .tables
            .iter()
            .rfind(|t| t.start_offset <= offset)
            .map(|t| t.start_offset)
            .unwrap_or(offset);
        self.tables.iter().rfind(|t| t.start_offset < current_start)
    }

    /// Returns the table containing the given offset.
    pub fn table_at_offset(&self, offset: usize) -> Option<&TableGrid> {
        self.tables
            .iter()
            .find(|t| offset >= t.start_offset && offset <= t.end_offset)
    }

    /// Resolves the node for a table starting at `offset`.
    pub fn find_table_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Table))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Returns the text slice of a span covering `offset`.
    pub fn span_text_at_offset(&self, offset: usize) -> Option<&str> {
        self.spans
            .iter()
            .find(|s| offset >= s.start_offset && offset < s.end_offset)
            .map(|s| &self.text[s.start_offset..s.end_offset])
    }

    /// Finds the AOM node associated with the given buffer offset.
    pub fn node_at_offset(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .filter(|s| offset >= s.start_offset && offset <= s.end_offset)
            .min_by_key(|s| s.end_offset - s.start_offset)
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Returns the dialog covering the given offset, if any.
    pub fn dialog_at_offset(&self, offset: usize) -> Option<&DialogInfo> {
        self.dialogs
            .iter()
            .find(|d| offset >= d.start_offset && offset <= d.end_offset)
    }

    /// Returns dialog information for a specific node ID.
    pub fn dialog_for_node(&self, node_id: usize) -> Option<&DialogInfo> {
        self.dialogs.iter().find(|d| d.node_id == node_id)
    }

    /// Returns the next frame starting after the given offset.
    pub fn next_frame_from(&self, offset: usize) -> Option<&FrameInfo> {
        self.frames.iter().find(|f| f.start_offset > offset)
    }

    /// Returns the prior frame starting before the current frame of the given offset.
    pub fn prior_frame_from(&self, offset: usize) -> Option<&FrameInfo> {
        let current_start = self
            .frames
            .iter()
            .rfind(|f| f.start_offset <= offset)
            .map(|f| f.start_offset)
            .unwrap_or(offset);
        self.frames.iter().rfind(|f| f.start_offset < current_start)
    }

    /// Returns the first frame in the buffer.
    pub fn first_frame(&self) -> Option<&FrameInfo> {
        self.frames.first()
    }

    /// Returns the last frame in the buffer.
    pub fn last_frame(&self) -> Option<&FrameInfo> {
        self.frames.last()
    }

    /// Resolves the node for a frame starting at `offset`.
    pub fn find_frame_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::IFrame))
            .and_then(|s| self.node_map.get(&s.node_id))
    }

    /// Resolves font formatting attributes at the given buffer offset.
    pub fn formatting_at_offset(&self, offset: usize) -> Option<TextFormatting> {
        self.spans
            .iter()
            .filter(|s| offset >= s.start_offset && offset <= s.end_offset)
            .min_by_key(|s| s.end_offset - s.start_offset)
            .and_then(|s| self.node_map.get(&s.node_id))
            .and_then(|n| n.formatting.clone())
            .or_else(|| {
                self.node_at_offset(offset)
                    .and_then(|n| n.formatting.clone())
            })
    }

    /// Tokenizes words from the buffer text.
    fn tokenize_words(&mut self) {
        let mut words = Vec::new();
        let mut word_index = 0;
        let mut char_indices = self.text.char_indices().peekable();

        while let Some((start, ch)) = char_indices.next() {
            if ch.is_alphanumeric() {
                let mut end = start + ch.len_utf8();
                while let Some(&(next_idx, next_ch)) = char_indices.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '\'' || next_ch == '-' {
                        end = next_idx + next_ch.len_utf8();
                        char_indices.next();
                    } else {
                        break;
                    }
                }
                let raw_slice = &self.text[start..end];
                let clean_text = raw_slice
                    .trim_matches(|c: char| c == '\'' || c == '-')
                    .to_string();
                if !clean_text.is_empty() {
                    words.push(BufferWord {
                        word_index,
                        start_offset: start,
                        end_offset: end,
                        text: clean_text,
                    });
                    word_index += 1;
                }
            }
        }

        self.words = words;
    }
}

struct BufferBuilder {
    formatting_stack: Vec<TextFormatting>,
    block_depth: usize,
}

impl BufferBuilder {
    fn new() -> Self {
        Self {
            formatting_stack: Vec::new(),
            block_depth: 0,
        }
    }

    fn build(&mut self, root: &AomNode, buffer: &mut VirtualBuffer) {
        if let Some(ref fmt) = root.formatting {
            self.formatting_stack.push(fmt.clone());
        }
        buffer.node_map.insert(root.id, root.clone());

        // If document node has a name (e.g. document title), add it as the first line
        if root.role == Role::Document {
            if let Some(ref title) = root.name {
                let start = buffer.text.len();
                buffer.text.push_str(title);
                let end = buffer.text.len();

                let line = BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset: start,
                    end_offset: end,
                    node_id: root.id,
                    role: Role::Document,
                    text: title.clone(),
                };
                let paragraph = BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset: start,
                    end_offset: end,
                    node_id: root.id,
                    role: Role::Document,
                    text: title.clone(),
                };
                let span = BufferSpan {
                    start_offset: start,
                    end_offset: end,
                    node_id: root.id,
                    role: Role::Document,
                };

                buffer.lines.push(line);
                buffer.paragraphs.push(paragraph);
                buffer.spans.push(span);
            }

            // Traverse children
            for child in &root.children {
                self.traverse_node(child, buffer);
            }
        } else {
            self.traverse_node(root, buffer);
        }
    }

    fn traverse_node(&mut self, node: &AomNode, buffer: &mut VirtualBuffer) {
        // Skip hidden nodes (aria-hidden or hidden state/attribute)
        if node.states.hidden
            || node
                .attributes
                .get("aria-hidden")
                .map(|v| v == "true")
                .unwrap_or(false)
            || node.attributes.contains_key("hidden")
        {
            return;
        }

        let effective_formatting = match (&node.formatting, self.formatting_stack.last()) {
            (Some(node_fmt), Some(parent_fmt)) => Some(parent_fmt.merge(node_fmt)),
            (Some(node_fmt), None) => Some(node_fmt.clone()),
            (None, Some(parent_fmt)) => Some(parent_fmt.clone()),
            (None, None) => None,
        };

        let mut node_with_effective = node.clone();
        if let Some(ref eff) = effective_formatting {
            node_with_effective.formatting = Some(eff.clone());
        }
        buffer.node_map.insert(node.id, node_with_effective);

        let pushed_fmt = if let Some(ref eff) = effective_formatting {
            self.formatting_stack.push(eff.clone());
            true
        } else {
            false
        };
        let anchor_offset =
            if node.role.is_block() && !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                buffer.text.len() + 1
            } else {
                buffer.text.len()
            };
        if let Some(id_attr) = node.attributes.get("id") {
            buffer.target_anchors.insert(id_attr.clone(), anchor_offset);
        }
        if let Some(name_attr) = node.attributes.get("name") {
            buffer
                .target_anchors
                .insert(name_attr.clone(), anchor_offset);
        }

        match &node.role {
            Role::Landmark { landmark_type: _ } => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.landmarks.push(start_offset);

                for child in &node.children {
                    self.traverse_node(child, buffer);
                }

                let end_offset = buffer.text.len();
                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: node.role.clone(),
                });
            }
            Role::List => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.lists.push(start_offset);

                for child in &node.children {
                    self.traverse_node(child, buffer);
                }

                let end_offset = buffer.text.len();
                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::List,
                });
            }
            Role::ListItem => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.list_items.push(start_offset);

                self.block_depth += 1;
                if !node.children.is_empty() {
                    for child in &node.children {
                        self.traverse_node(child, buffer);
                    }
                } else if let Some(ref text) = node.text {
                    buffer.text.push_str(text);
                } else if let Some(ref name) = node.name {
                    buffer.text.push_str(name);
                }
                self.block_depth -= 1;

                let end_offset = buffer.text.len();
                let text_content = buffer.text[start_offset..end_offset].to_string();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::ListItem,
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::ListItem,
                    text: text_content.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::ListItem,
                    text: text_content,
                });
            }
            Role::Heading { level } => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.headings.push((start_offset, *level));

                self.block_depth += 1;
                if !node.children.is_empty() {
                    for child in &node.children {
                        self.traverse_node(child, buffer);
                    }
                } else if let Some(ref text) = node.text {
                    buffer.text.push_str(text);
                } else if let Some(ref name) = node.name {
                    buffer.text.push_str(name);
                }
                self.block_depth -= 1;

                let end_offset = buffer.text.len();
                let text_content = buffer.text[start_offset..end_offset].to_string();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Heading { level: *level },
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Heading { level: *level },
                    text: text_content.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Heading { level: *level },
                    text: text_content,
                });
            }
            Role::Paragraph => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();

                self.block_depth += 1;
                if !node.children.is_empty() {
                    for child in &node.children {
                        self.traverse_node(child, buffer);
                    }
                } else if let Some(ref text) = node.text {
                    buffer.text.push_str(text);
                } else if let Some(ref name) = node.name {
                    buffer.text.push_str(name);
                }
                self.block_depth -= 1;

                let end_offset = buffer.text.len();
                let text_content = buffer.text[start_offset..end_offset].to_string();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Paragraph,
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Paragraph,
                    text: text_content.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Paragraph,
                    text: text_content,
                });
            }
            Role::Link { href: _ } => {
                if self.block_depth == 0 && !buffer.text.is_empty() && !buffer.text.ends_with('\n')
                {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.links.push(start_offset);
                buffer.tab_stops.push(start_offset);

                if !node.children.is_empty() {
                    for child in &node.children {
                        self.traverse_node(child, buffer);
                    }
                } else if let Some(ref text) = node.text {
                    buffer.text.push_str(text);
                } else if let Some(ref name) = node.name {
                    buffer.text.push_str(name);
                }

                let end_offset = buffer.text.len();
                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: node.role.clone(),
                });

                if self.block_depth == 0 {
                    let text_content = buffer.text[start_offset..end_offset].to_string();
                    buffer.lines.push(BufferLine {
                        line_index: buffer.lines.len(),
                        start_offset,
                        end_offset,
                        node_id: node.id,
                        role: node.role.clone(),
                        text: text_content.clone(),
                    });
                    buffer.paragraphs.push(BufferParagraph {
                        paragraph_index: buffer.paragraphs.len(),
                        start_offset,
                        end_offset,
                        node_id: node.id,
                        role: node.role.clone(),
                        text: text_content,
                    });
                }
            }
            Role::Edit => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.form_controls.push(start_offset);
                buffer.edits.push(start_offset);
                buffer.tab_stops.push(start_offset);

                let label = node.name.as_deref().unwrap_or("");
                let val = node.value.as_deref().unwrap_or("");
                if !label.is_empty() {
                    buffer.text.push_str(label);
                    if !val.is_empty() {
                        if !buffer.text.ends_with(' ') {
                            buffer.text.push(' ');
                        }
                        buffer.text.push_str(val);
                    }
                } else if !val.is_empty() {
                    buffer.text.push_str(val);
                } else if let Some(ref text) = node.text {
                    buffer.text.push_str(text);
                }

                let end_offset = buffer.text.len();
                let text_content = buffer.text[start_offset..end_offset].to_string();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Edit,
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Edit,
                    text: text_content.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Edit,
                    text: text_content,
                });
            }
            Role::Button => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.form_controls.push(start_offset);
                buffer.buttons.push(start_offset);
                buffer.tab_stops.push(start_offset);

                let label = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                buffer.text.push_str(label);

                let end_offset = buffer.text.len();
                let text_content = buffer.text[start_offset..end_offset].to_string();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Button,
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Button,
                    text: text_content.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Button,
                    text: text_content,
                });
            }
            Role::CheckBox => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.form_controls.push(start_offset);
                buffer.checkboxes.push(start_offset);
                buffer.tab_stops.push(start_offset);

                let label = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                buffer.text.push_str(label);

                let end_offset = buffer.text.len();
                let text_content = buffer.text[start_offset..end_offset].to_string();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::CheckBox,
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::CheckBox,
                    text: text_content.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::CheckBox,
                    text: text_content,
                });
            }
            Role::ComboBox => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.form_controls.push(start_offset);
                buffer.comboboxes.push(start_offset);
                buffer.tab_stops.push(start_offset);

                let label = node.name.as_deref().unwrap_or("");
                let val = node.value.as_deref().unwrap_or("");
                if !label.is_empty() {
                    buffer.text.push_str(label);
                    if !val.is_empty() {
                        if !buffer.text.ends_with(' ') {
                            buffer.text.push(' ');
                        }
                        buffer.text.push_str(val);
                    }
                } else if !val.is_empty() {
                    buffer.text.push_str(val);
                } else if let Some(ref text) = node.text {
                    buffer.text.push_str(text);
                }

                // Register options/children into buffer.node_map
                for child in &node.children {
                    buffer.node_map.insert(child.id, child.clone());
                }

                let end_offset = buffer.text.len();
                let text_content = buffer.text[start_offset..end_offset].to_string();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::ComboBox,
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::ComboBox,
                    text: text_content.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::ComboBox,
                    text: text_content,
                });
            }
            Role::Slider => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.form_controls.push(start_offset);
                buffer.sliders.push(start_offset);
                buffer.tab_stops.push(start_offset);

                let label = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                buffer.text.push_str(label);
                if let Some(val) = node.value_now {
                    buffer.text.push(' ');
                    buffer.text.push_str(&format!("{:.0}", val));
                }

                let end_offset = buffer.text.len();
                let text_content = buffer.text[start_offset..end_offset].to_string();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Slider,
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Slider,
                    text: text_content.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Slider,
                    text: text_content,
                });
            }
            Role::ProgressBar => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.progress_bars.push(start_offset);

                let label = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
                buffer.text.push_str(label);
                if let Some(val) = node.value_now {
                    buffer.text.push(' ');
                    buffer.text.push_str(&format!("{:.0}%", val));
                }

                let end_offset = buffer.text.len();
                let text_content = buffer.text[start_offset..end_offset].to_string();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::ProgressBar,
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::ProgressBar,
                    text: text_content.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::ProgressBar,
                    text: text_content,
                });
            }
            Role::Tree => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.trees.push(start_offset);

                for child in &node.children {
                    self.traverse_tree_item(child, 1, None, buffer);
                }

                let end_offset = buffer.text.len();
                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: Role::Tree,
                });
            }
            Role::TreeItem => {
                self.traverse_tree_item(node, 1, None, buffer);
            }
            Role::Generic => {
                if !node.children.is_empty() {
                    for child in &node.children {
                        self.traverse_node(child, buffer);
                    }
                } else if let Some(ref text) = node.text {
                    if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                        buffer.text.push('\n');
                    }
                    let start_offset = buffer.text.len();
                    buffer.text.push_str(text);
                    let end_offset = buffer.text.len();
                    let text_content = text.clone();

                    buffer.spans.push(BufferSpan {
                        start_offset,
                        end_offset,
                        node_id: node.id,
                        role: Role::Generic,
                    });
                    buffer.lines.push(BufferLine {
                        line_index: buffer.lines.len(),
                        start_offset,
                        end_offset,
                        node_id: node.id,
                        role: Role::Generic,
                        text: text_content.clone(),
                    });
                    buffer.paragraphs.push(BufferParagraph {
                        paragraph_index: buffer.paragraphs.len(),
                        start_offset,
                        end_offset,
                        node_id: node.id,
                        role: Role::Generic,
                        text: text_content,
                    });
                }
            }
            Role::IFrame => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();
                buffer.frame_offsets.push(start_offset);

                let frame_name = node
                    .name
                    .as_deref()
                    .or_else(|| node.attributes.get("title").map(|s| s.as_str()))
                    .unwrap_or("Inline Frame");
                let entry_text = SpeechFormatter::format_frame(frame_name);

                buffer.text.push_str(&entry_text);
                let entry_end = buffer.text.len();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset: entry_end,
                    node_id: node.id,
                    role: Role::IFrame,
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset: entry_end,
                    node_id: node.id,
                    role: Role::IFrame,
                    text: entry_text.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset: entry_end,
                    node_id: node.id,
                    role: Role::IFrame,
                    text: entry_text,
                });

                for child in &node.children {
                    self.traverse_node(child, buffer);
                }

                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let exit_start = buffer.text.len();
                let exit_text = SpeechFormatter::format_frame_end(frame_name);
                buffer.text.push_str(&exit_text);
                let exit_end = buffer.text.len();

                buffer.spans.push(BufferSpan {
                    start_offset: exit_start,
                    end_offset: exit_end,
                    node_id: node.id,
                    role: Role::Generic,
                });
                buffer.lines.push(BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset: exit_start,
                    end_offset: exit_end,
                    node_id: node.id,
                    role: Role::Generic,
                    text: exit_text.clone(),
                });
                buffer.paragraphs.push(BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset: exit_start,
                    end_offset: exit_end,
                    node_id: node.id,
                    role: Role::Generic,
                    text: exit_text,
                });

                buffer.frames.push(FrameInfo {
                    node_id: node.id,
                    name: frame_name.to_string(),
                    start_offset,
                    end_offset: exit_end,
                });
            }
            Role::Table => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let table_start = buffer.text.len();
                buffer.table_offsets.push(table_start);

                let mut row_nodes = Vec::new();
                Self::collect_table_rows(node, &mut row_nodes);

                let mut table_cells_2d: Vec<Vec<TableCellInfo>> = Vec::new();

                for (r_idx, row_node) in row_nodes.iter().enumerate() {
                    buffer.node_map.insert(row_node.id, row_node.clone());
                    let row_start = buffer.text.len();

                    let mut cell_nodes = Vec::new();
                    Self::collect_table_cells(row_node, &mut cell_nodes);

                    let mut row_cells = Vec::new();
                    for (c_idx, cell_node) in cell_nodes.iter().enumerate() {
                        buffer.node_map.insert(cell_node.id, cell_node.clone());
                        if c_idx > 0 {
                            buffer.text.push(' ');
                        }
                        let cell_start = buffer.text.len();
                        let mut cell_text = cell_node
                            .text
                            .clone()
                            .or_else(|| cell_node.name.clone())
                            .unwrap_or_default();
                        if cell_text.is_empty() {
                            cell_text = cell_node.collect_text();
                        }
                        let cell_text = cell_text.trim().to_string();

                        buffer.text.push_str(&cell_text);
                        let cell_end = buffer.text.len();

                        buffer.spans.push(BufferSpan {
                            start_offset: cell_start,
                            end_offset: cell_end,
                            node_id: cell_node.id,
                            role: Role::TableCell,
                        });

                        row_cells.push(TableCellInfo {
                            node_id: cell_node.id,
                            offset: cell_start,
                            row: r_idx + 1,
                            col: c_idx + 1,
                            text: cell_text,
                        });
                    }

                    let row_end = buffer.text.len();
                    buffer.spans.push(BufferSpan {
                        start_offset: row_start,
                        end_offset: row_end,
                        node_id: row_node.id,
                        role: Role::TableRow,
                    });
                    let row_text = buffer.text[row_start..row_end].to_string();
                    buffer.lines.push(BufferLine {
                        line_index: buffer.lines.len(),
                        start_offset: row_start,
                        end_offset: row_end,
                        node_id: row_node.id,
                        role: Role::TableRow,
                        text: row_text.clone(),
                    });
                    buffer.paragraphs.push(BufferParagraph {
                        paragraph_index: buffer.paragraphs.len(),
                        start_offset: row_start,
                        end_offset: row_end,
                        node_id: row_node.id,
                        role: Role::TableRow,
                        text: row_text,
                    });

                    table_cells_2d.push(row_cells);

                    if !buffer.text.ends_with('\n') {
                        buffer.text.push('\n');
                    }
                }

                let table_end = buffer.text.len();
                buffer.spans.push(BufferSpan {
                    start_offset: table_start,
                    end_offset: table_end,
                    node_id: node.id,
                    role: Role::Table,
                });

                let total_rows = table_cells_2d.len();
                let total_cols = table_cells_2d.iter().map(|r| r.len()).max().unwrap_or(0);

                let caption = node.name.clone().or_else(|| {
                    node.children.iter().find_map(|child| {
                        if child.role == Role::Generic || child.role == Role::Text {
                            child.name.clone().or_else(|| child.text.clone())
                        } else {
                            None
                        }
                    })
                });

                let grid = TableGrid {
                    table_id: node.id,
                    rows: total_rows,
                    cols: total_cols,
                    caption,
                    start_offset: table_start,
                    end_offset: table_end,
                    cells: table_cells_2d,
                };
                buffer.tables.push(grid);
            }
            Role::TabList => {
                let group_name = node.name.clone();
                let tab_count = node
                    .children
                    .iter()
                    .filter(|c| {
                        c.role == Role::Tab
                            && !c.states.hidden
                            && !c
                                .attributes
                                .get("aria-hidden")
                                .map(|v| v == "true")
                                .unwrap_or(false)
                    })
                    .count();
                let mut current_tab_idx = 1;
                for child in &node.children {
                    if child.role == Role::Tab {
                        self.traverse_tab(
                            child,
                            current_tab_idx,
                            tab_count,
                            group_name.clone(),
                            buffer,
                        );
                        current_tab_idx += 1;
                    } else {
                        self.traverse_node(child, buffer);
                    }
                }
            }
            Role::Tab => {
                self.traverse_tab(node, 1, 1, None, buffer);
            }
            Role::TabPanel => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                for child in &node.children {
                    self.traverse_node(child, buffer);
                }
            }
            Role::Dialog | Role::AlertDialog => {
                if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();

                let is_modal = node.states.modal
                    || node
                        .attributes
                        .get("aria-modal")
                        .map(|v| v == "true")
                        .unwrap_or(true);

                let dialog_name = node.name.clone().unwrap_or_else(|| "Dialog".to_string());

                for child in &node.children {
                    self.traverse_node(child, buffer);
                }

                let end_offset = buffer.text.len();

                buffer.spans.push(BufferSpan {
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: node.role.clone(),
                });

                buffer.dialogs.push(DialogInfo {
                    node_id: node.id,
                    name: dialog_name,
                    is_modal,
                    start_offset,
                    end_offset,
                });
            }
            _ => {
                let is_block = node.role.is_block();
                if is_block && !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
                    buffer.text.push('\n');
                }
                let start_offset = buffer.text.len();

                if !node.children.is_empty() {
                    for child in &node.children {
                        self.traverse_node(child, buffer);
                    }
                } else if let Some(ref text) = node.text {
                    buffer.text.push_str(text);
                } else if let Some(ref name) = node.name {
                    buffer.text.push_str(name);
                }

                let end_offset = buffer.text.len();
                if end_offset > start_offset {
                    let text_content = buffer.text[start_offset..end_offset].to_string();
                    buffer.spans.push(BufferSpan {
                        start_offset,
                        end_offset,
                        node_id: node.id,
                        role: node.role.clone(),
                    });
                    if is_block {
                        buffer.lines.push(BufferLine {
                            line_index: buffer.lines.len(),
                            start_offset,
                            end_offset,
                            node_id: node.id,
                            role: node.role.clone(),
                            text: text_content.clone(),
                        });
                        buffer.paragraphs.push(BufferParagraph {
                            paragraph_index: buffer.paragraphs.len(),
                            start_offset,
                            end_offset,
                            node_id: node.id,
                            role: node.role.clone(),
                            text: text_content,
                        });
                    }

                    if node.states.modal
                        || node
                            .attributes
                            .get("aria-modal")
                            .map(|v| v == "true")
                            .unwrap_or(false)
                    {
                        let dialog_name = node.name.clone().unwrap_or_else(|| "Dialog".to_string());
                        buffer.dialogs.push(DialogInfo {
                            node_id: node.id,
                            name: dialog_name,
                            is_modal: true,
                            start_offset,
                            end_offset,
                        });
                    }
                }
            }
        }

        if pushed_fmt {
            self.formatting_stack.pop();
        }
    }

    fn collect_table_rows(node: &AomNode, rows: &mut Vec<AomNode>) {
        if node.role == Role::TableRow {
            rows.push(node.clone());
        } else {
            for child in &node.children {
                Self::collect_table_rows(child, rows);
            }
        }
    }

    fn collect_table_cells(node: &AomNode, cells: &mut Vec<AomNode>) {
        if node.role == Role::TableCell {
            cells.push(node.clone());
        } else {
            for child in &node.children {
                Self::collect_table_cells(child, cells);
            }
        }
    }

    fn traverse_tab(
        &mut self,
        node: &AomNode,
        index: usize,
        total: usize,
        group_name: Option<String>,
        buffer: &mut VirtualBuffer,
    ) {
        if node.states.hidden
            || node
                .attributes
                .get("aria-hidden")
                .map(|v| v == "true")
                .unwrap_or(false)
            || node.attributes.contains_key("hidden")
        {
            return;
        }

        buffer.node_map.insert(node.id, node.clone());

        if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
            buffer.text.push('\n');
        }
        let start_offset = buffer.text.len();
        buffer.tab_stops.push(start_offset);
        buffer.tabs.push(TabInfo {
            node_id: node.id,
            offset: start_offset,
            index,
            total,
            group_name,
        });

        let label = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
        buffer.text.push_str(label);

        let end_offset = buffer.text.len();
        let text_content = buffer.text[start_offset..end_offset].to_string();

        buffer.spans.push(BufferSpan {
            start_offset,
            end_offset,
            node_id: node.id,
            role: Role::Tab,
        });
        buffer.lines.push(BufferLine {
            line_index: buffer.lines.len(),
            start_offset,
            end_offset,
            node_id: node.id,
            role: Role::Tab,
            text: text_content.clone(),
        });
        buffer.paragraphs.push(BufferParagraph {
            paragraph_index: buffer.paragraphs.len(),
            start_offset,
            end_offset,
            node_id: node.id,
            role: Role::Tab,
            text: text_content,
        });
    }

    fn traverse_tree_item(
        &mut self,
        node: &AomNode,
        default_level: u32,
        parent_id: Option<usize>,
        buffer: &mut VirtualBuffer,
    ) {
        if node.states.hidden
            || node
                .attributes
                .get("aria-hidden")
                .map(|v| v == "true")
                .unwrap_or(false)
            || node.attributes.contains_key("hidden")
        {
            return;
        }

        let level = node.level.unwrap_or(default_level);
        let mut node_with_level = node.clone();
        node_with_level.level = Some(level);
        buffer.node_map.insert(node.id, node_with_level);

        if let Some(pid) = parent_id {
            buffer.tree_parents.insert(node.id, pid);
        }

        if !buffer.text.is_empty() && !buffer.text.ends_with('\n') {
            buffer.text.push('\n');
        }
        let start_offset = buffer.text.len();
        buffer.tree_items.push(start_offset);
        buffer.tab_stops.push(start_offset);

        let label = node.name.as_deref().or(node.text.as_deref()).unwrap_or("");
        buffer.text.push_str(label);

        let end_offset = buffer.text.len();
        let text_content = buffer.text[start_offset..end_offset].to_string();

        buffer.spans.push(BufferSpan {
            start_offset,
            end_offset,
            node_id: node.id,
            role: Role::TreeItem,
        });
        buffer.lines.push(BufferLine {
            line_index: buffer.lines.len(),
            start_offset,
            end_offset,
            node_id: node.id,
            role: Role::TreeItem,
            text: text_content.clone(),
        });
        buffer.paragraphs.push(BufferParagraph {
            paragraph_index: buffer.paragraphs.len(),
            start_offset,
            end_offset,
            node_id: node.id,
            role: Role::TreeItem,
            text: text_content,
        });

        for child in &node.children {
            if child.role == Role::TreeItem {
                self.traverse_tree_item(child, level + 1, Some(node.id), buffer);
            } else {
                self.traverse_node(child, buffer);
            }
        }
    }
}
