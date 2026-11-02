//! Reads made right away on the calling thread for the harness: research views (`okur-lab
//! text-attributes`, `caret`, `tray`, `act`) and what the recorder saves beside a trial for
//! replay (Word's headings, links, paragraphs, view and status bar; Excel's place and lists).
//! The screen reader reads the same through `WindowsActions`, on its own threads.

/// Every UI Automation text attribute of the focused text at its caret, by `unit` around it
/// (0 character, 1 format, 2 word, 3 line, 4 paragraph): "id name: value" lines, for research
/// (`okur-lab text-attributes`). Empty without a text pattern.
#[cfg(windows)]
pub fn text_attributes_now(unit: i32) -> Vec<String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::{
        CUIAutomation8, IUIAutomation, IUIAutomationTextPattern, TextUnit, UIA_TextPatternId,
        UIA_TEXTATTRIBUTE_ID,
    };
    /// UI Automation's text attribute ids run from 40000 (AnimationStyle) to 40042
    /// (AfterParagraphSpacing) as of Windows 10.
    const FIRST: i32 = 40_000;
    const LAST: i32 = 40_042;
    let read = || -> Option<Vec<String>> {
        // SAFETY: COM initialization, in-process activation and UIA client calls on valid
        // objects.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let uia: IUIAutomation =
                CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).ok()?;
            let pattern: IUIAutomationTextPattern = uia
                .GetFocusedElement()
                .ok()?
                .GetCurrentPatternAs(UIA_TextPatternId)
                .ok()?;
            let range = pattern.GetSelection().ok()?.GetElement(0).ok()?;
            range.ExpandToEnclosingUnit(TextUnit(unit)).ok()?;
            let mixed = uia.ReservedMixedAttributeValue().ok();
            let unsupported = uia.ReservedNotSupportedValue().ok();
            Some(
                (FIRST..=LAST)
                    .filter_map(|id| {
                        let value: VARIANT =
                            range.GetAttributeValue(UIA_TEXTATTRIBUTE_ID(id)).ok()?;
                        let shown = match windows::core::IUnknown::try_from(&value) {
                            Ok(object) if Some(&object) == unsupported.as_ref() => return None,
                            Ok(object) if Some(&object) == mixed.as_ref() => "(mixed)".to_owned(),
                            _ => value.to_string(),
                        };
                        Some(format!("{id}: {shown}"))
                    })
                    .collect(),
            )
        }
    };
    read().unwrap_or_default()
}

/// The focused text field's text context right now, for harness checks (`okur-lab caret`).
#[cfg(windows)]
pub fn read_text_now() -> Option<okur_core::TextContext> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    // As the tracer decides: a page's field only with a document above the focus.
    if okur_windows::text_context::focus_is_in_page(&uia) {
        okur_windows::text_context::read_focused(&uia)
    } else {
        okur_windows::text_context::read_focused_outside_page(&uia)
    }
}
/// The headings of Word's document, read right away on the calling thread: the focused one,
/// or the one in the top-level window titled `window_title`. For harness checks and the
/// recorder; the engine goes through `WindowsActions`. `None` without Word's document.
#[cfg(windows)]
pub fn document_headings_now(
    window_title: Option<&str>,
) -> Option<Vec<okur_core::DocumentHeading>> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    let window = match window_title {
        Some(title) => Some(crate::harness::window_titled(title)?),
        None => None,
    };
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    okur_windows::text_context::word_document_headings(&uia, window)
}

/// The links of Word's document, read right away on the calling thread, as
/// [`document_headings_now`] reads its headings. For the recorder.
#[cfg(windows)]
pub fn document_links_now(window_title: Option<&str>) -> Option<Vec<okur_core::DocumentLink>> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    let window = match window_title {
        Some(title) => Some(crate::harness::window_titled(title)?),
        None => None,
    };
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    okur_windows::text_context::word_document_links(&uia, window)
}

/// The paragraphs of Word's document, read right away on the calling thread, as
/// [`document_headings_now`] reads its headings. For the recorder.
#[cfg(windows)]
pub fn document_paragraphs_now(
    window_title: Option<&str>,
) -> Option<Vec<okur_core::DocumentParagraph>> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    let window = match window_title {
        Some(title) => Some(crate::harness::window_titled(title)?),
        None => None,
    };
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    okur_windows::text_context::word_document_paragraphs(&uia, window)
}

/// The view Word's window shows its document in, read right away on the calling thread: the
/// window in front, or the top-level window titled `window_title`. For the recorder.
#[cfg(windows)]
pub fn document_view_now(window_title: Option<&str>) -> Option<String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    let window = match window_title {
        Some(title) => Some(crate::harness::window_titled(title)?),
        None => None,
    };
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    okur_windows::text_context::word_view(&uia, window)
}

