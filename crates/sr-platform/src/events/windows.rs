use crate::PlatformError;
use sr_core::event::{AccessibilityEvent, FocusEvent, NodeStates, Role};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tokio::sync::mpsc::UnboundedSender;
use windows::core::implement;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationFocusChangedEventHandler,
    IUIAutomationFocusChangedEventHandler_Impl, SetWinEventHook, TreeScope_Children,
    UIA_ButtonControlTypeId, UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId,
    UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_HeaderControlTypeId,
    UIA_HeaderItemControlTypeId, UIA_HyperlinkControlTypeId, UIA_ListControlTypeId,
    UIA_ListItemControlTypeId, UIA_PositionInSetPropertyId, UIA_ProgressBarControlTypeId,
    UIA_RadioButtonControlTypeId, UIA_SizeOfSetPropertyId, UIA_SliderControlTypeId,
    UIA_TabControlTypeId, UIA_TabItemControlTypeId, UIA_TableControlTypeId, UIA_TextControlTypeId,
    UIA_TreeControlTypeId, UIA_TreeItemControlTypeId, UnhookWinEvent, HWINEVENTHOOK,
};

const WINEVENT_OUTOFCONTEXT: u32 = 0;
const WINEVENT_SKIPOWNPROCESS: u32 = 2;
const EVENT_OBJECT_FOCUS: u32 = 0x8005;
const EVENT_OBJECT_SELECTION: u32 = 0x8006;
const OBJID_CLIENT: i32 = -4;

use super::AccessibilityEventListener;

#[link(name = "user32")]
extern "system" {
    fn OpenDesktopW(
        lpszDesktop: *const u16,
        dwFlags: u32,
        fInherit: i32,
        dwDesiredAccess: u32,
    ) -> *mut c_void;
    fn SetThreadDesktop(hDesktop: *mut c_void) -> i32;
}

#[implement(IUIAutomationFocusChangedEventHandler)]
struct FocusChangedHandler {
    uia: IUIAutomation,
    sender: UnboundedSender<AccessibilityEvent>,
    last_loaded_document: std::sync::Mutex<Option<String>>,
}

fn extract_uia_document_tree(
    uia: &IUIAutomation,
    element: &IUIAutomationElement,
    doc_title: &str,
) -> sr_core::AomNode {
    let mut next_id = 1;
    let mut root = sr_core::AomNode::new(0, Role::Document);
    root.name = Some(doc_title.to_string());

    fn populate_children(
        uia: &IUIAutomation,
        element: &IUIAutomationElement,
        parent_node: &mut sr_core::AomNode,
        next_id: &mut usize,
        depth: usize,
    ) {
        if depth == 0 || *next_id > 5000 {
            return;
        }
        let cond = match unsafe { uia.CreateTrueCondition() } {
            Ok(c) => c,
            Err(_) => return,
        };
        let children = match unsafe { element.FindAll(TreeScope_Children, &cond) } {
            Ok(c) => c,
            Err(_) => return,
        };
        let count = unsafe { children.Length() }.unwrap_or(0);
        for i in 0..count {
            if let Ok(child) = unsafe { children.GetElement(i) } {
                let ct = unsafe { child.CurrentControlType() }.unwrap_or_default();
                let role = detect_element_role(&child, ct);

                let name = unsafe { child.CurrentName() }
                    .ok()
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());

                *next_id += 1;
                let id = *next_id;
                let mut child_node = sr_core::AomNode::new(id, role.clone());
                if role == Role::Text || role == Role::Generic {
                    child_node.text = name.clone();
                }
                child_node.name = name;

                populate_children(uia, &child, &mut child_node, next_id, depth - 1);
                parent_node.add_child(child_node);
            }
        }
    }

    populate_children(uia, element, &mut root, &mut next_id, 32);
    root
}

