//! An AT-SPI element, as the vendored xa11y reads it, as the engine's focus event: its role,
//! name, value and states. The roles AT-SPI has and Okur has not (a menu, a scroll bar, a
//! switch, a spin button, a tool tip, a status bar) take the nearest of Okur's, provisionally:
//! there is no JAWS on Linux to record, and the plan's invariants for GTK and Qt controls
//! (docs/plans/2026-11-05-linux-first.md §2.4) are to settle how each is said.

use okur_core::aom::{NodeStates, Role};
use okur_core::event::FocusEvent;
use xa11y_core::{ElementData, Role as AtRole, Toggled};

/// Okur's role for an AT-SPI role; a heading's level when it has one.
pub fn role(role: AtRole, heading_level: Option<u32>) -> Role {
    match role {
        AtRole::Window => Role::Window,
        AtRole::Button => Role::Button,
        AtRole::CheckBox | AtRole::Switch => Role::CheckBox,
        AtRole::RadioButton => Role::RadioButton,
        AtRole::TextField | AtRole::TextArea | AtRole::SpinButton => Role::Edit,
        AtRole::StaticText => Role::Text,
        AtRole::ComboBox => Role::ComboBox,
        AtRole::List => Role::List,
        AtRole::ListItem => Role::ListItem,
        AtRole::MenuItem => Role::MenuItem,
        AtRole::MenuBar => Role::MenuBar,
        AtRole::Tab => Role::Tab,
        AtRole::TabGroup => Role::TabList,
        AtRole::Table => Role::Table,
        AtRole::TableRow => Role::TableRow,
        AtRole::TableCell => Role::TableCell,
        AtRole::Toolbar => Role::ToolBar,
        AtRole::Slider => Role::Slider,
        AtRole::Image => Role::Image,
        AtRole::Link => Role::Link { href: None },
        AtRole::Dialog => Role::Dialog,
        AtRole::Alert => Role::AlertDialog,
        AtRole::ProgressBar => Role::ProgressBar,
        AtRole::TreeItem => Role::TreeItem,
        AtRole::WebArea => Role::Document,
        AtRole::Heading => Role::Heading {
            level: heading_level.unwrap_or(0),
        },
        AtRole::Separator => Role::Separator,
        AtRole::Navigation => Role::Landmark {
            landmark_type: "navigation".to_owned(),
        },
        AtRole::Group | AtRole::SplitGroup | AtRole::Menu => Role::Group,
        _ => Role::Generic,
    }
}

/// Okur's states for an element's.
pub fn states(element: &ElementData) -> NodeStates {
    let s = &element.states;
    let mut states = NodeStates::default();
    match s.checked {
        Some(Toggled::On) => states.checked = Some(true),
        Some(Toggled::Off) => states.checked = Some(false),
        Some(Toggled::Mixed) => {
            states.checked = Some(false);
            states.mixed = true;
        }
        _ => {}
    }
    states.selected = s.selected.then_some(true);
    states.expanded = s.expanded;
    states.focused = s.focused;
    states.disabled = !s.enabled;
    states.hidden = !s.visible;
    states.modal = s.modal;
    states.focusable = Some(s.focusable);
    states
}

/// The focus event for the element that took the keyboard focus.
pub fn focus_event(element: &ElementData) -> FocusEvent {
    // xa11y gives a toggle button (GTK's GtkToggleButton) the Switch role, a switch's too:
    // AT-SPI's own role name tells them apart. A toggle button is Okur's button whose
    // checked state is its pressed one, "Toggle button Pressed", as Windows' pages give it.
    let toggle = element.raw.get("atspi_role").and_then(|r| r.as_str()) == Some("toggle button");
    let mut event = FocusEvent::new(if toggle {
        Role::Button
    } else {
        role(element.role, None)
    });
    event.name = element.name.clone().filter(|n| !n.is_empty());
    // A container's value is its text with U+FFFC where each child sits (a page's document):
    // not a value to say (the lab's first page read said them after the page's name).
    event.value = element
        .value
        .clone()
        .filter(|v| !v.is_empty() && !v.contains('\u{fffc}'));
    event.states = states(element);
    if toggle {
        event.states.checked = Some(event.states.checked == Some(true));
    }
    event
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_map_to_okurs() {
        assert_eq!(role(AtRole::Button, None), Role::Button);
        assert_eq!(role(AtRole::TextField, None), Role::Edit);
        assert_eq!(role(AtRole::WebArea, None), Role::Document);
        assert_eq!(role(AtRole::Heading, Some(2)), Role::Heading { level: 2 });
        assert_eq!(role(AtRole::Unknown, None), Role::Generic);
    }

    #[test]
    fn a_toggle_button_is_a_button_pressed_or_not() {
        let mut toggle = ElementData::for_role(AtRole::Switch);
        toggle.raw.insert(
            "atspi_role".to_owned(),
            serde_json::Value::String("toggle button".to_owned()),
        );
        let event = focus_event(&toggle);
        assert_eq!(event.role, Role::Button);
        assert_eq!(event.states.checked, Some(false));
        let switch = ElementData::for_role(AtRole::Switch);
        assert_eq!(focus_event(&switch).role, Role::CheckBox);
    }

    #[test]
    fn a_containers_text_is_not_its_value() {
        let mut page = ElementData::for_role(AtRole::WebArea);
        page.value = Some("\u{fffc}\u{fffc}".to_owned());
        assert_eq!(focus_event(&page).value, None);
        let mut field = ElementData::for_role(AtRole::TextField);
        field.value = Some("hello".to_owned());
        assert_eq!(focus_event(&field).value.as_deref(), Some("hello"));
    }
}
