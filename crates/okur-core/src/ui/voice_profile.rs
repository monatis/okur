//! Select a Voice Profile (Insert+Ctrl+S; the Okur window's Options, Voices), as JAWS's:
//! "Select a Voice Profile dialog | Default: Combo box | SAPI 5X 64 | 5 of 5"; by Tab, "For
//! notepad: Combo box | Use Default | 1 of 6", "OK Button", "Cancel Button" [obs, the walk of
//! Select a Voice Profile]. Default chooses the profile for all applications; "For
//! <application>" the application's, its first item "Use Default" following Default. A profile is
//! a synthesizer's own (Okur's factory profiles, by the synthesizer's name) or one saved by name.

use std::collections::BTreeMap;

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value};

use crate::messages::msg;
use crate::settings::registry::Choices;
use crate::settings::{Synthesizer, VoiceSettings};

const DEFAULT: &str = "default";
const APPLICATION: &str = "application";

/// The profiles offered: each synthesizer's own, then those saved by name.
pub fn profiles(saved: &BTreeMap<String, VoiceSettings>) -> Vec<(String, VoiceSettings)> {
    Synthesizer::ALL
        .iter()
        .filter(|&&synthesizer| synthesizer != Synthesizer::NoSpeech)
        .map(|&synthesizer| {
            (
                synthesizer.label().to_string(),
                VoiceSettings {
                    synthesizer,
                    ..VoiceSettings::default()
                },
            )
        })
        .chain(
            saved
                .iter()
                .map(|(name, voice)| (name.clone(), voice.clone())),
        )
        .collect()
}

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// OK: the profile chosen for all applications, if another; and the application's, if
    /// another: a profile by name, or none for "Use Default".
    Choose {
        default: Option<String>,
        application: Option<(String, Option<String>)>,
    },
    /// Cancel, Escape: nothing changes.
    Closed,
}

/// The dialog while it is open.
#[derive(Debug, Clone)]
pub struct VoiceProfileDialog {
    pub id: u32,
    names: Vec<String>,
    /// The application in front, by its settings name, and the profile it has of its own.
    application: Option<(String, Option<String>)>,
    default: usize,
    default_was: usize,
    /// "Use Default" (0) or a profile (its index + 1).
    for_application: usize,
    for_application_was: usize,
}

impl VoiceProfileDialog {
    /// The dialog over `profiles`, the voice for all applications being `global`, and
    /// `application`'s own profile if it has one.
    pub fn new(
        id: u32,
        profiles: &[(String, VoiceSettings)],
        global: &VoiceSettings,
        application: Option<(String, Option<String>)>,
    ) -> Self {
        let names: Vec<String> = profiles.iter().map(|(name, _)| name.clone()).collect();
        let named = |name: &Option<String>| {
            name.as_ref()
                .and_then(|name| names.iter().position(|n| n == name))
        };
        // The profile chosen, else the synthesizer's own.
        let default = named(&global.profile)
            .or_else(|| {
                profiles
                    .iter()
                    .position(|(_, p)| p.synthesizer == global.synthesizer)
            })
            .unwrap_or(0);
        let for_application = application
            .as_ref()
            .and_then(|(_, own)| named(own))
            .map_or(0, |i| i + 1);
        Self {
            id,
            names,
            application,
            default,
            default_was: default,
            for_application,
            for_application_was: for_application,
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
        };
        let mut controls = vec![control(
            DEFAULT,
            msg::voice_profile_default().to_string(),
            ControlKind::Choice {
                options: self.names.clone(),
                selected: self.default,
            },
        )];
        if let Some((application, _)) = &self.application {
            controls.push(control(
                APPLICATION,
                msg::voice_profile_for(application.as_str()).render(),
                ControlKind::Choice {
                    options: std::iter::once(msg::voice_profile_use_default().to_string())
                        .chain(self.names.iter().cloned())
                        .collect(),
                    selected: self.for_application,
                },
            ));
        }
        controls.push(control(
            "ok",
            msg::settings_ok().to_string(),
            ControlKind::Button { default: true },
        ));
        controls.push(control(
            "cancel",
            msg::settings_cancel().to_string(),
            ControlKind::Button { default: false },
        ));
        DialogSpec {
            id: self.id,
            title: msg::voice_profile_title().to_string(),
            controls,
            keys: Vec::new(),
        }
    }

