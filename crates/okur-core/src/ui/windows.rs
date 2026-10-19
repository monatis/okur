//! The Window List (Insert+F10): the running applications' windows, each with its state, to
//! switch to. Its controls are JAWS 2025's (help, Window List Dialog Box): the Running
//! Applications list, Only Include On Top Windows, and the buttons Switch To (the default) and
//! Cancel. JAWS reads it "Window List dialog | Running Applications List box | Command Prompt -
//! opendesk serve Minimized | 1 of 5" [obs, 511/s1].

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use crate::event::{TopWindow, WindowState};
use crate::messages::msg;

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Messages for the dialog.
    Send(Vec<ToUi>),
    /// Switch To (or Enter) on a window: the dialog closes and the window comes to the front.
    SwitchTo(u64),
    /// The dialog was cancelled.
    Closed,
}

/// The Window List while it is open.
#[derive(Debug, Clone)]
pub struct WindowListDialog {
    pub id: u32,
    windows: Vec<TopWindow>,
    on_top_only: bool,
    selected: usize,
}

impl WindowListDialog {
    pub fn new(id: u32, windows: Vec<TopWindow>) -> Self {
        Self {
            id,
            windows,
            on_top_only: false,
            selected: 0,
        }
    }

    /// The windows the list shows: the on-top ones alone when the check box is checked, the
    /// others when it is not.
    fn shown(&self) -> Vec<&TopWindow> {
        self.windows
            .iter()
            .filter(|w| w.on_top == self.on_top_only)
            .collect()
    }

    /// A window's line: its title, then its state.
    fn line(window: &TopWindow) -> String {
        let state = match window.state {
            WindowState::Minimized => msg::window_minimized(),
            WindowState::Maximized => msg::window_maximized(),
            WindowState::Restored => msg::window_restored(),
        };
        format!("{}  {state}", window.title)
    }

    fn items(&self) -> Vec<String> {
        self.shown().into_iter().map(Self::line).collect()
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
        };
        DialogSpec {
            id: self.id,
            title: msg::window_list_title().to_string(),
            controls: vec![
                control(
                    "windows",
                    msg::window_list_running().to_string(),
                    ControlKind::ListBox {
                        items: self.items(),
                        selected: self.selected,
                    },
                ),
                control(
                    "on_top",
                    msg::window_list_on_top().to_string(),
                    ControlKind::CheckBox {
                        checked: self.on_top_only,
                    },
                ),
                control(
                    "switch",
                    msg::window_list_switch().to_string(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "cancel",
                    msg::window_list_cancel().to_string(),
                    ControlKind::Button { default: false },
                ),
            ],
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("windows", Value::Selected(i)) => self.selected = *i,
                    ("on_top", Value::Checked(on)) => {
                        self.on_top_only = *on;
                        self.selected = 0;
                        return Outcome::Send(vec![ToUi::DialogItems {
                            id: self.id,
                            control: "windows".to_owned(),
                            items: self.items(),
                            selected: 0,
                        }]);
                    }
                    _ => {}
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id && control == "switch" => self
                .shown()
                .get(self.selected)
                .map_or(Outcome::Nothing, |window| Outcome::SwitchTo(window.id)),
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(id: u64, title: &str, state: WindowState, on_top: bool) -> TopWindow {
        TopWindow {
            id,
            title: title.to_owned(),
            state,
            on_top,
        }
    }

    fn dialog() -> WindowListDialog {
        WindowListDialog::new(
            7,
            vec![
                window(1, "Command Prompt", WindowState::Minimized, false),
                window(2, "Untitled - Notepad", WindowState::Restored, false),
                window(3, "Antivirus alert", WindowState::Restored, true),
            ],
        )
    }

    #[test]
    fn the_dialog_is_jaws_window_list() {
        let spec = dialog().spec();
        assert_eq!(spec.title, "Window List");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "&Running Applications",
                "&Only Include On Top Windows",
                "&Switch To",
                "Cancel"
            ]
        );
        assert!(matches!(
            spec.controls.first().map(|c| &c.kind),
            Some(ControlKind::ListBox { items, .. })
                if items == &["Command Prompt  Minimized", "Untitled - Notepad  Restored"]
        ));
    }

    #[test]
    fn switch_to_takes_the_selected_window_and_the_check_box_shows_the_on_top_ones() {
        let mut dialog = dialog();
        let _ = dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: "windows".to_owned(),
            value: Value::Selected(1),
        });
        let pressed = FromUi::DialogPressed {
            id: 7,
            control: "switch".to_owned(),
        };
        assert_eq!(dialog.handle(&pressed), Outcome::SwitchTo(2));
        let refilled = dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: "on_top".to_owned(),
            value: Value::Checked(true),
        });
        assert!(matches!(
            refilled,
            Outcome::Send(ref m) if matches!(m.first(), Some(ToUi::DialogItems { items, .. })
                if items == &["Antivirus alert  Restored"])
        ));
        assert_eq!(dialog.handle(&pressed), Outcome::SwitchTo(3));
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 7 }),
            Outcome::Closed
        );
    }
}
