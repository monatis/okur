//! The Linux driver's events as the engine's (docs/plans/2026-11-08-structure-for-three-platforms.md,
//! step 1): what okur-linux's event loop decides, made from what it read for each AT-SPI event (an
//! [`AtspiRecord`]), with no call to Linux, so it is tested on every OS and a live run's records
//! can be replayed as Windows' traces are. The driver asks the [`Converter`] what to read
//! ([`Converter::is_twin`], [`Converter::wants_page`], [`Converter::wants_native`],
//! [`Converter::wants_reread`]), reads it, and hands it the record ([`Converter::convert`]).

use okur_core::event::{AccessibilityEvent, ChangeKind, FocusEvent};
use serde::{Deserialize, Serialize};
use xa11y_core::{ElementData, EventKind, Role};

use crate::focus::{focus_event, in_active_window, native_focus};
use crate::focus_track::{is_control_change, is_inside_change, FocusTrack, Reread};
use crate::page::{to_aom_from, AtspiNode};

/// A control outside a page as the driver read it: its subtree, and its ancestors' from its
/// parent up to its window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeRead {
    pub focus: AtspiNode,
    pub ancestors: Vec<AtspiNode>,
}

/// A page read for the engine: its document's key, the base its objects are numbered from, its
/// tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageRead {
    pub document: String,
    pub base: i32,
    pub root: AtspiNode,
}

/// What the driver read for one AT-SPI event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtspiRecord {
    /// Milliseconds since the event source started.
    pub at_ms: u64,
    pub kind: EventKind,
    /// The application, by its AT-SPI name.
    pub app: String,
    /// The event's object, as the event's snapshot has it.
    pub target: Option<ElementData>,
    /// The page's document the target is in, by its key, where it is in one.
    pub document: Option<String>,
    /// The pages read: the focus's, new to the engine, and a page that finished loading.
    pub pages: Vec<PageRead>,
    /// The target read with what surrounds it, a focus or a control's change outside a page.
    pub native: Option<NativeRead>,
    /// The focused control read again, as something within it changed.
    pub reread: Option<NativeRead>,
}

/// An object's key, its bus name and path, as an element's raw data names them.
pub fn object_key(element: &ElementData) -> Option<String> {
    let raw = |key: &str| element.raw.get(key).and_then(|v| v.as_str());
    Some(format!("{}{}", raw("bus_name")?, raw("object_path")?))
}

/// Whether the event's object has the focus.
fn focused(target: Option<&ElementData>) -> bool {
    target.is_some_and(|e| e.states.focused)
}

/// The event loop's state: the focus as the engine was last told of it, the page read last,
/// and the control outside a page that has the focus.
#[derive(Debug, Default)]
pub struct Converter {
    track: FocusTrack,
    /// The document of the page read last.
    last_page: Option<String>,
    /// The focused control outside a page, as its focus event's snapshot had it.
    native_element: Option<ElementData>,
}

impl Converter {
    /// Whether the event is GTK's second focus event for one move, passed over before anything
    /// is read.
    pub fn is_twin(&mut self, kind: &EventKind, target: Option<&ElementData>, at_ms: u64) -> bool {
        *kind == EventKind::FocusChanged
            && self
                .track
                .is_twin(target.and_then(object_key).as_deref(), at_ms)
    }

    /// Whether the page whose document is `document` is read as the focus lands in it: on its
    /// document (a load, or Tab coming back to it), or in a page other than the one read last.
    pub fn wants_page(&self, on_document: bool, document: &str) -> bool {
        on_document || self.last_page.as_deref() != Some(document)
    }

    /// Whether the event's object is read with what surrounds it: a focus, or the focused
    /// control's change, outside a page.
    pub fn wants_native(kind: &EventKind, target: Option<&ElementData>, in_page: bool) -> bool {
        target.is_some()
            && !in_page
            && (*kind == EventKind::FocusChanged || is_control_change(kind, focused(target)))
    }

    /// The focused control to read again, as something within it changed in its application.
    pub fn wants_reread(
        &self,
        kind: &EventKind,
        target: Option<&ElementData>,
    ) -> Option<&ElementData> {
        let object = target.and_then(object_key);
        let inside = is_inside_change(kind, focused(target))
            && self.track.in_focused_application(object.as_deref());
        self.native_element.as_ref().filter(|_| inside)
    }

