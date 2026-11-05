//! Command Search in the engine (`crate::ui::command_search`): Insert+Space, J opens it over
//! Okur's commands; each change of the query lists what it finds and says the first command
//! found when it changes; Enter on a result closes the dialog and runs the command.

use super::ScreenReaderEngine;
use crate::command::Command;
use crate::ui::command_search::{CommandSearch, Entry, Outcome};
use crate::ui::{FromUi, ToUi};

/// A command's name: its identifier's words ("SayFont" is "Say Font"), as JAWS's Command Search
/// names its scripts ("Say Font. Insert + F" [obs, 628/s10]). A command with a value of its own
/// (a quick key's element, a find's text) has none.
fn name_of(command: &Command) -> Option<String> {
    let id = format!("{command:?}");
    if !id.chars().all(char::is_alphanumeric) {
        return None;
    }
    let mut words = String::with_capacity(id.len() + 8);
    for (i, c) in id.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            words.push(' ');
        }
        words.push(c);
    }
    Some(words)
}

impl ScreenReaderEngine {
    /// Insert+Space, J: "Search for Okur Commands dialog | Search for: Edit" [obs, 628/s6].
    pub(super) fn open_command_search(&mut self) {
        let entries = crate::command::bound_commands()
            .into_iter()
            .filter_map(|(command, keys)| {
                Some(Entry {
                    name: name_of(&command)?,
                    keys: crate::keyboard_help::key_name(&keys).replace('+', " + "),
                    command,
                })
            })
            .collect();
        self.dialog_count += 1;
        let search = CommandSearch::new(self.dialog_count, entries);
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogShow {
                dialog: search.spec(),
            });
        }
        self.command_search = Some(search);
    }

    pub(super) fn is_command_search_event(&self, message: &FromUi) -> bool {
        self.command_search
            .as_ref()
            .is_some_and(|search| Some(search.id) == super::dialog_of(message))
    }

    pub(super) fn handle_command_search_event(&mut self, message: &FromUi) -> Option<String> {
        let mut search = self.command_search.take()?;
        match search.handle(message) {
            Outcome::Nothing => {
                self.command_search = Some(search);
                None
            }
            Outcome::Found { items, first } => {
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogItems {
                        id: search.id,
                        control: "results".to_owned(),
                        items,
                        selected: 0,
                    });
                }
                self.command_search = Some(search);
                let first = first?;
                self.speech.speak(&first, true);
                Some(first)
            }
            Outcome::Run(command) => {
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogClose { id: search.id });
                }
                self.execute(command)
            }
            Outcome::Closed => None,
        }
    }
}
