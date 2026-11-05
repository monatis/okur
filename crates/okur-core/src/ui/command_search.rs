//! Command Search (Insert+Space, then J): Okur's commands found by name as the user types, as
//! JAWS's Search for JAWS Commands [obs, 628/s6-s11]: "Search for JAWS Commands dialog | Search
//! for: Edit"; each letter typed says the first command found, and only when it changes ("Toggle
//! Video in Tandem Session", "Say Font", then nothing for "say"); Down reaches the results, each
//! a command's name and its keys ("Say Font. Insert + F"); Escape closes it. Okur searches its
//! own commands, so what it finds is Okur's; Enter on one runs it.

use okur_ui_model::{Control, ControlKind, DialogShortcut, DialogSpec, FromUi, Value};

use crate::command::Command;
use crate::messages::msg;

/// A command as Command Search lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub command: Command,
    /// Its name, its identifier's words ("Say Font").
    pub name: String,
    /// Its keys in JAWS's notation ("Insert + F").
    pub keys: String,
}

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// The query changed: the results' lines, and the first found, when it is another than before.
    Found {
        items: Vec<String>,
        first: Option<String>,
    },
    /// Enter on a result: the dialog closes and the command runs.
    Run(Command),
    Closed,
}

#[derive(Debug, Clone)]
pub struct CommandSearch {
    pub id: u32,
    entries: Vec<Entry>,
    found: Vec<usize>,
    chosen: usize,
    first: Option<String>,
}

impl CommandSearch {
    pub fn new(id: u32, entries: Vec<Entry>) -> Self {
        let found = (0..entries.len()).collect();
        Self {
            id,
            entries,
            found,
            chosen: 0,
            first: None,
        }
    }

    fn line(entry: &Entry) -> String {
        msg::command_search_entry(&entry.name, &entry.keys).to_string()
    }

    fn items(&self) -> Vec<String> {
        self.found
            .iter()
            .filter_map(|i| self.entries.get(*i))
            .map(Self::line)
            .collect()
    }

    pub fn spec(&self) -> DialogSpec {
        DialogSpec {
            id: self.id,
            title: msg::command_search_title().to_string(),
            controls: vec![
                Control {
                    id: "query".to_owned(),
                    label: msg::command_search_for().to_string(),
                    kind: ControlKind::Edit {
                        text: String::new(),
                    },
                    enabled: true,
                    hidden: false,
                },
                Control {
                    id: "results".to_owned(),
                    label: msg::command_search_results().to_string(),
                    kind: ControlKind::List {
                        items: self.items(),
                        selected: 0,
                    },
                    enabled: true,
                    hidden: false,
                },
            ],
            // Down from the search box reaches the results, as JAWS's does [obs, 628/s10].
            keys: vec![DialogShortcut {
                key: "Down".to_owned(),
                control: "results".to_owned(),
                from: Some("query".to_owned()),
            }],
        }
    }

    /// The entries whose name holds every word of `query`, whatever their case.
    fn find(&self, query: &str) -> Vec<usize> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                let name = e.name.to_lowercase();
                words.iter().all(|w| name.contains(w.as_str()))
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("query", Value::Text(query)) => {
                        self.found = self.find(query);
                        self.chosen = 0;
                        let first = self
                            .found
                            .first()
                            .and_then(|i| self.entries.get(*i))
                            .map(|e| e.name.clone());
                        let changed = first != self.first;
                        self.first.clone_from(&first);
                        Outcome::Found {
                            items: self.items(),
                            first: first.filter(|_| changed),
                        }
                    }
                    ("results", Value::Selected(row)) => {
                        self.chosen = *row;
                        Outcome::Nothing
                    }
                    _ => Outcome::Nothing,
                }
            }
            FromUi::DialogPressed { id, control } if *id == self.id && control == "results" => self
                .found
                .get(self.chosen)
                .and_then(|i| self.entries.get(*i))
                .map_or(Outcome::Nothing, |e| Outcome::Run(e.command.clone())),
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<Entry> {
        let entry = |command: Command, name: &str, keys: &str| Entry {
            command,
            name: name.to_owned(),
            keys: keys.to_owned(),
        };
        vec![
            entry(Command::SayAll, "Say All", "Insert + Down Arrow"),
            entry(Command::SayFont, "Say Font", "Insert + F"),
            entry(
                Command::DictionaryManager,
                "Dictionary Manager",
                "Insert + D",
            ),
        ]
    }

    fn typed(id: u32, text: &str) -> FromUi {
        FromUi::DialogChanged {
            id,
            control: "query".to_owned(),
            value: Value::Text(text.to_owned()),
        }
    }

    /// Typing says the first command found, only when it changes; Enter on the result runs
    /// it [obs, 628/s7-s10].
    #[test]
    fn typing_finds_and_enter_runs() {
        let mut search = CommandSearch::new(4, entries());
        let spec = search.spec();
        assert_eq!(spec.title, "Search for Okur Commands");
        assert_eq!(
            search.handle(&typed(4, "s")),
            Outcome::Found {
                items: vec![
                    "Say All. Insert + Down Arrow".to_owned(),
                    "Say Font. Insert + F".to_owned()
                ],
                first: Some("Say All".to_owned())
            }
        );
        assert_eq!(
            search.handle(&typed(4, "say f")),
            Outcome::Found {
                items: vec!["Say Font. Insert + F".to_owned()],
                first: Some("Say Font".to_owned())
            }
        );
        assert_eq!(
            search.handle(&typed(4, "say fo")),
            Outcome::Found {
                items: vec!["Say Font. Insert + F".to_owned()],
                first: None
            }
        );
        let enter = FromUi::DialogPressed {
            id: 4,
            control: "results".to_owned(),
        };
        assert_eq!(search.handle(&enter), Outcome::Run(Command::SayFont));
        assert_eq!(
            search.handle(&FromUi::DialogClosed { id: 4 }),
            Outcome::Closed
        );
    }
}
