//! PlaceMarkers: places in a page to come back to with K and Shift+K (JAWS 2025 help, Surf's
//! Up chapter 15). The temporary PlaceMarker, Ctrl+Windows+K, is one per page for the
//! session: "Temporary PlaceMarker set", then the line [obs, 687/s4]. Saved PlaceMarkers are
//! kept per page from one visit to the next ([`crate::placemarkers`]), added, renamed and
//! removed in the PlaceMarker List (Ctrl+Shift+K, [`crate::ui::placemarkers`]). K and Shift+K
//! go to the next or prior marker, saved or temporary, and say the element there, "Maria
//! Lopez same page link"; with one marker K says it again, without a wrapping message [obs,
//! 687/s6-s8; 702/s11, s12]. A page without one says "There are no PlaceMarkers on this
//! page." [obs, 221/s4-s5]. 7 to 0 say PlaceMarker 1 to 4, Shift with them moves there.

use super::{AppMode, ScreenReaderEngine};
use crate::messages::msg;
use crate::placemarkers::{place_in, PlaceMarker, PlaceMarkerStore, PlaceMarkers};
use crate::ui::placemarkers::{
    Details, ListDialog, ListOutcome, NameDialog, NameOutcome, RemoveAllDialog, RemoveAllOutcome,
    Row, ANCHOR_TEXT, FOR_CHOSEN,
};
use crate::ui::{FromUi, ToUi};

/// Which PlaceMarker a row of the list is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Marker {
    /// The page's own, by its place in the page's list.
    Own(usize),
    /// One generalized to the site, kept with another of its pages.
    Site { page: String, index: usize },
    /// The page's temporary PlaceMarker.
    Temporary,
}

/// A dialog the PlaceMarker List opened over itself.
#[derive(Debug)]
enum Child {
    /// Add (`None`) or Change for a row.
    Name(NameDialog, Option<usize>),
    RemoveAll(RemoveAllDialog),
}

/// The PlaceMarker List while it is open, for the page it was opened on.
#[derive(Debug)]
pub(super) struct ListState {
    dialog: ListDialog,
    page: String,
    markers: Vec<Marker>,
    child: Option<Child>,
}

impl ScreenReaderEngine {
    /// The PlaceMarkers Okur has saved, and where it keeps them.
    pub fn set_placemarkers(
        &mut self,
        markers: PlaceMarkers,
        store: Option<Box<dyn PlaceMarkerStore>>,
    ) {
        self.placemarkers = markers;
        self.placemarker_store = store;
    }

    /// The page the markers belong to: its address, or none for a page without one.
    fn placemarker_page(&self) -> String {
        self.cursor
            .buffer
            .document_url()
            .unwrap_or_default()
            .to_owned()
    }

    /// Ctrl+Windows+K: the temporary PlaceMarker at the virtual cursor, in place of the page's
    /// earlier one.
    pub(super) fn define_temp_placemarker(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let page = self.placemarker_page();
        self.temp_placemarkers.insert(page, self.cursor.offset);
        let said = format!(
            "{}\n{}",
            msg::placemarker_temporary_set(),
            self.cursor.current_line()
        );
        self.say(said)
    }

