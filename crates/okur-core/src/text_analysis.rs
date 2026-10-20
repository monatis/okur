//! JAWS's text analysis: the inconsistencies of a line of text, which Say Current Focus
//! (Insert+Tab) names after a text field ("Space run at column 4" [obs, 568/s4]).
//!
//! The rules are the ones recordings show, each with its column (from 1, in characters):
//! - a run of spaces inside the line (568/s4);
//! - a sentence beginning with a small letter, after the full stop, question or exclamation
//!   mark that ends the one before it, on this line or an earlier one ("no capital at the
//!   start." after "Two  spaces here." [obs, 568/s7]); the text's first line has none before
//!   it and is not named ("fn main() {" [obs, 571/s4]);
//! - a word in inverted case, "tHIS" (568/s25);
//! - punctuation alone between spaces or the line's ends, "A stray , comma." (568/s10), "fn
//!   main() {" (571/s4);
//! - a comma, semicolon, colon, question or exclamation mark with a letter right after it,
//!   "An unspaced,comma." (568/s13);
//! - a bracket, brace or double quote the line does not close (568/s22, 571/s4).
//!
//! Open questions, which 614 records: whether parentheses are paired at all (568's "(" closed
//! on the next line is not named), how far pairing reaches, and leading spaces, full stops
//! inside words and symbols such as `*` and `=`.

use crate::messages::msg;

/// One inconsistency, in the order JAWS names them at one column: the start's capital, then
/// the word's case, then punctuation, then pairing ("Stray punctuation { at column 11 |
/// Unclosed { at column 11" [obs, 571/s4]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    SpaceRun,
    MissingInitialCap(char),
    InvertedCap(char),
    StrayPunctuation(char),
    UnspacedPunctuation(char),
    Unclosed(char),
}

/// What Insert+Tab adds for the line around `caret` (a character offset into `text`), one
/// entry per inconsistency, in column order.
pub fn describe_line(text: &str, caret: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let caret = caret.min(chars.len());
    let is_break = |c: &char| *c == '\n' || *c == '\r';
    let start = chars
        .get(..caret)
        .and_then(|before| before.iter().rposition(is_break))
        .map_or(0, |i| i + 1);
    let end = chars
        .get(caret..)
        .and_then(|after| after.iter().position(is_break))
        .map_or(chars.len(), |i| caret + i);
    let line = chars.get(start..end).unwrap_or_default();
    // What ends the text before the line, past its breaks and spaces.
    let before = chars
        .get(..start)
        .and_then(|before| before.iter().rev().find(|c| !c.is_whitespace()))
        .copied();
    let mut found = inconsistencies(line, before);
    found.sort_by_key(|&(column, kind)| (column, kind));
    found
        .into_iter()
        .map(|(column, kind)| describe(column, kind))
        .collect()
}

fn describe(column: usize, kind: Kind) -> String {
    let symbol = |c: char| c.to_string();
    match kind {
        Kind::SpaceRun => msg::text_analysis_space_run(column).to_string(),
        Kind::MissingInitialCap(c) => {
            msg::text_analysis_missing_initial_cap(symbol(c), column).to_string()
        }
        Kind::InvertedCap(c) => msg::text_analysis_inverted_cap(symbol(c), column).to_string(),
        Kind::StrayPunctuation(c) => {
            msg::text_analysis_stray_punctuation(symbol(c), column).to_string()
        }
        Kind::UnspacedPunctuation(c) => {
            msg::text_analysis_unspaced_punctuation(symbol(c), column).to_string()
        }
        Kind::Unclosed(c) => msg::text_analysis_unclosed(symbol(c), column).to_string(),
    }
}

