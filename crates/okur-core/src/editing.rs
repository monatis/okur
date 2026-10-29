//! What JAWS speaks for keys in a text field.
//!
//! JAWS speaks after the application has handled the key, from the field's text, caret and
//! selection before and after it ([`TextContext`]). The engine keeps the last context
//! and pairs the next one with the key that was pressed. Observations are from
//! `305-desktop-edit-review` (a WinForms edit, 5 trials), cited as `305/sN`.

use crate::apps::windows::word;
use crate::event::TextContext;
use crate::input::{Key, KeyEvent};
use crate::messages::msg;

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
    /// Ctrl+Shift+Left/Right: the word added to the selection, then "selected"; the word taken
    /// out, then "Unselected" [obs, 529/s12-s14].
    SelectWord,
    /// Other Shift+ moves: "selected", then the text added (305/s4); "Unselected", then the
    /// text taken out [obs, 529/s3, s7].
    SelectMore,
    /// Backspace: the character it deleted (305/s6).
    DeleteBackward,
    /// Ctrl+Backspace: the word it deleted (JAWS 2025, Windows 11 Notepad, dev box: "test").
    DeleteWordBackward,
    /// Delete: the character now at the caret (305/s8).
    DeleteForward,
    /// Insert+Up (Say Line): the field's name and role, then the line (305/s9, s15).
    SayLine,
    /// Ctrl+A: "selected", then the whole selection. User report: JAWS reads everything
    /// selected in Notepad. In a WinForms edit JAWS read only the first line (313/s7),
    /// apparently a fallback for that control.
    SelectAll,
    /// Word's Shift+F3: the selection as the key leaves it, its case changed ("LIBRARY HOURS")
    /// [obs, 1078/s3-s5].
    ChangeCase,
    /// Ctrl+C: "Copied selection to clipboard" when text is selected (313/s8).
    Copy,
    /// Ctrl+X: "cut selection to clipboard", in JAWS's lower case, when text was selected
    /// [obs, 504/s19].
    Cut,
    /// Ctrl+V: "Pasted from clipboard" [obs, 504/s6].
    Paste,
    /// Ctrl+Z: "Undo" [obs, 504/s7, s20].
    Undo,
    /// Ctrl+Home in a multi-line document: "top of file", then the line (JAWS 2025, Windows
    /// 11 Notepad, dev box: "top of file | First line of the viewer test").
    TopOfFile,
    /// Ctrl+End in a multi-line document: "Bottom of file", then the line when it has text
    /// (dev box, onto an empty last line: "Bottom of file"; the line after it is assumed).
    BottomOfFile,
    /// Tab, or JAWS's table keys (Ctrl+Alt+arrows, Home, End), in a table of Word's: the cell
    /// moved to, said for the way it was reached [obs, 1034/s10-s12; 518/s15; 1035].
    TableCell(word::CellMove),
    /// Tab or Shift+Tab among a document's form fields (a form of Word's protected for filling
    /// in): the field moved to, "Member: check box not checked" [obs, 1056/s2-s4].
    FormField,
    /// Space on a check box form field: its new state, "checked" [obs, 1057/s2, s5].
    FormFieldState,
}

