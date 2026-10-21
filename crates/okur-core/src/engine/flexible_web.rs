//! Flexible Web in the engine: Insert+Space, then X opens the wizard ([`crate::ui::flexible_web`])
//! on the elements at the virtual cursor; Finish makes a customization for the page in front,
//! Undo takes the last one back. Customizations are temporary: they hold until another page
//! loads, as JAWS's do until the page is left (the guide, Surf's Up chapter 16). Saved as a
//! rule, they hold on every page the rule applies to, from one visit to the next.
//!
//! When another page loads while the one before has temporary customizations, Okur asks
//! whether to save them, as JAWS does; Yes opens the wizard on its last page [obs, 742/s30,
//! s31].
//!
//! A hidden element is taken out of the page Okur reads, as the guide describes ("it is
//! immediately applied to the page"). JAWS 2025 on the reference machine still reads it after
//! Finish [obs, 725/s12-s30], an accident Okur does not copy
//! (`docs/references/accepted-divergences.md`). Starting to read at an element applies when a
//! page loads, so a temporary one changes nothing until rules are saved.

use super::{AppMode, ScreenReaderEngine};
use crate::aom::AomNode;
use crate::buffer::VirtualBuffer;
use crate::flexible_web::{elements_at, without, Action, Customization, Rule, RuleStore, Rules};
use crate::messages::msg;
use crate::ui::flexible_web::{FlexibleWebWizard, Outcome, UnsavedQuestion};
use crate::ui::{FromUi, ToUi};

/// The customizations on the page in front.
#[derive(Debug, Default)]
pub(super) struct PageCustomizations {
    /// The page as it was before them; none while nothing is customized on it.
    original: Option<AomNode>,
    /// Those made on it, not saved.
    made: Vec<Customization>,
}

/// Temporary customizations of a page another page replaced: asked about, then saved or lost.
#[derive(Debug)]
pub(super) struct Leaving {
    /// The page they were made on.
    url: String,
    made: Vec<Customization>,
    /// The question about them, while it is open.
    question: Option<UnsavedQuestion>,
}

impl ScreenReaderEngine {
    /// The Flexible Web rules Okur has saved, and where it keeps them.
    pub fn set_flexible_web_rules(&mut self, rules: Rules, store: Option<Box<dyn RuleStore>>) {
        self.flexible_web_rules = rules;
        self.flexible_web_store = store;
    }

    /// The Flexible Web rules saved.
    pub fn flexible_web_rules(&self) -> &Rules {
        &self.flexible_web_rules
    }

    /// Insert+Space, then X: Flexible Web's wizard, on the elements at the virtual cursor.
    /// Okur says nothing itself: the wizard is read as it takes the focus [obs, 705/s3].
    /// Outside a page: "You must be in the web page area to activate Flexible Web."
    pub(super) fn open_flexible_web(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer || self.cursor.buffer.page().is_none() {
            return self.say(msg::flexible_web_not_in_page().to_string());
        }
        self.ui.as_ref()?;
        let at = self.cursor.current_node_id.or_else(|| {
            self.cursor
                .buffer
                .line_at_offset(self.cursor.offset)
                .map(|line| line.node_id)
        });
        let elements = match (self.cursor.buffer.page(), at) {
            (Some(page), Some(id)) => elements_at(page, id),
            _ => Vec::new(),
        };
        self.dialog_count += 1;
        let wizard = FlexibleWebWizard::new(
            self.dialog_count,
            elements,
            self.page_customizations.made.clone(),
            !self.flexible_web_rules.rules.is_empty(),
        );
        self.show_wizard(wizard);
        None
    }

    fn show_wizard(&mut self, wizard: FlexibleWebWizard) {
        if let Some(ui) = &self.ui {
            ui.send(ToUi::WizardShow {
                wizard: wizard.spec(),
            });
        }
        self.flexible_web = Some(wizard);
    }

    /// Whether `message` is the wizard's, or the question's.
    pub(super) fn is_flexible_web_event(&self, message: &FromUi) -> bool {
        let id = super::dialog_of(message);
        id.is_some()
            && (self.flexible_web.as_ref().map(|w| w.id) == id
                || self
                    .flexible_web_leaving
                    .as_ref()
                    .and_then(|l| l.question.as_ref())
                    .map(|q| q.id)
                    == id)
    }

