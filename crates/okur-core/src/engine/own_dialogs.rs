//! Okur's own dialogs as the engine asked okur-ui to show them: what the platform may not say
//! of them. GTK 4 marks no button the default (the Linux lab's 222: no button had AT-SPI's
//! IS_DEFAULT), so on Linux a focus in Okur's Links List was in no dialog box, and Insert+E
//! said "Not in a dialog box" there; Windows' dialogs say both. A focus in one of Okur's
//! windows whose title is a dialog it showed is in a dialog box, and its default button is the
//! one the dialog's spec makes the default, where the platform names none.

use std::sync::{Arc, Mutex};

use super::held_keys::AskedDialog;
use super::{ScreenReaderEngine, OKUR_WINDOWS};
use crate::event::FocusEvent;
use crate::ui::{ControlKind, DialogSpec, ToUi, UiHost};

/// The dialogs okur-ui shows, as the engine last told it of them.
pub(super) type OpenDialogs = Arc<Mutex<Vec<DialogSpec>>>;

/// Okur's windows as the engine gives them messages: each dialog asked to show noted for the
/// keys held for it ([`super::held_keys`]), and the dialogs kept as they are shown.
pub(super) struct DialogWatch {
    pub(super) inner: Box<dyn UiHost>,
    pub(super) asked: Arc<Mutex<Option<AskedDialog>>>,
    pub(super) open: OpenDialogs,
}

impl UiHost for DialogWatch {
    fn send(&self, message: ToUi) {
        if let ToUi::DialogShow { dialog } = &message {
            if let Ok(mut asked) = self.asked.lock() {
                *asked = Some(AskedDialog {
                    id: dialog.id,
                    title: dialog.title.clone(),
                });
            }
        }
        if let Ok(mut open) = self.open.lock() {
            kept(&mut open, &message);
        }
        self.inner.send(message);
    }
}

/// `open` with what `message` changes in it: a dialog shown, closed, its default or a label.
fn kept(open: &mut Vec<DialogSpec>, message: &ToUi) {
    match message {
        ToUi::DialogShow { dialog } => {
            open.retain(|d| d.id != dialog.id);
            open.push(dialog.clone());
        }
        ToUi::DialogClose { id } => open.retain(|d| d.id != *id),
        ToUi::DialogDefault { id, control } => {
            for dialog in open.iter_mut().filter(|d| d.id == *id) {
                for c in &mut dialog.controls {
                    if let ControlKind::Button { default } = &mut c.kind {
                        *default = c.id == *control;
                    }
                }
            }
        }
        ToUi::DialogLabel { id, control, label } => {
            let controls = open
                .iter_mut()
                .filter(|d| d.id == *id)
                .flat_map(|d| d.controls.iter_mut());
            for c in controls.filter(|c| c.id == *control) {
                c.label.clone_from(label);
            }
        }
        _ => {}
    }
}

/// A label as it is shown: its access key's mark gone, "&&" a plain "&".
fn shown(label: &str) -> String {
    let mut out = String::new();
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '&' if chars.peek() == Some(&'&') => {
                chars.next();
                out.push('&');
            }
            '&' => {}
            c => out.push(c),
        }
    }
    out
}

impl ScreenReaderEngine {
    /// okur-ui closed a dialog itself (its close box, Escape).
    pub(super) fn own_dialog_closed(&mut self, id: u32) {
        if let Ok(mut open) = self.open_dialogs.lock() {
            open.retain(|d| d.id != id);
        }
    }

    /// `focus` in one of Okur's dialogs, told it is in a dialog box and its default button
    /// where the platform does not say; none when nothing is to be told.
    pub(super) fn as_own_dialog_focus(&self, focus: &FocusEvent) -> Option<FocusEvent> {
        let okur_windows = focus
            .app
            .as_deref()
            .map(crate::settings_layers::application_id)
            .is_some_and(|app| app == OKUR_WINDOWS);
        if !okur_windows || (focus.in_dialog_box && focus.default_button.is_some()) {
            return None;
        }
        let title = focus.window_title.as_deref()?;
        let open = self.open_dialogs.lock().ok()?;
        let dialog = open.iter().rev().find(|d| d.title == title)?;
        let default = dialog.controls.iter().find_map(|c| match c.kind {
            ControlKind::Button { default: true } => Some(shown(&c.label)),
            _ => None,
        });
        let mut told = focus.clone();
        told.in_dialog_box = true;
        if told.default_button.is_none() {
            told.default_button = default;
        }
        (told != *focus).then_some(told)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Control;

    fn button(id: &str, label: &str, default: bool) -> Control {
        Control {
            id: id.to_owned(),
            label: label.to_owned(),
            kind: ControlKind::Button { default },
            enabled: true,
            hidden: false,
        }
    }

    /// Shown, its default moved and a label changed, then closed: the dialogs kept follow.
    #[test]
    fn the_dialogs_kept_follow_what_okur_ui_is_told() {
        let mut open = Vec::new();
        let dialog = DialogSpec {
            id: 7,
            title: "PlaceMarker List".to_owned(),
            controls: vec![
                button("add", "&Add", true),
                button("move", "&Move To", false),
            ],
            keys: Vec::new(),
        };
        kept(&mut open, &ToUi::DialogShow { dialog });
        kept(
            &mut open,
            &ToUi::DialogDefault {
                id: 7,
                control: "move".to_owned(),
            },
        );
        kept(
            &mut open,
            &ToUi::DialogLabel {
                id: 7,
                control: "move".to_owned(),
                label: "Move &To && Close".to_owned(),
            },
        );
        let default = open.first().and_then(|d| {
            d.controls.iter().find_map(|c| match c.kind {
                ControlKind::Button { default: true } => Some(shown(&c.label)),
                _ => None,
            })
        });
        assert_eq!(default.as_deref(), Some("Move To & Close"));
        kept(&mut open, &ToUi::DialogClose { id: 7 });
        assert!(open.is_empty());
    }
}
