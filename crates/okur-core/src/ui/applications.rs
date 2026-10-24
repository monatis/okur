//! Okur's settings for every application at once, from the Okur window's Options, as JAWS's:
//! - Manage Application Settings: "Manage Application Settings dialog | Application Settings
//!   List box | Chrome checked | 1 of 5"; by Tab, Enable All, Disable All, Remove Selected, OK,
//!   Cancel [obs, the walk of JAWS's Options]. Each application with settings of its own, checked
//!   while they are used; Remove Selected forgets the selected one's.
//! - Restore to Factory Settings: "This option will delete all your current settings and
//!   restart JAWS. Are you sure you want to continue? | Yes Button", then No [obs, the same
//!   walk]. Okur's defaults apply at once, with no restart, so its question leaves "and restart"
//!   out.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use crate::messages::msg;

const LIST: &str = "applications";
const ENABLE_ALL: &str = "enable_all";
const DISABLE_ALL: &str = "disable_all";
const REMOVE: &str = "remove_selected";

/// What the engine does after a message from Manage Application Settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    Send(Vec<ToUi>),
    /// OK: these applications' settings are not used, and these are forgotten.
    Apply {
        disabled: Vec<String>,
        removed: Vec<String>,
    },
    /// Cancel, Escape: nothing changes.
    Closed,
}

/// Manage Application Settings while it is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManageApplications {
    pub id: u32,
    /// The applications with settings of their own, by settings name, and whether they are
    /// used.
    applications: Vec<(String, bool)>,
    removed: Vec<String>,
    selected: usize,
}

impl ManageApplications {
    /// The dialog over `applications` (their settings names), those in `disabled` unchecked.
    pub fn new(id: u32, applications: Vec<String>, disabled: &[String]) -> Self {
        Self {
            id,
            applications: applications
                .into_iter()
                .map(|app| {
                    let used = !disabled.contains(&app);
                    (app, used)
                })
                .collect(),
            removed: Vec::new(),
            selected: 0,
        }
    }

    fn rows(&self) -> Vec<Vec<String>> {
        self.applications
            .iter()
            .map(|(app, _)| vec![crate::settings_layers::application_name(app)])
            .collect()
    }

