//! JAWS's text analysis: the inconsistencies of a line of text, which Say Current Focus
//! (Insert+Tab) names after a text field ("Space run at column 4" [obs, 568/s4]).
//!
//! The rules are the ones recordings show, each with its column (from 1, in characters):
//! - a run of spaces inside the line, not the spaces it starts with (568/s4, 614/s15, s29);
//! - a small letter after a sentence's end, once for each full stop, question or exclamation
//!   mark before it ("Wait... what?" names "Missing initial cap w" three times [obs,
//!   614/s21]), the end on this line or the text before it ("no capital at the start." after
//!   "Two  spaces here." [obs, 568/s7]); a stop before a letter or digit ("1.5") or after a
//!   lone letter ("e.g.") ends nothing (614/s13, s25), and the text's first line has nothing
//!   before it ("fn main() {" [obs, 571/s4]);
//! - a word in inverted case, "tHIS" (568/s25, 614/s27);
//! - a comma, full stop, colon, semicolon, question or exclamation mark or a bracket alone
//!   between spaces or the line's ends, "A stray , comma." (568/s10, 571/s4, 614/s5), but not
//!   "*" or "=" (614/s11);
//! - a comma, semicolon, colon, question or exclamation mark with a letter right after it,
//!   "An unspaced,comma.", "Hello!World" (568/s13, 614/s23);
//! - a bracket, brace, parenthesis or double quote left open, or a closing one never opened,
//!   paired over the line and the two lines around it: a parenthesis closed two lines down is
//!   not named (614/s1, s5), one closed three lines down is (571/s4).

use crate::messages::msg;

/// How many lines before and after the caret's line brackets are paired over.
const PAIRING_LINES: usize = 2;

/// One inconsistency, in the order JAWS names them at one column: the start's capital, the
/// word's case, a closer never opened, then punctuation, then an opener left open
/// ("Unopened ] | Stray punctuation ]" [obs, 614/s9]; "Stray punctuation { | Unclosed {"
/// [obs, 571/s4]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    SpaceRun,
    MissingInitialCap(char),
    InvertedCap(char),
    Unopened(char),
    StrayPunctuation(char),
    UnspacedPunctuation(char),
    Unclosed(char),
}

/// What Insert+Tab adds for the line around `caret` (a character offset into `text`), one
/// entry per inconsistency, in column order.
pub fn describe_line(text: &str, caret: usize) -> Vec<String> {
    describe_line_as(text, caret, false)
}

/// [`describe_line`], with short messages when `short`: "Stray punctuation { at 11",
/// "initial cap t at 16" [obs, 942/s1, s6].
pub fn describe_line_as(text: &str, caret: usize, short: bool) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let caret = caret.min(chars.len());
    // The text's lines as character ranges, without their breaks.
    let mut lines: Vec<(usize, usize)> = Vec::new();
    let mut start = 0;
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            let end = if i > start && chars.get(i - 1) == Some(&'\r') {
                i - 1
            } else {
                i
            };
            lines.push((start, end));
            start = i + 1;
        }
    }
    lines.push((start, chars.len()));
    let here = lines
        .iter()
        .position(|&(s, e)| s <= caret && caret <= e)
        .unwrap_or(0);
    let line_of = |index: usize| -> &[char] {
        lines
            .get(index)
            .and_then(|&(s, e)| chars.get(s..e))
            .unwrap_or_default()
    };
    let line = line_of(here);
    // What ends the text before the line: its sentence ends, if any.
    let before_start = lines.get(here).map_or(0, |&(s, _)| s);
    let before = chars.get(..before_start).unwrap_or_default();
    let mut found = inconsistencies(line, sentence_ends_at_end(before));
    // Brackets, braces, parentheses and quotes, paired over the lines around.
    let first = here.saturating_sub(PAIRING_LINES);
    let last = (here + PAIRING_LINES).min(lines.len().saturating_sub(1));
    let window: Vec<(usize, &[char])> = (first..=last).map(|i| (i, line_of(i))).collect();
    found.extend(pairing(&window, here));
    found.sort_by_key(|&(column, kind)| (column, kind));
    found
        .into_iter()
        .map(|(column, kind)| describe(column, kind, short))
        .collect()
}

