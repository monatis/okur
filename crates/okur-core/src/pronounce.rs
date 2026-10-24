//! What the synthesizer is asked to say, as opposed to what JAWS's speech history records.
//!
//! JAWS keeps the text it decided to speak in its speech history (`(`, or an empty entry for
//! a space) and only then applies its symbol processing: reading a character names it ("left
//! paren", "space") whatever the punctuation level. The engine produces history-level text, so
//! parity compares it with the speech history, and this module turns it into spoken text for
//! the synthesizer and for comparison with what JAWS sent its synthesizer (the speech
//! recorder voice's log, `observation.spoken`).

use crate::settings::{Punctuation, TextProcessing};

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
/// [obs, 406/s5, s7: "[pitch +4] A d a space [pitch +4] R [pitch +4] E ...]. Without Caps
/// Indicate a capital is still a part of its own, at the voice's pitch [obs, 717/s3, s6].
pub fn spelled_parts(text: &str, indicate_caps: bool) -> Vec<(String, i32)> {
    let mut parts = Vec::new();
    let mut run: Vec<String> = Vec::new();
    for c in text.chars() {
        let one = c.to_string();
        if character_pitch(&one, true) == 0 {
            run.push(character(&one));
            continue;
        }
        if !run.is_empty() {
            parts.push((run.join(" "), 0));
            run.clear();
        }
        parts.push((character(&one), character_pitch(&one, indicate_caps)));
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

/// The pitch change of a character read alone: capitals are raised while capitals are
/// indicated (Caps Indicate, `IndicateCaps`, on by default); off, a capital is said at the
/// voice's pitch [obs, 717/s3].
pub fn character_pitch(text: &str, indicate_caps: bool) -> i32 {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if indicate_caps && c.is_uppercase() => CAPITAL_PITCH,
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
    spoken_with(text, level, TextProcessing::default())
}

/// [`spoken`] with Text Processing set otherwise than JAWS's defaults: mixed case left whole,
/// numbers digit by digit, dollars and euros left to the synthesizer [obs, 721-723].
pub fn spoken_with(text: &str, level: Punctuation, processing: TextProcessing) -> String {
    render(
        text,
        level,
        level == Punctuation::All || processing.numbers_as_digits,
        processing,
    )
}

/// Text for the synthesizer with every symbol named, whatever the level: a selection and a
/// word read on its own ("Friday comma, January 2 comma," [obs, 323/s7]; "dog comma,"
/// [obs, 213/s11]). Numbers stay as at the lower levels: "photo dot jpg 1.2 MB" after Ctrl+A
/// [obs, 322/s16].
pub fn spoken_symbols(text: &str) -> String {
    spoken_symbols_with(text, TextProcessing::default())
}

/// [`spoken_symbols`] with the voice's text processing: mixed case, numbers as digits and
/// dollar amounts as the settings have them, as for any text [obs, 721, 722, 723: words read
/// by Ctrl+Right].
pub fn spoken_symbols_with(text: &str, processing: TextProcessing) -> String {
    // Numbers digit by digit say a decimal point as "point": "2 point 5" [obs, 722/s20].
    render(
        text,
        Punctuation::All,
        processing.numbers_as_digits,
        processing,
    )
}

/// `spoken`'s work; `point_digits`: a decimal point is "point" and the digits after it are
/// read one by one, as at All.
fn render(
    text: &str,
    level: Punctuation,
    point_digits: bool,
    processing: TextProcessing,
) -> String {
    let digits = processing.numbers_as_digits;
    let text = single_digits(
        &split_words(
            &respelled(&repeats_filtered(&text.replace(['\u{1d}', '\u{1e}'], " "))),
            processing.mixed_case,
        ),
        digits,
    );
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
        // Dollars and euros left to the synthesizer [obs, 723/s23, s29].
        if processing.say_dollars && matches!(c, '$' | '\u{20ac}') {
            out.push(c);
            continue;
        }
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
            // Numbers digit by digit name the colon and the comma between digits, capitalized
            // as JAWS gives them [obs, 722/s21, s22].
            ':' if digits && is_digit(prev) && is_digit(next) => word(&mut out, "Colon"),
            ',' if digits && is_digit(prev) && is_digit(next) => word(&mut out, "Comma"),
            ':' if is_digit(prev) && is_digit(next) => out.push(c),
            // A thousands separator is part of the number, never named: "1,000,000", "dollar
            // 1,250.50" [obs, 405/s14, s15; 407/s21].
            ',' if is_digit(prev) && is_digit(next) => out.push(c),
            // An apostrophe below its level is passed on as it is, inside a word or among
            // symbols ("and ' left paren" [obs, 316/s3, 312/s1]).
            '\'' if !named => out.push(c),
            '\u{2019}'
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

/// Filter Repeated Characters at JAWS's default, Three: of an unbroken run of one symbol, three
/// are spoken ("=====" as "equals equals equals" [obs, 285/s12]). Letters, digits and spaces
/// are left alone.
fn repeats_filtered(text: &str) -> String {
    const KEPT: usize = 3;
    let mut out = String::with_capacity(text.len());
    let mut run: Option<(char, usize)> = None;
    for c in text.chars() {
        let count = match run {
            Some((previous, count)) if previous == c => count + 1,
            _ => 1,
        };
        run = Some((c, count));
        if count <= KEPT || c.is_alphanumeric() || c.is_whitespace() {
            out.push(c);
        }
    }
    out
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
/// With `mixed_case` off, only letters and digits are split [obs, 721/s1, s7].
fn split_words(text: &str, mixed_case: bool) -> String {
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
            if (mixed_case && p.is_lowercase() && c.is_uppercase())
                || (mixed_case && run_ends)
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

/// Numbers of five or more digits, and numbers with dashes, digit by digit; with `all`,
/// every number but an ordinal ("1st" [obs, 722/s11]).
fn single_digits(text: &str, all: bool) -> String {
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
        let ordinal = is_ordinal_ending(&chars, i);
        if number.contains('-') || digits >= 5 || (all && !ordinal) {
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
        let parts = spelled_parts("Ada RE", true);
        assert_eq!(
            parts,
            [
                ("A".to_owned(), CAPITAL_PITCH),
                ("d a space".to_owned(), 0),
                ("R".to_owned(), CAPITAL_PITCH),
                ("E".to_owned(), CAPITAL_PITCH),
            ]
        );
        // Without Caps Indicate, the same parts at the voice's pitch [obs, 717/s3, s6].
        let plain: Vec<i32> = spelled_parts("Ada RE", false)
            .into_iter()
            .map(|(_, pitch)| pitch)
            .collect();
        assert_eq!(plain, [0, 0, 0, 0]);
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
        assert_eq!(most("&'("), "and ' left paren");
        // Of a run of one symbol, three are spoken [obs, 285/s12].
        assert_eq!(
            most("===== *** ....."),
            "equals equals equals star star star dot dot dot"
        );
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

    /// A word read on its own names every symbol with the voice's text processing kept: numbers
    /// digit by digit with "point" [obs, 722/s20, s26].
    #[test]
    fn a_word_keeps_the_text_processing() {
        let digits = TextProcessing {
            numbers_as_digits: true,
            ..TextProcessing::default()
        };
        assert_eq!(spoken_symbols_with("2.5", digits), "2 point 5");
        assert_eq!(spoken_symbols_with("$0.02", digits), "dollar 0 point 0 2");
        assert_eq!(spoken_symbols("2.5"), "2.5");
    }

    /// Text Processing set otherwise than JAWS's defaults: mixed case whole, numbers digit by
    /// digit, dollars and euros left to the synthesizer [obs, 721-723 against 407].
    #[test]
    fn text_processing_set_otherwise() {
        let with = |text: &str, processing: TextProcessing| {
            spoken_with(text, Punctuation::Most, processing)
        };
        let plain = TextProcessing::default();
        assert_eq!(with("getElementById aB9", plain), "get Element By Id a B 9");
        let whole = TextProcessing {
            mixed_case: false,
            ..plain
        };
        assert_eq!(
            with("getElementById System32", whole),
            "getElementById System 32"
        );
        let digits = TextProcessing {
            numbers_as_digits: true,
            ..plain
        };
        assert_eq!(with("42 2.5 1st", digits), "4 2 2 point 5 1st");
        assert_eq!(
            with("1,000,000 10:30", digits),
            "1 Comma 0 0 0 Comma 0 0 0 1 0 Colon 3 0"
        );
        assert_eq!(with("$1,250.50", digits), "dollar 1 Comma 2 5 0 point 5 0");
        let dollars = TextProcessing {
            say_dollars: true,
            ..plain
        };
        assert_eq!(with("$0.02 €", dollars), "$0.02 €");
        assert_eq!(with("$0.02 €", plain), "dollar 0.02 euro sign");
    }

    #[test]
    fn capitals_read_alone_are_raised() {
        assert_eq!(character_pitch("B", true), CAPITAL_PITCH);
        assert_eq!(character_pitch("\u{130}", true), CAPITAL_PITCH);
        assert_eq!(character_pitch("b", true), 0);
        assert_eq!(character_pitch("(", true), 0);
        assert_eq!(character_pitch("Bo", true), 0);
        assert_eq!(character_pitch("B", false), 0);
        assert_eq!(pitched("B", 4), "[pitch +4] B");
        assert_eq!(pitched("b", 0), "b");
    }
}