/// The line's inconsistencies as `(column, kind)`, columns from 1. `before` is the last
/// character of the text before the line that is not a space or a line break.
fn inconsistencies(line: &[char], before: Option<char>) -> Vec<(usize, Kind)> {
    let mut found = Vec::new();
    let at = |i: usize| line.get(i).copied();
    let spaced = |c: Option<char>| c.is_none_or(char::is_whitespace);
    let first = line.iter().position(|c| !c.is_whitespace());

    // Runs of spaces inside the line.
    let mut i = first.unwrap_or(line.len());
    while i < line.len() {
        if at(i) == Some(' ') && at(i + 1) == Some(' ') {
            let run_end = (i..line.len())
                .find(|&j| at(j) != Some(' '))
                .unwrap_or(line.len());
            if run_end < line.len() {
                found.push((i + 1, Kind::SpaceRun));
            }
            i = run_end;
        } else {
            i += 1;
        }
    }

    // A sentence's first letter, small, after the one before it ended.
    let mut sentence_ended = before.is_some_and(ends_sentence);
    for (i, &c) in line.iter().enumerate() {
        if c.is_alphabetic() {
            if sentence_ended && c.is_lowercase() {
                found.push((i + 1, Kind::MissingInitialCap(c)));
            }
            sentence_ended = false;
        } else if ends_sentence(c) {
            sentence_ended = at(i + 1).is_none_or(char::is_whitespace);
        } else if !c.is_whitespace() {
            sentence_ended = false;
        }
    }

    // Words in inverted case: a small letter, then capitals only.
    let mut i = 0;
    while i < line.len() {
        let word_end = (i..line.len())
            .find(|&j| at(j).is_none_or(|c| !c.is_alphabetic()))
            .unwrap_or(line.len());
        if word_end > i {
            let word = line.get(i..word_end).unwrap_or_default();
            if let [head, rest @ ..] = word {
                if head.is_lowercase() && !rest.is_empty() && rest.iter().all(|c| c.is_uppercase())
                {
                    found.push((i + 1, Kind::InvertedCap(*head)));
                }
            }
            i = word_end;
        } else {
            i += 1;
        }
    }

    // Punctuation alone, and punctuation a letter follows.
    for (i, &c) in line.iter().enumerate() {
        if !c.is_ascii_punctuation() {
            continue;
        }
        let left = i.checked_sub(1).and_then(at);
        let right = at(i + 1);
        if spaced(left) && spaced(right) {
            found.push((i + 1, Kind::StrayPunctuation(c)));
        } else if matches!(c, ',' | ';' | ':' | '!' | '?') && right.is_some_and(char::is_alphabetic)
        {
            found.push((i + 1, Kind::UnspacedPunctuation(c)));
        }
    }

    // Brackets, braces and double quotes the line leaves open.
    let mut open: Vec<(usize, char)> = Vec::new();
    for (i, &c) in line.iter().enumerate() {
        match c {
            '[' | '{' => open.push((i, c)),
            ']' | '}' => {
                let opener = if c == ']' { '[' } else { '{' };
                if let Some(at) = open.iter().rposition(|&(_, o)| o == opener) {
                    open.remove(at);
                }
            }
            '"' => match open.iter().rposition(|&(_, o)| o == '"') {
                Some(at) => {
                    open.remove(at);
                }
                None => open.push((i, c)),
            },
            _ => {}
        }
    }
    found.extend(open.into_iter().map(|(i, c)| (i + 1, Kind::Unclosed(c))));
    found
}

fn ends_sentence(c: char) -> bool {
    matches!(c, '.' | '!' | '?')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 568's Notepad fixture, line by line, as JAWS names each line on Insert+Tab
    /// [obs, 568].
    #[test]
    fn the_proofing_lines_say_what_jaws_says() {
        let lines = [
            "This line is clean.",
            "Two  spaces here.",
            "no capital at the start.",
            "A stray , comma.",
            "An unspaced,comma.",
            "An open (paren.",
            "A close) paren.",
            "An open [bracket and a \"quote.",
            "tHIS IS INVERTED.",
            "Ends with a space. ",
        ];
        let text = lines.join("\r\n");
        let said = |line: usize| {
            let caret: usize = lines.iter().take(line).map(|l| l.chars().count() + 2).sum();
            describe_line(&text, caret)
        };
        let none: Vec<String> = Vec::new();
        assert_eq!(said(0), none);
        assert_eq!(said(1), ["Space run at column 4"]);
        assert_eq!(said(2), ["Missing initial cap n at column 1"]);
        assert_eq!(said(3), ["Stray punctuation , at column 9"]);
        assert_eq!(said(4), ["Unspaced punctuation , at column 12"]);
        assert_eq!(said(5), none);
        assert_eq!(said(6), none);
        assert_eq!(
            said(7),
            ["Unclosed [ at column 9", "Unclosed \" at column 24"]
        );
        assert_eq!(
            said(8),
            [
                "Missing initial cap t at column 1",
                "Inverted cap t at column 1"
            ]
        );
        assert_eq!(said(9), none);
    }

    /// The first line of a text is no sentence's continuation; a brace alone at its end is
    /// stray and unclosed, stray first [obs, 571/s4].
    #[test]
    fn a_code_line_names_its_open_brace() {
        let text = "fn main() {\r\n    let total = price * 1.5; // tax\r\n}";
        assert_eq!(
            describe_line(text, 0),
            [
                "Stray punctuation { at column 11",
                "Unclosed { at column 11"
            ]
        );
    }
}
