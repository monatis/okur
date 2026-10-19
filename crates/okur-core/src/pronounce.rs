//! What the synthesizer is asked to say, as opposed to what JAWS's speech history records.
//!
//! JAWS keeps the text it decided to speak in its speech history (`(`, or an empty entry for
//! a space) and only then applies its symbol processing: reading a character names it ("left
//! paren", "space") whatever the punctuation level. The engine produces history-level text, so
//! parity compares it with the speech history, and this module turns it into spoken text for
//! the synthesizer and for comparison with what JAWS sent its synthesizer (the speech
//! recorder voice's log, `observation.spoken`).

use crate::settings::Punctuation;

/// The name JAWS speaks for a character read on its own, when it names it.
pub fn character_name(c: char) -> Option<&'static str> {
    SYMBOL_NAMES
        .iter()
        .find(|(symbol, _)| *symbol == c)
        .map(|(_, name)| *name)
}

/// Emoji, which JAWS reads by their Unicode name in capitals ("GRINNING FACE") [obs, 307/s82].
fn emoji_name(c: char) -> Option<String> {
    (u32::from(c) >= 0x1F000)
        .then(|| unicode_names2::name(c))
        .flatten()
        .map(|name| name.to_string())
}

/// A character read on its own (Left/Right, Delete): its name, or the character itself.
/// Anything other than a single character is returned unchanged.
pub fn character(text: &str) -> String {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => character_name(c)
            .map(str::to_owned)
            .or_else(|| emoji_name(c))
            .unwrap_or_else(|| c.to_string()),
        _ => text.to_owned(),
    }
}

/// A character spelled phonetically (Say Character pressed twice quickly): a letter's word
/// ("juliett" for `J`), any other character as [`character`] reads it ("euro sign", "é")
/// [obs, 401/s2, s9, s13, s20, s22]. The history keeps the character.
pub fn phonetic(text: &str) -> String {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => ('a'..='z')
            .zip(PHONETIC)
            .find(|(letter, _)| *letter == c.to_ascii_lowercase())
            .map_or_else(|| character(text), |(_, word)| word.to_owned()),
        _ => character(text),
    }
}

/// A text spelled (Say Word or Say Line pressed twice quickly), as JAWS's history has it: its
/// characters one by one, a space as "space": "A d a space R E A D" [obs, 406/s2, s7].
pub fn spelled(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            ' ' => character(" "),
            c => c.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// What the synthesizer is given for a spelled text, in parts with their pitch change: each
/// capital alone, raised; the characters between capitals together, by name ("space")
/// [obs, 406/s5, s7: "[pitch +4] A d a space [pitch +4] R [pitch +4] E ...].
pub fn spelled_parts(text: &str) -> Vec<(String, i32)> {
    let mut parts = Vec::new();
    let mut run: Vec<String> = Vec::new();
    for c in text.chars() {
        let one = c.to_string();
        let pitch = character_pitch(&one);
        if pitch == 0 {
            run.push(character(&one));
            continue;
        }
        if !run.is_empty() {
            parts.push((run.join(" "), 0));
            run.clear();
        }
        parts.push((character(&one), pitch));
    }
    if !run.is_empty() {
        parts.push((run.join(" "), 0));
    }
    parts
}

/// The words of the letters a to z, as JAWS gives them to its synthesizer: NATO's, but "Fox Trot"
/// and "X ray" [obs, 404].
const PHONETIC: [&str; 26] = [
    "alpha", "bravo", "charlie", "delta", "echo", "Fox Trot", "golf", "hotel", "india", "juliett",
    "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra", "tango",
    "uniform", "victor", "whiskey", "X ray", "yankee", "zulu",
];

/// The pitch change of a capital letter read alone, on SAPI's scale (-10 to 10). JAWS sent
/// this for the reference voice profile's Upper Case Pitch Change of 20 [obs, 307/s35: `B` at
/// pitch 4; 207].
pub const CAPITAL_PITCH: i32 = 4;

/// The pitch change of a character read alone: capitals are raised.
pub fn character_pitch(text: &str) -> i32 {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_uppercase() => CAPITAL_PITCH,
        _ => 0,
    }
}