    /// What the user did to the wizard, or to the question.
    pub(super) fn handle_flexible_web_event(&mut self, message: &FromUi) -> Option<String> {
        if let Some(yes) = self
            .flexible_web_leaving
            .as_ref()
            .and_then(|l| l.question.as_ref())
            .and_then(|q| q.answer(message))
        {
            return self.answer_unsaved_question(yes);
        }
        let mut wizard = self.flexible_web.take()?;
        let outcome = wizard.handle(message, self.cursor.buffer.page().map(|p| &**p));
        let id = wizard.id;
        match outcome {
            Outcome::Nothing => self.flexible_web = Some(wizard),
            Outcome::Turn(page, buttons) => {
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::WizardPage { id, page, buttons });
                }
                self.flexible_web = Some(wizard);
            }
            Outcome::Buttons(buttons) => {
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::WizardButtons { id, buttons });
                }
                self.flexible_web = Some(wizard);
            }
            Outcome::NoElements => {
                self.flexible_web = Some(wizard);
                return self.say(msg::flexible_web_no_elements().to_string());
            }
            Outcome::Apply(customization) => {
                self.close_wizard(id);
                self.customize(customization);
            }
            Outcome::Undo => {
                self.close_wizard(id);
                self.page_customizations.made.pop();
                self.reread_customized();
            }
            Outcome::Save {
                name,
                scope,
                customizations,
            } => {
                self.close_wizard(id);
                // Saved from the question, they belong to the page that was left.
                let url = match self.flexible_web_leaving.take() {
                    Some(leaving) => leaving.url,
                    None => self.page_url(),
                };
                self.save_rule(Rule::new(name, scope, &url, customizations));
            }
            Outcome::Closed => {
                self.flexible_web_leaving = None;
                self.returning_from_dialog();
            }
        }
        None
    }

    /// The question answered: Yes opens the wizard on its last page alone, to save them as a
    /// rule [obs, 742/s31]; No loses them, the page taking the focus back as from any dialog.
    fn answer_unsaved_question(&mut self, yes: bool) -> Option<String> {
        let mut leaving = self.flexible_web_leaving.take()?;
        if let (Some(question), Some(ui)) = (leaving.question.take(), &self.ui) {
            ui.send(ToUi::DialogClose { id: question.id });
        }
        if yes {
            self.dialog_count += 1;
            let wizard = FlexibleWebWizard::saving(self.dialog_count, leaving.made.clone());
            self.flexible_web_leaving = Some(leaving);
            self.show_wizard(wizard);
        }
        None
    }

    /// The temporary customizations of the page in front, as another page replaces it: kept to
    /// ask about while Okur has its windows.
    pub(super) fn leaving_customizations(&mut self) -> Option<Leaving> {
        if self.page_customizations.made.is_empty() || self.ui.is_none() {
            return None;
        }
        Some(Leaving {
            url: self.page_url(),
            made: std::mem::take(&mut self.page_customizations.made),
            question: None,
        })
    }

    /// Asks whether to save the customizations of the page left.
    pub(super) fn ask_about_leaving(&mut self, mut leaving: Leaving) {
        self.dialog_count += 1;
        let question = UnsavedQuestion {
            id: self.dialog_count,
        };
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogShow {
                dialog: question.spec(),
            });
        }
        leaving.question = Some(question);
        self.flexible_web_leaving = Some(leaving);
    }

    /// The page in front's address, none for a page without one.
    fn page_url(&self) -> String {
        self.cursor
            .buffer
            .document_url()
            .unwrap_or_default()
            .to_owned()
    }

    /// Saves `rule`, applied at once: the customizations it keeps are no longer temporary.
    fn save_rule(&mut self, rule: Rule) {
        self.page_customizations
            .made
            .retain(|made| !rule.customizations.contains(made));
        self.flexible_web_rules.rules.push(rule);
        if let Some(store) = &self.flexible_web_store {
            store.save(&self.flexible_web_rules);
        }
        self.reread_customized();
    }

    /// Closes the wizard, the focus going back to the page quietly: JAWS says nothing after
    /// Finish, Undo or Escape [obs, 705/s21, 716/s15, s29].
    fn close_wizard(&mut self, id: u32) {
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogClose { id });
        }
        self.returning_from_dialog();
    }

    /// Makes `customization` for the page in front.
    fn customize(&mut self, customization: Customization) {
        self.page_customizations.made.push(customization);
        self.reread_customized();
    }

    /// The page in front built again from what it was before its customizations, with them,
    /// the cursor on the element it was on, or where it was.
    fn reread_customized(&mut self) {
        if self.page_customizations.original.is_none() {
            self.page_customizations.original =
                self.cursor.buffer.page().map(|p| AomNode::clone(p));
        }
        let Some(original) = &self.page_customizations.original else {
            return;
        };
        let page = self.customized_page(original, self.cursor.buffer.document_url());
        let at = self.cursor.current_node_id;
        let offset = self.cursor.offset;
        crate::retire::retire(std::mem::replace(
            &mut self.cursor.buffer,
            VirtualBuffer::from_aom_owned(page),
        ));
        self.say_all = None;
        match at.and_then(|id| self.cursor.buffer.start_of(id)) {
            Some(start) => self.cursor.move_to(start),
            None => self
                .cursor
                .move_to(offset.min(self.cursor.buffer.text.len())),
        }
    }

    /// The customizations on the page at `url`: its rules', then those made on it.
    fn customizations_on<'a>(
        &'a self,
        url: Option<&'a str>,
    ) -> impl Iterator<Item = &'a Customization> + 'a {
        url.into_iter()
            .flat_map(|url| self.flexible_web_rules.for_page(url))
            .flat_map(|rule| rule.customizations.iter())
            .chain(self.page_customizations.made.iter())
    }

    /// `page`, at `url`, with its customizations: what they hide taken out.
    fn customized_page(&self, page: &AomNode, url: Option<&str>) -> AomNode {
        self.customizations_on(url)
            .filter(|c| c.action == Action::Hide)
            .fold(page.clone(), |page, c| without(&page, &c.find(&page)))
    }

    /// A page loaded or refreshed keeps what Flexible Web hides on it.
    pub(super) fn with_customizations(&self, buffer: VirtualBuffer) -> VirtualBuffer {
        let url = buffer.document_url();
        if self.customizations_on(url).next().is_none() {
            return buffer;
        }
        let Some(page) = buffer.page().map(|p| self.customized_page(p, url)) else {
            return buffer;
        };
        crate::retire::retire(buffer);
        VirtualBuffer::from_aom_owned(page)
    }

    /// The page `buffer` holds, as it is before its customizations, kept to build it again
    /// when they change; none is kept for a page with none.
    pub(super) fn keep_original(&mut self, buffer: &VirtualBuffer) {
        let customized = self
            .customizations_on(buffer.document_url())
            .next()
            .is_some();
        self.page_customizations.original = buffer
            .page()
            .filter(|_| customized)
            .map(|p| AomNode::clone(p));
    }

    /// Another page loaded: the customizations made on the one before are gone.
    pub(super) fn forget_customizations(&mut self) {
        self.page_customizations = PageCustomizations::default();
    }
}
