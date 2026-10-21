//! Custom labels in the engine (Insert+Ctrl+Tab in a page): the Custom Label dialog opens for
//! the element at the virtual cursor; OK gives it the label typed, which every reading of the
//! page then says in place of its name, and keeps it for the element's site. JAWS's cursor is
//! at the page's top after a label, as its page is refreshed (688/s15); Okur's stays on the
//! element (an accepted divergence).

use super::{AppMode, ScreenReaderEngine};
use crate::buffer::VirtualBuffer;
use crate::labels::{place_of, CustomLabels, LabelStore};
use crate::ui::label::{LabelDialog, Outcome};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// The labels Okur has, and where it keeps them.
    pub fn set_custom_labels(&mut self, labels: CustomLabels, store: Option<Box<dyn LabelStore>>) {
        self.custom_labels = labels;
        self.label_store = store;
    }

    /// `buffer` with its site's custom labels, built again from its page when it has any.
    pub(super) fn labeled(&self, buffer: VirtualBuffer) -> VirtualBuffer {
        let Some(place) = buffer.document_url().map(place_of) else {
            return buffer;
        };
        if !self.custom_labels.has(&place) {
            return buffer;
        }
        let Some(page) = buffer.page() else {
            return buffer;
        };
        let rebuilt = VirtualBuffer::from_aom(&self.custom_labels.apply(&place, page));
        crate::retire::retire(buffer);
        rebuilt
    }

    /// The element a label is for: the form field, button, link or graphic at the virtual
    /// cursor (JAWS's guide, chapter 19). Text has none.
    fn label_target(&self) -> Option<usize> {
        let buffer = &self.cursor.buffer;
        let offset = self.cursor.offset;
        buffer
            .find_form_control_node(offset)
            .or_else(|| buffer.find_button_node(offset))
            .or_else(|| buffer.find_link_node(offset))
            .map(|node| node.id)
            .or_else(|| {
                let id = self.cursor.current_node_id?;
                matches!(buffer.node(id)?.role, crate::aom::Role::Image).then_some(id)
            })
    }

    /// Insert+Ctrl+Tab: Custom Label opens for the element at the virtual cursor, holding its
    /// label. Okur says nothing itself: the dialog is read as it takes the focus [obs,
    /// 688/s9]. On text no dialog opens: "A Custom Label cannot be created for this item."
    /// (JAWS on a paragraph, live).
    pub(super) fn open_custom_label(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let Some(target) = self.label_target() else {
            return self.say(crate::messages::msg::custom_label_not_here().to_string());
        };
        let place = place_of(self.cursor.buffer.document_url()?);
        let current = self
            .cursor
            .buffer
            .node(target)
            .and_then(crate::labels::element_key)
            .and_then(|key| self.custom_labels.places.get(&place)?.get(&key).cloned())
            .unwrap_or_default();
        let ui = self.ui.as_ref()?;
        self.dialog_count += 1;
        let dialog = LabelDialog::new(self.dialog_count, &current);
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.label_dialog = Some((dialog, target));
        None
    }

    /// Whether `message` is Custom Label's.
    pub(super) fn is_label_event(&self, message: &FromUi) -> bool {
        self.label_dialog
            .as_ref()
            .is_some_and(|(d, _)| Some(d.id) == super::dialog_of(message))
    }

    /// What the user did to Custom Label. OK closes it silently [obs, 688/s14].
    pub(super) fn handle_label_event(&mut self, message: &FromUi) -> Option<String> {
        let outcome = self.label_dialog.as_mut()?.0.handle(message);
        if outcome == Outcome::Nothing {
            return None;
        }
        let (dialog, target) = self.label_dialog.take()?;
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogClose { id: dialog.id });
        }
        if let Outcome::Label(text) = outcome {
            self.give_label(target, &text);
        }
        self.returning_from_dialog();
        None
    }

    /// Gives node `target` of the page the label `text` (none when empty), keeps it, and reads
    /// the page with it, the cursor on the node.
    fn give_label(&mut self, target: usize, text: &str) {
        let Some(url) = self.cursor.buffer.document_url() else {
            return;
        };
        let place = place_of(url);
        // The node as the page has it, its own name, not a label given before.
        let Some(original) = self
            .cursor
            .buffer
            .page()
            .and_then(|page| find_node(page, target))
            .cloned()
        else {
            return;
        };
        if !self.custom_labels.set(&place, &original, Some(text)) {
            return;
        }
        if let Some(store) = &self.label_store {
            store.save(&self.custom_labels);
        }
        let Some(page) = self.cursor.buffer.page().cloned() else {
            return;
        };
        let rebuilt = VirtualBuffer::from_aom(&self.custom_labels.apply(&place, &page));
        crate::retire::retire(std::mem::replace(&mut self.cursor.buffer, rebuilt));
        self.say_all = None;
        if let Some(offset) = self.cursor.buffer.start_of(target) {
            self.cursor.move_to(offset);
        } else {
            self.cursor.move_to_start();
        }
    }
}

/// Node `id` in `node`'s tree.
fn find_node(node: &crate::aom::AomNode, id: usize) -> Option<&crate::aom::AomNode> {
    if node.id == id {
        return Some(node);
    }
    node.children.iter().find_map(|child| find_node(child, id))
}
