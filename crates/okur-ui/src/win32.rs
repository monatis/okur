//! The Windows renderer.
//!
//! The Virtual Viewer's window is drawn as JAWS's is (JAWS 2025 on the dev box, 2026-10-17,
//! see `docs/references/virtual-viewer.md`): a window titled "Virtual Viewer", topmost, at
//! three quarters of the screen and centred, with the text in a read-only rich edit control
//! (Arial 12 point) and a status bar saying "Ready". It shows without taking the focus: the
//! screen reader keeps the keys. A red frame marks what was read last, where JAWS draws its
//! reading highlight. When the window is activated (a click, Alt+Tab) or closed, it hides and
//! says so, and the screen reader ends its viewer, as JAWS does.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::sync::Mutex;

use okur_ui_model::{decode, encode, utf16_offset, FromUi, TextSpan, ToUi};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, POINTL, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, ClientToScreen, CreateFontW, CreateSolidBrush, DeleteObject, EndPaint, FillRect,
    GetDC, GetMonitorInfoW, GetTextMetricsW, InvalidateRect, MonitorFromWindow, ReleaseDC,
    SelectObject, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, COLOR_WINDOW, DEFAULT_CHARSET,
    DEFAULT_PITCH, FF_SWISS, FW_NORMAL, HBRUSH, HFONT, HGDIOBJ, MONITORINFO,
    MONITOR_DEFAULTTOPRIMARY, OUT_DEFAULT_PRECIS, PAINTSTRUCT, TEXTMETRICW,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, LoadLibraryW};
use windows::Win32::UI::Controls::RichEdit::{
    EM_EXLINEFROMCHAR, EM_SETEVENTMASK, EM_SETTARGETDEVICE, ENM_SCROLL, ENM_UPDATE, MSFTEDIT_CLASS,
};
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, EM_GETFIRSTVISIBLELINE, EM_LINESCROLL, EM_POSFROMCHAR, EM_SETMARGINS,
    EM_SETSEL, ICC_BAR_CLASSES, INITCOMMONCONTROLSEX, SBARS_SIZEGRIP, STATUSCLASSNAMEW,
};
use windows::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetForegroundWindow,
    GetMessageW, GetWindowRect, LoadCursorW, MoveWindow, PostMessageW, PostQuitMessage,
    RegisterClassExW, SendMessageW, SetLayeredWindowAttributes, SetWindowPos, SetWindowTextW,
    ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, EC_LEFTMARGIN, EC_RIGHTMARGIN,
    ES_AUTOVSCROLL, ES_MULTILINE, ES_READONLY, HWND_MESSAGE, HWND_TOPMOST, IDC_ARROW, LWA_COLORKEY,
    MA_NOACTIVATE, MSG, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, WA_INACTIVE, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_ACTIVATE, WM_APP, WM_CLOSE, WM_COMMAND, WM_DPICHANGED, WM_MOUSEACTIVATE,
    WM_MOVE, WM_NCHITTEST, WM_PAINT, WM_SETFONT, WM_SIZE, WNDCLASSEXW, WS_CHILD, WS_EX_CLIENTEDGE,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_OVERLAPPEDWINDOW, WS_POPUP, WS_VISIBLE, WS_VSCROLL,
};

/// Messages from Okur wait here for the window thread.
static QUEUE: Mutex<VecDeque<ToUi>> = Mutex::new(VecDeque::new());

/// Posted to the control window: messages are waiting in [`QUEUE`].
const WM_OKUR_MESSAGES: u32 = WM_APP + 1;
/// Posted to the control window: Okur closed our input.
const WM_OKUR_EOF: u32 = WM_APP + 2;

