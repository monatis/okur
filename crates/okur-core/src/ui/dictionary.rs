//! The Dictionary Manager (Insert+D) and the dialogs it opens: the speech dictionary's rules
//! ([`crate::dictionary`]), listed, added, changed and deleted. Its controls are JAWS 2025's
//! [obs, 1154; JAWS 2025 help, Dictionary Manager Basics]:
//! - Insert+D in an application without a dictionary of its own asks for one first, "New
//!   Dictionary dialog | File Name: Edit | notepad.JDF"; Escape opens the default one, for all
//!   applications [obs, 1154/s4, s5];
//! - the manager, "default.jdf - JAWS Dictionary Manager": the tree "Dictionary Entries", a rule
//!   an item "total=sum" found by its first letter, then Change... (with a rule chosen), Delete
//!   and Add..., the focus on Add... as it opens [obs, 1154/s5-s8, s27-s29];
//! - Add... with the word at the cursor as the Actual Word, "Add Dictionary Definition dialog |
//!   Actual Word: Edit | total", then Replacement Word, and Case Sensitive; OK keeps the rule
//!   and the manager is in front again on Add... [obs, 1154/s9-s14];
//! - Delete asks first, "Confirm Dictionary Entries Delete dialog | Are you sure you want to
//!   delete the dictionary entry total=sum? | Yes Button" [obs, 1154/s30, s31].
//!
//! JAWS's rules may also play a sound or speak in another language, and a rule may be limited
//! to a language, a synthesizer or a voice; Okur's rules do not do that yet, and the add dialog
//! leaves those controls out.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, TreeItem, Value};

use crate::dictionary::DictionaryEntry;
use crate::messages::msg;

fn control(id: &str, label: String, kind: ControlKind, enabled: bool) -> Control {
    Control {
        id: id.to_owned(),
        label,
        kind,
        enabled,
        hidden: false,
    }
}

fn button(id: &str, label: String, default: bool, enabled: bool) -> Control {
    control(id, label, ControlKind::Button { default }, enabled)
}

/// A rule as the manager lists it: "total=sum".
pub fn entry_text(entry: &DictionaryEntry) -> String {
    msg::dictionary_entry(entry.actual.as_str(), entry.replacement.as_str()).to_string()
}

/// What the engine does after a message from the New Dictionary question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewOutcome {
    Nothing,
    /// OK: the application's own dictionary.
    Application,
    /// Escape or Cancel: the default dictionary, for all applications.
    Default,
}

/// The question Insert+D asks in an application without a dictionary of its own: its name,
/// which OK takes, or Escape for the default one.
#[derive(Debug, Clone)]
pub struct NewDialog {
    pub id: u32,
    name: String,
}

impl NewDialog {
    pub fn new(id: u32, name: &str) -> Self {
        Self {
            id,
            name: name.to_owned(),
        }
    }

    pub fn spec(&self) -> DialogSpec {
        DialogSpec {
            id: self.id,
            title: msg::dictionary_new_title().to_string(),
            controls: vec![
                control(
                    "name",
                    msg::dictionary_new_name().to_string(),
                    ControlKind::Edit {
                        text: self.name.clone(),
                    },
                    true,
                ),
                button("ok", msg::dictionary_new_ok().to_string(), true, true),
                button(
                    "cancel",
                    msg::dictionary_new_cancel().to_string(),
                    false,
                    true,
                ),
            ],
            keys: Vec::new(),
        }
    }

    pub fn handle(&self, event: &FromUi) -> NewOutcome {
        match event {
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" => NewOutcome::Application,
                "cancel" => NewOutcome::Default,
                _ => NewOutcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => NewOutcome::Default,
            _ => NewOutcome::Nothing,
        }
    }
}

/// What the engine does after a message from the manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerOutcome {
    Nothing,
    /// A rule was chosen: Change... becomes available.
    Chosen(usize),
    Add,
    Change(usize),
    Delete(usize),
    Closed,
}

/// The manager while it is open, on one dictionary's rules.
#[derive(Debug, Clone)]
pub struct ManagerDialog {
    pub id: u32,
    name: String,
    rows: Vec<String>,
    chosen: Option<usize>,
}

