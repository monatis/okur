//! An element as the vendored xa11y reads it, on any platform, as the engine's focus event: its
//! role, name, value and states. A platform's model adds what only its API tells (AT-SPI's role
//! name for a toggle button, in okur-linux-model). The roles xa11y has and Okur has not (a menu, a
//! scroll bar, a switch, a spin button, a tool tip, a status bar) take the nearest of Okur's,
//! provisionally: each platform's invariants are to settle how each is said.

use okur_core::aom::{NodeStates, Role};
use okur_core::event::FocusEvent;
use xa11y_core::{ElementData, Role as ElementRole, Toggled};

/// Okur's role for an xa11y role; a heading's level when it has one.
pub fn role(role: ElementRole, heading_level: Option<u32>) -> Role {
    match role {
        ElementRole::Window => Role::Window,
        ElementRole::Button => Role::Button,
        ElementRole::CheckBox | ElementRole::Switch => Role::CheckBox,
        ElementRole::RadioButton => Role::RadioButton,
        ElementRole::TextField | ElementRole::TextArea | ElementRole::SpinButton => Role::Edit,
        ElementRole::StaticText => Role::Text,
        ElementRole::ComboBox => Role::ComboBox,
        ElementRole::List => Role::List,
        ElementRole::ListItem => Role::ListItem,
        ElementRole::MenuItem => Role::MenuItem,
        ElementRole::MenuBar => Role::MenuBar,
        ElementRole::Tab => Role::Tab,
        ElementRole::TabGroup => Role::TabList,
        ElementRole::Table => Role::Table,
        ElementRole::TableRow => Role::TableRow,
        ElementRole::TableCell => Role::TableCell,
        ElementRole::Toolbar => Role::ToolBar,
        ElementRole::Slider => Role::Slider,
        ElementRole::Image => Role::Image,
        ElementRole::Link => Role::Link { href: None },
        ElementRole::Dialog => Role::Dialog,
        ElementRole::Alert => Role::AlertDialog,
        ElementRole::ProgressBar => Role::ProgressBar,
        ElementRole::TreeItem => Role::TreeItem,
        ElementRole::WebArea => Role::Document,
        ElementRole::Heading => Role::Heading {
            level: heading_level.unwrap_or(0),
        },
        ElementRole::Separator => Role::Separator,
        ElementRole::Navigation => Role::Landmark {
            landmark_type: "navigation".to_owned(),
        },
        ElementRole::Group | ElementRole::SplitGroup | ElementRole::Menu => Role::Group,
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

/// The focus event for the element that took the keyboard focus: its role, name, value and
/// states.
pub fn focus_event(element: &ElementData) -> FocusEvent {
    let mut event = FocusEvent::new(role(element.role, None));
    event.name = element.name.clone().filter(|n| !n.is_empty());
    // A container's value is its text with U+FFFC where each child sits (a page's document):
    // not a value to say (the Linux lab's first page read said them after the page's name).
    event.value = element
        .value
        .clone()
        .filter(|v| !v.is_empty() && !v.contains('\u{fffc}'));
    event.states = states(element);
    event
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_map_to_okurs() {
        assert_eq!(role(ElementRole::Button, None), Role::Button);
        assert_eq!(role(ElementRole::TextField, None), Role::Edit);
        assert_eq!(role(ElementRole::WebArea, None), Role::Document);
        assert_eq!(
            role(ElementRole::Heading, Some(2)),
            Role::Heading { level: 2 }
        );
        assert_eq!(role(ElementRole::Unknown, None), Role::Generic);
    }

    #[test]
    fn a_containers_text_is_not_its_value() {
        let mut page = ElementData::for_role(ElementRole::WebArea);
        page.value = Some("\u{fffc}\u{fffc}".to_owned());
        assert_eq!(focus_event(&page).value, None);
        let mut field = ElementData::for_role(ElementRole::TextField);
        field.value = Some("hello".to_owned());
        assert_eq!(focus_event(&field).value.as_deref(), Some("hello"));
    }
}
