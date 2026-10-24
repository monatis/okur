//! Voice Adjustment (the Okur window's Options, Voices, Voice Adjustment...), as JAWS's: a
//! "Voice" group of Adjust (the voice context), Person, Rate, Punctuation and Volume, then OK
//! and Cancel (old-pc's walk, the redesign's §1.1), for all applications. Okur's voice has one
//! context until voice contexts come (the redesign's milestone 4): Adjust offers "All Contexts"
//! alone, and Profile Name, Synthesizer Language, Speak Sample and Save As come with profiles.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value};

use super::settings::Outcome;
use crate::messages::{msg, StaticMessage};
use crate::settings::registry::choices;
use crate::settings::{Punctuation, Settings};
use crate::settings_layers::Scope;

const ADJUST: &str = "adjust";
const PERSON: &str = "person";
const RATE: &str = "rate";
const PUNCTUATION: &str = "punctuation";
const VOLUME: &str = "volume";

/// The dialog's state: the settings for all applications as it opened, and as changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceAdjustment {
    pub id: u32,
    before: Settings,
    draft: Settings,
    /// The synthesizer's voices, by name.
    voices: Vec<String>,
}

impl VoiceAdjustment {
    pub fn new(id: u32, settings: Settings, voices: Vec<String>) -> Self {
        Self {
            id,
            before: settings.clone(),
            draft: settings,
            voices,
        }
    }

    /// The voices offered, the synthesizer's default first.
    fn people(&self) -> Vec<String> {
        std::iter::once(msg::settings_voice_default().to_string())
            .chain(self.voices.iter().cloned())
            .collect()
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: StaticMessage, kind: ControlKind| Control {
            id: id.to_owned(),
            label: label.to_string(),
            kind,
            enabled: true,
        };
        let voice = &self.draft.voice;
        let punctuation = choices::<Punctuation>();
        DialogSpec {
            id: self.id,
            title: msg::voice_adjustment_title().to_string(),
            controls: vec![
                control("voice", msg::voice_adjustment_group(), ControlKind::Group),
                control(
                    ADJUST,
                    msg::voice_adjustment_adjust(),
                    ControlKind::Choice {
                        options: vec![msg::voice_adjustment_all_contexts().to_string()],
                        selected: 0,
                    },
                ),
                control(
                    PERSON,
                    msg::voice_adjustment_person(),
                    ControlKind::Choice {
                        selected: voice
                            .voice
                            .as_ref()
                            .and_then(|name| self.voices.iter().position(|v| v == name))
                            .map_or(0, |i| i + 1),
                        options: self.people(),
                    },
                ),
                control(
                    RATE,
                    msg::voice_adjustment_rate(),
                    ControlKind::Slider {
                        min: -10,
                        max: 10,
                        value: voice.rate,
                    },
                ),
                control(
                    PUNCTUATION,
                    msg::voice_adjustment_punctuation(),
                    ControlKind::Choice {
                        selected: punctuation
                            .iter()
                            .position(|c| {
                                serde_json::to_value(voice.punctuation).ok().as_ref()
                                    == Some(&c.value)
                            })
                            .unwrap_or(0),
                        options: punctuation.iter().map(|c| c.label.to_string()).collect(),
                    },
                ),
                control(
                    VOLUME,
                    msg::voice_adjustment_volume(),
                    ControlKind::Slider {
                        min: 0,
                        max: 100,
                        value: i32::from(voice.volume),
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

    /// What the dialog's window reported.
    pub fn handle(&mut self, message: &FromUi) -> Outcome {
        match message {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                let voice = &mut self.draft.voice;
                match (control.as_str(), value) {
                    (PERSON, Value::Selected(i)) => {
                        voice.voice = i.checked_sub(1).and_then(|i| self.voices.get(i)).cloned();
                    }
                    (RATE, Value::Number(n)) => voice.rate = (*n).clamp(-10, 10),
                    (VOLUME, Value::Number(n)) => {
                        voice.volume = u16::try_from((*n).clamp(0, 100)).unwrap_or(100);
                    }
                    (PUNCTUATION, Value::Selected(i)) => {
                        if let Some(level) = choices::<Punctuation>()
                            .into_iter()
                            .nth(*i)
                            .and_then(|c| serde_json::from_value(c.value).ok())
                        {
                            voice.punctuation = level;
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

    /// JAWS's Voice Adjustment's "Voice" group: Adjust, Person, Rate, Punctuation, Volume, in
    /// its Tab order [obs, the walk of Voice Adjustment].
    #[test]
    fn voice_adjustment_is_jaws_voice_group() {
        let dialog = VoiceAdjustment::new(5, Settings::default(), vec!["Zira".into()]);
        let spec = dialog.spec();
        assert_eq!(spec.title, "Voice Adjustment");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Voice",
                "Adjust:",
                "Person:",
                "Rate:",
                "Punctuation:",
                "Volume:",
                "OK",
                "Cancel"
            ]
        );
    }

    /// The rate, the voice and the punctuation changed apply for all applications with OK.
    #[test]
    fn voice_adjustment_applies_for_all_applications() {
        let mut dialog = VoiceAdjustment::new(5, Settings::default(), vec!["Zira".into()]);
        for (control, value) in [
            (RATE, Value::Number(4)),
            (PERSON, Value::Selected(1)),
            (PUNCTUATION, Value::Selected(0)),
        ] {
            dialog.handle(&FromUi::DialogChanged {
                id: 5,
                control: control.into(),
                value,
            });
        }
        let applied = dialog.handle(&FromUi::DialogPressed {
            id: 5,
            control: "ok".into(),
        });
        assert!(matches!(
            applied,
            Outcome::Apply { scope: Scope::Global, ref settings, .. }
                if settings.voice.rate == 4
                    && settings.voice.voice.as_deref() == Some("Zira")
                    && settings.voice.punctuation == Punctuation::None
        ));
    }
}