    pub fn handle(&mut self, message: &FromUi) -> Outcome {
        match message {
            FromUi::DialogChanged {
                id,
                control,
                value: Value::Selected(i),
            } if *id == self.id => {
                match control.as_str() {
                    DEFAULT if *i < self.names.len() => self.default = *i,
                    APPLICATION if *i <= self.names.len() => self.for_application = *i,
                    _ => {}
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id && control == "ok" => {
                let default = (self.default != self.default_was)
                    .then(|| self.names.get(self.default).cloned())
                    .flatten();
                let application = self
                    .application
                    .as_ref()
                    .filter(|_| self.for_application != self.for_application_was)
                    .map(|(application, _)| {
                        let profile = self
                            .for_application
                            .checked_sub(1)
                            .and_then(|i| self.names.get(i))
                            .cloned();
                        (application.clone(), profile)
                    });
                Outcome::Choose {
                    default,
                    application,
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

    fn dialog(own: Option<&str>) -> VoiceProfileDialog {
        let mut saved = BTreeMap::new();
        saved.insert(
            "Fast".to_owned(),
            VoiceSettings {
                rate: 6,
                ..VoiceSettings::default()
            },
        );
        VoiceProfileDialog::new(
            4,
            &profiles(&saved),
            &VoiceSettings {
                synthesizer: Synthesizer::Sapi5,
                ..VoiceSettings::default()
            },
            Some(("notepad".to_owned(), own.map(str::to_owned))),
        )
    }

    fn choice(spec: &DialogSpec, id: &str) -> Option<(String, Vec<String>, usize)> {
        spec.controls
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| match &c.kind {
                ControlKind::Choice { options, selected } => {
                    Some((c.label.clone(), options.clone(), *selected))
                }
                _ => None,
            })
    }

    /// Default, then "For notepad" with "Use Default" first, OK and Cancel, as JAWS's; the
    /// profile in use selected: the synthesizer's own when none was chosen [obs, the walk of
    /// Select a Voice Profile].
    #[test]
    fn select_a_voice_profile_is_jaws_dialog() {
        let spec = dialog(None).spec();
        assert_eq!(spec.title, "Select a Voice Profile");
        let ids: Vec<&str> = spec.controls.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, [DEFAULT, APPLICATION, "ok", "cancel"]);
        assert_eq!(
            choice(&spec, DEFAULT),
            Some((
                "Default:".to_owned(),
                vec![
                    "Automatic".to_owned(),
                    "OneCore".to_owned(),
                    "SAPI 5".to_owned(),
                    "Fast".to_owned()
                ],
                2
            ))
        );
        assert_eq!(
            choice(&spec, APPLICATION).map(|(label, options, selected)| (
                label,
                options.first().cloned(),
                selected
            )),
            Some(("For notepad:".to_owned(), Some("Use Default".to_owned()), 0))
        );
    }

    /// OK tells what changed: a profile for all applications, the application's by name or
    /// "Use Default"; nothing changed, nothing.
    #[test]
    fn ok_chooses_what_changed() {
        let mut chosen = dialog(Some("Fast"));
        let ok = FromUi::DialogPressed {
            id: 4,
            control: "ok".into(),
        };
        assert_eq!(
            chosen.clone().handle(&ok),
            Outcome::Choose {
                default: None,
                application: None
            }
        );
        chosen.handle(&FromUi::DialogChanged {
            id: 4,
            control: DEFAULT.into(),
            value: Value::Selected(1),
        });
        chosen.handle(&FromUi::DialogChanged {
            id: 4,
            control: APPLICATION.into(),
            value: Value::Selected(0),
        });
        assert_eq!(
            chosen.handle(&ok),
            Outcome::Choose {
                default: Some("OneCore".to_owned()),
                application: Some(("notepad".to_owned(), None))
            }
        );
    }
}
