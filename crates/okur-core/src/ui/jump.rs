//! Jump To Line (J in a page): a line number to move the virtual cursor to. Its controls are
//! JAWS 2025's [obs, 559/s1]: the line number's field, named "Enter Line Number (1 to 73):"
//! after the page's lines, holding the cursor's line, then OK (the default) and Cancel. JAWS's
//! help: the dialog "shows the number of lines available and the line number of your current
//! location", and Shift+J returns to where the jump was made.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value};

use crate::messages::msg;

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// OK (or Enter) with a line number, 1 for the first line, within the page's lines.
    Jump(usize),
    /// The dialog was cancelled, or OK pressed without a number.
    Closed,
}

/// Jump To Line while it is open.
#[derive(Debug, Clone)]
pub struct JumpDialog {
    pub id: u32,
    lines: usize,
    text: String,
}

impl JumpDialog {
    /// The dialog over a page of `lines` lines, the cursor on line `current` (1 first).
    pub fn new(id: u32, lines: usize, current: usize) -> Self {
        Self {
            id,
            lines,
            text: current.to_string(),
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
            hidden: false,
        };
        DialogSpec {
            id: self.id,
            title: msg::jump_title().to_string(),
            controls: vec![
                control(
                    "line",
                    msg::jump_line_number(self.lines).to_string(),
                    ControlKind::Edit {
                        text: self.text.clone(),
                    },
                ),
                control(
                    "ok",
                    msg::jump_ok().to_string(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "cancel",
                    msg::jump_cancel().to_string(),
                    ControlKind::Button { default: false },
                ),
            ],
            keys: Vec::new(),
        }
    }

    /// The line OK goes to: the number typed, at most the page's last line (JAWS took 99 on
    /// a page of 73 lines to its last [obs, 559/s14]); none for no number.
    fn line(&self) -> Option<usize> {
        let number: usize = self.text.trim().parse().ok()?;
        (number > 0).then(|| number.min(self.lines))
    }

    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                if let ("line", Value::Text(text)) = (control.as_str(), value) {
                    self.text = text.clone();
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" => self.line().map_or(Outcome::Closed, Outcome::Jump),
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

    #[test]
    fn the_dialog_names_the_pages_lines_and_holds_the_cursors_line() {
        let spec = JumpDialog::new(4, 73, 1).spec();
        assert_eq!(spec.title, "Jump To Line");
        assert_eq!(
            spec.controls.first().map(|c| (c.label.as_str(), &c.kind)),
            Some((
                "Enter Line Number (1 to 73):",
                &ControlKind::Edit {
                    text: "1".to_owned()
                }
            ))
        );
    }

    #[test]
    fn ok_jumps_to_the_line_typed_at_most_the_last() {
        let typed = |text: &str| {
            let mut dialog = JumpDialog::new(4, 73, 1);
            let _ = dialog.handle(&FromUi::DialogChanged {
                id: 4,
                control: "line".to_owned(),
                value: Value::Text(text.to_owned()),
            });
            dialog.handle(&FromUi::DialogPressed {
                id: 4,
                control: "ok".to_owned(),
            })
        };
        assert_eq!(typed("12"), Outcome::Jump(12));
        assert_eq!(typed("99"), Outcome::Jump(73));
        assert_eq!(typed(""), Outcome::Closed);
        assert_eq!(typed("0"), Outcome::Closed);
    }
}
