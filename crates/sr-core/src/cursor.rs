use crate::buffer::VirtualBuffer;
use crate::speech::SpeechFormatter;

/// The Virtual Cursor providing reading navigation primitives over a VirtualBuffer.
#[derive(Debug, Clone)]
pub struct VirtualCursor {
    pub buffer: VirtualBuffer,
    pub offset: usize,
}

impl VirtualCursor {
    pub fn new(buffer: VirtualBuffer) -> Self {
        Self { buffer, offset: 0 }
    }

    /// Resets the cursor to the top of document (`^{HOME}`).
    pub fn move_to_start(&mut self) -> String {
        self.offset = 0;
        self.current_line()
    }

    /// Moves cursor to the end of the document (`^{END}`).
    pub fn move_to_end(&mut self) -> String {
        self.offset = self.buffer.len();
        self.current_line()
    }

    /// Moves cursor down to next line (`{DOWN}`).
    pub fn move_next_line(&mut self) -> Option<String> {
        if let Some(next_line) = self.buffer.next_line_from(self.offset) {
            self.offset = next_line.start_offset;
            Some(SpeechFormatter::format_line(next_line))
        } else {
            None
        }
    }

    /// Moves cursor up to prior line (`{UP}`).
    pub fn move_prior_line(&mut self) -> Option<String> {
        if let Some(prior_line) = self.buffer.prior_line_from(self.offset) {
            self.offset = prior_line.start_offset;
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
}
