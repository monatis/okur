//! The converter's state across events (`EventConverter`): which events reach the engine and
//! as what, menus, live regions, notifications and toasts, and the focus repeats it leaves
//! out.

use super::*;

/// Whether a WinEvent focus had moved on by the time it was read: the keyboard focus was then
/// outside the event's window (`TraceEvent::focus_elsewhere`), and MSAA no longer says its
/// object is focused. File Explorer's list as Tab passes it on the way to a link [obs,
/// 3156/s6]. A window whose child has the focus is not one: Notepad's window coming to the
/// front with the focus in its edit, a dialog with the focus in its list.
pub(super) fn left_before_read(event: &TraceEvent) -> bool {
    const STATE_FOCUSED: u32 = 0x4;
    event.event == "focus"
        && event.focus_elsewhere
        && event
            .object
            .as_ref()
            .is_some_and(|o| o.states & STATE_FOCUSED == 0)
}

/// Whether a WinEvent focus is an editable combo box's, which gave the focus to its edit
/// before the event was read while UI Automation still gave the combo box: MSAA does not say
/// the combo box is focused, the keyboard focus is in an edit, and the focus chain ends at the
/// combo box. The edit's own focus follows, and JAWS says only it: "Open: edit combo | C: | 1
/// of 26" in the Run dialog [obs, 3159/s6]. A chain that reaches the edit is the edit's focus,
/// and no other follows: the Font dialog's "Font style: edit combo" [obs, 541/s5].
pub(super) fn combo_box_passed_to_its_edit(event: &TraceEvent) -> bool {
    const ROLE_COMBOBOX: i32 = 0x2E;
    const STATE_FOCUSED: u32 = 0x4;
    event.event == "focus"
        && event
            .object
            .as_ref()
            .is_some_and(|o| o.role == Some(ROLE_COMBOBOX) && o.states & STATE_FOCUSED == 0)
        && event
            .focus_window_class
            .as_deref()
            .is_some_and(|c| c.eq_ignore_ascii_case("Edit"))
        && event.window_class != event.focus_window_class
        && event
            .focus_chain
            .last()
            .is_some_and(|n| n.control_type == UIA_COMBOBOX)
}

/// Whether a value change is a choice's, not text's: its object has a value (a select's new
/// option, "Eastgate") while the focused element has no text. A text field's value is its
/// text.
pub(super) fn is_choice_without_text(event: &TraceEvent) -> bool {
    let value = event
        .object
        .as_ref()
        .and_then(|o| o.value.as_deref())
        .is_some_and(|v| !v.trim().is_empty());
    value
        && event
            .text_context
            .as_ref()
            .is_none_or(|t| t.text.trim().is_empty())
}

/// The menu mode transition a recorded event reports, if any.
pub fn menu_event_from_trace(event: &TraceEvent) -> Option<okur_core::MenuEvent> {
    use okur_core::MenuEvent;
    match event.event.as_str() {
        "menu_start" => Some(MenuEvent::Start {
            window_class: event.window_class.clone(),
        }),
        "menu_end" => Some(MenuEvent::End),
        "menu_popup_start" => Some(MenuEvent::PopupStart {
            window_class: event.window_class.clone(),
            window_title: event.window_title.clone(),
        }),
        "menu_popup_end" => Some(MenuEvent::PopupEnd),
        _ => None,
    }
}

/// Converts captured events (live or recorded) into engine events. The live listener and
/// parity replay share it, so both feed the engine the same sequence.
///
/// It remembers the focused object: a state, value or selection change is passed on only when
/// it concerns that object (a check box toggled with Space, a tree item expanded, a combo
/// box value changed), as JAWS speaks changes of the focused control and not of others.
#[derive(Debug, Default)]
pub struct EventConverter {
    /// The MSAA identity (`hwnd`, `id_object`, `id_child`) and name of the last focus.
    focused: Option<(isize, i32, i32, Option<String>)>,
    /// The last focus as the engine received it.
    last_focus: Option<okur_core::FocusEvent>,
    /// The window last brought to the front, whose renaming the engine is told of.
    foreground: Option<isize>,
    /// The focused control's own name, from the last focus event's focus chain.
    focused_control: Option<String>,
    /// The last UIA notification and when it came: the same text again within
    /// [`NOTIFICATION_REPEAT_MS`] is said once (Settings' search box says "Suggestions
    /// available" for every letter typed; JAWS said it once).
    last_notification: Option<(String, u128)>,
    /// The last native live region text and when it came, said once within
    /// [`NOTIFICATION_REPEAT_MS`] as notifications are.
    last_live_region: Option<(String, u128)>,
    /// The last toast and when it showed: the shell reports one toast twice, its view and
    /// the window around it [obs, 770/s1].
    last_toast: Option<(String, u128)>,
}