impl ManagerDialog {
    /// The manager on the dictionary named `name` ("default", or an application's), with its
    /// rules as `rows`.
    pub fn new(id: u32, name: &str, entries: &[DictionaryEntry]) -> Self {
        Self {
            id,
            name: name.to_owned(),
            rows: entries.iter().map(entry_text).collect(),
            chosen: None,
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let items = self
            .rows
            .iter()
            .map(|text| TreeItem {
                text: text.clone(),
                level: 0,
                open: true,
                checked: None,
            })
            .collect();
        let any = !self.rows.is_empty();
        DialogSpec {
            id: self.id,
            title: msg::dictionary_manager_title(self.name.as_str()).to_string(),
            controls: vec![
                control(
                    "entries",
                    msg::dictionary_manager_entries().to_string(),
                    ControlKind::Tree { items, selected: 0 },
                    true,
                ),
                button(
                    "change",
                    msg::dictionary_manager_change().to_string(),
                    false,
                    self.chosen.is_some(),
                ),
                button(
                    "delete",
                    msg::dictionary_manager_delete().to_string(),
                    false,
                    any,
                ),
                button("add", msg::dictionary_manager_add().to_string(), true, true),
            ],
            keys: Vec::new(),
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> ManagerOutcome {
        match event {
            FromUi::DialogChanged {
                id,
                control,
                value: Value::Selected(row),
            } if *id == self.id && control == "entries" && *row < self.rows.len() => {
                let first = self.chosen.is_none();
                self.chosen = Some(*row);
                if first {
                    ManagerOutcome::Chosen(*row)
                } else {
                    ManagerOutcome::Nothing
                }
            }
            FromUi::DialogPressed { id, control } if *id == self.id => {
                // Without a rule chosen, Delete acts on the first, the tree's selection.
                let row = self.chosen.or((!self.rows.is_empty()).then_some(0));
                match (control.as_str(), row) {
                    ("add", _) => ManagerOutcome::Add,
                    ("change", Some(row)) if self.chosen.is_some() => ManagerOutcome::Change(row),
                    ("delete", Some(row)) => ManagerOutcome::Delete(row),
                    _ => ManagerOutcome::Nothing,
                }
            }
            FromUi::DialogClosed { id } if *id == self.id => ManagerOutcome::Closed,
            _ => ManagerOutcome::Nothing,
        }
    }
}

/// What the engine does after a message from Add or Change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleOutcome {
    Nothing,
    /// OK, with the rule as the dialog leaves it.
    Done(DictionaryEntry),
    Closed,
}

/// Add Dictionary Definition, or Change Dictionary Definition for a rule already kept: the
/// Actual Word, the Replacement Word, Case Sensitive, OK and Cancel.
#[derive(Debug, Clone)]
pub struct RuleDialog {
    pub id: u32,
    change: bool,
    entry: DictionaryEntry,
}

impl RuleDialog {
    /// Add, proposing `word` (the word at the cursor) as the Actual Word [obs, 1154/s9].
    pub fn add(id: u32, word: &str) -> Self {
        Self {
            id,
            change: false,
            entry: DictionaryEntry::new(word, ""),
        }
    }

    /// Change, for a rule as it is.
    pub fn change(id: u32, entry: &DictionaryEntry) -> Self {
        Self {
            id,
            change: true,
            entry: entry.clone(),
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let title = if self.change {
            msg::dictionary_change_title()
        } else {
            msg::dictionary_add_title()
        };
        DialogSpec {
            id: self.id,
            title: title.to_string(),
            controls: vec![
                control(
                    "actual",
                    msg::dictionary_actual().to_string(),
                    ControlKind::Edit {
                        text: self.entry.actual.clone(),
                    },
                    true,
                ),
                control(
                    "replacement",
                    msg::dictionary_replacement().to_string(),
                    ControlKind::Edit {
                        text: self.entry.replacement.clone(),
                    },
                    true,
                ),
                control(
                    "case",
                    msg::dictionary_case_sensitive().to_string(),
                    ControlKind::CheckBox {
                        checked: self.entry.case_sensitive,
                    },
                    true,
                ),
                button("ok", msg::dictionary_ok().to_string(), true, true),
                button("cancel", msg::dictionary_cancel().to_string(), false, true),
            ],
            keys: Vec::new(),
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> RuleOutcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("actual", Value::Text(text)) => self.entry.actual.clone_from(text),
                    ("replacement", Value::Text(text)) => self.entry.replacement.clone_from(text),
                    ("case", Value::Checked(checked)) => self.entry.case_sensitive = *checked,
                    _ => {}
                }
                RuleOutcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                // A rule needs its word: OK without one does nothing.
                "ok" if !self.entry.actual.trim().is_empty() => {
                    RuleOutcome::Done(DictionaryEntry {
                        actual: self.entry.actual.trim().to_owned(),
                        // A word and its spaces, but not a line's end an edit may give with it.
                        replacement: self.entry.replacement.trim().to_owned(),
                        ..self.entry.clone()
                    })
                }
                "cancel" => RuleOutcome::Closed,
                _ => RuleOutcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => RuleOutcome::Closed,
            _ => RuleOutcome::Nothing,
        }
    }
}

/// What the engine does after a message from the question before Delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    Nothing,
    Yes,
    No,
}

/// The question before a rule is deleted.
#[derive(Debug, Clone)]
pub struct DeleteDialog {
    pub id: u32,
    entry: String,
}

impl DeleteDialog {
    pub fn new(id: u32, entry: &DictionaryEntry) -> Self {
        Self {
            id,
            entry: entry_text(entry),
        }
    }

    pub fn spec(&self) -> DialogSpec {
        DialogSpec {
            id: self.id,
            title: msg::dictionary_delete_title().to_string(),
            controls: vec![
                control(
                    "question",
                    String::new(),
                    ControlKind::Message {
                        text: msg::dictionary_delete_question(self.entry.as_str()).to_string(),
                    },
                    true,
                ),
                button("yes", msg::dictionary_delete_yes().to_string(), true, true),
                // Escape answers no.
                button(
                    "cancel",
                    msg::dictionary_delete_no().to_string(),
                    false,
                    true,
                ),
            ],
            keys: Vec::new(),
        }
    }

