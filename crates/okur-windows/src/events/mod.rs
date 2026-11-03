pub mod windows;

#[cfg(test)]
mod tests {
    #[test]
    fn test_create_event_listener() {
        use okur_runtime::EventSource;
        let listener = super::windows::WindowsAccessibilityEventListener::new();
        assert!(!listener.is_active());
    }
}