impl FocusChangedHandler {
    fn process_element(&self, element: &IUIAutomationElement) {
        let control_type = unsafe { element.CurrentControlType() }.unwrap_or_default();
        let mut role = map_control_type_to_role(control_type);

        let name = unsafe { element.CurrentName() }.ok().map(|s| s.to_string());
        let class_name = unsafe { element.CurrentClassName() }
            .map(|s| s.to_string())
            .unwrap_or_default();
        let help_text = unsafe { element.CurrentHelpText() }
            .ok()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        // Extract parent hierarchy and window handles
        let mut parent_name = String::new();
        let mut parent_class = String::new();
        let mut grandparent_class = String::new();
        let mut parent_hwnd = windows::Win32::Foundation::HWND::default();

        if let Ok(walker) = unsafe { self.uia.ControlViewWalker() } {
            if let Ok(parent) = unsafe { walker.GetParentElement(element) } {
                parent_name = unsafe { parent.CurrentName() }
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                parent_class = unsafe { parent.CurrentClassName() }
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if let Ok(h) = unsafe { parent.CurrentNativeWindowHandle() } {
                    parent_hwnd = h;
                }
                if let Ok(gp) = unsafe { walker.GetParentElement(&parent) } {
                    grandparent_class = unsafe { gp.CurrentClassName() }
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                }
            }
        }

        let mut top_hwnd = if !parent_hwnd.is_invalid() {
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::GetAncestor(
                    parent_hwnd,
                    windows::Win32::UI::WindowsAndMessaging::GA_ROOT,
                )
            }
        } else {
            windows::Win32::Foundation::HWND::default()
        };
        if top_hwnd.is_invalid() {
            top_hwnd = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
        }

        let mut top_class = String::new();
        let mut top_title = String::new();
        if !top_hwnd.is_invalid() {
            let mut title_buf = [0u16; 512];
            let len = unsafe {
                windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(top_hwnd, &mut title_buf)
            };
            if len > 0 {
                let slice = title_buf.get(..len as usize).unwrap_or(&[]);
                top_title = String::from_utf16_lossy(slice);
            }

            let mut class_buf = [0u16; 256];
            let clen = unsafe {
                windows::Win32::UI::WindowsAndMessaging::GetClassNameW(top_hwnd, &mut class_buf)
            };
            if clen > 0 {
                let slice = class_buf.get(..clen as usize).unwrap_or(&[]);
                top_class = String::from_utf16_lossy(slice);
            }
        }

        let mut container_name = None;
        let mut window_class = None;
        let mut window_title = None;
        let mut folder_name = None;
        let mut item_index = None;
        let mut item_count = None;

        let is_desktop = parent_name == "Desktop"
            || parent_name == "FolderView"
            || grandparent_class == "SHELLDLL_DefView"
            || grandparent_class == "Progman"
            || grandparent_class == "WorkerW"
            || top_class == "Progman"
            || top_class == "WorkerW"
            || (parent_class == "SysListView32"
                && !top_class.contains("CabinetWClass")
                && !top_title.contains("File Explorer"));

        let is_explorer = top_class == "CabinetWClass"
            || top_class == "ExploreWClass"
            || parent_name == "Items View"
            || parent_name == "Items view"
            || parent_class == "UIItemsView"
            || grandparent_class == "DUIListView"
            || grandparent_class == "CabinetWClass"
            || grandparent_class == "ShellTabWindowClass"
            || top_title.contains("File Explorer");

        let is_taskbar = parent_class.contains("Taskbar")
            || grandparent_class.contains("Taskbar")
            || parent_class == "Shell_TrayWnd"
            || grandparent_class == "Shell_TrayWnd"
            || parent_class == "Windows.UI.Input.InputSite.WindowClass"
            || parent_name == "Taskbar";

        if is_desktop {
            container_name = Some("FolderView".to_string());
            window_class = Some("SysListView32".to_string());
            role = Role::ListItem;

            if !parent_hwnd.is_invalid() {
                let total = unsafe {
                    windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                        parent_hwnd,
                        0x1004, // LVM_GETITEMCOUNT
                        Some(windows::Win32::Foundation::WPARAM(0)),
                        Some(windows::Win32::Foundation::LPARAM(0)),
                    )
                }
                .0;
                let focused = unsafe {
                    windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                        parent_hwnd,
                        0x100C, // LVM_GETNEXTITEM
                        Some(windows::Win32::Foundation::WPARAM(!0usize)),
                        Some(windows::Win32::Foundation::LPARAM(0x0001)), // LVNI_FOCUSED
                    )
                }
                .0;