    /// The engine's events for what was read for one event, in order: the pages read, then the
    /// focus or the change.
    pub fn convert(&mut self, record: &AtspiRecord) -> Vec<AccessibilityEvent> {
        let mut events = Vec::new();
        for page in &record.pages {
            self.last_page = Some(page.document.clone());
            let root = to_aom_from(&page.root, page.base);
            let title = root.name.clone().unwrap_or_default();
            events.push(AccessibilityEvent::DocumentLoaded { title, root });
        }
        let target = record.target.as_ref();
        let object = target.and_then(object_key);
        match record.kind {
            EventKind::WindowActivated => self.track.window_activated(object.clone()),
            EventKind::WindowDeactivated => self.track.window_deactivated(),
            _ => {}
        }
        let app = Some(record.app.clone()).filter(|a| !a.is_empty());
        let focus = record.kind == EventKind::FocusChanged;
        let changed = is_control_change(&record.kind, focused(target));
        let in_page = record.document.is_some();
        let outside_page = (focus || changed) && !in_page;
        // A focus outside a page, and its control's change, read with what surrounds it: the
        // change names the control as its focus did.
        let native = target
            .filter(|_| outside_page)
            .and_then(|element| match &record.native {
                Some(read) => native_control(element, read, app.clone()),
                // Not read (its object unnamed): as its event's snapshot has it.
                None => {
                    let mut control = focus_event(element);
                    control.app = app.clone();
                    Some(control)
                }
            })
            .map(|control| {
                if changed {
                    AccessibilityEvent::Change {
                        kind: ChangeKind::State,
                        control,
                    }
                } else {
                    AccessibilityEvent::Focus(control)
                }
            });
        if let Some(AccessibilityEvent::Focus(read)) = &native {
            if !self.track.native_focus(object.clone(), read) {
                // The toolkit's repeat of the focus: nothing more to say.
                return events;
            }
            self.native_element = record.target.clone();
        } else if focus && in_page {
            self.track.page_focus(object.clone());
            self.native_element = None;
        }
        // The focused control read again: said as a focus when it reads as another control
        // (GTK's notebook's next tab [obs, 303/s13]), as its value changing when only that
        // changed (GTK's drop-down's item [obs, 304/s9]).
        if let (Some(element), Some(read)) = (self.native_element.clone(), &record.reread) {
            let Some(control) = native_control(&element, read, app.clone()) else {
                return events;
            };
            match self.track.reread(&control) {
                Reread::Focus => events.push(AccessibilityEvent::Focus(control)),
                Reread::Value => events.push(AccessibilityEvent::Change {
                    kind: ChangeKind::Value,
                    control,
                }),
                Reread::Same => {}
            }
        }
        // Outside a page the native reading alone: none is a focus in a window behind.
        let converted = if outside_page && target.is_some() {
            native
        } else {
            native.or_else(|| plain(record, in_page, app))
        };
        events.extend(converted);
        events
    }
}

/// A control outside a page read with what surrounds it, as the engine's focus event; none in a
/// window behind the active one, which is not said.
fn native_control(
    element: &ElementData,
    read: &NativeRead,
    app: Option<String>,
) -> Option<FocusEvent> {
    if !in_active_window(&read.ancestors) {
        return None;
    }
    let mut control = native_focus(focus_event(element), &read.focus, &read.ancestors);
    control.app = app;
    Some(control)
}

/// A focus, or the focused control's change, as its event's snapshot has it: a page's (`in_page`,
/// where the virtual cursor reads and takes the keys, as the focus chain that holds a document
/// tells it on Windows).
fn plain(record: &AtspiRecord, in_page: bool, app: Option<String>) -> Option<AccessibilityEvent> {
    let element = record.target.as_ref()?;
    let mut control = focus_event(element);
    control.in_document = in_page;
    control.app = app;
    match record.kind {
        EventKind::FocusChanged => Some(AccessibilityEvent::Focus(control)),
        _ if is_control_change(&record.kind, element.states.focused) => {
            Some(AccessibilityEvent::Change {
                kind: ChangeKind::State,
                control,
            })
        }
        _ => None,
    }
}

