//! Basics (the Okur window's Options, Basics...), as JAWS's "Basic Settings": Tutor Messages,
//! Access Key, Prompt to confirm when exiting, Show Virtual Viewer on screen, Use Keyboard
//! Layout, OK and Cancel, in JAWS's Tab order (old-pc's walk, the redesign's §1.1). For all
//! applications. Of JAWS's choices only those Okur has are offered: Tutor Messages' "Announce
//! Custom Messages Only" and Access Key's "Speak Menus Only" and "Speak Dialogs Only" come with
//! their behavior.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value};

use super::settings::Outcome;
use crate::messages::{msg, StaticMessage};
use crate::settings::registry::{self, choices};
use crate::settings::{KeyboardLayout, OutputMode, OutputType, Settings, ViewerDisplay};
use crate::settings_layers::Scope;

const TUTOR: &str = "tutor";
const ACCESS_KEY: &str = "access_key";
const CONFIRM_EXIT: &str = "confirm_exit";
const VIEWER: &str = "viewer";
const LAYOUT: &str = "layout";

/// The Basics dialog's state: the settings for all applications as it opened, and as changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicsDialog {
    pub id: u32,
    before: Settings,
    draft: Settings,
}

impl BasicsDialog {
    /// The dialog over `settings`, those for all applications.
    pub fn new(id: u32, settings: Settings) -> Self {
        Self {
            id,
            before: settings.clone(),
            draft: settings,
        }
    }

    /// Whether `output` is on at every verbosity level.
    fn everywhere(&self, output: OutputType) -> bool {
        (0..3).all(|level| self.draft.verbosity.mode_at(output, level) == OutputMode::On)
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: StaticMessage, kind: ControlKind| Control {
            id: id.to_owned(),
            label: label.to_string(),
            kind,
            enabled: true,
            hidden: false,
        };
        let radios = |options: [StaticMessage; 2], on: bool| ControlKind::Radios {
            options: options.iter().map(ToString::to_string).collect(),
            selected: usize::from(on),
        };
        let layouts = choices::<KeyboardLayout>();
        let layout = registry::get(&self.draft, "keyboard.layout");
        DialogSpec {
            id: self.id,
            title: msg::basics_title().to_string(),
            controls: vec![
                control(
                    TUTOR,
                    msg::basics_tutor(),
                    radios(
                        [msg::basics_tutor_off(), msg::basics_tutor_on()],
                        self.everywhere(OutputType::Hint),
                    ),
                ),
                control(
                    ACCESS_KEY,
                    msg::basics_access_key(),
                    radios(
                        [msg::basics_access_key_off(), msg::basics_access_key_all()],
                        self.everywhere(OutputType::AccessKey),
                    ),
                ),
                control(
                    CONFIRM_EXIT,
                    msg::basics_confirm_exit(),
                    ControlKind::CheckBox {
                        checked: self.draft.confirm_exit,
                    },
                ),
                control(
                    VIEWER,
                    msg::basics_viewer(),
                    ControlKind::CheckBox {
                        checked: self.draft.viewer_display == ViewerDisplay::OnScreen,
                    },
                ),
                control(
                    LAYOUT,
                    msg::basics_layout(),
                    ControlKind::Choice {
                        selected: layouts
                            .iter()
                            .position(|c| Some(&c.value) == layout.as_ref())
                            .unwrap_or(0),
                        options: layouts.iter().map(|c| c.label.to_string()).collect(),
                    },
                ),
                control(
                    "ok",
                    msg::settings_ok(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "cancel",
                    msg::settings_cancel(),
                    ControlKind::Button { default: false },
                ),
            ],
            keys: Vec::new(),
        }
    }

    /// The settings as the dialog shows them now.
    pub fn draft(&self) -> &Settings {
        &self.draft
    }

    /// What the dialog's window reported.
    pub fn handle(&mut self, message: &FromUi) -> Outcome {
        let every = |on: bool| {
            if on {
                [OutputMode::On; 3]
            } else {
                [OutputMode::Off; 3]
            }
        };
        match message {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    (TUTOR, Value::Selected(i)) => self
                        .draft
                        .verbosity
                        .set_modes(OutputType::Hint, every(*i == 1)),
                    (ACCESS_KEY, Value::Selected(i)) => self
                        .draft
                        .verbosity
                        .set_modes(OutputType::AccessKey, every(*i == 1)),
                    (CONFIRM_EXIT, Value::Checked(on)) => self.draft.confirm_exit = *on,
                    (VIEWER, Value::Checked(on)) => {
                        self.draft.viewer_display = if *on {
                            ViewerDisplay::OnScreen
                        } else {
                            ViewerDisplay::SpeechOnly
                        }
                    }
                    (LAYOUT, Value::Selected(i)) => {
                        if let Some(choice) = choices::<KeyboardLayout>().into_iter().nth(*i) {
                            registry::set(&mut self.draft, "keyboard.layout", choice.value);
                        }
                    }
                    _ => {}
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id && control == "ok" => {
                Outcome::Apply {
                    scope: Scope::Global,
                    before: Box::new(self.before.clone()),
                    settings: Box::new(self.draft.clone()),
                    close: true,
                    temporary: Vec::new(),
                }
            }
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "Basic Settings": Tutor Messages first, then Access Key, the check boxes, the layout,
    /// OK and Cancel, as JAWS's Tab order [obs, the walk of Basics].
    #[test]
    fn basics_is_jaws_basic_settings() {
        let spec = BasicsDialog::new(3, Settings::default()).spec();
        assert_eq!(spec.title, "Basic Settings");
        let ids: Vec<&str> = spec.controls.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                TUTOR,
                ACCESS_KEY,
                CONFIRM_EXIT,
                VIEWER,
                LAYOUT,
                "ok",
                "cancel"
            ]
        );
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels.get(..5),
            Some(
                [
                    "Tutor Messages",
                    "Access Key",
                    "Prompt to confirm when exiting Okur",
                    "Show Virtual Viewer on screen",
                    "Use Keyboard Layout:"
                ]
                .as_slice()
            )
        );
    }

    /// Its changes apply for all applications with OK; Tutor Messages and Access Key at every
    /// verbosity level.
    #[test]
    fn basics_applies_for_all_applications() {
        let mut dialog = BasicsDialog::new(3, Settings::default());
        for (control, value) in [
            (TUTOR, Value::Selected(0)),
            (CONFIRM_EXIT, Value::Checked(false)),
            (LAYOUT, Value::Selected(1)),
        ] {
            dialog.handle(&FromUi::DialogChanged {
                id: 3,
                control: control.into(),
                value,
            });
        }
        let applied = dialog.handle(&FromUi::DialogPressed {
            id: 3,
            control: "ok".into(),
        });
        assert!(
            matches!(applied, Outcome::Apply { .. }),
            "not applied: {applied:?}"
        );
        let Outcome::Apply {
            scope, settings, ..
        } = applied
        else {
            return;
        };
        assert_eq!(scope, Scope::Global);
        assert!(!settings.confirm_exit);
        assert_eq!(settings.keyboard.layout, KeyboardLayout::Laptop);
        assert!((0..3).all(|l| settings.verbosity.mode_at(OutputType::Hint, l) == OutputMode::Off));
    }
}
