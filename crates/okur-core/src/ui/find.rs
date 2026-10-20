//! JAWS Find (Ctrl+F in a page, Insert+Ctrl+F): a dialog that searches the virtual buffer from
//! the virtual cursor. Its controls are JAWS 2025's, in its order
//! (`docs/references/jaws-dialogs-and-visual-features.md`, with its picture): Find what, an
//! editable combo box holding the last 20 searches; Context Specific History and Maintain
//! History of Recent Finds; Direction, Forward or Reverse; Ignore Case; and the buttons Find
//! (the default), Delete, Clear Find History and Cancel.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use crate::messages::{msg, StaticMessage};

/// How many earlier searches the history keeps.
pub const HISTORY: usize = 20;

/// The dialog's check boxes and direction, kept from one search to the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindOptions {
    /// The history is kept per site.
    pub context_specific: bool,
    /// Searches are added to the history.
    pub maintain_history: bool,
    pub forward: bool,
    pub ignore_case: bool,
}

impl Default for FindOptions {
    /// JAWS's: every check box checked, searching forward.
    fn default() -> Self {
        Self {
            context_specific: true,
            maintain_history: true,
            forward: true,
            ignore_case: true,
        }
    }
}

/// A search the user asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Search {
    pub text: String,
    pub forward: bool,
    pub ignore_case: bool,
}

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Messages for the dialog.
    Send(Vec<ToUi>),
    /// Find was pressed: the dialog closes and the search runs.
    Search(Search),
    /// The history changed (Delete, Clear Find History).
    History,
    /// The dialog was cancelled.
    Closed,
}

/// JAWS Find while it is open.
#[derive(Debug, Clone)]
pub struct FindDialog {
    pub id: u32,
    text: String,
    history: Vec<String>,
    options: FindOptions,
}

impl FindDialog {
    /// The dialog with the last search in its field.
    pub fn new(id: u32, history: &[String], options: FindOptions) -> Self {
        Self {
            id,
            text: history.first().cloned().unwrap_or_default(),
            history: history.to_vec(),
            options,
        }
    }

    pub fn options(&self) -> FindOptions {
        self.options
    }

