//! Windows Settings (Windows 11): its search box suggests settings as it is typed in, and Up
//! and Down move the selection through the suggestions, which the platform reports as
//! element selections ([`crate::event::AccessibilityEvent::Selected`]) while the focus stays
//! in the box. JAWS reads the selected suggestion, not the box's line (JAWS 2025 on the dev
//! box, 2026-10-16: "Display settings | 1 of 25"; Down in the empty box says nothing).

use crate::apps::AppModule;
use crate::event::FocusEvent;
use crate::input::{Key, KeyEvent};

/// Settings' process.
const PROCESS: &str = "SystemSettings.exe";
/// Its search box.
const SEARCH_BOX: &str = "CommandSearchTextBox";

/// Whether the focus is in Windows Settings.
fn owns(focus: &FocusEvent) -> bool {
    focus
        .app
        .as_deref()
        .is_some_and(|p| p.eq_ignore_ascii_case(PROCESS))
}

/// Windows Settings.
pub struct Settings;

impl AppModule for Settings {
    fn owns(&self, focus: &FocusEvent) -> bool {
        owns(focus)
    }

    fn reads_key(&self, field: &FocusEvent, key: &KeyEvent) -> bool {
        field.platform.automation_id.as_deref() == Some(SEARCH_BOX)
            && matches!(key.key, Key::UpArrow | Key::DownArrow)
            && key.modifiers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::Role;
    use crate::input::Modifiers;

    #[test]
    fn up_and_down_in_the_search_box_move_through_suggestions() {
        let mut search_box = FocusEvent::new(Role::Edit).with_name("Search box, Find a setting");
        search_box.app = Some("SystemSettings.exe".to_owned());
        search_box.platform.automation_id = Some(SEARCH_BOX.to_owned());
        let down = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
        let left = KeyEvent::new(Key::LeftArrow, Modifiers::none(), true);
        let settings = Settings;
        assert!(settings.owns(&search_box));
        assert!(settings.reads_key(&search_box, &down));
        assert!(!settings.reads_key(&search_box, &left));
        let mut other_field = search_box.clone();
        other_field.platform.automation_id = Some("PasswordBox".to_owned());
        assert!(!settings.reads_key(&other_field, &down));
    }
}