/// Spoken text as the spoken layer compares it: with its pitch change, when it has one.
pub fn pitched(text: &str, pitch: i32) -> String {
    if pitch == 0 {
        text.to_owned()
    } else {
        format!("[pitch {pitch:+}] {text}")
    }
}

/// Text for the synthesizer at the punctuation `level`, as JAWS 2025 sends it
/// (`docs/references/jaws-punctuation-levels.md`; checked against every history line of the
/// corpus paired with what JAWS sent for it):
/// - the role markers JAWS's history keeps (`\u{1d}…\u{1e}`) are not spoken characters;
/// - words are split where a capital follows a small letter and where letters meet digits
///   (Mixed Case Processing: "MainRegion" as "Main Region", "List1" as "List 1");
/// - a number of five or more digits, or with dashes, is read digit by digit ("5 5 5 dash 0 1
///   4 2", Speak Single Digits);
/// - a symbol named at the level is replaced by its name; above it, a pause mark (`, . ; : ? !`)
///   stays for the voice's pause and any other symbol becomes a space;
/// - periods have their special places: "dot" between letters or before a word, a run of
///   periods "dot dot dot", a decimal point left alone below All and "point" with the digits
///   after it one by one at All;
/// - a time's colon, a percent after a number and an apostrophe inside a word stay.
pub fn spoken(text: &str, level: Punctuation) -> String {
    render(text, level, level == Punctuation::All)
}

/// Text for the synthesizer with every symbol named, whatever the level: a selection and a
/// word read on its own ("Friday comma, January 2 comma," [obs, 323/s7]; "dog comma,"
/// [obs, 213/s11]). Numbers stay as at the lower levels: "photo dot jpg 1.2 MB" after Ctrl+A
/// [obs, 322/s16].
pub fn spoken_symbols(text: &str) -> String {
    render(text, Punctuation::All, false)
}

/// `spoken`'s work; `point_digits`: a decimal point is "point" and the digits after it are
/// read one by one, as at All.
fn render(text: &str, level: Punctuation, point_digits: bool) -> String {
    let text = single_digits(&split_words(&respelled(
        &text.replace(['\u{1d}', '\u{1e}'], " "),
    )));
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() * 2);
    let word = |out: &mut String, name: &str| {
        out.push(' ');
        out.push_str(name);
        out.push(' ');
    };
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        let prev = i.checked_sub(1).and_then(|p| chars.get(p)).copied();
        let next = chars.get(i + 1).copied();
        i += 1;
        if let Some(name) = named_always(c) {
            word(&mut out, &name);
            continue;
        }
        let Some((name, from)) = punctuation(c) else {
            out.push(c);
            continue;
        };
        let named = level >= from;
        let dot = |out: &mut String| {
            if level >= Punctuation::Some {
                word(out, "dot");
            } else {
                out.push_str(". ");
            }
        };
        match c {
            '.' if prev == Some('.') || next == Some('.') => {
                if level >= Punctuation::Some {
                    word(&mut out, "dot");
                } else {
                    out.push(' ');
                }
            }
            '.' if is_digit(prev) && is_digit(next) => {
                if point_digits {
                    word(&mut out, "point");
                    while let Some(&d) = chars.get(i).filter(|d| d.is_ascii_digit()) {
                        word(&mut out, &d.to_string());
                        i += 1;
                    }
                } else {
                    out.push('.');
                }
            }
            '.' if is_alphanumeric(prev) && is_alphanumeric(next) => dot(&mut out),
            '.' if next.is_some_and(char::is_alphabetic) => dot(&mut out),
            '%' if is_digit(prev) => out.push(c),
            ':' if is_digit(prev) && is_digit(next) => out.push(c),
            // A thousands separator is part of the number, never named: "1,000,000", "dollar
            // 1,250.50" [obs, 405/s14, s15; 407/s21].
            ',' if is_digit(prev) && is_digit(next) => out.push(c),
            '\'' | '\u{2019}'
                if !named
                    && prev.is_some_and(char::is_alphabetic)
                    && next.is_some_and(char::is_alphabetic) =>
            {
                out.push(c)
            }
            _ if named => word(&mut out, name),
            ',' | '.' | ';' | ':' | '?' | '!' => {
                out.push(c);
                out.push(' ');
            }
            '\u{2026}' => out.push(c),
            _ => out.push(' '),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_digit(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_digit())
}

