//! What JAWS speaks for keys in a text field.
//!
//! JAWS speaks after the application has handled the key, from the field's text, caret and
//! selection before and after it ([`TextContext`]). The engine keeps the last context
//! and pairs the next one with the key that was pressed. Observations are from
//! `305-desktop-edit-review` (a WinForms edit, 5 trials), cited as `305/sN`.

use crate::event::TextContext;
use crate::input::{Key, KeyEvent};

/// A key whose effect on a text field JAWS speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKey {
    /// Left/Right: the character at the caret (305/s2).
    Character,
    /// Ctrl+Left/Right: the word at the caret.
    Word,
    /// Up/Down, PageUp/PageDown, Ctrl+Home/End: the line at the caret.
    Line,
    /// Home/End: silent in a single-line edit (305/s1, s5, s7).
    Silent,
    /// Shift+Left/Right: the character added to the selection, then "selected" (305/s3).
    SelectCharacter,
    /// Other Shift+ moves: "selected", then the text added (305/s4).
    SelectMore,
    /// Backspace: the character it deleted (305/s6).
    DeleteBackward,
    /// Delete: the character now at the caret (305/s8).
    DeleteForward,
    /// Insert+Up (Say Line): the field's name and role, then the line (305/s9, s15).
    SayLine,
    /// Ctrl+A: "selected", then the first line of the selection (313/s7).
    SelectAll,
    /// Ctrl+C: "Copied selection to clipboard" when text is selected (313/s8).
    Copy,
}

impl EditKey {
    /// The edit key a key press is, if any.
    pub fn of(event: &KeyEvent) -> Option<Self> {
        let m = &event.modifiers;
        if m.alt || m.win {
            return None;
        }
        if m.insert {
            return (event.key == Key::UpArrow && !m.ctrl && !m.shift).then_some(Self::SayLine);
        }
        let horizontal = matches!(event.key, Key::LeftArrow | Key::RightArrow);
        let vertical = matches!(
            event.key,
            Key::UpArrow | Key::DownArrow | Key::PageUp | Key::PageDown
        );
        let ends = matches!(event.key, Key::Home | Key::End);
        Some(match (m.shift, m.ctrl) {
            (true, false) if horizontal => Self::SelectCharacter,
            (true, _) if horizontal || vertical || ends => Self::SelectMore,
            (false, false) if horizontal => Self::Character,
            (false, true) if horizontal => Self::Word,
            (false, _) if vertical => Self::Line,
            (false, true) if ends => Self::Line,
            (false, false) if ends => Self::Silent,
            (false, false) if event.key == Key::Backspace => Self::DeleteBackward,
            (false, false) if event.key == Key::Delete => Self::DeleteForward,
            (false, true) if event.key == Key::Alpha('A') => Self::SelectAll,
            (false, true) if event.key == Key::Alpha('C') => Self::Copy,
            _ => return None,
        })
    }
}

fn chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

/// Whether `c` ends a line. Win32 edits use `\r\n`, RichEdit (Windows 11 Notepad, WordPad)
/// a bare `\r`, browsers `\n`; Unicode adds its line and paragraph separators.
pub fn is_line_break(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `text` split into lines at any line break (`\r\n` counts once).
pub fn text_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if is_line_break(c) {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            lines.push(std::mem::take(&mut line));
        } else {
            line.push(c);
        }
    }
    lines.push(line);
    lines
}

/// The character at `caret` (a line break too, which JAWS's history records as an empty
/// entry and speaks as "blank" [obs, 307/s36]), or `None` at the end of the text.
fn char_at(context: &TextContext) -> Option<char> {
    chars(&context.text).get(context.caret).copied()
}

/// The line containing the caret, without its line break.
pub fn line_at(context: &TextContext) -> String {
    let text = chars(&context.text);
    let caret = context.caret.min(text.len());
    let start = text
        .get(..caret)
        .and_then(|before| before.iter().rposition(|c| is_line_break(*c)))
        .map_or(0, |i| i + 1);
    let end = text
        .get(caret..)
        .and_then(|after| after.iter().position(|c| is_line_break(*c)))
        .map_or(text.len(), |i| caret + i);
    text.get(start..end).unwrap_or_default().iter().collect()
}

/// The word starting at the caret (where Ctrl+Left/Right leave it).
fn word_at(context: &TextContext) -> String {
    chars(&context.text)
        .get(context.caret..)
        .unwrap_or_default()
        .iter()
        .take_while(|c| !c.is_whitespace())
        .collect()
}

fn selected_text(context: &TextContext) -> String {
    let text = chars(&context.text);
    match context.selection {
        Some((start, end)) if start < end => {
            text.get(start..end).unwrap_or_default().iter().collect()
        }
        _ => String::new(),
    }
}

/// The text `after` has selected beyond `before` (one end of the selection moved).
fn newly_selected(before: Option<&TextContext>, after: &TextContext) -> String {
    let now = selected_text(after);
    let was = before.map(selected_text).unwrap_or_default();
    if let Some(added) = now.strip_prefix(was.as_str()) {
        return added.to_owned();
    }
    if let Some(added) = now.strip_suffix(was.as_str()) {
        return added.to_owned();
    }
    now
}

/// The text Backspace removed: what `before` has and `after` lacks, ending at the caret.
fn deleted_before_caret(before: Option<&TextContext>, after: &TextContext) -> String {
    let Some(before) = before else {
        return String::new();
    };
    let was = chars(&before.text);
    let removed = was.len().saturating_sub(chars(&after.text).len());
    let end = before.caret.min(was.len());
    was.get(end.saturating_sub(removed)..end)
        .unwrap_or_default()
        .iter()
        .collect()
}

