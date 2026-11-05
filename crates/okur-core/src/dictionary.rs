//! The speech dictionary: words the synthesizer is given otherwise than they are written, as
//! JAWS's Dictionary Manager (Insert+D) keeps them: "If JAWS mispronounces your name, for
//! example, you can change the pronunciation with the Dictionary Manager" (JAWS 2025 help,
//! Introduction to Dictionary Manager).
//!
//! A rule changes what is spoken, not what is read: JAWS's speech history keeps "let total =
//! price" while its synthesizer is given "let sum equals price", and Say Word "total" while the
//! synthesizer says "sum"; a character read alone is itself ("o") [obs, old-pc 2026-11-05,
//! 1154]. So the dictionary works where the history's text becomes the synthesizer's
//! ([`crate::pronounce`]).
//!
//! From the help (Dictionary Manager Basics):
//! - a rule's word is matched whatever its case, unless it is Case Sensitive;
//! - "reposition*" is a root: it changes "reposition", "repositioning" and "repositions", its
//!   own letters replaced and the rest of the word kept ("tot*" as "sum" says "sumal" for
//!   "total" [obs, 1155/s15]);
//! - a rule may be limited to a language, a synthesizer or a voice, or play a sound: Okur's
//!   dialog has JAWS's fields for them [obs, 1158], and its rules keep a sound, but reading
//!   does not use them yet.

use serde::{Deserialize, Serialize};

/// One rule: the word as written, and what the synthesizer is given for it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DictionaryEntry {
    /// The word as written ("Actual Word"); ending in `*`, a root that any longer word starting
    /// with it matches too.
    pub actual: String,
    /// What is spoken in its place ("Replacement Word"); empty, nothing.
    pub replacement: String,
    /// Only the word written with the same capitals matches.
    pub case_sensitive: bool,
    /// The sound file the rule's Sound field names (Select Sound... in its dialog); empty, none
    /// ("<None>" [obs, 1158/s9]). Kept and played from the dialog; what JAWS does with it when
    /// the word is read is not recorded yet, so reading does not use it.
    pub sound: String,
}

impl DictionaryEntry {
    /// The rule `actual` is spoken as `replacement`, whatever its case.
    pub fn new(actual: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            actual: actual.into(),
            replacement: replacement.into(),
            ..Self::default()
        }
    }

    /// The word or root it matches, and whether it is a root.
    fn pattern(&self) -> (&str, bool) {
        match self.actual.strip_suffix('*') {
            Some(root) => (root, true),
            None => (self.actual.as_str(), false),
        }
    }

    /// Whether `word`, one word of the text, is this rule's.
    fn matches(&self, word: &str) -> bool {
        let (pattern, root) = self.pattern();
        if pattern.is_empty() {
            return false;
        }
        let same = |a: &str, b: &str| {
            if self.case_sensitive {
                a == b
            } else {
                a.chars()
                    .flat_map(char::to_lowercase)
                    .eq(b.chars().flat_map(char::to_lowercase))
            }
        };
        if !root {
            return same(word, pattern);
        }
        let length = pattern.chars().count();
        let start: String = word.chars().take(length).collect();
        word.chars().count() >= length && same(&start, pattern)
    }
}

/// Whether `c` is part of a word, for the dictionary: letters and digits.
fn word_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// `text` with every word a rule of `entries` matches spoken as its replacement: the first rule
/// that matches a word wins. Words are runs of letters and digits; a rule whose word holds
/// other characters ("w/") is matched where it stands between words.
pub fn apply(entries: &[DictionaryEntry], text: &str) -> String {
    if entries.is_empty() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        // A rule written with other characters than a word's, at a word's start.
        let at_start = out.chars().last().is_none_or(|c| !word_char(c));
        if at_start {
            if let Some(entry) = entries.iter().find(|e| symbol_rule_at(e, rest)) {
                out.push_str(&entry.replacement);
                rest = rest.get(entry.actual.len()..).unwrap_or("");
                continue;
            }
        }
        let word_len: usize = rest
            .chars()
            .take_while(|&c| word_char(c))
            .map(char::len_utf8)
            .sum();
        if word_len == 0 {
            let mut chars = rest.chars();
            if let Some(c) = chars.next() {
                out.push(c);
            }
            rest = chars.as_str();
            continue;
        }
        let word = rest.get(..word_len).unwrap_or(rest);
        match entries.iter().find(|e| e.matches(word)) {
            // A root gives its own letters' replacement and leaves the rest of the word:
            // "tot*" as "sum" says "total" "sumal" [obs, 1155/s15].
            Some(entry) => {
                out.push_str(&entry.replacement);
                let (pattern, root) = entry.pattern();
                if root {
                    out.extend(word.chars().skip(pattern.chars().count()));
                }
            }
            None => out.push_str(word),
        }
        rest = rest.get(word_len..).unwrap_or("");
    }
    out
}

