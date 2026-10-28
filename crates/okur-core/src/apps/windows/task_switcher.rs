//! Windows' task switcher (Alt+Tab with Alt held), as JAWS reads it: each window's item by
//! name as Tab selects it, the hint after the first, and nothing for the switcher itself or
//! its list.
//!
//! - JAWS 2025 on Windows 11, observed on the dev box (2026-10-15; research, scenario 317
//!   records it on old-pc): Tab 1 "Claude | To move to an item press the Arrow keys.", Tab 2
//!   "JAWS Professional", Tab 3 "Command Prompt".
//! - Windows 10, Alt held through three Tabs [obs, 317/s2-s4]: "SR Dialog Fixture | To move
//!   to an item press the Arrow keys.", "Windows PowerShell", then the Gazette page.

use crate::announcement::Announcement;
use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::event::FocusEvent;

/// The switcher's windows, by class: Windows 11's XAML host, Windows 10's frame.
const SWITCHER_WINDOWS: [&str; 2] = ["XamlExplorerHostIslandWindow", "MultitaskingViewFrame"];
/// The invisible window Windows 10 keeps in front while its switcher is up.
const STAGING_WINDOW: &str = "ForegroundStaging";

/// The window that raised the focus is the switcher's. The window in front, read after the
/// event, can already be the one chosen: a quick Alt+Tab closes the switcher within 20 ms of
/// its focus [obs, 316/s3].
fn raised_by_switcher(focus: &FocusEvent) -> bool {
    focus
        .platform
        .window_class
        .as_deref()
        .is_some_and(|w| SWITCHER_WINDOWS.contains(&w))
}

/// Reading state for the task switcher.
#[derive(Debug, Default)]
pub struct TaskSwitcher {
    /// An item was spoken since the switcher came up.
    entered: bool,
    /// The item last spoken: Windows 10 reports each selection two or three times (a WinEvent
    /// focus, then UIA's, with its staging window's between), and JAWS says it once [obs,
    /// 317/s2-s4].
    last_item: Option<String>,
}

impl TaskSwitcher {
    /// What a focus in the switcher says (empty for the switcher itself), or `None` when the
    /// focus is not the switcher's. Its items are read while Alt is held (`alt_held`): a quick
    /// Alt+Tab, released before the switcher's focus is read, says only the window it reaches
    /// (JAWS in 3 of 3 trials, and 2 of 3 in an earlier recording [obs, 316/s3]).
    pub fn speech_for(&mut self, focus: &FocusEvent, alt_held: bool) -> Option<String> {
        let window = focus
            .platform
            .top_window_class
            .as_deref()
            .or(focus.platform.window_class.as_deref());
        let item = focus.role == Role::ListItem;
        let ours = window.is_some_and(|w| SWITCHER_WINDOWS.contains(&w))
            || raised_by_switcher(focus)
            || (item && window == Some(STAGING_WINDOW));
        if !ours {
            self.entered = false;
            self.last_item = None;
            return None;
        }
        if !item || !alt_held {
            return Some(String::new());
        }
        let name = focus.name.clone().unwrap_or_default();
        if self.last_item.replace(name.clone()).as_ref() == Some(&name) {
            return Some(String::new());
        }
        Some(if std::mem::replace(&mut self.entered, true) {
            name
        } else {
            format!(
                "{name}\n{}",
                crate::messages::msg::hint_move_to_an_item_arrows()
            )
        })
    }
}

