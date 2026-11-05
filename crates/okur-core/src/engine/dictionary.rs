//! The Dictionary Manager (Insert+D): the speech dictionary's rules ([`crate::dictionary`]),
//! for all applications or for the application in front, in Okur's dialogs
//! ([`crate::ui::dictionary`]).
//!
//! As JAWS's: Insert+D opens the application's own dictionary when it has one; an application
//! without one is asked about first, its name proposed, and Escape opens the default
//! dictionary, for all applications [obs, 1154/s4, s5; JAWS 2025 help, Dictionary Manager
//! Basics]. Add... proposes the word at the cursor [obs, 1154/s9]. The application's rules
//! apply before the default ones, and both apply.
//!
//! JAWS keeps the changes until File, Save (Ctrl+S); Okur keeps each rule as OK or Yes leaves
//! it, so nothing is lost when the manager closes.

use super::{AppMode, ScreenReaderEngine};
use crate::dictionary::DictionaryEntry;
use crate::messages::msg;
use crate::settings_layers::Scope;
use crate::ui::dictionary::{
    entry_text, DeleteDialog, DeleteOutcome, ManagerDialog, ManagerOutcome, NewDialog, NewOutcome,
    RuleDialog, RuleOutcome,
};
use crate::ui::{FromUi, ToUi, TreeItem};

/// The settings key the rules are kept under, in each layer.
const KEY: &str = crate::settings_layers::DICTIONARY;

/// A dialog the manager opened, or the question before it.
#[derive(Debug)]
enum Child {
    /// The question in an application without a dictionary of its own.
    New(NewDialog),
    /// Add (`None`) or Change for a rule.
    Rule(RuleDialog, Option<usize>),
    /// The question before a rule is deleted.
    Delete(DeleteDialog, usize),
}

/// The Dictionary Manager while it is open.
#[derive(Debug)]
pub(super) struct DictionaryState {
    manager: Option<ManagerDialog>,
    /// Whose rules: all applications', or the application's.
    scope: Scope,
    /// The application in front as Insert+D was pressed, by its settings id.
    application: Option<String>,
    entries: Vec<DictionaryEntry>,
    /// The word at the cursor as Insert+D was pressed, which Add proposes.
    word: String,
    child: Option<Child>,
}

impl ScreenReaderEngine {
    /// Insert+D: the Dictionary Manager, or first the question about an application's own
    /// dictionary. Okur says nothing itself: the dialog is read as it takes the focus.
    pub(super) fn open_dictionary_manager(&mut self) -> Option<String> {
        self.ui.as_ref()?;
        let application = self.settings_place.0.clone();
        tracing::debug!(?application, "dictionary manager asked for");
        let mut state = DictionaryState {
            manager: None,
            scope: Scope::Global,
            application: application.clone(),
            entries: Vec::new(),
            word: self.word_for_dictionary(),
            child: None,
        };
        match application {
            Some(app) if self.rules_of(&Scope::Application(app.clone())).is_some() => {
                self.open_manager(&mut state, Scope::Application(app));
            }
            Some(app) => {
                self.dialog_count += 1;
                let dialog = NewDialog::new(self.dialog_count, &app);
                self.show_dictionary(dialog.spec());
                state.child = Some(Child::New(dialog));
            }
            None => self.open_manager(&mut state, Scope::Global),
        }
        self.dictionary_manager = Some(state);
        // Words as written while it is open.
        self.configure_voice();
        None
    }

    /// The word Add proposes: at the virtual cursor in a page, else at the caret.
    fn word_for_dictionary(&self) -> String {
        if self.app_mode == AppMode::VirtualBuffer {
            return self.cursor.current_word().trim().to_owned();
        }
        self.last_text
            .as_ref()
            .map(crate::editing::word_around)
            .unwrap_or_default()
    }

    /// The rules kept for `scope`, or none where it has no dictionary of its own.
    fn rules_of(&self, scope: &Scope) -> Option<Vec<DictionaryEntry>> {
        let layer = match scope {
            Scope::Global => Some(&self.settings_layers.global),
            Scope::Application(id) => self.settings_layers.applications.get(id),
            Scope::Site(_) | Scope::Session => None,
        }?;
        serde_json::from_value(layer.get(KEY)?.clone()).ok()
    }