    pub fn handle(&self, event: &FromUi) -> DeleteOutcome {
        match event {
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "yes" => DeleteOutcome::Yes,
                "cancel" => DeleteOutcome::No,
                _ => DeleteOutcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => DeleteOutcome::No,
            _ => DeleteOutcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(spec: &DialogSpec) -> Vec<(&str, &str, bool)> {
        spec.controls
            .iter()
            .map(|c| (c.id.as_str(), c.label.as_str(), c.enabled))
            .collect()
    }

    fn pressed(id: u32, control: &str) -> FromUi {
        FromUi::DialogPressed {
            id,
            control: control.to_owned(),
        }
    }

    /// JAWS's manager: the tree, then Change... (once a rule is chosen), Delete and Add...
    /// [obs, 1154/s6-s8, s28, s29]; a rule is "total=sum" [obs, 1154/s27].
    #[test]
    fn the_manager_is_jaws_dictionary_manager() {
        let rules = [DictionaryEntry::new("total", "sum")];
        let mut dialog = ManagerDialog::new(3, "default", &rules);
        let spec = dialog.spec();
        assert_eq!(spec.title, "default - Okur Dictionary Manager");
        assert_eq!(
            labels(&spec),
            [
                ("entries", "Dictionary Entries", true),
                ("change", "&Change...", false),
                ("delete", "&Delete", true),
                ("add", "&Add...", true),
            ]
        );
        let items = spec.controls.first().map(|c| c.kind.clone());
        assert!(matches!(items, Some(ControlKind::Tree { items, .. })
            if items.iter().map(|i| i.text.as_str()).eq(["total=sum"])));
        assert_eq!(
            dialog.handle(&pressed(3, "change")),
            ManagerOutcome::Nothing
        );
        let chosen = FromUi::DialogChanged {
            id: 3,
            control: "entries".to_owned(),
            value: Value::Selected(0),
        };
        assert_eq!(dialog.handle(&chosen), ManagerOutcome::Chosen(0));
        assert_eq!(
            dialog.handle(&pressed(3, "change")),
            ManagerOutcome::Change(0)
        );
        assert_eq!(
            dialog.handle(&pressed(3, "delete")),
            ManagerOutcome::Delete(0)
        );
        assert_eq!(dialog.handle(&pressed(3, "add")), ManagerOutcome::Add);
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 3 }),
            ManagerOutcome::Closed
        );
    }

    /// Add proposes the word at the cursor; OK keeps the rule as typed, and needs a word.
    #[test]
    fn add_keeps_the_rule_as_typed() {
        let mut dialog = RuleDialog::add(5, "total");
        let spec = dialog.spec();
        assert_eq!(spec.title, "Add Dictionary Definition");
        assert_eq!(
            labels(&spec),
            [
                ("actual", "&Actual Word:", true),
                ("replacement", "&Replacement Word:", true),
                ("case", "Case &Sensitive", true),
                ("ok", "OK", true),
                ("cancel", "Cancel", true),
            ]
        );
        let typed = FromUi::DialogChanged {
            id: 5,
            control: "replacement".to_owned(),
            value: Value::Text("sum".to_owned()),
        };
        assert_eq!(dialog.handle(&typed), RuleOutcome::Nothing);
        assert_eq!(
            dialog.handle(&pressed(5, "ok")),
            RuleOutcome::Done(DictionaryEntry::new("total", "sum"))
        );
        let mut empty = RuleDialog::add(6, " ");
        assert_eq!(empty.handle(&pressed(6, "ok")), RuleOutcome::Nothing);
        assert_eq!(empty.handle(&pressed(6, "cancel")), RuleOutcome::Closed);
    }

    /// Delete asks with the rule's text [obs, 1154/s30]; Escape answers no.
    #[test]
    fn delete_asks_first() {
        let dialog = DeleteDialog::new(7, &DictionaryEntry::new("total", "sum"));
        let spec = dialog.spec();
        assert_eq!(spec.title, "Confirm Dictionary Entries Delete");
        let question = spec.controls.first().map(|c| c.kind.clone());
        assert_eq!(
            question,
            Some(ControlKind::Message {
                text: "Are you sure you want to delete the dictionary entry total=sum?".to_owned()
            })
        );
        assert_eq!(dialog.handle(&pressed(7, "yes")), DeleteOutcome::Yes);
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 7 }),
            DeleteOutcome::No
        );
    }

    /// The question in an application without a dictionary: OK takes the application's,
    /// Escape the default one [obs, 1154/s4, s5].
    #[test]
    fn a_new_dictionary_or_the_default_one() {
        let dialog = NewDialog::new(2, "notepad");
        assert_eq!(dialog.spec().title, "New Dictionary");
        assert_eq!(dialog.handle(&pressed(2, "ok")), NewOutcome::Application);
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 2 }),
            NewOutcome::Default
        );
    }
}
