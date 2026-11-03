#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    #[test]
    fn test_create_event_listener() {
        use okur_runtime::EventSource;
        let listener = super::windows::WindowsAccessibilityEventListener::new();
        assert!(!listener.is_active());
    }
}
