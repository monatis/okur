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
pub(crate) use reader::{activate_word_document_link, forget_form_fields, forget_form_fields_of};
#[cfg(windows)]
pub use reader::{
    focus_has_taken_its_keys, focus_is_in_page, from_ia2, ia2_position, mouse_to_character_in_edit,
    move_caret_in_edit, move_to_cell, move_to_paragraph, read_focused, read_focused_outside_page,
    visible_text, with_paragraph_alignment, word_document_fields, word_document_headings,
    word_document_links, word_document_paragraphs, word_key_state, word_line_spacing,
    word_select_field, word_view,
};

#[cfg(windows)]
mod reader {
    use super::{utf16_to_chars, MAX_TEXT_CHARS};
    use okur_core::TextContext;
    use windows::Win32::UI::Accessibility::{
        IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern, IUIAutomationTextRange,
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

    /// The text the focused element shows on the screen, through UIA's visible ranges: a
    /// console's rows in its window, not its whole buffer (9,001 rows on Windows 10). The
    /// classic console gives a range per row, without its line break (round 273): each range
    /// is a line of its own.
    pub fn visible_text(uia: &IUIAutomation) -> Option<String> {
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let focused = uia.GetFocusedElement().ok()?;
            let pattern: IUIAutomationTextPattern =
                focused.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
            let ranges = pattern.GetVisibleRanges().ok()?;
            let mut rows = Vec::new();
            for i in 0..ranges.Length().ok()? {
                let range = ranges.GetElement(i).ok()?;
                let row = range.GetText(MAX_TEXT_CHARS).ok()?.to_string();
                rows.push(row.trim_end_matches(['\r', '\n']).replace("\r\n", "\n"));
            }
            Some(rows.join("\n"))
        }
    }

    /// `context` with a plain Win32 edit's own font (Notepad's on Windows 10): UIA's edit
    /// proxy gives its character cell's height for the size, 24 point for a Consolas of 22,
    /// where JAWS says "Font is Consolas 22 point", with no paragraph formatting [obs, 314/s1].
    fn with_edit_font(mut context: TextContext) -> TextContext {
        let focused = focused_window();
        if let Some(font) = focused
            .as_ref()
            .filter(|(_, class)| class == "EDIT")
            .and_then(|(hwnd, _)| win32_font(*hwnd))
        {
            context.font = Some(font);
        }
        // A Win32 or WinForms edit that reports no colors draws with the system's window
        // text and window colors, unless its parent paints it otherwise: JAWS reads "black on
        // white" from a WinForms text box's screen [obs, 661/s1, s4].
        if let (Some(font), Some((hwnd, class))) = (context.font.as_mut(), focused.as_ref()) {
            let edit = class == "EDIT" || class.contains(".EDIT.");
            if edit && font.color.is_none() && font.background.is_none() {
                let (color, background) = edit_system_colors(*hwnd);
                font.color = Some(color);
                font.background = Some(background);
            }
        }
        context
    }

