//! speech-dispatcher's protocol (SSIP, its `doc/ssip.texi`): the scales Okur's voice settings are
//! given in, the text of a message, and the lines the server answers with. A text protocol over
//! a Unix socket: commands are lines ending CR LF, replies are lines "<code>-<text>" followed by
//! a last "<code> <text>", and notifications (7xx) come the same way between replies.

/// SSIP's rate or pitch for Okur's: SSIP's run from -100 to 100 around the voice's own (0),
/// Okur's, as SAPI's, from -10 to 10. The ends and the middle meet; between them each Okur step
/// is ten of SSIP's. How fast a step sounds is each output module's own (espeak-ng's is not
/// SAPI's), to be measured against a SAPI voice before it is tuned.
pub fn ssip_scale(okur: i32) -> i32 {
    okur.clamp(-10, 10) * 10
}

/// SSIP's volume, -100 to 100, for Okur's, 0 to 100.
pub fn ssip_volume(okur: u16) -> i32 {
    i32::from(okur.min(100)) * 2 - 100
}

/// A message's text as SSML, which speech-dispatcher takes with `SSML_MODE on`: the text
/// escaped, a mark before it when it is one part of a reading, so the server says when the
/// part begins. Okur's markers around a role or a state (U+001D, U+001E), which SAPI and OneCore
/// leave out too, and the other control characters XML does not allow are left out.
pub fn ssml(text: &str, mark: Option<u64>) -> String {
    let mut out = String::with_capacity(text.len() + 32);
    out.push_str("<speak>");
    if let Some(mark) = mark {
        out.push_str(&format!("<mark name=\"{mark}\"/>"));
    }
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.push_str("</speak>");
    out
}

/// SPEAK's data: each line with CR LF, a line that starts with "." given another, and the end a
/// line holding "." alone.
pub fn speak_data(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for line in text.lines() {
        if line.starts_with('.') {
            out.push('.');
        }
        out.push_str(line);
        out.push_str("\r\n");
    }
    out.push_str(".\r\n");
    out
}

/// One line the server sent: its code, whether more lines of the same reply follow, and its
/// text.
pub fn parse_line(line: &str) -> Option<(u16, bool, &str)> {
    let line = line.trim_end_matches(['\r', '\n']);
    let code = line.get(..3)?.parse().ok()?;
    let more = match line.get(3..4) {
        Some("-") => true,
        Some(" ") | None => false,
        Some(_) => return None,
    };
    Some((code, more, line.get(4..).unwrap_or("")))
}

/// What a notification says of the messages being spoken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notification {
    Begin,
    End,
    Cancel,
    Pause,
    Resume,
    /// An index mark reached: the mark's name.
    Mark(String),
}

/// The notification a reply of code 700 to 705 is; `lines` are its lines before the last (the
/// message id, the client id, and for a mark its name).
pub fn notification(code: u16, lines: &[String]) -> Option<Notification> {
    Some(match code {
        700 => Notification::Mark(lines.get(2)?.clone()),
        701 => Notification::Begin,
        702 => Notification::End,
        703 => Notification::Cancel,
        704 => Notification::Pause,
        705 => Notification::Resume,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn okurs_scale_meets_ssips_at_the_ends_and_the_middle() {
        assert_eq!(ssip_scale(-10), -100);
        assert_eq!(ssip_scale(0), 0);
        assert_eq!(ssip_scale(10), 100);
        assert_eq!(ssip_scale(3), 30);
    }

    #[test]
    fn a_value_past_okurs_scale_is_held_to_its_end() {
        assert_eq!(ssip_scale(25), 100);
        assert_eq!(ssip_scale(-11), -100);
    }

    #[test]
    fn the_volume_runs_from_ssips_bottom_to_its_top() {
        assert_eq!(ssip_volume(0), -100);
        assert_eq!(ssip_volume(50), 0);
        assert_eq!(ssip_volume(100), 100);
        assert_eq!(ssip_volume(150), 100);
    }

    #[test]
    fn ssml_escapes_the_text_and_marks_a_part() {
        assert_eq!(ssml("a < b & c", None), "<speak>a &lt; b &amp; c</speak>");
        assert_eq!(
            ssml("Hello", Some(7)),
            "<speak><mark name=\"7\"/>Hello</speak>"
        );
    }

    /// A focus's speech carries Okur's markers around its role and state ("Subscribe to
    /// updates ‹check box› ‹checked›"); XML allows neither, nor the other control characters.
    #[test]
    fn ssml_leaves_out_okurs_markers_and_control_characters() {
        assert_eq!(
            ssml("OK \u{1d}Button\u{1e}\nTo activate\u{7}", None),
            "<speak>OK Button\nTo activate</speak>"
        );
    }

    #[test]
    fn speak_data_doubles_a_leading_dot_and_ends_with_one() {
        assert_eq!(speak_data("one\n.two"), "one\r\n..two\r\n.\r\n");
    }

    #[test]
    fn reply_lines_and_notifications() {
        assert_eq!(parse_line("225-21\r\n"), Some((225, true, "21")));
        assert_eq!(
            parse_line("225 OK MESSAGE QUEUED"),
            Some((225, false, "OK MESSAGE QUEUED"))
        );
        assert_eq!(parse_line("2x5 nonsense"), None);
        let lines = vec!["21".to_owned(), "3".to_owned(), "7".to_owned()];
        assert_eq!(
            notification(700, &lines),
            Some(Notification::Mark("7".to_owned()))
        );
        assert_eq!(notification(702, &lines), Some(Notification::End));
        assert_eq!(notification(225, &lines), None);
    }
}
