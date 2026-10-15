//! Windows search (the Start menu's search box, Windows 11): Up and Down move the selection
//! through the results, which the platform reports as element selections
//! ([`crate::event::AccessibilityEvent::Selected`]). JAWS reads the selected result, not the
//! search box's line (JAWS 2025 on the dev box: Down gave "notepad, See more search results,
//! Press right to switch preview | 2 of 5").

use crate::event::FocusEvent;
use crate::input::{Key, KeyEvent};

/// The search box's process.
const PROCESS: &str = "SearchHost.exe";

/// Whether the focus is in Windows search.
pub fn owns(focus: &FocusEvent) -> bool {
    focus
        .process
        .as_deref()
        .is_some_and(|p| p.eq_ignore_ascii_case(PROCESS))
}

/// Whether `key` in the focused `field` moves through the results instead of the text.
pub fn arrows_select(field: &FocusEvent, key: &KeyEvent) -> bool {
    owns(field) && matches!(key.key, Key::UpArrow | Key::DownArrow) && key.modifiers.is_empty()
}

/// Whether arriving in the search names its window first: JAWS starts at the search box
/// ("Search box Edit | Type in text.", dev box), without the window's title ("Search").
pub fn names_window(focus: &FocusEvent) -> bool {
    !owns(focus)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::Role;
    use crate::input::Modifiers;

    #[test]
    fn up_and_down_in_the_search_box_select_results() {
        let mut search_box = FocusEvent::new(Role::Edit).with_name("Search box");
        search_box.process = Some("SearchHost.exe".to_owned());
        let down = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
        let left = KeyEvent::new(Key::LeftArrow, Modifiers::none(), true);
        assert!(arrows_select(&search_box, &down));
        assert!(!arrows_select(&search_box, &left));
        let notepad = FocusEvent::new(Role::Edit);
        assert!(!arrows_select(&notepad, &down));
    }
}