fn describe(column: usize, kind: Kind, short: bool) -> String {
    let symbol = |c: char| c.to_string();
    match kind {
        Kind::SpaceRun if short => msg::text_analysis_space_run_short(column).to_string(),
        Kind::MissingInitialCap(c) if short => {
            msg::text_analysis_missing_initial_cap_short(symbol(c), column).to_string()
        }
        Kind::StrayPunctuation(c) if short => {
            msg::text_analysis_stray_punctuation_short(symbol(c), column).to_string()
        }
        Kind::UnspacedPunctuation(c) if short => {
            msg::text_analysis_unspaced_punctuation_short(symbol(c), column).to_string()
        }
        Kind::Unclosed(c) if short => {
            msg::text_analysis_unclosed_short(symbol(c), column).to_string()
        }
        Kind::SpaceRun => msg::text_analysis_space_run(column).to_string(),
        Kind::MissingInitialCap(c) => {
            msg::text_analysis_missing_initial_cap(symbol(c), column).to_string()
        }
        Kind::InvertedCap(c) => msg::text_analysis_inverted_cap(symbol(c), column).to_string(),
        Kind::Unopened(c) => msg::text_analysis_unopened(symbol(c), column).to_string(),
        Kind::StrayPunctuation(c) => {
            msg::text_analysis_stray_punctuation(symbol(c), column).to_string()
        }
        Kind::UnspacedPunctuation(c) => {
            msg::text_analysis_unspaced_punctuation(symbol(c), column).to_string()
        }
        Kind::Unclosed(c) => msg::text_analysis_unclosed(symbol(c), column).to_string(),
    }
}

fn is_terminator(c: char) -> bool {
    matches!(c, '.' | '!' | '?')
}

/// Whether the terminator at `i` of `text` ends a sentence: not before a letter or digit
/// ("1.5"), and not after a lone letter ("e.g.").
fn ends_sentence(text: &[char], i: usize) -> bool {
    let at = |j: usize| text.get(j).copied();
    let Some(c) = at(i) else {
        return false;
    };
    if !is_terminator(c) || at(i + 1).is_some_and(char::is_alphanumeric) {
        return false;
    }
    let lone_letter = c == '.'
        && i.checked_sub(1)
            .and_then(at)
            .is_some_and(char::is_alphabetic)
        && i.checked_sub(2)
            .and_then(at)
            .is_none_or(|p| p.is_whitespace() || p == '.');
    !lone_letter
}

/// How many sentence ends the text before a line closes with (its last characters past
/// spaces and line breaks).
fn sentence_ends_at_end(before: &[char]) -> usize {
    let end = before
        .iter()
        .rposition(|c| !c.is_whitespace())
        .map_or(0, |i| i + 1);
    let text = before.get(..end).unwrap_or_default();
    let mut count = 0;
    let mut i = end;
    while i > 0 && text.get(i - 1).copied().is_some_and(is_terminator) {
        if ends_sentence(text, i - 1) {
            count += 1;
        }
        i -= 1;
    }
    count
}

