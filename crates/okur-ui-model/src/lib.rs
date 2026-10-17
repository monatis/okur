//! What Okur and `okur-ui`, the process that draws Okur's windows, say to each other.
//!
//! Okur decides what is shown; `okur-ui` only draws it, natively on each platform, and tells
//! Okur what the user did to a window (see `docs/references/helper-ui-architecture.md`).
//! Messages are JSON, one per line: Okur writes them to the process's standard input, and
//! reads its answers from its standard output.

use serde::{Deserialize, Serialize};

/// A span of the viewer's text: `length` characters from `column` of `line`. An empty span
/// marks a position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSpan {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

/// What Okur asks `okur-ui` to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToUi {
    /// Show the Virtual Viewer's text on screen, without taking the focus, with `mark` on
    /// what the virtual cursor read last. Okur keeps the keys: the window only shows them
    /// to someone watching, as JAWS's does.
    ViewerShow {
        title: String,
        lines: Vec<String>,
        mark: TextSpan,
    },
    /// Move the viewer's mark.
    ViewerMark { mark: TextSpan },
    /// Close the viewer's window.
    ViewerClose,
}

/// What `okur-ui` tells Okur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromUi {
    /// The viewer's window took the focus (a click, Alt+Tab) or was closed by its close
    /// button. JAWS ends its viewer then, and so does Okur.
    ViewerDismissed,
}

/// One message as a line of JSON, without the line break.
pub fn encode<T: Serialize>(message: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(message)
}

/// The message on one line of JSON.
pub fn decode<'a, T: Deserialize<'a>>(line: &'a str) -> Result<T, serde_json::Error> {
    serde_json::from_str(line.trim_end())
}

/// Where `column` (in characters) of `line` falls in `lines` joined by one line break each,
/// counted in UTF-16 units: the positions of a Windows rich edit control holding them.
/// Positions past a line's end stop at its end; past the last line, at the text's end.
pub fn utf16_offset(lines: &[String], line: usize, column: usize) -> usize {
    let before: usize = lines
        .iter()
        .take(line)
        .map(|l| l.encode_utf16().count() + 1)
        .sum();
    let within = lines.get(line).map_or(0, |l| {
        l.chars().take(column).map(char::len_utf16).sum::<usize>()
    });
    if line >= lines.len() {
        before.saturating_sub(1)
    } else {
        before + within
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_one_line_of_json_each_way() {
        let show = ToUi::ViewerShow {
            title: "Virtual Viewer".into(),
            lines: vec!["About Windows".into(), "OK".into()],
            mark: TextSpan {
                line: 0,
                column: 0,
                length: 13,
            },
        };
        let line = encode(&show).unwrap_or_default();
        assert!(!line.contains('\n'));
        assert!(line.starts_with(r#"{"type":"viewer_show""#), "{line}");
        assert_eq!(decode::<ToUi>(&format!("{line}\r\n")).ok(), Some(show));
        assert_eq!(
            decode::<FromUi>(r#"{"type":"viewer_dismissed"}"#).ok(),
            Some(FromUi::ViewerDismissed)
        );
    }

    #[test]
    fn offsets_count_utf16_units_and_one_break_per_line() {
        let lines = vec!["ab".to_owned(), "çağ 𝄞x".to_owned(), String::new()];
        assert_eq!(utf16_offset(&lines, 0, 0), 0);
        assert_eq!(utf16_offset(&lines, 0, 2), 2);
        // Past the line's end: its end.
        assert_eq!(utf16_offset(&lines, 0, 9), 2);
        assert_eq!(utf16_offset(&lines, 1, 0), 3);
        // "𝄞" is two UTF-16 units.
        assert_eq!(utf16_offset(&lines, 1, 5), 3 + 6);
        assert_eq!(utf16_offset(&lines, 2, 0), 3 + 7 + 1);
        // Past the last line: the text's end.
        assert_eq!(utf16_offset(&lines, 3, 0), 3 + 7 + 1);
    }
}
