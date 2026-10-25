//! Voice Adjustment (the Okur window's Options, Voices, Voice Adjustment...), as JAWS's, for all
//! applications (old-pc's walks, the redesign's §1.1):
//! - Profile Name, the focus on opening: the voice profiles, the synthesizers' own ("SAPI 5 -
//!   (factory)") and those saved by name, in alphabetical order; choosing one shows its voice;
//! - Delete Profile, for a saved profile;
//! - a "Voice" group of Adjust (the voice context), Person, Rate, Punctuation and Volume. Adjust
//!   chooses All Contexts or one of the voice contexts ([`VoiceContext`]): a context shows its own
//!   Person, Rate, Punctuation and Pitch, and no Volume, which is All Contexts' alone; a change in
//!   All Contexts is every context's;
//! - Save As..., which asks for the profile's name ("Save Voice Profile dialog | Voice Profile
//!   Name: edit combo"), then OK and Cancel. OK is there once something changed, and not for a
//!   synthesizer's own profile changed, which is kept only by Save As (JAWS's help).
//!
//! JAWS's Synthesizer Language, Speak Sample, Spell Rate Change and Upper Case Pitch Change have
//! nothing in Okur yet.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use super::voice_profile::{self, Profile};
use crate::messages::{msg, StaticMessage};
use crate::settings::registry::choices;
use crate::settings::{Punctuation, Settings, VoiceContext, VoiceSettings};

const PROFILE: &str = "profile";
const DELETE: &str = "delete_profile";
const SAMPLE: &str = "speak_sample";
const ADJUST: &str = "adjust";
const PERSON: &str = "person";
const RATE: &str = "rate";
const PUNCTUATION: &str = "punctuation";
const PITCH: &str = "pitch";
const VOLUME: &str = "volume";
const SAVE_AS: &str = "save_as";
/// Save Voice Profile's name box.
const PROFILE_NAME: &str = "profile_name";

/// What the engine does after a message from the dialog or its Save Voice Profile dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    Send(Vec<ToUi>),
    /// Speak Sample: JAWS's sentence said in `voice`, the voice the dialog shows, then the voice
    /// in force again [obs, the walk of Voice Adjustment's Speak Sample].
    Sample(Box<VoiceSettings>),
    /// OK: `settings` for all applications, where they were `before`: the voice chosen or
    /// changed, and a saved profile changed kept under its name.
    Apply {
        before: Box<Settings>,
        settings: Box<Settings>,
    },
    /// Save As... pressed: the engine shows [`VoiceAdjustment::save_as`] under a new id.
    AskName,
    /// Save Voice Profile's OK: keep `voice` as profile `name`, then send these to the dialogs.
    Save {
        name: String,
        voice: Box<VoiceSettings>,
        send: Vec<ToUi>,
    },
    /// Delete Profile: the saved profile `name` goes, then send these to the dialog.
    Delete {
        name: String,
        send: Vec<ToUi>,
    },
    /// Cancel, Escape: nothing changes.
    Closed,
}

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
    profiles: Vec<Profile>,
    /// The profile Profile Name shows.
    profile: usize,
    /// Another profile was chosen.
    chosen: bool,
    /// The voice was changed since the profile was chosen or saved.
    modified: bool,
    /// Save Voice Profile, while it is open, and the name typed in it.
    save_as: Option<(u32, String)>,
}

impl VoiceAdjustment {
    pub fn new(id: u32, settings: Settings, voices: Vec<String>) -> Self {
        let profiles = voice_profile::profiles(&settings.voice_profiles);
        let profile = voice_profile::current(&profiles, &settings.voice);
        Self {
            id,
            before: settings.clone(),
            draft: settings,
            voices,
            context: None,
            profiles,
            profile,
            chosen: false,
            modified: false,
            save_as: None,
        }
    }

    /// Whether dialog `id` is this one or its Save Voice Profile.
    pub fn owns(&self, id: u32) -> bool {
        id == self.id || self.save_as.as_ref().is_some_and(|(open, _)| *open == id)
    }