fn is_alphanumeric(c: Option<char>) -> bool {
    c.is_some_and(char::is_alphanumeric)
}

/// A symbol's name in running text, and the level it is named from.
fn punctuation(c: char) -> Option<(&'static str, Punctuation)> {
    PUNCTUATION
        .iter()
        .find(|(symbol, _, _)| *symbol == c)
        .map(|(_, name, level)| (*name, *level))
}

/// Symbols named at every level, in running text too: "euro sign", "copyright", emoji by
/// their Unicode name [obs, 313/s4].
fn named_always(c: char) -> Option<String> {
    const NAMED: &[char] = &[
        '\u{20ac}', '\u{2192}', '\u{a9}', '\u{bd}', '\u{2260}', '\u{2264}', '\u{3bb}',
    ];
    NAMED
        .contains(&c)
        .then(|| character_name(c).map(str::to_owned))
        .flatten()
        .or_else(|| emoji_name(c))
}

/// Words JAWS respells for the voice (Use Dictionary): only those seen in evidence, since
/// JAWS's own dictionary is not read [obs, 306/s1 "space Bar", 205/s2 "Space Bar"].
const RESPELLINGS: &[(&str, &str)] = &[("spacebar", "space Bar"), ("Spacebar", "Space Bar")];

fn respelled(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut letters = String::new();
    let flush = |out: &mut String, letters: &mut String| {
        let word = std::mem::take(letters);
        match RESPELLINGS.iter().find(|(from, _)| *from == word) {
            Some((_, to)) => out.push_str(to),
            None => out.push_str(&word),
        }
    };
    for c in text.chars() {
        if c.is_alphabetic() {
            letters.push(c);
        } else {
            flush(&mut out, &mut letters);
            out.push(c);
        }
    }
    flush(&mut out, &mut letters);
    out
}

/// A word starts where a capital follows a small letter, at the last capital of a run of them
/// followed by a small letter ("XMLHttpRequest" as "XML Http Request" [obs, 405/s1]), and where
/// letters and digits meet, but for an ordinal's ending ("1st", "2nd", "3rd" [obs, 405/s7]).
fn split_words(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 8);
    for (i, &c) in chars.iter().enumerate() {
        let at = |k: usize| chars.get(k).copied();
        let prev = i.checked_sub(1).and_then(at);
        let next = at(i + 1);
        if let Some(p) = prev {
            let run_ends =
                p.is_uppercase() && c.is_uppercase() && next.is_some_and(char::is_lowercase);
            let ordinal = p.is_ascii_digit() && is_ordinal_ending(&chars, i);
            if (p.is_lowercase() && c.is_uppercase())
                || run_ends
                || (p.is_alphabetic() && c.is_ascii_digit())
                || (p.is_ascii_digit() && c.is_alphabetic() && !ordinal)
            {
                out.push(' ');
            }
        }
        out.push(c);
    }
    out
}

/// Whether the letters from `start` are an ordinal's ending after a number: "st", "nd", "rd"
/// or "th", then no other letter.
fn is_ordinal_ending(chars: &[char], start: usize) -> bool {
    let ending: String = chars
        .get(start..)
        .unwrap_or_default()
        .iter()
        .take_while(|c| c.is_alphabetic())
        .collect();
    matches!(ending.as_str(), "st" | "nd" | "rd" | "th")
}

/// Numbers of five or more digits, and numbers with dashes, digit by digit.
fn single_digits(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() * 2);
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if !c.is_ascii_digit() {
            out.push(c);
            i += 1;
            continue;
        }
        let mut number = String::new();
        while let Some(&d) = chars.get(i) {
            let dash_between = d == '-' && is_digit(chars.get(i + 1).copied());
            if !(d.is_ascii_digit() || dash_between) {
                break;
            }
            number.push(d);
            i += 1;
        }
        let digits = number.chars().filter(char::is_ascii_digit).count();
        if number.contains('-') || digits >= 5 {
            let spaced: Vec<String> = number.chars().map(|d| d.to_string()).collect();
            out.push_str(&spaced.join(" "));
        } else {
            out.push_str(&number);
        }
    }
    out
}

