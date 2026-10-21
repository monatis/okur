//! PlaceMarkers: places in a page to come back to with K and Shift+K (JAWS 2025 help, Surf's
//! Up chapter 15). The temporary PlaceMarker, Ctrl+Windows+K, is one per page for the
//! session: "Temporary PlaceMarker set", then the line [obs, 687/s4]. K and Shift+K go to the
//! next or prior marker and say the element there, "Maria Lopez same page link"; with one
//! marker K says it again, without a wrapping message [obs, 687/s6-s8]. A page without one
//! says "There are no PlaceMarkers on this page." [obs, 221/s4-s5].

use super::{AppMode, ScreenReaderEngine};
use crate::messages::msg;

impl ScreenReaderEngine {
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

    /// K or Shift+K: the next or prior PlaceMarker of the page.
    pub(super) fn move_to_placemarker(&mut self, forward: bool) -> Option<String> {
        let markers: Vec<usize> = self
            .temp_placemarkers
            .get(&self.placemarker_page())
            .copied()
            .into_iter()
            .collect();
        // Said by quick navigation, as the other elements are.
        Some(
            self.cursor
                .move_to_placemarker(&markers, forward)
                .unwrap_or_else(|| msg::quicknav_none_placemarkers().to_string()),
        )
    }
}