    /// The colors a Win32 edit draws with by default: the window text on the window color,
    /// or on the button face color when it is read-only (`WM_CTLCOLORSTATIC`).
    fn edit_system_colors(
        hwnd: windows::Win32::Foundation::HWND,
    ) -> (okur_core::color::Rgb, okur_core::color::Rgb) {
        use windows::Win32::Graphics::Gdi::{GetSysColor, SYS_COLOR_INDEX};
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongW, GWL_STYLE};
        const COLOR_WINDOW: i32 = 5;
        const COLOR_WINDOWTEXT: i32 = 8;
        const COLOR_BTNFACE: i32 = 15;
        const ES_READONLY: i32 = 0x0800;
        // SAFETY: a style read of a window, and system color reads; no preconditions.
        let (style, text, window, face) = unsafe {
            (
                GetWindowLongW(hwnd, GWL_STYLE),
                GetSysColor(SYS_COLOR_INDEX(COLOR_WINDOWTEXT)),
                GetSysColor(SYS_COLOR_INDEX(COLOR_WINDOW)),
                GetSysColor(SYS_COLOR_INDEX(COLOR_BTNFACE)),
            )
        };
        let background = if style & ES_READONLY != 0 {
            face
        } else {
            window
        };
        (colorref(text as i32), colorref(background as i32))
    }

    /// A Windows COLORREF (0x00BBGGRR) as a color.
    pub(super) fn colorref(value: i32) -> okur_core::color::Rgb {
        let byte = |shift: i32| ((value >> shift) & 0xFF) as u8;
        okur_core::color::Rgb {
            r: byte(0),
            g: byte(8),
            b: byte(16),
        }
    }

    /// Puts the caret of the focused Win32 edit (Notepad's, a WinForms text box, a rich edit) at
    /// character `offset` of `text`, its text as last read, with EM_SETSEL: JAWS's next and prior
    /// sentence move the caret [obs, 510/s2, s3, s7]. False for any other focus.
    pub fn move_caret_in_edit(text: &str, offset: usize) -> bool {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG};
        const EM_SETSEL: u32 = 0x00B1;
        const EM_SCROLLCARET: u32 = 0x00B7;
        let Some((hwnd, class)) = focused_window() else {
            return false;
        };
        if !(class.contains("EDIT") || class.contains("RICHEDIT")) {
            return false;
        }
        let units: usize = text.chars().take(offset).map(char::len_utf16).sum();
        // SAFETY: standard edit messages to the focused window, which may be another
        // process's; a hung window is given up on.
        unsafe {
            let set = SendMessageTimeoutW(
                hwnd,
                EM_SETSEL,
                WPARAM(units),
                LPARAM(units as isize),
                SMTO_ABORTIFHUNG,
                500,
                None,
            );
            let _ = SendMessageTimeoutW(
                hwnd,
                EM_SCROLLCARET,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                500,
                None,
            );
            set.0 != 0
        }
    }

    /// Puts the mouse on character `offset` of the focused Win32 edit's `text`: its place
    /// (EM_POSFROMCHAR, in the edit's client area), half a line down, a pixel in. False for
    /// any other field, or a character the edit does not show.
    pub fn mouse_to_character_in_edit(text: &str, offset: usize) -> bool {
        use windows::Win32::Foundation::{LPARAM, POINT, WPARAM};
        use windows::Win32::Graphics::Gdi::ClientToScreen;
        use windows::Win32::UI::WindowsAndMessaging::{
            SendMessageTimeoutW, SetCursorPos, SMTO_ABORTIFHUNG,
        };
        const EM_POSFROMCHAR: u32 = 0x00D6;
        const EM_LINEINDEX: u32 = 0x00BB;
        const EM_LINEFROMCHAR: u32 = 0x00C9;
        let Some((hwnd, class)) = focused_window() else {
            return false;
        };
        // A rich edit takes a pointer to fill, which another process cannot be given.
        if !class.contains("EDIT") || class.contains("RICHEDIT") {
            return false;
        }
        let units: usize = text.chars().take(offset).map(char::len_utf16).sum();
        let send = |message: u32, w: usize| -> Option<isize> {
            let mut result = 0usize;
            // SAFETY: a standard edit message to the focused window, which may be another
            // process's; a hung window is given up on.
            let sent = unsafe {
                SendMessageTimeoutW(
                    hwnd,
                    message,
                    WPARAM(w),
                    LPARAM(0),
                    SMTO_ABORTIFHUNG,
                    500,
                    Some(&mut result),
                )
            };
            (sent.0 != 0).then_some(result as isize)
        };
        // LOWORD x, HIWORD y, each signed; -1 for a character past the text.
        let position = |units: usize| -> Option<(i32, i32)> {
            let packed = send(EM_POSFROMCHAR, units)?;
            if packed == -1 {
                return None;
            }
            Some((
                i32::from((packed & 0xFFFF) as u16 as i16),
                i32::from(((packed >> 16) & 0xFFFF) as u16 as i16),
            ))
        };
        let Some((x, y)) = position(units) else {
            return false;
        };
        // The line's height: the next line's top, or the line above's.
        let line = send(EM_LINEFROMCHAR, units).unwrap_or(0).max(0) as usize;
        let next_top = send(EM_LINEINDEX, line + 1)
            .filter(|&start| start > 0)
            .and_then(|start| position(start as usize))
            .map(|(_, top)| top - y);
        let above_top = line
            .checked_sub(1)
            .and_then(|above| send(EM_LINEINDEX, above))
            .filter(|&start| start >= 0)
            .and_then(|start| position(start as usize))
            .map(|(_, top)| y - top);
        let height = next_top
            .or(above_top)
            .filter(|h| (4..200).contains(h))
            .unwrap_or(16);
        let mut point = POINT {
            x: x + 1,
            y: y + height / 2,
        };
        // SAFETY: plain Win32 calls with a valid window and point.
        unsafe {
            if !ClientToScreen(hwnd, &mut point).as_bool() {
                return false;
            }
            SetCursorPos(point.x, point.y).is_ok()
        }
    }

    /// The focused window of the application in front, with its class in capitals.
    /// Whether the thread of the window with the keyboard focus has handled the input queued
    /// for it: a message sent to it is handled when the thread next asks for one, before the
    /// keys in its queue, so the second of two sent in turn returns once the key before them
    /// has been taken. `false` when the window does not answer within 100 ms each.
    pub fn focus_has_taken_its_keys() -> bool {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_NULL,
        };
        let Some((hwnd, _)) = focused_window() else {
            return false;
        };
        (0..2).all(|_| {
            let mut result = 0usize;
            // SAFETY: a message without arguments to another thread's window, with a timeout.
            let sent = unsafe {
                SendMessageTimeoutW(
                    hwnd,
                    WM_NULL,
                    WPARAM(0),
                    LPARAM(0),
                    SMTO_ABORTIFHUNG,
                    100,
                    Some(&mut result),
                )
            };
            sent.0 != 0
        })
    }

    /// The line spacing of the paragraph at the caret of Word's focused document, from Word's
    /// own object model (`crate::word_om`); `None` outside Word's document.
    pub fn word_line_spacing() -> Option<okur_core::LineSpacing> {
        let (window, class) = focused_window()?;
        if class != "_WWG" {
            return None;
        }
        let spacing = crate::word_om::selection_line_spacing(window)?;
        Some(okur_core::LineSpacing {
            rule: spacing.rule,
            centipoints: (spacing.points * 100.0).round().clamp(0.0, u32::MAX as f32) as u32,
        })
    }

    /// What one of Word's keys (`key`) concerns in Word's focused document, from Word's own
    /// object model (`crate::word_om`): whether it tracks its changes, its paragraph's
    /// alignment, its selection's font size. `None` outside Word's document.
    pub fn word_key_state(key: okur_core::WordKey) -> Option<okur_core::WordKeyState> {
        use okur_core::{WordKey, WordKeyState};
        let (window, class) = focused_window()?;
        if class != "_WWG" {
            return None;
        }
        match key {
            WordKey::TrackChanges => {
                crate::word_om::track_revisions(window).map(WordKeyState::TrackChanges)
            }
            WordKey::Alignment => {
                crate::word_om::paragraph_alignment(window).map(WordKeyState::Alignment)
            }
            WordKey::FontSize => crate::word_om::selection_font_size(window)
                .map(|points| WordKeyState::FontSize((points * 10.0).round() as u32)),
        }
    }

    /// The fields of Word's focused document, in order, from Word's own object model
    /// (`crate::word_om`); `None` outside Word's document.
    pub fn word_document_fields() -> Option<Vec<okur_core::DocumentField>> {
        let (window, class) = focused_window()?;
        if class != "_WWG" {
            return None;
        }
        let fields = crate::word_om::document_fields(window)?;
        Some(
            fields
                .into_iter()
                .map(|field| okur_core::DocumentField {
                    code: field.code,
                    result: field.result,
                })
                .collect(),
        )
    }

    /// Puts the caret of Word's focused document on its field `index` (from 0, as
    /// [`word_document_fields`] lists them). False outside Word's document.
    pub fn word_select_field(index: usize) -> bool {
        focused_window()
            .filter(|(_, class)| class == "_WWG")
            .is_some_and(|(window, _)| crate::word_om::select_field(window, index))
    }

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
    /// is a page field's, which stops at punctuation as Chrome's word moves do. Its font is
    /// the colors at its caret alone, for Say Color ("black on white" in a page field with
    /// text [obs, 662/s5, s7]); Say Font has its own reading of a page field.
    pub fn from_ia2(state: xa11y_windows::ia2::Ia2TextState) -> TextContext {
        let caret = utf16_to_chars(&state.text, state.caret);
        let css = |value: Option<&String>| value.and_then(|v| okur_core::color::Rgb::parse_css(v));
        let (color, background) = (css(state.color.as_ref()), css(state.background.as_ref()));
        let font = (color.is_some() || background.is_some()).then(|| okur_core::FontInfo {
            color,
            background,
            ..okur_core::FontInfo::default()
        });
        TextContext {
            caret,
            selection: state.selection.map(|(s, e)| {
                (
                    utf16_to_chars(&state.text, s),
                    utf16_to_chars(&state.text, e),
                )
            }),
            word: okur_core::editing::web_word_at(&state.text, caret),
            misspelled: state
                .misspelled
                .iter()
                .map(|&(s, e)| {
                    (
                        utf16_to_chars(&state.text, s),
                        utf16_to_chars(&state.text, e),
                    )
                })
                .collect(),
            text: state.text,
            font,
            structure: None,
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
            // The selection through EM_GETSEL's out-pointers (marshalled by the system, as
            // WM_GETTEXT's buffer is): its result packs both ends in 16-bit words, which a
            // text past 65,535 characters overflows (performance review 2026-10-23, P24).
            let (mut start, mut end) = (0u32, 0u32);
            ask(
                EM_GETSEL,
                std::ptr::from_mut(&mut start) as usize,
                std::ptr::from_mut(&mut end) as isize,
            )?;
            let start = i32::try_from(start).ok()?;
            let end = i32::try_from(end).ok()?;
            Some(TextContext {
                caret: utf16_to_chars(&text, start),
                selection: (start < end)
                    .then(|| (utf16_to_chars(&text, start), utf16_to_chars(&text, end))),
                text,
                font: win32_font(hwnd),
                word: None,
                misspelled: Vec::new(),
                structure: None,
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

    /// Characters in `range`'s text.
    fn chars_of(range: &IUIAutomationTextRange) -> Option<usize> {
        // SAFETY: a UIA text range call on a valid range.
        let text = unsafe { range.GetText(MAX_TEXT_CHARS) }.ok()?;
        Some(text.to_string().chars().count())
    }

    /// The range of `unit` around `caret` (at offset `start`) as `(start, end)`, by the
    /// field's own unit, counted from the caret over the short texts between them.
    fn unit_at(
        caret: &IUIAutomationTextRange,
        start: usize,
        unit: windows::Win32::UI::Accessibility::TextUnit,
    ) -> Option<(usize, usize)> {
        // SAFETY: UIA text range calls on valid ranges of one provider.
        unsafe {
            let range = caret.Clone().ok()?;
            range.ExpandToEnclosingUnit(unit).ok()?;
            let unit_start = match range
                .CompareEndpoints(
                    TextPatternRangeEndpoint_Start,
                    caret,
                    TextPatternRangeEndpoint_Start,
                )
                .ok()?
            {
                // Before the caret: the text from the unit's start to the caret.
                ..0 => {
                    let before = range.Clone().ok()?;
                    before
                        .MoveEndpointByRange(
                            TextPatternRangeEndpoint_End,
                            caret,
                            TextPatternRangeEndpoint_Start,
                        )
                        .ok()?;
                    start.checked_sub(chars_of(&before)?)?
                }
                0 => start,
                // After it: the text from the caret to the unit's start.
                _ => {
                    let after = caret.Clone().ok()?;
                    after
                        .MoveEndpointByRange(
                            TextPatternRangeEndpoint_End,
                            &range,
                            TextPatternRangeEndpoint_Start,
                        )
                        .ok()?;
                    start + chars_of(&after)?
                }
            };
            Some((unit_start, unit_start + chars_of(&range)?))
        }
    }

    /// Word's document window, whose text tells its headings, lists and tables.
    const WORD_DOCUMENT_CLASS: &str = "_WwG";
    /// UI Automation's style ids: Heading 1 to 9, and the bulleted and numbered lists.
    const STYLE_HEADING_1: i32 = 70001;
    const STYLE_HEADING_9: i32 = 70009;
    const STYLE_BULLETED_LIST: i32 = 70015;
    const STYLE_NUMBERED_LIST: i32 = 70016;

    /// What Word's text says of the caret's line: its own line (a table cell's end ends one),
    /// the paragraph's heading level and list, and the table cell. `None` outside Word's
    /// document.
    fn word_structure(
        uia: &IUIAutomation,
        focused: &IUIAutomationElement,
        pattern: &IUIAutomationTextPattern,
        caret: &IUIAutomationTextRange,
        start: usize,
    ) -> Option<okur_core::LineStructure> {
        use windows::Win32::UI::Accessibility::{
            TextUnit_Line, TextUnit_Paragraph, UIA_BulletStyleAttributeId, UIA_StyleIdAttributeId,
        };
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            if focused.CurrentClassName().ok()? != WORD_DOCUMENT_CLASS {
                return None;
            }
            let line = unit_at(caret, start, TextUnit_Line)?;
            let paragraph = caret.Clone().ok()?;
            paragraph.ExpandToEnclosingUnit(TextUnit_Paragraph).ok()?;
            let attribute = |id| {
                paragraph
                    .GetAttributeValue(id)
                    .ok()
                    .and_then(|v| i32::try_from(&v).ok())
            };
            let style = attribute(UIA_StyleIdAttributeId);
            let heading = style
                .filter(|id| (STYLE_HEADING_1..=STYLE_HEADING_9).contains(id))
                .and_then(|id| u8::try_from(id - STYLE_HEADING_1 + 1).ok());
            let list_item = attribute(UIA_BulletStyleAttributeId).is_some_and(|b| b != 0)
                || matches!(style, Some(STYLE_BULLETED_LIST | STYLE_NUMBERED_LIST));
            Some(okur_core::LineStructure {
                line,
                heading,
                list_item,
                cell: table_cell(uia, focused, pattern, caret),
                form_fields: word_form_fields(focused, caret, start).unwrap_or_default(),
                links: line_links(pattern, caret, line.0).unwrap_or_default(),
            })
        }
    }

    /// The hyperlinks on the caret's line, each as `(start, end)` from `line_start`, the line's
    /// offset in the text: the line's children UI Automation calls hyperlinks.
    fn line_links(
        pattern: &IUIAutomationTextPattern,
        caret: &IUIAutomationTextRange,
        line_start: usize,
    ) -> Option<Vec<(usize, usize)>> {
        use windows::Win32::UI::Accessibility::{TextUnit_Line, UIA_HyperlinkControlTypeId};
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let line = caret.Clone().ok()?;
            line.ExpandToEnclosingUnit(TextUnit_Line).ok()?;
            let children = line.GetChildren().ok()?;
            let mut links = Vec::new();
            for index in 0..children.Length().unwrap_or(0) {
                let Ok(child) = children.GetElement(index) else {
                    continue;
                };
                if child.CurrentControlType().ok() != Some(UIA_HyperlinkControlTypeId) {
                    continue;
                }
                let Ok(link) = pattern.RangeFromChild(&child) else {
                    continue;
                };
                let before = line.Clone().ok()?;
                if before
                    .MoveEndpointByRange(
                        TextPatternRangeEndpoint_End,
                        &link,
                        TextPatternRangeEndpoint_Start,
                    )
                    .is_err()
                {
                    continue;
                }
                let at = line_start + chars_of(&before).unwrap_or(0);
                links.push((at, at + chars_of(&link).unwrap_or(0)));
            }
            Some(links)
        }
    }

    thread_local! {
        /// The Word document last asked whether it has form fields, when, and its answer.
        static FORM_FIELDS_SEEN: std::cell::Cell<Option<(isize, std::time::Instant, bool)>> =
            const { std::cell::Cell::new(None) };
    }

    /// The form fields last read, for whichever thread reads the text next: the reads after a
    /// key forget them first ([`forget_form_fields`]).
    static FORM_FIELDS_READ: std::sync::Mutex<Option<FormFieldsRead>> = std::sync::Mutex::new(None);

    /// Forgets the form fields last read: a key can have changed one (Space on a check box)
    /// where the text shows nothing of it, so the read after it asks Word again.
    pub fn forget_form_fields() {
        if let Ok(mut read) = FORM_FIELDS_READ.lock() {
            *read = None;
        }
    }

    /// [`forget_form_fields`] where they are the window `window's`: a value change of the
    /// document whose fields are kept, not of Word's ribbon boxes beside it, which change as
    /// often as the caret moves.
    pub fn forget_form_fields_of(window: isize) {
        if let Ok(mut read) = FORM_FIELDS_READ.lock() {
            if read.as_ref().is_some_and(|r| r.window == window) {
                *read = None;
            }
        }
    }

    /// A paragraph's form fields as last read, kept for the events a key sets off.
    struct FormFieldsRead {
        window: isize,
        paragraph: usize,
        text: String,
        when: std::time::Instant,
        fields: Vec<okur_core::DocumentFormField>,
    }

    /// How long a paragraph's form fields are kept: the flood of events one key sets off (the
    /// tracer reads the text for each, twice for a caret's) reads Word once. Live, the read
    /// after each key reads Word again; the tracer, which knows no keys, at most once a second
    /// (with 300 ms it fell seconds behind the keys in Word's form, round-1055d).
    const FORM_FIELDS_FRESH: std::time::Duration = std::time::Duration::from_secs(1);

    /// How long a document's answer to whether it has form fields is kept.
    const FORM_FIELDS_KEPT: std::time::Duration = std::time::Duration::from_secs(2);

    /// The form fields on the caret's paragraph in Word's document, placed in the text from
    /// `start`, the caret's offset: Word's object model knows them, its UI Automation does not
    /// (it shows a text field's result, nothing for a check box or a drop-down). A document
    /// without any is asked again only every few seconds.
    fn word_form_fields(
        focused: &IUIAutomationElement,
        caret: &IUIAutomationTextRange,
        start: usize,
    ) -> Option<Vec<okur_core::DocumentFormField>> {
        use okur_core::FormFieldKind;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Accessibility::TextUnit_Paragraph;
        // SAFETY: a UIA property read on a valid element.
        let window = unsafe { focused.CurrentNativeWindowHandle() }.ok()?;
        let window = HWND(window.0);
        let key = window.0 as isize;
        let known = FORM_FIELDS_SEEN.with(|seen| {
            seen.get()
                .filter(|(at, when, _)| *at == key && when.elapsed() < FORM_FIELDS_KEPT)
                .map(|(_, _, has)| has)
        });
        let has = match known {
            Some(has) => has,
            None => {
                let has = crate::word_om::has_form_fields(window).unwrap_or(false);
                FORM_FIELDS_SEEN.with(|seen| seen.set(Some((key, std::time::Instant::now(), has))));
                has
            }
        };
        if !has {
            return None;
        }
        let (paragraph, end) = unit_at(caret, start, TextUnit_Paragraph)?;
        // SAFETY: UIA text range calls on a valid range.
        let paragraph_text = unsafe {
            let range = caret.Clone().ok()?;
            {
                range.ExpandToEnclosingUnit(TextUnit_Paragraph).ok()?;
                range
                    .GetText(i32::try_from(end - paragraph).ok()?.saturating_add(1))
                    .ok()?
                    .to_string()
            }
        };
        let kept = FORM_FIELDS_READ.lock().ok().and_then(|read| {
            read.as_ref()
                .filter(|r| {
                    r.window == key
                        && r.paragraph == paragraph
                        && r.text == paragraph_text
                        && r.when.elapsed() < FORM_FIELDS_FRESH
                })
                .map(|r| r.fields.clone())
        });
        if let Some(fields) = kept {
            return Some(fields);
        }
        let placed = crate::word_om::paragraph_form_fields(window)?;
        let fields: Vec<okur_core::DocumentFormField> = placed
            .into_iter()
            .filter_map(|placed| {
                let field = placed.field;
                let kind = match field.kind {
                    crate::word_om::TEXT_INPUT => FormFieldKind::Text,
                    crate::word_om::CHECK_BOX => FormFieldKind::CheckBox,
                    crate::word_om::DROP_DOWN => FormFieldKind::DropDown,
                    _ => return None,
                };
                let chosen = field.chosen.and_then(|c| usize::try_from(c).ok());
                let value = match kind {
                    FormFieldKind::Text => field.result.clone(),
                    FormFieldKind::DropDown => chosen
                        .and_then(|c| field.entries.get(c.checked_sub(1)?).cloned())
                        .unwrap_or_default(),
                    FormFieldKind::CheckBox => String::new(),
                };
                let at = paragraph + placed.before;
                Some(okur_core::DocumentFormField {
                    kind,
                    name: field.name,
                    start: at,
                    end: at + placed.length,
                    value,
                    checked: field.checked,
                    choice: chosen
                        .filter(|_| kind == FormFieldKind::DropDown)
                        .map(|c| (c, field.entries.len())),
                })
            })
            .collect();
        if let Ok(mut read) = FORM_FIELDS_READ.lock() {
            *read = Some(FormFieldsRead {
                window: key,
                paragraph,
                text: paragraph_text,
                when: std::time::Instant::now(),
                fields: fields.clone(),
            });
        }
        Some(fields)
    }

    /// The cell holding `caret` in `document`, and its grid item: the first element around
    /// the caret with one (Word's cell is the caret's own).
    fn caret_cell(
        uia: &IUIAutomation,
        document: &IUIAutomationElement,
        caret: &IUIAutomationTextRange,
    ) -> Option<(
        IUIAutomationElement,
        windows::Win32::UI::Accessibility::IUIAutomationGridItemPattern,
    )> {
        use windows::Win32::UI::Accessibility::{
            IUIAutomationGridItemPattern, UIA_GridItemPatternId,
        };
        /// Elements walked up from the caret to its cell.
        const MAX_DEPTH: usize = 8;
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let walker = uia.ControlViewWalker().ok()?;
            let mut element = caret.GetEnclosingElement().ok()?;
            for _ in 0..MAX_DEPTH {
                if let Ok(item) = element
                    .GetCurrentPatternAs::<IUIAutomationGridItemPattern>(UIA_GridItemPatternId)
                {
                    return Some((element, item));
                }
                if uia.CompareElements(&element, document).ok()?.as_bool() {
                    return None;
                }
                element = walker.GetParentElement(&element).ok()?;
            }
            None
        }
    }

    /// A cell's text, read through the document's text: `None` when empty.
    fn element_text(
        pattern: &IUIAutomationTextPattern,
        cell: &IUIAutomationElement,
    ) -> Option<String> {
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            pattern
                .RangeFromChild(cell)
                .and_then(|range| range.GetText(MAX_CELL_CHARS))
                .ok()
                .map(|text| cell_text(&text.to_string()))
                .filter(|text| !text.is_empty())
        }
    }

    /// The table cell holding `caret` in `document`: its place, its table's number among the
    /// document's tables, its column's header text, and its row's title, the row's first cell.
    fn table_cell(
        uia: &IUIAutomation,
        document: &IUIAutomationElement,
        pattern: &IUIAutomationTextPattern,
        caret: &IUIAutomationTextRange,
    ) -> Option<okur_core::TableCell> {
        use windows::Win32::System::Variant::VARIANT;
        use windows::Win32::UI::Accessibility::{
            IUIAutomationGridPattern, IUIAutomationTableItemPattern, TreeScope_Descendants,
            UIA_ControlTypePropertyId, UIA_GridPatternId, UIA_TableControlTypeId,
            UIA_TableItemPatternId,
        };
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let (element, item) = caret_cell(uia, document, caret)?;
            let index = |v: i32| usize::try_from(v).ok().map(|v| v + 1);
            let grid = item.CurrentContainingGrid().ok()?;
            let size: IUIAutomationGridPattern =
                grid.GetCurrentPatternAs(UIA_GridPatternId).ok()?;
            let condition = uia
                .CreatePropertyCondition(
                    UIA_ControlTypePropertyId,
                    &VARIANT::from(UIA_TableControlTypeId.0),
                )
                .ok()?;
            let tables = document.FindAll(TreeScope_Descendants, &condition).ok()?;
            let table = (0..tables.Length().ok()?)
                .position(|i| {
                    tables
                        .GetElement(i)
                        .and_then(|t| uia.CompareElements(&t, &grid))
                        .is_ok_and(|same| same.as_bool())
                })
                .map_or(1, |i| i + 1);
            // JAWS takes a column's title from the first row, header row or not, and a row's
            // from the first column only where the table has one, which Word reports as row
            // header items [obs, 1040/s12, s13; word-probe.ps1 on 1034's and 1040's tables].
            let row = item.CurrentRow().ok()?;
            let column = item.CurrentColumn().ok()?;
            let column_header = size
                .GetItem(0, column)
                .ok()
                .and_then(|first| element_text(pattern, &first));
            let row_titles = element
                .GetCurrentPatternAs::<IUIAutomationTableItemPattern>(UIA_TableItemPatternId)
                .and_then(|cell| cell.GetCurrentRowHeaderItems())
                .and_then(|headers| headers.Length())
                .is_ok_and(|count| count > 0);
            let row_header = size
                .GetItem(row, 0)
                .ok()
                .filter(|_| row_titles)
                .and_then(|first| element_text(pattern, &first));
            Some(okur_core::TableCell {
                table,
                row: index(row)?,
                column: index(column)?,
                rows: usize::try_from(size.CurrentRowCount().ok()?).ok()?,
                columns: usize::try_from(size.CurrentColumnCount().ok()?).ok()?,
                column_header,
                row_header,
            })
        }
    }

    /// The caret of Word's focused document to the start of its table's cell at `row` and
    /// `column` (from 1), in the table holding the caret: JAWS's table keys in Word. False
    /// when the focus is not in a table of Word's or there is no such cell.
    pub fn move_to_cell(uia: &IUIAutomation, row: usize, column: usize) -> bool {
        use windows::Win32::UI::Accessibility::{IUIAutomationGridPattern, UIA_GridPatternId};
        let index = |n: usize| n.checked_sub(1).and_then(|n| i32::try_from(n).ok());
        let (Some(row), Some(column)) = (index(row), index(column)) else {
            return false;
        };
        // SAFETY: UIA client calls on valid objects.
        let moved = || -> Option<()> {
            unsafe {
                let focused = uia.GetFocusedElement().ok()?;
                if focused.CurrentClassName().ok()? != WORD_DOCUMENT_CLASS {
                    return None;
                }
                let pattern: IUIAutomationTextPattern =
                    focused.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
                let caret = pattern.GetSelection().ok()?.GetElement(0).ok()?;
                let (_, item) = caret_cell(uia, &focused, &caret)?;
                let grid: IUIAutomationGridPattern = item
                    .CurrentContainingGrid()
                    .ok()?
                    .GetCurrentPatternAs(UIA_GridPatternId)
                    .ok()?;
                let cell = grid.GetItem(row, column).ok()?;
                let range = pattern.RangeFromChild(&cell).ok()?;
                range
                    .MoveEndpointByRange(
                        TextPatternRangeEndpoint_End,
                        &range,
                        TextPatternRangeEndpoint_Start,
                    )
                    .ok()?;
                range.Select().ok()
            }
        };
        moved().is_some()
    }

    /// The most paragraphs of a document read for its headings: a long book's, at about four
    /// UI Automation calls each.
    const MAX_PARAGRAPHS: usize = 20_000;
    /// The most of a heading's text read.
    const MAX_HEADING_CHARS: i32 = 500;

    /// The headings of Word's document: the focused one, or with `window`, the one in that
    /// top-level window, wherever the focus is (the recorder's, after JAWS's Heading List took
    /// it). Read paragraph by paragraph, by their styles. `None` without Word's document.
    pub fn word_document_headings(
        uia: &IUIAutomation,
        window: Option<windows::Win32::Foundation::HWND>,
    ) -> Option<Vec<okur_core::DocumentHeading>> {
        use windows::core::BSTR;
        use windows::Win32::System::Variant::VARIANT;
        use windows::Win32::UI::Accessibility::{
            TextUnit_Paragraph, TreeScope_Descendants, UIA_ClassNamePropertyId,
            UIA_StyleIdAttributeId,
        };
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let document = match window {
                Some(window) => {
                    let condition = uia
                        .CreatePropertyCondition(
                            UIA_ClassNamePropertyId,
                            &VARIANT::from(BSTR::from(WORD_DOCUMENT_CLASS)),
                        )
                        .ok()?;
                    uia.ElementFromHandle(window)
                        .ok()?
                        .FindFirst(TreeScope_Descendants, &condition)
                        .ok()?
                }
                None => {
                    let focused = uia.GetFocusedElement().ok()?;
                    if focused.CurrentClassName().ok()? != WORD_DOCUMENT_CLASS {
                        return None;
                    }
                    focused
                }
            };
            let pattern: IUIAutomationTextPattern =
                document.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
            let at = pattern.DocumentRange().ok()?;
            at.MoveEndpointByRange(
                TextPatternRangeEndpoint_End,
                &at,
                TextPatternRangeEndpoint_Start,
            )
            .ok()?;
            let mut headings = Vec::new();
            for paragraph in 0..MAX_PARAGRAPHS {
                let range = at.Clone().ok()?;
                range.ExpandToEnclosingUnit(TextUnit_Paragraph).ok()?;
                let level = range
                    .GetAttributeValue(UIA_StyleIdAttributeId)
                    .ok()
                    .and_then(|v| i32::try_from(&v).ok())
                    .filter(|id| (STYLE_HEADING_1..=STYLE_HEADING_9).contains(id))
                    .and_then(|id| u8::try_from(id - STYLE_HEADING_1 + 1).ok());
                if let Some(level) = level {
                    let text = range.GetText(MAX_HEADING_CHARS).ok()?.to_string();
                    let text = cell_text(&text);
                    if !text.is_empty() {
                        headings.push(okur_core::DocumentHeading {
                            text,
                            level,
                            paragraph,
                        });
                    }
                }
                if at.Move(TextUnit_Paragraph, 1).ok()? < 1 {
                    break;
                }
            }
            Some(headings)
        }
    }

    /// Word's document element: the focused one, or with `window` the one in that top-level
    /// window. `None` without Word's document.
    fn word_document(
        uia: &IUIAutomation,
        window: Option<windows::Win32::Foundation::HWND>,
    ) -> Option<IUIAutomationElement> {
        use windows::core::BSTR;
        use windows::Win32::System::Variant::VARIANT;
        use windows::Win32::UI::Accessibility::{TreeScope_Descendants, UIA_ClassNamePropertyId};
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            match window {
                Some(window) => {
                    let condition = uia
                        .CreatePropertyCondition(
                            UIA_ClassNamePropertyId,
                            &VARIANT::from(BSTR::from(WORD_DOCUMENT_CLASS)),
                        )
                        .ok()?;
                    uia.ElementFromHandle(window)
                        .ok()?
                        .FindFirst(TreeScope_Descendants, &condition)
                        .ok()
                }
                None => {
                    let focused = uia.GetFocusedElement().ok()?;
                    (focused.CurrentClassName().ok()? == WORD_DOCUMENT_CLASS).then_some(focused)
                }
            }
        }
    }

    /// The hyperlinks of Word's document, the focused one or with `window` the one in that
    /// top-level window, for its Links List: each one's text, its address, and the paragraph it
    /// starts in, counted as [`word_document_headings`] counts them. Word gives them as the
    /// document range's children, UI Automation hyperlinks (old-pc, `fields-uia.ps1`,
    /// 2026-10-25). `None` without Word's document.
    pub fn word_document_links(
        uia: &IUIAutomation,
        window: Option<windows::Win32::Foundation::HWND>,
    ) -> Option<Vec<okur_core::DocumentLink>> {
        use windows::Win32::UI::Accessibility::{
            IUIAutomationValuePattern, TextUnit_Paragraph, UIA_HyperlinkControlTypeId,
            UIA_ValuePatternId,
        };
        let document = word_document(uia, window)?;
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let pattern: IUIAutomationTextPattern =
                document.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
            let whole = pattern.DocumentRange().ok()?;
            let children = whole.GetChildren().ok()?;
            let mut found = Vec::new();
            for index in 0..children.Length().unwrap_or(0) {
                let Ok(child) = children.GetElement(index) else {
                    continue;
                };
                if child.CurrentControlType().ok() != Some(UIA_HyperlinkControlTypeId) {
                    continue;
                }
                let text = child
                    .CurrentName()
                    .map(|n| n.to_string())
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                let url = child
                    .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
                    .ok()
                    .and_then(|v| v.CurrentValue().ok())
                    .map(|v| v.to_string())
                    .filter(|v| !v.is_empty());
                let Ok(range) = pattern.RangeFromChild(&child) else {
                    continue;
                };
                found.push((text, url, range));
            }
            // The paragraph each starts in, in one pass over the paragraphs: the first whose
            // end is past the link's start.
            let at = whole.Clone().ok()?;
            at.MoveEndpointByRange(
                TextPatternRangeEndpoint_End,
                &at,
                TextPatternRangeEndpoint_Start,
            )
            .ok()?;
            let mut links = Vec::new();
            let mut pending = found.into_iter().peekable();
            for paragraph in 0..MAX_PARAGRAPHS {
                let range = at.Clone().ok()?;
                range.ExpandToEnclosingUnit(TextUnit_Paragraph).ok()?;
                while let Some((_, _, link)) = pending.peek() {
                    let starts_here = range
                        .CompareEndpoints(
                            TextPatternRangeEndpoint_End,
                            link,
                            TextPatternRangeEndpoint_Start,
                        )
                        .is_ok_and(|order| order > 0);
                    if !starts_here {
                        break;
                    }
                    if let Some((text, url, _)) = pending.next() {
                        links.push(okur_core::DocumentLink {
                            text,
                            url,
                            paragraph,
                        });
                    }
                }
                if pending.peek().is_none() || at.Move(TextUnit_Paragraph, 1).ok()? < 1 {
                    break;
                }
            }
            Some(links)
        }
    }

    /// Follows the hyperlink `index` of Word's focused document, as
    /// [`word_document_links`] lists them. False without it.
    pub fn activate_word_document_link(uia: &IUIAutomation, index: usize) -> bool {
        use windows::Win32::UI::Accessibility::{
            IUIAutomationInvokePattern, UIA_HyperlinkControlTypeId, UIA_InvokePatternId,
        };
        let Some(document) = word_document(uia, None) else {
            return false;
        };
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let Ok(pattern) =
                document.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
            else {
                return false;
            };
            let Ok(children) = pattern.DocumentRange().and_then(|r| r.GetChildren()) else {
                return false;
            };
            let link = (0..children.Length().unwrap_or(0))
                .filter_map(|i| children.GetElement(i).ok())
                .filter(|child| child.CurrentControlType().ok() == Some(UIA_HyperlinkControlTypeId))
                .nth(index);
            link.and_then(|link| {
                link.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                    .ok()
            })
            .is_some_and(|invoke| invoke.Invoke().is_ok())
        }
    }

    /// The paragraphs of Word's document, the focused one or with `window` the one in that
    /// top-level window, for Say All: each one's start in the document's text (the sum of the
    /// paragraphs' lengths before it, as the document's text is theirs one after another), its
    /// heading level and list from its style, and its table cell. `None` without Word's
    /// document.
    pub fn word_document_paragraphs(
        uia: &IUIAutomation,
        window: Option<windows::Win32::Foundation::HWND>,
    ) -> Option<Vec<okur_core::DocumentParagraph>> {
        use windows::core::BSTR;
        use windows::Win32::System::Variant::VARIANT;
        use windows::Win32::UI::Accessibility::{
            IUIAutomationGridPattern, TextUnit_Paragraph, TreeScope_Descendants,
            UIA_BulletStyleAttributeId, UIA_ClassNamePropertyId, UIA_ControlTypePropertyId,
            UIA_GridPatternId, UIA_StyleIdAttributeId, UIA_TableControlTypeId,
        };
        // SAFETY: UIA client calls on valid objects.
        unsafe {
            let document = match window {
                Some(window) => {
                    let condition = uia
                        .CreatePropertyCondition(
                            UIA_ClassNamePropertyId,
                            &VARIANT::from(BSTR::from(WORD_DOCUMENT_CLASS)),
                        )
                        .ok()?;
                    uia.ElementFromHandle(window)
                        .ok()?
                        .FindFirst(TreeScope_Descendants, &condition)
                        .ok()?
                }
                None => {
                    let focused = uia.GetFocusedElement().ok()?;
                    if focused.CurrentClassName().ok()? != WORD_DOCUMENT_CLASS {
                        return None;
                    }
                    focused
                }
            };
            let pattern: IUIAutomationTextPattern =
                document.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
            // The document's tables, which number a cell's table.
            let condition = uia
                .CreatePropertyCondition(
                    UIA_ControlTypePropertyId,
                    &VARIANT::from(UIA_TableControlTypeId.0),
                )
                .ok()?;
            let tables = document.FindAll(TreeScope_Descendants, &condition).ok()?;
            let at = pattern.DocumentRange().ok()?;
            at.MoveEndpointByRange(
                TextPatternRangeEndpoint_End,
                &at,
                TextPatternRangeEndpoint_Start,
            )
            .ok()?;
            let mut paragraphs = Vec::new();
            let mut start = 0;
            for _ in 0..MAX_PARAGRAPHS {
                let range = at.Clone().ok()?;
                range.ExpandToEnclosingUnit(TextUnit_Paragraph).ok()?;
                let attribute = |id| {
                    range
                        .GetAttributeValue(id)
                        .ok()
                        .and_then(|v| i32::try_from(&v).ok())
                };
                let style = attribute(UIA_StyleIdAttributeId);
                let heading = style
                    .filter(|id| (STYLE_HEADING_1..=STYLE_HEADING_9).contains(id))
                    .and_then(|id| u8::try_from(id - STYLE_HEADING_1 + 1).ok());
                let list_item = attribute(UIA_BulletStyleAttributeId).is_some_and(|b| b != 0)
                    || matches!(style, Some(STYLE_BULLETED_LIST | STYLE_NUMBERED_LIST));
                let cell = caret_cell(uia, &document, &range).and_then(|(_, item)| {
                    let grid = item.CurrentContainingGrid().ok()?;
                    let size: IUIAutomationGridPattern =
                        grid.GetCurrentPatternAs(UIA_GridPatternId).ok()?;
                    let index = |v: i32| usize::try_from(v).ok().map(|v| v + 1);
                    let table = (0..tables.Length().ok()?)
                        .position(|i| {
                            tables
                                .GetElement(i)
                                .and_then(|t| uia.CompareElements(&t, &grid))
                                .is_ok_and(|same| same.as_bool())
                        })
                        .map_or(1, |i| i + 1);
                    Some(okur_core::TableCell {
                        table,
                        row: index(item.CurrentRow().ok()?)?,
                        column: index(item.CurrentColumn().ok()?)?,
                        rows: usize::try_from(size.CurrentRowCount().ok()?).ok()?,
                        columns: usize::try_from(size.CurrentColumnCount().ok()?).ok()?,
                        column_header: None,
                        row_header: None,
                    })
                });
                paragraphs.push(okur_core::DocumentParagraph {
                    start,
                    heading,
                    list_item,
                    cell,
                });
                start += chars_of(&range)?;
                if at.Move(TextUnit_Paragraph, 1).ok()? < 1 {
                    break;
                }
            }
            Some(paragraphs)
        }
    }

    /// The view Word's window shows its document in: the name of its status bar's view button
    /// that is on ("Print Layout"), in `window` or the window in front. `None` when none is.
    pub fn word_view(
        uia: &IUIAutomation,
        window: Option<windows::Win32::Foundation::HWND>,
    ) -> Option<String> {
        use windows::core::BSTR;
        use windows::Win32::System::Variant::VARIANT;
        use windows::Win32::UI::Accessibility::{
            IUIAutomationTogglePattern, ToggleState_On, TreeScope_Descendants,
            UIA_ClassNamePropertyId, UIA_TogglePatternId,
        };
        /// Word's status bar's view buttons (Read Mode, Print Layout, Web Layout).
        const VIEW_BUTTON_CLASS: &str = "NetUIStickyButton";
        // SAFETY: plain Win32 and UIA client calls on valid objects.
        unsafe {
            let window = window.unwrap_or_else(|| GetForegroundWindow());
            let condition = uia
                .CreatePropertyCondition(
                    UIA_ClassNamePropertyId,
                    &VARIANT::from(BSTR::from(VIEW_BUTTON_CLASS)),
                )
                .ok()?;
            let buttons = uia
                .ElementFromHandle(window)
                .ok()?
                .FindAll(TreeScope_Descendants, &condition)
                .ok()?;
            (0..buttons.Length().ok()?).find_map(|i| {
                let button = buttons.GetElement(i).ok()?;
                let toggle: IUIAutomationTogglePattern =
                    button.GetCurrentPatternAs(UIA_TogglePatternId).ok()?;
                (toggle.CurrentToggleState().ok()? == ToggleState_On)
                    .then(|| button.CurrentName().ok().map(|n| n.to_string()))
                    .flatten()
            })
        }
    }

    /// The caret of Word's focused document to the start of its `paragraph` (from 0, as
    /// [`word_document_headings`] counts them). False when the focus is not in Word's document
    /// or it has no such paragraph.
    pub fn move_to_paragraph(uia: &IUIAutomation, paragraph: usize) -> bool {
        use windows::Win32::UI::Accessibility::TextUnit_Paragraph;
        let Ok(count) = i32::try_from(paragraph) else {
            return false;
        };
        // SAFETY: UIA client calls on valid objects.
        let moved = || -> Option<()> {
            unsafe {
                let focused = uia.GetFocusedElement().ok()?;
                if focused.CurrentClassName().ok()? != WORD_DOCUMENT_CLASS {
                    return None;
                }
                let pattern: IUIAutomationTextPattern =
                    focused.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
                let at = pattern.DocumentRange().ok()?;
                at.MoveEndpointByRange(
                    TextPatternRangeEndpoint_End,
                    &at,
                    TextPatternRangeEndpoint_Start,
                )
                .ok()?;
                if count > 0 && at.Move(TextUnit_Paragraph, count).ok()? != count {
                    return None;
                }
                at.Select().ok()
            }
        };
        moved().is_some()
    }

    /// The most of a header cell's text read.
    const MAX_CELL_CHARS: i32 = 200;

    /// A cell's text without Word's end-of-cell and paragraph marks.
    fn cell_text(text: &str) -> String {
        text.trim_matches(|c: char| c.is_control() || c.is_whitespace())
            .to_owned()
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
            // Only the caret is counted from the document's start; the selection's end and the
            // caret's word are counted from the caret, over their own short texts. Each count
            // from the start had the text up to there sent again, four times per read and per
            // poll after a key (performance review 2026-10-23, P3).
            let start = offset_of(&document, &selection, TextPatternRangeEndpoint_Start)?;
            let end = start + chars_of(&selection)?;
            // The caret, as a range of its own.
            let caret = selection.Clone().ok()?;
            caret
                .MoveEndpointByRange(
                    TextPatternRangeEndpoint_End,
                    &selection,
                    TextPatternRangeEndpoint_Start,
                )
                .ok()?;
            // The word at the caret by the field's own word unit.
            let word = unit_at(
                &caret,
                start,
                windows::Win32::UI::Accessibility::TextUnit_Word,
            );
            let structure = word_structure(uia, &focused, &pattern, &caret, start);
            Some(TextContext {
                text,
                caret: start,
                selection: (start < end).then_some((start, end)),
                font: uia_font(&selection),
                word,
                misspelled: Vec::new(),
                structure,
            })
        }
    }

    /// The font of the character at `range`'s start, from UIA text attributes. Attributes
    /// that vary over the range (UIA's "mixed" value) or are unsupported are left out.
    fn uia_font(range: &IUIAutomationTextRange) -> Option<okur_core::FontInfo> {
        use windows::Win32::System::Variant::VARIANT;
        use windows::Win32::UI::Accessibility::{
            TextUnit_Character, UIA_BackgroundColorAttributeId, UIA_FontNameAttributeId,
            UIA_FontSizeAttributeId, UIA_FontWeightAttributeId, UIA_ForegroundColorAttributeId,
            UIA_HorizontalTextAlignmentAttributeId, UIA_IsItalicAttributeId,
            UIA_StyleNameAttributeId, UIA_UnderlineStyleAttributeId, UIA_TEXTATTRIBUTE_ID,
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
                color: int(UIA_ForegroundColorAttributeId).map(colorref),
                background: int(UIA_BackgroundColorAttributeId).map(colorref),
                style: get(UIA_StyleNameAttributeId)
                    .and_then(|v| windows::core::BSTR::try_from(&v).ok())
                    .map(|b| b.to_string())
                    .filter(|s| !s.is_empty()),
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
            let (color, background) = edit_system_colors(hwnd);
            Some(okur_core::FontInfo {
                family,
                // Whole points, as a font dialog sets them.
                size_tenths: (points_tenths + 5) / 10 * 10,
                bold: logfont.lfWeight >= 600,
                italic: logfont.lfItalic != 0,
                underline: logfont.lfUnderline != 0,
                alignment: None,
                color: Some(color),
                background: Some(background),
                style: None,
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

    /// A COLORREF holds blue, green and red from its high byte down.
    #[cfg(windows)]
    #[test]
    fn a_colorref_is_a_color() {
        assert_eq!(
            reader::colorref(0x00FF_8000),
            okur_core::color::Rgb {
                r: 0,
                g: 0x80,
                b: 0xFF
            }
        );
        assert_eq!(
            reader::colorref(0x00FF_FFFF),
            okur_core::color::Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
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
            misspelled: Vec::new(),
            structure: None,
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