impl EditKey {
    /// The key as a multi-line document reads it, where that differs from a single-line
    /// field (JAWS, Windows 11 Notepad, dev box): Home reads the character it lands on ("S";
    /// a single-line field is silent, 305/s1), and Ctrl+Home/Ctrl+End name the file's ends.
    pub fn in_document(self, key: Key, ctrl: bool) -> Self {
        match (self, key, ctrl) {
            (Self::Silent, Key::Home, false) => Self::Character,
            (Self::Line, Key::Home, true) => Self::TopOfFile,
            (Self::Line, Key::End, true) => Self::BottomOfFile,
            (other, _, _) => other,
        }
    }
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
            (true, true) if horizontal => Self::SelectWord,
            (true, _) if horizontal || vertical || ends => Self::SelectMore,
            (false, false) if horizontal => Self::Character,
            (false, true) if horizontal => Self::Word,
            (false, _) if vertical => Self::Line,
            (false, true) if ends => Self::Line,
            (false, false) if ends => Self::Silent,
            (false, false) if event.key == Key::Backspace => Self::DeleteBackward,
            (false, true) if event.key == Key::Backspace => Self::DeleteWordBackward,
            (false, false) if event.key == Key::Delete => Self::DeleteForward,
            (false, true) if event.key == Key::Alpha('A') => Self::SelectAll,
            (false, true) if event.key == Key::Alpha('C') => Self::Copy,
            (false, true) if event.key == Key::Alpha('X') => Self::Cut,
            (false, true) if event.key == Key::Alpha('V') => Self::Paste,
            (false, true) if event.key == Key::Alpha('Z') => Self::Undo,
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
pub(crate) fn char_at(context: &TextContext) -> Option<char> {
    context.text.chars().nth(context.caret)
}

/// The line containing the caret, without its line break: the field's own line where the
/// platform read it (Word's lines, wrapped or ended by a table cell [obs, 518]), else the text
/// between line breaks.
pub fn line_at(context: &TextContext) -> String {
    if let Some((start, end)) = context
        .structure
        .as_ref()
        .map(|s| s.line)
        .filter(|&(start, _)| start <= context.text.chars().count())
    {
        let line: String = context
            .text
            .chars()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect();
        return line.trim_matches(char::is_control).to_owned();
    }
    let (start, _, end) = line_bytes(context);
    context.text.get(start..end).unwrap_or_default().to_owned()
}

/// Where the caret's line starts, where the caret is and where the line ends, as byte offsets
/// into the text, without the line's break; the caret past the text is at its end. Found
/// within the line: each edit key made the field's whole text a vector of characters two to
/// four times for these (performance review 2026-10-23, C4).
fn line_bytes(context: &TextContext) -> (usize, usize, usize) {
    let text = &context.text;
    let caret = text
        .char_indices()
        .nth(context.caret)
        .map_or(text.len(), |(at, _)| at);
    let start = text
        .get(..caret)
        .and_then(|before| {
            before
                .char_indices()
                .rev()
                .find(|&(_, c)| is_line_break(c))
                .map(|(at, c)| at + c.len_utf8())
        })
        .unwrap_or(0);
    let end = text
        .get(caret..)
        .and_then(|after| after.find(is_line_break))
        .map_or(text.len(), |at| caret + at);
    (start, caret, end)
}

/// Where the caret's line starts and ends, in characters, without its line break.
fn line_range(context: &TextContext) -> (usize, usize) {
    let (start, caret, end) = line_bytes(context);
    let text = &context.text;
    let count = |from: usize, to: usize| text.get(from..to).map_or(0, |s| s.chars().count());
    let caret_chars = if caret < text.len() {
        context.caret
    } else {
        text.chars().count()
    };
    (
        caret_chars - count(start, caret),
        caret_chars + count(caret, end),
    )
}

/// JAWS's mark before a field's line that holds a word the field marks misspelled, "
/// ‹Misspelled › ada" [obs, 284/s9, s12]; `None` without one.
pub fn misspelled_mark(context: &TextContext) -> Option<String> {
    let (start, end) = line_range(context);
    context
        .misspelled
        .iter()
        .any(|&(s, e)| s < end.max(start + 1) && e > start)
        .then(|| format!("\u{1d}{} \u{1e} ", msg::misspelled()))
}

/// JAWS's mark before a word the field marks misspelled, the caret at it: "‹Misspelled › teh"
/// [obs, 567/s7].
pub fn misspelled_word_mark(context: &TextContext) -> Option<String> {
    context
        .misspelled
        .iter()
        .any(|&(s, e)| s <= context.caret && context.caret < e)
        .then(|| format!("\u{1d}{} \u{1e} ", msg::misspelled()))
}

/// The first line of the caret's paragraph, the lines between blank lines: JAWS's paragraph
/// keys in a text field read it [obs, 510/s4-s6, 504/s15].
pub fn paragraph_first_line(context: &TextContext) -> String {
    let lines = text_lines(&context.text);
    let before = chars(&context.text);
    let before = before
        .get(..context.caret.min(before.len()))
        .unwrap_or_default();
    let mut index = 0;
    let mut previous = None;
    for &c in before {
        if is_line_break(c) && !(c == '\n' && previous == Some('\r')) {
            index += 1;
        }
        previous = Some(c);
    }
    let blank = |i: usize| lines.get(i).is_none_or(|line| line.trim().is_empty());
    while index > 0 && !blank(index - 1) && !blank(index) {
        index -= 1;
    }
    lines.get(index).cloned().unwrap_or_default()
}

/// The caret's line before the caret, and from the caret to its end: Say To Cursor and Say From
/// Cursor [obs, 504/s9, s10].
pub fn line_around_caret(context: &TextContext) -> (String, String) {
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
    let part = |from: usize, to: usize| text.get(from..to).unwrap_or_default().iter().collect();
    (part(start, caret), part(caret, end))
}

/// A text's lines, a Windows line break counting once, and the index of the line the caret is
/// on: what the Okur cursor moves over in a text field.
pub fn lines_and_caret_line(context: &TextContext) -> (Vec<String>, usize) {
    let text = chars(&context.text);
    let mut lines = Vec::new();
    let mut caret_line = 0;
    let mut start = 0;
    let mut i = 0;
    while i <= text.len() {
        let at_break = text.get(i).is_some_and(|c| is_line_break(*c));
        if at_break || i == text.len() {
            if context.caret >= start {
                caret_line = lines.len();
            }
            lines.push(text.get(start..i).unwrap_or_default().iter().collect());
            i += 1;
            if at_break && text.get(i - 1) == Some(&'\r') && text.get(i) == Some(&'\n') {
                i += 1;
            }
            start = i;
            continue;
        }
        i += 1;
    }
    (lines, caret_line)
}

/// The character offset each line of `context`'s text starts at, the lines as
/// [`lines_and_caret_line`] splits them.
pub fn line_starts(context: &TextContext) -> Vec<usize> {
    let text = chars(&context.text);
    let mut starts = vec![0];
    let mut i = 0;
    while i < text.len() {
        if text.get(i).is_some_and(|c| is_line_break(*c)) {
            i += 1;
            if text.get(i - 1) == Some(&'\r') && text.get(i) == Some(&'\n') {
                i += 1;
            }
            starts.push(i);
            continue;
        }
        i += 1;
    }
    starts
}

/// The sentences from the one the caret is in to the end of the text, each with the
/// character offset it starts at: what Say All reads in a text field by sentence (Say All
/// Reads). A sentence ends at a line's end too; a line without a sentence mark is one.
pub fn sentences_from_caret(context: &TextContext) -> Vec<(usize, String)> {
    let text = chars(&context.text);
    let text = &text;
    let pieces: Vec<(usize, String)> = lines_from_caret(context)
        .into_iter()
        .flat_map(|(start, _)| {
            let end = text
                .get(start..)
                .and_then(|rest| rest.iter().position(|c| is_line_break(*c)))
                .map_or(text.len(), |i| start + i);
            let line: String = text.get(start..end).unwrap_or_default().iter().collect();
            sentences(&line)
                .into_iter()
                .map(|(s, e)| {
                    let offset = start + line.get(..s).map_or(0, |b| b.chars().count());
                    (offset, line.get(s..e).unwrap_or_default().to_owned())
                })
                .collect::<Vec<_>>()
        })
        .collect();
    // From the sentence the caret is in.
    let first = pieces
        .iter()
        .rposition(|(offset, _)| *offset <= context.caret)
        .unwrap_or(0);
    pieces.into_iter().skip(first).collect()
}

/// The lines from the caret's line to the end of the text, without empty ones, each with the
/// character offset it starts at: what Say All reads in a document (JAWS in Google Docs'
/// document, dev box 2026-10-16: each line from the caret's, to the last), the caret going
/// to each line's start as it is read.
pub fn lines_from_caret(context: &TextContext) -> Vec<(usize, String)> {
    let text = chars(&context.text);
    let caret = context.caret.min(text.len());
    let mut start = text
        .get(..caret)
        .and_then(|before| before.iter().rposition(|c| is_line_break(*c)))
        .map_or(0, |i| i + 1);
    let mut lines = Vec::new();
    loop {
        let end = text
            .get(start..)
            .and_then(|rest| rest.iter().position(|c| is_line_break(*c)))
            .map_or(text.len(), |i| start + i);
        let line: String = text.get(start..end).unwrap_or_default().iter().collect();
        if !line.trim().is_empty() {
            lines.push((start, line.trim().to_owned()));
        }
        if end >= text.len() {
            return lines;
        }
        // Past the line break, `\r\n` counting once.
        start = end + 1;
        if text.get(end) == Some(&'\r') && text.get(start) == Some(&'\n') {
            start += 1;
        }
    }
}

/// A page field's word around `caret` (in characters), as Chrome moves by words and JAWS
/// reads them (dev box, 2026-10-16): letters, digits and `_` make a word, and so does a run
/// of punctuation ("dog", ","; "std", "::", "collections"); an apostrophe between letters
/// and a point or comma between digits stay inside ("can't", "3.5"). `None` on a space.
pub fn web_word_at(text: &str, caret: usize) -> Option<(usize, usize)> {
    let text = chars(text);
    let at = |i: usize| text.get(i).copied();
    let joins = |i: usize| {
        let (before, after) = (i.checked_sub(1).and_then(at), at(i + 1));
        match at(i) {
            Some('\'' | '\u{2019}') => {
                before.is_some_and(char::is_alphabetic) && after.is_some_and(char::is_alphabetic)
            }
            Some('.' | ',') => {
                before.is_some_and(|c| c.is_ascii_digit())
                    && after.is_some_and(|c| c.is_ascii_digit())
            }
            _ => false,
        }
    };
    // Some(true) for a word character, Some(false) for punctuation, None for a space.
    let class = |i: usize| -> Option<bool> {
        let c = at(i)?;
        if c.is_whitespace() {
            None
        } else {
            Some(c.is_alphanumeric() || c == '_' || joins(i))
        }
    };
    let kind = class(caret)?;
    let start = (0..caret)
        .rev()
        .find(|&i| class(i) != Some(kind))
        .map_or(0, |i| i + 1);
    let end = (caret..text.len())
        .find(|&i| class(i) != Some(kind))
        .unwrap_or(text.len());
    Some((start, end))
}

/// The field's own word around the caret, when it reports one that holds the caret.
fn field_word(context: &TextContext) -> Option<(usize, usize)> {
    context
        .word
        .filter(|&(start, end)| start <= context.caret && context.caret < end)
}

/// The word at the caret (where Ctrl+Left/Right leave it): to the end of the field's word; in a
/// plain Win32 edit (`plain_edit`), or a field that reports no words, the whole run around the
/// caret between spaces and hyphens.
fn word_at(context: &TextContext, plain_edit: bool) -> String {
    let text = chars(&context.text);
    if let Some((_, end)) = field_word(context).filter(|_| !plain_edit) {
        let word: String = text
            .get(context.caret..end)
            .unwrap_or_default()
            .iter()
            .collect();
        return word.trim().to_owned();
    }
    // JAWS says the whole run around the caret, between spaces and hyphens, wherever in it a
    // plain edit stopped: "C:\Users\Ada\Downloads" at each of classic Notepad's three stops in
    // it, "https://example.org/a", then "b" [obs, 1037/s5-s11].
    // At the end of the text, or on a space or line break, nothing [obs, 1037/s14-s28].
    let caret = context.caret.min(text.len());
    let apart = |c: &char| c.is_whitespace() || *c == '-';
    if text.get(caret).is_none_or(apart) {
        return String::new();
    }
    let start = text
        .get(..caret)
        .and_then(|before| before.iter().rposition(apart))
        .map_or(0, |i| i + 1);
    text.get(start..)
        .unwrap_or_default()
        .iter()
        .take_while(|c| !apart(c))
        .collect()
}

/// The word around the caret, whole (Say Word reads it wherever in the word the caret is): in
/// a field that reports words (UI Automation's, a page's), as JAWS splits words, letters and
/// digits apart from punctuation ("println" in "println!(" [obs, 756/s9]), else the field's
/// word; in one that reports none (a WinForms edit, read by window messages) the run between
/// spaces, "Hello," [obs, 974/s22]; nothing on a space [obs, 974/s15].
pub fn word_around(context: &TextContext) -> String {
    let text = chars(&context.text);
    if text.get(context.caret).is_some_and(|c| c.is_whitespace()) {
        return String::new();
    }
    if context.word.is_some() {
        if let Some((start, end)) = web_word_at(&context.text, context.caret) {
            return text.get(start..end).unwrap_or_default().iter().collect();
        }
    }
    if let Some((start, end)) = field_word(context) {
        let word: String = text.get(start..end).unwrap_or_default().iter().collect();
        return word.trim().to_owned();
    }
    let caret = context.caret.min(text.len());
    let start = text
        .get(..caret)
        .and_then(|before| before.iter().rposition(|c| c.is_whitespace()))
        .map_or(0, |i| i + 1);
    let end = text
        .get(caret..)
        .and_then(|after| after.iter().position(|c| c.is_whitespace()))
        .map_or(text.len(), |i| caret + i);
    text.get(start..end).unwrap_or_default().iter().collect()
}

pub(crate) fn selected_text(context: &TextContext) -> String {
    let text = chars(&context.text);
    match context.selection {
        Some((start, end)) if start < end => {
            text.get(start..end).unwrap_or_default().iter().collect()
        }
        _ => String::new(),
    }
}

/// What a selection key added to the selection and what it took out of it, from the field
/// before and after (one end moves). A selection that turned over is all added.
fn selection_change(before: Option<&TextContext>, after: &TextContext) -> (String, String) {
    let range = |c: &TextContext| c.selection.filter(|(start, end)| start < end);
    let part = |c: &TextContext, start: usize, end: usize| -> String {
        chars(&c.text)
            .get(start..end)
            .unwrap_or_default()
            .iter()
            .collect()
    };
    let none = String::new;
    match (before.and_then(range), range(after)) {
        (_, Some((start, end))) if before.is_none() => (part(after, start, end), none()),
        (None, Some((start, end))) => (part(after, start, end), none()),
        (Some(was), Some(now)) if was == now => (none(), none()),
        (Some((start, end)), None) => (none(), before.map_or_else(none, |b| part(b, start, end))),
        (Some((ws, we)), Some((ns, ne))) if ns == ws && ne > we => (part(after, we, ne), none()),
        (Some((ws, we)), Some((ns, ne))) if ns == ws && ne < we => {
            (none(), before.map_or_else(none, |b| part(b, ne, we)))
        }
        (Some((ws, we)), Some((ns, ne))) if ne == we && ns < ws => (part(after, ns, ws), none()),
        (Some((ws, we)), Some((ns, ne))) if ne == we && ns > ws => {
            (none(), before.map_or_else(none, |b| part(b, ws, ns)))
        }
        (Some(_), Some((start, end))) => (part(after, start, end), none()),
        (_, None) => (none(), none()),
    }
}

/// `text`'s lines that are not blank, or only the first in a plain Win32 edit, where JAWS
/// reads no further [obs, 313/s7, 529/s6, s9, s10]; elsewhere it reads them all (Windows 11
/// Notepad, the owner's report and the dev box).
pub(crate) fn selection_lines(text: &str, plain_edit: bool) -> Vec<String> {
    let mut lines: Vec<String> = text_lines(text)
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .collect();
    if plain_edit {
        lines.truncate(1);
    }
    lines
}

/// The text Backspace removed: what `before` has and `after` lacks, ending at the caret.
fn deleted_before_caret(before: Option<&TextContext>, after: &TextContext) -> String {
    let Some(before) = before else {
        return String::new();
    };
    let was = chars(&before.text);
    // What the text lost, or how far the caret came back where a console's rows, padded to
    // its width, keep the text's length (Windows Terminal: Backspace in "echo abc" deleted
    // "b"; dev box, 2026-10-17).
    let removed = was
        .len()
        .saturating_sub(chars(&after.text).len())
        .max(before.caret.saturating_sub(after.caret));
    let end = before.caret.min(was.len());
    was.get(end.saturating_sub(removed)..end)
        .unwrap_or_default()
        .iter()
        .collect()
}

/// `text` trimmed, or a line break when nothing is left. JAWS's reading keys keep nothing of
/// an empty line or word in its history, and its synthesizer is given "blank"
/// ([`crate::pronounce::character`]) [obs, 315/s2, s7; 320/s7, s11]. "blank" goes into the
/// history only for an empty field's value (202-204).
fn or_line_break(text: String) -> String {
    let text = text.trim();
    if text.is_empty() {
        "\n".to_owned()
    } else {
        text.to_owned()
    }
}

/// How the verbosity level words what an edit key did: whether "selected" and "unselected"
/// are said (JAWS's Select output), whether JAWS's own messages are ("Copied selection to
/// clipboard", JAWS Message), and whether they take their short forms ("Copied").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wording {
    pub selection: bool,
    pub messages: bool,
    pub short: bool,
}

