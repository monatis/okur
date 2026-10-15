//! Stub backend for non-Windows platforms (allows compilation on all targets).

use xa11y_core::input::{InputProvider, Key, MouseButton, Point, ScrollDelta};
use xa11y_core::{ElementData, Error, Provider, Result, ShellSurfaceKind, Subscription};

/// What every operation reports: the stub cannot be constructed through `new()`, but it can
/// through `Default`, and calling it must fail rather than panic.
fn unavailable() -> Error {
    Error::Unsupported {
        feature: "the Windows accessibility backend on this platform".to_string(),
    }
}

#[derive(Default)]
pub struct WindowsProvider;

impl WindowsProvider {
    pub fn new() -> Result<Self> {
        Err(Error::Platform {
            code: -1,
            message: "Windows backend only available on Windows".to_string(),
        })
    }
}

#[derive(Default)]
pub struct WindowsInputProvider;

impl WindowsInputProvider {
    pub fn new() -> Result<Self> {
        Err(Error::Platform {
            code: -1,
            message: "Windows input backend only available on Windows".to_string(),
        })
    }
}

impl InputProvider for WindowsInputProvider {
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

impl Provider for WindowsProvider {
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
