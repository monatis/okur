//! Keeps people's own data out of what the harness records. The corpus is public, and the
//! reference machine's browser may show its user's account: Chrome's new tab page named the
//! signed-in Google account, with its e-mail address, in a trace of scenario 220 (2026-10-19).

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
                out.push_str("<address>");
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