impl Default for Wording {
    /// JAWS's Beginner level: everything said, in its long form.
    fn default() -> Self {
        Self {
            selection: true,
            messages: true,
            short: false,
        }
    }
}

impl Wording {
    pub fn of(verbosity: &crate::settings::Verbosity) -> Self {
        use crate::settings::{MessageLength, OutputType};
        Self {
            selection: verbosity.is_on(OutputType::Select),
            messages: verbosity.is_on(OutputType::OkurMessage),
            short: verbosity.message_length() == MessageLength::Short,
        }
    }

    /// `text` with "selected" or "unselected" (`word`) on a line of its own before or after
    /// it, where the level says it.
    fn selection(self, word: impl std::fmt::Display, text: &str, word_first: bool) -> String {
        if !self.selection {
            text.to_owned()
        } else if word_first {
            format!("{word}\n{text}")
        } else {
            format!("{text}\n{word}")
        }
    }

    /// JAWS's message `long`, or its `short` form, or nothing where the level says none.
    fn message(self, long: String, short: String) -> Option<String> {
        if !self.messages {
            None
        } else if self.short {
            Some(short)
        } else {
            Some(long)
        }
    }
}

/// A document's top or bottom: its message where the level says one, then the line there.
fn edge_of_file(message: Option<String>, line: &str) -> Option<String> {
    let line = line.trim();
    match (message, line.is_empty()) {
        (Some(message), true) => Some(message),
        (Some(message), false) => Some(format!("{message}\n{line}")),
        (None, false) => Some(line.to_owned()),
        (None, true) => None,
    }
}

