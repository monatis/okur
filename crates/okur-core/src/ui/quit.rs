//! The question before Okur exits (Insert+F4, JAWS's ShutDownJAWS), while JAWS's "Prompt to
//! Confirm When Exiting JAWS" is on. JAWS's help (Options > Exit): "a dialog is brought up to
//! confirm whether or not you wish to exit the JAWS application. Press ENTER to exit JAWS, or
//! press ESC to return". The question is static text and OK, the default, has the focus, so
//! Enter exits and Escape returns. Its words wait for scenario 571's recording
//! (`locales/en-US/dialogs.ftl`).

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi};

use crate::messages::msg;

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// OK (or Enter): Okur exits.
    Exit,
    /// Cancel or Escape: the dialog is gone, and Okur stays.
    Closed,
}

/// The question while it is open.
#[derive(Debug, Clone)]
pub struct QuitDialog {
    pub id: u32,
}

impl QuitDialog {
    pub fn new(id: u32) -> Self {
        Self { id }
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
            title: msg::quit_title().to_string(),
            controls: vec![
                control(
                    "question",
                    String::new(),
                    ControlKind::Message {
                        text: msg::quit_question().to_string(),
                    },
                ),
                control(
                    "ok",
                    msg::quit_ok().to_string(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "cancel",
                    msg::quit_cancel().to_string(),
                    ControlKind::Button { default: false },
                ),
            ],
            keys: Vec::new(),
        }
    }

    pub fn handle(&self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogPressed { id, control } if *id == self.id && control == "ok" => {
                Outcome::Exit
            }
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_question_is_static_text_with_ok_the_default() {
        let spec = QuitDialog::new(7).spec();
        assert_eq!(spec.title, "Quit Okur");
        let kinds: Vec<(&str, &ControlKind)> = spec
            .controls
            .iter()
            .map(|c| (c.id.as_str(), &c.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                (
                    "question",
                    &ControlKind::Message {
                        text: "Are you sure you want to quit Okur?  Press Enter to quit now, or Escape to cancel."
                            .to_owned()
                    }
                ),
                ("ok", &ControlKind::Button { default: true }),
                ("cancel", &ControlKind::Button { default: false }),
            ]
        );
    }

    #[test]
    fn ok_exits_and_cancel_closes() {
        let dialog = QuitDialog::new(7);
        let pressed = |id, control: &str| FromUi::DialogPressed {
            id,
            control: control.to_owned(),
        };
        assert_eq!(dialog.handle(&pressed(7, "ok")), Outcome::Exit);
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 7 }),
            Outcome::Closed
        );
        // Another dialog's OK is not this one's.
        assert_eq!(dialog.handle(&pressed(8, "ok")), Outcome::Nothing);
    }
}