/// The last focus's MSAA identity (`hwnd`, `id_object`, `id_child`) and name, and the focused
/// control's own name from its focus chain.
type FocusIdentity = ((isize, i32, i32, Option<String>), Option<String>);

thread_local! {
    /// The last focus as the converter on this thread knows it. Live, the WinEvent thread reads
    /// a state or selection change's MSAA object first and skips the rest of the read when the
    /// change is not the focused control's, which the converter would drop: 73% of the state
    /// changes recorded (performance review P5, step 2).
    static CONVERTER_FOCUS: std::cell::RefCell<Option<FocusIdentity>> =
        const { std::cell::RefCell::new(None) };
}

/// The new value of a page's progress bar or meter (MSAA `ROLE_SYSTEM_PROGRESSBAR`, which
/// Chromium gives both) a value change event names.
pub(super) fn page_progress_value(event: &TraceEvent) -> Option<String> {
    let object = event.object.as_ref()?;
    if !is_page_progress(event.window_class.as_deref()?, Some(object)) {
        return None;
    }
    object
        .value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
}

/// MSAA's `ROLE_SYSTEM_PROGRESSBAR`.
pub(super) const ROLE_SYSTEM_PROGRESSBAR: i32 = 0x30;

/// Whether an object in a window of this class is a page's progress bar or meter.
pub(super) fn is_page_progress(window_class: &str, object: Option<&TraceObject>) -> bool {
    WEB_CONTENT_WINDOW_CLASSES.contains(&window_class)
        && object.is_some_and(|o| o.role == Some(ROLE_SYSTEM_PROGRESSBAR))
}

/// Whether a change event, by its MSAA identity and object's name, concerns the focused
/// control, as the converter passes changes on: the focus's own object, or one of its name.
/// The focus event's MSAA object can be the browser window while the focused control is a page
/// button (focus left on it, then Space expanded it, GitHub): the control's own name, from the
/// focus chain, is compared too.
pub(super) fn concerns_focus(
    (focused, control_name): (&(isize, i32, i32, Option<String>), Option<&String>),
    (hwnd, id_object, id_child): (isize, i32, i32),
    object_name: Option<&String>,
) -> bool {
    let (focus_hwnd, focus_object, focus_child, name) = focused;
    let same_object = hwnd == *focus_hwnd && id_object == *focus_object && id_child == *focus_child;
    let focus_on_window = control_name.is_some() && name.as_ref() != control_name;
    let same_name = object_name.is_some()
        && (object_name == name.as_ref() || (focus_on_window && object_name == control_name));
    same_object || same_name
}

/// Whether the converter on this thread may make something of a state, selection or value
/// change, judged from the event and its MSAA object only, before anything else is read: live,
/// the WinEvent thread reads no further a change this refuses. None before a focus.
///
/// Kept: a change of the focused control, and two value changes that are not the focus's:
/// UI Automation's own, which names no MSAA object, in the window of the last focus or the
/// window with the keyboard focus (`in_focus_window`: Excel's grid after Delete,
/// [`EventConverter::focus_moved_by_change`]; Word's document after Space on a check box form
/// field, the frame having had the last focus event), and a page's progress bar or meter, said
/// without the focus. Every change the
/// converter makes something of in the corpus's traces is kept (the corpus test
/// `live_prefilter_keeps_what_the_converter_uses`); the three it once refused were said only
/// in replay (performance review 2026-11-02, live path finding 1).
pub fn live_change_kept(
    event: &str,
    (hwnd, id_object, id_child): (isize, i32, i32),
    object: Option<&TraceObject>,
    window_class: &str,
    in_focus_window: bool,
) -> bool {
    CONVERTER_FOCUS.with(|focus| {
        let focus = focus.borrow();
        let Some((focused, control)) = focus.as_ref() else {
            return false;
        };
        let object_name = object.and_then(|o| o.name.as_ref());
        if concerns_focus(
            (focused, control.as_ref()),
            (hwnd, id_object, id_child),
            object_name,
        ) {
            return true;
        }
        event == "value_change"
            && ((object.is_none() && (hwnd == focused.0 || in_focus_window))
                || is_page_progress(window_class, object))
    })
}

