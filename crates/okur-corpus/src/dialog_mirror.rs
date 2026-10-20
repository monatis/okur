//! Okur's own dialogs in replay. Live, Okur shows its list dialogs (the Links List, the
//! Heading List, ...) through `okur-ui`: the focus moves in a real Win32 dialog, which the
//! platform reports as for any window, and the window tells Okur what the user did (an item
//! selected, a button pressed, the dialog closed). Okur's dialogs are JAWS's, label for label
//! (`okur_core::ui::lists`), so in replay JAWS's own dialog, traced while JAWS ran it, stands
//! for Okur's: its focus events are what Okur's window would report, and this mirror turns
//! them, and the keys pressed, into what Okur's window would tell Okur.
//!
//! The mirror knows the dialog the engine asked to show ([`okur_core::ui::ToUi::DialogShow`])
//! and which of its controls the trace last gave the focus. A list item taking the focus
//! selects it; Enter presses the focused button, or the default one elsewhere; Space presses
//! the focused button; Escape and Cancel close the dialog.

use okur_core::input::{Key, KeyEvent};
use okur_core::ui::{Control, ControlKind, DialogSpec, FromUi, RecordingUiHost, ToUi, Value};
use okur_core::{FocusEvent, Role};

/// A dialog's label without its access key mark ("&Move To Heading": "Move To Heading"), its
/// spaces single: JAWS's items join their parts with runs of them ("I agree to the terms of
/// use check box   not checked" [obs, 250/s2]).
fn plain(label: &str) -> String {
    label
        .replace('&', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Default)]
pub struct DialogMirror {
    dialog: Option<DialogSpec>,
    /// The control the focus is on, by its index in the dialog's controls.
    focused: Option<usize>,
    /// The title of the dialog last closed: JAWS's dialog, standing for it, can still report
    /// a focus of its own as it goes, which Okur's window would not.
    closed: Option<String>,
}

impl DialogMirror {
    /// Takes what the engine sent its windows: a dialog shown or closed.
    pub fn sync(&mut self, host: &RecordingUiHost) {
        for message in host.take() {
            match message {
                ToUi::DialogShow { dialog } => {
                    self.closed = None;
                    self.focused = dialog.controls.iter().position(is_tab_stop);
                    self.dialog = Some(dialog);
                }
                ToUi::DialogClose { id } if self.dialog.as_ref().is_some_and(|d| d.id == id) => {
                    self.closed = self.dialog.as_ref().map(|d| plain(&d.title));
                    self.dialog = None;
                    self.focused = None;
                }
                _ => {}
            }
        }
    }

    pub fn is_open(&self) -> bool {
        self.dialog.is_some()
    }

    /// Whether `focus` is in the dialog just closed: JAWS's dialog taking the focus as it
    /// goes ("Select a Form Field" dialog, after Enter [obs, 228/s9]), which Okur's window,
    /// already closed, does not report. Delivered, it was a window of its own, which took
    /// the field just focused out of forms mode.
    pub fn is_closed_dialog(&self, focus: &FocusEvent) -> bool {
        self.dialog.is_none()
            && self.closed.is_some()
            && focus.window_title.as_deref().map(plain) == self.closed
    }

    /// The focus as Okur's window would report it where JAWS's dialog differs: Okur's list
    /// or tree is named by the label above it (okur-ui draws one), where JAWS's Document
    /// Regions tree has none in its window ("Regions Tree view" [obs, 241/s1]).
    pub fn as_okur_window(&self, focus: &mut FocusEvent) {
        let Some(dialog) = &self.dialog else {
            return;
        };
        let unnamed = focus
            .container_name
            .as_deref()
            .is_none_or(|n| n.trim().is_empty());
        let label = match focus.container_role {
            Some(Role::Tree) if unnamed => dialog.controls.iter().find_map(|c| match c.kind {
                ControlKind::Tree { .. } => Some(plain(&c.label)),
                _ => None,
            }),
            _ => None,
        };
        if label.is_some() {
            focus.container_name = label;
        }
    }