/// Whether `element` is a page's document (Chrome's web area).
pub fn is_document(element: &ElementData) -> bool {
    element.role == Role::WebArea
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::state;
    use okur_core::aom::Role as OkurRole;
    use xa11y_core::{Role as ElementRole, StateFlag};

    fn element(role: ElementRole, name: &str, path: &str, focused: bool) -> ElementData {
        let mut e = ElementData::for_role(role);
        e.name = Some(name.to_owned());
        e.states.focused = focused;
        e.raw.insert(
            "bus_name".to_owned(),
            serde_json::Value::String(":1.5".to_owned()),
        );
        e.raw.insert(
            "object_path".to_owned(),
            serde_json::Value::String(path.to_owned()),
        );
        e
    }

    fn node(role: &str, name: &str, states: &[u32]) -> AtspiNode {
        let mut n = AtspiNode {
            role: role.to_owned(),
            name: name.to_owned(),
            ..AtspiNode::default()
        };
        for s in states {
            n.states |= 1u64 << s;
        }
        n
    }

    fn window() -> AtspiNode {
        node("frame", "SR Desktop Fixture", &[state::ACTIVE])
    }

    fn record(kind: EventKind, target: ElementData, native: Option<NativeRead>) -> AtspiRecord {
        AtspiRecord {
            at_ms: 0,
            kind,
            app: "desktop-forms.py".to_owned(),
            target: Some(target),
            document: None,
            pages: Vec::new(),
            native,
            reread: None,
        }
    }

    fn read(focus: AtspiNode) -> NativeRead {
        NativeRead {
            focus,
            ancestors: vec![window()],
        }
    }

    /// A focus outside a page is said once, with its application; GTK's repeat is passed over;
    /// Space's check is said as the control's change [obs, 304/s1].
    #[test]
    fn a_native_focus_its_repeat_and_its_change() {
        let mut converter = Converter::default();
        let mut check = element(ElementRole::CheckBox, "Remember me", "/check", true);
        check.states.checked = Some(xa11y_core::Toggled::Off);
        let focus = record(
            EventKind::FocusChanged,
            check.clone(),
            Some(read(node("check box", "Remember me", &[]))),
        );
        let events = converter.convert(&focus);
        assert!(matches!(
            events.as_slice(),
            [AccessibilityEvent::Focus(f)] if f.role == OkurRole::CheckBox
                && f.app.as_deref() == Some("desktop-forms.py")
        ));
        assert!(converter.convert(&focus).is_empty());
        let checked = record(
            EventKind::StateChanged {
                flag: StateFlag::Checked,
                value: true,
            },
            check,
            Some(read(node("check box", "Remember me", &[state::CHECKED]))),
        );
        assert!(matches!(
            converter.convert(&checked).as_slice(),
            [AccessibilityEvent::Change { kind: ChangeKind::State, control }]
                if control.states.checked == Some(true)
        ));
    }

    /// A focus in a window behind the active one is not said.
    #[test]
    fn a_focus_in_a_window_behind_is_not_said() {
        let mut converter = Converter::default();
        let button = element(ElementRole::Button, "OK", "/ok", true);
        let behind = NativeRead {
            focus: node("push button", "OK", &[]),
            ancestors: vec![node("frame", "Chat", &[])],
        };
        let events = converter.convert(&record(EventKind::FocusChanged, button, Some(behind)));
        assert!(events.is_empty());
    }

    /// A label renamed within the focused drop-down is read again and said as its value
    /// [obs, 304/s9: "Brazil"]; the converter asks for the read only in the focus's application.
    #[test]
    fn a_change_within_the_focused_control_is_read_again() {
        let mut converter = Converter::default();
        let country = element(ElementRole::ComboBox, "Country:", "/country", true);
        let shown = |item: &str| {
            let mut combo = node("combo box", "Country:", &[]);
            combo
                .attributes
                .insert("valuetext".to_owned(), item.to_owned());
            combo.text = Some(item.to_owned());
            combo
        };
        let _ = converter.convert(&record(
            EventKind::FocusChanged,
            country.clone(),
            Some(read(shown("Germany"))),
        ));
        let label = element(ElementRole::StaticText, "Brazil", "/label", false);
        assert!(converter
            .wants_reread(&EventKind::NameChanged, Some(&label))
            .is_some());
        let mut other = element(ElementRole::StaticText, "12:00", "/clock", false);
        other.raw.insert(
            "bus_name".to_owned(),
            serde_json::Value::String(":1.9".to_owned()),
        );
        assert!(converter
            .wants_reread(&EventKind::NameChanged, Some(&other))
            .is_none());
    }

    /// A page read is given to the engine first, numbered from its base; the page read last is
    /// not read again for a focus in it, its document's focus is.
    #[test]
    fn a_page_read_is_given_first_and_remembered() {
        let mut converter = Converter::default();
        let link = element(ElementRole::Link, "News", "/link", true);
        let mut page = record(EventKind::FocusChanged, link, None);
        page.document = Some(":1.5/doc".to_owned());
        page.pages = vec![PageRead {
            document: ":1.5/doc".to_owned(),
            base: 1 << 20,
            root: node("document web", "News", &[]),
        }];
        let events = converter.convert(&page);
        assert!(matches!(
            events.as_slice(),
            [AccessibilityEvent::DocumentLoaded { title, .. }, AccessibilityEvent::Focus(f)]
                if title == "News" && f.in_document
        ));
        assert!(!converter.wants_page(false, ":1.5/doc"));
        assert!(converter.wants_page(true, ":1.5/doc"));
        assert!(converter.wants_page(false, ":1.5/other"));
    }
}