/// A window's own name changed (its title), not a control's in it.
pub fn is_window_renamed(event: &TraceEvent) -> bool {
    event.event == "name_change" && event.id_object == OBJID_WINDOW
}

/// What the mouse points to, as the engine takes it: the control there described as a focus on
/// it would be, and its text there by unit. Shared by the converter and the live worker.
pub fn mouse_point_event(chain: &[UiaNode], point: &TracePoint) -> okur_core::AccessibilityEvent {
    okur_core::AccessibilityEvent::MousePoint(Box::new(okur_core::MousePoint {
        x: point.at[0],
        y: point.at[1],
        control: focus_event_from_chain(chain, None, None),
        texts: point.texts.clone(),
    }))
}

/// How long the same UIA notification again is not said, in milliseconds.
pub(super) const NOTIFICATION_REPEAT_MS: u128 = 2000;

impl EventConverter {
    /// The engine events for one captured event, in order (usually one, or none).
    pub fn convert(&mut self, event: &TraceEvent) -> Vec<okur_core::AccessibilityEvent> {
        if let Some(point) = &event.point {
            return vec![mouse_point_event(&event.focus_chain, point)];
        }
        if let Some([x, y]) = event.mouse {
            return vec![okur_core::AccessibilityEvent::MouseMoved { x, y }];
        }
        // A page's alert carries its content as a live region does [obs, round-aria12].
        if event.event == "live_region_change" || (event.event == "alert" && event.region.is_some())
        {
            return live_region_events(event);
        }
        if event.event == "uia_live_region" {
            let Some(live) = event.focus_subtree.as_ref().and_then(uia_live_region_event) else {
                return Vec::new();
            };
            let repeat = self.last_live_region.as_ref().is_some_and(|(text, at)| {
                *text == live.text && event.t_ms.saturating_sub(*at) < NOTIFICATION_REPEAT_MS
            });
            if repeat {
                return Vec::new();
            }
            self.last_live_region = Some((live.text.clone(), event.t_ms));
            return vec![okur_core::AccessibilityEvent::LiveRegion(live)];
        }
        // A console's screen after it wrote to it, for Screen Echo.
        if event.event == "console_update" {
            return event
                .text_context
                .as_ref()
                .map(|context| okur_core::AccessibilityEvent::ConsoleText(context.text.clone()))
                .into_iter()
                .collect();
        }
        if event.event == "uia_window_opened" {
            let Some((text, sender)) = event.focus_subtree.as_ref().and_then(toast_text) else {
                return Vec::new();
            };
            let repeat = self.last_toast.as_ref().is_some_and(|(said, at)| {
                *said == text && event.t_ms.saturating_sub(*at) < NOTIFICATION_REPEAT_MS
            });
            if repeat {
                return Vec::new();
            }
            self.last_toast = Some((text.clone(), event.t_ms));
            return vec![okur_core::AccessibilityEvent::Toast { text, sender }];
        }
        if let Some(notification) = &event.notification {
            let repeat = self.last_notification.as_ref().is_some_and(|(text, at)| {
                *text == notification.text
                    && event.t_ms.saturating_sub(*at) < NOTIFICATION_REPEAT_MS
            });
            if repeat {
                return Vec::new();
            }
            self.last_notification = Some((notification.text.clone(), event.t_ms));
            return vec![notification_event(notification)];
        }
        // A window's own menu bar reports the focus as its menus end (MSAA's menu bar object,
        // OBJID_MENU): the focus is back in the window. When UI Automation already gives the
        // focus elsewhere (the dialog a menu item opened), that came after: JAWS says both,
        // "Leaving menus | edit | Type in text. | Page Setup dialog | Paper Size: ..."
        // [obs, 541/s2, 544/s2].
        // A menu item's focus that came after its menu closed (an item chosen): the popup
        // window has no item left to read, and UI Automation's focus is the menu bar. It is
        // the menus ending, said by the events after it [obs, 541/s2, 544/s2].
        if event.event == "focus"
            && event.window_class.as_deref() == Some(WIN32_MENU_CLASS)
            && event.object.is_none()
        {
            return Vec::new();
        }
        // A focus that had moved on before it was read is not said: the focus after it is.
        // Tab from File Explorer's search box passes its list for 30 ms on the way to the
        // first link, and JAWS says only "Control Panel Home Link" [obs, 3156/s6].
        if left_before_read(event) || combo_box_passed_to_its_edit(event) {
            return Vec::new();
        }
        if let Some(window) = menu_bar_return(event) {
            let mut events = vec![okur_core::AccessibilityEvent::Focus(window.clone())];
            if let Some(okur_core::AccessibilityEvent::Focus(next)) = self.convert_one(event) {
                if next.window_title != window.window_title {
                    events.push(okur_core::AccessibilityEvent::Focus(next));
                }
            }
            return events;
        }
        self.convert_one(event).into_iter().collect()
    }

