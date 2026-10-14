//! The focused text field's text, caret and selection ([`sr_core::TextContext`]).
//!
//! The tracer records it with events, and the live worker reads it after edit keys, so
//! replay and live use the same data. Chromium pages are read through IA2, because
//! Chromium's UIA selection does not follow the caret in form fields. Other applications
//! are read through UIA TextPattern.

/// Characters read from a field at most (a whole document in a word processor).
pub const MAX_TEXT_CHARS: i32 = 100_000;

/// Converts a UTF-16 offset into `text` to a character offset.
pub fn utf16_to_chars(text: &str, offset: i32) -> usize {
    let target = usize::try_from(offset).unwrap_or(0);
    let mut units = 0usize;
    for (i, c) in text.chars().enumerate() {
        if units >= target {
            return i;
        }
        units += c.len_utf16();
    }
    text.chars().count()
}

#[cfg(windows)]
pub use reader::read_focused;

#[cfg(windows)]
mod reader {
    use super::{utf16_to_chars, MAX_TEXT_CHARS};
    use sr_core::TextContext;
    use windows::Win32::UI::Accessibility::{
        IUIAutomation, IUIAutomationTextPattern, IUIAutomationTextRange,
        TextPatternRangeEndpoint_End, TextPatternRangeEndpoint_Start, UIA_TextPatternId,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    /// The focused field's text context, or `None` when the focus has no text.
    pub fn read_focused(uia: &IUIAutomation) -> Option<TextContext> {
        // SAFETY: plain Win32 call with no preconditions.
        let window = unsafe { GetForegroundWindow() };
        match xa11y_windows::ia2::chromium_text_state(window) {
            Ok(Some(state)) => Some(TextContext {
                caret: utf16_to_chars(&state.text, state.caret),
                selection: state.selection.map(|(s, e)| {
                    (
                        utf16_to_chars(&state.text, s),
                        utf16_to_chars(&state.text, e),
                    )
                }),
                text: state.text,
            }),
            // A Chromium page whose focused object has no text.
            Ok(None) => None,
            // Not a Chromium window.
            Err(_) => read_uia(uia),
        }
    }

    /// Characters from the start of `document` to `endpoint` of `range`.
    fn offset_of(
        document: &IUIAutomationTextRange,
        range: &IUIAutomationTextRange,
        endpoint: windows::Win32::UI::Accessibility::TextPatternRangeEndpoint,
    ) -> Option<usize> {
        // SAFETY: UIA text range calls on valid ranges of one provider.
        unsafe {
            let prefix = document.Clone().ok()?;
            prefix
                .MoveEndpointByRange(TextPatternRangeEndpoint_End, range, endpoint)
                .ok()?;
            Some(
                prefix
                    .GetText(MAX_TEXT_CHARS)
                    .ok()?
                    .to_string()
                    .chars()
                    .count(),
            )
        }
    }

    fn read_uia(uia: &IUIAutomation) -> Option<TextContext> {
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let focused = uia.GetFocusedElement().ok()?;
            let pattern: IUIAutomationTextPattern =
                focused.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
            let document = pattern.DocumentRange().ok()?;
            let text = document.GetText(MAX_TEXT_CHARS).ok()?.to_string();
            let selection = pattern.GetSelection().ok()?.GetElement(0).ok()?;
            let start = offset_of(&document, &selection, TextPatternRangeEndpoint_Start)?;
            let end = offset_of(&document, &selection, TextPatternRangeEndpoint_End)?;
            Some(TextContext {
                text,
                caret: start,
                selection: (start < end).then_some((start, end)),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_offsets_become_character_offsets() {
        assert_eq!(utf16_to_chars("abc", 2), 2);
        // "𝄞" is two UTF-16 units and one character.
        assert_eq!(utf16_to_chars("𝄞ab", 2), 1);
        assert_eq!(utf16_to_chars("ab", 10), 2);
    }
}
