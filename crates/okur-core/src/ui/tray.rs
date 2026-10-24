//! The system tray list (Insert+F11): the icons of the taskbar's notification area, to click
//! one. Its controls are JAWS 2025's (help, Select a System Tray Icon Dialog Box): the icons'
//! list, then Right Single Click (what Enter in the list does), Left Single Click, Left Double
//! Click and Cancel. JAWS's list has no label and is said by its dialog's name: "Select a
//! System Tray Icon dialog | Select a System Tray Icon List box | Notification Chevron | 1 of 6"
//! [obs, 511/s4].

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value};

use crate::actions::MouseClick;
use crate::event::TrayIcon;
use crate::messages::msg;

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// A click chosen on an icon: the dialog closes and the click goes to the icon.
    Click(TrayIcon, MouseClick),
    /// The dialog was cancelled.
    Closed,
}

/// The system tray list while it is open.
#[derive(Debug, Clone)]
pub struct TrayDialog {
    pub id: u32,
    icons: Vec<TrayIcon>,
    selected: usize,
}

impl TrayDialog {
    pub fn new(id: u32, icons: Vec<TrayIcon>) -> Self {
        Self {
            id,
            icons,
            selected: 0,
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
        };
        let button = |id: &str, label: String, default: bool| {
            control(id, label, ControlKind::Button { default })
        };
        DialogSpec {
            id: self.id,
            title: msg::tray_title().to_string(),
            controls: vec![
                control(
                    "icons",
                    msg::tray_title().to_string(),
                    ControlKind::ListBox {
                        items: self.icons.iter().map(|i| i.name.clone()).collect(),
                        selected: self.selected,
                    },
                ),
                button("right", msg::tray_right_click().to_string(), true),
                button("left", msg::tray_left_click().to_string(), false),
                button("double", msg::tray_left_double_click().to_string(), false),
                button("cancel", msg::tray_cancel().to_string(), false),
            ],
            keys: Vec::new(),
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                if let ("icons", Value::Selected(i)) = (control.as_str(), value) {
                    self.selected = *i;
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => {
                let click = match control.as_str() {
                    "right" => MouseClick::RightSingle,
                    "left" => MouseClick::LeftSingle,
                    "double" => MouseClick::LeftDouble,
                    _ => return Outcome::Nothing,
                };
                self.icons
                    .get(self.selected)
                    .map_or(Outcome::Nothing, |icon| Outcome::Click(icon.clone(), click))
            }
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(name: &str, x: i32) -> TrayIcon {
        TrayIcon {
            name: name.to_owned(),
            x,
            y: 1060,
        }
    }

    fn dialog() -> TrayDialog {
        TrayDialog::new(
            9,
            vec![
                icon("Notification Chevron", 1500),
                icon("Touch keyboard", 1600),
                icon("Action Center, 4 new notifications", 1900),
            ],
        )
    }

    #[test]
    fn the_dialog_is_jaws_system_tray_list() {
        let spec = dialog().spec();
        assert_eq!(spec.title, "Select a System Tray Icon");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Select a System Tray Icon",
                "Right Single Click",
                "Left Single Click",
                "Left Double Click",
                "Cancel"
            ]
        );
        assert!(matches!(
            spec.controls.get(1).map(|c| &c.kind),
            Some(ControlKind::Button { default: true })
        ));
    }

    #[test]
    fn a_button_clicks_the_selected_icon() {
        let mut dialog = dialog();
        let _ = dialog.handle(&FromUi::DialogChanged {
            id: 9,
            control: "icons".to_owned(),
            value: Value::Selected(1),
        });
        let press = |control: &str| FromUi::DialogPressed {
            id: 9,
            control: control.to_owned(),
        };
        assert_eq!(
            dialog.handle(&press("right")),
            Outcome::Click(icon("Touch keyboard", 1600), MouseClick::RightSingle)
        );
        assert_eq!(
            dialog.handle(&press("double")),
            Outcome::Click(icon("Touch keyboard", 1600), MouseClick::LeftDouble)
        );
        assert_eq!(dialog.handle(&press("cancel")), Outcome::Nothing);
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 9 }),
            Outcome::Closed
        );
    }
}