    /// Whether `event`, UI Automation's own change of a value in the window with the last focus,
    /// leaves the keyboard focus on that window's own element, where the last focus was another.
    fn focus_moved_by_change(&self, event: &TraceEvent) -> bool {
        let Some(focused) = event.focus_chain.last() else {
            return false;
        };
        event.object.is_none()
            && event.text_context.is_none()
            && focused.has_keyboard_focus
            && focused.class_name.is_some()
            && focused.class_name == event.window_class
            && self
                .focused
                .as_ref()
                .is_some_and(|(hwnd, ..)| *hwnd == event.hwnd)
            && self
                .last_focus
                .as_ref()
                .is_some_and(|last| last.name != focused.name)
    }

    fn convert_one(&mut self, event: &TraceEvent) -> Option<okur_core::AccessibilityEvent> {
        use okur_core::{AccessibilityEvent, ChangeKind};
        if let Some(menu) = menu_event_from_trace(event) {
            return Some(AccessibilityEvent::Menu(menu));
        }
        if event.event == "foreground" {
            self.foreground = Some(event.hwnd);
        }
        if is_window_renamed(event) && self.foreground == Some(event.hwnd) {
            return event
                .window_title
                .clone()
                .map(AccessibilityEvent::WindowRenamed);
        }
        // UI Automation's keyboard focus moved within the focused window with no focus event:
        // Excel's cell emptied by Delete leaves it on the grid, which reports only the change
        // of its value [obs, 1123/s7]. The focus chain read after the change is the new focus.
        if event.event == "value_change" && self.focus_moved_by_change(event) {
            let mut moved = event.clone();
            moved.event = "focus".to_owned();
            return self.convert_one(&moved);
        }
        let kind = match event.event.as_str() {
            // A list item selected while the focus stays in a field (Windows search).
            "uia_selected" => {
                return focus_event_from_trace(event).map(AccessibilityEvent::Selected);
            }
            // The dialog in front shown, read again: its controls as they are now, for Say
            // Window, after a focus read while it was still hidden (the Font dialog [obs,
            // 665/s3]).
            "dialog_shown" => {
                let shown = focus_event_from_trace(event)?;
                if let Some(last) = self.last_focus.as_mut() {
                    if last.window_title == shown.window_title {
                        last.window_items = shown.window_items.clone();
                    }
                }
                return Some(AccessibilityEvent::WindowShown(shown));
            }
            "focus" if is_drop_down_item(event) => {
                let last = self.last_focus.clone()?;
                let name = event.object.as_ref().and_then(|o| o.name.clone());
                let index = usize::try_from(event.id_child).ok().filter(|&i| i > 0);
                // A combo box whose text field has the focus (the Font dialog's, whose lists
                // stay open): its list's item is selected while the focus stays in the field.
                // The dialog's other lists follow it, selecting their own items; the field's
                // new text tells which item is its own [obs, 541/s3].
                if last.role == okur_core::Role::Edit
                    && last.container_role == Some(okur_core::Role::ComboBox)
                {
                    let mut item = okur_core::FocusEvent::new(okur_core::Role::ListItem);
                    item.name = name;
                    item.item_index = index;
                    item.item_count = last.item_count;
                    return Some(AccessibilityEvent::Selected(item));
                }
                // Arrow keys in an open combo box move focus into its drop-down list; JAWS
                // speaks that as the combo box's new value [obs, 304/s12].
                let mut combo = Some(last).filter(|f| f.role == okur_core::Role::ComboBox)?;
                combo.value = name;
                combo.item_index = index;
                self.last_focus = Some(combo.clone());
                return Some(AccessibilityEvent::Change {
                    kind: ChangeKind::Value,
                    control: combo,
                });
            }
            "focus" | "uia_focus" => {
                let mut focus = focus_event_from_trace(event)?;
                // UIA repeats a focus change some WinEvent may also have reported.
                if event.event == "uia_focus" && self.last_focus.as_ref() == Some(&focus) {
                    return None;
                }
                // Word names a ribbon command twice: the ribbon's own focus names its tab, then
                // the tab's own event does; Cut's event comes twice [obs, 1045/s1, s3]. The same
                // command again, right after, is the same focus.
                if focus.in_ribbon
                    && self.last_focus.as_ref().is_some_and(|last| {
                        last.in_ribbon && last.role == focus.role && last.name == focus.name
                    })
                {
                    return None;
                }
                // The event's MSAA object can be a window around the focused control: File
                // Explorer's "Explorer Pane" for its view buttons, whose states are not the
                // checked "Details" button's (dev box, 2026-10-17). Only the control's count.
                if let Some(object) = event
                    .object
                    .as_ref()
                    .filter(|o| o.name.is_some() && o.name == focus.name)
                {
                    apply_msaa_states(&mut focus, object.states);
                }
                focus.text = event.text_context.clone();
                let name = event.object.as_ref().and_then(|o| o.name.clone());
                self.focused = Some((event.hwnd, event.id_object, event.id_child, name));
                self.focused_control = focus.name.clone();
                CONVERTER_FOCUS.with(|known| {
                    *known.borrow_mut() = self
                        .focused
                        .clone()
                        .map(|focused| (focused, self.focused_control.clone()));
                });
                self.last_focus = Some(focus.clone());
                return Some(AccessibilityEvent::Focus(focus));
            }
            // A select's value changed: it has no text, only the new choice as its value. It is
            // said as a value change [obs, 205/s9, s10].
            "value_change" if is_choice_without_text(event) => ChangeKind::Value,
            // A page's progress bar or meter changing without the focus says its new value,
            // "86.0274", the CPU meter as its button changes it [obs, round-aria38, 3113/s1,
            // 3116/s1].
            "value_change"
                if page_progress_value(event).is_some()
                    && !self.focused.as_ref().is_some_and(|focused| {
                        concerns_focus(
                            (focused, self.focused_control.as_ref()),
                            (event.hwnd, event.id_object, event.id_child),
                            event.object.as_ref().and_then(|o| o.name.as_ref()),
                        )
                    }) =>
            {
                return page_progress_value(event).map(|value| {
                    AccessibilityEvent::LiveRegion(okur_core::LiveRegionEvent::new(
                        value,
                        okur_core::Politeness::Polite,
                    ))
                });
            }
            // The focused text field's caret moved or its text changed. A value change of
            // another object is not the field's: Word's ribbon and its Font boxes change as the
            // caret crosses a style, and each was read as the document's text (performance
            // review 2026-10-25, finding 2).
            "caret" | "text_selection_change" | "value_change" if event.text_context.is_some() => {
                let object_name = event.object.as_ref().and_then(|o| o.name.as_ref());
                let concerns = self.focused.as_ref().is_some_and(|focused| {
                    concerns_focus(
                        (focused, self.focused_control.as_ref()),
                        (event.hwnd, event.id_object, event.id_child),
                        object_name,
                    )
                });
                // UI Automation's own change of the text in the window with the keyboard focus
                // names no MSAA object: Word's document after Space on a check box form field,
                // the only event with its new state [1057 traces]. Word's frame can have had the
                // last focus event, so the window is told by UI Automation's focus, its class.
                let own_window = event.object.is_none()
                    && event.window_class.is_some()
                    && event
                        .focus_chain
                        .last()
                        .and_then(|n| n.class_name.as_deref())
                        == event.window_class.as_deref();
                if event.event == "value_change" && !concerns && !own_window {
                    return None;
                }
                return event
                    .text_context
                    .clone()
                    .map(AccessibilityEvent::TextContext);
            }
            "state_change" => ChangeKind::State,
            "value_change" => ChangeKind::Value,
            "selection" => ChangeKind::Selection,
            _ => return None,
        };
        let focused = self.focused.as_ref()?;
        let (hwnd, id_object, id_child, _) = focused;
        let same_object =
            event.hwnd == *hwnd && event.id_object == *id_object && event.id_child == *id_child;
        let object_name = event.object.as_ref().and_then(|o| o.name.as_ref());
        if !concerns_focus(
            (focused, self.focused_control.as_ref()),
            (event.hwnd, event.id_object, event.id_child),
            object_name,
        ) {
            return None;
        }
        // The control as UIA reports it now; without a focus chain (a failed UIA read), the
        // last focus with the event object's MSAA states. The chain is read after the event,
        // and the focus can have moved on from the control the event is about: a select's
        // list takes the focus as the select expands [obs, 276/s7]. The change is then the
        // last focus's, the event object.
        let chain_control = focus_event_from_trace(event);
        let last_name = self.last_focus.as_ref().and_then(|l| l.name.as_ref());
        let moved_on = same_object
            && object_name.is_some()
            && object_name == last_name
            && chain_control
                .as_ref()
                .is_some_and(|c| c.name.as_ref() != object_name);
        let mut control = match chain_control.filter(|_| !moved_on) {
            Some(control) => control,
            None => self.last_focus.clone()?,
        };
        // A change event reads no dialog: the dialog's controls are the last focus's, in the
        // same window, so Say Window still has them after a button's state changes (537/s8).
        if control.window_items.is_empty() {
            if let Some(last) = self
                .last_focus
                .as_ref()
                .filter(|l| l.window_title == control.window_title)
            {
                control.window_items = last.window_items.clone();
                if control.default_button.is_none() {
                    control.default_button = last.default_button.clone();
                }
            }
        }
        // The event object is the focused control: its MSAA states are current, and they
        // carry what UIA may not report (a WinForms combo box's expanded state, 304/s11).
        if same_object {
            if let Some(states) = event.object.as_ref().map(|o| o.states) {
                apply_msaa_states(&mut control, states);
            }
        }
        // A change event carries no siblings to count a position from; the control is the
        // one last focused, whose position it keeps [obs, 304/s17].
        if control.item_index.is_none() {
            if let Some(last) = self.last_focus.as_ref().filter(|l| l.name == control.name) {
                control.item_index = last.item_index;
                control.item_count = last.item_count;
            }
        }
        self.last_focus = Some(control.clone());
        Some(AccessibilityEvent::Change { kind, control })
    }
}

