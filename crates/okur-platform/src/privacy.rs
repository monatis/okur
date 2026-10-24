//! Keeps people's own data out of what the harness records. The corpus is public, and the
//! reference machine's browser may show its user's account: Chrome's new tab page named the
//! signed-in Google account, with its e-mail address, in a trace of scenario 220 (2026-10-19).

/// Whether `event` is from a web view the focus is not in: Windows' taskbar news feed renders
/// in one (msedgewebview2.exe), and its headlines and links changed in a trace of scenario 1033
/// while File Explorer had the focus (round 298). That is the internet's content, never a
/// fixture's; a web view app in front has its focus chain, and is kept.
pub fn internet_feed(event: &crate::event_trace::TraceEvent) -> bool {
    event.process.as_deref() == Some("msedgewebview2.exe") && event.focus_chain.is_empty()
}

/// What an e-mail address is replaced with.
pub const ADDRESS_MARK: &str = "<address>";

/// `text` with every e-mail address replaced by `<address>`, except addresses at the domains
/// reserved for examples (RFC 2606: `example.com`, `example.net`, `example.org`, and names
/// under `.example`, `.test`, `.invalid`, `.localhost`), which the fixtures use and replay
/// must see as they are.
pub fn redact_addresses(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if c == '@' {
            let local = out.chars().rev().take_while(|c| is_local(*c)).count();
            let domain: String = chars
                .iter()
                .skip(i + 1)
                .take_while(|c| is_domain(**c))
                .collect();
            // A sentence's period after the address is not part of it.
            let domain = domain.trim_end_matches('.');
            if local > 0 && domain.contains('.') && !domain.starts_with('.') && !reserved(domain) {
                for _ in 0..local {
                    out.pop();
                }
                out.push_str(ADDRESS_MARK);
                i += 1 + domain.chars().count();
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn is_local(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-')
}

fn is_domain(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-')
}

fn reserved(domain: &str) -> bool {
    let domain = domain.to_ascii_lowercase();
    ["example.com", "example.net", "example.org"]
        .iter()
        .any(|d| domain == *d || domain.ends_with(&format!(".{d}")))
        || [".example", ".test", ".invalid", ".localhost"]
            .iter()
            .any(|tld| domain.ends_with(tld))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The taskbar's news feed in its web view is left out; a web view the focus is in stays.
    #[test]
    fn a_web_view_out_of_focus_is_left_out() {
        let feed = r#"{"t_ms":1,"event":"description_change","win_event":32781,"hwnd":1,"id_object":-4,"id_child":-7,"process":"msedgewebview2.exe","object":{"role":30,"name":"Markets","value":"https://www.example.com/markets","states":0}}"#;
        let event: Option<crate::event_trace::TraceEvent> = serde_json::from_str(feed).ok();
        assert_eq!(event.as_ref().map(internet_feed), Some(true));
        let focused = feed.replace(
            r#""process""#,
            r#""focus_chain":[{"control_type":50030,"localized_control_type":"document","name":"App","runtime_id":[42,1],"is_enabled":true,"is_offscreen":false,"is_password":false,"is_required_for_form":false,"is_keyboard_focusable":true,"has_keyboard_focus":true,"is_control_element":true,"is_content_element":true}],"process""#,
        );
        let event: Option<crate::event_trace::TraceEvent> = serde_json::from_str(&focused).ok();
        assert_eq!(event.as_ref().map(internet_feed), Some(false));
    }

    #[test]
    fn real_addresses_go_and_example_ones_stay() {
        assert_eq!(
            redact_addresses(
                r#"{"name":"Google Account: A. Person (a.person+x@mail.co.uk), Member"}"#
            ),
            r#"{"name":"Google Account: A. Person (<address>), Member"}"#
        );
        assert_eq!(
            redact_addresses("Mail ada@example.org or sam@example.com."),
            "Mail ada@example.org or sam@example.com."
        );
        assert_eq!(
            redact_addresses("Write to me@host.net."),
            "Write to <address>."
        );
        assert_eq!(
            redact_addresses("a@b, @handle, x@localhost"),
            "a@b, @handle, x@localhost"
        );
        assert_eq!(redact_addresses("dev@app.test"), "dev@app.test");
        assert_eq!(redact_addresses("ışık çağ"), "ışık çağ");
    }
}