    /// The page's PlaceMarkers in the list's order, with their rows and where they are now:
    /// its own as they were added, its site's generalized ones, then its temporary one.
    fn page_markers(&self, page: &str) -> Vec<(Marker, Row, usize)> {
        let buffer = &self.cursor.buffer;
        let own_page = crate::placemarkers::page_of(page);
        let mut rows = Vec::new();
        let own = self.placemarkers.pages.get(&own_page).into_iter().flatten();
        for (index, marker) in own.enumerate() {
            let row = Row {
                name: marker.name.clone(),
                domain: marker.domain,
            };
            rows.push((Marker::Own(index), row, place_in(buffer, marker)));
        }
        if let Some(site) = crate::settings_layers::site_of(page) {
            for (other, list) in &self.placemarkers.pages {
                if *other == own_page
                    || crate::settings_layers::site_of(other).as_ref() != Some(&site)
                {
                    continue;
                }
                for (index, marker) in list.iter().enumerate().filter(|(_, m)| m.domain) {
                    let row = Row {
                        name: marker.name.clone(),
                        domain: true,
                    };
                    let place = Marker::Site {
                        page: other.clone(),
                        index,
                    };
                    rows.push((place, row, place_in(buffer, marker)));
                }
            }
        }
        if let Some(&offset) = self.temp_placemarkers.get(page) {
            let row = Row {
                name: msg::placemarker_temporary().to_string(),
                domain: false,
            };
            rows.push((Marker::Temporary, row, offset));
        }
        rows
    }

    /// Where the page's markers are, in the list's order.
    fn placemarker_offsets(&self) -> Vec<usize> {
        self.page_markers(&self.placemarker_page())
            .into_iter()
            .map(|(_, _, offset)| offset)
            .collect()
    }

    /// K or Shift+K: the next or prior PlaceMarker of the page. Without any, once the page's
    /// PlaceMarker List was opened since it loaded, K opens it, to add one [obs, 703/s1, s4;
    /// 702/s30].
    pub(super) fn move_to_placemarker(&mut self, forward: bool) -> Option<String> {
        let markers = self.placemarker_offsets();
        if markers.is_empty()
            && self.placemarker_list_seen.as_deref() == Some(self.placemarker_page().as_str())
        {
            return self.open_placemarker_list();
        }
        // Said by quick navigation, as the other elements are.
        Some(
            self.cursor
                .move_to_placemarker(&markers, forward)
                .unwrap_or_else(|| msg::quicknav_none_placemarkers().to_string()),
        )
    }