/// A focus on an item of a Win32 combo box's drop-down list (`ComboLBox`).
pub(super) fn is_drop_down_item(event: &TraceEvent) -> bool {
    const MSAA_LISTITEM: i32 = 0x22;
    event.window_class.as_deref() == Some("ComboLBox")
        && event.object.as_ref().and_then(|o| o.role) == Some(MSAA_LISTITEM)
}

/// Text lines of an IA2 subtree: its static text runs, in order.
pub(super) fn text_lines(node: &crate::ia2_snapshot::Ia2Node, out: &mut Vec<String>) {
    const MSAA_STATICTEXT: i32 = 0x29;
    if node.msaa_role == Some(MSAA_STATICTEXT) {
        if let Some(name) = node
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            out.push(name.to_owned());
        }
    }
    for child in &node.children {
        text_lines(child, out);
    }
}

/// A UIA notification, said as an announcement, interrupting speech when the application marks
/// it important, under its program's name without ".exe" ("powershell" [obs, 820/s3]).
pub(super) fn notification_event(
    notification: &TraceNotification,
) -> okur_core::AccessibilityEvent {
    use okur_core::Politeness;
    // NotificationProcessing_ImportantAll and _ImportantMostRecent.
    let politeness = if matches!(notification.processing, 0 | 1) {
        Politeness::Assertive
    } else {
        Politeness::Polite
    };
    let app = notification.process.as_deref().map(|name| {
        name.rsplit_once('.')
            .filter(|(_, extension)| extension.eq_ignore_ascii_case("exe"))
            .map_or(name, |(stem, _)| stem)
            .to_owned()
    });
    okur_core::AccessibilityEvent::AppNotification {
        text: notification.text.trim().to_owned(),
        politeness,
        app,
    }
}

