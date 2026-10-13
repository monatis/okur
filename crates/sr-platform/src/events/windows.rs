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
    IUIAutomationFocusChangedEventHandler_Impl, UIA_ButtonControlTypeId, UIA_CheckBoxControlTypeId,
    UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_HyperlinkControlTypeId,
    UIA_ListControlTypeId, UIA_ListItemControlTypeId, UIA_TabControlTypeId,
    UIA_TabItemControlTypeId, UIA_TableControlTypeId, UIA_TextControlTypeId,
};

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

        let mut container_name = None;
        let mut window_class = None;
        let mut item_index = None;
        let mut item_count = None;

        let is_desktop = parent_class == "SysListView32"
            || parent_name == "Desktop"
            || grandparent_class == "Progman"
            || grandparent_class == "WorkerW";

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
            if !parent_name.is_empty() {
                container_name = Some(parent_name);
            }
            if !parent_class.is_empty() {
                window_class = Some(parent_class);
            } else if !class_name.is_empty() {
                window_class = Some(class_name);
            }
        }

        // Check live states
        let states = NodeStates {
            focused: true,
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
                sender,
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
                let _ = uia.RemoveFocusChangedEventHandler(&handler);
                CoUninitialize();
            }
            tracing::info!("UIA focus changed event handler uninstalled");
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
    } else {
        Role::Generic
    }
}
