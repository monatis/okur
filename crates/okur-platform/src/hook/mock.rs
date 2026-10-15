use super::{HookCallback, KeyAction, KeyEvent, KeyboardHook};
use crate::PlatformError;
use std::sync::{Arc, Mutex};

type SharedCallback = Arc<Mutex<HookCallback>>;

/// Mock keyboard hook for unit tests, headless testing, and non-supported OS platforms.
#[derive(Default)]
pub struct MockKeyboardHook {
    active: bool,
    callback: Option<SharedCallback>,
}

impl MockKeyboardHook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Injects a synthetic key event directly into the hook callback for verification.
    pub fn inject(&mut self, event: KeyEvent) -> Option<KeyAction> {
        if let Some(ref cb) = self.callback {
            if let Ok(mut handler) = cb.lock() {
                return Some(handler(event));
            }
        }
        None
    }
}

impl KeyboardHook for MockKeyboardHook {
    fn start(&mut self, callback: HookCallback) -> Result<(), PlatformError> {
        self.active = true;
        self.callback = Some(Arc::new(Mutex::new(callback)));
        Ok(())
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        self.active = false;
        self.callback = None;
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::{Key, Modifiers};

    #[test]
    fn test_mock_hook_lifecycle_and_injection() {
        let mut hook = MockKeyboardHook::new();
        assert!(!hook.is_active());

        let received_clone = Arc::new(Mutex::new(Vec::new()));
        let r_clone = Arc::clone(&received_clone);

        let start_res = hook.start(Box::new(move |event| {
            if let Ok(mut list) = r_clone.lock() {
                list.push(event.clone());
            }
            KeyAction::Consume
        }));
        assert!(start_res.is_ok(), "Hook start should succeed");

        assert!(hook.is_active());

        let event = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
        let action = hook.inject(event.clone());
        assert_eq!(action, Some(KeyAction::Consume));

        let events = received_clone.lock().map(|g| g.clone()).unwrap_or_default();
        assert_eq!(events.len(), 1);
        assert_eq!(events.first().map(|e| e.key), Some(Key::DownArrow));

        let stop_res = hook.stop();
        assert!(stop_res.is_ok(), "Hook stop should succeed");
        assert!(!hook.is_active());
    }
}