    /// 7 to 0: the line at the page's PlaceMarker `number`, without moving there [obs,
    /// 702/s13]; with Shift, moving there [obs, 702/s16]. One the page does not have: "No
    /// place marker #2" [obs, 702/s14].
    pub(super) fn say_placemarker_number(&mut self, number: u8, moves: bool) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let index = usize::from(number).checked_sub(1)?;
        let Some(&offset) = self.placemarker_offsets().get(index) else {
            return self.say(msg::placemarker_none_numbered(number).render());
        };
        let said = if moves {
            self.cursor.move_to(offset);
            self.cursor.current_line()
        } else {
            self.cursor.line_speech_at(offset)
        };
        self.say(said)
    }

    /// Ctrl+Shift+K: the PlaceMarker List. Okur says nothing itself: the dialog is read as it
    /// takes the focus [obs, 702/s4].
    pub(super) fn open_placemarker_list(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let page = self.placemarker_page();
        let (markers, rows): (Vec<Marker>, Vec<Row>) = self
            .page_markers(&page)
            .into_iter()
            .map(|(marker, row, _)| (marker, row))
            .unzip();
        self.ui.as_ref()?;
        self.placemarker_list_seen = Some(page.clone());
        self.dialog_count += 1;
        let dialog = ListDialog::new(self.dialog_count, rows);
        self.show(dialog.spec());
        self.placemarker_list = Some(ListState {
            dialog,
            page,
            markers,
            child: None,
        });
        None
    }

    /// Whether `message` is the PlaceMarker List's or its child dialog's.
    pub(super) fn is_placemarker_event(&self, message: &FromUi) -> bool {
        let Some(state) = &self.placemarker_list else {
            return false;
        };
        let id = super::dialog_of(message);
        let child = state.child.as_ref().map(|child| match child {
            Child::Name(dialog, _) => dialog.id,
            Child::RemoveAll(dialog) => dialog.id,
        });
        id.is_some() && (id == Some(state.dialog.id) || id == child)
    }

    /// What the user did to the PlaceMarker List or the dialog it opened.
    pub(super) fn handle_placemarker_event(&mut self, message: &FromUi) -> Option<String> {
        let mut state = self.placemarker_list.take()?;
        let (open, said) = match state.child.take() {
            Some(child) => self.child_event(&mut state, child, message),
            None => self.list_event(&mut state, message),
        };
        if open {
            self.placemarker_list = Some(state);
        }
        said
    }

    /// The list's own controls; whether it is still open, and what is said.
    fn list_event(&mut self, state: &mut ListState, message: &FromUi) -> (bool, Option<String>) {
        let id = state.dialog.id;
        match state.dialog.handle(message) {
            ListOutcome::Nothing => (true, None),
            // MoveTo, Change... and Remove become available, MoveTo the default (JAWS's
            // guide, chapter 15).
            ListOutcome::Chosen(_) => {
                if let Some(ui) = &self.ui {
                    for control in FOR_CHOSEN {
                        ui.send(ToUi::DialogEnable {
                            id,
                            control: control.to_owned(),
                            enabled: true,
                        });
                    }
                    ui.send(ToUi::DialogDefault {
                        id,
                        control: "moveto".to_owned(),
                    });
                }
                (true, None)
            }
            // The text at the cursor is proposed as the name [obs, 702/s5].
            ListOutcome::Add => {
                let name = self
                    .cursor
                    .buffer
                    .line_at_offset(self.cursor.offset)
                    .map(|line| line.text.trim().to_owned())
                    .unwrap_or_default();
                self.dialog_count += 1;
                let dialog = NameDialog::add(self.dialog_count, &name);
                self.show(dialog.spec());
                state.child = Some(Child::Name(dialog, None));
                (true, None)
            }
            ListOutcome::Change(row) => {
                let details = state
                    .markers
                    .get(row)
                    .and_then(|marker| self.details_of(&state.page, marker));
                let Some(details) = details else {
                    return (true, None);
                };
                self.dialog_count += 1;
                let dialog = NameDialog::change(self.dialog_count, &details);
                self.show(dialog.spec());
                state.child = Some(Child::Name(dialog, Some(row)));
                (true, None)
            }
            ListOutcome::RemoveAll => {
                self.dialog_count += 1;
                let dialog = RemoveAllDialog {
                    id: self.dialog_count,
                };
                self.show(dialog.spec());
                state.child = Some(Child::RemoveAll(dialog));
                (true, None)
            }
            ListOutcome::MoveTo(row) => {
                let offset = self
                    .page_markers(&state.page)
                    .into_iter()
                    .nth(row)
                    .map(|(_, _, offset)| offset);
                self.close_list(state);
                let Some(offset) = offset else {
                    return (false, None);
                };
                self.cursor.move_to(offset);
                (false, self.say(self.cursor.current_line()))
            }
            // The list again, without it.
            ListOutcome::Remove(row) => {
                if let Some(marker) = state.markers.get(row).cloned() {
                    self.forget(&state.page, &marker);
                }
                self.close(id);
                self.open_placemarker_list();
                (false, None)
            }
            ListOutcome::Closed => {
                self.returning_from_dialog();
                (false, None)
            }
        }
    }

    /// Add or Change, and the Remove All question; whether the list is still open.
    fn child_event(
        &mut self,
        state: &mut ListState,
        mut child: Child,
        message: &FromUi,
    ) -> (bool, Option<String>) {
        match &mut child {
            Child::Name(dialog, row) => match dialog.handle(message) {
                NameOutcome::Nothing => {
                    state.child = Some(child);
                    (true, None)
                }
                NameOutcome::Closed => {
                    self.close(dialog.id);
                    (true, None)
                }
                // "Anchor to text" makes its text field available.
                NameOutcome::Anchoring(anchored) => {
                    if let Some(ui) = &self.ui {
                        ui.send(ToUi::DialogEnable {
                            id: dialog.id,
                            control: ANCHOR_TEXT.to_owned(),
                            enabled: anchored,
                        });
                    }
                    state.child = Some(child);
                    (true, None)
                }
                NameOutcome::Done(details) => {
                    match row.and_then(|row| state.markers.get(row).cloned()) {
                        Some(marker) => self.change(&state.page, &marker, details),
                        None => self.add_placemarker(&state.page, details),
                    }
                    // OK closes the list too [obs, 702/s6].
                    self.close(dialog.id);
                    self.close_list(state);
                    (false, None)
                }
            },
            Child::RemoveAll(dialog) => match dialog.handle(message) {
                RemoveAllOutcome::Nothing => {
                    state.child = Some(child);
                    (true, None)
                }
                RemoveAllOutcome::No => {
                    self.close(dialog.id);
                    (true, None)
                }
                RemoveAllOutcome::Yes => {
                    self.placemarkers.remove_all(&state.page);
                    self.temp_placemarkers.remove(&state.page);
                    self.save_placemarkers();
                    // Yes closes the list too [obs, 702/s28].
                    self.close(dialog.id);
                    self.close_list(state);
                    (false, None)
                }
            },
        }
    }

    /// Shows one of Okur's dialogs.
    fn show(&self, dialog: crate::ui::DialogSpec) {
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogShow { dialog });
        }
    }

    /// Closes one of Okur's dialogs.
    fn close(&self, id: u32) {
        if let Some(ui) = &self.ui {
            ui.send(ToUi::DialogClose { id });
        }
    }

    /// Closes the list, the focus going back to the page quietly [obs, 702/s6, s28].
    fn close_list(&mut self, state: &ListState) {
        self.close(state.dialog.id);
        self.returning_from_dialog();
    }

    /// A new PlaceMarker at the virtual cursor, as Add left it, kept for the page.
    fn add_placemarker(&mut self, page: &str, details: Details) {
        let marker = PlaceMarker {
            name: details.name,
            offset: self.cursor.offset,
            anchor: details.anchor,
            domain: details.domain,
        };
        self.placemarkers.add(page, marker);
        self.save_placemarkers();
    }

    /// A saved PlaceMarker as Change shows it; the temporary one has none to change.
    fn details_of(&self, page: &str, marker: &Marker) -> Option<Details> {
        let (page, index) = match marker {
            Marker::Own(index) => (crate::placemarkers::page_of(page), *index),
            Marker::Site { page, index } => (page.clone(), *index),
            Marker::Temporary => return None,
        };
        let saved = self.placemarkers.pages.get(&page)?.get(index)?;
        Some(Details {
            name: saved.name.clone(),
            domain: saved.domain,
            anchor: saved.anchor.clone(),
        })
    }

    fn change(&mut self, page: &str, marker: &Marker, details: Details) {
        if let Some(saved) = self.saved_mut(page, marker) {
            saved.name = details.name;
            saved.domain = details.domain;
            saved.anchor = details.anchor;
            self.save_placemarkers();
        }
    }

    fn forget(&mut self, page: &str, marker: &Marker) {
        match marker {
            Marker::Own(index) => {
                self.placemarkers.remove(page, *index);
            }
            Marker::Site { page, index } => {
                self.placemarkers.remove(page, *index);
            }
            Marker::Temporary => {
                self.temp_placemarkers.remove(page);
            }
        }
        self.save_placemarkers();
    }

    fn saved_mut(&mut self, page: &str, marker: &Marker) -> Option<&mut PlaceMarker> {
        let (page, index) = match marker {
            Marker::Own(index) => (crate::placemarkers::page_of(page), *index),
            Marker::Site { page, index } => (page.clone(), *index),
            Marker::Temporary => return None,
        };
        self.placemarkers.pages.get_mut(&page)?.get_mut(index)
    }

    fn save_placemarkers(&self) {
        if let Some(store) = &self.placemarker_store {
            store.save(&self.placemarkers);
        }
    }
}