/// JAWS's reading highlight: this red, one pixel wide at 100% scaling (measured on a
/// screenshot of JAWS 2025, dev box, 2026-10-17).
const HIGHLIGHT_RED: COLORREF = COLORREF(0x005C_73FF);
/// The highlight window's colour key: what is painted in it shows the window behind.
const TRANSPARENT_KEY: COLORREF = COLORREF(0x00FF_00FF);
/// The font JAWS's viewer shows its text in, and its size in points.
const FONT_NAME: PCWSTR = w!("Arial");
const FONT_POINTS: i32 = 12;
/// EN_UPDATE and EN_VSCROLL: the rich edit control scrolled or is about to redraw.
const EN_UPDATE: u32 = 0x0400;
const EN_VSCROLL: u32 = 0x0602;

/// A window procedure.
type WindowProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

/// The viewer's windows and what they show.
struct Viewer {
    frame: HWND,
    edit: HWND,
    status: HWND,
    highlight: HWND,
    font: HFONT,
    /// One line's height and a character's average width, in the font, in pixels.
    line_height: i32,
    char_width: i32,
    lines: Vec<String>,
    mark: TextSpan,
}

thread_local! {
    static VIEWER: RefCell<Option<Viewer>> = const { RefCell::new(None) };
}

/// The viewer's window handles, copied out so that no borrow is held while Windows calls
/// back into the window procedures.
#[derive(Clone, Copy)]
struct Handles {
    frame: HWND,
    edit: HWND,
    status: HWND,
    highlight: HWND,
    line_height: i32,
    char_width: i32,
}

fn handles() -> Option<Handles> {
    VIEWER.with(|v| {
        v.try_borrow().ok().and_then(|v| {
            v.as_ref().map(|v| Handles {
                frame: v.frame,
                edit: v.edit,
                status: v.status,
                highlight: v.highlight,
                line_height: v.line_height,
                char_width: v.char_width,
            })
        })
    })
}

/// Where the mark starts and ends, in the rich edit control's positions.
fn mark_range() -> Option<(usize, usize)> {
    VIEWER.with(|v| {
        v.try_borrow().ok().and_then(|v| {
            v.as_ref().map(|v| {
                let m = v.mark;
                (
                    utf16_offset(&v.lines, m.line, m.column),
                    utf16_offset(&v.lines, m.line, m.column + m.length),
                )
            })
        })
    })
}

/// Tells Okur what the user did.
fn tell(message: &FromUi) {
    if let Ok(line) = encode(message) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn low_word(value: usize) -> u32 {
    u32::try_from(value & 0xFFFF).unwrap_or(0)
}

/// Draws Okur's windows until Okur closes our input.
pub fn run() {
    // SAFETY: process-wide setup calls with constant arguments, made before any window.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if LoadLibraryW(w!("Msftedit.dll")).is_err() {
            eprintln!("okur-ui: Msftedit.dll did not load");
        }
        let controls = INITCOMMONCONTROLSEX {
            dwSize: u32::try_from(std::mem::size_of::<INITCOMMONCONTROLSEX>()).unwrap_or(0),
            dwICC: ICC_BAR_CLASSES,
        };
        let _ = InitCommonControlsEx(&controls);
    }
    let Some(instance) = register_classes() else {
        eprintln!("okur-ui: window classes could not be registered");
        return;
    };
    // SAFETY: a message-only window of the class registered above.
    let control = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("OkurUiControl"),
            PCWSTR::null(),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance),
            None,
        )
    };
    let Ok(control) = control else {
        eprintln!("okur-ui: the control window could not be created");
        return;
    };
    // The window handle crosses to the reader thread as a number: posting to it is safe from
    // any thread.
    let control_id = control.0 as isize;
    let reader = std::thread::Builder::new()
        .name("okur-ui input".to_owned())
        .spawn(move || {
            let post = |message: u32| {
                // SAFETY: posting to our control window, which lives until the loop ends.
                let _ = unsafe {
                    PostMessageW(
                        Some(HWND(control_id as *mut _)),
                        message,
                        WPARAM(0),
                        LPARAM(0),
                    )
                };
            };
            for line in std::io::stdin().lock().lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                match decode::<ToUi>(&line) {
                    Ok(message) => {
                        if let Ok(mut queue) = QUEUE.lock() {
                            queue.push_back(message);
                        }
                        post(WM_OKUR_MESSAGES);
                    }
                    Err(e) => eprintln!("okur-ui: unreadable message {line:?}: {e}"),
                }
            }
            post(WM_OKUR_EOF);
        });
    if reader.is_err() {
        eprintln!("okur-ui: the input thread could not start");
        return;
    }
    let mut msg = MSG::default();
    // SAFETY: the standard message loop of this thread's windows.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Registers the window classes; returns the module's instance.