    /// The focus moved in the traced dialog: which control of Okur's it is, and the window's
    /// report of a list item selected or a radio button checked by it.
    pub fn focus(&mut self, focus: &FocusEvent) -> Vec<FromUi> {
        let Some(dialog) = &mut self.dialog else {
            return Vec::new();
        };
        let name = focus.name.as_deref().map(plain).unwrap_or_default();
        let mut reports = Vec::new();
        for (index, control) in dialog.controls.iter_mut().enumerate() {
            let (here, report) = match (&mut control.kind, &focus.role) {
                (
                    ControlKind::List { items, selected }
                    | ControlKind::ListBox { items, selected },
                    Role::ListItem,
                ) => {
                    // The item by its position, else by its text.
                    let at = focus
                        .item_index
                        .and_then(|i| i.checked_sub(1))
                        .filter(|&i| items.get(i).is_some_and(|item| plain(item) == name))
                        .or_else(|| items.iter().position(|item| plain(item) == name));
                    match at {
                        Some(i) if i != *selected => {
                            *selected = i;
                            (true, Some(Value::Selected(i)))
                        }
                        Some(_) => (true, None),
                        None => (false, None),
                    }
                }
                (ControlKind::Tree { items, selected }, Role::TreeItem) => {
                    match items.iter().position(|item| plain(&item.text) == name) {
                        Some(i) if i != *selected => {
                            *selected = i;
                            (true, Some(Value::Selected(i)))
                        }
                        Some(_) => (true, None),
                        None => (false, None),
                    }
                }
                (ControlKind::Radios { options, selected }, Role::RadioButton) => {
                    match options.iter().position(|option| plain(option) == name) {
                        Some(i) if i != *selected && focus.states.checked == Some(true) => {
                            *selected = i;
                            (true, Some(Value::Selected(i)))
                        }
                        Some(_) => (true, None),
                        None => (false, None),
                    }
                }
                (ControlKind::Button { .. }, Role::Button)
                | (ControlKind::CheckBox { .. }, Role::CheckBox) => {
                    (plain(&control.label) == name, None)
                }
                _ => (false, None),
            };
            if here {
                self.focused = Some(index);
                reports.extend(report.map(|value| FromUi::DialogChanged {
                    id: dialog.id,
                    control: control.id.clone(),
                    value,
                }));
                break;
            }
        }
        reports
    }

    /// What the window reports for a key the user pressed in it.
    pub fn key(&mut self, event: &KeyEvent) -> Vec<FromUi> {
        let Some(dialog) = &self.dialog else {
            return Vec::new();
        };
        if !event.modifiers.is_empty() {
            return Vec::new();
        }
        let focused = self.focused.and_then(|i| dialog.controls.get(i));
        let press = |control: &Control| {
            if control.id == "cancel" {
                FromUi::DialogClosed { id: dialog.id }
            } else {
                FromUi::DialogPressed {
                    id: dialog.id,
                    control: control.id.clone(),
                }
            }
        };
        let report = match event.key {
            Key::Escape => Some(FromUi::DialogClosed { id: dialog.id }),
            Key::Enter => focused
                .filter(|c| matches!(c.kind, ControlKind::Button { .. }))
                .or_else(|| {
                    dialog
                        .controls
                        .iter()
                        .find(|c| matches!(c.kind, ControlKind::Button { default: true }))
                })
                .map(press),
            Key::Space => focused
                .filter(|c| matches!(c.kind, ControlKind::Button { .. }))
                .map(press),
            _ => None,
        };
        if matches!(report, Some(FromUi::DialogClosed { .. })) {
            self.closed = self.dialog.as_ref().map(|d| plain(&d.title));
            self.dialog = None;
            self.focused = None;
        }
        report.into_iter().collect()
    }
}

/// Whether the dialog's first focus can land on the control.
fn is_tab_stop(control: &Control) -> bool {
    control.enabled && !matches!(control.kind, ControlKind::Text { .. } | ControlKind::Group)
}
