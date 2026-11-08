//! What okur-linux's event loop keeps of the focus outside a page, to tell a focus move from a
//! toolkit's repeats, and a control's own change from a new focus. Each rule came from a live
//! run of SR Desktop Fixture's GTK replica in the Linux lab (2026-11-08):
//! - GTK 4 sends a focus event with its Focus and another with its focused state, and repeated
//!   it as the window was reactivated around each key: a focus event for the object already
//!   focused, read as before (its role and name), is passed over, so a change (Space on a check
//!   box [obs, 304/s1]) is compared with the state last said;
//! - read otherwise, it is said: GTK's notebook keeps the focus as Right selects its next tab
//!   [obs, 303/s13];
//! - another window activated forgets the object, so a focus coming back from elsewhere is said;
//!   the same window activated again keeps it;
//! - something changing within the focused control (a label renamed, an object selected without
//!   the focus) has it read again: said as a focus when it reads as another control, as its
//!   value changing when only that changed (GTK's drop-down's item [obs, 304/s9]).

use okur_core::aom::Role;
use okur_core::event::FocusEvent;

/// The focus as the event loop last told the engine of it. Objects and windows are named by
/// their AT-SPI bus name and path.
#[derive(Debug, Default)]
pub struct FocusTrack {
    /// The object that took the focus last, in a page or outside one.
    object: Option<String>,
    /// The window activated last.
    window: Option<String>,
    /// What the focus outside a page was said as: its role and name; none in a page.
    said: Option<(Role, Option<String>)>,
    /// The value it was said with.
    value: Option<String>,
}

/// What the focused control, read again, is to the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reread {
    /// Another control: said as a focus.
    Focus,
    /// The same control with another value: said as its value changing.
    Value,
    /// As it was: nothing to say.
    Same,
}

impl FocusTrack {
    /// `window` activated: another window than the last forgets the focused object.
    pub fn window_activated(&mut self, window: Option<String>) {
        if window != self.window {
            self.object = None;
        }
        self.window = window;
    }

    /// A focus event for `object` outside a page, read as `read`: whether it is said. The object
    /// already focused, read as before, is the toolkit's repeat.
    pub fn native_focus(&mut self, object: Option<String>, read: &FocusEvent) -> bool {
        let again = object.is_some() && object == self.object;
        self.object = object;
        let said = (read.role.clone(), read.name.clone());
        if again && self.said.as_ref() == Some(&said) {
            return false;
        }
        self.said = Some(said);
        self.value.clone_from(&read.value);
        true
    }

    /// A focus event for `object` in a page: no control outside one has the focus.
    pub fn page_focus(&mut self, object: Option<String>) {
        self.object = object;
        self.said = None;
        self.value = None;
    }

    /// Whether a control outside a page has the focus.
    pub fn is_native(&self) -> bool {
        self.said.is_some()
    }

    /// The focused control read again as `read`, as something within it changed.
    pub fn reread(&mut self, read: &FocusEvent) -> Reread {
        let said = (read.role.clone(), read.name.clone());
        if self.said.as_ref() != Some(&said) {
            self.said = Some(said);
            self.value.clone_from(&read.value);
            Reread::Focus
        } else if read.value != self.value {
            self.value.clone_from(&read.value);
            Reread::Value
        } else {
            Reread::Same
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(role: Role, name: &str, value: Option<&str>) -> FocusEvent {
        let mut event = FocusEvent::new(role);
        event.name = Some(name.to_owned());
        event.value = value.map(str::to_owned);
        event
    }

    fn object(path: &str) -> Option<String> {
        Some(format!(":1.5/org/okur/a11y/{path}"))
    }

    /// GTK's repeat of the focus is passed over; the check box's change says the rest
    /// [obs, 304/s1].
    #[test]
    fn the_same_object_read_as_before_is_a_repeat() {
        let mut track = FocusTrack::default();
        let check = read(Role::CheckBox, "Subscribe to updates", None);
        assert!(track.native_focus(object("check"), &check));
        let mut unchecked = check.clone();
        unchecked.states.checked = Some(false);
        assert!(!track.native_focus(object("check"), &unchecked));
        // Another object is a move, even read alike.
        assert!(track.native_focus(object("other"), &check));
    }

    /// GTK's notebook keeps the focus as Right selects its next tab [obs, 303/s13].
    #[test]
    fn the_same_object_read_otherwise_is_said() {
        let mut track = FocusTrack::default();
        assert!(track.native_focus(object("notebook"), &read(Role::Tab, "General", None)));
        assert!(track.native_focus(object("notebook"), &read(Role::Tab, "Advanced", None)));
    }

    /// The same window activated again (the hook's grabs once did it around every key) keeps
    /// the focus; another window's activation forgets it.
    #[test]
    fn only_another_window_forgets_the_focus() {
        let mut track = FocusTrack::default();
        let name = read(Role::Edit, "Name:", Some("Ada"));
        track.window_activated(object("fixture"));
        assert!(track.native_focus(object("name"), &name));
        track.window_activated(object("fixture"));
        assert!(!track.native_focus(object("name"), &name));
        track.window_activated(object("terminal"));
        track.window_activated(object("fixture"));
        assert!(track.native_focus(object("name"), &name));
    }

    /// A focus in a page between two outside it: the second is said.
    #[test]
    fn a_page_between_forgets_the_control() {
        let mut track = FocusTrack::default();
        let button = read(Role::Button, "OK", None);
        assert!(track.native_focus(object("ok"), &button));
        track.page_focus(object("link"));
        assert!(!track.is_native());
        assert!(track.native_focus(object("ok"), &button));
        assert!(track.is_native());
    }

    /// Read again as something within it changes: another control is a focus, another value a
    /// value change [obs, 304/s9: GTK's drop-down's "Brazil"], the same nothing.
    #[test]
    fn a_control_read_again_says_what_changed() {
        let mut track = FocusTrack::default();
        assert!(track.native_focus(
            object("country"),
            &read(Role::ComboBox, "Country:", Some("Germany"))
        ));
        assert_eq!(
            track.reread(&read(Role::ComboBox, "Country:", Some("Germany"))),
            Reread::Same
        );
        assert_eq!(
            track.reread(&read(Role::ComboBox, "Country:", Some("Brazil"))),
            Reread::Value
        );
        assert_eq!(
            track.reread(&read(Role::ComboBox, "Country:", Some("Brazil"))),
            Reread::Same
        );
        assert_eq!(
            track.reread(&read(Role::Tab, "Advanced", None)),
            Reread::Focus
        );
    }
}
