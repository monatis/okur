//! The PlaceMarker List (Ctrl+Shift+K in a page) and the dialogs it opens. Its controls are
//! JAWS 2025's [obs, 699, 702; picture `docs/references/jaws-dialogs/placemarkers.png`]:
//! - the list "PlaceMarker", two columns, the PlaceMarker and "Generalized to Domain"
//!   ("Document" or "Domain"), none chosen as it opens ("Not Selected Water in a closed bottle
//!   ... | Document | 1 of 1" [obs, 702/s18]);
//! - Add... (the default until a PlaceMarker is chosen, then MoveTo), MoveTo, Change... and
//!   Remove (available once one is chosen), Remove All (with any), Cancel: with one
//!   PlaceMarker not chosen, Tab goes from the list to Add..., Remove All, Cancel [obs,
//!   702/s19-s21];
//! - Display PlaceMarkers For: URL, Domain, All (All).
//!
//! Add proposes the text at the virtual cursor as the name ("Add PlaceMarker dialog | Name:
//! Edit | Water in a closed bottle is allowed; meals are not." [obs, 702/s5]); OK keeps it and
//! closes the list too [obs, 702/s6]. Remove All asks first ("PlaceMarker RemoveAll Warning
//! dialog | Are you sure you want to remove all PlaceMarkers? | Yes Button" [obs, 702/s27]);
//! Yes closes the list too [obs, 702/s28].

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value};

use crate::messages::msg;

fn control(id: &str, label: String, kind: ControlKind, enabled: bool) -> Control {
    Control {
        id: id.to_owned(),
        label,
        kind,
        enabled,
    }
}

fn button(id: &str, label: String, default: bool, enabled: bool) -> Control {
    control(id, label, ControlKind::Button { default }, enabled)
}

/// A row of the list: a PlaceMarker's name and whether it is generalized to its site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub domain: bool,
}

/// What the engine does after a message from the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListOutcome {
    Nothing,
    /// A PlaceMarker was chosen: MoveTo, Change... and Remove become available, MoveTo the
    /// default.
    Chosen(usize),
    Add,
    MoveTo(usize),
    Change(usize),
    Remove(usize),
    RemoveAll,
    Closed,
}

/// The PlaceMarker List while it is open.
#[derive(Debug, Clone)]
pub struct ListDialog {
    pub id: u32,
    rows: Vec<Row>,
    chosen: Option<usize>,
}

/// The buttons a chosen PlaceMarker makes available.
pub const FOR_CHOSEN: [&str; 3] = ["moveto", "change", "remove"];