/// Symbols and their names in running text, with the level each is named from: JAWS 2025's
/// Customize Punctuation list on the reference machine
/// (`docs/references/jaws-punctuation-levels.md`). Its U+001E, a non-breaking hyphen, is left
/// out: here it ends a role marker.
const PUNCTUATION: &[(char, &str, Punctuation)] = {
    use Punctuation::{All, Most, None, Some};
    &[
        ('!', "exclaim!", All),
        ('"', "quote", Most),
        ('#', "number", Some),
        ('$', "dollar", Some),
        ('%', "percent", Some),
        ('&', "and", Some),
        ('\'', "apostrophe", All),
        ('(', "left paren", Most),
        (')', "right paren", Most),
        ('*', "star", Some),
        ('+', "plus", Some),
        (',', "comma,", All),
        ('-', "dash", Most),
        ('.', "period.", All),
        ('/', "slash", Some),
        (':', "colon", Most),
        (';', "semi colon", Most),
        ('<', "less", Most),
        ('=', "equals", Some),
        ('>', "greater", Most),
        ('?', "question?", All),
        ('@', "at", Some),
        ('[', "left bracket", Most),
        ('\\', "back slash", Most),
        (']', "right bracket", Most),
        ('^', "carat", Some),
        ('_', "underline", Most),
        ('`', "grawv", Most),
        ('{', "left brace", Some),
        ('|', "vertical bar", Most),
        ('}', "right brace", Some),
        ('~', "til de", Most),
        ('\u{b}', "vertical tab", Most),
        ('\u{c}', "page break", Some),
        ('\u{a1}', "inverted exclaim", All),
        ('\u{a6}', "broken bar", Most),
        ('\u{ab}', "left double angle bracket", Most),
        ('\u{b7}', "Middle Dot", Most),
        ('\u{bb}', "right double angle bracket", Most),
        ('\u{bf}', "inverted question", All),
        ('\u{192}', "florin", All),
        ('\u{2013}', "en dash", All),
        ('\u{2014}', "em dash", All),
        ('\u{2018}', "apostrophe", Most),
        ('\u{2019}', "apostrophe", Most),
        ('\u{201a}', "low single quote", Most),
        ('\u{201b}', "reversed single quote", Most),
        ('\u{201c}', "left quote", Most),
        ('\u{201d}', "right quote", Most),
        ('\u{201e}', "low quote", Most),
        ('\u{201f}', "reversed quote", Most),
        ('\u{2020}', "single dagger", All),
        ('\u{2021}', "double dagger", All),
        ('\u{2022}', "bullet", None),
        ('\u{2023}', "triangular bullet", None),
        ('\u{2026}', "ellipsis", All),
        ('\u{2043}', "dash bullet", None),
        ('\u{204c}', "leftwards bullet", None),
        ('\u{204d}', "rightwards bullet", None),
        ('\u{2219}', "Bullet Operator", Most),
        ('\u{25c9}', "japanese tainome bullet", None),
        ('\u{25cf}', "star", Some),
        ('\u{25e6}', "white bullet", None),
        ('\u{2619}', "rotated heart bullet", None),
        ('\u{2b9a}', "Hollow Arrow Bullet", None),
        ('\u{f06e}', "emdash bullet", Some),
        ('\u{f071}', "hollow square bullet", None),
        ('\u{f076}', "starred bullet", None),
        ('\u{f0a7}', "filled square bullet", None),
        ('\u{f0b7}', "bullet", None),
        ('\u{f0d8}', "arrow bullet", None),
        ('\u{f0e0}', "right arrow bullet", None),
        ('\u{f0fc}', "checkmark bullet", None),
    ]
};

