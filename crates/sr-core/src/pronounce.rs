//! What the synthesizer is asked to say, as opposed to what JAWS's speech history records.
//!
//! JAWS keeps the text it decided to speak in its speech history (`(`, or an empty entry for
//! a space) and only then applies its symbol processing: reading a character names it ("left
//! paren", "space") whatever the punctuation level. The engine produces history-level text, so
//! parity compares it with the speech history, and this module turns it into spoken text for
//! the synthesizer and for comparison with what JAWS sent its synthesizer (the speech
//! recorder voice's log, `observation.spoken`).

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

/// Text for the synthesizer: the role markers JAWS's history keeps (`\u{1d}…\u{1e}`) are
/// not spoken characters.
pub fn text(text: &str) -> String {
    text.replace(['\u{1d}', '\u{1e}'], " ")
}

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
        assert_eq!(text("Save... \u{1d}Button\u{1e}"), "Save...  Button ");
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
