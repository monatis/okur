//! Control texts as Windows gives them.

/// A control's text without its keyboard accelerator marks (`&Open:` is shown as `Open:`;
/// `&&` is a literal ampersand).
pub fn without_accelerators(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                out.push('&');
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}