/// Names of characters read on their own: exactly what JAWS 2025 sent its SAPI 5 (64-bit)
/// synthesizer for Left/Right over each character [obs, 307]. A few names keep the symbol
/// after them ("exclaim!", "comma,") and a few are respelled for the voice ("grawv", "til
/// de"). Letters, digits and symbols JAWS does not name (`é`, `°`, `×`, `±`) go to the
/// synthesizer as they are.
const SYMBOL_NAMES: &[(char, &str)] = &[
    (' ', "space"),
    ('\u{a0}', "space"),
    ('\t', "tab"),
    ('\n', "blank"),
    ('\r', "blank"),
    ('!', "exclaim!"),
    ('"', "quote"),
    ('#', "number"),
    ('$', "dollar"),
    ('%', "percent"),
    ('&', "and"),
    ('\'', "apostrophe"),
    ('(', "left paren"),
    (')', "right paren"),
    ('*', "star"),
    ('+', "plus"),
    (',', "comma,"),
    ('-', "dash"),
    ('.', "period."),
    ('/', "slash"),
    (':', "colon"),
    (';', "semi colon"),
    ('<', "less"),
    ('=', "equals"),
    ('>', "greater"),
    ('?', "question?"),
    ('@', "at"),
    ('[', "left bracket"),
    ('\\', "back slash"),
    (']', "right bracket"),
    ('^', "carat"),
    ('_', "underline"),
    ('`', "grawv"),
    ('{', "left brace"),
    ('|', "vertical bar"),
    ('}', "right brace"),
    ('~', "til de"),
    ('\u{20ac}', "euro sign"),
    ('\u{2014}', "em dash \u{2014}"),
    ('\u{201c}', "left quote"),
    ('\u{201d}', "right quote"),
    ('\u{2018}', "apostrophe"),
    ('\u{2019}', "apostrophe"),
    ('\u{2026}', "ellipsis \u{2026}"),
    ('\u{2022}', "bullet"),
    ('\u{2192}', "rightwards arrow"),
    ('\u{a9}', "copyright"),
    ('\u{bd}', "one half"),
    ('\u{2260}', "not equal to"),
    ('\u{2264}', "less-than or equal to"),
    ('\u{3bb}', "lamda"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_character_is_named_and_text_passes_through() {
        assert_eq!(character(" "), "space");
        assert_eq!(character("("), "left paren");
        assert_eq!(character("."), "period.");
        assert_eq!(character("\t"), "tab");
        assert_eq!(character("\n"), "blank");
        assert_eq!(character("\u{1F600}"), "GRINNING FACE");
        assert_eq!(character("a"), "a");
        assert_eq!(character("\u{e9}"), "\u{e9}");
        assert_eq!(character("blank"), "blank");
    }

    /// What JAWS sent its synthesizer for the reading fixture's line 4 at each level
    /// [obs, 400/s4, s10, s16, s22].
    #[test]
    fn words_numbers_and_ordinals_as_jaws_splits_them() {
        // [obs, 405/s1, s7, s14, s15]
        let most = |t: &str| spoken(t, Punctuation::Most);
        assert_eq!(most("XMLHttpRequest"), "XML Http Request");
        assert_eq!(
            most("System32 1st 2nd 3rd 4th"),
            "System 32 1st 2nd 3rd 4th"
        );
        assert_eq!(most("1,000,000 $1,250.50"), "1,000,000 dollar 1,250.50");
        assert_eq!(spoken_symbols("$1,250.50"), "dollar 1,250.50");
    }

    #[test]
    fn a_text_spelled_is_its_characters_capitals_raised() {
        // [obs, 406/s5, s7]
        assert_eq!(spelled("Ada READ"), "A d a space R E A D");
        let parts = spelled_parts("Ada RE");
        assert_eq!(
            parts,
            [
                ("A".to_owned(), CAPITAL_PITCH),
                ("d a space".to_owned(), 0),
                ("R".to_owned(), CAPITAL_PITCH),
                ("E".to_owned(), CAPITAL_PITCH),
            ]
        );
    }

    #[test]
    fn a_letter_spelled_phonetically_is_its_word() {
        // [obs, 401/s2, s9, s13, s17, s20, s22]
        assert_eq!(phonetic("J"), "juliett");
        assert_eq!(phonetic("B"), "bravo");
        assert_eq!(phonetic("a"), "alpha");
        assert_eq!(phonetic("€"), "euro sign");
        assert_eq!(phonetic("é"), "é");
        assert_eq!(phonetic("9"), "9");
    }

    #[test]
    fn each_level_names_its_symbols() {
        let line = "Price $9.95, 50% off; total = 3 * 4 + 1 - 2 / 5 ^ 2 ~ 7";
        assert_eq!(
            spoken(line, Punctuation::None),
            "Price 9.95, 50% off; total 3 4 1 2 5 2 7"
        );
        assert_eq!(
            spoken(line, Punctuation::Some),
            "Price dollar 9.95, 50% off; total equals 3 star 4 plus 1 2 slash 5 carat 2 7"
        );
        assert_eq!(
            spoken(line, Punctuation::Most),
            "Price dollar 9.95, 50% off semi colon total equals 3 star 4 plus 1 dash 2 slash 5 \
             carat 2 til de 7"
        );
        assert_eq!(
            spoken(line, Punctuation::All),
            "Price dollar 9 point 9 5 comma, 50% off semi colon total equals 3 star 4 plus 1 \
             dash 2 slash 5 carat 2 til de 7"
        );
        let address = "Mail ada@example.org or see https://example.org/a-b_c?x=1&y=2#top";
        assert_eq!(
            spoken(address, Punctuation::None),
            "Mail ada example. org or see https: example. org a b c? x 1 y 2 top"
        );
        assert_eq!(
            spoken(address, Punctuation::Most),
            "Mail ada at example dot org or see https colon slash slash example dot org slash a \
             dash b underline c? x equals 1 and y equals 2 number top"
        );
    }

    /// Periods, numbers, words and markers in running text at Most, as JAWS sent them.
    #[test]
    fn running_text_at_most() {
        let most = |text: &str| spoken(text, Punctuation::Most);
        // Role markers are not spoken; a run of periods is "dot dot dot" [obs, 306/s3].
        assert_eq!(
            most("Save... \u{1d}Button\u{1e}"),
            "Save dot dot dot Button"
        );
        // A period before a word [obs, 323/s6].
        assert_eq!(
            most("Type of file: Text Document (.txt)"),
            "Type of file colon Text Document left paren dot txt right paren"
        );
        // Words split by case and from digits; JAWS's respelling [obs, 205/s2, 227/s1, 306/s1].
        assert_eq!(most("MainRegion"), "Main Region");
        assert_eq!(most("List1 List view"), "List 1 List view");
        assert_eq!(
            most("To activate press spacebar."),
            "To activate press space Bar."
        );
        // A dashed number digit by digit; a time, a decimal and a percent left alone
        // [obs, 239/s3, 219/s6].
        assert_eq!(most("call 555-0142."), "call 5 5 5 dash 0 1 4 2.");
        assert_eq!(most("09:00 Read"), "09:00 Read");
        assert_eq!(most("rose 3.5% last year"), "rose 3.5% last year");
        // An apostrophe inside a word stays; an em dash above the level is a pause only
        // [obs, 229/s22, 239/s3].
        assert_eq!(most("Chrome'u"), "Chrome'u");
        assert_eq!(most("help — or call"), "help or call");
        // Symbols named at every level [obs, 313/s4].
        assert_eq!(
            most("\u{20ac}\u{2014}\u{2026}\u{2022}"),
            "euro sign \u{2026} bullet"
        );
    }

    /// A selection names every symbol, numbers as at the lower levels [obs, 313/s7, 322/s16].
    #[test]
    fn a_selection_names_every_symbol() {
        assert_eq!(
            spoken_symbols("a !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~ B"),
            "a exclaim! quote number dollar percent and apostrophe left paren right paren star \
             plus comma, dash period. slash colon semi colon less equals greater question? at \
             left bracket back slash right bracket carat underline grawv left brace vertical bar \
             right brace til de B"
        );
        assert_eq!(spoken_symbols("photo.jpg 1.2 MB"), "photo dot jpg 1.2 MB");
        assert_eq!(spoken_symbols("dog,"), "dog comma,");
    }

    #[test]
    fn capitals_read_alone_are_raised() {
        assert_eq!(character_pitch("B"), CAPITAL_PITCH);
        assert_eq!(character_pitch("\u{130}"), CAPITAL_PITCH);
        assert_eq!(character_pitch("b"), 0);
        assert_eq!(character_pitch("("), 0);
        assert_eq!(character_pitch("Bo"), 0);
        assert_eq!(pitched("B", 4), "[pitch +4] B");
        assert_eq!(pitched("b", 0), "b");
    }
}
