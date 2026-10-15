//! What a window or control shows, for the Virtual Viewer: its visible elements and where
//! they are on screen, read through UI Automation when Virtualize Window or Virtualize
//! Current Control is pressed. [`okur_core::viewer_text`] lays them out as JAWS does: by
//! screen rows for classic windows, one element per line for modern apps.

#[cfg(windows)]
pub use reader::{control_lines, window_lines};

#[cfg(windows)]
mod reader {
    use crate::screen_lines::{
        class_of, control_text, owner_drawn, static_lines, symbol_font, without_accelerators,
    };
    use crate::uia_snapshot::capture::{boolean, int, string};
    use okur_core::aom::Role;
    use okur_core::viewer_text::{lines, ScreenItem, ViewerLayout};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Accessibility::*;

    /// Elements read at most: a window's visible controls, not a list's thousand items.
    const MAX_ELEMENTS: usize = 2000;
    /// Pixels a check box's or radio button's box takes beside its text.
    const BOX_INSET: i32 = 24;

    const BUTTON: i32 = 50000;
    const CHECKBOX: i32 = 50002;
    const COMBOBOX: i32 = 50003;
    const EDIT: i32 = 50004;
    const HYPERLINK: i32 = 50005;
    const IMAGE: i32 = 50006;
    const LISTITEM: i32 = 50007;
    const LIST: i32 = 50008;
    const MENU: i32 = 50009;
    const MENUBAR: i32 = 50010;
    const MENUITEM: i32 = 50011;
    const PROGRESSBAR: i32 = 50012;
    const RADIOBUTTON: i32 = 50013;
    const SCROLLBAR: i32 = 50014;
    const SLIDER: i32 = 50015;
    const SPINNER: i32 = 50016;
    const STATUSBAR: i32 = 50017;
    const TAB: i32 = 50018;
    const TABITEM: i32 = 50019;
    const TEXT: i32 = 50020;
    const TOOLTIP: i32 = 50022;
    const TREE: i32 = 50023;
    const TREEITEM: i32 = 50024;
    const GROUP: i32 = 50026;
    const THUMB: i32 = 50027;
    const DATAGRID: i32 = 50028;
    const DATAITEM: i32 = 50029;
    const DOCUMENT: i32 = 50030;
    const SPLITBUTTON: i32 = 50031;
    const HEADERITEM: i32 = 50035;
    const TABLE: i32 = 50036;
    const TITLEBAR: i32 = 50037;
    const SEPARATOR: i32 = 50038;

    /// What is read of each element.
    const PROPERTIES: &[UIA_PROPERTY_ID] = &[
        UIA_ControlTypePropertyId,
        UIA_NamePropertyId,
        UIA_ValueValuePropertyId,
        UIA_BoundingRectanglePropertyId,
        UIA_IsOffscreenPropertyId,
        UIA_IsEnabledPropertyId,
        UIA_IsPasswordPropertyId,
        UIA_NativeWindowHandlePropertyId,
        UIA_ClassNamePropertyId,
        UIA_FrameworkIdPropertyId,
        UIA_ToggleToggleStatePropertyId,
        UIA_SelectionItemIsSelectedPropertyId,
    ];