/// What JAWS speaks after `key`, given the field before and after it. `field` is the
/// field's name-and-role line, for Say Line. In a field that is not a plain Win32 edit.
pub fn speech(
    key: EditKey,
    before: Option<&TextContext>,
    after: &TextContext,
    field: &str,
) -> Option<String> {
    speech_in(key, before, after, field, false, Wording::default())
}

/// [`speech`] in a field that may be a plain Win32 edit (`plain_edit`), where JAWS reads a
/// selection's first line only, worded for the verbosity level ([`Wording`]).
pub fn speech_in(
    key: EditKey,
    before: Option<&TextContext>,
    after: &TextContext,
    field: &str,
    plain_edit: bool,
    wording: Wording,
) -> Option<String> {
    // A form field of Word's at the caret is said as a control, "Name: Edit", whatever key
    // brought the caret there [obs, 1055/s4, s7; 1056].
    if let Some(field) = word::form_field_at(after).filter(|_| {
        matches!(
            key,
            EditKey::Character
                | EditKey::Word
                | EditKey::Line
                | EditKey::SayLine
                | EditKey::FormField
                // End, silent on a line, says the field it lands on [obs, 1055/s4].
                | EditKey::Silent
                // Ctrl+Home onto a field: the field alone, as one trial of JAWS's says it
                // (the others three times over) [obs, 1057/s11].
                | EditKey::TopOfFile
                | EditKey::BottomOfFile
        )
    }) {
        return word::around_line(
            key,
            before,
            after,
            word::form_field_speech(after, field, key == EditKey::FormField),
        );
    }
    let said = match key {
        EditKey::Silent => return None,
        EditKey::TableCell(how) => return word::cell_speech(before, after, how),
        EditKey::FormField => return None,
        EditKey::FormFieldState => return word::form_field_state(after),
        EditKey::Character | EditKey::DeleteForward => {
            char_at(after).map_or_else(|| msg::blank().to_string(), |c| c.to_string())
        }
        // A word move that stops at a line's end reads the line break, as a character move onto
        // it does [obs, 320/s7, s11].
        EditKey::Word => format!(
            "{}{}",
            misspelled_word_mark(after).unwrap_or_default(),
            or_line_break(word_at(after, plain_edit))
        ),
        EditKey::Line => format!(
            "{}{}",
            misspelled_mark(after).unwrap_or_default(),
            or_line_break(word::spoken_line(after, line_at(after)))
        ),
        EditKey::SelectCharacter | EditKey::SelectWord => {
            // The text, then the word: "Hello, selected", "world. Unselected" [obs, 529/s12,
            // s14]; a character taken out is read as a word is.
            match selection_change(before, after) {
                (added, _) if !added.is_empty() => {
                    wording.selection(msg::selection_added(), &added, false)
                }
                (_, removed) if !removed.is_empty() => {
                    wording.selection(msg::selection_removed_lines(), &removed, false)
                }
                _ => return None,
            }
        }
        EditKey::SelectMore => {
            // Text over several lines is read line by line (Ctrl+Shift+Home in Windows 11
            // Notepad, JAWS on the dev box: "selected | First line … | Second line …"), but for
            // a plain edit's first line.
            let (word, text) = match selection_change(before, after) {
                (added, _) if !added.is_empty() => (msg::selection_added(), added),
                (_, removed) if !removed.is_empty() => (msg::selection_removed_lines(), removed),
                _ => return None,
            };
            let lines = selection_lines(&text, plain_edit);
            let text = if lines.is_empty() {
                text
            } else {
                lines.join("\n")
            };
            wording.selection(word, &text, true)
        }
        // "top of file", "top" at Intermediate, nothing at Advanced, then the line [obs, 660/s4,
        // 668/s4, 669/s4].
        EditKey::TopOfFile => edge_of_file(
            wording.message(
                msg::text_top_of_file().to_string(),
                msg::text_top_of_file_short().to_string(),
            ),
            &word::spoken_line(after, line_at(after)),
        )?,
        EditKey::BottomOfFile => edge_of_file(
            wording.message(
                msg::text_bottom_of_file().to_string(),
                msg::text_bottom_of_file_short().to_string(),
            ),
            &word::spoken_line(after, line_at(after)),
        )?,
        EditKey::DeleteBackward => {
            let deleted = deleted_before_caret(before, after);
            if deleted.is_empty() {
                return None;
            }
            deleted
        }
        EditKey::DeleteWordBackward => {
            let deleted = deleted_before_caret(before, after);
            if deleted.trim().is_empty() {
                return None;
            }
            deleted.trim().to_owned()
        }
        EditKey::SelectAll => {
            let lines = selection_lines(&selected_text(after), plain_edit);
            if lines.is_empty() {
                return None;
            }
            wording.selection(msg::selection_added(), &lines.join("\n"), true)
        }
        EditKey::ChangeCase => {
            let lines = selection_lines(&selected_text(after), plain_edit);
            if lines.is_empty() {
                return None;
            }
            lines.join("\n")
        }
        // JAWS's messages: short above Beginner ("Copied", "cut", "Pasted" [obs, 599/s3, s19,
        // s6]), none at Advanced [obs, 609/s3, s6, s7, s19, s20].
        EditKey::Copy => {
            if selected_text(after).is_empty() {
                return None;
            }
            wording.message(
                msg::clipboard_copied_selection().to_string(),
                msg::clipboard_copied_selection_short().to_string(),
            )?
        }
        EditKey::Cut => {
            if before.is_none_or(|b| selected_text(b).is_empty()) {
                return None;
            }
            wording.message(
                msg::clipboard_cut_selection().to_string(),
                msg::clipboard_cut_selection_short().to_string(),
            )?
        }
        EditKey::Paste => wording.message(
            msg::clipboard_pasted().to_string(),
            msg::clipboard_pasted_short().to_string(),
        )?,
        EditKey::Undo => {
            wording.message(msg::edit_undo().to_string(), msg::edit_undo().to_string())?
        }
        EditKey::SayLine => {
            // A heading of Word's with its level [obs, 518/s1].
            let line = word::spoken_line(after, line_at(after).trim().to_owned());
            if line.trim().is_empty() {
                field.to_owned()
            } else {
                // After the mark of a misspelled word in it [obs, 567/s3].
                format!(
                    "{field}\n{}{}",
                    misspelled_mark(after).unwrap_or_default(),
                    line.trim()
                )
            }
        }
    };
    word::around_line(key, before, after, said)
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
            word: None,
            misspelled: Vec::new(),
            structure: None,
        }
    }

    /// Say Word: a field that reports no words (a WinForms edit) reads the run between spaces,
    /// "Hello,", one that does splits off punctuation, and a caret on a space reads nothing
    /// [obs, 974/s15, s22; 756/s9].
    #[test]
    fn say_word_follows_the_fields_words() {
        let mut field = ctx("Hello, world.", 1, None);
        assert_eq!(word_around(&field), "Hello,");
        field.word = Some((0, 5));
        assert_eq!(word_around(&field), "Hello");
        field.caret = 6;
        assert_eq!(word_around(&field), "");
    }

    /// The caret's line, its range and its character, found within the line, are what the
    /// whole text as characters gave: every caret of texts with `\r\n`, bare `\r` and `\n`
    /// breaks, empty lines, letters of several bytes, and a caret past the end.
    #[test]
    fn the_carets_line_is_found_within_it() {
        let oracle = |context: &TextContext| {
            let text: Vec<char> = context.text.chars().collect();
            let caret = context.caret.min(text.len());
            let start = text
                .get(..caret)
                .and_then(|before| before.iter().rposition(|c| is_line_break(*c)))
                .map_or(0, |i| i + 1);
            let end = text
                .get(caret..)
                .and_then(|after| after.iter().position(|c| is_line_break(*c)))
                .map_or(text.len(), |i| caret + i);
            let line: String = text.get(start..end).unwrap_or_default().iter().collect();
            ((start, end), line, text.get(context.caret).copied())
        };
        for text in [
            "",
            "one line",
            "first\r\nsecond\r\n\r\nfourth",
            "Türkçe ğüş\rikinci\r",
            "a\nb\u{2028}c 𝄞 d\n",
        ] {
            for caret in 0..text.chars().count() + 3 {
                let context = ctx(text, caret, None);
                let ((start, end), line, at) = oracle(&context);
                assert_eq!(line_range(&context), (start, end), "{text:?} {caret}");
                assert_eq!(line_at(&context), line, "{text:?} {caret}");
                assert_eq!(char_at(&context), at, "{text:?} {caret}");
            }
        }
    }

    /// Ctrl+Home says "top of file" and the line, "top" at Intermediate, the line alone at
    /// Advanced [obs, 660/s4, 668/s4, 669/s4].
    #[test]
    fn the_top_of_a_file_is_said_by_the_level() {
        let top = ctx("first\nsecond", 0, None);
        let level = |selection: bool, messages: bool, short: bool| Wording {
            selection,
            messages,
            short,
        };
        let said = |wording| speech_in(EditKey::TopOfFile, None, &top, "", true, wording);
        assert_eq!(
            said(Wording::default()).as_deref(),
            Some("top of file\nfirst")
        );
        assert_eq!(said(level(true, true, true)).as_deref(), Some("top\nfirst"));
        assert_eq!(said(level(false, false, true)).as_deref(), Some("first"));
    }

    /// A line holding a word the field marks misspelled is read after JAWS's mark, "‹Misspelled
    /// › ada"; the line before it, without [obs, 284/s9].
    #[test]
    fn a_misspelled_line_is_marked() {
        let mut field = ctx("Dear ada\nyours", 2, None);
        field.misspelled = vec![(5, 8)];
        assert_eq!(
            speech(EditKey::Line, None, &field, "").as_deref(),
            Some("\u{1d}Misspelled \u{1e} Dear ada")
        );
        field.caret = 10;
        assert_eq!(
            speech(EditKey::Line, None, &field, "").as_deref(),
            Some("yours")
        );
    }

    /// A selection that shrinks says what left it: "Unselected" and the line, the word and
    /// "Unselected"; a plain Win32 edit reads a selection's first line only [obs, 529/s3,
    /// s9, s14].
    #[test]
    fn a_shrinking_selection_says_what_was_unselected() {
        let text = "one two\nthree";
        let lines = speech(
            EditKey::SelectMore,
            Some(&ctx(text, 13, Some((0, 13)))),
            &ctx(text, 8, Some((0, 8))),
            "",
        );
        assert_eq!(lines.as_deref(), Some("Unselected\nthree"));
        let word = speech(
            EditKey::SelectWord,
            Some(&ctx(text, 7, Some((0, 7)))),
            &ctx(text, 4, Some((0, 4))),
            "",
        );
        assert_eq!(word.as_deref(), Some("two\nUnselected"));
        let unchanged = ctx(text, 4, Some((0, 4)));
        assert_eq!(
            speech(EditKey::SelectMore, Some(&unchanged), &unchanged, ""),
            None
        );
        let all = ctx(text, 13, Some((0, 13)));
        assert_eq!(
            speech_in(EditKey::SelectAll, None, &all, "", true, Wording::default()).as_deref(),
            Some("selected\none two")
        );
    }

    /// A paragraph is read from its first line, wherever in it the caret is; a blank line
    /// starts the next [obs, 504/s15].
    #[test]
    fn paragraphs_read_their_first_line() {
        let text = "one\r\ntwo\r\n\r\nthree\r\nfour";
        assert_eq!(paragraph_first_line(&ctx(text, 6, None)), "one");
        assert_eq!(paragraph_first_line(&ctx(text, 0, None)), "one");
        assert_eq!(paragraph_first_line(&ctx(text, 18, None)), "three");
        assert_eq!(paragraph_first_line(&ctx("a\rb", 2, None)), "a");
    }

    /// Ctrl+Backspace reads the word it deleted, without its space (JAWS on the dev box).
    #[test]
    fn ctrl_backspace_reads_the_deleted_word() {
        let before = ctx("of the viewer test ", 19, None);
        let after = ctx("of the viewer ", 14, None);
        assert_eq!(
            speech(EditKey::DeleteWordBackward, Some(&before), &after, "").as_deref(),
            Some("test")
        );
    }

    /// A word move says the word it reaches, and at a line's end the line break, which the
    /// synthesizer says as "blank" [obs, 320/s11-s12].
    #[test]
    fn a_word_move_to_a_line_end_reads_the_line_break() {
        let text = "fn main() {\r\n    let total";
        let at_end = speech(EditKey::Word, None, &ctx(text, 11, None), "");
        assert_eq!(at_end.as_deref(), Some("\n"));
        assert_eq!(crate::pronounce::character("\n"), "blank");
        assert_eq!(
            speech(EditKey::Word, None, &ctx(text, 17, None), "").as_deref(),
            Some("let")
        );
    }

    /// Say All in a document: from the caret's line, whatever the caret's column, to the last
    /// line, without empty lines.
    #[test]
    fn say_all_reads_from_the_carets_line() {
        let text = "First line\nSecond line\n\nLast line\n";
        assert_eq!(
            lines_from_caret(&ctx(text, 14, None)),
            vec![(11, "Second line".to_owned()), (24, "Last line".to_owned())]
        );
        // A Windows line break is one break.
        assert_eq!(
            lines_from_caret(&ctx("one\r\n\r\n  two", 0, None)),
            vec![(0, "one".to_owned()), (7, "two".to_owned())]
        );
        assert_eq!(lines_from_caret(&ctx(text, 0, None)).len(), 3);
        assert!(lines_from_caret(&ctx("", 0, None)).is_empty());
    }

    /// By sentence, Say All reads from the caret's sentence, each with its offset; a line
    /// without a sentence mark is one.
    #[test]
    fn say_all_by_sentence_reads_from_the_carets_sentence() {
        let text = "Intro\nSay All reads this. It has three sentences. The last one ends here.\n";
        assert_eq!(
            sentences_from_caret(&ctx(text, 0, None)),
            vec![
                (0, "Intro".to_owned()),
                (6, "Say All reads this.".to_owned()),
                (26, "It has three sentences.".to_owned()),
                (50, "The last one ends here.".to_owned()),
            ]
        );
        // The caret in the second sentence: from its start.
        assert_eq!(
            sentences_from_caret(&ctx(text, 30, None)).first(),
            Some(&(26, "It has three sentences.".to_owned()))
        );
    }

    /// Windows 11 Notepad (JAWS on the dev box): Home reads its character, Ctrl+Home and
    /// Ctrl+End name the file's ends.
    #[test]
    fn a_document_names_its_ends_and_reads_home() {
        let text = "First line\rSecond\r";
        assert_eq!(
            EditKey::Silent.in_document(Key::Home, false),
            EditKey::Character
        );
        assert_eq!(
            EditKey::Silent.in_document(Key::End, false),
            EditKey::Silent
        );
        let top = EditKey::Line.in_document(Key::Home, true);
        assert_eq!(
            speech(top, None, &ctx(text, 0, None), "").as_deref(),
            Some("top of file\nFirst line")
        );
        let bottom = EditKey::Line.in_document(Key::End, true);
        assert_eq!(
            speech(bottom, None, &ctx(text, 18, None), "").as_deref(),
            Some("Bottom of file")
        );
    }

    /// Windows 11 Notepad's words (UIA): Ctrl+Right stops at "line", then at "," alone
    /// (JAWS on the dev box: "line", ",", "with").
    #[test]
    fn a_fields_own_word_unit_decides_words() {
        let text = "Second line, with";
        let at = |caret: usize, word: (usize, usize)| TextContext {
            word: Some(word),
            misspelled: Vec::new(),
            ..ctx(text, caret, None)
        };
        assert_eq!(word_at(&at(7, (7, 11)), false), "line");
        assert_eq!(word_at(&at(11, (11, 13)), false), ",");
        assert_eq!(word_around(&at(9, (7, 11))), "line");
        // Without a word unit, a word runs to the next space.
        assert_eq!(word_at(&ctx(text, 7, None), false), "line,");
    }

    /// A plain Win32 edit's word is the whole run around the caret between spaces and hyphens,
    /// wherever in it the edit stopped; nothing at the text's end [obs, 1037/s5-s14].
    #[test]
    fn a_plain_edits_word_is_the_run_around_the_caret() {
        let text = "C:\\Users\\Ada\\Downloads or https://example.org/a-b";
        let at = |caret: usize| TextContext {
            word: Some((caret, caret + 1)),
            misspelled: Vec::new(),
            ..ctx(text, caret, None)
        };
        assert_eq!(word_at(&at(1), true), "C:\\Users\\Ada\\Downloads");
        assert_eq!(word_at(&at(9), true), "C:\\Users\\Ada\\Downloads");
        assert_eq!(word_at(&at(26), true), "https://example.org/a");
        assert_eq!(word_at(&at(48), true), "b");
        assert_eq!(word_at(&ctx(text, 49, None), true), "");
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

    /// Ctrl+Right in a page's text area and in vscode.dev's editor, JAWS on the dev box:
    /// "lazy", "dog", ",", "then"; "use", "std", "::", "collections", "::", "HashMap", ";".
    #[test]
    fn a_page_fields_words_stop_at_punctuation() {
        let word = |text: &str, caret: usize| {
            let at = |c: usize| TextContext {
                word: web_word_at(text, c),
                misspelled: Vec::new(),
                ..ctx(text, c, None)
            };
            word_at(&at(caret), false)
        };
        let prose = "the lazy dog, then";
        assert_eq!(word(prose, 9), "dog");
        assert_eq!(word(prose, 12), ",");
        assert_eq!(word(prose, 14), "then");
        let code = "use std::collections::HashMap;";
        let stops: Vec<String> = [0, 4, 7, 9, 20, 22, 29]
            .iter()
            .map(|&c| word(code, c))
            .collect();
        assert_eq!(
            stops,
            ["use", "std", "::", "collections", "::", "HashMap", ";"]
        );
        assert_eq!(word("    let mut counts: usize", 12), "counts");
        assert_eq!(word("I can't stop", 2), "can't");
        assert_eq!(word("rose 3.5% last", 5), "3.5");
        assert_eq!(word("rose 3.5% last", 8), "%");
        assert_eq!(web_word_at("a b", 1), None);
    }

    #[test]
    fn lines_and_words_in_multiline_text() {
        let text = "first line\r\nsecond line";
        assert_eq!(line_at(&ctx(text, 3, None)), "first line");
        assert_eq!(line_at(&ctx(text, 14, None)), "second line");
        assert_eq!(word_at(&ctx(text, 12, None), false), "second");
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

    /// User report: selecting several lines (Ctrl+A, Ctrl+Shift+End) must speak all of them.
    #[test]
    fn multi_line_selections_are_spoken_whole() {
        let text = "first\rsecond\rthird";
        let all = ctx(text, 0, Some((0, 18)));
        assert_eq!(
            speech(EditKey::SelectAll, None, &all, "").as_deref(),
            Some("selected\nfirst\nsecond\nthird")
        );
        let before = ctx(text, 6, None);
        let to_end = ctx(text, 18, Some((6, 18)));
        assert_eq!(
            speech(EditKey::SelectMore, Some(&before), &to_end, "").as_deref(),
            Some("selected\nsecond\nthird")
        );
    }

    /// A console's rows are padded to its width: Backspace keeps the text's length, and the
    /// caret coming back tells what went (Windows Terminal, dev box, 2026-10-17).
    #[test]
    fn backspace_in_a_padded_console_row_says_the_deleted_character() {
        let before = ctx("> echo abc    ", 9, None);
        let after = ctx("> echo ac     ", 8, None);
        assert_eq!(
            speech(EditKey::DeleteBackward, Some(&before), &after, "").as_deref(),
            Some("b")
        );
    }
}

/// `text`'s sentences, as byte ranges without the spaces between them: a sentence ends at
/// ".", "!" or "?" before a space or the text's end ("Hello, world." | "Is it done?" | "Yes!"
/// | "Wait; maybe: no." [obs, 510/s1-s3]).
pub fn sentences(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start: Option<usize> = None;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if start.is_none() && !c.is_whitespace() {
            start = Some(i);
        }
        let next_is_space = chars.peek().is_none_or(|(_, n)| n.is_whitespace());
        if matches!(c, '.' | '!' | '?') && next_is_space {
            if let Some(s) = start.take() {
                ranges.push((s, i + c.len_utf8()));
            }
        }
    }
    if let Some(s) = start {
        let end = s + text.get(s..).unwrap_or_default().trim_end().len();
        if end > s {
            ranges.push((s, end));
        }
    }
    ranges
}

#[cfg(test)]
mod sentence_tests {
    use super::sentences;

    #[test]
    fn sentences_end_at_their_marks() {
        let text = "Hello, world. Is it done? Yes! Wait; maybe: no.";
        let found: Vec<&str> = sentences(text)
            .into_iter()
            .filter_map(|(s, e)| text.get(s..e))
            .collect();
        assert_eq!(
            found,
            ["Hello, world.", "Is it done?", "Yes!", "Wait; maybe: no."]
        );
        assert_eq!(sentences("  no mark at the end  "), [(2, 20)]);
        assert!(sentences("   ").is_empty());
    }
}
