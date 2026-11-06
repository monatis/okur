//! Manage Key Labels (Settings Center, Keyboard, General, "Manage Key Labels..."): every key's
//! label and mute status, as JAWS's dialog lists them [obs, the walk of Manage Key Labels,
//! key-labels-walk.ps1, old-pc 2026-11-06]:
//! - a list of the keys in three columns, Key Name, Key Label and Mute Status, "Add | + | Speak |
//!   1 of 103", the first row selected as it opens [obs, 1169/s18]; each row read whole as the
//!   arrows reach it, "Apps | Applications | Speak | 2 of 103" (s19); F2 edits the selected row's
//!   label in place, " Edit | Tab | Type in text." (s31);
//! - Toggle Mute, which turns the selected key between Speak and Mute (an Always Speak key stays
//!   as it is) and says the row again, "Space Space Mute"; Change Label, the label edited as F2
//!   does; OK and Cancel.
//!
//! OK gives Settings Center's draft the changes ([`crate::key_labels::KeyLabelChange`]), kept
//! when Settings Center's own OK or Apply is pressed, as JAWS keeps them in the application's
//! settings file [obs, key-labels-store-probe.ps1]; Cancel drops them.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use crate::key_labels::{self, KeyLabel, KeyLabelChange, MuteStatus};
use crate::messages::msg;

/// The list's control id.
const KEYS: &str = "keys";
/// The label's column, which F2 and Change Label edit.
const LABEL_COLUMN: usize = 1;

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyLabelsOutcome {
    Nothing,
    /// Send these to the dialog.
    Send(Vec<ToUi>),
    /// Send these, and say this (the row Toggle Mute changed).
    Speak(Vec<ToUi>, String),
    /// OK: these are the key labels' changes now.
    Done(Vec<KeyLabelChange>),
    /// Cancel, Escape, or the dialog closed: nothing changes.
    Closed,
}

/// Manage Key Labels while it is open.
#[derive(Debug, Clone)]
pub struct KeyLabelsDialog {
    pub id: u32,
    changes: Vec<KeyLabelChange>,
    selected: usize,
}

fn button(id: &str, label: String, default: bool) -> Control {
    Control {
        id: id.to_owned(),
        label,
        kind: ControlKind::Button { default },
        enabled: true,
        hidden: false,
    }
}

/// A row's mute status, as the list says it.
fn status_text(status: MuteStatus) -> String {
    match status {
        MuteStatus::Speak => msg::key_labels_speak(),
        MuteStatus::Mute => msg::key_labels_mute(),
        MuteStatus::AlwaysSpeak => msg::key_labels_always_speak(),
    }
    .to_string()
}

/// A row's cells: its name, its label ("No Label" for a group of keys), its status.
fn cells(row: &KeyLabel) -> Vec<String> {
    let label = if row.label.is_empty() {
        msg::key_labels_no_label().to_string()
    } else {
        row.label.clone()
    };
    vec![row.name.clone(), label, status_text(row.status)]
}

impl KeyLabelsDialog {
    /// The dialog on `changes`, the key labels Settings Center's draft holds.
    pub fn new(id: u32, changes: Vec<KeyLabelChange>) -> Self {
        Self {
            id,
            changes,
            selected: 0,
        }
    }

    fn rows(&self) -> Vec<KeyLabel> {
        key_labels::rows(&self.changes)
    }

    fn table_rows(&self) -> Vec<Vec<String>> {
        self.rows().iter().map(cells).collect()
    }

    pub fn spec(&self) -> DialogSpec {
        DialogSpec {
            id: self.id,
            title: msg::key_labels_title().to_string(),
            controls: vec![
                Control {
                    id: KEYS.to_owned(),
                    label: msg::key_labels_list().to_string(),
                    kind: ControlKind::Table {
                        columns: vec![
                            msg::key_labels_column_name().to_string(),
                            msg::key_labels_column_label().to_string(),
                            msg::key_labels_column_status().to_string(),
                        ],
                        rows: self.table_rows(),
                        selected: Some(self.selected),
                        edit_column: Some(LABEL_COLUMN),
                    },
                    enabled: true,
                    hidden: false,
                },
                button("toggle", msg::key_labels_toggle_mute().to_string(), false),
                button("change", msg::key_labels_change_label().to_string(), false),
                button("ok", msg::key_labels_ok().to_string(), true),
                button("cancel", msg::key_labels_cancel().to_string(), false),
            ],
            keys: Vec::new(),
        }
    }

    /// The changes as they are now.
    pub fn changes(&self) -> &[KeyLabelChange] {
        &self.changes
    }

    /// `row` as `label` and `muted`: its change kept, or dropped where it is the default.
    fn set(&mut self, row: &KeyLabel, label: String, muted: bool) {
        self.changes
            .retain(|c| !c.key.eq_ignore_ascii_case(&row.name));
        let default = key_labels::defaults()
            .iter()
            .find(|d| d.name == row.name)
            .is_some_and(|d| d.label == label && (d.status == MuteStatus::Mute) == muted);
        if !default {
            self.changes.push(KeyLabelChange {
                key: row.name.clone(),
                label,
                muted,
            });
        }
    }

    /// The list's rows again, the selected one kept.
    fn rows_shown(&self) -> ToUi {
        ToUi::DialogRows {
            id: self.id,
            control: KEYS.to_owned(),
            rows: self.table_rows(),
            checked: Vec::new(),
            selected: self.selected,
        }
    }