impl ListDialog {
    pub fn new(id: u32, rows: Vec<Row>) -> Self {
        Self {
            id,
            rows,
            chosen: None,
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let rows = self
            .rows
            .iter()
            .map(|row| {
                let scope = if row.domain {
                    msg::placemarker_scope_domain()
                } else {
                    msg::placemarker_scope_document()
                };
                vec![row.name.clone(), scope.to_string()]
            })
            .collect();
        let any = !self.rows.is_empty();
        DialogSpec {
            id: self.id,
            title: msg::placemarker_list_title().to_string(),
            controls: vec![
                control(
                    "markers",
                    msg::placemarker_list_markers().to_string(),
                    ControlKind::Table {
                        columns: vec![
                            msg::placemarker_list_markers().to_string(),
                            msg::placemarker_list_column_domain().to_string(),
                        ],
                        rows,
                        selected: None,
                    },
                    true,
                ),
                button("add", msg::placemarker_list_add().to_string(), true, true),
                button(
                    "moveto",
                    msg::placemarker_list_move_to().to_string(),
                    false,
                    false,
                ),
                button(
                    "change",
                    msg::placemarker_list_change().to_string(),
                    false,
                    false,
                ),
                button(
                    "remove",
                    msg::placemarker_list_remove().to_string(),
                    false,
                    false,
                ),
                button(
                    "removeall",
                    msg::placemarker_list_remove_all().to_string(),
                    false,
                    any,
                ),
                button(
                    "cancel",
                    msg::placemarker_list_cancel().to_string(),
                    false,
                    true,
                ),
                control(
                    "display",
                    msg::placemarker_list_display().to_string(),
                    ControlKind::Radios {
                        options: vec![
                            msg::placemarker_list_display_url().to_string(),
                            msg::placemarker_list_display_domain().to_string(),
                            msg::placemarker_list_display_all().to_string(),
                        ],
                        selected: 2,
                    },
                    true,
                ),
            ],
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> ListOutcome {
        match event {
            FromUi::DialogChanged {
                id,
                control,
                value: Value::Selected(row),
            } if *id == self.id && control == "markers" && *row < self.rows.len() => {
                let first = self.chosen.is_none();
                self.chosen = Some(*row);
                if first {
                    ListOutcome::Chosen(*row)
                } else {
                    ListOutcome::Nothing
                }
            }
            FromUi::DialogPressed { id, control } if *id == self.id => {
                match (control.as_str(), self.chosen) {
                    ("add", _) => ListOutcome::Add,
                    ("moveto", Some(row)) => ListOutcome::MoveTo(row),
                    ("change", Some(row)) => ListOutcome::Change(row),
                    ("remove", Some(row)) => ListOutcome::Remove(row),
                    ("removeall", _) if !self.rows.is_empty() => ListOutcome::RemoveAll,
                    ("cancel", _) => ListOutcome::Closed,
                    _ => ListOutcome::Nothing,
                }
            }
            FromUi::DialogClosed { id } if *id == self.id => ListOutcome::Closed,
            _ => ListOutcome::Nothing,
        }
    }
}

/// What the engine does after a message from Add or Change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameOutcome {
    Nothing,
    /// OK, with the name typed.
    Name(String),
    Closed,
}

/// Add PlaceMarker, or Change PlaceMarker for one already kept: its name.
#[derive(Debug, Clone)]
pub struct NameDialog {
    pub id: u32,
    change: bool,
    name: String,
}

impl NameDialog {
    /// Add, proposing `name` (the text at the cursor).
    pub fn add(id: u32, name: &str) -> Self {
        Self {
            id,
            change: false,
            name: name.to_owned(),
        }
    }

    /// Change, for a PlaceMarker named `name`.
    pub fn change(id: u32, name: &str) -> Self {
        Self {
            id,
            change: true,
            name: name.to_owned(),
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let title = if self.change {
            msg::placemarker_change_title()
        } else {
            msg::placemarker_add_title()
        };
        DialogSpec {
            id: self.id,
            title: title.to_string(),
            controls: vec![
                control(
                    "name",
                    msg::placemarker_add_name().to_string(),
                    ControlKind::Edit {
                        text: self.name.clone(),
                    },
                    true,
                ),
                button("ok", msg::placemarker_add_ok().to_string(), true, true),
                button(
                    "cancel",
                    msg::placemarker_add_cancel().to_string(),
                    false,
                    true,
                ),
            ],
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> NameOutcome {
        match event {
            FromUi::DialogChanged {
                id,
                control,
                value: Value::Text(text),
            } if *id == self.id && control == "name" => {
                self.name.clone_from(text);
                NameOutcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                // A PlaceMarker needs a name: OK with none does nothing.
                "ok" if !self.name.trim().is_empty() => {
                    NameOutcome::Name(self.name.trim().to_owned())
                }
                "cancel" => NameOutcome::Closed,
                _ => NameOutcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => NameOutcome::Closed,
            _ => NameOutcome::Nothing,
        }
    }
}

/// What the engine does after a message from the Remove All question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveAllOutcome {
    Nothing,
    Yes,
    No,
}

/// The question before Remove All.
#[derive(Debug, Clone)]
pub struct RemoveAllDialog {
    pub id: u32,
}

impl RemoveAllDialog {
    pub fn spec(&self) -> DialogSpec {
        DialogSpec {
            id: self.id,
            title: msg::placemarker_remove_all_title().to_string(),
            controls: vec![
                control(
                    "question",
                    String::new(),
                    ControlKind::Message {
                        text: msg::placemarker_remove_all_question().to_string(),
                    },
                    true,
                ),
                button(
                    "yes",
                    msg::placemarker_remove_all_yes().to_string(),
                    true,
                    true,
                ),
                // Escape answers no.
                button(
                    "cancel",
                    msg::placemarker_remove_all_no().to_string(),
                    false,
                    true,
                ),
            ],
        }
    }

    pub fn handle(&self, event: &FromUi) -> RemoveAllOutcome {
        match event {
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "yes" => RemoveAllOutcome::Yes,
                "cancel" => RemoveAllOutcome::No,
                _ => RemoveAllOutcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => RemoveAllOutcome::No,
            _ => RemoveAllOutcome::Nothing,
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

    /// JAWS's PlaceMarker List: with one PlaceMarker none is chosen, so only Add..., Remove
    /// All and Cancel are buttons Tab reaches [obs, 702/s18-s22].
    #[test]
    fn the_list_is_jaws_placemarker_list() {
        let dialog = ListDialog::new(
            4,
            vec![Row {
                name: "Water in a closed bottle".to_owned(),
                domain: false,
            }],
        );
        let spec = dialog.spec();
        assert_eq!(spec.title, "PlaceMarker List");
        assert_eq!(
            labels(&spec),
            [
                ("markers", "PlaceMarker", true),
                ("add", "&Add...", true),
                ("moveto", "&MoveTo", false),
                ("change", "C&hange...", false),
                ("remove", "&Remove", false),
                ("removeall", "Remo&ve All", true),
                ("cancel", "&Cancel", true),
                ("display", "Display PlaceMarkers For:", true),
            ]
        );
        let rows = spec.controls.first().map(|c| c.kind.clone());
        assert_eq!(
            rows,
            Some(ControlKind::Table {
                columns: vec!["PlaceMarker".to_owned(), "Generalized to Domain".to_owned()],
                rows: vec![vec![
                    "Water in a closed bottle".to_owned(),
                    "Document".to_owned()
                ]],
                selected: None,
            })
        );
        // Without PlaceMarkers there is nothing to remove [obs, 699/s3-s5].
        let empty = ListDialog::new(5, Vec::new()).spec();
        assert!(empty
            .controls
            .iter()
            .any(|c| c.id == "removeall" && !c.enabled));
    }

    /// MoveTo and the others act on the PlaceMarker chosen; before one is, only Add, Remove
    /// All and Cancel do anything.
    #[test]
    fn a_chosen_placemarker_is_moved_to_changed_or_removed() {
        let rows = vec![
            Row {
                name: "One".to_owned(),
                domain: false,
            },
            Row {
                name: "Two".to_owned(),
                domain: true,
            },
        ];
        let mut dialog = ListDialog::new(9, rows);
        let press = |dialog: &mut ListDialog, control: &str| {
            dialog.handle(&FromUi::DialogPressed {
                id: 9,
                control: control.to_owned(),
            })
        };
        assert_eq!(press(&mut dialog, "moveto"), ListOutcome::Nothing);
        assert_eq!(press(&mut dialog, "add"), ListOutcome::Add);
        let choose = |dialog: &mut ListDialog, row: usize| {
            dialog.handle(&FromUi::DialogChanged {
                id: 9,
                control: "markers".to_owned(),
                value: Value::Selected(row),
            })
        };
        assert_eq!(choose(&mut dialog, 1), ListOutcome::Chosen(1));
        assert_eq!(choose(&mut dialog, 0), ListOutcome::Nothing);
        assert_eq!(press(&mut dialog, "moveto"), ListOutcome::MoveTo(0));
        assert_eq!(press(&mut dialog, "change"), ListOutcome::Change(0));
        assert_eq!(press(&mut dialog, "remove"), ListOutcome::Remove(0));
        assert_eq!(press(&mut dialog, "removeall"), ListOutcome::RemoveAll);
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 9 }),
            ListOutcome::Closed
        );
    }

    /// Add proposes the text at the cursor; OK keeps the name typed, never an empty one.
    #[test]
    fn add_keeps_the_name_typed() {
        let mut dialog = NameDialog::add(3, "Water in a closed bottle");
        let spec = dialog.spec();
        assert_eq!(spec.title, "Add PlaceMarker");
        assert_eq!(
            spec.controls
                .first()
                .map(|c| (c.label.clone(), c.kind.clone())),
            Some((
                "&Name:".to_owned(),
                ControlKind::Edit {
                    text: "Water in a closed bottle".to_owned()
                }
            ))
        );
        let ok = FromUi::DialogPressed {
            id: 3,
            control: "ok".to_owned(),
        };
        let typed = |text: &str| FromUi::DialogChanged {
            id: 3,
            control: "name".to_owned(),
            value: Value::Text(text.to_owned()),
        };
        let _ = dialog.handle(&typed("  "));
        assert_eq!(dialog.handle(&ok), NameOutcome::Nothing);
        let _ = dialog.handle(&typed("Drinks "));
        assert_eq!(dialog.handle(&ok), NameOutcome::Name("Drinks".to_owned()));
        assert_eq!(
            NameDialog::change(3, "x").spec().title,
            "Change PlaceMarker"
        );
    }

    /// The Remove All question: Yes, the default, or No, which Escape answers [obs,
    /// 702/s27].
    #[test]
    fn remove_all_asks_first() {
        let dialog = RemoveAllDialog { id: 2 };
        let spec = dialog.spec();
        assert_eq!(spec.title, "PlaceMarker RemoveAll Warning");
        assert!(spec.controls.iter().any(|c| matches!(&c.kind,
            ControlKind::Message { text } if text == "Are you sure you want to remove all PlaceMarkers?")));
        let pressed = |control: &str| {
            dialog.handle(&FromUi::DialogPressed {
                id: 2,
                control: control.to_owned(),
            })
        };
        assert_eq!(pressed("yes"), RemoveAllOutcome::Yes);
        assert_eq!(pressed("cancel"), RemoveAllOutcome::No);
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 2 }),
            RemoveAllOutcome::No
        );
    }
}