    /// Save Voice Profile, while it is open.
    pub fn save_as_id(&self) -> Option<u32> {
        self.save_as.as_ref().map(|(id, _)| *id)
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

    fn names(&self) -> Vec<String> {
        self.profiles.iter().map(|p| p.name.clone()).collect()
    }

    /// The profile shown is a synthesizer's own.
    fn factory(&self) -> bool {
        self.profiles.get(self.profile).is_none_or(|p| p.factory)
    }

    /// OK is there once another profile was chosen, or the voice changed in a saved one.
    fn ok_enabled(&self) -> bool {
        if self.modified {
            !self.factory()
        } else {
            self.chosen
        }
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
        let button = |id: &str, label: StaticMessage, enabled: bool| Control {
            enabled,
            ..control(id, label, ControlKind::Button { default: false })
        };
        let voice = self.shown();
        DialogSpec {
            id: self.id,
            title: msg::voice_adjustment_title().to_string(),
            controls: vec![
                control(
                    PROFILE,
                    msg::voice_adjustment_profile(),
                    ControlKind::Choice {
                        options: self.names(),
                        selected: self.profile,
                    },
                ),
                button(DELETE, msg::voice_adjustment_delete(), !self.factory()),
                button(SAMPLE, msg::voice_adjustment_sample(), true),
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
                // A context's Pitch, in Volume's place [obs, the walk of Voice Adjustment's
                // contexts].
                Control {
                    enabled: self.context.is_some(),
                    ..control(
                        PITCH,
                        msg::voice_adjustment_pitch(),
                        ControlKind::Slider {
                            min: -10,
                            max: 10,
                            value: voice.pitch,
                        },
                    )
                },
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
                button(SAVE_AS, msg::voice_adjustment_save_as(), true),
                Control {
                    enabled: self.ok_enabled(),
                    ..control(
                        "ok",
                        msg::settings_ok(),
                        ControlKind::Button { default: true },
                    )
                },
                control(
                    "cancel",
                    msg::settings_cancel(),
                    ControlKind::Button { default: false },
                ),
            ],
            keys: Vec::new(),
        }
    }

    /// Save Voice Profile as dialog `id`: "Voice Profile Name:", an editable combo box of the
    /// profiles' names with the one shown typed in, then OK and Cancel [obs, the walk of Voice
    /// Adjustment's profiles].
    pub fn save_as(&mut self, id: u32) -> DialogSpec {
        let name = self
            .profiles
            .get(self.profile)
            .filter(|p| !p.factory)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        self.save_as = Some((id, name.clone()));
        let control = |id: &str, label: StaticMessage, kind: ControlKind| Control {
            id: id.to_owned(),
            label: label.to_string(),
            kind,
            enabled: true,
        };
        DialogSpec {
            id,
            title: msg::voice_adjustment_save_title().to_string(),
            controls: vec![
                control(
                    PROFILE_NAME,
                    msg::voice_adjustment_save_name(),
                    ControlKind::ComboEdit {
                        text: name,
                        history: self
                            .profiles
                            .iter()
                            .filter(|p| !p.factory)
                            .map(|p| p.name.clone())
                            .collect(),
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

    fn adjust_index(&self) -> usize {
        self.context
            .and_then(|c| VoiceContext::ALL.iter().position(|&a| a == c))
            .map_or(0, |i| i + 1)
    }

    /// The controls set to the voice shown: its values, Pitch for a context, Volume for All
    /// Contexts.
    fn show_voice(&self) -> Vec<ToUi> {
        let voice = self.shown();
        let set = |control: &str, value: Value| ToUi::DialogSet {
            id: self.id,
            control: control.to_owned(),
            value,
        };
        let enable = |control: &str, enabled: bool| ToUi::DialogEnable {
            id: self.id,
            control: control.to_owned(),
            enabled,
        };
        vec![
            set(PERSON, Value::Selected(self.person(&voice))),
            set(RATE, Value::Number(voice.rate)),
            set(PUNCTUATION, Value::Selected(Self::punctuation(&voice))),
            set(PITCH, Value::Number(voice.pitch)),
            set(VOLUME, Value::Number(i32::from(voice.volume))),
            enable(PITCH, self.context.is_some()),
            enable(VOLUME, self.context.is_none()),
        ]
    }

    /// Delete Profile and OK as the profile shown and the changes allow.
    fn show_buttons(&self) -> Vec<ToUi> {
        let enable = |control: &str, enabled: bool| ToUi::DialogEnable {
            id: self.id,
            control: control.to_owned(),
            enabled,
        };
        vec![
            enable(DELETE, !self.factory()),
            enable("ok", self.ok_enabled()),
        ]
    }

    /// Profile `name` shown, its voice the draft's.
    fn choose(&mut self, index: usize) -> Vec<ToUi> {
        let Some(profile) = self.profiles.get(index).cloned() else {
            return Vec::new();
        };
        self.profile = index;
        self.draft.voice = self.draft.voice.with_profile(&profile.name, &profile.voice);
        self.chosen = true;
        self.modified = false;
        let mut sent = self.show_voice();
        sent.extend(self.show_buttons());
        sent
    }

    /// A value changed in the voice shown.
    fn change(&mut self, control: &str, value: &Value) -> Outcome {
        let person = |i: &usize| i.checked_sub(1).and_then(|i| self.voices.get(i)).cloned();
        let punctuation = |i: &usize| {
            choices::<Punctuation>()
                .into_iter()
                .nth(*i)
                .and_then(|c| serde_json::from_value::<Punctuation>(c.value).ok())
        };
        let voice = &mut self.draft.voice;
        match (self.context, control, value) {
            // A context's own value.
            (Some(context), PERSON, Value::Selected(i)) => {
                voice.contexts.get_mut(context).voice = person(i);
            }
            (Some(context), RATE, Value::Number(n)) => {
                voice.contexts.get_mut(context).rate = Some((*n).clamp(-10, 10));
            }
            (Some(context), PITCH, Value::Number(n)) => {
                voice.contexts.get_mut(context).pitch = Some((*n).clamp(-10, 10));
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
            _ => return Outcome::Nothing,
        }
        self.modified = true;
        Outcome::Send(self.show_buttons())
    }

    /// Save Voice Profile's OK: the voice kept under the name typed, the profile shown.
    fn save(&mut self) -> Outcome {
        let Some((id, name)) = self.save_as.take() else {
            return Outcome::Nothing;
        };
        let name = name.trim().to_owned();
        // A synthesizer's own name is not a saved profile's.
        if name.is_empty() || self.profiles.iter().any(|p| p.factory && p.name == name) {
            self.save_as = Some((id, name));
            return Outcome::Nothing;
        }
        let voice = VoiceSettings {
            profile: None,
            ..self.draft.voice.clone()
        };
        self.profiles.retain(|p| p.name != name);
        self.profiles.push(Profile {
            name: name.clone(),
            voice: voice.clone(),
            factory: false,
        });
        self.profiles.sort_by_key(|p| p.name.to_lowercase());
        self.profile = self
            .profiles
            .iter()
            .position(|p| p.name == name)
            .unwrap_or(0);
        self.draft.voice.profile = Some(name.clone());
        self.draft
            .voice_profiles
            .insert(name.clone(), voice.clone());
        self.before
            .voice_profiles
            .insert(name.clone(), voice.clone());
        self.chosen = true;
        self.modified = false;
        let mut send = vec![
            ToUi::DialogClose { id },
            ToUi::DialogItems {
                id: self.id,
                control: PROFILE.to_owned(),
                items: self.names(),
                selected: self.profile,
            },
        ];
        send.extend(self.show_buttons());
        Outcome::Save {
            name,
            voice: Box::new(voice),
            send,
        }
    }

    /// Delete Profile: the saved profile shown goes; its synthesizer's own is shown.
    fn delete(&mut self) -> Outcome {
        let Some(gone) = self
            .profiles
            .get(self.profile)
            .filter(|p| !p.factory)
            .cloned()
        else {
            return Outcome::Nothing;
        };
        self.profiles.retain(|p| p.name != gone.name);
        self.draft.voice_profiles.remove(&gone.name);
        self.before.voice_profiles.remove(&gone.name);
        let own = self
            .profiles
            .iter()
            .position(|p| p.factory && p.voice.synthesizer == gone.voice.synthesizer)
            .unwrap_or(0);
        let mut send = vec![ToUi::DialogItems {
            id: self.id,
            control: PROFILE.to_owned(),
            items: self.names(),
            selected: own,
        }];
        send.extend(self.choose(own));
        Outcome::Delete {
            name: gone.name,
            send,
        }
    }

    /// What the dialog's window, or its Save Voice Profile's, reported.
    pub fn handle(&mut self, message: &FromUi) -> Outcome {
        let save_as = self.save_as_id();
        match message {
            // Save Voice Profile: the name typed, OK, Cancel.
            FromUi::DialogChanged {
                id,
                value: Value::Text(text),
                ..
            } if Some(*id) == save_as => {
                if let Some((_, name)) = self.save_as.as_mut() {
                    *name = text.clone();
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if Some(*id) == save_as && control == "ok" => {
                self.save()
            }
            FromUi::DialogClosed { id } if Some(*id) == save_as => {
                self.save_as = None;
                Outcome::Nothing
            }
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    (PROFILE, Value::Selected(i)) => Outcome::Send(self.choose(*i)),
                    (ADJUST, Value::Selected(i)) => {
                        self.context = i
                            .checked_sub(1)
                            .and_then(|i| VoiceContext::ALL.get(i))
                            .copied();
                        Outcome::Send(self.show_voice())
                    }
                    (control, value) => self.change(control, value),
                }
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                SAVE_AS => Outcome::AskName,
                DELETE => self.delete(),
                SAMPLE => Outcome::Sample(Box::new(VoiceSettings {
                    contexts: Default::default(),
                    ..self.shown()
                })),
                "ok" if self.ok_enabled() => {
                    let mut settings = self.draft.clone();
                    if let Some(profile) = self.profiles.get(self.profile) {
                        settings.voice.profile = Some(profile.name.clone());
                        // A saved profile changed keeps its change.
                        if !profile.factory {
                            settings.voice_profiles.insert(
                                profile.name.clone(),
                                VoiceSettings {
                                    profile: None,
                                    ..settings.voice.clone()
                                },
                            );
                        }
                    }
                    Outcome::Apply {
                        before: Box::new(self.before.clone()),
                        settings: Box::new(settings),
                    }
                }
                _ => Outcome::Nothing,
            },
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

    fn pressed(dialog: &mut VoiceAdjustment, id: u32, control: &str) -> Outcome {
        dialog.handle(&FromUi::DialogPressed {
            id,
            control: control.into(),
        })
    }

    fn panic_free_fail(message: &str) {
        assert!(message.is_empty(), "{message}");
    }

    /// Speak Sample says JAWS's sentence in the voice the dialog shows: a context's, once
    /// Adjust shows it [obs, the walk of Voice Adjustment's Speak Sample].
    #[test]
    fn speak_sample_uses_the_voice_shown() {
        let mut settings = Settings::default();
        settings.voice.rate = 3;
        settings.voice.contexts.keyboard.rate = Some(-6);
        let mut dialog = VoiceAdjustment::new(5, settings, vec!["Zira".into()]);
        let Outcome::Sample(voice) = pressed(&mut dialog, 5, SAMPLE) else {
            panic_free_fail("no sample");
            return;
        };
        assert_eq!(voice.rate, 3);
        dialog.handle(&FromUi::DialogChanged {
            id: 5,
            control: ADJUST.into(),
            value: Value::Selected(3),
        });
        let Outcome::Sample(voice) = pressed(&mut dialog, 5, SAMPLE) else {
            panic_free_fail("no sample");
            return;
        };
        assert_eq!(voice.rate, -6);
    }

    /// JAWS's Voice Adjustment: Profile Name, Delete Profile, Speak Sample, the "Voice" group
    /// of Adjust, Person, Rate, Punctuation, Pitch or Volume, Save As..., OK, Cancel, in its Tab
    /// order [obs, the walks of Voice Adjustment]; Adjust offers All Contexts and the four
    /// contexts, with JAWS's names in Okur's [obs, the walk of its contexts].
    #[test]
    fn voice_adjustment_is_jaws_dialog() {
        let dialog = VoiceAdjustment::new(5, Settings::default(), vec!["Zira".into()]);
        let spec = dialog.spec();
        assert_eq!(spec.title, "Voice Adjustment");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Profile Name:",
                "Delete Profile",
                "Speak Sample",
                "Voice",
                "Adjust:",
                "Person:",
                "Rate:",
                "Punctuation:",
                "Pitch:",
                "Volume:",
                "Save As...",
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
        // The synthesizer's own profile shown: Delete Profile and OK unavailable.
        let enabled = |id: &str| spec.controls.iter().find(|c| c.id == id).map(|c| c.enabled);
        assert_eq!(enabled(DELETE), Some(false));
        assert_eq!(enabled("ok"), Some(false));
        assert!(spec.controls.iter().any(|c| c.id == PROFILE
            && matches!(&c.kind, ControlKind::Choice { options, selected }
                if options.get(*selected).map(String::as_str) == Some("Automatic - (factory)"))));
    }

    /// A synthesizer's own profile changed is kept only by Save As: OK stays unavailable. Saved
    /// as "Fast", the profile is shown, OK applies it for all applications, with its voice kept
    /// under its name.
    #[test]
    fn a_profile_is_saved_as_a_name() {
        let mut dialog = VoiceAdjustment::new(5, Settings::default(), vec!["Zira".into()]);
        let Outcome::Send(sent) = changed(&mut dialog, RATE, Value::Number(4)) else {
            panic_free_fail("no update");
            return;
        };
        assert!(sent.contains(&ToUi::DialogEnable {
            id: 5,
            control: "ok".into(),
            enabled: false,
        }));
        assert_eq!(pressed(&mut dialog, 5, SAVE_AS), Outcome::AskName);
        let asked = dialog.save_as(9);
        assert_eq!(asked.title, "Save Voice Profile");
        assert!(dialog.owns(9));
        dialog.handle(&FromUi::DialogChanged {
            id: 9,
            control: PROFILE_NAME.into(),
            value: Value::Text("Fast".into()),
        });
        let Outcome::Save { name, voice, send } = pressed(&mut dialog, 9, "ok") else {
            panic_free_fail("not saved");
            return;
        };
        assert_eq!((name.as_str(), voice.rate), ("Fast", 4));
        assert!(send.contains(&ToUi::DialogClose { id: 9 }));
        assert!(send
            .iter()
            .any(|m| matches!(m, ToUi::DialogItems { items, selected, .. }
            if items.get(*selected).map(String::as_str) == Some("Fast"))));
        assert!(!dialog.owns(9));
        let Outcome::Apply { settings, .. } = pressed(&mut dialog, 5, "ok") else {
            panic_free_fail("not applied");
            return;
        };
        assert_eq!(settings.voice.rate, 4);
        assert_eq!(settings.voice.profile.as_deref(), Some("Fast"));
        assert_eq!(settings.voice_profiles.get("Fast").map(|v| v.rate), Some(4));
    }

    /// A saved profile changed keeps its change with OK; Delete Profile removes it, and its
    /// synthesizer's own is shown.
    #[test]
    fn a_saved_profile_is_changed_and_deleted() {
        let mut settings = Settings::default();
        settings.voice_profiles.insert(
            "Fast".into(),
            VoiceSettings {
                rate: 6,
                ..VoiceSettings::default()
            },
        );
        settings.voice.profile = Some("Fast".into());
        settings.voice.rate = 6;
        let mut dialog = VoiceAdjustment::new(5, settings, Vec::new());
        changed(&mut dialog, RATE, Value::Number(8));
        let Outcome::Apply { settings, .. } = pressed(&mut dialog.clone(), 5, "ok") else {
            panic_free_fail("not applied");
            return;
        };
        assert_eq!(settings.voice_profiles.get("Fast").map(|v| v.rate), Some(8));
        let Outcome::Delete { name, send } = pressed(&mut dialog, 5, DELETE) else {
            panic_free_fail("not deleted");
            return;
        };
        assert_eq!(name, "Fast");
        assert!(send
            .iter()
            .any(|m| matches!(m, ToUi::DialogItems { items, selected, .. }
            if items.get(*selected).map(String::as_str) == Some("Automatic - (factory)")
                && !items.iter().any(|i| i == "Fast"))));
    }

    /// A context's rate and pitch are its own; Volume is All Contexts' alone; a rate set in All
    /// Contexts again is every context's.
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
        assert!(shown.contains(&ToUi::DialogEnable {
            id: 5,
            control: PITCH.into(),
            enabled: true,
        }));
        changed(&mut dialog, RATE, Value::Number(-3));
        changed(&mut dialog, PITCH, Value::Number(4));
        let voice = &dialog.draft.voice;
        assert_eq!(voice.in_context(VoiceContext::OkurCursor).rate, -3);
        assert_eq!(voice.in_context(VoiceContext::OkurCursor).pitch, 4);
        assert_eq!(voice.in_context(VoiceContext::PcCursor).rate, 0);
        assert_eq!(voice.in_context(VoiceContext::PcCursor).pitch, 0);
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
}
