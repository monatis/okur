//! Configure Web Verbosity Levels over Settings Center ([`crate::ui::web_verbosity`]): a
//! level's choices opened from its item ("Medium...") or its button in the pane, on the draft's
//! choices; OK gives the draft the changes, which Settings Center's own OK or Apply keeps.

use super::ScreenReaderEngine;
use crate::settings::WebVerbosity;
use crate::ui::web_verbosity::{WebLevelDialog, WebLevelOutcome};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// Opens `level`'s choices over Settings Center.
    pub(super) fn open_web_level(&mut self, level: WebVerbosity) {
        let Some(tree) = self.settings_tree.as_ref() else {
            return;
        };
        self.dialog_count += 1;
        let dialog = WebLevelDialog::new(
            self.dialog_count,
            level,
            tree.draft().web_structures.clone(),
        );
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogShow {
                dialog: dialog.spec(),
            });
        }
        self.web_level_dialog = Some(dialog);
    }

    pub(super) fn is_web_level_event(&self, message: &FromUi) -> bool {
        let id = super::dialog_of(message);
        id.is_some() && self.web_level_dialog.as_ref().map(|d| d.id) == id
    }

    /// What the user did in a level's choices.
    pub(super) fn handle_web_level_event(&mut self, message: &FromUi) -> Option<String> {
        let dialog = self.web_level_dialog.as_mut()?;
        let id = dialog.id;
        let mut sent = match dialog.handle(message) {
            WebLevelOutcome::Nothing => Vec::new(),
            WebLevelOutcome::Send(sent) => sent,
            WebLevelOutcome::Done(changes) => {
                self.web_level_dialog = None;
                let mut sent = vec![ToUi::DialogClose { id }];
                if let Some(tree) = self.settings_tree.as_mut() {
                    sent.extend(tree.set_web_structures(changes));
                }
                sent
            }
            WebLevelOutcome::Closed => {
                self.web_level_dialog = None;
                vec![ToUi::DialogClose { id }]
            }
        };
        if let Some(ui) = &self.ui {
            sent.drain(..).for_each(|m| ui.send(m));
        }
        None
    }
}
