//! What okur-linux's event loop keeps of the focus outside a page, to tell a focus move from a
//! toolkit's repeats, and a control's own change from a new focus. Each rule came from a live
//! run of SR Desktop Fixture's GTK replica in the Linux lab (2026-11-08):
//! - GTK 4 sends a focus event with its Focus and another with its focused state, and repeated
//!   it as the window was reactivated around each key: a focus event for the object already
//!   focused, read as before (its role and name), is passed over, so a change (Space on a check
//!   box [obs, 304/s1]) is compared with the state last said;
//! - read otherwise, it is said: GTK's notebook keeps the focus as Right selects its next tab
//!   [obs, 303/s13];
//! - a window left forgets the object, so the focus is said as the user comes back to it, and
//!   another window activated too (the hook once made GTK leave and enter its window around
//!   every key: that was the hook's, fixed in eb0f0ced);
//! - something changing within the focused control (a label renamed, an object selected without
//!   the focus) has it read again: said as a focus when it reads as another control, as its
//!   value changing when only that changed (GTK's drop-down's item [obs, 304/s9]); a change in
//!   another application (a clock's label, a browser's tab title) is not the focus's.

use okur_core::aom::Role;
use okur_core::event::FocusEvent;
use xa11y_core::{EventKind, StateFlag};

/// Whether okur-linux's event loop takes events of `kind`: the focus, a control's checked,
/// selected or expanded state, a name, a page loaded, a window activated or left. The others are
/// dropped before their object is read (xa11y-linux's `subscribe_desktop_kinds`): a page loading
/// sends thousands of structure and text signals (the code review of 2026-11-08).
pub fn is_heard(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::FocusChanged
            | EventKind::NameChanged
            | EventKind::ValueChanged
            | EventKind::DocumentLoaded
            | EventKind::WindowActivated
            | EventKind::WindowDeactivated
            | EventKind::StateChanged {
                flag: StateFlag::Checked | StateFlag::Selected | StateFlag::Expanded,
                ..
            }
    )
}

/// Whether an event of `kind` on an object (`focused`: the object has the focus) is a change of
/// the focused control the engine is told of: its checked, selected or expanded state, as
/// Windows' state change gives it (Space on a check box [obs, 304/s1]).
pub fn is_control_change(kind: &EventKind, focused: bool) -> bool {
    focused
        && matches!(
            kind,
            EventKind::StateChanged {
                flag: StateFlag::Checked | StateFlag::Selected | StateFlag::Expanded,
                ..
            }
        )
}

/// Whether an event of `kind` changes something within the focused control without moving
/// the focus, for the control to be read again: a label renamed (GTK's drop-down's item [obs,
/// 304/s9]), an object selected without the focus (GTK's notebook's tab [obs, 303/s13]).
pub fn is_inside_change(kind: &EventKind, focused: bool) -> bool {
    *kind == EventKind::NameChanged
        || (!focused
            && *kind
                == EventKind::StateChanged {
                    flag: StateFlag::Selected,
                    value: true,
                })
}

/// The focus as the event loop last told the engine of it. Objects and windows are named by
/// their AT-SPI bus name and path.
#[derive(Debug, Default)]
pub struct FocusTrack {
    /// The object that took the focus last, in a page or outside one.
    object: Option<String>,
    /// The window activated last.
    window: Option<String>,
    /// The object of the last focus event and when it came, in milliseconds.
    last_event: Option<(String, u64)>,
    /// What the focus outside a page was said as: its role and name; none in a page.
    said: Option<(Role, Option<String>)>,
    /// The value it was said with.
    value: Option<String>,
}