/// A toast's text and sender, in a window of the shell that opened: its view's name
/// (AutomationId `NormalToastView`, on Windows 10), as JAWS says it, and its `SenderName`'s
/// [obs, 770/s1].
pub(super) fn toast_text(window: &UiaNode) -> Option<(String, Option<String>)> {
    const TOAST_VIEW: &str = "NormalToastView";
    const SENDER: &str = "SenderName";
    let named = |node: &UiaNode| {
        node.name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_owned)
    };
    if window.automation_id.as_deref() == Some(TOAST_VIEW) {
        let sender = window
            .children
            .iter()
            .find(|c| c.automation_id.as_deref() == Some(SENDER))
            .and_then(named);
        return named(window).map(|text| (text, sender));
    }
    window.children.iter().find_map(toast_text)
}

pub(super) fn uia_live_region_event(region: &UiaNode) -> Option<okur_core::LiveRegionEvent> {
    use okur_core::{LiveRegionEvent, Politeness};
    fn texts(node: &UiaNode, out: &mut Vec<String>) {
        for child in &node.children {
            if child.control_type == UIA_TEXT {
                if let Some(name) = child
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                {
                    out.push(name.to_owned());
                }
            }
            texts(child, out);
        }
    }
    let politeness = match region.live_setting {
        Some(0) => return None,
        Some(2) => Politeness::Assertive,
        _ => Politeness::Polite,
    };
    let mut held = Vec::new();
    texts(region, &mut held);
    let name = region
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty() && !held.iter().any(|t| t == n));
    let mut parts = Vec::new();
    if !held.is_empty() {
        parts.push(held.join(" "));
    }
    parts.extend(name.map(str::to_owned));
    if parts.is_empty() {
        return None;
    }
    let mut live = LiveRegionEvent::new(parts.join("\n"), politeness);
    live.region_id = Some("uia-live-region".to_owned());
    live.atomic = true;
    Some(live)
}