/// Whether `entry` is a rule written with other characters than a word's ("w/") and `text`
/// starts with it, ending where a word does.
fn symbol_rule_at(entry: &DictionaryEntry, text: &str) -> bool {
    let actual = entry.actual.as_str();
    if actual.is_empty() || actual.ends_with('*') || actual.chars().all(word_char) {
        return false;
    }
    let Some(head) = text.get(..actual.len()) else {
        return false;
    };
    let same = if entry.case_sensitive {
        head == actual
    } else {
        head.to_lowercase() == actual.to_lowercase()
    };
    let ends_word = text
        .get(actual.len()..)
        .and_then(|after| after.chars().next())
        .is_none_or(|c| !word_char(c) || !actual.chars().last().is_some_and(word_char));
    same && ends_word
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JAWS gave its synthesizer "let sum equals price" for "let total = price" with the rule
    /// "total" as "sum" [obs, old-pc 2026-11-05]; the word inside a longer one is left.
    #[test]
    fn a_rule_replaces_its_whole_word() {
        let rules = [DictionaryEntry::new("total", "sum")];
        assert_eq!(
            apply(&rules, "    let total = price * 1.5; // tax"),
            "    let sum = price * 1.5; // tax"
        );
        assert_eq!(apply(&rules, "totals subtotal"), "totals subtotal");
    }

    /// Matched whatever its case, unless Case Sensitive (Dictionary Manager Basics).
    #[test]
    fn case_matters_only_when_the_rule_says_so() {
        let any = [DictionaryEntry::new("total", "sum")];
        assert_eq!(apply(&any, "Total TOTAL"), "sum sum");
        let exact = [DictionaryEntry {
            case_sensitive: true,
            ..DictionaryEntry::new("Total", "Sum")
        }];
        assert_eq!(apply(&exact, "Total total"), "Sum total");
    }

    /// "reposition*" changes "reposition", "repositioning" and "repositions" (Dictionary
    /// Manager Basics, Root Word Pronunciation Changes): the root alone is replaced, the rest
    /// of the word kept, as JAWS said "sumal" for "total" with "tot*" as "sum" [obs, 1155/s15].
    #[test]
    fn a_root_replaces_its_own_letters() {
        let rules = [DictionaryEntry::new("reposition*", "re position")];
        assert_eq!(
            apply(&rules, "reposition, repositioning and repositions; posit"),
            "re position, re positioning and re positions; posit"
        );
        let tot = [DictionaryEntry::new("tot*", "sum")];
        assert_eq!(apply(&tot, "let total = {total}"), "let sumal = {sumal}");
    }

    /// A rule of other characters than letters: "w/" as "with" (the help's Outlook example).
    #[test]
    fn a_rule_with_a_symbol_is_matched_where_it_stands() {
        let rules = [DictionaryEntry::new("w/", "with")];
        assert_eq!(apply(&rules, "tea w/ milk, tw/o"), "tea with milk, tw/o");
    }

    /// No rules, or an empty word, change nothing.
    #[test]
    fn no_rule_changes_nothing() {
        assert_eq!(apply(&[], "let total"), "let total");
        assert_eq!(
            apply(&[DictionaryEntry::new("", "x")], "let total"),
            "let total"
        );
    }
}
