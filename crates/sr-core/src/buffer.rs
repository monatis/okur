use crate::aom::{AomNode, Role};
use std::collections::HashMap;

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

struct BufferBuilder;

impl BufferBuilder {
    fn new() -> Self {
        Self
    }

    fn build(&mut self, root: &AomNode, buffer: &mut VirtualBuffer) {
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
        // Skip hidden nodes
        if node.states.hidden {
            return;
        }

        buffer.node_map.insert(node.id, node.clone());

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
                let start_offset = buffer.text.len();
                buffer.links.push(start_offset);

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
                }
            }
        }
    }
}
