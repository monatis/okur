// Vendored from xa11y 0.15.2: exempt from the workspace bounds-safety gate until hardened
// (tracked in crates/xa11y/VENDORED.md). New code here must still avoid unchecked indexing.
#![allow(clippy::indexing_slicing)]
//! macOS accessibility backend using AXUIElement API.

#[cfg(target_os = "macos")]
mod ax;
#[cfg(target_os = "macos")]
mod input;
#[cfg(target_os = "macos")]
mod screenshot;

#[cfg(target_os = "macos")]
pub use ax::accessibility_grant_instructions;
#[cfg(target_os = "macos")]
pub use ax::MacOSProvider;
#[cfg(target_os = "macos")]
pub use input::MacOSInputProvider;
#[cfg(target_os = "macos")]
pub use screenshot::MacOSScreenshot;

#[cfg(not(target_os = "macos"))]
mod stub {
    use xa11y_core::input::{InputProvider, Key, MouseButton, Point, ScrollDelta};
    use xa11y_core::*;

    /// What every operation reports: the stub cannot be constructed through `new()`, but it
    /// can through `Default`, and calling it must fail rather than panic.
    fn unavailable() -> Error {
        Error::Unsupported {
            feature: "the macOS accessibility backend on this platform".to_string(),
        }
    }

    pub struct MacOSProvider;

    impl MacOSProvider {
        pub fn new() -> Result<Self> {
            Err(Error::Platform {
                code: -1,
                message: "macOS backend only available on macOS".to_string(),
            })
        }
    }

    #[derive(Default)]
    pub struct MacOSInputProvider;

    impl MacOSInputProvider {
        pub fn new() -> Result<Self> {
            Err(Error::Platform {
                code: -1,
                message: "macOS input backend only available on macOS".to_string(),
            })
        }
    }

    pub struct MacOSScreenshot;

    impl MacOSScreenshot {
        pub fn new() -> Result<Self> {
            Err(Error::Platform {
                code: -1,
                message: "macOS screenshot backend only available on macOS".to_string(),
            })
        }
    }

    impl ScreenshotProvider for MacOSScreenshot {
        fn capture_full(&self) -> Result<(Screenshot, Point)> {
            Err(unavailable())
        }
        fn capture_region(&self, _: Rect) -> Result<Screenshot> {
            Err(unavailable())
        }
    }

    impl InputProvider for MacOSInputProvider {
        fn pointer_move(&self, _: Point) -> Result<()> {
            Err(unavailable())
        }
        fn pointer_down(&self, _: MouseButton) -> Result<()> {
            Err(unavailable())
        }
        fn pointer_up(&self, _: MouseButton) -> Result<()> {
            Err(unavailable())
        }
        fn pointer_click(&self, _: Point, _: MouseButton, _: u32) -> Result<()> {
            Err(unavailable())
        }
        fn pointer_scroll(&self, _: Point, _: ScrollDelta) -> Result<()> {
            Err(unavailable())
        }
        fn key_down(&self, _: &Key) -> Result<()> {
            Err(unavailable())
        }
        fn key_up(&self, _: &Key) -> Result<()> {
            Err(unavailable())
        }
        fn type_text(&self, _: &str) -> Result<()> {
            Err(unavailable())
        }
    }

    impl Provider for MacOSProvider {
        fn get_children(&self, _: Option<&ElementData>) -> Result<Vec<ElementData>> {
            Err(unavailable())
        }
        fn get_parent(&self, _: &ElementData) -> Result<Option<ElementData>> {
            Err(unavailable())
        }
        fn list_apps(&self) -> Result<Vec<ElementData>> {
            Err(unavailable())
        }
        fn focused_app(&self) -> Result<ElementData> {
            Err(unavailable())
        }
        fn list_shell_surfaces(&self) -> Result<Vec<(ShellSurfaceKind, ElementData)>> {
            Err(unavailable())
        }
        fn press(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn focus(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn blur(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn toggle(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn select(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn expand(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn collapse(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn show_menu(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn increment(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn decrement(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn scroll_into_view(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn set_value(&self, _: &ElementData, _: &str) -> Result<()> {
            Err(unavailable())
        }
        fn set_numeric_value(&self, _: &ElementData, _: f64) -> Result<()> {
            Err(unavailable())
        }
        fn type_text(&self, _: &ElementData, _: &str) -> Result<()> {
            Err(unavailable())
        }
        fn set_text_selection(&self, _: &ElementData, _: u32, _: u32) -> Result<()> {
            Err(unavailable())
        }
        fn perform_action(&self, _: &ElementData, _: &str) -> Result<()> {
            Err(unavailable())
        }
        fn activate(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn minimize(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn maximize(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn enter_fullscreen(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn restore(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn close(&self, _: &ElementData) -> Result<()> {
            Err(unavailable())
        }
        fn move_to(&self, _: &ElementData, _: i32, _: i32) -> Result<()> {
            Err(unavailable())
        }
        fn resize_to(&self, _: &ElementData, _: u32, _: u32) -> Result<()> {
            Err(unavailable())
        }
        fn subscribe(&self, _: &ElementData) -> Result<Subscription> {
            Err(unavailable())
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub use stub::{MacOSInputProvider, MacOSProvider, MacOSScreenshot};

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use xa11y_core::*;

    #[test]
    fn create_provider() {
        let result = MacOSProvider::new();
        #[cfg(target_os = "macos")]
        assert!(result.is_ok());
        #[cfg(not(target_os = "macos"))]
        assert!(result.is_err());
    }
}
