//! The Windows taskbar: its buttons, with the taskbar named on entering it.

use crate::announcement::Announcement;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::engine::AppMode;
use crate::event::FocusEvent;

/// The Windows taskbar.
pub struct Taskbar;

impl AppModule for Taskbar {
    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        let on_taskbar = focus.platform.window_class.as_deref() == Some("Shell_TrayWnd")
            || focus.container_name.as_deref() == Some("Taskbar");
        if !on_taskbar {
            return FocusResponse::Pass;
        }
        let entering = *cx.app_mode != AppMode::NativeShell;
        *cx.app_mode = AppMode::NativeShell;
        let name = focus.name.as_deref().unwrap_or("Start Button");
        let container = entering.then(|| focus.container_name.as_deref().unwrap_or("Taskbar"));
        FocusResponse::Speak(Announcement::from_text(&button(
            container,
            name,
            focus.help_text.as_deref(),
        )))
    }
}

/// A taskbar button, after the taskbar's name when it is being entered.
fn button(container: Option<&str>, name: &str, help: Option<&str>) -> String {
    let mut lines = Vec::new();
    // The Start button is said without its role, as JAWS says it.
    let role = crate::messages::msg::role_button();
    match container {
        Some(c @ "Taskbar") if name == "Start Button" => {
            lines.push(format!("{c} \n   {name}   "));
        }
        Some(c) => lines.push(format!("  {}    {}    {role}   ", c, name)),
        None => lines.push(format!("{}    {role}   ", name)),
    }
    if let Some(h) = help {
        lines.push(h.to_string());
    }
    lines.join("\n")
}
