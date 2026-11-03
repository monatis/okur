pub mod windows;

#[cfg(test)]
mod tests {
    #[test]
    fn test_create_keyboard_hook() {
        use okur_runtime::KeyboardHook;
        let hook = super::windows::WindowsKeyboardHook::new();
        assert!(!hook.is_active());
    }
}
