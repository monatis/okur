//! The question before Ctrl+C or Ctrl+X overwrites a clipboard the selection was added to
//! (Okur key+Windows+C), as JAWS asks it: "Overwrite Clipboard", "Pressing Control+C or
//! Control+X will overwrite the clipboard. Do you want to continue this operation?", Yes and
//! No [obs, 825/s18]. Yes (or Y) copies or cuts [obs, 825/s19].

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi};

use crate::messages::msg;

/// What the engine does after a message from the question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Yes: the copy or the cut goes ahead.
    Overwrite,
    /// No, or Escape: the clipboard stays as it is.
    Keep,
}

/// The question while it is open.
#[derive(Debug, Clone)]
pub struct OverwriteDialog {
    pub id: u32,
    /// Ctrl+X asked it, not Ctrl+C.
    pub cut: bool,
    /// The selection the copy takes.
    pub selection: String,
    /// The window the question opened over, which the focus returns to without its title
    /// being said again ("edit | Type in text." [obs, 825/s19]).
    pub window: Option<String>,
    /// That window's class, as the focus last reported it.
    pub window_kind: Option<String>,
}

impl OverwriteDialog {
    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
        };
        DialogSpec {
            id: self.id,
            title: msg::clipboard_overwrite_title().to_string(),
            controls: vec![
                control(
                    "question",
                    String::new(),
                    ControlKind::Message {
                        text: msg::clipboard_overwrite_question().to_string(),
                    },
                ),
                control(
                    "yes",
                    msg::clipboard_overwrite_yes().to_string(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "cancel",
                    msg::clipboard_overwrite_no().to_string(),
                    ControlKind::Button { default: false },
                ),
            ],
            keys: Vec::new(),
        }
    }

    pub fn handle(&self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogPressed { id, control } if *id == self.id && control == "yes" => {
                Outcome::Overwrite
            }
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Keep,
            _ => Outcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Yes overwrites; No and Escape close the question, which No's id "cancel" makes one.
    #[test]
    fn yes_overwrites_and_no_keeps() {
        let dialog = OverwriteDialog {
            id: 7,
            cut: false,
            selection: "fn main() {".to_owned(),
            window: None,
            window_kind: None,
        };
        let spec = dialog.spec();
        assert_eq!(spec.title, "Overwrite Clipboard");
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 7,
                control: "yes".to_owned()
            }),
            Outcome::Overwrite
        );
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 7 }),
            Outcome::Keep
        );
    }
}
