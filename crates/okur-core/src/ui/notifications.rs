//! The Notification History (Insert+Space, N) and the details of a notification. Its controls
//! are JAWS 2025's [obs, 774]: the list "Recent Notifications", the latest first, each row a
//! notification's text, its sender and the time it came ("New notification from Windows
//! PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1 Windows PowerShell 11:07 PM");
//! then Enable Rules (checked), Create Rule..., Clear History and Close, in that Tab order.
//! Enter on a row shows its details: "Notification Details", the text in a read-only edit
//! "Notification Text:" [obs, 774/s11]. JAWS keeps the past 24 hours (JAWS 2025 help,
//! Notification History).

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value};

use crate::messages::{msg, StaticMessage};

/// A notification as it came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub text: String,
    /// The application it came from, as the toast names it ("Windows PowerShell").
    pub sender: Option<String>,
    /// When it came, as Say System Time says it ("11:07 PM").
    pub time: String,
}

impl Notification {
    /// Its row in the history: text, sender, time.
    pub fn row(&self) -> String {
        [
            Some(self.text.as_str()),
            self.sender.as_deref(),
            Some(self.time.as_str()),
        ]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
    }
}

/// What the engine does after the history handled a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Enter on row `usize`: its details.
    Details(usize),
    /// Create Rule...
    CreateRule,
    /// Clear History.
    Clear,
    Closed,
}

/// The Notification History while it is open.
#[derive(Debug, Clone)]
pub struct HistoryDialog {
    pub id: u32,
    rows: Vec<String>,
    selected: usize,
    rules: bool,
}

fn control(id: &str, label: StaticMessage, kind: ControlKind) -> Control {
    Control {
        id: id.to_owned(),
        label: label.to_string(),
        kind,
        enabled: true,
    }
}

impl HistoryDialog {
    /// The history over `rows`, the latest first.
    pub fn new(id: u32, rows: Vec<String>, rules: bool) -> Self {
        Self {
            id,
            rows,
            selected: 0,
            rules,
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let button = |id: &str, label: StaticMessage| {
            control(id, label, ControlKind::Button { default: false })
        };
        DialogSpec {
            id: self.id,
            title: msg::notification_history_title().to_string(),
            controls: vec![
                control(
                    "notifications",
                    msg::notification_history_list(),
                    ControlKind::ListBox {
                        items: self.rows.clone(),
                        selected: 0,
                    },
                ),
                control(
                    "rules",
                    msg::notification_history_rules(),
                    ControlKind::CheckBox {
                        checked: self.rules,
                    },
                ),
                button("create", msg::notification_history_create_rule()),
                button("clear", msg::notification_history_clear()),
                button("cancel", msg::notification_history_close()),
            ],
        }
    }

    /// Whether rules are on (Enable Rules).
    pub fn rules(&self) -> bool {
        self.rules
    }

    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("notifications", Value::Selected(i)) => self.selected = *i,
                    ("rules", Value::Checked(on)) => self.rules = *on,
                    _ => {}
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "notifications" if self.selected < self.rows.len() => {
                    Outcome::Details(self.selected)
                }
                "create" => Outcome::CreateRule,
                "clear" => {
                    self.rows.clear();
                    self.selected = 0;
                    Outcome::Clear
                }
                "cancel" => Outcome::Closed,
                _ => Outcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

/// A notification's details: "Notification Details", its text in the read-only edit
/// "Notification Text:" [obs, 774/s11].
pub fn details(id: u32, text: &str) -> DialogSpec {
    DialogSpec {
        id,
        title: msg::notification_details_title().to_string(),
        controls: vec![control(
            "text",
            msg::notification_details_text(),
            ControlKind::Note {
                text: text.to_owned(),
            },
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_history_is_jaws_notification_history() {
        let toast = Notification {
            text: "New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1".to_owned(),
            sender: Some("Windows PowerShell".to_owned()),
            time: "11:07 PM".to_owned(),
        };
        let mut dialog = HistoryDialog::new(4, vec![toast.row()], true);
        let spec = dialog.spec();
        assert_eq!(spec.title, "Notification History");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Recent Notifications",
                "Enable Rules",
                "Create Rule...",
                "Clear History",
                "Close"
            ]
        );
        assert_eq!(
            spec.controls.first().map(|c| c.kind.clone()),
            Some(ControlKind::ListBox {
                items: vec!["New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1 Windows PowerShell 11:07 PM".to_owned()],
                selected: 0
            })
        );
        let pressed = |dialog: &mut HistoryDialog, control: &str| {
            dialog.handle(&FromUi::DialogPressed {
                id: 4,
                control: control.to_owned(),
            })
        };
        assert_eq!(pressed(&mut dialog, "notifications"), Outcome::Details(0));
        assert_eq!(pressed(&mut dialog, "clear"), Outcome::Clear);
        // Nothing left to show.
        assert_eq!(pressed(&mut dialog, "notifications"), Outcome::Nothing);
        assert_eq!(pressed(&mut dialog, "cancel"), Outcome::Closed);
    }
}
