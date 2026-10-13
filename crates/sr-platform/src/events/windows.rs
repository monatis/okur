use crate::PlatformError;
use sr_core::event::{AccessibilityEvent, FocusEvent, NodeStates, Role};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tokio::sync::mpsc::UnboundedSender;
use windows::core::implement;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationFocusChangedEventHandler,
    IUIAutomationFocusChangedEventHandler_Impl, UIA_ButtonControlTypeId, UIA_CheckBoxControlTypeId,
    UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_HyperlinkControlTypeId,
    UIA_ListControlTypeId, UIA_ListItemControlTypeId, UIA_TabControlTypeId,
    UIA_TabItemControlTypeId, UIA_TableControlTypeId, UIA_TextControlTypeId,
};

use super::AccessibilityEventListener;

#[implement(IUIAutomationFocusChangedEventHandler)]
struct FocusChangedHandler {
    sender: UnboundedSender<AccessibilityEvent>,
}

impl IUIAutomationFocusChangedEventHandler_Impl for FocusChangedHandler_Impl {
    fn HandleFocusChangedEvent(
        &self,
        sender: windows::core::Ref<'_, IUIAutomationElement>,
    ) -> windows::core::Result<()> {
        if let Some(element) = sender.as_ref() {
            let role = match unsafe { element.CurrentControlType() } {
                Ok(ct) => map_control_type_to_role(ct),
                Err(_) => Role::Generic,
            };

            let name = unsafe { element.CurrentName() }.ok().map(|s| s.to_string());
            let value: Option<String> = None;

            // Check live states
            let states = NodeStates {
                focused: true,
                ..Default::default()
            };

            let mut focus = FocusEvent::new(role).with_states(states);
            if let Some(n) = name {
                focus = focus.with_name(n);
            }
            if let Some(v) = value {
                focus = focus.with_value(v);
            }

            let _ = self.sender.send(AccessibilityEvent::Focus(focus));
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
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

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

            let handler: IUIAutomationFocusChangedEventHandler =
                FocusChangedHandler { sender }.into();

            let add_res = unsafe { uia.AddFocusChangedEventHandler(None, &handler) };
            if let Err(e) = add_res {
                tracing::warn!("Failed to register UIA focus changed event handler: {}", e);
            } else {
                tracing::info!("Registered UIA focus changed event handler successfully");
            }

            // Keep the event listener active while running
            while active_clone.load(Ordering::SeqCst) {
                thread::sleep(std::time::Duration::from_millis(50));
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
