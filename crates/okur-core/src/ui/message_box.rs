//! Okur's message box: a message and OK, as JAWS shows its own ("JAWS for Windows Message Box
//! dialog | OCR Failed to start. No file currently selected. | OK Button" [obs, 1026/s3]). OK,
//! Enter or Escape closes it, and the focus returns to the window it was asked in.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi};

use crate::messages::msg;

/// A message box while it is open.
#[derive(Debug, Clone)]
pub struct MessageBox {
    pub id: u32,
    pub text: String,
    /// The kind of window it was shown over, which the focus returns to: back from Okur's own
    /// box, the window is the same application's, its hint said [obs, 1026/s4].
    pub window_kind: Option<String>,
}

impl MessageBox {
    pub fn new(id: u32, text: String) -> Self {
        Self {
            id,
            text,
            window_kind: None,
        }
    }

    pub fn spec(&self) -> DialogSpec {
        DialogSpec {
            id: self.id,
            title: msg::message_box_title().to_string(),
            controls: vec![
                Control {
                    id: "message".to_owned(),
                    label: String::new(),
                    kind: ControlKind::Message {
                        text: self.text.clone(),
                    },
                    enabled: true,
                },
                Control {
                    id: "ok".to_owned(),
                    label: msg::message_box_ok().to_string(),
                    kind: ControlKind::Button { default: true },
                    enabled: true,
                },
            ],
            keys: Vec::new(),
        }
    }

    /// Whether `event` closes it: OK pressed, or the window closed (Escape).
    pub fn closes(&self, event: &FromUi) -> bool {
        match event {
            FromUi::DialogPressed { id, control } => *id == self.id && control == "ok",
            FromUi::DialogClosed { id } => *id == self.id,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_is_static_text_with_ok_the_default() {
        let spec = MessageBox::new(3, "OCR Failed to start.".to_owned()).spec();
        assert_eq!(spec.title, "Okur for Windows Message Box");
        let kinds: Vec<(&str, &ControlKind)> = spec
            .controls
            .iter()
            .map(|c| (c.id.as_str(), &c.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                (
                    "message",
                    &ControlKind::Message {
                        text: "OCR Failed to start.".to_owned()
                    }
                ),
                ("ok", &ControlKind::Button { default: true }),
            ]
        );
    }

    #[test]
    fn ok_and_escape_close_it() {
        let dialog = MessageBox::new(3, String::new());
        assert!(dialog.closes(&FromUi::DialogPressed {
            id: 3,
            control: "ok".to_owned()
        }));
        assert!(dialog.closes(&FromUi::DialogClosed { id: 3 }));
        // Another dialog's OK is not this one's.
        assert!(!dialog.closes(&FromUi::DialogClosed { id: 4 }));
    }
}
