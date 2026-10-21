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
use okur_core::ui::WizardButtons;
use okur_core::ui::{Control, ControlKind, DialogSpec, FromUi, RecordingUiHost, ToUi, Value};
use okur_core::{FocusEvent, Role};

/// A dialog's label without its access key mark ("&Move To Heading": "Move To Heading"), its
/// spaces single: JAWS's items join their parts with runs of them ("I agree to the terms of
/// use check box   not checked" [obs, 250/s2]). JAWS's name for itself reads as Okur's, as
/// in every comparison with JAWS ([`crate::parity::in_okurs_name`]): JAWS's "JAWS Find"
/// dialog stands for Okur's "Okur Find".
fn plain(label: &str) -> String {
    crate::parity::in_okurs_name(
        &label
            .replace('&', "")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

#[derive(Debug, Default)]
pub struct DialogMirror {
    dialog: Option<DialogSpec>,
    /// The control the focus is on, by its index in the dialog's controls.
    focused: Option<usize>,
    /// The title of the dialog last closed: JAWS's dialog, standing for it, can still report
    /// a focus of its own as it goes, which Okur's window would not.
    closed: Option<String>,
    /// Dialogs another one opened over (the PlaceMarker List under Add), each with its
    /// focused control: the last is in front again when the one over it closes.
    below: Vec<(DialogSpec, Option<usize>)>,
}

/// A wizard's page as the window shows it: the page's controls, then its sheet's buttons, named
/// as JAWS's wizard names them ("< Back", "Next >", "Cancel" [obs, 705/s5, s10]), Next the
/// default or Finish in its place, those not available unavailable: Alt+N on the last page
/// presses nothing [obs, 708/s25].
fn wizard_dialog(mut page: DialogSpec, buttons: WizardButtons) -> DialogSpec {
    let button = |id: &str, label: &str, default: bool, enabled: bool| Control {
        id: id.to_owned(),
        label: label.to_owned(),
        kind: ControlKind::Button { default },
        enabled,
    };
    page.controls.extend([
        button("back", "< &Back", false, buttons.back),
        button("next", "&Next >", buttons.next, buttons.next),
        button("finish", "Finish", !buttons.next, buttons.finish),
        button("cancel", "Cancel", false, true),
    ]);
    page
}

impl DialogMirror {
    /// Takes what the engine sent its windows: a dialog shown or closed.
    pub fn sync(&mut self, host: &RecordingUiHost) {
        for message in host.take() {
            let message = match message {
                // A wizard is a dialog: its page and its sheet's buttons, the page turned in
                // the same window.
                ToUi::WizardShow { wizard } => ToUi::DialogShow {
                    dialog: wizard_dialog(wizard.page, wizard.buttons),
                },
                ToUi::WizardPage { id, page, buttons }
                    if self.dialog.as_ref().is_some_and(|d| d.id == id) =>
                {
                    let page = wizard_dialog(page, buttons);
                    self.focused = page.controls.iter().position(is_tab_stop);
                    self.dialog = Some(page);
                    continue;
                }
                other => other,
            };
            match message {
                ToUi::DialogShow { dialog } => {
                    self.closed = None;
                    let focused = dialog.controls.iter().position(is_tab_stop);
                    let over = self.dialog.replace(dialog);
                    if let Some(over) =
                        over.filter(|d| Some(d.id) != self.dialog.as_ref().map(|n| n.id))
                    {
                        self.below.push((over, self.focused));
                    }
                    self.focused = focused;
                }
                ToUi::DialogClose { id } if self.dialog.as_ref().is_some_and(|d| d.id == id) => {
                    self.close_front();
                }
                ToUi::DialogClose { id } => self.below.retain(|(d, _)| d.id != id),
                ToUi::DialogEnable {
                    id,
                    control,
                    enabled,
                } => {
                    if let Some(c) = self.control_of(id, &control) {
                        c.enabled = enabled;
                    }
                }
                ToUi::DialogDefault { id, control } => {
                    let dialog = self
                        .dialog
                        .iter_mut()
                        .chain(self.below.iter_mut().map(|(d, _)| d))
                        .find(|d| d.id == id);
                    for c in dialog.into_iter().flat_map(|d| d.controls.iter_mut()) {
                        if let ControlKind::Button { default } = &mut c.kind {
                            *default = c.id == control;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// The dialog in front closes: the one under it, if any, is in front again.
    fn close_front(&mut self) {
        self.closed = self.dialog.as_ref().map(|d| plain(&d.title));
        let (dialog, focused) = self.below.pop().map_or((None, None), |(d, f)| (Some(d), f));
        self.dialog = dialog;
        self.focused = focused;
    }

    /// Control `control` of dialog `id`, in front or under another.
    fn control_of(&mut self, id: u32, control: &str) -> Option<&mut Control> {
        self.dialog
            .iter_mut()
            .chain(self.below.iter_mut().map(|(d, _)| d))
            .find(|d| d.id == id)?
            .controls
            .iter_mut()
            .find(|c| c.id == control)
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
    /// Regions tree has none in its window ("Regions Tree view" [obs, 241/s1]); and Okur's
    /// window has Okur's name in its title where JAWS's has its own (AGENTS.md §1.5): "Okur
    /// Find" for "JAWS Find" [obs, 506/s12].
    pub fn as_okur_window(&self, focus: &mut FocusEvent) {
        let Some(dialog) = &self.dialog else {
            return;
        };
        let title = focus.window_title.as_deref().unwrap_or_default();
        if crate::parity::in_okurs_name(title) != title && plain(title) == plain(&dialog.title) {
            focus.window_title = Some(dialog.title.clone());
        }
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
        // A list item is Okur's own label where JAWS's differs from it only in punctuation:
        // JAWS's \"Radio Buttons List,\" is a typo (an accepted divergence) [obs, 230/s18].
        if focus.role == Role::ListItem {
            let letters = |text: &str| -> String {
                text.chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
            };
            let name = focus.name.as_deref().unwrap_or_default().to_owned();
            let own = dialog.controls.iter().find_map(|c| match &c.kind {
                ControlKind::List { items, .. } | ControlKind::ListBox { items, .. } => items
                    .iter()
                    .find(|item| letters(item) == letters(&name) && plain(item) != plain(&name))
                    .map(|item| plain(item)),
                _ => None,
            });
            if own.is_some() {
                focus.name = own;
            }
        }
    }

    /// The focus moved in the traced dialog, or its focused control changed: which control of
    /// Okur's it is, and the window's report of a list item selected, a radio button checked
    /// or a text typed.
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
                // A table's row by its first column; chosen only when the list reports it
                // selected: JAWS's PlaceMarker List opens with its first row "Not Selected"
                // [obs, 702/s18].
                (ControlKind::Table { rows, selected, .. }, Role::ListItem) => {
                    let first =
                        |i: usize| rows.get(i).and_then(|row| row.first()).map(|c| plain(c));
                    let at = focus
                        .item_index
                        .and_then(|i| i.checked_sub(1))
                        .filter(|&i| first(i).as_deref() == Some(name.as_str()))
                        .or_else(|| {
                            (0..rows.len()).find(|&i| first(i).as_deref() == Some(name.as_str()))
                        });
                    match at {
                        Some(i) if focus.states.selected == Some(true) && *selected != Some(i) => {
                            *selected = Some(i);
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
                (
                    ControlKind::Radios { options, selected }
                    | ControlKind::RadioButtons {
                        options, selected, ..
                    },
                    Role::RadioButton,
                ) => match options.iter().position(|option| plain(option) == name) {
                    Some(i) if i != *selected && focus.states.checked == Some(true) => {
                        *selected = i;
                        (true, Some(Value::Selected(i)))
                    }
                    Some(_) => (true, None),
                    None => (false, None),
                },
                // The text typed into an edit combo, as its window reports each change
                // ("b", "bo", … "board" in JAWS Find's Find what [obs, 506/s2-s6]), or into an
                // edit (Custom Label's "lamp" [obs, 688/s10-s13]).
                (
                    ControlKind::ComboEdit { text, .. } | ControlKind::Edit { text },
                    Role::Edit | Role::ComboBox | Role::Document,
                ) if plain(&control.label) == name => match focus.value.as_deref() {
                    Some(value) if value != text => {
                        value.clone_into(text);
                        (true, Some(Value::Text(value.to_owned())))
                    }
                    _ => (true, None),
                },
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

    /// The focused field's text, as the traced dialog's edit reports it after a key: what
    /// Okur's window reports of the text typed into its focused edit combo ("b", "bo", …
    /// "board" in JAWS Find's Find what [obs, 633/s2-s6]).
    pub fn typed(&mut self, typed: &str) -> Vec<FromUi> {
        let Some(dialog) = &mut self.dialog else {
            return Vec::new();
        };
        let Some(control) = self.focused.and_then(|i| dialog.controls.get_mut(i)) else {
            return Vec::new();
        };
        match &mut control.kind {
            ControlKind::ComboEdit { text, .. } | ControlKind::Edit { text } if text != typed => {
                typed.clone_into(text);
                vec![FromUi::DialogChanged {
                    id: dialog.id,
                    control: control.id.clone(),
                    value: Value::Text(typed.to_owned()),
                }]
            }
            _ => Vec::new(),
        }
    }

    /// What the window reports for a key the user pressed in it.
    pub fn key(&mut self, event: &KeyEvent) -> Vec<FromUi> {
        let Some(dialog) = &self.dialog else {
            return Vec::new();
        };
        // Alt and a letter: the control whose label marks that letter as its access key, as
        // the window's dialog manager finds it: a button pressed ("Remo&ve All" for Alt+V
        // [obs, 702/s27]), a radio option chosen, any other control taking the focus.
        if let (Key::Alpha(letter), true) = (
            event.key,
            event.modifiers.alt
                && !event.modifiers.ctrl
                && !event.modifiers.shift
                && !event.modifiers.insert
                && !event.modifiers.win,
        ) {
            return self.access_key(letter);
        }
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
            self.close_front();
        }
        report.into_iter().collect()
    }
}

/// Whether `label` marks `letter` as its access key ("Remo&ve All" for V).
fn marks(label: &str, letter: char) -> bool {
    label
        .split('&')
        .skip(1)
        .filter_map(|rest| rest.chars().next())
        .any(|c| c.eq_ignore_ascii_case(&letter))
}

impl DialogMirror {
    /// Alt+`letter` in the dialog in front.
    fn access_key(&mut self, letter: char) -> Vec<FromUi> {
        let Some(dialog) = &mut self.dialog else {
            return Vec::new();
        };
        let id = dialog.id;
        let mut reports = Vec::new();
        let mut closes = false;
        for (index, control) in dialog.controls.iter_mut().enumerate() {
            if !control.enabled {
                continue;
            }
            if let ControlKind::Radios { options, selected }
            | ControlKind::RadioButtons {
                options, selected, ..
            } = &mut control.kind
            {
                if let Some(option) = options.iter().position(|o| marks(o, letter)) {
                    self.focused = Some(index);
                    if option != *selected {
                        *selected = option;
                        reports.push(FromUi::DialogChanged {
                            id,
                            control: control.id.clone(),
                            value: Value::Selected(option),
                        });
                    }
                    break;
                }
                continue;
            }
            if !marks(&control.label, letter) {
                continue;
            }
            self.focused = Some(index);
            if matches!(control.kind, ControlKind::Button { .. }) {
                if control.id == "cancel" {
                    closes = true;
                    reports.push(FromUi::DialogClosed { id });
                } else {
                    reports.push(FromUi::DialogPressed {
                        id,
                        control: control.id.clone(),
                    });
                }
            }
            break;
        }
        if closes {
            self.close_front();
        }
        reports
    }
}

/// Whether the dialog's first focus can land on the control.
fn is_tab_stop(control: &Control) -> bool {
    control.enabled
        && !matches!(
            control.kind,
            ControlKind::Text { .. } | ControlKind::Group | ControlKind::Message { .. }
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use okur_core::ui::UiHost;

    /// JAWS's dialog, traced, stands for Okur's of the same title in Okur's name: "JAWS Find"
    /// taking the focus after Okur's "Okur Find" closed is that dialog going away.
    #[test]
    fn jaws_dialog_stands_for_okurs_by_its_name() {
        let host = RecordingUiHost::default();
        let mut mirror = DialogMirror::default();
        host.send(ToUi::DialogShow {
            dialog: DialogSpec {
                id: 7,
                title: "Okur Find".to_owned(),
                controls: Vec::new(),
            },
        });
        host.send(ToUi::DialogClose { id: 7 });
        mirror.sync(&host);
        let mut focus = FocusEvent::new(Role::Edit);
        focus.window_title = Some("JAWS Find".to_owned());
        assert!(mirror.is_closed_dialog(&focus));
        focus.window_title = Some("Find".to_owned());
        assert!(!mirror.is_closed_dialog(&focus));
    }

    /// In JAWS's dialog standing for Okur's, the title is Okur's, as live [obs, 506/s12];
    /// another window keeps its own.
    #[test]
    fn jaws_dialog_reports_okurs_title() {
        let host = RecordingUiHost::default();
        let mut mirror = DialogMirror::default();
        host.send(ToUi::DialogShow {
            dialog: DialogSpec {
                id: 3,
                title: "Okur Find".to_owned(),
                controls: Vec::new(),
            },
        });
        mirror.sync(&host);
        let mut focus = FocusEvent::new(Role::ComboBox).with_name("Find what:");
        focus.window_title = Some("JAWS Find".to_owned());
        mirror.as_okur_window(&mut focus);
        assert_eq!(focus.window_title.as_deref(), Some("Okur Find"));
        let mut other = FocusEvent::new(Role::Edit);
        other.window_title = Some("JAWS notes - Notepad".to_owned());
        mirror.as_okur_window(&mut other);
        assert_eq!(other.window_title.as_deref(), Some("JAWS notes - Notepad"));
    }

    /// The text typed into the focused edit combo, as the traced field reports it after each
    /// key, is what Okur's window reports; the same text again reports nothing
    /// [obs, 633/s2-s6].
    #[test]
    fn typed_text_is_reported_as_okurs_window_would() {
        let host = RecordingUiHost::default();
        let mut mirror = DialogMirror::default();
        host.send(ToUi::DialogShow {
            dialog: DialogSpec {
                id: 4,
                title: "Okur Find".to_owned(),
                controls: vec![Control {
                    id: "what".to_owned(),
                    label: "Find &what:".to_owned(),
                    kind: ControlKind::ComboEdit {
                        text: "library".to_owned(),
                        history: vec!["library".to_owned()],
                    },
                    enabled: true,
                }],
            },
        });
        mirror.sync(&host);
        assert_eq!(
            mirror.typed("b"),
            vec![FromUi::DialogChanged {
                id: 4,
                control: "what".to_owned(),
                value: Value::Text("b".to_owned()),
            }]
        );
        assert_eq!(mirror.typed("b"), Vec::new());
    }

    /// A dialog Okur opens over another (Remove All's question over the PlaceMarker List):
    /// closing it puts the one under it in front again; Alt and a letter press the button it
    /// marks, and the default moves as Okur asks [obs, 702/s27, s28].
    #[test]
    fn a_dialog_over_another_and_its_access_keys() {
        use okur_core::input::{Key, KeyEvent, Modifiers};
        let button = |id: &str, label: &str, default: bool| Control {
            id: id.to_owned(),
            label: label.to_owned(),
            kind: ControlKind::Button { default },
            enabled: true,
        };
        let host = RecordingUiHost::default();
        let mut mirror = DialogMirror::default();
        host.send(ToUi::DialogShow {
            dialog: DialogSpec {
                id: 1,
                title: "PlaceMarker List".to_owned(),
                controls: vec![
                    Control {
                        id: "markers".to_owned(),
                        label: "PlaceMarker".to_owned(),
                        kind: ControlKind::Table {
                            columns: vec!["PlaceMarker".to_owned()],
                            rows: vec![vec!["Water".to_owned()]],
                            selected: None,
                        },
                        enabled: true,
                    },
                    button("add", "&Add...", true),
                    button("moveto", "&MoveTo", false),
                    button("removeall", "Remo&ve All", false),
                    button("cancel", "&Cancel", false),
                ],
            },
        });
        host.send(ToUi::DialogDefault {
            id: 1,
            control: "moveto".to_owned(),
        });
        mirror.sync(&host);
        let alt = Modifiers {
            alt: true,
            ..Modifiers::none()
        };
        // Enter in the list presses the default Okur made it.
        assert_eq!(
            mirror.key(&KeyEvent::new(Key::Enter, Modifiers::none(), true)),
            [FromUi::DialogPressed {
                id: 1,
                control: "moveto".to_owned()
            }]
        );
        assert_eq!(
            mirror.key(&KeyEvent::new(Key::Alpha('V'), alt, true)),
            [FromUi::DialogPressed {
                id: 1,
                control: "removeall".to_owned()
            }]
        );
        host.send(ToUi::DialogShow {
            dialog: DialogSpec {
                id: 2,
                title: "PlaceMarker RemoveAll Warning".to_owned(),
                controls: vec![button("yes", "&Yes", true), button("cancel", "&No", false)],
            },
        });
        mirror.sync(&host);
        // No closes the question; the list is in front again, the focus on Remove All.
        assert_eq!(
            mirror.key(&KeyEvent::new(Key::Alpha('N'), alt, true)),
            [FromUi::DialogClosed { id: 2 }]
        );
        assert!(mirror.is_open());
        assert_eq!(
            mirror.key(&KeyEvent::new(Key::Enter, Modifiers::none(), true)),
            [FromUi::DialogPressed {
                id: 1,
                control: "removeall".to_owned()
            }]
        );
    }
}