fn register_classes() -> Option<HINSTANCE> {
    // SAFETY: class registration with static names and window procedures defined below.
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).ok()?.into();
        let cursor = LoadCursorW(None, IDC_ARROW).ok()?;
        let size = u32::try_from(std::mem::size_of::<WNDCLASSEXW>()).unwrap_or(0);
        let classes: [(PCWSTR, WindowProc, HBRUSH); 3] = [
            (w!("OkurUiControl"), control_proc, HBRUSH::default()),
            (
                w!("OkurVirtualViewer"),
                viewer_proc,
                HBRUSH((COLOR_WINDOW.0 + 1) as isize as *mut _),
            ),
            (w!("OkurHighlight"), highlight_proc, HBRUSH::default()),
        ];
        for (name, proc, background) in classes {
            let class = WNDCLASSEXW {
                cbSize: size,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(proc),
                hInstance: instance,
                hCursor: cursor,
                hbrBackground: background,
                lpszClassName: name,
                ..Default::default()
            };
            if RegisterClassExW(&class) == 0 {
                return None;
            }
        }
        Some(instance)
    }
}

/// The message-only window that receives Okur's messages from the input thread.
unsafe extern "system" fn control_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_OKUR_MESSAGES => {
            loop {
                let next = QUEUE.lock().ok().and_then(|mut q| q.pop_front());
                let Some(message) = next else { break };
                handle(message);
            }
            LRESULT(0)
        }
        WM_OKUR_EOF => {
            // SAFETY: ends this thread's message loop.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn handle(message: ToUi) {
    match message {
        ToUi::ViewerShow { title, lines, mark } => show_viewer(&title, lines, mark),
        ToUi::ViewerMark { mark } => {
            VIEWER.with(|v| {
                if let Ok(mut v) = v.try_borrow_mut() {
                    if let Some(v) = v.as_mut() {
                        v.mark = mark;
                    }
                }
            });
            apply_mark();
        }
        ToUi::ViewerClose => hide_viewer(),
    }
}

