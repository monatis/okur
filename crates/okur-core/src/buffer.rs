use crate::aom::{AomNode, Role, TextFormatting};
use std::collections::{HashMap, HashSet};

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
    /// The link's current state (`aria-current`) as JAWS words it, "Current Page".
    pub current: Option<&'static str>,
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
    /// A check box's or radio button's state.
    pub checked: Option<bool>,
    /// A combo box one types in (an edit combo).
    pub editable: bool,
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
    /// What JAWS speaks for the paragraph (Ctrl+Up/Down): its lines' speech, space-joined.
    pub speech: String,
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
    /// The node the buffer was built from (the page).
    pub root_id: Option<usize>,
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
    /// Elements that can take the focus but are not controls (WhatsApp's messages, a
    /// `tabindex` card), as (their first line's offset, node id), in offset order; of
    /// nested ones starting on one line, the innermost.
    pub focus_stops: Vec<(usize, usize)>,
    pub tabs: Vec<TabInfo>,
    pub tables: Vec<TableGrid>,
    pub table_offsets: Vec<usize>,
    pub dialogs: Vec<DialogInfo>,
    pub frames: Vec<FrameInfo>,
    pub frame_offsets: Vec<usize>,
    pub math_blocks: Vec<usize>,
    pub target_anchors: HashMap<String, usize>,
    /// The page the buffer was built from (`root_id`), with what the buffer adds to it:
    /// radio buttons' groups, same-page links, and state changes made through the buffer.
    /// Nodes are read through [`VirtualBuffer::node`].
    page: Option<AomNode>,
    /// Where each page node sits, so a node is found without copying it: one slot per node,
    /// pointing at its parent's slot.
    slots: Vec<NodeSlot>,
    /// Each node id's slot (for an id the page repeats, its last node in document order).
    slot_of: HashMap<usize, usize>,
    /// The page nodes the buffer reads; hidden ones are not.
    readable: HashSet<usize>,
}

/// A page node's place: its parent's slot (none for the page itself) and its index among
/// the parent's children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NodeSlot {
    parent: Option<usize>,
    index: usize,
}

impl VirtualBuffer {
    /// Creates a VirtualBuffer from an AOM root node.
    pub fn from_aom(root: &AomNode) -> Self {
        crate::linearize::build(root)
    }

    /// The buffer rebuilt from its page with one node changed, as when the page reports a
    /// check box toggled or a value changed. `None` when the node is not in the page.
    pub fn with_node_changed(
        &self,
        node_id: usize,
        change: impl FnOnce(&mut AomNode),
    ) -> Option<VirtualBuffer> {
        fn find(node: &mut AomNode, id: usize) -> Option<&mut AomNode> {
            if node.id == id {
                return Some(node);
            }
            node.children.iter_mut().find_map(|c| find(c, id))
        }
        let mut root = self.page.clone()?;
        change(find(&mut root, node_id)?);
        Some(VirtualBuffer::from_aom(&root))
    }