                if focused >= 0 {
                    item_index = Some((focused + 1) as usize);
                }
                if total > 0 {
                    item_count = Some(total as usize);
                }
            }
        } else if is_explorer {
            container_name = Some("Items View".to_string());
            window_class = Some("CabinetWClass".to_string());
            role = Role::ListItem;

            if !top_title.is_empty() {
                window_title = Some(top_title.clone());
                let folder = top_title
                    .strip_suffix(" - File Explorer")
                    .or_else(|| top_title.strip_suffix(" - Dosya Gezgini"))
                    .unwrap_or(&top_title)
                    .trim();
                folder_name = Some(folder.to_string());
            }
        } else if is_taskbar {
            container_name = Some("Taskbar".to_string());
            window_class = Some("Shell_TrayWnd".to_string());
            if control_type == UIA_ButtonControlTypeId
                || class_name == "ToggleButton"
                || class_name == "Button"
            {
                role = Role::Button;
            }
        } else {
            let is_browser = top_class == "Chrome_WidgetWin_1"
                || top_class.contains("Chrome")
                || top_class.contains("Mozilla")
                || top_title.contains("Chrome")
                || top_title.contains("Edge")
                || top_title.contains("Firefox");

            let mut doc_element: Option<IUIAutomationElement> = None;

            if control_type == UIA_DocumentControlTypeId {
                doc_element = Some(element.clone());
            } else if let Ok(walker) = unsafe { self.uia.ControlViewWalker() } {
                let mut curr = element.clone();
                for _ in 0..6 {
                    if let Ok(p) = unsafe { walker.GetParentElement(&curr) } {
                        let p_ct = unsafe { p.CurrentControlType() }.unwrap_or_default();
                        let p_cls = unsafe { p.CurrentClassName() }
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        if p_ct == UIA_DocumentControlTypeId {
                            doc_element = Some(p);
                            break;
                        } else if p_cls == "Chrome_RenderWidgetHostHWND"
                            || p_cls == "MozillaContentWindowClass"
                        {
                            if let Ok(cond) = unsafe { self.uia.CreateTrueCondition() } {
                                if let Ok(children) =
                                    unsafe { p.FindAll(TreeScope_Children, &cond) }
                                {
                                    let clen = unsafe { children.Length() }.unwrap_or(0);
                                    for i in 0..clen {
                                        if let Ok(ch) = unsafe { children.GetElement(i) } {
                                            if unsafe { ch.CurrentControlType() }
                                                .unwrap_or_default()
                                                == UIA_DocumentControlTypeId
                                            {
                                                doc_element = Some(ch);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            break;
                        }
                        curr = p;
                    } else {
                        break;
                    }
                }
            }

            // If in a browser window and not address bar, check if children contain a document
            if doc_element.is_none()
                && is_browser
                && name.as_deref() != Some("Address and search bar")
            {
                if let Ok(cond) = unsafe { self.uia.CreateTrueCondition() } {
                    if let Ok(children) = unsafe { element.FindAll(TreeScope_Children, &cond) } {
                        let clen = unsafe { children.Length() }.unwrap_or(0);
                        for i in 0..clen {
                            if let Ok(ch) = unsafe { children.GetElement(i) } {
                                if unsafe { ch.CurrentControlType() }.unwrap_or_default()
                                    == UIA_DocumentControlTypeId
                                {
                                    doc_element = Some(ch);
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if let Some(ref doc) = doc_element {
                let doc_name = unsafe { doc.CurrentName() }
                    .ok()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let doc_title = if !doc_name.is_empty() {
                    doc_name
                } else {
                    top_title
                        .strip_suffix(" - Google Chrome")
                        .or_else(|| top_title.strip_suffix(" - Microsoft Edge"))
                        .or_else(|| top_title.strip_suffix(" - Mozilla Firefox"))
                        .unwrap_or(&top_title)
                        .trim()
                        .to_string()
                };

                if !doc_title.is_empty() {
                    if let Ok(mut last_doc) = self.last_loaded_document.lock() {
                        let should_load = match &*last_doc {
                            Some(prev) => prev != &doc_title,
                            None => true,
                        };
                        if should_load {
                            *last_doc = Some(doc_title.clone());
                            let aom = extract_uia_document_tree(&self.uia, doc, &doc_title);
                            let _ = self.sender.send(AccessibilityEvent::DocumentLoaded {
                                title: doc_title.clone(),
                                root: aom,
                            });
                        }
                    }
                    container_name = Some("Document".to_string());
                    if !top_title.is_empty() {
                        window_title = Some(top_title.clone());
                    }
                }
            } else {
                if let Ok(mut last_doc) = self.last_loaded_document.lock() {
                    *last_doc = None;
                }
                if !top_title.is_empty() {
                    window_title = Some(top_title);
                }
                if !top_class.is_empty() {
                    window_class = Some(top_class);
                } else if !class_name.is_empty() {
                    window_class = Some(class_name);
                }
                if !parent_name.is_empty() {
                    container_name = Some(parent_name);
                }
            }
        }

        // Query UIA PositionInSet and SizeOfSet for any list item
        if item_index.is_none() {
            if let Some(idx) = get_int_property(element, UIA_PositionInSetPropertyId) {
                if idx > 0 {
                    item_index = Some(idx as usize);
                }
            }
        }
        if item_count.is_none() {
            if let Some(cnt) = get_int_property(element, UIA_SizeOfSetPropertyId) {
                if cnt > 0 {
                    item_count = Some(cnt as usize);
                }
            }
        }

        let is_selected = get_bool_property(
            element,
            windows::Win32::UI::Accessibility::UIA_PROPERTY_ID(30079),
        );

        // Check live states
        let states = NodeStates {
            focused: true,
            selected: is_selected,
            ..Default::default()
        };

        let mut focus = FocusEvent::new(role).with_states(states);
        if let Some(n) = name {
            focus = focus.with_name(n);
        }
        if let Some(c) = container_name {
            focus = focus.with_container_name(c);
        }
        if let Some(w) = window_class {
            focus = focus.with_window_class(w);
        }
        if let Some(t) = window_title {
            focus = focus.with_window_title(t);
        }
        if let Some(f) = folder_name {
            focus = focus.with_folder_name(f);
        }
        if let Some(h) = help_text {
            focus = focus.with_help_text(h);
        }
        if let Some(i) = item_index {
            focus = focus.with_item_index(i);
        }
        if let Some(c) = item_count {
            focus = focus.with_item_count(c);
        }

        let _ = self.sender.send(AccessibilityEvent::Focus(focus));
    }
}

impl IUIAutomationFocusChangedEventHandler_Impl for FocusChangedHandler_Impl {
    fn HandleFocusChangedEvent(
        &self,
        sender: windows::core::Ref<'_, IUIAutomationElement>,
    ) -> windows::core::Result<()> {
        if let Some(element) = sender.as_ref() {
            self.this.process_element(element);
        }
        Ok(())
    }
}

struct WinEventDesktopState {
    sender: UnboundedSender<AccessibilityEvent>,
    uia: IUIAutomation,
    items_cache: Vec<String>,
    cached_hwnd: windows::Win32::Foundation::HWND,
}

unsafe impl Send for WinEventDesktopState {}
unsafe impl Sync for WinEventDesktopState {}

impl WinEventDesktopState {
    fn handle_desktop_event(&mut self, hwnd: windows::Win32::Foundation::HWND, child_index: usize) {
        let total_count = unsafe {
            windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                hwnd,
                0x1004, // LVM_GETITEMCOUNT
                Some(windows::Win32::Foundation::WPARAM(0)),
                Some(windows::Win32::Foundation::LPARAM(0)),
            )
        }
        .0 as usize;

        if total_count == 0 {
            return;
        }

        if self.cached_hwnd != hwnd
            || self.items_cache.len() != total_count
            || self.items_cache.is_empty()
        {
            if let Ok(element) = unsafe { self.uia.ElementFromHandle(hwnd) } {
                if let Ok(cond) = unsafe { self.uia.CreateTrueCondition() } {
                    if let Ok(children) = unsafe { element.FindAll(TreeScope_Children, &cond) } {
                        if let Ok(count) = unsafe { children.Length() } {
                            let mut new_cache = Vec::with_capacity(count as usize);
                            for i in 0..count {
                                if let Ok(item) = unsafe { children.GetElement(i) } {
                                    if let Ok(name) = unsafe { item.CurrentName() } {
                                        new_cache.push(name.to_string());
                                    }
                                }
                            }
                            if !new_cache.is_empty() {
                                self.items_cache = new_cache;
                                self.cached_hwnd = hwnd;
                            }
                        }
                    }
                }
            }
        }

        let item_name = self
            .items_cache
            .get(child_index.saturating_sub(1))
            .cloned()
            .unwrap_or_else(|| format!("Item {}", child_index));

        let states = NodeStates {
            focused: true,
            selected: Some(true),
            ..Default::default()
        };

        let focus = FocusEvent::new(Role::ListItem)
            .with_name(item_name)
            .with_container_name("FolderView".to_string())
            .with_window_class("SysListView32".to_string())
            .with_item_index(child_index)
            .with_item_count(total_count)
            .with_states(states);

        let _ = self.sender.send(AccessibilityEvent::Focus(focus));
    }
}

static WIN_EVENT_STATE: std::sync::Mutex<Option<WinEventDesktopState>> =
    std::sync::Mutex::new(None);

unsafe extern "system" fn winevent_desktop_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: windows::Win32::Foundation::HWND,
    id_object: i32,
    id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if id_object != OBJID_CLIENT || id_child <= 0 {
        return;
    }

    let mut class_buf = [0u16; 64];
    let len = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut class_buf);
    if len <= 0 {
        return;
    }
    let class_name = String::from_utf16_lossy(class_buf.get(..len as usize).unwrap_or(&[]));
    if class_name != "SysListView32" {
        return;
    }

    if let Ok(mut guard) = WIN_EVENT_STATE.try_lock() {
        if let Some(ref mut state) = *guard {
            state.handle_desktop_event(hwnd, id_child as usize);
        }
    }
}

pub struct WindowsAccessibilityEventListener {
    active: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl WindowsAccessibilityEventListener {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }
}

impl Default for WindowsAccessibilityEventListener {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessibilityEventListener for WindowsAccessibilityEventListener {
    fn start(&mut self, sender: UnboundedSender<AccessibilityEvent>) -> Result<(), PlatformError> {
        if self.active.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.active.store(true, Ordering::SeqCst);
        let active_clone = self.active.clone();

        let (init_tx, init_rx) = std::sync::mpsc::channel();

        let handle = thread::spawn(move || {
            // Attach thread to interactive user desktop
            unsafe {
                let default_name = windows::core::w!("default");
                let h = OpenDesktopW(default_name.as_ptr(), 0, 0, 0x01FF);
                if !h.is_null() {
                    let _ = SetThreadDesktop(h);
                }
            }

            let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };

            let uia: Result<IUIAutomation, windows::core::Error> =
                unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) };

            let uia = match uia {
                Ok(u) => {
                    let _ = init_tx.send(Ok(()));
                    u
                }
                Err(e) => {
                    let _ = init_tx.send(Err(PlatformError::EventListenerError(e.to_string())));
                    active_clone.store(false, Ordering::SeqCst);
                    unsafe { CoUninitialize() };
                    return;
                }
            };

            let raw_handler = FocusChangedHandler {
                uia: uia.clone(),
                sender: sender.clone(),
                last_loaded_document: std::sync::Mutex::new(None),
            };
            if let Ok(focused) = unsafe { uia.GetFocusedElement() } {
                raw_handler.process_element(&focused);
            }

            let handler: IUIAutomationFocusChangedEventHandler = raw_handler.into();

            let add_res = unsafe { uia.AddFocusChangedEventHandler(None, &handler) };
            if let Err(ref e) = add_res {
                tracing::warn!("Failed to register UIA focus changed event handler: {}", e);
            } else {
                tracing::info!("Registered UIA focus changed event handler successfully");
            }

            if let Ok(mut guard) = WIN_EVENT_STATE.lock() {
                *guard = Some(WinEventDesktopState {
                    sender,
                    uia: uia.clone(),
                    items_cache: Vec::new(),
                    cached_hwnd: windows::Win32::Foundation::HWND::default(),
                });
            }

            let win_event_hook = unsafe {
                SetWinEventHook(
                    EVENT_OBJECT_FOCUS,
                    EVENT_OBJECT_SELECTION,
                    None,
                    Some(winevent_desktop_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                )
            };

            // Message pump to dispatch COM STA / WinEvent / UIA cross-process messages
            unsafe {
                let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
                while active_clone.load(Ordering::SeqCst) {
                    while windows::Win32::UI::WindowsAndMessaging::PeekMessageW(
                        &mut msg,
                        None,
                        0,
                        0,
                        windows::Win32::UI::WindowsAndMessaging::PM_REMOVE,
                    )
                    .as_bool()
                    {
                        let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                        windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
                    }
                    thread::sleep(std::time::Duration::from_millis(15));
                }
            }

            unsafe {
                if !win_event_hook.is_invalid() {
                    let _ = UnhookWinEvent(win_event_hook);
                }
                let _ = uia.RemoveFocusChangedEventHandler(&handler);
                CoUninitialize();
            }
            if let Ok(mut guard) = WIN_EVENT_STATE.lock() {
                *guard = None;
            }
            tracing::info!("UIA and WinEvent focus handlers uninstalled");
        });

        match init_rx.recv() {
            Ok(Ok(())) => {
                self.thread_handle = Some(handle);
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(PlatformError::EventListenerError(
                "Failed to initialize UIA event thread".into(),
            )),
        }
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        if !self.active.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.active.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

impl Drop for WindowsAccessibilityEventListener {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn detect_element_role(
    child: &IUIAutomationElement,
    ct: windows::Win32::UI::Accessibility::UIA_CONTROLTYPE_ID,
) -> Role {
    // 1. Check UIA_HeadingLevelPropertyId (30173)
    let heading_lvl = get_int_property(
        child,
        windows::Win32::UI::Accessibility::UIA_PROPERTY_ID(30173),
    )
    .and_then(decode_uia_heading_level);

    if let Some(level) = heading_lvl {
        return Role::Heading { level };
    }

    // 2. Check UIA_AriaRolePropertyId (30101)
    let aria_role = get_string_property(
        child,
        windows::Win32::UI::Accessibility::UIA_PROPERTY_ID(30101),
    );
    if let Some(ref ar) = aria_role {
        let ar_lower = ar.to_lowercase();
        match ar_lower.as_str() {
            "heading" => {
                let aria_props = get_string_property(
                    child,
                    windows::Win32::UI::Accessibility::UIA_PROPERTY_ID(30102),
                );
                let level = aria_props
                    .as_deref()
                    .and_then(parse_aria_level)
                    .unwrap_or(2);
                return Role::Heading { level };
            }
            "link" => return Role::Link { href: None },
            "button" => return Role::Button,
            "checkbox" => return Role::CheckBox,
            "radio" => return Role::CheckBox,
            "textbox" | "searchbox" => return Role::Edit,
            "combobox" => return Role::ComboBox,
            "list" => return Role::List,
            "listitem" => return Role::ListItem,
            "table" | "grid" => return Role::Table,
            "row" => return Role::TableRow,
            "cell" | "gridcell" | "columnheader" | "rowheader" => return Role::TableCell,
            "main" => {
                return Role::Landmark {
                    landmark_type: "main".to_string(),
                }
            }
            "navigation" => {
                return Role::Landmark {
                    landmark_type: "navigation".to_string(),
                }
            }
            "banner" => {
                return Role::Landmark {
                    landmark_type: "banner".to_string(),
                }
            }
            "contentinfo" => {
                return Role::Landmark {
                    landmark_type: "contentinfo".to_string(),
                }
            }
            "complementary" => {
                return Role::Landmark {
                    landmark_type: "complementary".to_string(),
                }
            }
            "search" => {
                return Role::Landmark {
                    landmark_type: "search".to_string(),
                }
            }
            "region" => {
                return Role::Landmark {
                    landmark_type: "region".to_string(),
                }
            }
            "form" => {
                return Role::Landmark {
                    landmark_type: "form".to_string(),
                }
            }
            _ => {}
        }
    }

    // 3. Check LocalizedControlType (30004) for headings
    if let Ok(lct) = unsafe { child.CurrentLocalizedControlType() } {
        let lct_str = lct.to_string().to_lowercase();
        if lct_str.contains("heading")
            || lct_str.contains("başlık")
            || lct_str.contains("überschrift")
        {
            let level = lct_str
                .chars()
                .find(|c| c.is_ascii_digit())
                .and_then(|c| c.to_digit(10))
                .unwrap_or(2);
            return Role::Heading { level };
        }
    }

    // 4. Map standard ControlType
    map_control_type_to_role(ct)
}

pub(crate) fn decode_uia_heading_level(lvl: i32) -> Option<u32> {
    match lvl {
        1..=9 => Some(lvl as u32),
        80051..=80059 => Some((lvl - 80050) as u32),
        _ => None,
    }
}

pub(crate) fn parse_aria_level(props: &str) -> Option<u32> {
    for part in props.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("level=") {
            if let Ok(num) = val.trim().parse::<u32>() {
                if (1..=9).contains(&num) {
                    return Some(num);
                }
            }
        }
    }
    None
}

fn map_control_type_to_role(ct: windows::Win32::UI::Accessibility::UIA_CONTROLTYPE_ID) -> Role {
    if ct == UIA_ButtonControlTypeId {
        Role::Button
    } else if ct == UIA_EditControlTypeId {
        Role::Edit
    } else if ct == UIA_CheckBoxControlTypeId {
        Role::CheckBox
    } else if ct == UIA_HyperlinkControlTypeId {
        Role::Link { href: None }
    } else if ct == UIA_TabItemControlTypeId {
        Role::Tab
    } else if ct == UIA_TabControlTypeId {
        Role::TabList
    } else if ct == UIA_TableControlTypeId {
        Role::Table
    } else if ct == UIA_ListControlTypeId {
        Role::List
    } else if ct == UIA_ListItemControlTypeId {
        Role::ListItem
    } else if ct == UIA_DocumentControlTypeId {
        Role::Document
    } else if ct == UIA_TextControlTypeId {
        Role::Text
    } else if ct == UIA_HeaderControlTypeId || ct == UIA_HeaderItemControlTypeId {
        Role::Heading { level: 1 }
    } else if ct == UIA_ComboBoxControlTypeId {
        Role::ComboBox
    } else if ct == UIA_ProgressBarControlTypeId {
        Role::ProgressBar
    } else if ct == UIA_SliderControlTypeId {
        Role::Slider
    } else if ct == UIA_TreeControlTypeId {
        Role::Tree
    } else if ct == UIA_TreeItemControlTypeId {
        Role::TreeItem
    } else if ct == UIA_RadioButtonControlTypeId {
        Role::CheckBox
    } else {
        Role::Generic
    }
}

fn get_int_property(
    element: &IUIAutomationElement,
    prop_id: windows::Win32::UI::Accessibility::UIA_PROPERTY_ID,
) -> Option<i32> {
    unsafe {
        let val = element.GetCurrentPropertyValue(prop_id).ok()?;
        let vt = val.Anonymous.Anonymous.vt.0;
        // VT_I4 = 3, VT_INT = 22, VT_UI4 = 19
        if vt == 3 || vt == 22 || vt == 19 {
            let num = val.Anonymous.Anonymous.Anonymous.lVal;
            if num > 0 {
                return Some(num);
            }
        }
        None
    }
}

fn get_bool_property(
    element: &IUIAutomationElement,
    prop_id: windows::Win32::UI::Accessibility::UIA_PROPERTY_ID,
) -> Option<bool> {
    unsafe {
        let val = element.GetCurrentPropertyValue(prop_id).ok()?;
        let vt = val.Anonymous.Anonymous.vt.0;
        // VT_BOOL = 11
        if vt == 11 {
            let b = val.Anonymous.Anonymous.Anonymous.boolVal.0;
            return Some(b != 0);
        }
        None
    }
}

fn get_string_property(
    element: &IUIAutomationElement,
    prop_id: windows::Win32::UI::Accessibility::UIA_PROPERTY_ID,
) -> Option<String> {
    unsafe {
        let val = element.GetCurrentPropertyValue(prop_id).ok()?;
        let vt = val.Anonymous.Anonymous.vt.0;
        // VT_BSTR = 8
        if vt == 8 {
            let bstr = &val.Anonymous.Anonymous.Anonymous.bstrVal;
            let s = bstr.to_string();
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_uia_heading_levels() {
        assert_eq!(decode_uia_heading_level(1), Some(1));
        assert_eq!(decode_uia_heading_level(6), Some(6));
        assert_eq!(decode_uia_heading_level(9), Some(9));
        assert_eq!(decode_uia_heading_level(80051), Some(1));
        assert_eq!(decode_uia_heading_level(80052), Some(2));
        assert_eq!(decode_uia_heading_level(80053), Some(3));
        assert_eq!(decode_uia_heading_level(80054), Some(4));
        assert_eq!(decode_uia_heading_level(80055), Some(5));
        assert_eq!(decode_uia_heading_level(80056), Some(6));
        assert_eq!(decode_uia_heading_level(80050), None); // HeadingLevel_None
        assert_eq!(decode_uia_heading_level(0), None);
        assert_eq!(decode_uia_heading_level(-1), None);
    }

    #[test]
    fn test_parse_aria_level() {
        assert_eq!(parse_aria_level("level=1"), Some(1));
        assert_eq!(parse_aria_level("level=2;posinset=1"), Some(2));
        assert_eq!(parse_aria_level("setsize=5; level=3 ; other=x"), Some(3));
        assert_eq!(parse_aria_level("none"), None);
        assert_eq!(parse_aria_level("level=0"), None);
        assert_eq!(parse_aria_level("level=10"), None);
    }

    #[test]
    fn test_virtual_buffer_extracted_headings_and_navigation() {
        use sr_core::buffer::VirtualBuffer;
        use sr_core::cursor::VirtualCursor;

        // Construct an AOM tree simulating live Chrome extraction
        let mut root = sr_core::AomNode::new(0, Role::Document);
        root.name = Some("ALTAI — Kurumlar için yapay zekâ".to_string());

        let mut h1 = sr_core::AomNode::new(1, Role::Heading { level: 1 });
        h1.name = Some("ALTAI Home".to_string());
        h1.text = Some("ALTAI Home".to_string());

        let mut link = sr_core::AomNode::new(2, Role::Link { href: None });
        link.name = Some("Explore Yada".to_string());
        link.text = Some("Explore Yada".to_string());

        let mut h2 = sr_core::AomNode::new(3, Role::Heading { level: 2 });
        h2.name = Some("Products & services".to_string());
        h2.text = Some("Products & services".to_string());

        let mut h3 = sr_core::AomNode::new(4, Role::Heading { level: 3 });
        h3.name = Some("Data handling rules".to_string());
        h3.text = Some("Data handling rules".to_string());

        root.add_child(h1);
        root.add_child(link);
        root.add_child(h2);
        root.add_child(h3);

        let vbuf = VirtualBuffer::from_aom(&root);
        assert_eq!(vbuf.headings.len(), 3);
        assert_eq!(vbuf.links.len(), 1);

        let mut cursor = VirtualCursor::new(vbuf.clone());
        let first_h = cursor.move_next_heading();
        assert!(
            first_h.is_some(),
            "Should move to first heading: {:?}",
            first_h
        );
        let first_text = first_h.unwrap();
        assert!(
            first_text.contains("ALTAI Home") && first_text.contains("heading level 1"),
            "Unexpected heading announcement: {}",
            first_text
        );

        let next_h = cursor.move_next_heading();
        assert!(next_h.is_some());
        let next_text = next_h.unwrap();
        assert!(
            next_text.contains("Products & services") && next_text.contains("heading level 2"),
            "Unexpected heading announcement: {}",
            next_text
        );

        let h3_jump = cursor.move_next_heading_level(3);
        assert!(h3_jump.is_some());
        let h3_text = h3_jump.unwrap();
        assert!(
            h3_text.contains("Data handling rules") && h3_text.contains("heading level 3"),
            "Unexpected heading announcement: {}",
            h3_text
        );

        let prior_l = cursor.move_prior_link();
        assert!(prior_l.is_some());
        let l_text = prior_l.unwrap();
        assert!(l_text.contains("Explore Yada") && l_text.contains("link"));
    }
}
