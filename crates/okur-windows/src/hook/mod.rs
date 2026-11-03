#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    #[test]
    fn test_create_keyboard_hook() {
        use okur_runtime::KeyboardHook;
        let hook = super::windows::WindowsKeyboardHook::new();
        assert!(!hook.is_active());
    }
}