impl AppModule for TaskSwitcher {
    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        let Some(speech) = self.speech_for(focus, cx.alt_held) else {
            return FocusResponse::Pass;
        };
        // Its window is remembered, so going back to an application announces that
        // application's window, even when the window in front was read as that application's
        // already.
        cx.note_window(focus);
        if raised_by_switcher(focus) {
            *cx.window_kind = focus.platform.window_class.clone();
        }
        if speech.is_empty() {
            FocusResponse::Silent
        } else {
            FocusResponse::Speak(Announcement::from_text(&speech))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::AppMode;

    fn in_switcher(role: Role, name: &str) -> FocusEvent {
        let mut focus = FocusEvent::new(role).with_name(name);
        focus.platform.top_window_class = Some("XamlExplorerHostIslandWindow".to_owned());
        focus
    }

    #[test]
    fn items_by_name_with_the_hint_first() {
        let mut switcher = TaskSwitcher::default();
        assert_eq!(
            switcher
                .speech_for(&in_switcher(Role::Window, "Task Switching"), true)
                .as_deref(),
            Some("")
        );
        assert_eq!(
            switcher
                .speech_for(&in_switcher(Role::ListItem, "Claude"), true)
                .as_deref(),
            Some("Claude\nTo move to an item press the Arrow keys.")
        );
        // The switcher's own input pane takes focus between items.
        assert_eq!(
            switcher
                .speech_for(&in_switcher(Role::Generic, ""), true)
                .as_deref(),
            Some("")
        );
        assert_eq!(
            switcher
                .speech_for(&in_switcher(Role::ListItem, "JAWS Professional"), true)
                .as_deref(),
            Some("JAWS Professional")
        );
        // Out of the switcher: not ours, and the next visit starts over.
        let notepad = FocusEvent::new(Role::Document)
            .with_name("Text editor")
            .with_window_class("RichEditD2DPT");
        assert_eq!(switcher.speech_for(&notepad, true), None);
        assert!(switcher
            .speech_for(&in_switcher(Role::ListItem, "Claude"), true)
            .is_some_and(|s| s.ends_with("Arrow keys.")));
    }

    /// Windows 10 reports a selection more than once, its staging window between: said once.
    #[test]
    fn an_item_reported_again_is_said_once() {
        let mut switcher = TaskSwitcher::default();
        let item = in_switcher(Role::ListItem, "SR Dialog Fixture");
        assert!(switcher
            .speech_for(&item, true)
            .is_some_and(|s| s.starts_with("SR Dialog")));
        assert_eq!(
            switcher
                .speech_for(&in_switcher(Role::Window, ""), true)
                .as_deref(),
            Some("")
        );
        assert_eq!(switcher.speech_for(&item, true).as_deref(), Some(""));
        assert_eq!(
            switcher
                .speech_for(&in_switcher(Role::ListItem, "Windows PowerShell"), true)
                .as_deref(),
            Some("Windows PowerShell")
        );
    }

    #[test]
    fn windows_10_items_sit_behind_the_staging_window() {
        let mut switcher = TaskSwitcher::default();
        let mut item = FocusEvent::new(Role::ListItem).with_name("SR Second Window");
        item.platform.top_window_class = Some(STAGING_WINDOW.to_owned());
        assert_eq!(
            switcher.speech_for(&item, true).as_deref(),
            Some("SR Second Window\nTo move to an item press the Arrow keys.")
        );
        // The staging window's other focus events are an application's, not the switcher's.
        let mut edit = FocusEvent::new(Role::Edit).with_name("Notes");
        edit.platform.top_window_class = Some(STAGING_WINDOW.to_owned());
        assert_eq!(switcher.speech_for(&edit, true), None);
    }

    /// A quick Alt+Tab: the switcher's focus is read after Alt is up and the window in front is
    /// already the application chosen. The focus is still the switcher's, said as nothing, and
    /// its window is remembered as the switcher, so the application's focus that follows names
    /// its window [obs, 316/s3].
    #[test]
    fn a_quick_alt_tab_is_the_switchers_focus() {
        let mut switcher = TaskSwitcher::default();
        let mut item = FocusEvent::new(Role::ListItem)
            .with_name("SR Editor Fixture")
            .with_window_class("MultitaskingViewFrame")
            .with_window_title("SR Editor Fixture");
        item.platform.top_window_class = Some("WindowsForms10.Window.8.app".to_owned());
        let mut mode = AppMode::NativeControl;
        let mut title = None;
        let mut kind = None;
        let mut cx = AppContext {
            app_mode: &mut mode,
            window_title: &mut title,
            window_kind: &mut kind,
            selecting: false,
            alt_held: false,
            letter_key: false,
            escape_key: false,
            ask_view: false,
        };
        assert!(matches!(
            switcher.focus(&item, &mut cx),
            FocusResponse::Silent
        ));
        assert_eq!(kind.as_deref(), Some("MultitaskingViewFrame"));
    }
}
