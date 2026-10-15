//! Live regions that grow (a streamed chat reply): which part of the region's text to speak.
//!
//! A region remembers how much of its text was spoken. Each change speaks what follows, up
//! to the last whole word: a reply streamed in pieces splits words ("arr", "ives"), and the
//! last word may still grow. What is held back is spoken when the region goes quiet.

/// A character inside a word: a piece that ends on one may continue the word.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '\'' | '\u{2019}' | '_')
}

/// Where the word around byte `at` starts, when `at` falls inside a word (the text before it
/// ends with a word character and the text from it starts with one); else `at` itself.
pub fn word_start(text: &str, at: usize) -> usize {
    let Some((before, after)) = text.split_at_checked(at) else {
        return at;
    };
    let inside = before.chars().next_back().is_some_and(is_word_char)
        && after.chars().next().is_some_and(is_word_char);
    if !inside {
        return at;
    }
    before
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word_char(*c))
        .last()
        .map_or(at, |(i, _)| i)
}

/// Where the text after byte `from` can be spoken up to now: its end when it ends between
/// words, else the start of its last word, which may still grow.
pub fn complete_end(text: &str, from: usize) -> usize {
    let Some(rest) = text.get(from..) else {
        return from;
    };
    if !rest.chars().next_back().is_some_and(is_word_char) {
        return text.len();
    }
    let trailing_word = rest
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word_char(*c))
        .last()
        .map_or(rest.len(), |(i, _)| i);
    from + trailing_word
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piece_that_ends_inside_a_word_resumes_from_the_word_start() {
        let text = "This reply arrives";
        // Spoken up to "This reply arr": the next piece starts at "arrives".
        assert_eq!(
            word_start(text, "This reply arr".len()),
            "This reply ".len()
        );
        // Spoken up to a space: nothing to take back.
        assert_eq!(word_start(text, "This reply ".len()), "This reply ".len());
        assert_eq!(word_start(text, 0), 0);
    }

    #[test]
    fn the_last_word_waits_until_it_is_whole() {
        let text = "This reply arrives a fe";
        assert_eq!(complete_end(text, 0), "This reply arrives a ".len());
        let finished = "This reply arrives a few.";
        assert_eq!(complete_end(finished, 0), finished.len());
        // Only a partial word so far: nothing yet.
        assert_eq!(complete_end("Th", 0), 0);
        // Words split by an apostrophe stay one word.
        assert_eq!(complete_end("I don", 0), "I ".len());
        assert_eq!(word_start("I don't", "I don".len()), "I ".len());
    }

    #[test]
    fn non_ascii_text_is_cut_on_character_boundaries() {
        let text = "Günaydın dünya";
        let end = complete_end(text, 0);
        assert_eq!(text.get(..end), Some("Günaydın "));
        assert_eq!(word_start(text, "Günaydın dün".len()), "Günaydın ".len());
    }
}
