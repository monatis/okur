//! Flexible Web in the engine: Insert+Space, then X opens the wizard ([`crate::ui::flexible_web`])
//! on the elements at the virtual cursor; Finish makes a customization for the page in front,
//! Undo takes the last one back. Customizations are temporary: they hold until another page
//! loads, as JAWS's do until the page is left (the guide, Surf's Up chapter 16).
//!
//! A hidden element is taken out of the page Okur reads, as the guide describes ("it is
//! immediately applied to the page"). JAWS 2025 on the reference machine still reads it after
//! Finish [obs, 725/s12-s30], an accident Okur does not copy
//! (`docs/references/accepted-divergences.md`). Starting to read at an element applies when a
//! page loads, so a temporary one changes nothing until rules are saved.

use super::{AppMode, ScreenReaderEngine};
use crate::aom::AomNode;
use crate::buffer::VirtualBuffer;
use crate::flexible_web::{elements_at, without, Action, Customization};
use crate::messages::msg;
use crate::ui::flexible_web::{FlexibleWebWizard, Outcome};
use crate::ui::{FromUi, ToUi};

/// The customizations made on the page in front.
#[derive(Debug, Default)]
pub(super) struct PageCustomizations {
    /// The page as it was before them.
    original: Option<AomNode>,
    made: Vec<Customization>,
}

impl ScreenReaderEngine {
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
        let last = self
            .page_customizations
            .made
            .last()
            .map(Customization::label);
        self.dialog_count += 1;
        let wizard = FlexibleWebWizard::new(self.dialog_count, elements, last);
        if let Some(ui) = &self.ui {
            ui.send(ToUi::WizardShow {
                wizard: wizard.spec(),
            });
        }
        self.flexible_web = Some(wizard);
        None
    }

    /// Whether `message` is the wizard's.
    pub(super) fn is_flexible_web_event(&self, message: &FromUi) -> bool {
        let id = super::dialog_of(message);
        id.is_some() && self.flexible_web.as_ref().map(|w| w.id) == id
    }

    /// What the user did to the wizard.
    pub(super) fn handle_flexible_web_event(&mut self, message: &FromUi) -> Option<String> {
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
            Outcome::Closed => self.returning_from_dialog(),
        }
        None
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
        if self.page_customizations.original.is_none() {
            self.page_customizations.original =
                self.cursor.buffer.page().map(|p| AomNode::clone(p));
        }
        self.page_customizations.made.push(customization);
        self.reread_customized();
    }

    /// The page in front built again from what it was before its customizations, with them,
    /// the cursor on the element it was on, or where it was.
    fn reread_customized(&mut self) {
        let Some(original) = &self.page_customizations.original else {
            return;
        };
        let page = self.customized_page(original);
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

    /// `page` with the customizations made on it: what they hide taken out.
    fn customized_page(&self, page: &AomNode) -> AomNode {
        self.page_customizations
            .made
            .iter()
            .filter(|c| c.action == Action::Hide)
            .fold(page.clone(), |page, c| without(&page, &c.find(&page)))
    }

    /// A page refreshed after a change keeps what Flexible Web hides.
    pub(super) fn with_customizations(&self, buffer: VirtualBuffer) -> VirtualBuffer {
        if self.page_customizations.made.is_empty() {
            return buffer;
        }
        let Some(page) = buffer.page().map(|p| self.customized_page(p)) else {
            return buffer;
        };
        crate::retire::retire(buffer);
        VirtualBuffer::from_aom_owned(page)
    }

    /// Another page loaded: the customizations made on the one before are gone.
    pub(super) fn forget_customizations(&mut self) {
        self.page_customizations = PageCustomizations::default();
    }
}
