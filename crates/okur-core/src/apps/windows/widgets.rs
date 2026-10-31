//! Windows 11 Widgets: its tiles update in the background and report focus while the user
//! is elsewhere, from the board and from the taskbar's Widgets button. Those events are not
//! read. Only they are: other applications' controls called "Widgets" (Windows Settings'
//! Taskbar page has a "Widgets" switch) are read as any other.

use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::event::FocusEvent;

/// The processes of the Widgets board.
const BOARD_PROCESSES: [&str; 2] = ["WidgetBoard.exe", "Widgets.exe"];
/// The taskbar's top-level window, where the Widgets button is.
const TASKBAR_CLASS: &str = "Shell_TrayWnd";

/// Windows 11 Widgets.
pub struct Widgets;

/// Whether the focus is in the Widgets board or on the taskbar.
fn in_widgets(focus: &FocusEvent) -> bool {
    let board = focus
        .app
        .as_deref()
        .is_some_and(|app| BOARD_PROCESSES.iter().any(|p| app.eq_ignore_ascii_case(p)));
    board || focus.platform.top_window_class.as_deref() == Some(TASKBAR_CLASS)
}

impl AppModule for Widgets {
    fn focus(&mut self, focus: &FocusEvent, _cx: &mut AppContext<'_>) -> FocusResponse {
        if in_widgets(focus)
            && focus
                .name
                .as_deref()
                .is_some_and(|n| n.starts_with("Widgets"))
        {
            FocusResponse::Silent
        } else {
            FocusResponse::Pass
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::Role;

    fn widgets(focus: &FocusEvent) -> bool {
        matches!(
            Widgets.focus(
                focus,
                &mut AppContext {
                    app_mode: &mut crate::engine::AppMode::NativeControl,
                    window_title: &mut None,
                    window_kind: &mut None,
                    selecting: false,
                    alt_held: false,
                    letter_key: false,
                    escape_key: false,
                    key: None,
                    ask_view: false,
                    ask_sheet: false,
                }
            ),
            FocusResponse::Silent
        )
    }

    #[test]
    fn the_taskbar_widgets_button_is_not_read() {
        let mut button = FocusEvent::new(Role::Button).with_name("Widgets 15°C Cloudy");
        button.app = Some("explorer.exe".to_owned());
        button.platform.top_window_class = Some(TASKBAR_CLASS.to_owned());
        assert!(widgets(&button));
        let mut tile = FocusEvent::new(Role::Group).with_name("Widgets board");
        tile.app = Some("WidgetBoard.exe".to_owned());
        assert!(widgets(&tile));
    }

    #[test]
    fn a_settings_switch_named_widgets_is_read() {
        // Windows Settings > Personalization > Taskbar (dev box, 2026-10-17): JAWS says
        // "Widgets | Widgets Button | On | To activate press spacebar."
        let mut switch = FocusEvent::new(Role::Button).with_name("Widgets");
        switch.app = Some("SystemSettings.exe".to_owned());
        switch.platform.top_window_class = Some("ApplicationFrameWindow".to_owned());
        assert!(!widgets(&switch));
    }
}