/// A live region changed: its announcement, then the page as it is now, which refreshes the
/// buffer [obs, 206/s2-s10].
pub(super) fn live_region_events(event: &TraceEvent) -> Vec<okur_core::AccessibilityEvent> {
    use okur_core::{AccessibilityEvent, LiveRegionEvent, Politeness};
    let mut events = Vec::new();
    if let Some(region) = &event.region {
        let mut lines = Vec::new();
        text_lines(region, &mut lines);
        // The region's ARIA properties, as Chromium gives them (its own, or inherited from the
        // live region it is in): an assertive region interrupts what is being said (Google
        // Docs' announcements as the caret moves), an atomic one is read whole.
        let attribute = |name: &str, inherited: &str| {
            region
                .attributes
                .get(name)
                .or_else(|| region.attributes.get(inherited))
                .map(String::as_str)
        };
        let politeness = match attribute("live", "container-live") {
            Some("assertive") => Politeness::Assertive,
            Some("off") => Politeness::Off,
            _ => Politeness::Polite,
        };
        let mut live = LiveRegionEvent::new(lines.join("\n"), politeness);
        live.container_name = region.name.clone().filter(|n| !n.trim().is_empty());
        live.region_id = region.attributes.get("id").cloned();
        live.atomic = attribute("atomic", "container-atomic") == Some("true");
        events.push(AccessibilityEvent::LiveRegion(live));
    }
    if let Some(page) = &event.page {
        events.push(AccessibilityEvent::DocumentLoaded {
            title: page.name.clone().unwrap_or_default(),
            root: crate::ia2_snapshot::ia2_to_aom(page),
        });
    }
    events
}

/// The window a menu bar's focus report gives the focus back to as its menus end (a focus
/// event on OBJID_MENU, MSAA's menu bar object): the window itself, which the engine takes
/// as the control focused before the menus.
pub(super) fn menu_bar_return(event: &TraceEvent) -> Option<okur_core::FocusEvent> {
    const OBJID_MENU: i32 = -3;
    const ROLE_SYSTEM_MENUBAR: i32 = 0x02;
    if event.event != "focus" || event.id_object != OBJID_MENU {
        return None;
    }
    event
        .object
        .as_ref()
        .filter(|o| o.role == Some(ROLE_SYSTEM_MENUBAR))?;
    let title = event.window_title.clone()?;
    let mut focus = okur_core::FocusEvent::new(okur_core::Role::Window).with_name(title.clone());
    focus.window_title = Some(title);
    focus.platform.window_class = event.window_class.clone();
    focus.app = event.process.clone();
    focus.app_product = event.process_product.clone();
    focus.app_version = event.process_version.clone();
    focus.bounds = event.focus_chain.last().and_then(|n| n.bounds);
    // The field the window gives the focus back to, as the trace read it.
    focus.text = event.text_context.clone();
    Some(focus)
}