    fn checked(&self) -> Vec<bool> {
        self.applications.iter().map(|(_, used)| *used).collect()
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
        };
        let button =
            |id: &str, label: String| control(id, label, ControlKind::Button { default: false });
        DialogSpec {
            id: self.id,
            title: msg::applications_title().to_string(),
            controls: vec![
                control(
                    LIST,
                    msg::applications_list().to_string(),
                    ControlKind::CheckList {
                        columns: Vec::new(),
                        rows: self.rows(),
                        checked: self.checked(),
                        selected: self.selected,
                        renamable: false,
                    },
                ),
                button(ENABLE_ALL, msg::applications_enable_all().to_string()),
                button(DISABLE_ALL, msg::applications_disable_all().to_string()),
                button(REMOVE, msg::applications_remove().to_string()),
                control(
                    "ok",
                    msg::settings_ok().to_string(),
                    ControlKind::Button { default: true },
                ),
                button("cancel", msg::settings_cancel().to_string()),
            ],
            keys: Vec::new(),
        }
    }

    /// The list as it is now.
    fn refill(&self) -> ToUi {
        ToUi::DialogRows {
            id: self.id,
            control: LIST.to_owned(),
            rows: self.rows(),
            checked: self.checked(),
            selected: self.selected,
        }
    }

    pub fn handle(&mut self, message: &FromUi) -> Outcome {
        match message {
            FromUi::DialogChanged { id, control, value } if *id == self.id && control == LIST => {
                match value {
                    Value::Selected(i) => self.selected = *i,
                    Value::ItemChecked(i, checked) => {
                        if let Some((_, used)) = self.applications.get_mut(*i) {
                            *used = *checked;
                        }
                    }
                    _ => {}
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                ENABLE_ALL | DISABLE_ALL => {
                    let used = control == ENABLE_ALL;
                    self.applications.iter_mut().for_each(|(_, u)| *u = used);
                    Outcome::Send(vec![self.refill()])
                }
                REMOVE if self.selected < self.applications.len() => {
                    let (app, _) = self.applications.remove(self.selected);
                    self.removed.push(app);
                    self.selected = self.selected.min(self.applications.len().saturating_sub(1));
                    Outcome::Send(vec![self.refill()])
                }
                "ok" => Outcome::Apply {
                    disabled: self
                        .applications
                        .iter()
                        .filter(|(_, used)| !used)
                        .map(|(app, _)| app.clone())
                        .collect(),
                    removed: self.removed.clone(),
                },
                _ => Outcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

/// Restore to Factory Settings' question, as dialog `id`: the message, Yes (the default), No.
pub fn restore_question(id: u32) -> DialogSpec {
    let control = |id: &str, label: String, kind: ControlKind| Control {
        id: id.to_owned(),
        label,
        kind,
        enabled: true,
    };
    DialogSpec {
        id,
        title: msg::restore_factory_title().to_string(),
        controls: vec![
            control(
                "question",
                String::new(),
                ControlKind::Message {
                    text: msg::restore_factory_question().to_string(),
                },
            ),
            control(
                "yes",
                msg::restore_factory_yes().to_string(),
                ControlKind::Button { default: true },
            ),
            control(
                "cancel",
                msg::restore_factory_no().to_string(),
                ControlKind::Button { default: false },
            ),
        ],
        keys: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dialog() -> ManageApplications {
        ManageApplications::new(
            3,
            vec!["chrome".into(), "notepad".into(), "winword".into()],
            &["notepad".to_owned()],
        )
    }

    /// The list of applications, checked while their settings are used, then Enable All,
    /// Disable All, Remove Selected, OK, Cancel, as JAWS's [obs, the walk of JAWS's Options].
    #[test]
    fn manage_application_settings_is_jaws_dialog() {
        let spec = dialog().spec();
        assert_eq!(spec.title, "Manage Application Settings");
        let ids: Vec<&str> = spec.controls.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, [LIST, ENABLE_ALL, DISABLE_ALL, REMOVE, "ok", "cancel"]);
        assert!(spec
            .controls
            .iter()
            .any(|c| c.label == "Application Settings"
                && matches!(&c.kind, ControlKind::CheckList { rows, checked, .. }
                if rows.first() == Some(&vec!["Chrome".to_owned()])
                    && checked == &[true, false, true])));
    }

    /// OK tells which applications' settings are not used and which are forgotten.
    #[test]
    fn ok_tells_what_is_disabled_and_removed() {
        let mut dialog = dialog();
        let pressed = |control: &str| FromUi::DialogPressed {
            id: 3,
            control: control.into(),
        };
        dialog.handle(&pressed(ENABLE_ALL));
        dialog.handle(&FromUi::DialogChanged {
            id: 3,
            control: LIST.into(),
            value: Value::ItemChecked(0, false),
        });
        dialog.handle(&FromUi::DialogChanged {
            id: 3,
            control: LIST.into(),
            value: Value::Selected(2),
        });
        dialog.handle(&pressed(REMOVE));
        assert_eq!(
            dialog.handle(&pressed("ok")),
            Outcome::Apply {
                disabled: vec!["chrome".into()],
                removed: vec!["winword".into()],
            }
        );
    }

    /// Restore to Factory Settings asks first, in JAWS's words but for the restart Okur does not
    /// need.
    #[test]
    fn restore_to_factory_settings_asks_first() {
        let spec = restore_question(4);
        assert_eq!(spec.title, "Restore to Factory Settings");
        assert!(spec.controls.iter().any(|c| matches!(&c.kind,
            ControlKind::Message { text } if text == "This option will delete all your current settings. Are you sure you want to continue?")));
    }
}
