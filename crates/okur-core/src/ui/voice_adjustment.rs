//! Voice Adjustment (the Okur window's Options, Voices, Voice Adjustment...), as JAWS's: a
//! "Voice" group of Adjust (the voice context), Person, Rate, Punctuation and Volume, then OK
//! and Cancel (old-pc's walk, the redesign's §1.1), for all applications. Adjust chooses All
//! Contexts or one of the voice contexts ([`VoiceContext`]): a context shows its own Person,
//! Rate and Punctuation, and no Volume, which is All Contexts' alone [obs, the walk of Voice
//! Adjustment's contexts]; a change in All Contexts is every context's. Profile Name, Synthesizer
//! Language, Speak Sample, Save As and a context's Pitch come with voice profiles (the
//! redesign's milestone 4).

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use super::settings::Outcome;
use crate::messages::{msg, StaticMessage};
use crate::settings::registry::choices;
use crate::settings::{Punctuation, Settings, VoiceContext, VoiceSettings};
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
    /// The context Adjust shows; none, All Contexts.
    context: Option<VoiceContext>,
}

impl VoiceAdjustment {
    pub fn new(id: u32, settings: Settings, voices: Vec<String>) -> Self {
        Self {
            id,
            before: settings.clone(),
            draft: settings,
            voices,
            context: None,
        }
    }

    /// The voices offered, the synthesizer's default first.
    fn people(&self) -> Vec<String> {
        std::iter::once(msg::settings_voice_default().to_string())
            .chain(self.voices.iter().cloned())
            .collect()
    }

    /// Adjust's choices: All Contexts, then each context [obs, the walk of Voice Adjustment's
    /// contexts].
    fn contexts() -> Vec<String> {
        std::iter::once(msg::voice_adjustment_all_contexts())
            .chain(VoiceContext::ALL.iter().map(|&context| match context {
                VoiceContext::PcCursor => msg::voice_adjustment_pc_cursor(),
                VoiceContext::OkurCursor => msg::voice_adjustment_okur_cursor(),
                VoiceContext::Keyboard => msg::voice_adjustment_keyboard(),
                VoiceContext::Message => msg::voice_adjustment_message(),
            }))
            .map(|label| label.to_string())
            .collect()
    }

    /// The voice the controls show: All Contexts', or the context's.
    fn shown(&self) -> VoiceSettings {
        match self.context {
            Some(context) => self.draft.voice.in_context(context),
            None => self.draft.voice.clone(),
        }
    }

    fn person(&self, voice: &VoiceSettings) -> usize {
        voice
            .voice
            .as_ref()
            .and_then(|name| self.voices.iter().position(|v| v == name))
            .map_or(0, |i| i + 1)
    }

