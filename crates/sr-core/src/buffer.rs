use crate::aom::{AomNode, Role};

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

/// The linearized Virtual Buffer maintaining document offsets, spans, lines, and words.
#[derive(Debug, Clone, Default)]
pub struct VirtualBuffer {
    pub text: String,
    pub spans: Vec<BufferSpan>,
    pub lines: Vec<BufferLine>,
    pub words: Vec<BufferWord>,
    pub paragraphs: Vec<BufferParagraph>,
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

        // If generic container with children, traverse children
        if node.role == Role::Generic && !node.children.is_empty() {
            for child in &node.children {
                self.traverse_node(child, buffer);
            }
            return;
        }

        let is_block = node.role.is_block();
        if is_block && !buffer.text.is_empty() {
            buffer.text.push('\n');
        }

        let start_offset = buffer.text.len();
        let text_content = node.collect_text();

        if !text_content.is_empty() {
            buffer.text.push_str(&text_content);
            let end_offset = buffer.text.len();

            let span = BufferSpan {
                start_offset,
                end_offset,
                node_id: node.id,
                role: node.role.clone(),
            };
            buffer.spans.push(span);

            if is_block {
                let line = BufferLine {
                    line_index: buffer.lines.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: node.role.clone(),
                    text: text_content.clone(),
                };
                let paragraph = BufferParagraph {
                    paragraph_index: buffer.paragraphs.len(),
                    start_offset,
                    end_offset,
                    node_id: node.id,
                    role: node.role.clone(),
                    text: text_content,
                };
                buffer.lines.push(line);
                buffer.paragraphs.push(paragraph);
            }
        } else {
            // Traverse children if block element has no direct text
            for child in &node.children {
                self.traverse_node(child, buffer);
            }
        }
    }
}