/// The line's own inconsistencies as `(column, kind)`, columns from 1. `ends_before` is how
/// many sentence ends the text before the line closes with.
fn inconsistencies(line: &[char], ends_before: usize) -> Vec<(usize, Kind)> {
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

    // A small letter after sentence ends: once for each.
    let mut pending = ends_before;
    for (i, &c) in line.iter().enumerate() {
        if c.is_alphabetic() {
            if c.is_lowercase() {
                found.extend((0..pending).map(|_| (i + 1, Kind::MissingInitialCap(c))));
            }
            pending = 0;
        } else if ends_sentence(line, i) {
            pending += 1;
        } else if c.is_ascii_digit() {
            pending = 0;
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
        let left = i.checked_sub(1).and_then(at);
        let right = at(i + 1);
        let clause = matches!(c, ',' | '.' | ';' | ':' | '!' | '?');
        let bracket = matches!(c, '(' | ')' | '[' | ']' | '{' | '}');
        if (clause || bracket) && spaced(left) && spaced(right) {
            found.push((i + 1, Kind::StrayPunctuation(c)));
        } else if matches!(c, ',' | ';' | ':' | '!' | '?') && right.is_some_and(char::is_alphabetic)
        {
            found.push((i + 1, Kind::UnspacedPunctuation(c)));
        }
    }
    found
}

/// The caret's line's brackets, braces, parentheses and double quotes left open or never
/// opened, paired over `window`: the lines around it, as `(index, line)`, in order.
fn pairing(window: &[(usize, &[char])], here: usize) -> Vec<(usize, Kind)> {
    // Open symbols as (line index, column from 0, symbol).
    let mut open: Vec<(usize, usize, char)> = Vec::new();
    let mut found = Vec::new();
    for &(index, line) in window {
        for (column, &c) in line.iter().enumerate() {
            let opener = match c {
                ')' => Some('('),
                ']' => Some('['),
                '}' => Some('{'),
                _ => None,
            };
            match (c, opener) {
                ('(' | '[' | '{', _) => open.push((index, column, c)),
                (_, Some(opener)) => match open.iter().rposition(|&(_, _, o)| o == opener) {
                    Some(at) => {
                        open.remove(at);
                    }
                    None if index == here => found.push((column + 1, Kind::Unopened(c))),
                    None => {}
                },
                ('"', _) => match open.iter().rposition(|&(_, _, o)| o == '"') {
                    Some(at) => {
                        open.remove(at);
                    }
                    None => open.push((index, column, c)),
                },
                _ => {}
            }
        }
    }
    found.extend(
        open.into_iter()
            .filter(|&(index, _, _)| index == here)
            .map(|(_, column, c)| (column + 1, Kind::Unclosed(c))),
    );
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Insert+Tab's words on each line of `lines`, a CRLF text as Notepad holds it, against
    /// `expected`, line by line.
    fn check(lines: &[&str], expected: &[&[&str]]) {
        let text = lines.join("\r\n");
        let said: Vec<Vec<String>> = (0..lines.len())
            .map(|line| {
                let caret: usize = lines.iter().take(line).map(|l| l.chars().count() + 2).sum();
                describe_line(&text, caret)
            })
            .collect();
        let expected: Vec<Vec<String>> = expected
            .iter()
            .map(|words| words.iter().map(|w| (*w).to_owned()).collect())
            .collect();
        assert_eq!(said, expected);
    }

    /// 568's Notepad fixture, line by line, as JAWS names each line on Insert+Tab
    /// [obs, 568].
    #[test]
    fn the_proofing_lines_say_what_jaws_says() {
        check(
            &[
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
            ],
            &[
                &[],
                &["Space run at column 4"],
                &["Missing initial cap n at column 1"],
                &["Stray punctuation , at column 9"],
                &["Unspaced punctuation , at column 12"],
                &[],
                &[],
                &["Unclosed [ at column 9", "Unclosed \" at column 24"],
                &[
                    "Missing initial cap t at column 1",
                    "Inverted cap t at column 1",
                ],
                &[],
            ],
        );
    }

    /// 614's second fixture: pairing over two lines around, an unopened bracket, symbols that
    /// are not punctuation, numbers, abbreviations and an ellipsis [obs, 614]. Lines 12 and 14
    /// leave out JAWS's "Space run" past the line's end, an accepted divergence.
    #[test]
    fn the_second_proofing_lines_say_what_jaws_says() {
        let ellipsis = "Missing initial cap w at column 9";
        check(
            &[
                "One (open paren here.",
                "Nothing on this line.",
                "Here ) it closes.",
                "A lone (paren.",
                "A lone ] bracket.",
                "Math 2 * 3 = 6.",
                "Price 1.5 and 3,000 here.",
                "Twice  and   thrice.",
                "Quote \"closed\" here.",
                "It's fine.",
                "Wait... what?",
                "Hello!World",
                "For e.g. this.",
                "Mixed Case wORD.",
                "  Leading spaces.",
                "A {brace} and <angle> here.",
            ],
            &[
                &[],
                &[],
                &["Stray punctuation ) at column 6"],
                &["Unclosed ( at column 8"],
                &["Unopened ] at column 8", "Stray punctuation ] at column 8"],
                &[],
                &[],
                &["Space run at column 6", "Space run at column 11"],
                &[],
                &[],
                &[ellipsis, ellipsis, ellipsis],
                &["Unspaced punctuation ! at column 6"],
                &[],
                &["Inverted cap w at column 12"],
                &[],
                &[],
            ],
        );
    }

    /// The first line of a text is no sentence's continuation; a brace alone at its end is
    /// stray, and unclosed when its closer is three lines down [obs, 571/s4].
    #[test]
    fn a_code_line_names_its_open_brace() {
        let text =
            "fn main() {\r\n    let total = price * 1.5; // tax\r\n    println!(\"{total}\");\r\n}";
        assert_eq!(
            describe_line(text, 0),
            [
                "Stray punctuation { at column 11",
                "Unclosed { at column 11"
            ]
        );
    }
}
