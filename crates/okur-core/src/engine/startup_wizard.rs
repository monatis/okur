//! The Startup Wizard in the engine ([`crate::ui::startup_wizard`]): opened from the Okur
//! window's Help, Startup Wizard..., and by Okur's first start. A change is heard at once: the
//! wizard's settings are put in the session layer while it is open, as JAWS's take effect as
//! they are made [R-SUI-35]; Finish keeps them for all applications, Cancel and Escape give the
//! session layer back as it was (JAWS's help: only Finish saves).

use super::ScreenReaderEngine;
use crate::messages::msg;
use crate::settings_layers::Scope;
use crate::ui::startup_wizard::{Outcome, StartupWizard};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// Opens the Startup Wizard on its first page, or brings it to the front.
    pub fn open_startup_wizard(&mut self) -> Option<String> {
        let Some(ui) = &self.ui else {
            let speech = msg::settings_dialog_unavailable().to_string();
            self.speech.speak(&speech, true);
            return Some(speech);
        };
        let wizard = match self.startup_wizard.take() {
            Some(wizard) => wizard,
            None => {
                self.dialog_count += 1;
                self.startup_wizard_session = self.settings_layers.session.clone();
                let wizard =
                    StartupWizard::new(self.dialog_count, self.settings_layers.resolve(None, None));
                match &self.start_options {
                    Some(options) => wizard.with_start(options.user_start().unwrap_or_default()),
                    None => wizard,
                }
            }
        };
        ui.send(ToUi::WizardShow {
            wizard: wizard.spec(),
        });
        self.startup_wizard = Some(wizard);
        None
    }

    /// Whether `message` is the wizard's.
    pub(super) fn is_startup_wizard_event(&self, message: &FromUi) -> bool {
        let id = super::dialog_of(message);
        id.is_some() && self.startup_wizard.as_ref().map(|w| w.id) == id
    }

    /// What the user did in the wizard.
    pub(super) fn handle_startup_wizard_event(&mut self, message: &FromUi) -> Option<String> {
        let outcome = self.startup_wizard.as_mut()?.handle(message);
        match outcome {
            Outcome::Nothing => {}
            Outcome::Turn(page, buttons) => {
                if let (Some(ui), Some(wizard)) = (&self.ui, &self.startup_wizard) {
                    ui.send(ToUi::WizardPage {
                        id: wizard.id,
                        page,
                        buttons,
                    });
                }
            }
            Outcome::Try { before, settings } => {
                self.settings_layers.session = self.startup_wizard_session.clone();
                self.settings_layers
                    .change(&Scope::Session, &before, &settings, None);
                self.resolve_settings();
            }
            Outcome::Finish {
                before,
                settings,
                start,
            } => {
                self.settings_layers.session = std::mem::take(&mut self.startup_wizard_session);
                self.apply_settings(&Scope::Global, &before, &settings, &[]);
                if let (Some(options), Some(start)) = (&self.start_options, start) {
                    if options.user_start() != Some(start) {
                        options.set_user_start(start);
                    }
                }
                if let (Some(wizard), Some(ui)) = (self.startup_wizard.take(), &self.ui) {
                    ui.send(ToUi::DialogClose { id: wizard.id });
                }
                self.returning_from_dialog();
            }
            Outcome::Closed => {
                self.settings_layers.session = std::mem::take(&mut self.startup_wizard_session);
                self.startup_wizard = None;
                self.resolve_settings();
                self.returning_from_dialog();
            }
        }
        None
    }
}