    /// Toggle Mute: the selected key's status turned, and its row said again, its cells
    /// without the list's separators, "Space Space Mute" [obs, key-labels-store-probe.ps1].
    fn toggle(&mut self) -> KeyLabelsOutcome {
        let Some(row) = self.rows().into_iter().nth(self.selected) else {
            return KeyLabelsOutcome::Nothing;
        };
        if row.status != MuteStatus::AlwaysSpeak {
            let muted = row.status != MuteStatus::Mute;
            self.set(&row, row.label.clone(), muted);
        }
        let Some(now) = self.rows().into_iter().nth(self.selected) else {
            return KeyLabelsOutcome::Nothing;
        };
        let said = [now.name.clone(), now.label.clone(), status_text(now.status)]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        KeyLabelsOutcome::Speak(vec![self.rows_shown()], said)
    }

    pub fn handle(&mut self, message: &FromUi) -> KeyLabelsOutcome {
        match message {
            FromUi::DialogChanged { id, control, value } if *id == self.id && control == KEYS => {
                match value {
                    Value::Selected(row) => {
                        self.selected = *row;
                        KeyLabelsOutcome::Nothing
                    }
                    // The label edited in place: kept as typed, its status as it was.
                    Value::ItemRenamed(index, text) => {
                        let Some(row) = self.rows().into_iter().nth(*index) else {
                            return KeyLabelsOutcome::Nothing;
                        };
                        self.selected = *index;
                        let muted = row.status == MuteStatus::Mute;
                        self.set(&row, text.trim().to_owned(), muted);
                        KeyLabelsOutcome::Send(vec![self.rows_shown()])
                    }
                    _ => KeyLabelsOutcome::Nothing,
                }
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "toggle" => self.toggle(),
                "change" => KeyLabelsOutcome::Send(vec![ToUi::DialogEditCell {
                    id: self.id,
                    control: KEYS.to_owned(),
                }]),
                "ok" => KeyLabelsOutcome::Done(self.changes.clone()),
                "cancel" => KeyLabelsOutcome::Closed,
                _ => KeyLabelsOutcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => KeyLabelsOutcome::Closed,
            _ => KeyLabelsOutcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pressed(control: &str) -> FromUi {
        FromUi::DialogPressed {
            id: 4,
            control: control.to_owned(),
        }
    }

    fn select(row: usize) -> FromUi {
        FromUi::DialogChanged {
            id: 4,
            control: KEYS.to_owned(),
            value: Value::Selected(row),
        }
    }

    fn row_of(name: &str) -> usize {
        key_labels::defaults()
            .iter()
            .position(|r| r.name == name)
            .unwrap_or(usize::MAX)
    }

    #[test]
    fn it_lists_every_key_with_its_label_and_status() {
        let spec = KeyLabelsDialog::new(4, Vec::new()).spec();
        let row = |rows: &[Vec<String>], i: usize| rows.get(i).map(|r| r.join("|"));
        let table = spec.controls.first().map(|c| c.kind.clone());
        assert!(matches!(&table, Some(ControlKind::Table {
            columns, rows, selected: Some(0), edit_column: Some(1)
        }) if columns.join("|") == "Key Name|Key Label|Mute Status"
            && rows.len() == 103
            && row(rows, 0).as_deref() == Some("Add|+|Speak")
            && row(rows, row_of("Letters")).as_deref() == Some("Letters|No Label|Speak")
            && row(rows, row_of("Tab")).as_deref() == Some("Tab|Tab|Mute")));
        let buttons: Vec<&str> = spec
            .controls
            .iter()
            .skip(1)
            .map(|c| c.label.as_str())
            .collect();
        assert_eq!(buttons, ["&Toggle Mute", "&Change Label", "OK", "Cancel"]);
    }

    #[test]
    fn toggle_mute_turns_a_key_and_says_its_row() {
        let mut dialog = KeyLabelsDialog::new(4, Vec::new());
        dialog.handle(&select(row_of("Space")));
        assert!(matches!(dialog.handle(&pressed("toggle")),
            KeyLabelsOutcome::Speak(_, said) if said == "Space Space Mute"));
        assert_eq!(
            dialog.handle(&pressed("ok")),
            KeyLabelsOutcome::Done(vec![KeyLabelChange {
                key: "Space".into(),
                label: "Space".into(),
                muted: true,
            }])
        );
        // Toggled back, it is the default again, and no change is kept.
        dialog.handle(&pressed("toggle"));
        assert!(dialog.changes().is_empty());
        // Shift is always spoken.
        dialog.handle(&select(row_of("Shift")));
        assert!(matches!(dialog.handle(&pressed("toggle")),
            KeyLabelsOutcome::Speak(_, said) if said == "Shift Shift Always Speak"));
        assert!(dialog.changes().is_empty());
    }

    #[test]
    fn a_label_edited_in_place_is_kept() {
        let mut dialog = KeyLabelsDialog::new(4, Vec::new());
        let row = row_of("Return");
        dialog.handle(&select(row));
        assert!(matches!(
            dialog.handle(&pressed("change")),
            KeyLabelsOutcome::Send(sent) if matches!(sent.first(), Some(ToUi::DialogEditCell { .. }))
        ));
        dialog.handle(&FromUi::DialogChanged {
            id: 4,
            control: KEYS.to_owned(),
            value: Value::ItemRenamed(row, "Return key".into()),
        });
        assert_eq!(
            dialog.changes(),
            [KeyLabelChange {
                key: "Return".into(),
                label: "Return key".into(),
                muted: false,
            }]
        );
        assert_eq!(dialog.handle(&pressed("cancel")), KeyLabelsOutcome::Closed);
    }
}