    /// The buffer rebuilt from its page with one part read again: `subtree` replaces the node
    /// that is the same live object (its [`crate::aom::UNIQUE_ID_ATTR`]), and its nodes get ids
    /// the page does not use. `None` when the page has no such object (it is new: the caller
    /// reads the whole page again).
    pub fn with_subtree_replaced(&self, subtree: &AomNode) -> Option<VirtualBuffer> {
        use crate::aom::UNIQUE_ID_ATTR;
        fn max_id(node: &AomNode) -> usize {
            node.children.iter().map(max_id).fold(node.id, usize::max)
        }
        fn renumber(node: &mut AomNode, next: &mut usize) {
            node.id = *next;
            *next += 1;
            for child in &mut node.children {
                renumber(child, next);
            }
        }
        fn find<'a>(node: &'a mut AomNode, object: &str) -> Option<&'a mut AomNode> {
            if node.attributes.get(UNIQUE_ID_ATTR).map(String::as_str) == Some(object) {
                return Some(node);
            }
            node.children.iter_mut().find_map(|c| find(c, object))
        }
        let object = subtree.attributes.get(UNIQUE_ID_ATTR)?;
        let mut root = self.page.clone()?;
        let mut next = max_id(&root) + 1;
        let mut replacement = subtree.clone();
        renumber(&mut replacement, &mut next);
        *find(&mut root, object)? = replacement;
        Some(VirtualBuffer::from_aom(&root))
    }

    /// Makes `page` the page the buffer reads its nodes from, and indexes where its nodes
    /// sit (the linearizer then marks the ones it reads).
    pub(crate) fn set_page(&mut self, page: AomNode) {
        self.slots.clear();
        self.slot_of.clear();
        self.readable.clear();
        self.root_id = Some(page.id);
        self.slots.push(NodeSlot {
            parent: None,
            index: 0,
        });
        // Document order, so that an id the page repeats keeps its last node, as the
        // linearizer meets them.
        let mut stack = vec![(&page, 0_usize)];
        while let Some((node, slot)) = stack.pop() {
            self.slot_of.insert(node.id, slot);
            for (index, child) in node.children.iter().enumerate().rev() {
                self.slots.push(NodeSlot {
                    parent: Some(slot),
                    index,
                });
                stack.push((child, self.slots.len() - 1));
            }
        }
        self.page = Some(page);
    }

    /// Marks page node `id` as one the buffer reads.
    pub(crate) fn mark_readable(&mut self, id: usize) {
        self.readable.insert(id);
    }

    /// Child indexes from the page down to the node in `slot`.
    fn path_to(&self, slot: usize) -> Option<Vec<usize>> {
        let mut path = Vec::new();
        let mut current = self.slots.get(slot)?;
        while let Some(parent) = current.parent {
            path.push(current.index);
            current = self.slots.get(parent)?;
        }
        path.reverse();
        Some(path)
    }

    /// The page node that had keyboard focus when the page was captured (the innermost). The
    /// document itself focused is no element focused.
    pub fn focused_node(&self) -> Option<usize> {
        fn find(node: &AomNode) -> Option<usize> {
            node.children
                .iter()
                .find_map(find)
                .or((node.states.focused && node.role != Role::Document).then_some(node.id))
        }
        self.page.as_ref().and_then(find)
    }

    /// Where the node `id` starts in the buffer: its own span, else the first span of a node
    /// inside it (an element whose lines are its children's: WhatsApp's focused message, its
    /// PDF viewer's content).
    pub fn start_of(&self, id: usize) -> Option<usize> {
        if let Some(span) = self.spans.iter().find(|s| s.node_id == id) {
            return Some(span.start_offset);
        }
        fn subtree(node: &AomNode, ids: &mut HashSet<usize>) {
            ids.insert(node.id);
            for child in &node.children {
                subtree(child, ids);
            }
        }
        let mut ids = HashSet::new();
        subtree(self.node(id)?, &mut ids);
        self.spans
            .iter()
            .find(|s| ids.contains(&s.node_id))
            .map(|s| s.start_offset)
    }

    /// Whether the buffer has text of the node `id` or of a node inside it.
    pub fn shows(&self, id: usize) -> bool {
        fn subtree(node: &AomNode, ids: &mut HashSet<usize>) {
            ids.insert(node.id);
            for child in &node.children {
                subtree(child, ids);
            }
        }
        let Some(node) = self.node(id) else {
            return false;
        };
        let mut ids = HashSet::new();
        subtree(node, &mut ids);
        self.spans.iter().any(|s| ids.contains(&s.node_id))
    }

    /// The page node with DOM id `dom_id` (the first, where a page repeats an id).
    pub fn node_with_dom_id(&self, dom_id: &str) -> Option<usize> {
        self.nodes()
            .find(|n| n.attributes.get("id").is_some_and(|id| id == dom_id))
            .map(|n| n.id)
    }

    /// Every page node in document order, read or not: a paragraph or a division has no line
    /// of its own, only its children's.
    pub fn page_nodes(&self) -> impl Iterator<Item = &AomNode> + '_ {
        let mut stack: Vec<&AomNode> = self.page.iter().collect();
        std::iter::from_fn(move || {
            let node = stack.pop()?;
            stack.extend(node.children.iter().rev());
            Some(node)
        })
    }

    /// Where `node`'s subtree starts in the buffer: the first span of it.
    pub fn start_of_subtree(&self, node: &AomNode) -> Option<usize> {
        fn subtree(node: &AomNode, ids: &mut HashSet<usize>) {
            ids.insert(node.id);
            for child in &node.children {
                subtree(child, ids);
            }
        }
        let mut ids = HashSet::new();
        subtree(node, &mut ids);
        self.spans
            .iter()
            .find(|s| ids.contains(&s.node_id))
            .map(|s| s.start_offset)
    }

    /// The page nodes around node `id`, nearest first: its parent, the parent's parent, up to
    /// the page.
    pub fn ancestors(&self, id: usize) -> Vec<&AomNode> {
        let (Some(page), Some(path)) = (
            self.page.as_ref(),
            self.slot_of.get(&id).and_then(|&slot| self.path_to(slot)),
        ) else {
            return Vec::new();
        };
        let mut chain = vec![page];
        let mut node = page;
        for &index in path.iter().take(path.len().saturating_sub(1)) {
            match node.children.get(index) {
                Some(child) => node = child,
                None => break,
            }
            chain.push(node);
        }
        chain.reverse();
        chain
    }

    /// The page node `id`, with its subtree, if the buffer reads it.
    pub fn node(&self, id: usize) -> Option<&AomNode> {
        if !self.readable.contains(&id) {
            return None;
        }
        let mut node = self.page.as_ref()?;
        for index in self.path_to(*self.slot_of.get(&id)?)? {
            node = node.children.get(index)?;
        }
        Some(node)
    }

    /// The page node `id`, to change what the buffer knows of it (a combo box's value, a
    /// tree item's expansion).
    pub fn node_mut(&mut self, id: usize) -> Option<&mut AomNode> {
        if !self.readable.contains(&id) {
            return None;
        }
        let path = self.path_to(*self.slot_of.get(&id)?)?;
        let mut node = self.page.as_mut()?;
        for index in path {
            node = node.children.get_mut(index)?;
        }
        Some(node)
    }

    /// Whether the buffer reads page node `id`.
    pub fn contains_node(&self, id: usize) -> bool {
        self.readable.contains(&id)
    }

    /// The ids of the page nodes the buffer reads, in no particular order.
    pub fn node_ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.readable.iter().copied()
    }

    /// The page nodes the buffer reads, in document order.
    pub fn nodes(&self) -> impl Iterator<Item = &AomNode> + '_ {
        let mut stack: Vec<&AomNode> = self.page.iter().collect();
        std::iter::from_fn(move || {
            while let Some(node) = stack.pop() {
                stack.extend(node.children.iter().rev());
                if self.readable.contains(&node.id) {
                    return Some(node);
                }
            }
            None
        })
    }

    /// Applies `change` to every page node the buffer reads.
    pub fn for_each_node_mut(&mut self, mut change: impl FnMut(&mut AomNode)) {
        let readable = &self.readable;
        let mut stack: Vec<&mut AomNode> = self.page.iter_mut().collect();
        while let Some(node) = stack.pop() {
            if readable.contains(&node.id) {
                change(node);
            }
            stack.extend(node.children.iter_mut());
        }
    }

    /// Returns the character at the specified byte/char offset.
    pub fn char_at(&self, offset: usize) -> Option<char> {
        self.text.get(offset..)?.chars().next()
    }

    /// Total length in bytes of the buffer text.
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Returns the line containing the given offset (the first, where two share it), or the
    /// last line. Lines are in document order, their starts and ends never decreasing, so this
    /// is a binary search: it ran in the keyboard hook once per tab stop (responsiveness review,
    /// 2026-10-19).
    pub fn line_at_offset(&self, offset: usize) -> Option<&BufferLine> {
        let first = self.lines.partition_point(|line| line.end_offset < offset);
        self.lines
            .get(first)
            .filter(|line| line.start_offset <= offset)
            .or_else(|| self.lines.last())
    }

    /// Returns the next line starting after the given offset.
    pub fn next_line_from(&self, offset: usize) -> Option<&BufferLine> {
        let next = self.lines.partition_point(|l| l.start_offset <= offset);
        self.lines.get(next)
    }

    /// Returns the prior line starting before the current line of the given offset.
    pub fn prior_line_from(&self, offset: usize) -> Option<&BufferLine> {
        let current_line_start = self
            .line_at_offset(offset)
            .map(|l| l.start_offset)
            .unwrap_or(offset);
        let before = self
            .lines
            .partition_point(|l| l.start_offset < current_line_start);
        before.checked_sub(1).and_then(|i| self.lines.get(i))
    }

    /// Returns the word containing or starting at the given offset.
    pub fn word_at_offset(&self, offset: usize) -> Option<&BufferWord> {
        let first = self.words.partition_point(|w| w.end_offset <= offset);
        self.words
            .get(first)
            .filter(|w| w.start_offset <= offset)
            .or_else(|| self.next_word_starting_at(offset))
    }

    /// The first word starting at `offset` or after it.
    fn next_word_starting_at(&self, offset: usize) -> Option<&BufferWord> {
        let first = self.words.partition_point(|w| w.start_offset < offset);
        self.words.get(first)
    }

    /// Returns the next word starting after the given offset.
    pub fn next_word_from(&self, offset: usize) -> Option<&BufferWord> {
        let next = self.words.partition_point(|w| w.start_offset <= offset);
        self.words.get(next)
    }

    /// Returns the prior word starting before the given offset.
    pub fn prior_word_from(&self, offset: usize) -> Option<&BufferWord> {
        let before = self.words.partition_point(|w| w.start_offset < offset);
        before.checked_sub(1).and_then(|i| self.words.get(i))
    }

    /// Returns the paragraph containing the given offset.
    pub fn paragraph_at_offset(&self, offset: usize) -> Option<&BufferParagraph> {
        let first = self.paragraphs.partition_point(|p| p.end_offset < offset);
        self.paragraphs
            .get(first)
            .filter(|p| p.start_offset <= offset)
            .or_else(|| self.paragraphs.last())
    }

    /// Returns the next paragraph starting after the given offset.
    pub fn next_paragraph_from(&self, offset: usize) -> Option<&BufferParagraph> {
        let next = self
            .paragraphs
            .partition_point(|p| p.start_offset <= offset);
        self.paragraphs.get(next)
    }

    /// Returns the prior paragraph starting before the current paragraph.
    pub fn prior_paragraph_from(&self, offset: usize) -> Option<&BufferParagraph> {
        let current_p_start = self
            .paragraph_at_offset(offset)
            .map(|p| p.start_offset)
            .unwrap_or(offset);
        let before = self
            .paragraphs
            .partition_point(|p| p.start_offset < current_p_start);
        before.checked_sub(1).and_then(|i| self.paragraphs.get(i))
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

    /// The page's address, when it has one.
    pub fn document_url(&self) -> Option<&str> {
        self.page
            .as_ref()?
            .attributes
            .get("url")
            .map(String::as_str)
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
            .and_then(|s| self.node(s.node_id))
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
            .and_then(|s| self.node(s.node_id))
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
            .and_then(|s| self.node(s.node_id))
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
            .and_then(|s| self.node(s.node_id))
    }

    /// Resolves the node for a button starting at `offset`.
    pub fn find_button_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Button | Role::MenuButton))
            .and_then(|s| self.node(s.node_id))
    }

    /// Resolves the node for an edit box starting at `offset`.
    pub fn find_edit_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Edit))
            .and_then(|s| self.node(s.node_id))
    }

    /// Resolves the node for a checkbox starting at `offset`.
    pub fn find_checkbox_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::CheckBox))
            .and_then(|s| self.node(s.node_id))
    }

    /// Resolves the node for a heading starting at `offset`.
    pub fn find_heading_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Heading { .. }))
            .and_then(|s| self.node(s.node_id))
    }

    /// Resolves the node for a link starting at `offset`.
    pub fn find_link_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Link { .. }))
            .and_then(|s| self.node(s.node_id))
    }

    /// Resolves the node for a list starting at `offset`.
    pub fn find_list_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::List))
            .and_then(|s| self.node(s.node_id))
    }

    /// Resolves the node for a list item starting at `offset`.
    pub fn find_list_item_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::ListItem))
            .and_then(|s| self.node(s.node_id))
    }

    /// Resolves the node for a landmark starting at `offset`.
    pub fn find_landmark_node(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .find(|s| s.start_offset == offset && matches!(s.role, Role::Landmark { .. }))
            .and_then(|s| self.node(s.node_id))
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
            .and_then(|s| self.node(s.node_id))
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
            .and_then(|s| self.text.get(s.start_offset..s.end_offset))
    }

    /// Finds the AOM node associated with the given buffer offset.
    pub fn node_at_offset(&self, offset: usize) -> Option<&AomNode> {
        self.spans
            .iter()
            .filter(|s| offset >= s.start_offset && offset <= s.end_offset)
            .min_by_key(|s| s.end_offset - s.start_offset)
            .and_then(|s| self.node(s.node_id))
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
            .and_then(|s| self.node(s.node_id))
    }

    /// Resolves font formatting attributes at the given buffer offset.
    pub fn formatting_at_offset(&self, offset: usize) -> Option<TextFormatting> {
        self.spans
            .iter()
            .filter(|s| offset >= s.start_offset && offset <= s.end_offset)
            .min_by_key(|s| s.end_offset - s.start_offset)
            .and_then(|s| self.node(s.node_id))
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
                // The linearizer marks same page links.
                let is_same_page = node
                    .attributes
                    .get("same_page")
                    .is_some_and(|v| v == "true");
                items.push(LinkListItem {
                    node_id: node.id,
                    text,
                    href,
                    is_same_page,
                    offset,
                    current: node.current_phrase(),
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

    /// The document's form fields in its order, for Select a Form Field: every element
    /// `form_controls` holds, button menus and radio buttons too [obs, 227].
    pub fn get_form_fields_list(&self) -> Vec<FormFieldListItem> {
        let field = |offset: usize| {
            self.spans
                .iter()
                .find(|s| {
                    s.start_offset == offset
                        && matches!(
                            s.role,
                            Role::Edit
                                | Role::Button
                                | Role::MenuButton
                                | Role::CheckBox
                                | Role::RadioButton
                                | Role::ComboBox
                                | Role::Slider
                        )
                })
                .and_then(|s| self.node(s.node_id))
        };
        self.form_controls
            .iter()
            .filter_map(|&offset| {
                let node = field(offset)?;
                Some(FormFieldListItem {
                    node_id: node.id,
                    name: node
                        .name
                        .clone()
                        .or_else(|| node.text.clone())
                        .unwrap_or_default(),
                    role: node.role.clone(),
                    value: node.value.clone(),
                    checked: node.states.checked,
                    editable: node.attributes.get("editable").is_some_and(|e| e == "true"),
                    offset,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::VirtualBuffer;
    use crate::aom::{AomNode, NodeStates, Role};

    fn page() -> AomNode {
        let hidden = AomNode::new(4, Role::Paragraph)
            .with_text("Hidden")
            .with_states(NodeStates {
                hidden: true,
                ..NodeStates::default()
            });
        AomNode::new(0, Role::Document)
            .with_name("Page")
            .with_children(vec![
                AomNode::new(1, Role::List).with_children(vec![
                    AomNode::new(2, Role::ListItem).with_text("One"),
                    AomNode::new(3, Role::ListItem).with_text("Two"),
                ]),
                hidden,
            ])
    }

    fn text(buffer: Option<&VirtualBuffer>, id: usize) -> Option<&str> {
        buffer?.node(id)?.text.as_deref()
    }

    #[test]
    fn nodes_are_read_with_their_subtrees_and_hidden_ones_are_not() {
        let buffer = VirtualBuffer::from_aom(&page());
        assert_eq!(buffer.node(1).map(|list| list.children.len()), Some(2));
        assert_eq!(text(Some(&buffer), 3), Some("Two"));
        assert!(!buffer.contains_node(4));
        assert!(buffer.node(4).is_none());
        let order: Vec<usize> = buffer.nodes().map(|node| node.id).collect();
        assert_eq!(order, vec![0, 1, 2, 3]);
    }

    #[test]
    fn a_change_made_through_the_buffer_is_part_of_the_page_and_survives_a_rebuild() {
        let mut buffer = VirtualBuffer::from_aom(&page());
        if let Some(item) = buffer.node_mut(2) {
            item.text = Some("First".to_owned());
        }
        let first_item = buffer.node(1).and_then(|list| list.children.first());
        assert_eq!(
            first_item.and_then(|item| item.text.as_deref()),
            Some("First")
        );

        let rebuilt = buffer.with_node_changed(3, |item| item.text = Some("Second".to_owned()));
        assert_eq!(text(rebuilt.as_ref(), 2), Some("First"));
        assert_eq!(text(rebuilt.as_ref(), 3), Some("Second"));
    }

    #[test]
    fn a_repeated_id_reads_its_last_node_in_document_order() {
        let page = AomNode::new(0, Role::Document).with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Earlier"),
            AomNode::new(1, Role::Paragraph).with_text("Later"),
        ]);
        let buffer = VirtualBuffer::from_aom(&page);
        assert_eq!(text(Some(&buffer), 1), Some("Later"));
    }

    /// A page of `leaves` paragraphs under a chain of `depth` wrapping groups, as web apps nest
    /// their content.
    fn wrapped_page(depth: usize, leaves: usize) -> AomNode {
        let paragraphs = (0..leaves)
            .map(|i| AomNode::new(10_000 + i, Role::Paragraph).with_text("A line of the page."))
            .collect();
        let mut root = AomNode::new(1, Role::Group).with_children(paragraphs);
        for id in 2..depth + 2 {
            root = AomNode::new(id, Role::Group).with_children(vec![root]);
        }
        AomNode::new(0, Role::Document).with_children(vec![root])
    }

    /// The fastest of three builds of `page`.
    fn build_time(page: &AomNode) -> std::time::Duration {
        (0..3)
            .map(|_| {
                let started = std::time::Instant::now();
                let buffer = VirtualBuffer::from_aom(page);
                let elapsed = started.elapsed();
                assert!(!buffer.lines.is_empty());
                elapsed
            })
            .min()
            .unwrap_or_default()
    }

    /// Building a buffer stays linear in the page: wrapping the same content four times
    /// deeper adds a few nodes, not a copy of the content per level (the node map once held a
    /// deep copy of every node, 2026-10-15). A ratio, so the machine's speed does not matter:
    /// linear building gives about 1, copying per level about 4.
    #[test]
    fn building_a_buffer_does_not_grow_with_wrapping_depth() {
        let shallow = build_time(&wrapped_page(50, 2000));
        let deep = build_time(&wrapped_page(200, 2000));
        let ratio = deep.as_secs_f64() / shallow.as_secs_f64().max(f64::EPSILON);
        assert!(
            ratio < 2.0,
            "4x deeper wrapping took {ratio:.2}x as long ({shallow:?} -> {deep:?})"
        );
    }
}