    struct Reader<'a> {
        walker: &'a IUIAutomationTreeWalker,
        request: &'a IUIAutomationCacheRequest,
        layout: ViewerLayout,
        budget: usize,
        items: Vec<ScreenItem>,
    }

    fn request(uia: &IUIAutomation) -> Option<IUIAutomationCacheRequest> {
        // SAFETY: building a cache request on a valid UIA client.
        unsafe {
            let request = uia.CreateCacheRequest().ok()?;
            for &prop in PROPERTIES {
                request.AddProperty(prop).ok()?;
            }
            request.SetTreeScope(TreeScope_Element).ok()?;
            Some(request)
        }
    }

    /// Classic windows are read by screen rows; modern app hosts (UWP, WinUI, WPF) by their
    /// elements, as JAWS reads them.
    fn layout_of(root: &IUIAutomationElement) -> ViewerLayout {
        let class = string(root, UIA_ClassNamePropertyId).unwrap_or_default();
        let framework = string(root, UIA_FrameworkIdPropertyId).unwrap_or_default();
        let modern = matches!(
            class.as_str(),
            "ApplicationFrameWindow"
                | "Windows.UI.Core.CoreWindow"
                | "WinUIDesktopWin32WindowClass"
        ) || matches!(framework.as_str(), "XAML" | "WPF" | "WinUI");
        if modern {
            ViewerLayout::Elements
        } else {
            ViewerLayout::Rows
        }
    }

    fn role_of(control_type: i32) -> Role {
        match control_type {
            BUTTON | SPLITBUTTON => Role::Button,
            CHECKBOX => Role::CheckBox,
            RADIOBUTTON => Role::RadioButton,
            COMBOBOX => Role::ComboBox,
            EDIT => Role::Edit,
            HYPERLINK => Role::Link { href: None },
            LISTITEM | DATAITEM => Role::ListItem,
            MENUITEM => Role::MenuItem,
            TABITEM => Role::Tab,
            TREEITEM => Role::TreeItem,
            GROUP => Role::Group,
            SLIDER => Role::Slider,
            PROGRESSBAR => Role::ProgressBar,
            _ => Role::Text,
        }
    }

    impl Reader<'_> {
        /// Reads `element` (fetched with the request) and what is below it.
        fn read(&mut self, element: &IUIAutomationElement, root: bool) {
            let Some(left) = self.budget.checked_sub(1) else {
                return;
            };
            self.budget = left;
            let control_type = int(element, UIA_ControlTypePropertyId).unwrap_or_default();
            // SAFETY: cached properties of a valid element.
            let offscreen = unsafe { element.CachedIsOffscreen() }.is_ok_and(|b| b.as_bool());
            // A modern app's frame draws its title bar as a window of its own.
            let title_bar = string(element, UIA_ClassNamePropertyId)
                .is_some_and(|class| class == "ApplicationFrameTitleBarWindow");
            if matches!(
                control_type,
                TITLEBAR | SCROLLBAR | THUMB | TOOLTIP | SEPARATOR | IMAGE
            ) || title_bar
                || (offscreen && !root)
            {
                return;
            }
            match self.layout {
                ViewerLayout::Rows => self.read_row_element(element, control_type),
                ViewerLayout::Elements => self.read_listed_element(element, control_type, root),
            }
        }

        fn children(&mut self, element: &IUIAutomationElement) {
            // SAFETY: tree walking with a cache request, on valid elements.
            let mut child = unsafe {
                self.walker
                    .GetFirstChildElementBuildCache(element, self.request)
            }
            .ok();
            while let Some(current) = child {
                if self.budget == 0 {
                    return;
                }
                self.read(&current, false);
                // SAFETY: as above.
                child = unsafe {
                    self.walker
                        .GetNextSiblingElementBuildCache(&current, self.request)
                }
                .ok();
            }
        }

        /// Rows: the text each control draws, at its place.
        fn read_row_element(&mut self, element: &IUIAutomationElement, control_type: i32) {
            match control_type {
                EDIT | COMBOBOX | SPINNER => {
                    // SAFETY: cached property of a valid element.
                    let password = unsafe { element.CachedIsPassword() }.is_ok_and(|b| b.as_bool());
                    if !password {
                        if let Some(value) = string(element, UIA_ValueValuePropertyId) {
                            self.push(element, control_type, value);
                        }
                    }
                }
                DOCUMENT => {}
                GROUP => {
                    // A group box draws its caption; other groups ("View Modes" in File
                    // Explorer) only name what is inside them.
                    // SAFETY: cached property of a valid element.
                    let boxed = unsafe { element.CachedNativeWindowHandle() }
                        .is_ok_and(|hwnd| !hwnd.is_invalid());
                    if let Some(name) = string(element, UIA_NamePropertyId).filter(|_| boxed) {
                        let mut item = self.item(element, control_type, name);
                        item.caption = true;
                        self.items.push(item);
                    }
                    self.children(element);
                }
                TEXT | BUTTON | SPLITBUTTON | CHECKBOX | RADIOBUTTON | HYPERLINK | MENUITEM
                | TABITEM | HEADERITEM => {
                    // A control shows its name; its parts (a column header's filter button)
                    // are read only when it has none.
                    match string(element, UIA_NamePropertyId) {
                        Some(name) => self.push(element, control_type, name),
                        None => self.children(element),
                    }
                }
                LISTITEM | TREEITEM | DATAITEM | STATUSBAR => {
                    // An item whose parts have text of their own (a list-view row's columns,
                    // a status bar's parts) is read through them.
                    let before = self.items.len();
                    self.children(element);
                    if self.items.len() == before {
                        if let Some(name) = string(element, UIA_NamePropertyId) {
                            self.push(element, control_type, name);
                        }
                    }
                }
                _ => self.children(element),
            }
        }

        /// Elements: one line per named element, in tree order.
        fn read_listed_element(
            &mut self,
            element: &IUIAutomationElement,
            control_type: i32,
            root: bool,
        ) {
            if root {
                self.children(element);
                return;
            }
            match control_type {
                BUTTON | CHECKBOX | RADIOBUTTON | HYPERLINK | MENUITEM | TABITEM | LISTITEM
                | TREEITEM | DATAITEM | SPLITBUTTON | COMBOBOX | EDIT | SLIDER | SPINNER | TEXT
                | HEADERITEM | PROGRESSBAR => {
                    if let Some(name) = string(element, UIA_NamePropertyId) {
                        self.push(element, control_type, name);
                    }
                }
                GROUP => {
                    if let Some(name) = string(element, UIA_NamePropertyId) {
                        self.push(element, control_type, name);
                    }
                    self.children(element);
                }
                _ => self.children(element),
            }
        }

        fn item(
            &self,
            element: &IUIAutomationElement,
            control_type: i32,
            text: String,
        ) -> ScreenItem {
            // SAFETY: cached properties of a valid element.
            let rect = unsafe { element.CachedBoundingRectangle() }.unwrap_or_default();
            let mut item = ScreenItem::new(
                role_of(control_type),
                text,
                (rect.left, rect.top, rect.right, rect.bottom),
            );
            // SAFETY: as above.
            item.unavailable = unsafe { element.CachedIsEnabled() }.is_ok_and(|b| !b.as_bool());
            item.checked = match control_type {
                CHECKBOX => int(element, UIA_ToggleToggleStatePropertyId).map(|s| s == 1),
                RADIOBUTTON => boolean(element, UIA_SelectionItemIsSelectedPropertyId),
                _ => None,
            };
            item
        }

        fn push(&mut self, element: &IUIAutomationElement, control_type: i32, text: String) {
            let mut item = self.item(element, control_type, text);
            // SAFETY: cached property of a valid element.
            let hwnd = unsafe { element.CachedNativeWindowHandle() }.unwrap_or_default();
            if !hwnd.is_invalid() && self.layout == ViewerLayout::Rows {
                let class = class_of(hwnd).to_ascii_uppercase();
                if class.contains("STATIC") || class.contains("BUTTON") {
                    // A classic label or button shows its window text: UI Automation names
                    // an unlabeled control after the label before it ("410 GB" for a colour
                    // square). One that draws itself, or draws in a symbol font, shows no
                    // text of it.
                    if owner_drawn(hwnd) || symbol_font(hwnd) {
                        return;
                    }
                    let shown = without_accelerators(&control_text(hwnd));
                    if shown.trim().is_empty() {
                        return;
                    }
                    // Wrapped where the control wraps it.
                    let inset = if matches!(control_type, CHECKBOX | RADIOBUTTON) {
                        BOX_INSET
                    } else {
                        0
                    };
                    item.lines = static_lines(hwnd, &shown, inset);
                    item.text = shown;
                } else if symbol_font(hwnd) {
                    return;
                }
            }
            self.items.push(item);
        }
    }

    fn read_lines(
        uia: &IUIAutomation,
        root: &IUIAutomationElement,
        title: Option<String>,
    ) -> Option<Vec<String>> {
        let request = request(uia)?;
        // SAFETY: plain UIA calls on a valid client.
        let walker = unsafe { uia.ControlViewWalker() }.ok()?;
        let root = unsafe { root.BuildUpdatedCache(&request) }.ok()?;
        let layout = layout_of(&root);
        let mut reader = Reader {
            walker: &walker,
            request: &request,
            layout,
            budget: MAX_ELEMENTS,
            items: Vec::new(),
        };
        reader.read(&root, true);
        Some(lines(title.as_deref(), layout, &reader.items))
    }

    /// Runs `read` with this thread aware of each monitor's DPI, so rectangles and fonts are
    /// in the same real pixels (text would otherwise wrap too early).
    fn dpi_aware<T>(read: impl FnOnce() -> T) -> T {
        use windows::Win32::UI::HiDpi::{
            SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        // SAFETY: changes this thread's DPI awareness, restored below.
        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        let result = read();
        // SAFETY: restores the awareness this thread had.
        unsafe { SetThreadDpiAwarenessContext(previous) };
        result
    }

    /// The Virtual Viewer's lines for window `hwnd` (Virtualize Window): its title, then what
    /// it shows.
    pub fn window_lines(uia: &IUIAutomation, hwnd: HWND) -> Option<Vec<String>> {
        dpi_aware(|| {
            // SAFETY: a UIA element for a window handle.
            let root = unsafe { uia.ElementFromHandle(hwnd) }.ok()?;
            // SAFETY: a current property of a valid element.
            let title = unsafe { root.CurrentName() }
                .ok()
                .map(|t| t.to_string())
                .filter(|t| !t.trim().is_empty());
            read_lines(uia, &root, title)
        })
    }

    /// The control `focused` is part of: a list's focused row is in the list, and the whole
    /// list is the control (JAWS 2025 shows only the list's last row there).
    fn control_of(uia: &IUIAutomation, focused: IUIAutomationElement) -> IUIAutomationElement {
        const LEVELS: usize = 8;
        // SAFETY: current properties and tree walking on valid elements.
        unsafe {
            let item = focused
                .CurrentControlType()
                .is_ok_and(|t| matches!(t.0, LISTITEM | DATAITEM | TREEITEM | TABITEM | MENUITEM));
            let walker = match uia.ControlViewWalker() {
                Ok(walker) if item => walker,
                _ => return focused,
            };
            let mut current = focused.clone();
            for _ in 0..LEVELS {
                let Ok(parent) = walker.GetParentElement(&current) else {
                    break;
                };
                let container = parent.CurrentControlType().is_ok_and(|t| {
                    matches!(t.0, LIST | DATAGRID | TABLE | TREE | TAB | MENU | MENUBAR)
                });
                if container {
                    return parent;
                }
                current = parent;
            }
            focused
        }
    }

    /// The Virtual Viewer's lines for the focused control (Virtualize Current Control).
    pub fn control_lines(uia: &IUIAutomation) -> Option<Vec<String>> {
        dpi_aware(|| {
            // SAFETY: plain UIA call on a valid client.
            let focused = unsafe { uia.GetFocusedElement() }.ok()?;
            read_lines(uia, &control_of(uia, focused), None)
        })
    }
}

/// The Virtual Viewer's lines for the window in front, or with `control`, its focused
/// control: `okur window-text`, to compare with JAWS's viewer.
#[cfg(windows)]
pub fn foreground_lines(control: bool) -> Result<Vec<String>, crate::PlatformError> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    // SAFETY: COM for this thread's UIA client.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let uia = crate::uia_snapshot::capture::automation()?;
    let lines = if control {
        control_lines(&uia)
    } else {
        // SAFETY: plain Win32 call with no preconditions.
        window_lines(&uia, unsafe { GetForegroundWindow() })
    };
    lines.ok_or_else(|| {
        crate::PlatformError::EventListenerError("the window could not be read".to_owned())
    })
}
