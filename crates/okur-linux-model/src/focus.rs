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
    let mut event = FocusEvent::new(role(element.role, None));
    event.name = element.name.clone().filter(|n| !n.is_empty());
    event.value = element.value.clone().filter(|v| !v.is_empty());
    event.states = states(element);
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
}
