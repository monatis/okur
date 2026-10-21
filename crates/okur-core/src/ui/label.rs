//! Custom Label (Insert+Ctrl+Tab in a page): a name for the element at the virtual cursor,
//! read in place of the page's own. Its controls are JAWS 2025's [obs, 688/s9]: the dialog's
//! text "Enter a custom label for this item", a field of that name holding the element's
//! label, OK (the default) and Cancel. OK keeps the label; an empty one takes it away.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value};

use crate::messages::msg;

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// OK (or Enter) with the label typed: empty takes the element's label away.
    Label(String),
    /// The dialog was cancelled.
    Closed,
}

/// Custom Label while it is open.
#[derive(Debug, Clone)]
pub struct LabelDialog {
    pub id: u32,
    text: String,
}

impl LabelDialog {
    /// The dialog for an element whose label is `current` (empty for none).
    pub fn new(id: u32, current: &str) -> Self {
        Self {
            id,
            text: current.to_owned(),
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
        };
        let prompt = msg::custom_label_prompt().to_string();
        DialogSpec {
            id: self.id,
            title: msg::custom_label_title().to_string(),
            controls: vec![
                control(
                    "prompt",
                    String::new(),
                    ControlKind::Message {
                        text: prompt.clone(),
                    },
                ),
                control(
                    "label",
                    prompt,
                    ControlKind::Edit {
                        text: self.text.clone(),
                    },
                ),
                control(
                    "ok",
                    msg::custom_label_ok().to_string(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "cancel",
                    msg::custom_label_cancel().to_string(),
                    ControlKind::Button { default: false },
                ),
            ],
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                if let ("label", Value::Text(text)) = (control.as_str(), value) {
                    self.text = text.clone();
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" => Outcome::Label(self.text.trim().to_owned()),
                "cancel" => Outcome::Closed,
                _ => Outcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JAWS's dialog: "Custom Label", its text and a field of the same name [obs, 688/s9].
    #[test]
    fn the_dialog_is_jaws_custom_label() {
        let spec = LabelDialog::new(5, "").spec();
        assert_eq!(spec.title, "Custom Label");
        let kinds: Vec<(&str, &str)> = spec
            .controls
            .iter()
            .map(|c| (c.id.as_str(), c.label.as_str()))
            .collect();
        assert_eq!(
            kinds,
            [
                ("prompt", ""),
                ("label", "Enter a custom label for this item"),
                ("ok", "OK"),
                ("cancel", "Cancel")
            ]
        );
    }

    #[test]
    fn ok_gives_the_label_typed() {
        let mut dialog = LabelDialog::new(5, "old");
        let _ = dialog.handle(&FromUi::DialogChanged {
            id: 5,
            control: "label".to_owned(),
            value: Value::Text(" lamp ".to_owned()),
        });
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 5,
                control: "ok".to_owned(),
            }),
            Outcome::Label("lamp".to_owned())
        );
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 5,
                control: "cancel".to_owned(),
            }),
            Outcome::Closed
        );
    }
}