fn blank_if_empty(text: String) -> String {
    if text.trim().is_empty() {
        "blank".to_owned()
    } else {
        text.trim().to_owned()
    }
}

/// What JAWS speaks after `key`, given the field before and after it. `field` is the
/// field's name-and-role line, for Say Line.
pub fn speech(
    key: EditKey,
    before: Option<&TextContext>,
    after: &TextContext,
    field: &str,
) -> Option<String> {
    Some(match key {
        EditKey::Silent => return None,
        EditKey::Character | EditKey::DeleteForward => {
            char_at(after).map_or_else(|| "blank".to_owned(), |c| c.to_string())
        }
        EditKey::Word => blank_if_empty(word_at(after)),
        EditKey::Line => blank_if_empty(line_at(after)),
        EditKey::SelectCharacter => {
            let added = newly_selected(before, after);
            if added.is_empty() {
                return None;
            }
            format!("{added}\nselected")
        }
        EditKey::SelectMore => {
            let added = newly_selected(before, after);
            if added.is_empty() {
                return None;
            }
            format!("selected\n{added}")
        }
        EditKey::DeleteBackward => {
            let deleted = deleted_before_caret(before, after);
            if deleted.is_empty() {
                return None;
            }
            deleted
        }
        EditKey::SelectAll => {
            let selected = selected_text(after);
            let first = text_lines(&selected)
                .into_iter()
                .find(|l| !l.trim().is_empty())?;
            format!("selected\n{}", first.trim())
        }
        EditKey::Copy => {
            if selected_text(after).is_empty() {
                return None;
            }
            "Copied selection to clipboard".to_owned()
        }
        EditKey::SayLine => {
            let line = line_at(after);
            if line.trim().is_empty() {
                field.to_owned()
            } else {
                format!("{field}\n{}", line.trim())
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(text: &str, caret: usize, selection: Option<(usize, usize)>) -> TextContext {
        TextContext {
            text: text.to_owned(),
            caret,
            selection,
            font: None,
        }
    }

    // The 305 sequence on the field "Ada" (caret at the end after the reset End key).
    #[test]
    fn edit_review_sequence() {
        let field = "Name:  \u{1d} Edit \u{1e} ";
        // s1 Home: silent.
        assert_eq!(
            speech(EditKey::Silent, None, &ctx("Ada", 0, None), field),
            None
        );
        // s2 Right: "d".
        assert_eq!(
            speech(EditKey::Character, None, &ctx("Ada", 1, None), field).as_deref(),
            Some("d")
        );
        // s3 Shift+Right: "d | selected".
        let s3 = ctx("Ada", 2, Some((1, 2)));
        assert_eq!(
            speech(
                EditKey::SelectCharacter,
                Some(&ctx("Ada", 1, None)),
                &s3,
                field
            )
            .as_deref(),
            Some("d\nselected")
        );
        // s4 Shift+End: "selected | a".
        let s4 = ctx("Ada", 3, Some((1, 3)));
        assert_eq!(
            speech(EditKey::SelectMore, Some(&s3), &s4, field).as_deref(),
            Some("selected\na")
        );
        // s6 Backspace at the end: "a".
        assert_eq!(
            speech(
                EditKey::DeleteBackward,
                Some(&ctx("Ada", 3, None)),
                &ctx("Ad", 2, None),
                field
            )
            .as_deref(),
            Some("a")
        );
        // s8 Delete at the start: the character now at the caret, "d".
        assert_eq!(
            speech(EditKey::DeleteForward, None, &ctx("d", 0, None), field).as_deref(),
            Some("d")
        );
        // s9 Insert+Up: "Name: ‹ Edit › | d".
        assert_eq!(
            speech(EditKey::SayLine, None, &ctx("d", 0, None), field).as_deref(),
            Some("Name:  \u{1d} Edit \u{1e} \nd")
        );
        // s15 Insert+Up on an empty field: the field alone.
        assert_eq!(
            speech(
                EditKey::SayLine,
                None,
                &ctx("", 0, None),
                "Notes:  \u{1d} Edit \u{1e} "
            )
            .as_deref(),
            Some("Notes:  \u{1d} Edit \u{1e} ")
        );
    }

    #[test]
    fn lines_and_words_in_multiline_text() {
        let text = "first line\r\nsecond line";
        assert_eq!(line_at(&ctx(text, 3, None)), "first line");
        assert_eq!(line_at(&ctx(text, 14, None)), "second line");
        assert_eq!(word_at(&ctx(text, 12, None)), "second");
        // A line break is a character: an empty history entry, spoken "blank" [obs, 307/s36].
        let at_break = speech(EditKey::Character, None, &ctx(text, 10, None), "");
        assert_eq!(at_break.as_deref(), Some("\r"));
        assert_eq!(crate::pronounce::character("\r"), "blank");
    }

    /// RichEdit (Windows 11 Notepad) separates lines with a bare `\r`: Up and Down must read
    /// each line, not the first one.
    #[test]
    fn carriage_return_lines() {
        let text = "fn main() {\r    let x = 1;\r}";
        assert_eq!(line_at(&ctx(text, 0, None)), "fn main() {");
        assert_eq!(line_at(&ctx(text, 12, None)), "    let x = 1;");
        assert_eq!(line_at(&ctx(text, 27, None)), "}");
        assert_eq!(
            speech(EditKey::Line, None, &ctx(text, 12, None), "").as_deref(),
            Some("let x = 1;")
        );
        assert_eq!(text_lines(text), ["fn main() {", "    let x = 1;", "}"]);
        assert_eq!(text_lines("a\r\nb\nc"), ["a", "b", "c"]);
    }
}