fn show_viewer(title: &str, lines: Vec<String>, mark: TextSpan) {
    if handles().is_none() {
        create_viewer();
    }
    let Some(h) = handles() else { return };
    let text = wide(&lines.join("\r"));
    VIEWER.with(|v| {
        if let Ok(mut v) = v.try_borrow_mut() {
            if let Some(v) = v.as_mut() {
                v.lines = lines;
                v.mark = mark;
            }
        }
    });
    let title = wide(title);
    // SAFETY: our own windows; the text buffers outlive the calls.
    unsafe {
        let _ = SetWindowTextW(h.frame, PCWSTR(title.as_ptr()));
        let _ = SetWindowTextW(h.edit, PCWSTR(text.as_ptr()));
        // Three quarters of the screen the user is working on, centred, as JAWS's.
        let monitor = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: u32::try_from(std::mem::size_of::<MONITORINFO>()).unwrap_or(0),
            ..Default::default()
        };
        let area = if GetMonitorInfoW(monitor, &mut info).as_bool() {
            info.rcMonitor
        } else {
            RECT {
                left: 0,
                top: 0,
                right: 1280,
                bottom: 720,
            }
        };
        let (width, height) = (area.right - area.left, area.bottom - area.top);
        let _ = SetWindowPos(
            h.frame,
            Some(HWND_TOPMOST),
            area.left + width / 8,
            area.top + height / 8,
            width * 3 / 4,
            height * 3 / 4,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
    layout();
    apply_mark();
}

fn hide_viewer() {
    if let Some(h) = handles() {
        // SAFETY: hiding our own windows.
        unsafe {
            let _ = ShowWindow(h.highlight, SW_HIDE);
            let _ = ShowWindow(h.frame, SW_HIDE);
        }
    }
}

/// Creates the viewer's windows, hidden.
fn create_viewer() {
    // SAFETY: creating our own windows of the classes registered in `run`, and a font.
    unsafe {
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let instance: HINSTANCE = instance.into();
        let Ok(frame) = CreateWindowExW(
            WS_EX_TOPMOST,
            w!("OkurVirtualViewer"),
            w!("Virtual Viewer"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            800,
            600,
            None,
            None,
            Some(instance),
            None,
        ) else {
            return;
        };
        let edit_style = WS_CHILD
            | WS_VISIBLE
            | WS_VSCROLL
            | WINDOW_STYLE((ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL) as u32);
        let Ok(edit) = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            MSFTEDIT_CLASS,
            PCWSTR::null(),
            edit_style,
            0,
            0,
            0,
            0,
            Some(frame),
            None,
            Some(instance),
            None,
        ) else {
            return;
        };
        let status = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            STATUSCLASSNAMEW,
            w!("Ready"),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(SBARS_SIZEGRIP),
            0,
            0,
            0,
            0,
            Some(frame),
            None,
            Some(instance),
            None,
        )
        .unwrap_or_default();
        let Ok(highlight) = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("OkurHighlight"),
            PCWSTR::null(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            // Not owned by the viewer, so that the viewer's accessibility tree holds only its
            // text and status bar, as JAWS's does; placed above it each time it moves.
            None,
            None,
            Some(instance),
            None,
        ) else {
            return;
        };
        let _ = SetLayeredWindowAttributes(highlight, TRANSPARENT_KEY, 0, LWA_COLORKEY);
        // Long lines wrap at the window's edge; the frame learns when the text scrolls.
        SendMessageW(edit, EM_SETTARGETDEVICE, Some(WPARAM(0)), Some(LPARAM(0)));
        SendMessageW(
            edit,
            EM_SETEVENTMASK,
            Some(WPARAM(0)),
            Some(LPARAM((ENM_SCROLL | ENM_UPDATE) as isize)),
        );
        VIEWER.with(|v| {
            if let Ok(mut v) = v.try_borrow_mut() {
                *v = Some(Viewer {
                    frame,
                    edit,
                    status,
                    highlight,
                    font: HFONT::default(),
                    line_height: 16,
                    char_width: 8,
                    lines: Vec::new(),
                    mark: TextSpan::default(),
                });
            }
        });
        set_font(GetDpiForWindow(frame));
    }
}