    fn punctuation(voice: &VoiceSettings) -> usize {
        choices::<Punctuation>()
            .iter()
            .position(|c| serde_json::to_value(voice.punctuation).ok().as_ref() == Some(&c.value))
            .unwrap_or(0)
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: StaticMessage, kind: ControlKind| Control {
            id: id.to_owned(),
            label: label.to_string(),
            kind,
            enabled: true,
        };
        let voice = self.shown();
        DialogSpec {
            id: self.id,
            title: msg::voice_adjustment_title().to_string(),
            controls: vec![
                control("voice", msg::voice_adjustment_group(), ControlKind::Group),
                control(
                    ADJUST,
                    msg::voice_adjustment_adjust(),
                    ControlKind::Choice {
                        options: Self::contexts(),
                        selected: self.adjust_index(),
                    },
                ),
                control(
                    PERSON,
                    msg::voice_adjustment_person(),
                    ControlKind::Choice {
                        selected: self.person(&voice),
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
                        selected: Self::punctuation(&voice),
                        options: choices::<Punctuation>()
                            .iter()
                            .map(|c| c.label.to_string())
                            .collect(),
                    },
                ),
                Control {
                    enabled: self.context.is_none(),
                    ..control(
                        VOLUME,
                        msg::voice_adjustment_volume(),
                        ControlKind::Slider {
                            min: 0,
                            max: 100,
                            value: i32::from(voice.volume),
                        },
                    )
                },
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

    fn adjust_index(&self) -> usize {
        self.context
            .and_then(|c| VoiceContext::ALL.iter().position(|&a| a == c))
            .map_or(0, |i| i + 1)
    }

    /// The controls set to the context chosen: its values, and Volume for All Contexts only.
    fn show_context(&self) -> Vec<ToUi> {
        let voice = self.shown();
        let set = |control: &str, value: Value| ToUi::DialogSet {
            id: self.id,
            control: control.to_owned(),
            value,
        };
        vec![
            set(PERSON, Value::Selected(self.person(&voice))),
            set(RATE, Value::Number(voice.rate)),
            set(PUNCTUATION, Value::Selected(Self::punctuation(&voice))),
            ToUi::DialogEnable {
                id: self.id,
                control: VOLUME.to_owned(),
                enabled: self.context.is_none(),
            },
        ]
    }

    /// What the dialog's window reported.
    pub fn handle(&mut self, message: &FromUi) -> Outcome {
        match message {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                if let (ADJUST, Value::Selected(i)) = (control.as_str(), value) {
                    self.context = i
                        .checked_sub(1)
                        .and_then(|i| VoiceContext::ALL.get(i))
                        .copied();
                    return Outcome::Send(self.show_context());
                }
                let person = |i: &usize| i.checked_sub(1).and_then(|i| self.voices.get(i)).cloned();
                let punctuation = |i: &usize| {
                    choices::<Punctuation>()
                        .into_iter()
                        .nth(*i)
                        .and_then(|c| serde_json::from_value::<Punctuation>(c.value).ok())
                };
                let voice = &mut self.draft.voice;
                match (self.context, control.as_str(), value) {
                    // A context's own value.
                    (Some(context), PERSON, Value::Selected(i)) => {
                        voice.contexts.get_mut(context).voice = person(i);
                    }
                    (Some(context), RATE, Value::Number(n)) => {
                        voice.contexts.get_mut(context).rate = Some((*n).clamp(-10, 10));
                    }
                    (Some(context), PUNCTUATION, Value::Selected(i)) => {
                        if let Some(level) = punctuation(i) {
                            voice.contexts.get_mut(context).punctuation = Some(level);
                        }
                    }
                    // All Contexts': every context's, each context's own value given up.
                    (None, PERSON, Value::Selected(i)) => {
                        voice.voice = person(i);
                        VoiceContext::ALL
                            .iter()
                            .for_each(|&c| voice.contexts.get_mut(c).voice = None);
                    }
                    (None, RATE, Value::Number(n)) => {
                        voice.rate = (*n).clamp(-10, 10);
                        VoiceContext::ALL
                            .iter()
                            .for_each(|&c| voice.contexts.get_mut(c).rate = None);
                    }
                    (None, PUNCTUATION, Value::Selected(i)) => {
                        if let Some(level) = punctuation(i) {
                            voice.punctuation = level;
                            VoiceContext::ALL
                                .iter()
                                .for_each(|&c| voice.contexts.get_mut(c).punctuation = None);
                        }
                    }
                    (None, VOLUME, Value::Number(n)) => {
                        voice.volume = u16::try_from((*n).clamp(0, 100)).unwrap_or(100);
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

    fn changed(dialog: &mut VoiceAdjustment, control: &str, value: Value) -> Outcome {
        dialog.handle(&FromUi::DialogChanged {
            id: 5,
            control: control.into(),
            value,
        })
    }

    /// JAWS's Voice Adjustment's "Voice" group: Adjust, Person, Rate, Punctuation, Volume, in
    /// its Tab order [obs, the walk of Voice Adjustment]; Adjust offers All Contexts and the
    /// four contexts, with JAWS's names in Okur's [obs, the walk of its contexts].
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
        assert!(spec.controls.iter().any(|c| c.id == ADJUST
            && c.kind
                == ControlKind::Choice {
                    options: vec![
                        "All Contexts".into(),
                        "PC Cursor Voice".into(),
                        "Okur Cursor Voice".into(),
                        "Keyboard Voice".into(),
                        "Tutor and Message Voice".into()
                    ],
                    selected: 0
                }));
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
            changed(&mut dialog, control, value);
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

    /// A context's rate is its own; Volume is All Contexts' alone; a rate set in All Contexts
    /// again is every context's.
    #[test]
    fn a_context_has_its_own_voice() {
        let mut dialog = VoiceAdjustment::new(5, Settings::default(), vec!["Zira".into()]);
        // The Okur Cursor Voice: third in Adjust.
        let Outcome::Send(shown) = changed(&mut dialog, ADJUST, Value::Selected(2)) else {
            panic_free_fail("no update");
            return;
        };
        assert!(shown.contains(&ToUi::DialogEnable {
            id: 5,
            control: VOLUME.into(),
            enabled: false,
        }));
        changed(&mut dialog, RATE, Value::Number(-3));
        let voice = &dialog.draft.voice;
        assert_eq!(voice.in_context(VoiceContext::OkurCursor).rate, -3);
        assert_eq!(voice.in_context(VoiceContext::PcCursor).rate, 0);
        // Back to All Contexts: its rate shown, and a new one for every context.
        let Outcome::Send(shown) = changed(&mut dialog, ADJUST, Value::Selected(0)) else {
            panic_free_fail("no update");
            return;
        };
        assert!(shown.contains(&ToUi::DialogSet {
            id: 5,
            control: RATE.into(),
            value: Value::Number(0),
        }));
        changed(&mut dialog, RATE, Value::Number(5));
        let voice = &dialog.draft.voice;
        assert!(VoiceContext::ALL
            .iter()
            .all(|&c| voice.in_context(c).rate == 5));
    }

    fn panic_free_fail(message: &str) {
        assert!(message.is_empty(), "{message}");
    }
}