/// Everything any ask of the place reads, for a recording's snapshot that answers them all in
/// replay: the active cell's row and column, the workbook's named titles, the active cell's
/// font and formatting (Insert+F) and its column's and row's sums (the totals readers).
#[cfg(windows)]
pub fn sheet_place_complete_now(window_title: Option<&str>) -> Option<okur_core::SheetPlace> {
    use okur_core::actions::SheetCells;
    let grid = sheet_grid(window_title)?;
    let mut place = okur_windows::excel_om::sheet_place(grid, SheetCells::Lines)?;
    let around = okur_windows::excel_om::sheet_place(grid, SheetCells::Around(None));
    if let Some(around) = around {
        (place.title_row, place.title_column) = (around.title_row, around.title_column);
    }
    place.format = okur_windows::excel_om::cell_format(grid);
    (place.column_sum, place.row_sum) = okur_windows::excel_om::cell_sums(grid);
    Some(place)
}

/// The items of the status bar of Word's window, read right away on the calling thread: the
/// window in front, or the top-level window titled `window_title`. For the recorder.
#[cfg(windows)]
pub fn status_bar_now(window_title: Option<&str>) -> Option<Vec<String>> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation};
    let window = match window_title {
        Some(title) => Some(crate::harness::window_titled(title)?),
        None => None,
    };
    // SAFETY: COM initialization and in-process activation.
    let uia: IUIAutomation = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    okur_windows::text_context::word_status_bar(&uia, window)
}

/// Performs an action on a node's live object right away, on the calling thread, reporting
/// why it failed. For harness checks; the engine goes through `WindowsActions`.
#[cfg(windows)]
pub fn perform_now(node: &okur_core::AomNode, activate: bool) -> Result<(), String> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use xa11y_windows::ia2::{perform_chromium_action, Ia2Action};
    let id = okur_windows::actions::unique_id(node).ok_or("node has no IA2 uniqueID")?;
    let action = if activate {
        Ia2Action::DefaultAction
    } else {
        Ia2Action::Focus
    };
    // SAFETY: COM initialization; repeated initialization is tolerated.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    // SAFETY: plain Win32 call with no preconditions.
    let window = unsafe { GetForegroundWindow() };
    perform_chromium_action(window, id, action).map_err(|e| format!("IA2 object {id}: {e}"))
}

/// The notification area's icons now, for harness checks (`okur-lab tray`).
#[cfg(windows)]
pub fn tray_icons_now() -> Vec<okur_core::event::TrayIcon> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    // SAFETY: COM initialization for this thread.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    okur_windows::uia_snapshot::capture::automation()
        .map(|uia| okur_windows::uia_snapshot::capture::tray_icons(&uia))
        .unwrap_or_default()
}

/// The taskbar's control tree, one element a line (indented by depth: control type, class,
/// name, rectangle), for finding its notification area on a new Windows (`okur-lab tray --dump`).
#[cfg(windows)]
pub fn taskbar_dump_now() -> Vec<String> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    // SAFETY: COM initialization for this thread.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let (Ok(uia), Some((taskbar, _))) = (
        okur_windows::uia_snapshot::capture::automation(),
        okur_windows::uia_snapshot::capture::taskbar_windows(),
    ) else {
        return Vec::new();
    };
    okur_windows::uia_snapshot::capture::taskbar_elements(&uia, taskbar)
        .into_iter()
        .map(|e| {
            let rect = e
                .rect
                .map(|r| format!("[{}, {}, {}, {}]", r.left, r.top, r.right, r.bottom))
                .unwrap_or_default();
            format!(
                "{}{} {} {:?} {rect}{}",
                "  ".repeat(e.depth),
                e.control_type,
                e.class_name.unwrap_or_default(),
                e.name.unwrap_or_default(),
                if e.offscreen { " offscreen" } else { "" }
            )
        })
        .collect()
}

/// Excel's grid window in the window in front, or in the top-level window titled
/// `window_title`.
#[cfg(windows)]
fn sheet_grid(window_title: Option<&str>) -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let top = match window_title {
        Some(title) => crate::harness::window_titled(title)?,
        // SAFETY: a plain query.
        None => unsafe { GetForegroundWindow() },
    };
    okur_windows::excel_om::grid_window(top)
}

/// One of JAWS's lists of Excel's window, with the active cell's column's letters, read right
/// away on the calling thread: the window in front, or the top-level window titled
/// `window_title`.
#[cfg(windows)]
pub fn sheet_list_now(
    window_title: Option<&str>,
    kind: okur_core::SheetListKind,
) -> Option<(String, Vec<okur_core::SheetEntry>)> {
    okur_windows::excel_om::sheet_list(sheet_grid(window_title)?, kind)
}
