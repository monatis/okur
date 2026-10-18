//! The focused text field's text, caret and selection ([`okur_core::TextContext`]).
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

/// A CSS `text-align` as Say Font's alignments name it.
pub fn css_alignment(align: &str) -> Option<&'static str> {
    Some(match align {
        "left" | "start" => "left",
        "right" | "end" => "right",
        "center" => "centered",
        "justify" => "justified",
        _ => return None,
    })
}

#[cfg(windows)]
pub use reader::{
    focus_is_in_page, from_ia2, ia2_position, read_focused, read_focused_outside_page,
    with_paragraph_alignment,
};

#[cfg(windows)]
mod reader {
    use super::{utf16_to_chars, MAX_TEXT_CHARS};
    use okur_core::TextContext;
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
            // A field whose paragraphs are objects of their own (Google Docs with braille
            // support) is read as one text through UIA.
            Ok(Some(state))
                if state
                    .text
                    .contains(xa11y_windows::ia2::EMBEDDED_OBJECT_CHAR) =>
            {
                read_uia(uia)
                    .map(with_paragraph_alignment)
                    .or_else(|| Some(from_ia2(state)))
            }
            Ok(Some(state)) => Some(from_ia2(state)),
            // A Chromium page whose focused object has no text.
            Ok(None) => None,
            // Not a Chromium window.
            Err(_) => read_uia(uia).map(with_edit_font).or_else(read_win32_edit),
        }
    }

    /// Whether a document is above the focused element: a page's field (a browser's or an
    /// Electron app's), not the browser's own controls (its address bar).
    pub fn focus_is_in_page(uia: &IUIAutomation) -> bool {
        use windows::Win32::UI::Accessibility::UIA_DocumentControlTypeId;
        // SAFETY: UIA tree calls on valid objects.
        unsafe {
            let (Ok(walker), Ok(mut element)) = (uia.RawViewWalker(), uia.GetFocusedElement())
            else {
                return false;
            };
            for _ in 0..40 {
                if element.CurrentControlType() == Ok(UIA_DocumentControlTypeId) {
                    return true;
                }
                match walker.GetParentElement(&element) {
                    Ok(parent) => element = parent,
                    Err(_) => return false,
                }
            }
            false
        }
    }

    /// The focused field of an application other than a page: a browser's own controls (its
    /// address bar), whose page still names a focus of its own, which is not the one typed in.
    pub fn read_focused_outside_page(uia: &IUIAutomation) -> Option<TextContext> {
        read_uia(uia).map(with_edit_font).or_else(read_win32_edit)
    }

    /// `context` with a plain Win32 edit's own font (Notepad's on Windows 10): UIA's edit
    /// proxy gives its character cell's height for the size, 24 point for a Consolas of 22,
    /// where JAWS says "Font is Consolas 22 point", with no paragraph formatting [obs, 314/s1].
    fn with_edit_font(mut context: TextContext) -> TextContext {
        if let Some(font) = focused_window()
            .filter(|(_, class)| class == "EDIT")
            .and_then(|(hwnd, _)| win32_font(hwnd))
        {
            context.font = Some(font);
        }
        context
    }

    /// The focused window of the application in front, with its class in capitals.
    fn focused_window() -> Option<(windows::Win32::Foundation::HWND, String)> {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetClassNameW, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
        };
        // SAFETY: Win32 queries with valid out-structures.
        unsafe {
            let thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let mut info = GUITHREADINFO {
                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            GetGUIThreadInfo(thread, &mut info).ok()?;
            let mut class = [0u16; 128];
            let n = usize::try_from(GetClassNameW(info.hwndFocus, &mut class)).ok()?;
            let class = String::from_utf16_lossy(class.get(..n)?).to_ascii_uppercase();
            Some((info.hwndFocus, class))
        }
    }

    /// A Chromium text object's state, its UTF-16 offsets made character offsets. Its word
    /// is a page field's, which stops at punctuation as Chrome's word moves do.
    pub fn from_ia2(state: xa11y_windows::ia2::Ia2TextState) -> TextContext {
        let caret = utf16_to_chars(&state.text, state.caret);
        TextContext {
            caret,
            selection: state.selection.map(|(s, e)| {
                (
                    utf16_to_chars(&state.text, s),
                    utf16_to_chars(&state.text, e),
                )
            }),
            word: okur_core::editing::web_word_at(&state.text, caret),
            text: state.text,
            font: None,
        }
    }

    /// Where a context's caret and selection are, and its length, in UTF-16 code units: what
    /// a Chromium field is polled for while a key takes effect.
    pub fn ia2_position(context: &TextContext) -> xa11y_windows::ia2::Ia2TextPosition {
        let units = |chars: usize| -> i32 {
            let n: usize = context.text.chars().take(chars).map(char::len_utf16).sum();
            i32::try_from(n).unwrap_or(i32::MAX)
        };
        xa11y_windows::ia2::Ia2TextPosition {
            caret: units(context.caret),
            selection: context.selection.map(|(s, e)| (units(s), units(e))),
            length: units(usize::MAX),
        }
    }

    /// A Win32 edit control read with window messages, as for subclassed edits that UIA's
    /// edit proxy does not recognize (WinForms `WindowsForms10.EDIT…`, rich edits).
    fn read_win32_edit() -> Option<TextContext> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG};
        const WM_GETTEXT: u32 = 0x000D;
        const WM_GETTEXTLENGTH: u32 = 0x000E;
        const EM_GETSEL: u32 = 0x00B0;
        // SAFETY: Win32 queries with valid out-structures; messages to another process use
        // a timeout, and WM_GETTEXT's buffer is marshalled by the system.
        unsafe {
            let (hwnd, class) = focused_window()?;
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
                font: win32_font(hwnd),
                word: None,
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
            // The word at the caret by the field's own word unit.
            let word = selection.Clone().ok().and_then(|range| {
                use windows::Win32::UI::Accessibility::TextUnit_Word;
                range
                    .MoveEndpointByRange(
                        TextPatternRangeEndpoint_End,
                        &range,
                        TextPatternRangeEndpoint_Start,
                    )
                    .ok()?;
                range.ExpandToEnclosingUnit(TextUnit_Word).ok()?;
                Some((
                    offset_of(&document, &range, TextPatternRangeEndpoint_Start)?,
                    offset_of(&document, &range, TextPatternRangeEndpoint_End)?,
                ))
            });
            Some(TextContext {
                text,
                caret: start,
                selection: (start < end).then_some((start, end)),
                font: uia_font(&selection),
                word,
            })
        }
    }

    /// The font of the character at `range`'s start, from UIA text attributes. Attributes
    /// that vary over the range (UIA's "mixed" value) or are unsupported are left out.
    fn uia_font(range: &IUIAutomationTextRange) -> Option<okur_core::FontInfo> {
        use windows::Win32::System::Variant::VARIANT;
        use windows::Win32::UI::Accessibility::{
            TextUnit_Character, UIA_FontNameAttributeId, UIA_FontSizeAttributeId,
            UIA_FontWeightAttributeId, UIA_HorizontalTextAlignmentAttributeId,
            UIA_IsItalicAttributeId, UIA_UnderlineStyleAttributeId, UIA_TEXTATTRIBUTE_ID,
        };
        // SAFETY: UIA text range calls on a valid range.
        unsafe {
            let character = range.Clone().ok()?;
            let _ = character.ExpandToEnclosingUnit(TextUnit_Character);
            let get = |id: UIA_TEXTATTRIBUTE_ID| -> Option<VARIANT> {
                character.GetAttributeValue(id).ok()
            };
            let family = get(UIA_FontNameAttributeId)
                .and_then(|v| windows::core::BSTR::try_from(&v).ok())
                .map(|b| b.to_string())
                .filter(|f| !f.is_empty())?;
            let size = get(UIA_FontSizeAttributeId).and_then(|v| f64::try_from(&v).ok());
            let int = |id| get(id).and_then(|v| i32::try_from(&v).ok());
            let alignment = match int(UIA_HorizontalTextAlignmentAttributeId) {
                Some(0) => Some("left"),
                Some(1) => Some("centered"),
                Some(2) => Some("right"),
                Some(3) => Some("justified"),
                _ => None,
            };
            Some(okur_core::FontInfo {
                family,
                size_tenths: size
                    .map(|s| (s * 10.0).round())
                    .and_then(|s| u32::try_from(s as i64).ok())
                    .unwrap_or(0),
                bold: int(UIA_FontWeightAttributeId).is_some_and(|w| w >= 600),
                italic: get(UIA_IsItalicAttributeId)
                    .is_some_and(|v| bool::try_from(&v) == Ok(true)),
                underline: int(UIA_UnderlineStyleAttributeId).is_some_and(|u| u != 0),
                alignment: alignment.map(str::to_owned),
            })
        }
    }

    /// `context` with the alignment of the paragraph at its caret, when its font has none:
    /// Chromium's UIA reports no text alignment, and the paragraph's IA2 object attributes
    /// have it (`text-align`), as JAWS says it for Google Docs' document ("Paragraph
    /// formatting left justified", dev box 2026-10-16).
    pub fn with_paragraph_alignment(mut context: TextContext) -> TextContext {
        if context.font.as_ref().is_none_or(|f| f.alignment.is_some()) {
            return context;
        }
        // SAFETY: plain Win32 call with no preconditions.
        let window = unsafe { GetForegroundWindow() };
        let alignment = xa11y_windows::ia2::ChromiumTextField::focused(window)
            .ok()
            .flatten()
            .and_then(|field| field.caret_paragraph_attributes())
            .and_then(|attributes| attributes.get("text-align").cloned())
            .and_then(|align| super::css_alignment(&align))
            .map(str::to_owned);
        if let Some(font) = context.font.as_mut() {
            font.alignment = alignment;
        }
        context
    }

    /// The font a Win32 edit draws with (`WM_GETFONT`; GDI font handles are valid across the
    /// session), in points at the screen's DPI.
    fn win32_font(hwnd: windows::Win32::Foundation::HWND) -> Option<okur_core::FontInfo> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::Graphics::Gdi::{
            GetDC, GetDeviceCaps, GetObjectW, ReleaseDC, HFONT, LOGFONTW, LOGPIXELSY,
        };
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG};
        const WM_GETFONT: u32 = 0x0031;
        // SAFETY: a query message with a timeout, then GDI queries with valid buffers.
        unsafe {
            let mut result = 0usize;
            let sent = SendMessageTimeoutW(
                hwnd,
                WM_GETFONT,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                200,
                Some(&mut result),
            );
            if sent.0 == 0 || result == 0 {
                return None;
            }
            let font = HFONT(result as *mut core::ffi::c_void);
            let mut logfont = LOGFONTW::default();
            let size = i32::try_from(std::mem::size_of::<LOGFONTW>()).ok()?;
            if GetObjectW(
                font.into(),
                size,
                Some((&mut logfont as *mut LOGFONTW).cast()),
            ) == 0
            {
                return None;
            }
            // The window's own DPI: this process is not DPI-aware, and the screen's reads 96
            // at old-pc's 125% scaling.
            let dpi = match windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd) {
                0 => {
                    let dc = GetDC(None);
                    let screen = GetDeviceCaps(Some(dc), LOGPIXELSY);
                    ReleaseDC(None, dc);
                    screen
                }
                window => i32::try_from(window).unwrap_or(96),
            }
            .max(1);
            // JAWS gives a plain edit's font the height of its letters above the baseline (its
            // ascent), not its em: Notepad's Consolas set to 12, 24 and 36 point is "Font is
            // Consolas 11 point", 22 and 33 (old-pc, 2026-10-18; Consolas's ascent is 0.92 of its
            // em) [obs, 314/s1]. The application's font cannot be selected into this process's
            // device context: a copy made from its description is measured.
            let ascent = {
                use windows::Win32::Graphics::Gdi::{
                    CreateCompatibleDC, CreateFontIndirectW, DeleteDC, DeleteObject,
                    GetTextMetricsW, SelectObject, TEXTMETRICW,
                };
                let copy = CreateFontIndirectW(&logfont);
                if copy.is_invalid() {
                    None
                } else {
                    let dc = CreateCompatibleDC(None);
                    let previous = SelectObject(dc, copy.into());
                    let mut metrics = TEXTMETRICW::default();
                    let measured = GetTextMetricsW(dc, &mut metrics).as_bool();
                    SelectObject(dc, previous);
                    let _ = DeleteDC(dc);
                    let _ = DeleteObject(copy.into());
                    measured.then_some(metrics.tmAscent)
                }
            };
            let pixels = ascent.unwrap_or(logfont.lfHeight).unsigned_abs();
            let family_len = logfont
                .lfFaceName
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(logfont.lfFaceName.len());
            let family =
                String::from_utf16_lossy(logfont.lfFaceName.get(..family_len).unwrap_or_default());
            let points_tenths = pixels * 720 / dpi.unsigned_abs();
            Some(okur_core::FontInfo {
                family,
                // Whole points, as a font dialog sets them.
                size_tenths: (points_tenths + 5) / 10 * 10,
                bold: logfont.lfWeight >= 600,
                italic: logfont.lfItalic != 0,
                underline: logfont.lfUnderline != 0,
                alignment: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_alignments_are_named_as_say_font_names_them() {
        assert_eq!(css_alignment("start"), Some("left"));
        assert_eq!(css_alignment("center"), Some("centered"));
        assert_eq!(css_alignment("justify"), Some("justified"));
        assert_eq!(css_alignment("end"), Some("right"));
        assert_eq!(css_alignment("-webkit-auto"), None);
    }

    #[test]
    fn utf16_offsets_become_character_offsets() {
        assert_eq!(utf16_to_chars("abc", 2), 2);
        // "𝄞" is two UTF-16 units and one character.
        assert_eq!(utf16_to_chars("𝄞ab", 2), 1);
        assert_eq!(utf16_to_chars("ab", 10), 2);
    }

    /// A context's position is polled against a page field's IA2 offsets (UTF-16 units).
    #[cfg(windows)]
    #[test]
    fn a_contexts_position_is_in_utf16_units() {
        let context = okur_core::TextContext {
            text: "𝄞ab cd".to_owned(),
            caret: 3,
            selection: Some((1, 3)),
            font: None,
            word: None,
        };
        assert_eq!(
            ia2_position(&context),
            xa11y_windows::ia2::Ia2TextPosition {
                caret: 4,
                selection: Some((2, 4)),
                length: 7,
            }
        );
    }
}
