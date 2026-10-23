//! Screen Echo: what an application wrote to its screen since it was last read, as JAWS reads a
//! console's output after Enter, "Output line 1 for a | Second output line 1 | Command:"
//! [obs, 962/s2, 964/s2], and nothing with Screen Echo off [obs, 963/s2].
//!
//! The screen is compared by rows. Its new text is the rows after the longest stretch of the
//! old rows' end that the new screen starts with (the rows above it scrolled away). The
//! stretch's last row may have changed: it is the row being typed on, and what is typed is
//! not output (typing "a" at the prompt said nothing [obs, 962/s1]).

/// The rows `after` holds below those it shares with `before`; none when the screen did not
/// change, and all of `after` when it shares none (the screen was cleared). `before` empty:
/// nothing is known to compare with, and nothing is new.
pub fn new_rows<'a>(before: &[String], after: &'a [String]) -> &'a [String] {
    let Some(last) = before.last() else {
        return &[];
    };
    // The fewest rows scrolled away first: the longest stretch.
    for scrolled in 0..before.len() {
        let shared = before.len() - scrolled;
        if shared > after.len() {
            continue;
        }
        let (Some(kept), Some(above)) = (
            before.get(scrolled..before.len() - 1),
            after.get(..shared - 1),
        ) else {
            continue;
        };
        let typed_on = after
            .get(shared - 1)
            .is_some_and(|row| row == last || row.starts_with(last.as_str()));
        if kept == above && typed_on {
            return after.get(shared..).unwrap_or_default();
        }
    }
    after
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &[&str]) -> Vec<String> {
        text.iter().map(|row| (*row).to_owned()).collect()
    }

    #[test]
    fn a_commands_output_and_the_next_prompt_are_new() {
        let before = rows(&["Console echo fixture", "Command: a"]);
        let after = rows(&[
            "Console echo fixture",
            "Command: a",
            "Output line 1 for a",
            "Second output line 1",
            "Command:",
        ]);
        assert_eq!(
            new_rows(&before, &after),
            ["Output line 1 for a", "Second output line 1", "Command:"]
        );
    }

    #[test]
    fn typing_at_the_prompt_writes_nothing_new() {
        let before = rows(&["Console echo fixture", "Command:"]);
        let after = rows(&["Console echo fixture", "Command: a"]);
        assert!(new_rows(&before, &after).is_empty());
        assert!(new_rows(&after, &after).is_empty());
    }

    #[test]
    fn rows_scrolled_away_are_not_new() {
        let before = rows(&["one", "two", "three", "Command: b"]);
        let after = rows(&["three", "Command: b", "four", "Command:"]);
        assert_eq!(new_rows(&before, &after), ["four", "Command:"]);
    }

    #[test]
    fn a_cleared_screen_is_new_and_nothing_known_is_nothing_new() {
        let before = rows(&["one", "Command: cls"]);
        let after = rows(&["Command:"]);
        assert_eq!(new_rows(&before, &after), ["Command:"]);
        assert!(new_rows(&[], &after).is_empty());
    }
}
