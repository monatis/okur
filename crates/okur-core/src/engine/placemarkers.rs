//! PlaceMarkers: places in a page to come back to with K and Shift+K (JAWS 2025 help, Surf's
//! Up chapter 15). The temporary PlaceMarker, Ctrl+Windows+K, is one per page for the
//! session: "Temporary PlaceMarker set", then the line [obs, 687/s4]. Saved PlaceMarkers are
//! kept per page from one visit to the next ([`crate::placemarkers`]). K and Shift+K go to
//! the next or prior marker, saved or temporary, and say the element there, "Maria Lopez
//! same page link"; with one marker K says it again, without a wrapping message [obs,
//! 687/s6-s8]. A page without one says "There are no PlaceMarkers on this page." [obs,
//! 221/s4-s5].

use super::{AppMode, ScreenReaderEngine};
use crate::messages::msg;
use crate::placemarkers::{place_in, PlaceMarkerStore, PlaceMarkers};

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

    /// Where the page's markers are: its saved ones, found in its text as it is now, and its
    /// temporary one.
    fn placemarker_offsets(&self) -> Vec<usize> {
        let page = self.placemarker_page();
        let buffer = &self.cursor.buffer;
        self.placemarkers
            .on_page(&page)
            .into_iter()
            .map(|marker| place_in(buffer, marker))
            .chain(self.temp_placemarkers.get(&page).copied())
            .collect()
    }

    /// K or Shift+K: the next or prior PlaceMarker of the page.
    pub(super) fn move_to_placemarker(&mut self, forward: bool) -> Option<String> {
        let markers = self.placemarker_offsets();
        // Said by quick navigation, as the other elements are.
        Some(
            self.cursor
                .move_to_placemarker(&markers, forward)
                .unwrap_or_else(|| msg::quicknav_none_placemarkers().to_string()),
        )
    }
}
