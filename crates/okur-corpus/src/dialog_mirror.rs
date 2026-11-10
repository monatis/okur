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

/// The program Okur's windows are in, as the platform names it.
const OKUR_UI: &str = "okur-ui.exe";

#[derive(Debug, Default)]
pub struct DialogMirror {
    dialog: Option<DialogSpec>,
    /// The control the focus is on, by its index in the dialog's controls.
    focused: Option<usize>,
    /// The trace has given the dialog shown last a focus of its own.
    focus_traced: bool,
    /// The title of the dialog last closed: JAWS's dialog, standing for it, can still report
    /// a focus of its own as it goes, which Okur's window would not.
    closed: Option<String>,
    /// Dialogs another one opened over (the PlaceMarker List under Add), each with its
    /// focused control: the last is in front again when the one over it closes.
    below: Vec<(DialogSpec, Option<usize>)>,
    /// The check list row F2 is renaming: the row the focus comes back to under another name.
    renaming: Option<usize>,
    /// The system's Open dialog shown over this dialog (Select Sound...): keys and text are
    /// its own until Escape closes it [obs, 802/s13].
    file_open: Option<u32>,
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
        hidden: false,
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
                ToUi::WizardButtons { id, buttons } => {
                    for (control, enabled) in [
                        ("back", buttons.back),
                        ("next", buttons.next),
                        ("finish", buttons.finish),
                    ] {
                        if let Some(c) = self.control_of(id, control) {
                            c.enabled = enabled;
                        }
                    }
                    continue;
                }
                other => other,
            };
            match message {
                ToUi::DialogShow { dialog } => {
                    self.closed = None;
                    self.focus_traced = false;
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
                ToUi::DialogRows {
                    id,
                    control,
                    rows: new_rows,
                    checked: new_checked,
                    selected: new_selected,
                } => {
                    if let Some(ControlKind::CheckList {
                        rows,
                        checked,
                        selected,
                        ..
                    }) = self.control_of(id, &control).map(|c| &mut c.kind)
                    {
                        *rows = new_rows;
                        *checked = new_checked;
                        *selected = new_selected;
                    }
                }
                // The system's Open dialog over Okur's: its keys are its own until it closes.
                ToUi::FileOpen { id, .. } => self.file_open = Some(id),
                ToUi::DialogFocus { id, control } => {
                    let at = self
                        .dialog
                        .as_ref()
                        .filter(|d| d.id == id)
                        .and_then(|d| d.controls.iter().position(|c| c.id == control));
                    if at.is_some() {
                        self.focused = at;
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

    /// Whether `event`, JAWS's dialog coming to the front, can stand for its first focus:
    /// Okur's dialog of that title is open and the trace has given it no focus yet. JAWS's main
    /// window can come back to the front between the dialog's first focus and its list's
    /// selection, and the trace's converter then drops both (1183/s6: Cells with Notes); the
    /// foreground keeps the focus read with it, which Okur's own window would report. Replay
    /// takes it only where the step ends with still no focus traced in the dialog
    /// ([`Self::awaits_first_focus`]): a focus event of its own reports more (a list item's
    /// position).
    /// Whether Okur's dialog is open and the trace has given it no focus yet.
    pub fn awaits_first_focus(&self) -> bool {
        self.dialog.is_some() && !self.focus_traced
    }

    pub fn first_focus_by_foreground(
        &self,
        event: &okur_windows_model::event_trace::TraceEvent,
    ) -> bool {
        event.event == "foreground"
            && !self.focus_traced
            && !event.focus_chain.is_empty()
            && self.dialog.as_ref().is_some_and(|dialog| {
                event.window_title.as_deref().map(plain) == Some(plain(&dialog.title))
            })
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
            tracing::debug!(window = ?focus.window_title, "no dialog of Okur's mirrored");
            return;
        };
        let title = focus.window_title.as_deref().unwrap_or_default();
        let ours = plain(title) == plain(&dialog.title);
        tracing::debug!(title, okurs = %dialog.title, ours, "a focus beside Okur's dialog");
        if crate::parity::in_okurs_name(title) != title && ours {
            focus.window_title = Some(dialog.title.clone());
        }
        // Okur's dialog is a window of okur-ui's, drawn with the system's controls, where JAWS's
        // can be a page in a web view (Search for JAWS Commands, in msedgewebview2.exe [obs,
        // 628/s6]): the focus in it is in Okur's own windows, and on no page, so typing goes to
        // its field rather than to a page's quick keys.
        if ours {
            focus.app = Some(OKUR_UI.to_owned());
            focus.in_document = false;
            focus.dom_id = None;
            focus.unique_id = None;
            focus.web_application = None;
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
        // A list box is named as Okur's dialog names it: JAWS names an unlabelled one by its
        // dialog in one of its dialogs and not in another, "Select a System Tray Icon List box"
        // [obs, 511/s4], "Select Hyperlink dialog | List box" [obs, 1191/s4], with the same
        // controls traced in both.
        if ours && focus.container_role == Some(Role::List) && !focus.container_is_list_view {
            let own = dialog.controls.iter().find_map(|c| match c.kind {
                ControlKind::ListBox { .. } => Some(plain(&c.label)),
                _ => None,
            });
            if let Some(own) = own {
                if let Some(list) = focus.groups.iter_mut().rev().find(|g| g.list) {
                    list.name.clone_from(&own);
                }
                focus.container_name = Some(own).filter(|n| !n.is_empty());
            }
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
            // Its place in the list, which Okur's list reports, where the trace read none (a
            // focus read with the dialog's coming to the front, 1183/s6).
            if focus.item_index.is_none() && focus.item_count.is_none() {
                let name = focus.name.as_deref().map(plain).unwrap_or_default();
                let place = dialog.controls.iter().find_map(|c| match &c.kind {
                    ControlKind::List { items, .. } | ControlKind::ListBox { items, .. } => items
                        .iter()
                        .position(|item| plain(item) == name)
                        .map(|at| (at + 1, items.len())),
                    _ => None,
                });
                if let Some((index, count)) = place {
                    focus.item_index = Some(index);
                    focus.item_count = Some(count);
                }
            }
        }
    }

    /// The focus moved in the traced dialog, or its focused control changed: which control of
    /// Okur's it is, and the window's report of a list item selected, a radio button checked
    /// or a text typed.
    pub fn focus(&mut self, focus: &FocusEvent) -> Vec<FromUi> {
        if self.file_open.is_some() {
            return Vec::new();
        }
        // The dialog's first focus: okur-ui's word that it is shown, the keyboard in it.
        let first = self.dialog.is_some() && !self.focus_traced;
        if self.dialog.is_some() {
            self.focus_traced = true;
        }
        let Some(dialog) = &mut self.dialog else {
            return Vec::new();
        };
        let name = focus.name.as_deref().map(plain).unwrap_or_default();
        let mut reports = Vec::new();
        if first {
            reports.push(FromUi::DialogShown { id: dialog.id });
        }
        let focused = self.focused;
        for (index, control) in dialog.controls.iter_mut().enumerate() {
            // An unnamed field's change is the focused field's: a WPF edit is named only
            // through the text before it, which its value changes do not carry (Create
            // Rule's Shortened Text [obs, 798/s10]).
            let unnamed_here = name.is_empty() && focused == Some(index);
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
                // A check list's row by its first cell, and its check as the window reports it
                // turned (Space on the row).
                (
                    ControlKind::CheckList {
                        rows,
                        checked,
                        selected,
                        ..
                    },
                    Role::ListItem,
                ) => {
                    let first =
                        |i: usize| rows.get(i).and_then(|row| row.first()).map(|c| plain(c));
                    let at = focus
                        .item_index
                        .and_then(|i| i.checked_sub(1))
                        .filter(|&i| first(i).as_deref() == Some(name.as_str()))
                        .or_else(|| {
                            (0..rows.len()).find(|&i| first(i).as_deref() == Some(name.as_str()))
                        });
                    // The row being renamed, back under its new name [obs, 751/s20].
                    let renamed = match (at, self.renaming) {
                        (None, Some(row))
                            if focus.item_index.and_then(|i| i.checked_sub(1)) == Some(row) =>
                        {
                            rows.get_mut(row).and_then(|r| r.first_mut()).map(|cell| {
                                name.clone_into(cell);
                                row
                            })
                        }
                        _ => None,
                    };
                    if let Some(row) = renamed {
                        self.renaming = None;
                        self.focused = Some(index);
                        reports.push(FromUi::DialogChanged {
                            id: dialog.id,
                            control: control.id.clone(),
                            value: Value::ItemRenamed(row, name.clone()),
                        });
                        break;
                    }
                    match at {
                        Some(i) => {
                            let turned = match (focus.states.checked, checked.get_mut(i)) {
                                (Some(now), Some(was)) if now != *was => {
                                    *was = now;
                                    Some(Value::ItemChecked(i, now))
                                }
                                _ => None,
                            };
                            let chosen = (i != *selected).then(|| {
                                *selected = i;
                                Value::Selected(i)
                            });
                            (true, turned.or(chosen))
                        }
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
                    ControlKind::ComboEdit { text, .. }
                    | ControlKind::Edit { text }
                    | ControlKind::SearchBox { text },
                    Role::Edit | Role::ComboBox | Role::Document,
                ) if plain(&control.label) == name || unnamed_here => {
                    match focus.value.as_deref() {
                        Some(value) if value != text => {
                            value.clone_into(text);
                            (true, Some(Value::Text(value.to_owned())))
                        }
                        _ => (true, None),
                    }
                }
                // A read-only text reached by Tab ("Sound: read only edit | <None>" in Add
                // Dictionary Definition [obs, 1158/s9]): the focus only, its text not Okur's to
                // change, so what the trace reports of it is typed nowhere.
                (ControlKind::Note { .. }, Role::Edit | Role::Document)
                    if plain(&control.label) == name =>
                {
                    (true, None)
                }
                // A combo box's choice, as its window reports the arrows choosing it (Create
                // Rule's Speech Action going to "Shorten" [obs, 798/s8]), by the traced value.
                (ControlKind::Choice { options, selected }, Role::ComboBox)
                    if plain(&control.label) == name =>
                {
                    let value = focus.value.as_deref().map(plain);
                    match options
                        .iter()
                        .position(|option| Some(plain(option)) == value)
                    {
                        Some(i) if i != *selected => {
                            *selected = i;
                            (true, Some(Value::Selected(i)))
                        }
                        _ => (true, None),
                    }
                }
                // A check box turned, as its window reports Space on it (Skim Reading's Bold
                // [obs, 799/s9]), by the traced state.
                (ControlKind::CheckBox { checked }, Role::CheckBox)
                    if plain(&control.label) == name =>
                {
                    match focus.states.checked {
                        Some(now) if now != *checked => {
                            *checked = now;
                            (true, Some(Value::Checked(now)))
                        }
                        _ => (true, None),
                    }
                }
                (ControlKind::Button { .. }, Role::Button) => (plain(&control.label) == name, None),
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
    /// "board" in JAWS Find's Find what [obs, 633/s2-s6]). `field` is the name the trace gives
    /// the text's field: the text of another of the dialog's fields is not the focused one's,
    /// though its window reports it a moment before the focus moves there (Add Dictionary
    /// Definition's "Sound:" and its "<None>", 2 ms before Tab's focus on it [obs, 1156/s12]).
    pub fn typed(&mut self, field: Option<&str>, typed: &str) -> Vec<FromUi> {
        if self.file_open.is_some() {
            return Vec::new();
        }
        let Some(dialog) = &mut self.dialog else {
            return Vec::new();
        };
        let focused = self.focused;
        let elsewhere = field.filter(|name| !name.is_empty()).is_some_and(|name| {
            dialog
                .controls
                .iter()
                .enumerate()
                .any(|(i, c)| Some(i) != focused && plain(&c.label) == name)
        });
        if elsewhere {
            return Vec::new();
        }
        let Some(control) = focused.and_then(|i| dialog.controls.get_mut(i)) else {
            return Vec::new();
        };
        match &mut control.kind {
            ControlKind::ComboEdit { text, .. }
            | ControlKind::Edit { text }
            | ControlKind::SearchBox { text }
                if text != typed =>
            {
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
        if self.file_open.is_some() {
            if event.key == Key::Escape && event.modifiers.is_empty() {
                self.file_open = None;
            }
            return Vec::new();
        }
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
        // Alt+F4 closes the window, as its close box does: JAWS's Dictionary Manager, closed
        // so after Ctrl+S [obs, 1154/s16, s33].
        let alt_only = event.modifiers.alt
            && !event.modifiers.ctrl
            && !event.modifiers.shift
            && !event.modifiers.insert
            && !event.modifiers.win;
        if event.key == Key::F(4) && alt_only {
            return vec![FromUi::DialogClosed { id: dialog.id }];
        }
        if !event.modifiers.is_empty() {
            return Vec::new();
        }
        let focused = self.focused.and_then(|i| dialog.controls.get(i));
        // A letter alone on a button is its access key too, as a dialog's buttons take it: Y
        // answers Yes (Overwrite Clipboard [obs, 825/s19]).
        if let (Key::Alpha(letter), Some(ControlKind::Button { .. })) =
            (event.key, focused.map(|c| &c.kind))
        {
            return self.access_key(letter);
        }
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
        // While a row is renamed in place, Enter keeps the name and Escape drops it: the edit's
        // keys, not the dialog's [obs, 751/s20].
        if self.renaming.is_some() && matches!(event.key, Key::Enter | Key::Escape) {
            if event.key == Key::Escape {
                self.renaming = None;
            }
            return Vec::new();
        }
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
            // Delete and F2 on a check list's row; F2 renames it in place.
            Key::F(2) => {
                if let Some(ControlKind::CheckList { selected, .. }) = focused.map(|c| &c.kind) {
                    self.renaming = Some(*selected);
                }
                focused
                    .filter(|c| matches!(c.kind, ControlKind::CheckList { .. }))
                    .map(|c| FromUi::DialogKey {
                        id: dialog.id,
                        control: c.id.clone(),
                        key: okur_core::ui::DialogKey::Rename,
                    })
            }
            Key::Delete => focused
                .filter(|c| matches!(c.kind, ControlKind::CheckList { .. }))
                .map(|c| FromUi::DialogKey {
                    id: dialog.id,
                    control: c.id.clone(),
                    key: okur_core::ui::DialogKey::Delete,
                }),
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

    /// JAWS's Search for JAWS Commands is a page in a web view; Okur's dialog of the same
    /// title is a window of okur-ui's with the system's controls: its focus is on no page, so
    /// that a letter typed goes to its field and not to a page's quick keys [obs, 628/s7].
    #[test]
    fn a_jaws_dialog_in_a_web_view_stands_for_okurs_native_one() {
        let host = RecordingUiHost::default();
        let mut mirror = DialogMirror::default();
        host.send(ToUi::DialogShow {
            dialog: DialogSpec {
                id: 3,
                title: "Search for Okur Commands".to_owned(),
                controls: Vec::new(),
                keys: Vec::new(),
            },
        });
        mirror.sync(&host);
        let mut focus = FocusEvent::new(Role::Edit).with_name("Search for:");
        focus.window_title = Some("Search for JAWS Commands".to_owned());
        focus.app = Some("msedgewebview2.exe".to_owned());
        focus.in_document = true;
        focus.dom_id = Some("query".to_owned());
        mirror.as_okur_window(&mut focus);
        assert_eq!(
            focus.window_title.as_deref(),
            Some("Search for Okur Commands")
        );
        assert_eq!(focus.app.as_deref(), Some(OKUR_UI));
        assert!(!focus.in_document);
        assert_eq!(focus.dom_id, None);
        // A page's focus elsewhere is the page's.
        let mut page = FocusEvent::new(Role::Button).with_name("Home");
        page.window_title = Some("News - Google Chrome".to_owned());
        page.in_document = true;
        mirror.as_okur_window(&mut page);
        assert!(page.in_document);
    }

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
                keys: Vec::new(),
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
                keys: Vec::new(),
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
                    hidden: false,
                }],
                keys: Vec::new(),
            },
        });
        mirror.sync(&host);
        assert_eq!(
            mirror.typed(None, "b"),
            vec![FromUi::DialogChanged {
                id: 4,
                control: "what".to_owned(),
                value: Value::Text("b".to_owned()),
            }]
        );
        assert_eq!(mirror.typed(None, "b"), Vec::new());
    }

    /// The text of another field, reported a moment before Tab's focus reaches it, is not
    /// typed into the field still focused: Sound's "<None>" while Replacement Word has the
    /// focus [obs, 1156/s12].
    #[test]
    fn another_fields_text_is_not_typed_into_the_focused_one() {
        let field = |id: &str, label: &str, kind: ControlKind| Control {
            id: id.to_owned(),
            label: label.to_owned(),
            kind,
            enabled: true,
            hidden: false,
        };
        let host = RecordingUiHost::default();
        let mut mirror = DialogMirror::default();
        host.send(ToUi::DialogShow {
            dialog: DialogSpec {
                id: 7,
                title: "Add Dictionary Definition".to_owned(),
                controls: vec![
                    field(
                        "replacement",
                        "&Replacement Word:",
                        ControlKind::Edit {
                            text: String::new(),
                        },
                    ),
                    field(
                        "sound",
                        "&Sound:",
                        ControlKind::Note {
                            text: "<None>".to_owned(),
                        },
                    ),
                ],
                keys: Vec::new(),
            },
        });
        mirror.sync(&host);
        assert_eq!(mirror.typed(Some("Sound:"), "<None>"), Vec::new());
        assert_eq!(
            mirror.typed(Some("Replacement Word:"), "sum"),
            vec![FromUi::DialogChanged {
                id: 7,
                control: "replacement".to_owned(),
                value: Value::Text("sum".to_owned()),
            }]
        );
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
            hidden: false,
        };
        let host = RecordingUiHost::default();
        let mut mirror = DialogMirror::default();
        host.send(ToUi::DialogShow {
            dialog: DialogSpec {
                id: 1,
                keys: Vec::new(),
                title: "PlaceMarker List".to_owned(),
                controls: vec![
                    Control {
                        id: "markers".to_owned(),
                        label: "PlaceMarker".to_owned(),
                        kind: ControlKind::Table {
                            columns: vec!["PlaceMarker".to_owned()],
                            rows: vec![vec!["Water".to_owned()]],
                            selected: None,
                            edit_column: None,
                        },
                        enabled: true,
                        hidden: false,
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
                keys: Vec::new(),
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