    /// The searches remembered, the latest first.
    pub fn history(&self) -> &[String] {
        &self.history
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: StaticMessage, kind: ControlKind| Control {
            id: id.to_owned(),
            label: label.to_string(),
            kind,
            enabled: true,
        };
        let check = |id: &str, label: StaticMessage, checked: bool| {
            control(id, label, ControlKind::CheckBox { checked })
        };
        let button = |id: &str, label: StaticMessage, default: bool, enabled: bool| Control {
            enabled,
            ..control(id, label, ControlKind::Button { default })
        };
        let remembered = !self.history.is_empty();
        DialogSpec {
            id: self.id,
            title: msg::find_dialog().to_string(),
            controls: vec![
                control(
                    "what",
                    msg::find_what(),
                    ControlKind::ComboEdit {
                        text: self.text.clone(),
                        history: self.history.clone(),
                    },
                ),
                check(
                    "context",
                    msg::find_context_history(),
                    self.options.context_specific,
                ),
                check(
                    "maintain",
                    msg::find_maintain_history(),
                    self.options.maintain_history,
                ),
                control(
                    "direction",
                    msg::find_direction(),
                    ControlKind::Radios {
                        options: vec![
                            msg::find_forward().to_string(),
                            msg::find_reverse().to_string(),
                        ],
                        selected: usize::from(!self.options.forward),
                    },
                ),
                check("case", msg::find_ignore_case(), self.options.ignore_case),
                button("find", msg::find_find(), true, true),
                button("delete", msg::find_delete(), false, remembered),
                button("clear", msg::find_clear_history(), false, remembered),
                button("cancel", msg::find_cancel(), false, true),
            ],
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("what", Value::Text(text)) => self.text = text.clone(),
                    ("context", Value::Checked(on)) => self.options.context_specific = *on,
                    ("maintain", Value::Checked(on)) => self.options.maintain_history = *on,
                    ("direction", Value::Selected(i)) => self.options.forward = *i == 0,
                    ("case", Value::Checked(on)) => self.options.ignore_case = *on,
                    _ => {}
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "find" if !self.text.trim().is_empty() => {
                    if self.options.maintain_history {
                        self.remember(self.text.clone());
                    }
                    Outcome::Search(Search {
                        text: self.text.clone(),
                        forward: self.options.forward,
                        ignore_case: self.options.ignore_case,
                    })
                }
                "delete" => {
                    let before = self.history.len();
                    self.history.retain(|earlier| *earlier != self.text);
                    self.refill(before)
                }
                "clear" => {
                    let before = self.history.len();
                    self.history.clear();
                    self.refill(before)
                }
                _ => Outcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }

    /// `text` first in the history, once, the history no longer than [`HISTORY`].
    fn remember(&mut self, text: String) {
        self.history.retain(|earlier| *earlier != text);
        self.history.insert(0, text);
        self.history.truncate(HISTORY);
    }

    /// After Delete or Clear Find History: the dialog shows the history left.
    fn refill(&mut self, before: usize) -> Outcome {
        if self.history.len() == before {
            return Outcome::Nothing;
        }
        self.text = self.history.first().cloned().unwrap_or_default();
        Outcome::History
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pressed(dialog: &mut FindDialog, control: &str) -> Outcome {
        dialog.handle(&FromUi::DialogPressed {
            id: dialog.id,
            control: control.to_owned(),
        })
    }

    fn typed(dialog: &mut FindDialog, text: &str) {
        let _ = dialog.handle(&FromUi::DialogChanged {
            id: dialog.id,
            control: "what".to_owned(),
            value: Value::Text(text.to_owned()),
        });
    }

    #[test]
    fn the_dialog_is_jaws_finds() {
        let spec = FindDialog::new(3, &[], FindOptions::default()).spec();
        assert_eq!(spec.title, "Okur Find");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Find &what:",
                "Conte&xt Specific History",
                "&Maintain History of Recent Finds",
                "Direction",
                "&Ignore Case",
                "Find",
                "Delete",
                "Clear Find History",
                "Cancel"
            ]
        );
        // Delete and Clear Find History have nothing to act on yet.
        let enabled: Vec<bool> = spec.controls.iter().map(|c| c.enabled).collect();
        assert_eq!(
            enabled,
            [true, true, true, true, true, true, false, false, true]
        );
    }

    #[test]
    fn find_searches_and_remembers_the_text_first() {
        let history = vec!["board".to_owned(), "library".to_owned()];
        let mut dialog = FindDialog::new(1, &history, FindOptions::default());
        // The field holds the last search.
        assert!(matches!(
            dialog.spec().controls.first().map(|c| &c.kind),
            Some(ControlKind::ComboEdit { text, .. }) if text == "board"
        ));
        typed(&mut dialog, "library");
        assert_eq!(
            pressed(&mut dialog, "find"),
            Outcome::Search(Search {
                text: "library".to_owned(),
                forward: true,
                ignore_case: true,
            })
        );
        assert_eq!(dialog.history(), ["library", "board"]);
        // Nothing to search for: Find does nothing.
        typed(&mut dialog, " ");
        assert_eq!(pressed(&mut dialog, "find"), Outcome::Nothing);
    }

    #[test]
    fn delete_and_clear_change_the_history() {
        let history = vec!["board".to_owned(), "library".to_owned()];
        let mut dialog = FindDialog::new(1, &history, FindOptions::default());
        assert_eq!(pressed(&mut dialog, "delete"), Outcome::History);
        assert_eq!(dialog.history(), ["library"]);
        assert_eq!(pressed(&mut dialog, "clear"), Outcome::History);
        assert!(dialog.history().is_empty());
        assert_eq!(pressed(&mut dialog, "clear"), Outcome::Nothing);
    }

    #[test]
    fn options_change_with_the_controls() {
        let mut dialog = FindDialog::new(1, &[], FindOptions::default());
        let _ = dialog.handle(&FromUi::DialogChanged {
            id: 1,
            control: "direction".to_owned(),
            value: Value::Selected(1),
        });
        let _ = dialog.handle(&FromUi::DialogChanged {
            id: 1,
            control: "case".to_owned(),
            value: Value::Checked(false),
        });
        assert!(!dialog.options().forward);
        assert!(!dialog.options().ignore_case);
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 1 }),
            Outcome::Closed
        );
    }
}
