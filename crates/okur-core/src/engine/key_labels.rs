//! Manage Key Labels over Settings Center ([`crate::ui::key_labels`]): opened from its tree's
//! "Manage Key Labels..." or its button in the pane, on the draft's key labels; OK gives the
//! draft its changes, which Settings Center's own OK or Apply keeps, as JAWS's [obs,
//! key-labels-store-probe.ps1].

use super::ScreenReaderEngine;
use crate::ui::key_labels::{KeyLabelsDialog, KeyLabelsOutcome};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// Opens Manage Key Labels over Settings Center.
    pub(super) fn open_key_labels(&mut self) {
        let Some(tree) = self.settings_tree.as_ref() else {
            return;
        };
        self.dialog_count += 1;
        let dialog = KeyLabelsDialog::new(self.dialog_count, tree.draft().key_labels.clone());
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogShow {
                dialog: dialog.spec(),
            });
        }
        self.key_labels_dialog = Some(dialog);
    }

    pub(super) fn is_key_labels_event(&self, message: &FromUi) -> bool {
        let id = super::dialog_of(message);
        id.is_some() && self.key_labels_dialog.as_ref().map(|d| d.id) == id
    }

    /// What the user did in Manage Key Labels.
    pub(super) fn handle_key_labels_event(&mut self, message: &FromUi) -> Option<String> {
        let dialog = self.key_labels_dialog.as_mut()?;
        let id = dialog.id;
        match dialog.handle(message) {
            KeyLabelsOutcome::Nothing => None,
            KeyLabelsOutcome::Send(sent) => {
                if let Some(ui) = &self.ui {
                    sent.into_iter().for_each(|m| ui.send(m));
                }
                None
            }
            KeyLabelsOutcome::Speak(sent, said) => {
                if let Some(ui) = &self.ui {
                    sent.into_iter().for_each(|m| ui.send(m));
                }
                self.speech.speak(&said, true);
                Some(said)
            }
            KeyLabelsOutcome::Done(changes) => {
                self.key_labels_dialog = None;
                let mut sent = vec![ToUi::DialogClose { id }];
                if let Some(tree) = self.settings_tree.as_mut() {
                    sent.extend(tree.set_key_labels(changes));
                }
                if let Some(ui) = &self.ui {
                    sent.into_iter().for_each(|m| ui.send(m));
                }
                None
            }
            KeyLabelsOutcome::Closed => {
                self.key_labels_dialog = None;
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogClose { id });
                }
                None
            }
        }
    }
}
