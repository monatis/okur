//! Windows 11 Widgets: its tiles update in the background and report focus while the user
//! is elsewhere. Those events are not read.

use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::event::FocusEvent;

/// Windows 11 Widgets.
pub struct Widgets;

impl AppModule for Widgets {
    fn focus(&mut self, focus: &FocusEvent, _cx: &mut AppContext<'_>) -> FocusResponse {
        if focus
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
