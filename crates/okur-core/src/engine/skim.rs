//! Skim Reading in the engine (Ctrl+Shift+Insert+Down). The dialog opens over the page; its
//! choices stay for Ctrl+Insert+Down while Okur runs. Escape and Close give the page back
//! silently, as JAWS's dialog does [obs, 760/s14].

use super::{AppMode, ScreenReaderEngine};
use crate::ui::skim::{Outcome, SkimDialog, SkimRule};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// Opens Skim Reading over the page, or brings it to the front. Okur says nothing itself:
    /// it reads the dialog as the dialog takes the focus.
    pub(super) fn open_skim_dialog(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let ui = self.ui.as_ref()?;
        let dialog = match self.skim_dialog.take() {
            Some(dialog) => dialog,
            None => {
                self.dialog_count += 1;
                SkimDialog::new(self.dialog_count, self.skim_options)
            }
        };
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.skim_dialog = Some(dialog);
        None
    }

    /// Whether `message` is Skim Reading's.
    pub(super) fn is_skim_event(&self, message: &FromUi) -> bool {
        self.skim_dialog
            .as_ref()
            .is_some_and(|d| Some(d.id) == super::dialog_of(message))
    }

    /// What the user did to Skim Reading.
    pub(super) fn handle_skim_event(&mut self, message: &FromUi) -> Option<String> {
        let outcome = self.skim_dialog.as_mut()?.handle(message);
        let options = match outcome {
            Outcome::Nothing => return None,
            Outcome::Start(options) | Outcome::Summary(options) | Outcome::Closed(options) => {
                options
            }
        };
        self.skim_options = options;
        if let (Some(dialog), Some(ui)) = (self.skim_dialog.take(), &self.ui) {
            if !matches!(outcome, Outcome::Closed(_)) {
                ui.send(ToUi::DialogClose { id: dialog.id });
            }
        }
        self.returning_from_dialog();
        match outcome {
            Outcome::Start(_) => self.start_skim_read(),
            Outcome::Nothing | Outcome::Summary(_) | Outcome::Closed(_) => None,
        }
    }

    /// The rule Ctrl+Insert+Down reads by, when Okur cannot read by it: said so.
    pub(super) fn skim_rule_unread(&mut self) -> Option<String> {
        match self.skim_options.rule {
            SkimRule::FirstLine | SkimRule::FirstSentence => None,
            SkimRule::TextRules | SkimRule::Attributes => {
                let said = crate::messages::msg::skim_rules_not_yet().to_string();
                self.speech.speak(&said, true);
                Some(said)
            }
        }
    }
}