/// How close a focus event's twin follows it, in milliseconds.
const TWIN_MS: u64 = 100;

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
    /// The window activated last, by its object.
    pub fn active_window(&self) -> Option<&str> {
        self.window.as_deref()
    }

    pub fn window_activated(&mut self, window: Option<String>) {
        if window != self.window {
            self.object = None;
        }
        self.window = window;
    }

    /// Whether a focus event for `object` at `at_ms` is the twin of the one before it: GTK 4
    /// sends one with its Focus and one with its focused state, at once, and each was read in
    /// full before the second was found a repeat (the code review of 2026-11-08). A focus coming
    /// back to the same object later is no twin.
    pub fn is_twin(&mut self, object: Option<&str>, at_ms: u64) -> bool {
        let twin = object.is_some_and(|o| {
            self.last_event
                .as_ref()
                .is_some_and(|(last, at)| last == o && at_ms.saturating_sub(*at) <= TWIN_MS)
        });
        self.last_event = object.map(|o| (o.to_owned(), at_ms));
        twin
    }

    /// The active window left: the focus coming back to the same object is said.
    pub fn window_deactivated(&mut self) {
        self.object = None;
        self.window = None;
    }

    /// Whether `object` (an event's) is in the application that has the focus outside a page:
    /// its bus name is the focused object's.
    pub fn in_focused_application(&self, object: Option<&str>) -> bool {
        let bus = |o: &str| o.split('/').next().map(str::to_owned);
        self.is_native()
            && object
                .and_then(bus)
                .is_some_and(|b| self.object.as_deref().and_then(bus).is_some_and(|f| f == b))
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
        // A control read again with no name, where the one said had one, is going away: the
        // wizard's old list row, still focused and emptied as its page goes, said only "To move
        // to an item press the Arrow keys." (742/s9 in night3, at 6cf0846f). Nothing is said;
        // the next page's focus follows.
        let nameless = read.name.as_deref().is_none_or(|n| n.trim().is_empty());
        let named_before = self
            .said
            .as_ref()
            .and_then(|(_, name)| name.as_deref())
            .is_some_and(|n| !n.trim().is_empty());
        if nameless && named_before {
            return Reread::Same;
        }
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

    /// Every kind the event loop acts on is heard; a structure change or a focus state is not.
    #[test]
    fn the_loop_hears_what_it_acts_on() {
        let checked = EventKind::StateChanged {
            flag: StateFlag::Checked,
            value: true,
        };
        let selected = EventKind::StateChanged {
            flag: StateFlag::Selected,
            value: true,
        };
        for kind in [
            EventKind::FocusChanged,
            EventKind::NameChanged,
            checked,
            selected,
        ] {
            assert!(is_heard(&kind));
        }
        assert!(is_heard(&EventKind::DocumentLoaded));
        assert!(is_heard(&EventKind::WindowDeactivated));
        assert!(!is_heard(&EventKind::StructureChanged));
        assert!(!is_heard(&EventKind::StateChanged {
            flag: StateFlag::Focused,
            value: true,
        }));
    }

    /// Space on a check box changes the focused control; its focus state, or another object's
    /// check, does not.
    #[test]
    fn a_control_change_is_the_focused_objects_state() {
        let checked = EventKind::StateChanged {
            flag: StateFlag::Checked,
            value: false,
        };
        assert!(is_control_change(&checked, true));
        assert!(!is_control_change(&checked, false));
        let focused = EventKind::StateChanged {
            flag: StateFlag::Focused,
            value: true,
        };
        assert!(!is_control_change(&focused, true));
    }

    /// A label renamed, or a tab selected while the notebook keeps the focus, is within the
    /// focused control; the focused item selected is its own change, one let go nothing.
    #[test]
    fn an_inside_change_leaves_the_focus_where_it_is() {
        let selected = EventKind::StateChanged {
            flag: StateFlag::Selected,
            value: true,
        };
        assert!(is_inside_change(&EventKind::NameChanged, false));
        assert!(is_inside_change(&selected, false));
        assert!(!is_inside_change(&selected, true));
        let unselected = EventKind::StateChanged {
            flag: StateFlag::Selected,
            value: false,
        };
        assert!(!is_inside_change(&unselected, false));
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

    /// Leaving the window and coming back says the focus again, as JAWS says the window and its
    /// focus; another window's activation forgets it too.
    #[test]
    fn a_window_left_forgets_the_focus() {
        let mut track = FocusTrack::default();
        let name = read(Role::Edit, "Name:", Some("Ada"));
        track.window_activated(object("fixture"));
        assert!(track.native_focus(object("name"), &name));
        assert!(!track.native_focus(object("name"), &name));
        track.window_deactivated();
        track.window_activated(object("fixture"));
        assert!(track.native_focus(object("name"), &name));
        track.window_activated(object("terminal"));
        track.window_activated(object("fixture"));
        assert!(track.native_focus(object("name"), &name));
    }

    /// GTK's two focus events for one move are one; the same object focused again later is not
    /// a twin, nor another object at once.
    #[test]
    fn a_focus_events_twin_is_one_move() {
        let mut track = FocusTrack::default();
        let check = object("check");
        assert!(!track.is_twin(check.as_deref(), 1_000));
        assert!(track.is_twin(check.as_deref(), 1_030));
        assert!(!track.is_twin(check.as_deref(), 2_000));
        assert!(!track.is_twin(object("other").as_deref(), 2_010));
        assert!(!track.is_twin(None, 2_020));
    }

    /// A label renamed in another application (a clock, a browser's tab) is not within the
    /// focused control.
    #[test]
    fn only_the_focused_application_changes_the_focus() {
        let mut track = FocusTrack::default();
        assert!(!track.in_focused_application(Some(":1.5/org/a11y/label")));
        assert!(track.native_focus(object("country"), &read(Role::ComboBox, "Country:", None)));
        assert!(track.in_focused_application(Some(":1.5/org/a11y/label")));
        assert!(!track.in_focused_application(Some(":1.9/org/a11y/clock")));
        assert!(!track.in_focused_application(None));
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