    /// Keeps `entries` as `scope`'s rules, saved, and in force at once. An application's
    /// dictionary is kept even with no rules, as JAWS's file is: Insert+D opens it next time.
    fn keep_rules(&mut self, scope: &Scope, entries: &[DictionaryEntry]) {
        let value = serde_json::to_value(entries).unwrap_or_default();
        match scope {
            Scope::Global if entries.is_empty() => {
                self.settings_layers.global.remove(KEY);
            }
            Scope::Global => {
                self.settings_layers.global.insert(KEY.to_owned(), value);
            }
            Scope::Application(id) => {
                self.settings_layers
                    .applications
                    .entry(id.clone())
                    .or_default()
                    .insert(KEY.to_owned(), value);
            }
            Scope::Site(_) | Scope::Session => return,
        }
        if let Some(store) = &self.settings_store {
            store.save(&self.settings_layers);
        }
        self.resolve_settings();
    }

    /// The manager on `scope`'s rules.
    fn open_manager(&mut self, state: &mut DictionaryState, scope: Scope) {
        let name = match &scope {
            Scope::Application(id) => id.clone(),
            _ => msg::dictionary_default_name().to_string(),
        };
        state.entries = self.rules_of(&scope).unwrap_or_default();
        state.scope = scope;
        self.dialog_count += 1;
        let dialog = ManagerDialog::new(self.dialog_count, &name, &state.entries);
        self.show_dictionary(dialog.spec());
        state.manager = Some(dialog);
    }

