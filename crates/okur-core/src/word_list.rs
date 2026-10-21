//! The Word List (Ctrl+Insert+W, JAWS's WordList): the words of the page, each with how many
//! times it occurs and where, for its dialog and for W and Shift+W (MoveToNextWordFromList,
//! MoveToPriorWordFromList).
//!
//! As JAWS lists them on the news article [obs, 560/s1]: a word is a run of letters and
//! digits ("m" and "p" from "a.m." and "p.m.", "e" and "g" from "e.g."), keeping an apostrophe
//! and an at sign within ("can't", "help@riverside"); numbers are not words; short common
//! words are left out ("a", "the", "to"); "Library" and "library" are two words. The lines
//! counted are the page's text, its title among them, and what the virtual cursor says of
//! regions, articles and block quotes ("Primary navigation region end"), not of lists and
//! tables. The list goes by count, most first, then by the words' characters, capitals
//! first; a count shows from two: "m (10)", "Attendance".

use std::collections::HashMap;

use crate::aom::Role;
use crate::buffer::VirtualBuffer;

/// The English words left out, one per line ("the", "to") [obs, 560/s1].
const LEFT_OUT: &str = include_str!("../locales/en-US/word-list-common-words.txt");

/// Whether the list leaves `word` out, whatever its capitals.
fn left_out(word: &str) -> bool {
    LEFT_OUT
        .lines()
        .filter(|line| !line.starts_with('#'))
        .any(|line| line.eq_ignore_ascii_case(word))
}

/// A word of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub word: String,
    /// Where it occurs in the page's buffer, in order.
    pub offsets: Vec<usize>,
}

impl Word {
    /// As the list shows it: "m (10)", "Attendance".
    pub fn label(&self) -> String {
        match self.offsets.len() {
            0 | 1 => self.word.clone(),
            count => format!("{} ({count})", self.word),
        }
    }
}

/// Whether `c` goes in a word: a letter or digit, or one of the marks JAWS keeps in one. The
/// white bullet of a nested list is one ("◦ (2)" [obs, 560/s1]), the black one is not.
fn in_word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '@' | '\'' | '\u{2019}' | '\u{25E6}')
}

/// Whether a line of the buffer is counted: not a list's or a table's announcement.
fn counted(role: &Role) -> bool {
    !matches!(role, Role::List | Role::Table)
}

/// The page's words, in the list's order.
pub fn words(buffer: &VirtualBuffer) -> Vec<Word> {
    let mut found: HashMap<String, Vec<usize>> = HashMap::new();
    for line in buffer.lines.iter().filter(|l| counted(&l.role)) {
        let mut start: Option<usize> = None;
        let chars: Vec<char> = line.text.chars().collect();
        for (i, c) in chars.iter().chain(std::iter::once(&' ')).enumerate() {
            match (in_word(*c), start) {
                (true, None) => start = Some(i),
                (false, Some(from)) => {
                    start = None;
                    let word: String = chars.get(from..i).unwrap_or_default().iter().collect();
                    let word = word.trim_matches(|c| matches!(c, '\'' | '\u{2019}'));
                    if word.is_empty() || word.chars().all(|c| c.is_ascii_digit()) || left_out(word)
                    {
                        continue;
                    }
                    found
                        .entry(word.to_owned())
                        .or_default()
                        .push(line.start_offset + from);
                }
                _ => {}
            }
        }
    }
    let mut words: Vec<Word> = found
        .into_iter()
        .map(|(word, offsets)| Word { word, offsets })
        .collect();
    words.sort_by(|a, b| {
        b.offsets
            .len()
            .cmp(&a.offsets.len())
            .then_with(|| a.word.cmp(&b.word))
    });
    words
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::AomNode;

    /// The news article's ways with words, in miniature [obs, 560/s1].
    #[test]
    fn the_list_counts_words_as_jaws_does() {
        let mut nav = AomNode::new(
            2,
            Role::Landmark {
                landmark_type: "navigation".into(),
            },
        )
        .with_children(vec![AomNode::new(3, Role::List)
            .with_children(vec![AomNode::new(4, Role::ListItem).with_text("News")])]);
        nav.name = Some("Primary".into());
        let page = AomNode::new(0, Role::Document).with_children(vec![
            nav,
            AomNode::new(5, Role::Paragraph)
                .with_text("The library opens at 9 a.m. and closes at 9 p.m."),
            AomNode::new(6, Role::Paragraph).with_text("Library hours: e.g. 10 a.m. to noon."),
        ]);
        let buffer = VirtualBuffer::from_aom(&page);
        let labels: Vec<String> = words(&buffer).iter().map(Word::label).collect();
        // "m (3)" first; "library" and "Library" apart; no "the", "at", "to", "and", digits,
        // nor the list's announcement.
        assert_eq!(labels.first().map(String::as_str), Some("m (3)"));
        assert!(labels.contains(&"library".to_owned()), "{labels:?}");
        assert!(labels.contains(&"Library".to_owned()), "{labels:?}");
        assert!(labels.contains(&"e".to_owned()), "{labels:?}");
        for word in ["the", "at", "to", "and", "9", "10", "list", "items"] {
            assert!(
                !labels
                    .iter()
                    .any(|l| l == word || l.starts_with(&format!("{word} ("))),
                "{word} in {labels:?}"
            );
        }
    }
}
