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
            Err(_) => read_uia(uia).or_else(read_win32_edit),
        }
    }

    /// A Win32 edit control read with window messages, as for subclassed edits that UIA's
    /// edit proxy does not recognize (WinForms `WindowsForms10.EDIT…`, rich edits).
    fn read_win32_edit() -> Option<TextContext> {
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            GetClassNameW, GetGUIThreadInfo, GetWindowThreadProcessId, SendMessageTimeoutW,
            GUITHREADINFO, SMTO_ABORTIFHUNG,
        };
        const WM_GETTEXT: u32 = 0x000D;
        const WM_GETTEXTLENGTH: u32 = 0x000E;
        const EM_GETSEL: u32 = 0x00B0;
        // SAFETY: Win32 queries with valid out-structures; messages to another process use
        // a timeout, and WM_GETTEXT's buffer is marshalled by the system.
        unsafe {
            let thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let mut info = GUITHREADINFO {
                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            GetGUIThreadInfo(thread, &mut info).ok()?;
            let hwnd: HWND = info.hwndFocus;
            let mut class = [0u16; 128];
            let n = usize::try_from(GetClassNameW(hwnd, &mut class)).ok()?;
            let class = String::from_utf16_lossy(class.get(..n)?).to_ascii_uppercase();
            if !class.contains("EDIT") {
                return None;
            }
            let ask = |msg: u32, wparam: usize, lparam: isize| {
                let mut result = 0usize;
                let sent = SendMessageTimeoutW(
                    hwnd,
                    msg,
                    WPARAM(wparam),
                    LPARAM(lparam),
                    SMTO_ABORTIFHUNG,
                    200,
                    Some(&mut result),
                );
                (sent.0 != 0).then_some(result)
            };
            let len = ask(WM_GETTEXTLENGTH, 0, 0)?.min(MAX_TEXT_CHARS as usize);
            let mut buf = vec![0u16; len + 1];
            let copied = ask(WM_GETTEXT, buf.len(), buf.as_mut_ptr() as isize)?;
            let text = String::from_utf16_lossy(buf.get(..copied.min(len))?);
            // RichEdit returns `\r\n` to WM_GETTEXT but counts a line break as one character
            // in EM_GETSEL: give the text its own bare `\r` breaks so offsets line up.
            let text = if class.contains("RICHEDIT") {
                text.replace("\r\n", "\r")
            } else {
                text
            };
            // EM_GETSEL's result packs the selection's start and end in its low and high words.
            let sel = ask(EM_GETSEL, 0, 0)?;
            let start = i32::try_from(sel & 0xFFFF).ok()?;
            let end = i32::try_from((sel >> 16) & 0xFFFF).ok()?;
            Some(TextContext {
                caret: utf16_to_chars(&text, start),
                selection: (start < end)
                    .then(|| (utf16_to_chars(&text, start), utf16_to_chars(&text, end))),
                text,
            })
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
