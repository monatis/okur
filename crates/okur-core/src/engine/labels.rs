//! Custom labels in the engine (Insert+Ctrl+Tab in a page): the Custom Label dialog opens for
//! the element at the virtual cursor; OK gives it the label typed, which every reading of the
//! page then says in place of its name, and keeps it for the element's site. JAWS's cursor is
//! at the page's top after a label, as its page is refreshed (688/s15); Okur's stays on the
//! element (an accepted divergence).

use super::{AppMode, ScreenReaderEngine};
use crate::buffer::VirtualBuffer;
use crate::labels::{place_of, CustomLabels, LabelStore};
use crate::ui::label::{DeleteDialog, LabelDialog, Outcome};

/// What Custom Label Delete's question takes away on OK.
#[derive(Debug, Clone)]
pub(super) enum Deleting {
    /// The label of the element known by its key, in its place: kept by key, not by the page's
    /// node, which a page read again while the question is open renumbers (the code review of
    /// 2026-11-10, third pass, 5).
    One { place: String, key: String },
    /// Every label of the place.
    All(String),
}
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
            return self.say_message(crate::messages::msg::custom_label_not_here().to_string());
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

    /// Excel's Shift+F2 in a sheet: the active cell's note asked for, for Edit note. False
    /// outside a sheet.
    pub(super) fn ask_note_to_edit(&mut self) -> bool {
        if self.sheet_focus().is_none() || self.ui.is_none() {
            return false;
        }
        let asked = self
            .actions
            .as_ref()
            .is_some_and(|actions| actions.read_sheet_list(crate::event::SheetListKind::Note));
        if asked {
            self.sheet_list_asked = Some(crate::event::SheetListKind::Note);
            self.note_edit_asked = true;
        }
        asked
    }

    /// Edit note, holding the cell's note `current`.
    pub(super) fn open_note_dialog(&mut self, current: &str) {
        let Some(ui) = self.ui.as_ref() else {
            return;
        };
        self.dialog_count += 1;
        let dialog = LabelDialog::note(self.dialog_count, current);
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.note_dialog = Some(dialog);
    }

    /// Whether `message` is Edit note's.
    pub(super) fn is_note_event(&self, message: &FromUi) -> bool {
        self.note_dialog
            .as_ref()
            .is_some_and(|d| Some(d.id) == super::dialog_of(message))
    }

    /// What the user did to Edit note: OK writes the note through Excel, an empty one taking
    /// it away; the cell's focus then says itself as the dialog goes.
    pub(super) fn handle_note_event(&mut self, message: &FromUi) -> Option<String> {
        let outcome = self.note_dialog.as_mut()?.handle(message);
        if outcome == Outcome::Nothing {
            return None;
        }
        let dialog = self.note_dialog.take()?;
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogClose { id: dialog.id });
        }
        if let (Outcome::Label(text), Some(actions)) = (outcome, &self.actions) {
            actions.write_sheet_note(&text);
        }
        self.returning_from_dialog();
        // The cell, as the dialog gives the focus back to it, "Plan | A 1" [obs, 1197/s6].
        let cell = self.sheet_focus()?;
        let again = self.apps.iter().find_map(|app| app.focus_again(&cell))?;
        self.speech.speak(&again, true);
        self.said_again(&cell);
        Some(again)
    }

    /// The label of the element at the virtual cursor, if it has one: its place, its key and
    /// the label.
    fn label_at_cursor(&self) -> Option<(String, String, String)> {
        let target = self.label_target()?;
        let place = place_of(self.cursor.buffer.document_url()?);
        let key = self
            .cursor
            .buffer
            .node(target)
            .and_then(crate::labels::element_key)?;
        let label = self.custom_labels.places.get(&place)?.get(&key).cloned()?;
        Some((place, key, label))
    }

    /// Custom Label Delete (Run Okur Manager, in a page): asks whether to take the label of the
    /// element at the virtual cursor away, "Delete Custom Label? dialog | lamp | OK Button"
    /// [obs, 1207/s21]; with none there, or on text, "There is no Custom Label to delete for
    /// this item." [obs, 1207/s36]. Okur says nothing itself as it asks: the question is read as
    /// it takes the focus.
    pub(super) fn ask_delete_label(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let Some((place, key, label)) = self.label_at_cursor() else {
            return self
                .say_message(crate::messages::msg::custom_label_none_to_delete().to_string());
        };
        let ui = self.ui.as_ref()?;
        self.dialog_count += 1;
        let dialog = DeleteDialog {
            id: self.dialog_count,
            label: Some(label),
        };
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.labels_delete = Some((dialog, Deleting::One { place, key }));
        None
    }

    /// Custom Label Delete All (Run Okur Manager, in a page): asks whether to take away every
    /// custom label of the page's site, "Delete all Custom Labels? dialog | This will remove all
    /// custom labels for the current document or web page | OK Button" [obs, 688/s6]. Okur
    /// says nothing itself: the question is read as it takes the focus.
    pub(super) fn ask_delete_all_labels(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let place = place_of(self.cursor.buffer.document_url()?);
        let ui = self.ui.as_ref()?;
        self.dialog_count += 1;
        let dialog = DeleteDialog {
            id: self.dialog_count,
            label: None,
        };
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.labels_delete = Some((dialog, Deleting::All(place)));
        None
    }

    /// Whether `message` is Custom Label's, or Custom Label Delete's or Delete All's.
    pub(super) fn is_label_event(&self, message: &FromUi) -> bool {
        let id = super::dialog_of(message);
        self.label_dialog
            .as_ref()
            .is_some_and(|(d, _)| Some(d.id) == id)
            || self
                .labels_delete
                .as_ref()
                .is_some_and(|(d, _)| Some(d.id) == id)
    }

    /// What the user did to Custom Label. OK closes it silently [obs, 688/s14]. OK in Custom
    /// Label Delete All takes the site's labels away, quietly too [obs, 688/s7]; in Custom
    /// Label Delete, the element's; Cancel keeps it [obs, 1207/s29, s30].
    pub(super) fn handle_label_event(&mut self, message: &FromUi) -> Option<String> {
        if let Some(yes) = self
            .labels_delete
            .as_ref()
            .and_then(|(dialog, _)| dialog.answer(message))
        {
            let (dialog, deleting) = self.labels_delete.take()?;
            if let Some(ui) = &self.ui {
                ui.send(ToUi::DialogClose { id: dialog.id });
            }
            match deleting {
                Deleting::All(place) if yes => self.delete_labels(&place, None),
                Deleting::One { place, key } if yes => self.delete_labels(&place, Some(&key)),
                _ => {}
            }
            self.returning_from_dialog();
            return None;
        }
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

impl ScreenReaderEngine {
    /// Takes the label of the element known by `key` in `place` away, or every label of
    /// `place` with none, keeps that, and reads the page without them, the cursor where it was.
    fn delete_labels(&mut self, place: &str, key: Option<&str>) {
        let removed = match key {
            Some(key) => self.custom_labels.remove(place, key),
            None => self.custom_labels.remove_place(place),
        };
        if !removed {
            return;
        }
        if let Some(store) = &self.label_store {
            store.save(&self.custom_labels);
        }
        if self.cursor.buffer.document_url().map(place_of).as_deref() != Some(place) {
            return;
        }
        let Some(page) = self.cursor.buffer.page().cloned() else {
            return;
        };
        // The cursor by its node, not its offset: a label longer or shorter than its element's
        // own name moved the text after it (the code review of 2026-11-10, 4). Where it was in
        // a node whose text stays (a paragraph read over several lines), it stays; in one
        // whose label went, at its start, as a label given leaves it (second pass, 8).
        let offset = self.cursor.offset;
        let node = self.cursor.current_node_id;
        let text_of = |buffer: &VirtualBuffer, id: usize| {
            buffer.node(id).map(|n| (n.name.clone(), n.text.clone()))
        };
        let before = node.and_then(|id| {
            let start = self.cursor.buffer.start_of(id)?;
            Some((
                offset.saturating_sub(start),
                text_of(&self.cursor.buffer, id),
            ))
        });
        let rebuilt = VirtualBuffer::from_aom(&self.custom_labels.apply(place, &page));
        crate::retire::retire(std::mem::replace(&mut self.cursor.buffer, rebuilt));
        self.say_all = None;
        let at = node.zip(before).and_then(|(id, (within, text))| {
            let start = self.cursor.buffer.start_of(id)?;
            let same = text == text_of(&self.cursor.buffer, id);
            Some(if same {
                start.saturating_add(within)
            } else {
                start
            })
        });
        self.cursor.move_to(at.unwrap_or(offset));
    }
}

/// Node `id` in `node`'s tree.
fn find_node(node: &crate::aom::AomNode, id: usize) -> Option<&crate::aom::AomNode> {
    if node.id == id {
        return Some(node);
    }
    node.children.iter().find_map(|child| find_node(child, id))
}
