//! A dialog box's text as it is laid out on screen, one entry per visual row: what JAWS's
//! Virtual Viewer shows for Virtualize Window (Insert+Alt+W) on a dialog box. JAWS builds it
//! from its off-screen model; here it is built from the dialog's controls (Win32 child
//! windows), their screen rectangles, and static text wrapped as the control draws it.
//!
//! JAWS 2025 on the Run dialog, observed on the dev box (2026-10-15; research, a scenario on
//! old-pc records it):
//! `Run` | `Type the name of a program, folder, document, or` |
//! `Internet resource, and Windows will open it for you.` | `Open: C:` | `OK Cancel Browse...`

/// One piece of text at its place on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub left: i32,
    pub top: i32,
    pub bottom: i32,
    pub text: String,
}

/// Runs grouped into rows, top to bottom: a run joins the row whose span holds its middle,
/// and a row's runs read left to right.
pub fn rows(mut runs: Vec<Run>) -> Vec<String> {
    runs.retain(|r| !r.text.trim().is_empty());
    runs.sort_by_key(|r| (r.top, r.left));
    let mut rows: Vec<(i32, i32, Vec<Run>)> = Vec::new();
    for run in runs {
        let middle = (run.top + run.bottom) / 2;
        match rows
            .iter_mut()
            .find(|(top, bottom, _)| (*top..=*bottom).contains(&middle))
        {
            Some((_, _, row)) => row.push(run),
            None => rows.push((run.top, run.bottom.max(run.top), vec![run])),
        }
    }
    rows.into_iter()
        .map(|(_, _, mut row)| {
            row.sort_by_key(|r| r.left);
            row.iter()
                .map(|r| r.text.trim())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// Text broken into the lines a control of `width` pixels draws, at spaces, as
/// `DrawText`'s word break does; `measure` gives a string's width in pixels. A word wider
/// than the control stays whole on its own line.
pub fn wrap(text: &str, width: i32, measure: impl Fn(&str) -> i32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split(['\n', '\r']).filter(|p| !p.is_empty()) {
        let mut line = String::new();
        for word in paragraph.split(' ').filter(|w| !w.is_empty()) {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && measure(&candidate) > width {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

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

/// A dialog box as read from its controls.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DialogLayout {
    /// Its text by screen rows (the Virtual Viewer's lines).
    pub rows: Vec<String>,
    /// Its static text that labels no field, top to bottom: what JAWS reads after the
    /// dialog's title ("Run dialog | Type the name of a program, …").
    pub text: Vec<String>,
}

/// Static texts (`top`, `bottom`, text) that label no field: a static sharing a row with a
/// field (`top`, `bottom`) is that field's label ("Open:"), read with the field.
pub fn standalone(mut statics: Vec<(i32, i32, String)>, fields: &[(i32, i32)]) -> Vec<String> {
    statics.sort_by_key(|(top, _, _)| *top);
    statics
        .into_iter()
        .filter(|(top, bottom, _)| {
            !fields
                .iter()
                .any(|(field_top, field_bottom)| field_top < bottom && top < field_bottom)
        })
        .map(|(_, _, text)| text)
        .collect()
}

#[cfg(windows)]
pub(crate) use windows_lines::text_of as control_text;
#[cfg(windows)]
pub(crate) use windows_lines::{class_of, owner_drawn, static_lines, symbol_font};
#[cfg(windows)]
pub use windows_lines::{dialog_layout, group_box_of, page_of, page_window, window_bars};

#[cfg(windows)]
mod windows_lines {
    use super::{rows, standalone, without_accelerators, wrap, DialogLayout, Run};
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateFontIndirectW, DeleteDC, DeleteObject, GetObjectW,
        GetTextExtentPoint32W, SelectObject, HFONT, LOGFONTW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetClassNameW, GetClientRect, GetParent, GetWindowLongW, GetWindowRect,
        IsWindowVisible, SendMessageTimeoutW, GWL_STYLE, SMTO_ABORTIFHUNG,
    };

    const WM_GETTEXT: u32 = 0x000D;
    const WM_GETFONT: u32 = 0x0031;
    /// How long a control may take to answer a message.
    const TIMEOUT_MS: u32 = 200;

    pub(crate) fn class_of(hwnd: HWND) -> String {
        let mut buf = [0u16; 64];
        // SAFETY: valid buffer for the class name.
        let n = unsafe { GetClassNameW(hwnd, &mut buf) };
        String::from_utf16_lossy(buf.get(..usize::try_from(n).unwrap_or(0)).unwrap_or(&[]))
    }

    /// A control's text through WM_GETTEXT, which reaches edits in other processes.
    pub(crate) fn text_of(hwnd: HWND) -> String {
        let mut buf = vec![0u16; 2048];
        let mut copied = 0usize;
        // SAFETY: the buffer outlives the call and its length is passed with it.
        let sent = unsafe {
            SendMessageTimeoutW(
                hwnd,
                WM_GETTEXT,
                WPARAM(buf.len()),
                LPARAM(buf.as_mut_ptr() as isize),
                SMTO_ABORTIFHUNG,
                TIMEOUT_MS,
                Some(&mut copied),
            )
        };
        if sent.0 == 0 {
            return String::new();
        }
        String::from_utf16_lossy(buf.get(..copied.min(buf.len())).unwrap_or(&[]))
    }

    fn rect_of(hwnd: HWND) -> RECT {
        let mut rect = RECT::default();
        // SAFETY: valid out-pointer.
        let _ = unsafe { GetWindowRect(hwnd, &mut rect) };
        rect
    }

    /// A tab control's tabs as one row where its strip is: a property sheet's "General
    /// Security Details Previous Versions", which JAWS's view of the window shows under the
    /// title [obs, 323/s4, s7]. The tabs are not windows: MSAA gives them, with their places.
    fn tab_row(tabs: HWND) -> Option<Run> {
        let children = msaa_children(tabs);
        let (mut left, mut top, mut bottom) = (i32::MAX, i32::MAX, i32::MIN);
        let mut names = Vec::new();
        for (name, place) in children {
            if let Some((x, y, _, height)) = place {
                left = left.min(x);
                top = top.min(y);
                bottom = bottom.max(y + height);
            }
            names.push(without_accelerators(&name));
        }
        (!names.is_empty() && top <= bottom).then(|| Run {
            left,
            top,
            bottom,
            text: names.join(" "),
        })
    }

    /// Where an MSAA object is: left, top, width, height, in screen pixels.
    type Place = (i32, i32, i32, i32);

    /// The named children of a window's MSAA client object, with their places: a tab control's tabs, a status bar's parts. They are not windows.
    fn msaa_children(window: HWND) -> Vec<(String, Option<Place>)> {
        use windows::core::Interface;
        use windows::Win32::System::Variant::VARIANT;
        use windows::Win32::UI::Accessibility::{AccessibleObjectFromWindow, IAccessible};
        const OBJID_CLIENT: i32 = -4;
        let mut object: *mut core::ffi::c_void = std::ptr::null_mut();
        // SAFETY: MSAA's client object of a window handle, with a valid out-pointer.
        let found = unsafe {
            AccessibleObjectFromWindow(window, OBJID_CLIENT as u32, &IAccessible::IID, &mut object)
        };
        if found.is_err() || object.is_null() {
            return Vec::new();
        }
        // SAFETY: AccessibleObjectFromWindow returned an IAccessible for this IID.
        let accessible = unsafe { IAccessible::from_raw(object) };
        // SAFETY: plain MSAA query on the object just obtained.
        let count = unsafe { accessible.accChildCount() }.unwrap_or(0);
        let mut children = Vec::new();
        for id in 1..=count {
            let child = VARIANT::from(id);
            // SAFETY: plain MSAA queries with valid arguments and out-pointers.
            unsafe {
                let name = accessible
                    .get_accName(&child)
                    .map(|b| b.to_string())
                    .unwrap_or_default();
                if name.trim().is_empty() {
                    continue;
                }
                let (mut x, mut y, mut width, mut height) = (0, 0, 0, 0);
                let place = accessible
                    .accLocation(&mut x, &mut y, &mut width, &mut height, &child)
                    .is_ok()
                    .then_some((x, y, width, height));
                children.push((name, place));
            }
        }
        children
    }

    /// A focus's top-level window's menu bar items as written ("&File") and its status bar's
    /// parts: JAWS's view of the window shows them above and below its text [obs, 314/s10,
    /// s11]. Menus are read across processes by Win32; a status bar's parts through MSAA.
    pub fn window_bars(focus: HWND) -> (Vec<String>, Vec<String>) {
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{
            FindWindowExW, GetAncestor, GetMenu, GetMenuItemCount, GetMenuStringW, GA_ROOT,
            MF_BYPOSITION,
        };
        // SAFETY: plain Win32 queries on window and menu handles; buffers outlive the calls.
        unsafe {
            let root = GetAncestor(focus, GA_ROOT);
            if root.is_invalid() {
                return (Vec::new(), Vec::new());
            }
            let mut menu_bar = Vec::new();
            let menu = GetMenu(root);
            if !menu.is_invalid() {
                for i in 0..GetMenuItemCount(Some(menu)).max(0) {
                    let mut buf = [0u16; 128];
                    let n = GetMenuStringW(menu, i as u32, Some(&mut buf), MF_BYPOSITION);
                    let text = String::from_utf16_lossy(
                        buf.get(..usize::try_from(n).unwrap_or(0)).unwrap_or(&[]),
                    );
                    if !text.trim().is_empty() {
                        menu_bar.push(text);
                    }
                }
            }
            let status_bar = FindWindowExW(Some(root), None, w!("msctls_statusbar32"), None)
                .ok()
                .filter(|bar| IsWindowVisible(*bar).as_bool())
                .map(|bar| {
                    msaa_children(bar)
                        .into_iter()
                        .map(|(name, _)| name)
                        .collect()
                })
                .unwrap_or_default();
            (menu_bar, status_bar)
        }
    }

    /// A static control's lines as it draws them: its text wrapped at its width, less
    /// `inset` pixels (a check box's box), in its font.
    pub(crate) fn static_lines(hwnd: HWND, text: &str, inset: i32) -> Vec<String> {
        use windows::Win32::UI::HiDpi::{
            GetWindowDpiAwarenessContext, SetThreadDpiAwarenessContext,
        };
        let mut client = RECT::default();
        let mut font_handle = 0usize;
        // The control's width in its own units, those of its font: a window not aware of
        // the monitor's scaling is stretched, and its width scaled for a caller that is.
        // SAFETY: changes this thread's DPI awareness for one call, restored after it.
        let sized = unsafe {
            let previous = SetThreadDpiAwarenessContext(GetWindowDpiAwarenessContext(hwnd));
            let sized = GetClientRect(hwnd, &mut client).is_ok();
            SetThreadDpiAwarenessContext(previous);
            sized
        };
        // SAFETY: valid out-pointers; the message only reads the control's font handle.
        let ok = sized
            && unsafe {
                SendMessageTimeoutW(
                    hwnd,
                    WM_GETFONT,
                    WPARAM(0),
                    LPARAM(0),
                    SMTO_ABORTIFHUNG,
                    TIMEOUT_MS,
                    Some(&mut font_handle),
                )
                .0 != 0
            };
        let mut logfont = LOGFONTW::default();
        let size = i32::try_from(std::mem::size_of::<LOGFONTW>()).unwrap_or(0);
        // SAFETY: the font handle is only read, into a LOGFONT of the size given.
        let have_font = ok
            && font_handle != 0
            && unsafe {
                GetObjectW(
                    HFONT(font_handle as *mut core::ffi::c_void).into(),
                    size,
                    Some((&mut logfont as *mut LOGFONTW).cast()),
                )
            } != 0;
        if !have_font || client.right <= 0 {
            return vec![text.to_owned()];
        }
        // SAFETY: a memory DC and a font of our own, both released below.
        unsafe {
            let dc = CreateCompatibleDC(None);
            let font = CreateFontIndirectW(&logfont);
            let old = SelectObject(dc, font.into());
            let measure = |s: &str| {
                let wide: Vec<u16> = s.encode_utf16().collect();
                let mut extent = windows::Win32::Foundation::SIZE::default();
                let _ = GetTextExtentPoint32W(dc, &wide, &mut extent);
                extent.cx
            };
            let lines = wrap(text, client.right - inset, measure);
            SelectObject(dc, old);
            let _ = DeleteObject(font.into());
            let _ = DeleteDC(dc);
            lines
        }
    }

    /// Whether a Win32 label or button draws itself (`SS_OWNERDRAW`, `BS_OWNERDRAW`): its
    /// window text is then not what it shows (a drive's colour squares and pie chart).
    /// WinForms paints its own labels and buttons through the same style, from their window
    /// text, so they are not counted.
    pub(crate) fn owner_drawn(hwnd: HWND) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongW, GWL_STYLE};
        const SS_TYPEMASK: i32 = 0x1F;
        const SS_OWNERDRAW: i32 = 0x0D;
        const BS_TYPEMASK: i32 = 0x0F;
        const BS_OWNERDRAW: i32 = 0x0B;
        // SAFETY: plain Win32 query on a window handle.
        let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) };
        match class_of(hwnd).as_str() {
            "Static" => style & SS_TYPEMASK == SS_OWNERDRAW,
            "Button" => style & BS_TYPEMASK == BS_OWNERDRAW,
            _ => false,
        }
    }

    /// Whether a control draws its text in a symbol font (Wingdings and the like): its text
    /// is pictures, such as the colour squares beside a drive's used and free space, which
    /// read as letters ("z").
    pub(crate) fn symbol_font(hwnd: HWND) -> bool {
        const SYMBOL_CHARSET: u8 = 2;
        let mut font_handle = 0usize;
        // SAFETY: the message only returns the control's font handle.
        let sent = unsafe {
            SendMessageTimeoutW(
                hwnd,
                WM_GETFONT,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                TIMEOUT_MS,
                Some(&mut font_handle),
            )
        };
        if sent.0 == 0 || font_handle == 0 {
            return false;
        }
        let mut logfont = LOGFONTW::default();
        let size = i32::try_from(std::mem::size_of::<LOGFONTW>()).unwrap_or(0);
        // SAFETY: the font handle is only read, into a LOGFONT of the size given.
        let read = unsafe {
            GetObjectW(
                HFONT(font_handle as *mut core::ffi::c_void).into(),
                size,
                Some((&mut logfont as *mut LOGFONTW).cast()),
            )
        };
        read != 0 && logfont.lfCharSet.0 == SYMBOL_CHARSET
    }

    unsafe extern "system" fn collect(hwnd: HWND, children: LPARAM) -> BOOL {
        // SAFETY: LPARAM is the Vec passed to EnumChildWindows below, alive for the call.
        let children = unsafe { &mut *(children.0 as *mut Vec<HWND>) };
        children.push(hwnd);
        BOOL(1)
    }

    /// The property sheet page `focus` is on in `dialog`, by name ("General"): a dialog box
    /// inside the dialog box.
    pub fn page_of(focus: HWND, dialog: HWND) -> Option<String> {
        let page = page_window(focus, dialog)?;
        let name = text_of(page);
        (!name.trim().is_empty()).then(|| without_accelerators(&name))
    }

    /// The property page (a `#32770` inside the dialog) the focused window is on, if any:
    /// what Say Window reads under the dialog's title, without the sheet's OK, Cancel and
    /// Apply and its tabs [obs, 323/s2].
    pub fn page_window(focus: HWND, dialog: HWND) -> Option<HWND> {
        const LEVELS: usize = 16;
        let mut current = focus;
        for _ in 0..LEVELS {
            // SAFETY: plain Win32 query on a window handle.
            let parent = unsafe { GetParent(current) }.ok()?;
            if parent == dialog || parent.is_invalid() {
                return None;
            }
            if class_of(parent) == "#32770" {
                return Some(parent);
            }
            current = parent;
        }
        None
    }

    /// The group box around the focused window in a dialog: the smallest visible one whose
    /// rectangle holds the focus's, by its text without access-key marks. JAWS names it
    /// before every control in it: "Speech Rate: left right slider", "Sort Links In Tab
    /// Order radio button" (dev box, 2026-10-17). A group box is a sibling of the controls
    /// it frames, not their parent, in Win32 as in UI Automation, so only geometry tells.
    pub fn group_box_of(focus: HWND, dialog: HWND) -> Option<String> {
        const BS_TYPEMASK: i32 = 0xF;
        const BS_GROUPBOX: i32 = 7;
        let target = rect_of(focus);
        let mut children: Vec<HWND> = Vec::new();
        // SAFETY: the callback only pushes into the Vec, which outlives the call.
        let _ = unsafe {
            EnumChildWindows(
                Some(dialog),
                Some(collect),
                LPARAM(&mut children as *mut Vec<HWND> as isize),
            )
        };
        children
            .into_iter()
            .filter(|&child| child != focus && class_of(child) == "Button")
            // SAFETY: plain Win32 queries on a window handle.
            .filter(|&child| unsafe {
                IsWindowVisible(child).as_bool()
                    && GetWindowLongW(child, GWL_STYLE) & BS_TYPEMASK == BS_GROUPBOX
            })
            .map(|child| (child, rect_of(child)))
            .filter(|(_, r)| {
                r.left <= target.left
                    && r.top <= target.top
                    && r.right >= target.right
                    && r.bottom >= target.bottom
            })
            .min_by_key(|(_, r)| i64::from(r.right - r.left) * i64::from(r.bottom - r.top))
            .map(|(child, _)| without_accelerators(&text_of(child)))
            .filter(|name| !name.trim().is_empty())
    }

    /// A dialog's text by screen rows, and its text that labels no field.
    pub fn dialog_layout(dialog: HWND) -> DialogLayout {
        use windows::Win32::UI::HiDpi::{
            SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        // Rectangles in real pixels, as the controls' fonts are: a DPI-unaware thread is
        // given scaled-down rectangles, and text then wraps too early.
        // SAFETY: changes this thread's DPI awareness, restored below.
        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        let layout = read_layout(dialog);
        // SAFETY: restores the awareness this thread had.
        unsafe { SetThreadDpiAwarenessContext(previous) };
        layout
    }

    fn read_layout(dialog: HWND) -> DialogLayout {
        let mut children: Vec<HWND> = Vec::new();
        // SAFETY: the callback only pushes into the Vec, which outlives the call.
        let _ = unsafe {
            EnumChildWindows(
                Some(dialog),
                Some(collect),
                LPARAM(&mut children as *mut Vec<HWND> as isize),
            )
        };
        let mut runs = Vec::new();
        let mut statics: Vec<(i32, i32, String)> = Vec::new();
        let mut fields: Vec<(i32, i32)> = Vec::new();
        for child in children {
            // SAFETY: plain Win32 queries on a window handle.
            if !unsafe { IsWindowVisible(child) }.as_bool() {
                continue;
            }
            let class = class_of(child);
            // A property sheet's page holds controls that are enumerated themselves; as one
            // run it would join every row it covers.
            if class == "#32770" {
                continue;
            }
            // A combo box's edit is read through its combo box.
            // SAFETY: plain Win32 query.
            let parent = unsafe { GetParent(child) }.ok();
            if class == "Edit" && parent.is_some_and(|p| class_of(p) == "ComboBox") {
                continue;
            }
            // A page's text is not the dialog's: JAWS 2025 opens Local Disk (C:) Properties
            // with "General page", not the page's labels (dev box, 2026-10-15).
            let in_page = parent.is_some_and(|p| p != dialog && class_of(p) == "#32770");
            let rect = rect_of(child);
            if !matches!(class.as_str(), "Static" | "Button") {
                fields.push((rect.top, rect.bottom));
            }
            if class == "SysTabControl32" {
                runs.extend(tab_row(child));
                continue;
            }
            let text = text_of(child);
            if text.trim().is_empty() {
                continue;
            }
            match class.as_str() {
                "Static" => {
                    let text = without_accelerators(&text);
                    if !in_page {
                        statics.push((rect.top, rect.bottom, text.clone()));
                    }
                    let lines = static_lines(child, &text, 0);
                    let height =
                        (rect.bottom - rect.top) / i32::try_from(lines.len().max(1)).unwrap_or(1);
                    for (i, line) in lines.into_iter().enumerate() {
                        let top = rect.top + height * i32::try_from(i).unwrap_or(0);
                        runs.push(Run {
                            left: rect.left,
                            top,
                            bottom: top + height,
                            text: line,
                        });
                    }
                }
                "Button" => runs.push(Run {
                    left: rect.left,
                    top: rect.top,
                    bottom: rect.bottom,
                    text: without_accelerators(&text),
                }),
                _ => runs.push(Run {
                    left: rect.left,
                    top: rect.top,
                    bottom: rect.bottom,
                    text,
                }),
            }
        }
        DialogLayout {
            rows: rows(runs),
            text: standalone(statics, &fields),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(left: i32, top: i32, bottom: i32, text: &str) -> Run {
        Run {
            left,
            top,
            bottom,
            text: text.to_owned(),
        }
    }

    /// The Run dialog's layout, as JAWS's viewer reads it.
    #[test]
    fn controls_on_one_row_read_left_to_right() {
        let lines = rows(vec![
            run(300, 200, 216, "OK"),
            run(380, 200, 216, "Cancel"),
            run(
                60,
                20,
                36,
                "Type the name of a program, folder, document, or",
            ),
            run(
                60,
                36,
                52,
                "Internet resource, and Windows will open it for you.",
            ),
            run(110, 80, 100, "C:"),
            run(20, 83, 97, "Open:"),
            run(460, 200, 216, "Browse..."),
        ]);
        assert_eq!(
            lines,
            [
                "Type the name of a program, folder, document, or",
                "Internet resource, and Windows will open it for you.",
                "Open: C:",
                "OK Cancel Browse...",
            ]
        );
    }

    #[test]
    fn text_wraps_at_spaces_within_the_width() {
        // Every character 10 pixels wide.
        let measure = |s: &str| i32::try_from(s.chars().count()).unwrap_or(0) * 10;
        assert_eq!(
            wrap("one two three four", 90, measure),
            ["one two", "three", "four"]
        );
        assert_eq!(wrap("first\nsecond", 1000, measure), ["first", "second"]);
        assert_eq!(wrap("unbreakable", 30, measure), ["unbreakable"]);
    }

    #[test]
    fn a_field_label_is_not_dialog_text() {
        let text = standalone(
            vec![
                (83, 97, "Open:".to_owned()),
                (20, 52, "Type the name of a program.".to_owned()),
            ],
            &[(80, 100)],
        );
        assert_eq!(text, ["Type the name of a program."]);
    }

    #[test]
    fn accelerator_marks_are_not_shown() {
        assert_eq!(without_accelerators("&Open:"), "Open:");
        assert_eq!(without_accelerators("Save && Exit"), "Save & Exit");
    }
}
