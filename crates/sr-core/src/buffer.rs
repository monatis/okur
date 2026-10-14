use crate::aom::{AomNode, Role, TextFormatting};
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

/// Item in a Links list dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkListItem {
    pub node_id: usize,
    pub text: String,
    pub href: Option<String>,
    pub is_same_page: bool,
    pub offset: usize,
}

/// Item in a Headings list dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingListItem {
    pub node_id: usize,
    pub text: String,
    pub level: u32,
    pub offset: usize,
}

/// Item in a Form Fields list dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormFieldListItem {
    pub node_id: usize,
    pub name: String,
    pub role: Role,
    pub value: Option<String>,
    pub offset: usize,
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
    /// Exactly what the virtual cursor speaks on this line (JAWS phrasing).
    pub speech: String,
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
    pub row: usize,      // 1-indexed row position
    pub col: usize,      // 1-indexed col position
    pub row_span: usize, // number of rows spanned (default 1)
    pub col_span: usize, // number of columns spanned (default 1)
    pub is_header: bool,
    pub is_col_header: bool,
    pub is_row_header: bool,
    pub col_header: Option<String>,
    pub row_header: Option<String>,
    pub text: String,
    /// The cell content as line reading speaks it (links keep their role markers).
    pub speech: String,
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

    pub fn first_cell(&self) -> Option<&TableCellInfo> {
        self.cell(1, 1)
    }

    pub fn last_cell(&self) -> Option<&TableCellInfo> {
        if self.rows == 0 || self.cols == 0 {
            return None;
        }
        self.cell(self.rows, self.cols)
    }

    pub fn row_start(&self, row: usize) -> Option<&TableCellInfo> {
        self.cell(row, 1)
    }

    pub fn row_end(&self, row: usize) -> Option<&TableCellInfo> {
        self.cell(row, self.cols)
    }

    pub fn col_top(&self, col: usize) -> Option<&TableCellInfo> {
        self.cell(1, col)
    }

    pub fn col_bottom(&self, col: usize) -> Option<&TableCellInfo> {
        self.cell(self.rows, col)
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
    pub radio_buttons: Vec<usize>,
    pub graphics: Vec<usize>,
    pub articles: Vec<usize>,
    /// Start of the first main landmark, if any (`Q`).
    pub main_region: Option<usize>,
    /// Elements with onMouseOver handlers (`;`); empty until a source exposes them.
    pub on_mouse_over: Vec<usize>,
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
    pub math_blocks: Vec<usize>,
    pub target_anchors: HashMap<String, usize>,
    pub node_map: HashMap<usize, AomNode>,
}

impl VirtualBuffer {
    /// Creates a VirtualBuffer from an AOM root node.
    pub fn from_aom(root: &AomNode) -> Self {
        crate::linearize::build(root)
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
        let is_on_link = self.spans.iter().any(|s| {
            s.start_offset <= offset && offset < s.end_offset && matches!(s.role, Role::Link { .. })
        });
        if is_on_link {
            let current_start = self
                .links
                .iter()
                .rfind(|&&l| l <= offset)
                .copied()
                .unwrap_or(offset);
            self.links.iter().rfind(|&&l| l < current_start).copied()
        } else {
            self.links.iter().rfind(|&&l| l <= offset).copied()
        }
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

    /// Returns the next math block starting after the given offset.
    pub fn next_math_from(&self, offset: usize) -> Option<usize> {
        self.math_blocks.iter().copied().find(|&o| o > offset)
    }

    /// Returns the prior math block starting before the current math block of the given offset.
    pub fn prior_math_from(&self, offset: usize) -> Option<usize> {
        let current_start = self
            .math_blocks
            .iter()
            .copied()
            .rfind(|&o| o <= offset)
            .unwrap_or(offset);
        self.math_blocks
            .iter()
            .copied()
            .rfind(|&o| o < current_start)
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

    /// Returns the list of all links in the document for the Links List dialog.
    pub fn get_links_list(&self) -> Vec<LinkListItem> {
        let mut items = Vec::new();
        for &offset in &self.links {
            if let Some(node) = self.find_link_node(offset) {
                let text = node
                    .name
                    .clone()
                    .or_else(|| node.text.clone())
                    .or_else(|| self.span_text_at_offset(offset).map(|s| s.to_string()))
                    .unwrap_or_default();
                let href = match node.role {
                    Role::Link { ref href } => href.clone(),
                    _ => None,
                };
                let is_same_page = href.as_deref().map(|h| h.starts_with('#')).unwrap_or(false);
                items.push(LinkListItem {
                    node_id: node.id,
                    text,
                    href,
                    is_same_page,
                    offset,
                });
            }
        }
        items
    }

    /// Returns the list of all headings in the document for the Headings List dialog.
    pub fn get_headings_list(&self) -> Vec<HeadingListItem> {
        let mut items = Vec::new();
        for &(offset, level) in &self.headings {
            let node = self.find_heading_node(offset);
            let text = node
                .and_then(|n| n.text.as_deref().or(n.name.as_deref()))
                .or_else(|| self.line_at_offset(offset).map(|l| l.text.as_str()))
                .unwrap_or("")
                .to_string();
            let node_id = node.map(|n| n.id).unwrap_or(0);
            items.push(HeadingListItem {
                node_id,
                text,
                level,
                offset,
            });
        }
        items
    }

    /// Returns the list of all interactive form controls in the document for Form Fields List.
    pub fn get_form_fields_list(&self) -> Vec<FormFieldListItem> {
        let mut items = Vec::new();
        for &offset in &self.form_controls {
            if let Some(node) = self.find_form_control_node(offset) {
                let name = node
                    .name
                    .clone()
                    .or_else(|| node.text.clone())
                    .unwrap_or_default();
                items.push(FormFieldListItem {
                    node_id: node.id,
                    name,
                    role: node.role.clone(),
                    value: node.value.clone(),
                    offset,
                });
            }
        }
        items
    }
}