    fn show_dictionary(&self, dialog: crate::ui::DialogSpec) {
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogShow { dialog });
        }
    }

    fn close_dictionary_dialog(&self, id: u32) {
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogClose { id });
        }
    }

    /// Whether `message` is the manager's or one of its dialogs'.
    pub(super) fn is_dictionary_event(&self, message: &FromUi) -> bool {
        let Some(state) = &self.dictionary_manager else {
            return false;
        };
        let id = super::dialog_of(message);
        let child = state.child.as_ref().map(|child| match child {
            Child::New(dialog) => dialog.id,
            Child::Rule(dialog, _) => dialog.id,
            Child::Delete(dialog, _) => dialog.id,
        });
        id.is_some() && (id == state.manager.as_ref().map(|m| m.id) || id == child)
    }

    /// What the user did in the manager or a dialog it opened.
    pub(super) fn handle_dictionary_event(&mut self, message: &FromUi) -> Option<String> {
        let mut state = self.dictionary_manager.take()?;
        tracing::debug!(?message, rules = state.entries.len(), "dictionary event");
        let open = match state.child.take() {
            Some(child) => self.dictionary_child_event(&mut state, child, message),
            None => self.dictionary_manager_event(&mut state, message),
        };
        if open {
            self.dictionary_manager = Some(state);
        } else {
            // The dictionary in force again. The window it goes back to is said, as JAWS says
            // Notepad as its manager closes [obs, 1154/s16, s33].
            self.configure_voice();
        }
        None
    }

    /// The manager's own controls; whether it is still open.
    fn dictionary_manager_event(&mut self, state: &mut DictionaryState, message: &FromUi) -> bool {
        let Some(manager) = state.manager.as_mut() else {
            return false;
        };
        let id = manager.id;
        match manager.handle(message) {
            ManagerOutcome::Nothing => true,
            ManagerOutcome::Chosen(_) => {
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogEnable {
                        id,
                        control: "change".to_owned(),
                        enabled: true,
                    });
                }
                true
            }
            ManagerOutcome::Add => {
                self.dialog_count += 1;
                let dialog = RuleDialog::add(self.dialog_count, &state.word);
                self.show_dictionary(dialog.spec());
                state.child = Some(Child::Rule(dialog, None));
                true
            }
            ManagerOutcome::Change(row) => {
                let Some(entry) = state.entries.get(row) else {
                    return true;
                };
                self.dialog_count += 1;
                let dialog = RuleDialog::change(self.dialog_count, entry);
                self.show_dictionary(dialog.spec());
                state.child = Some(Child::Rule(dialog, Some(row)));
                true
            }
            ManagerOutcome::Delete(row) => {
                let Some(entry) = state.entries.get(row) else {
                    return true;
                };
                self.dialog_count += 1;
                let dialog = DeleteDialog::new(self.dialog_count, entry);
                self.show_dictionary(dialog.spec());
                state.child = Some(Child::Delete(dialog, row));
                true
            }
            ManagerOutcome::Closed => {
                self.close_dictionary_dialog(id);
                false
            }
        }
    }

    /// The question, Add or Change, and Delete's question; whether the manager is still open.
    fn dictionary_child_event(
        &mut self,
        state: &mut DictionaryState,
        mut child: Child,
        message: &FromUi,
    ) -> bool {
        match &mut child {
            Child::New(dialog) => match dialog.handle(message) {
                NewOutcome::Nothing => {
                    state.child = Some(child);
                    true
                }
                outcome => {
                    self.close_dictionary_dialog(dialog.id);
                    let scope = match (outcome, state.application.clone()) {
                        (NewOutcome::Application, Some(app)) => {
                            // The application's dictionary, made at once, empty.
                            self.keep_rules(&Scope::Application(app.clone()), &[]);
                            Scope::Application(app)
                        }
                        _ => Scope::Global,
                    };
                    self.open_manager(state, scope);
                    true
                }
            },
            Child::Rule(dialog, row) => match dialog.handle(message) {
                RuleOutcome::Nothing => {
                    state.child = Some(child);
                    true
                }
                RuleOutcome::Send(messages) => {
                    if let Some(ui) = &self.ui {
                        messages.into_iter().for_each(|m| ui.send(m));
                    }
                    state.child = Some(child);
                    true
                }
                RuleOutcome::Play(path) => {
                    if let Some(sounds) = self.sounds.as_mut() {
                        sounds.play_file(&path);
                    }
                    state.child = Some(child);
                    true
                }
                RuleOutcome::Closed => {
                    self.close_dictionary_dialog(dialog.id);
                    true
                }
                RuleOutcome::Done(entry) => {
                    self.close_dictionary_dialog(dialog.id);
                    match row.and_then(|row| state.entries.get_mut(row)) {
                        Some(kept) => *kept = entry,
                        None => state.entries.push(entry),
                    }
                    let scope = state.scope.clone();
                    self.keep_rules(&scope, &state.entries);
                    self.refresh_manager(state);
                    true
                }
            },
            Child::Delete(dialog, row) => match dialog.handle(message) {
                DeleteOutcome::Nothing => {
                    state.child = Some(child);
                    true
                }
                DeleteOutcome::No => {
                    self.close_dictionary_dialog(dialog.id);
                    true
                }
                DeleteOutcome::Yes => {
                    self.close_dictionary_dialog(dialog.id);
                    if *row < state.entries.len() {
                        state.entries.remove(*row);
                    }
                    let scope = state.scope.clone();
                    self.keep_rules(&scope, &state.entries);
                    self.refresh_manager(state);
                    true
                }
            },
        }
    }

    /// The manager's tree with the rules as they are now, and its buttons to match.
    fn refresh_manager(&mut self, state: &mut DictionaryState) {
        let Some(manager) = &state.manager else {
            return;
        };
        let id = manager.id;
        let name = match &state.scope {
            Scope::Application(app) => app.clone(),
            _ => msg::dictionary_default_name().to_string(),
        };
        state.manager = Some(ManagerDialog::new(id, &name, &state.entries));
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogTree {
                id,
                control: "entries".to_owned(),
                items: state
                    .entries
                    .iter()
                    .map(|entry| TreeItem {
                        text: entry_text(entry),
                        level: 0,
                        open: true,
                        checked: None,
                    })
                    .collect(),
                selected: 0,
            });
            ui.send(ToUi::DialogEnable {
                id,
                control: "change".to_owned(),
                enabled: !state.entries.is_empty(),
            });
            ui.send(ToUi::DialogEnable {
                id,
                control: "delete".to_owned(),
                enabled: !state.entries.is_empty(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::aom::{AomNode, Role};
    use crate::buffer::VirtualBuffer;
    use crate::cursor::VirtualCursor;
    use crate::dictionary::DictionaryEntry;
    use crate::engine::tests::SharedSpeech;
    use crate::engine::ScreenReaderEngine;
    use crate::event::AccessibilityEvent;
    use crate::input::{Key, KeyEvent, Modifiers};
    use crate::ui::{DialogSpec, FromUi, RecordingUiHost, ToUi, Value};

    fn engine() -> (ScreenReaderEngine, RecordingUiHost) {
        let page =
            AomNode::new(0, Role::Document).with_children(vec![
                AomNode::new(1, Role::Paragraph).with_text("let total = price")
            ]);
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&page)),
            Box::new(SharedSpeech::default()),
        );
        let ui = RecordingUiHost::default();
        engine.set_ui(Box::new(ui.clone()));
        engine.cursor.move_to_start();
        engine.execute_command("^{RIGHT}");
        (engine, ui)
    }

    fn insert_d(engine: &mut ScreenReaderEngine) {
        let insert = Modifiers {
            insert: true,
            ..Modifiers::none()
        };
        engine.handle_key_event(&KeyEvent::new(Key::Alpha('D'), insert, true));
    }

    fn shown(ui: &RecordingUiHost) -> Option<DialogSpec> {
        ui.take().into_iter().find_map(|m| match m {
            ToUi::DialogShow { dialog } => Some(dialog),
            _ => None,
        })
    }

    fn send(engine: &mut ScreenReaderEngine, message: FromUi) {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(message));
    }

    fn press(engine: &mut ScreenReaderEngine, id: u32, control: &str) {
        send(
            engine,
            FromUi::DialogPressed {
                id,
                control: control.to_owned(),
            },
        );
    }

    /// Insert+D with no application: the default dictionary's manager. Add proposes the word at
    /// the cursor, OK keeps the rule ("total" as "sum") in force at once; Delete, answered Yes,
    /// takes it away [obs, 1154/s5-s14, s27-s31].
    #[test]
    fn a_rule_is_added_and_deleted_in_the_manager() {
        let (mut engine, ui) = engine();
        insert_d(&mut engine);
        let manager = shown(&ui);
        assert_eq!(
            manager.as_ref().map(|d| d.title.as_str()),
            Some("default - Okur Dictionary Manager")
        );
        let manager = manager.map(|d| d.id).unwrap_or_default();
        press(&mut engine, manager, "add");
        let add = shown(&ui);
        let actual = add
            .as_ref()
            .and_then(|d| d.controls.first())
            .map(|c| c.kind.clone());
        assert_eq!(
            actual,
            Some(crate::ui::ControlKind::Edit {
                text: "total".to_owned()
            })
        );
        let add = add.map(|d| d.id).unwrap_or_default();
        send(
            &mut engine,
            FromUi::DialogChanged {
                id: add,
                control: "replacement".to_owned(),
                value: Value::Text("sum".to_owned()),
            },
        );
        press(&mut engine, add, "ok");
        assert_eq!(
            engine.settings.dictionary,
            [DictionaryEntry::new("total", "sum")]
        );
        let _ = ui.take();
        press(&mut engine, manager, "delete");
        let question = shown(&ui).map(|d| d.id).unwrap_or_default();
        press(&mut engine, question, "yes");
        assert!(engine.settings.dictionary.is_empty());
        send(&mut engine, FromUi::DialogClosed { id: manager });
        assert!(engine.dictionary_manager.is_none());
    }

    /// In an application without a dictionary Insert+D asks first, its name proposed: OK makes
    /// the application's own, whose rules go before the default ones; Escape is the default one
    /// [obs, 1154/s4, s5].
    #[test]
    fn an_application_gets_its_own_dictionary() {
        let (mut engine, ui) = engine();
        engine.settings_place(Some("notepad"), None);
        insert_d(&mut engine);
        let question = shown(&ui);
        assert_eq!(
            question.as_ref().map(|d| d.title.as_str()),
            Some("New Dictionary")
        );
        let question = question.map(|d| d.id).unwrap_or_default();
        send(&mut engine, FromUi::DialogClosed { id: question });
        let manager = shown(&ui);
        assert_eq!(
            manager.as_ref().map(|d| d.title.as_str()),
            Some("default - Okur Dictionary Manager")
        );
        send(
            &mut engine,
            FromUi::DialogClosed {
                id: manager.map(|d| d.id).unwrap_or_default(),
            },
        );
        insert_d(&mut engine);
        let question = shown(&ui).map(|d| d.id).unwrap_or_default();
        press(&mut engine, question, "ok");
        let manager = shown(&ui);
        assert_eq!(
            manager.as_ref().map(|d| d.title.as_str()),
            Some("notepad - Okur Dictionary Manager")
        );
        let manager = manager.map(|d| d.id).unwrap_or_default();
        press(&mut engine, manager, "add");
        let add = shown(&ui).map(|d| d.id).unwrap_or_default();
        send(
            &mut engine,
            FromUi::DialogChanged {
                id: add,
                control: "replacement".to_owned(),
                value: Value::Text("amount".to_owned()),
            },
        );
        press(&mut engine, add, "ok");
        send(&mut engine, FromUi::DialogClosed { id: manager });
        // The application's own dictionary now: Insert+D opens it without asking.
        insert_d(&mut engine);
        assert_eq!(
            shown(&ui).map(|d| d.title),
            Some("notepad - Okur Dictionary Manager".to_owned())
        );
        assert_eq!(
            engine.settings.dictionary,
            [DictionaryEntry::new("total", "amount")]
        );
    }
}