/// Gives the text its font at `dpi`, and measures it.
fn set_font(dpi: u32) {
    let Some(h) = handles() else { return };
    let dpi = i32::try_from(dpi).unwrap_or(96).max(96);
    // SAFETY: a font for our rich edit control; the old one is deleted after the switch.
    unsafe {
        let font = CreateFontW(
            -(FONT_POINTS * dpi / 72),
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            u32::from(DEFAULT_PITCH.0 | FF_SWISS.0),
            FONT_NAME,
        );
        SendMessageW(
            h.edit,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
        // A small margin on both sides, as JAWS's viewer keeps its text off the edge.
        let margin = isize::try_from(4 * dpi / 96).unwrap_or(4);
        SendMessageW(
            h.edit,
            EM_SETMARGINS,
            Some(WPARAM((EC_LEFTMARGIN | EC_RIGHTMARGIN) as usize)),
            Some(LPARAM(margin | (margin << 16))),
        );
        let dc = GetDC(Some(h.edit));
        let old = SelectObject(dc, HGDIOBJ(font.0));
        let mut metrics = TEXTMETRICW::default();
        let measured = GetTextMetricsW(dc, &mut metrics).as_bool();
        SelectObject(dc, old);
        ReleaseDC(Some(h.edit), dc);
        let previous = VIEWER.with(|v| {
            v.try_borrow_mut().ok().and_then(|mut v| {
                v.as_mut().map(|v| {
                    if measured {
                        v.line_height = metrics.tmHeight;
                        v.char_width = metrics.tmAveCharWidth;
                    }
                    std::mem::replace(&mut v.font, font)
                })
            })
        });
        if let Some(previous) = previous.filter(|f| !f.is_invalid()) {
            let _ = DeleteObject(HGDIOBJ(previous.0));
        }
    }
}

/// Fits the text and the status bar to the window.
fn layout() {
    let Some(h) = handles() else { return };
    // SAFETY: sizing our own child windows.
    unsafe {
        SendMessageW(h.status, WM_SIZE, Some(WPARAM(0)), Some(LPARAM(0)));
        let mut client = RECT::default();
        let mut status = RECT::default();
        if GetClientRect(h.frame, &mut client).is_err() {
            return;
        }
        let status_height = if GetWindowRect(h.status, &mut status).is_ok() {
            status.bottom - status.top
        } else {
            0
        };
        let _ = MoveWindow(
            h.edit,
            0,
            0,
            client.right,
            (client.bottom - status_height).max(0),
            true,
        );
    }
}

/// Puts the caret at the mark, scrolls it into view, and frames it.
fn apply_mark() {
    let (Some(h), Some((start, _))) = (handles(), mark_range()) else {
        return;
    };
    let start_param = isize::try_from(start).unwrap_or(0);
    // SAFETY: messages to our own rich edit control.
    unsafe {
        SendMessageW(
            h.edit,
            EM_SETSEL,
            Some(WPARAM(start)),
            Some(LPARAM(start_param)),
        );
        // Bring the mark's line into view, a third of the way down when it was out of it.
        let line = SendMessageW(
            h.edit,
            EM_EXLINEFROMCHAR,
            Some(WPARAM(0)),
            Some(LPARAM(start_param)),
        )
        .0;
        let first = SendMessageW(h.edit, EM_GETFIRSTVISIBLELINE, None, None).0;
        let mut client = RECT::default();
        let _ = GetClientRect(h.edit, &mut client);
        let rows = isize::try_from((client.bottom / h.line_height.max(1)).max(1)).unwrap_or(1);
        if line < first || line >= first + rows {
            let target = (line - rows / 3).max(0);
            SendMessageW(
                h.edit,
                EM_LINESCROLL,
                Some(WPARAM(0)),
                Some(LPARAM(target - first)),
            );
        }
    }
    frame_mark();
}

/// The position of the character at `index`, in the rich edit control's client coordinates.
fn position(edit: HWND, index: usize) -> POINT {
    let mut point = POINTL::default();
    // SAFETY: EM_POSFROMCHAR fills the POINTL passed by pointer (rich edit's form).
    unsafe {
        SendMessageW(
            edit,
            EM_POSFROMCHAR,
            Some(WPARAM(&mut point as *mut POINTL as usize)),
            Some(LPARAM(isize::try_from(index).unwrap_or(0))),
        );
    }
    POINT {
        x: point.x,
        y: point.y,
    }
}

/// Places the red frame around the mark, or hides it when the mark is out of view.
fn frame_mark() {
    let (Some(h), Some((start, end))) = (handles(), mark_range()) else {
        return;
    };
    // SAFETY: geometry queries on our own windows, and placing our own highlight window.
    unsafe {
        if !windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(h.frame).as_bool() {
            return;
        }
        let first = position(h.edit, start);
        let last = position(h.edit, end.max(start + 1) - 1);
        let after = position(h.edit, end);
        let mut client = RECT::default();
        let _ = GetClientRect(h.edit, &mut client);
        let line_height = h.line_height.max(1);
        let (left, right, bottom) = if last.y == first.y {
            // One row: up to the next character, or past the last one on the line.
            let right = if end > start && after.y == first.y && after.x > first.x {
                after.x
            } else if end > start {
                last.x + h.char_width
            } else {
                first.x + h.char_width
            };
            (first.x, right, first.y + line_height)
        } else {
            // A line wrapped over several rows: all of them.
            (client.left, client.right, last.y + line_height)
        };
        let dpi = i32::try_from(GetDpiForWindow(h.frame))
            .unwrap_or(96)
            .max(96);
        let pad = 2 * dpi / 96;
        let visible = first.y >= 0 && bottom <= client.bottom + line_height / 2;
        if !visible {
            let _ = ShowWindow(h.highlight, SW_HIDE);
            return;
        }
        let mut top_left = POINT {
            x: left - pad,
            y: first.y - pad,
        };
        let mut bottom_right = POINT {
            x: right + pad,
            y: bottom + pad,
        };
        let _ = ClientToScreen(h.edit, &mut top_left);
        let _ = ClientToScreen(h.edit, &mut bottom_right);
        let _ = SetWindowPos(
            h.highlight,
            Some(HWND_TOPMOST),
            top_left.x,
            top_left.y,
            bottom_right.x - top_left.x,
            bottom_right.y - top_left.y,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        let _ = InvalidateRect(Some(h.highlight), None, true);
    }
}

/// Hides the viewer and tells Okur, once: activation can arrive more than once.
fn dismiss(frame: HWND) {
    // SAFETY: a state query on our own window.
    if unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(frame) }.as_bool() {
        hide_viewer();
        tell(&FromUi::ViewerDismissed);
    }
}

/// The viewer's top-level window.
unsafe extern "system" fn viewer_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // Activated by the user (a click, Alt+Tab), or closed: the viewer ends, as JAWS's.
        WM_ACTIVATE if low_word(wparam.0) != WA_INACTIVE => {
            dismiss(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            dismiss(hwnd);
            LRESULT(0)
        }
        WM_SIZE => {
            layout();
            frame_mark();
            LRESULT(0)
        }
        WM_MOVE => {
            frame_mark();
            LRESULT(0)
        }
        WM_COMMAND => {
            let code = (wparam.0 >> 16) & 0xFFFF;
            if code == EN_UPDATE as usize || code == EN_VSCROLL as usize {
                frame_mark();
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            set_font(low_word(wparam.0));
            // SAFETY: Windows passes the suggested new window rectangle in lparam.
            unsafe {
                let suggested = &*(lparam.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOACTIVATE,
                );
            }
            LRESULT(0)
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// The red frame: a click-through layered window whose inside is transparent.
unsafe extern "system" fn highlight_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            // SAFETY: painting our own window between BeginPaint and EndPaint; the brushes
            // are deleted after use.
            unsafe {
                let mut paint = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut paint);
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                let red = CreateSolidBrush(HIGHLIGHT_RED);
                let clear = CreateSolidBrush(TRANSPARENT_KEY);
                FillRect(dc, &rect, red);
                let width = (i32::try_from(GetDpiForWindow(hwnd)).unwrap_or(96) / 96).max(1);
                let inside = RECT {
                    left: rect.left + width,
                    top: rect.top + width,
                    right: rect.right - width,
                    bottom: rect.bottom - width,
                };
                FillRect(dc, &inside, clear);
                let _ = DeleteObject(HGDIOBJ(red.0));
                let _ = DeleteObject(HGDIOBJ(clear.0));
                let _ = EndPaint(hwnd, &paint);
            }
            LRESULT(0)
        }
        WM_NCHITTEST => LRESULT(-1),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
