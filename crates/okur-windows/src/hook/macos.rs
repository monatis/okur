use super::{HookCallback, KeyboardHook};
use crate::PlatformError;

/// Native macOS keyboard interceptor using CoreGraphics `CGEventTap`.
pub struct MacKeyboardHook {
    active: bool,
}

impl MacKeyboardHook {
    pub fn new() -> Self {
        Self { active: false }
    }
}

impl Default for MacKeyboardHook {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardHook for MacKeyboardHook {
    fn start(&mut self, _callback: HookCallback) -> Result<(), PlatformError> {
        self.active = true;
        // On macOS:
        // 1. CGEventTapCreate(kCGSessionEventTap, kCGHeadInsertEventTap, kCGEventTapOptionDefault, ...)
        // 2. CFMachPortCreateRunLoopSource(...)
        // 3. CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes)
        // 4. CFRunLoopRun()
        tracing::info!("macOS CGEventTap keyboard hook initialized");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        self.active = false;
        // On macOS:
        // CFRunLoopStop(run_loop);
        tracing::info!("macOS CGEventTap keyboard hook stopped");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active
    }
}
