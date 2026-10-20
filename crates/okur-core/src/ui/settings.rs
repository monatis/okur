//! The settings dialog (Insert+V, Insert+6): every setting Okur has, in one dialog, until
//! there are enough for JAWS's Quick Settings and Settings Center. It keeps a draft of the
//! settings as the user changes them; Apply and OK make the draft Okur's settings, Cancel
//! drops what was not applied.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use crate::messages::{msg, StaticMessage};
use crate::settings::{
    KeyboardLayout, KeyboardSettings, LiveRegionAppend, OkurKey, Punctuation, Settings,
    Synthesizer, TypingEcho, VerbosityLevel, ViewerDisplay,
};

/// A choice's label in the catalog.
type Label = fn() -> StaticMessage;

const SYNTHESIZERS: [(Synthesizer, Label); 4] = [
    (Synthesizer::Auto, msg::settings_synthesizer_automatic),
    (Synthesizer::OneCore, msg::settings_synthesizer_onecore),
    (Synthesizer::Sapi5, msg::settings_synthesizer_sapi5),
    (Synthesizer::NoSpeech, msg::settings_synthesizer_none),
];
const LAYOUTS: [(KeyboardLayout, Label); 2] = [
    (KeyboardLayout::Desktop, msg::settings_layout_desktop),
    (KeyboardLayout::Laptop, msg::settings_layout_laptop),
];
const OKUR_KEYS: [(OkurKey, Label); 3] = [
    (OkurKey::Insert, msg::settings_okur_key_insert),
    (OkurKey::CapsLock, msg::settings_okur_key_caps_lock),
    (OkurKey::ScrollLock, msg::settings_okur_key_scroll_lock),
];
/// JAWS's punctuation levels, as Voice Adjustment lists them.
const PUNCTUATIONS: [(Punctuation, Label); 4] = [
    (Punctuation::None, msg::punctuation_none),
    (Punctuation::Some, msg::punctuation_some),
    (Punctuation::Most, msg::punctuation_most),
    (Punctuation::All, msg::punctuation_all),
];
/// JAWS's names for its typing echo choices (Settings Center, User).
const ECHOES: [(TypingEcho, Label); 2] = [
    (TypingEcho::None, msg::settings_typing_echo_off),
    (TypingEcho::Characters, msg::settings_typing_echo_characters),
];
/// JAWS's user verbosity levels.
const LEVELS: [(VerbosityLevel, Label); 3] = [
    (VerbosityLevel::Beginner, msg::settings_verbosity_beginner),
    (
        VerbosityLevel::Intermediate,
        msg::settings_verbosity_intermediate,
    ),
    (VerbosityLevel::Advanced, msg::settings_verbosity_advanced),
];

/// What the engine does after the dialog handled an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Send these to the dialog.
    Send(Vec<ToUi>),
    /// Make these Okur's settings; with `close`, close the dialog too.
    Apply {
        settings: Settings,
        close: bool,
    },
    /// The dialog is gone; nothing more is applied.
    Closed,
}

/// The settings dialog's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDialog {
    pub id: u32,
    draft: Settings,
    voices: Vec<String>,
}

fn position<T: PartialEq + Copy, const N: usize>(table: &[(T, Label); N], value: T) -> usize {
    table.iter().position(|(v, _)| *v == value).unwrap_or(0)
}

fn names<T, const N: usize>(table: &[(T, Label); N]) -> Vec<String> {
    table.iter().map(|(_, name)| name().to_string()).collect()
}

fn pick<T: Copy, const N: usize>(table: &[(T, Label); N], index: usize) -> Option<T> {
    table.get(index).map(|(v, _)| *v)
}

fn control(id: &str, label: StaticMessage, kind: ControlKind) -> Control {
    Control {
        id: id.to_owned(),
        label: label.to_string(),
        kind,
        enabled: true,
    }
}

impl SettingsDialog {
    /// The dialog over `settings`, offering `voices` (the synthesizer's, by name).
    pub fn new(id: u32, settings: &Settings, voices: Vec<String>) -> Self {
        Self {
            id,
            draft: settings.clone(),
            voices,
        }
    }

    /// The settings as the dialog shows them now.
    pub fn draft(&self) -> &Settings {
        &self.draft
    }

    fn voice_options(&self) -> Vec<String> {
        std::iter::once(msg::settings_voice_default().to_string())
            .chain(self.voices.iter().cloned())
            .collect()
    }

    /// The dialog's description, with the draft's values.
    pub fn spec(&self) -> DialogSpec {
        let s = &self.draft;
        let voice = s
            .voice
            .voice
            .as_ref()
            .and_then(|name| self.voices.iter().position(|v| v == name))
            .map_or(0, |i| i + 1);
        let choice =
            |options: Vec<String>, selected: usize| ControlKind::Choice { options, selected };
        DialogSpec {
            id: self.id,
            title: msg::settings_dialog_title().to_string(),
            controls: vec![
                control("speech", msg::settings_group_speech(), ControlKind::Group),
                control(
                    "synthesizer",
                    msg::settings_synthesizer(),
                    choice(
                        names(&SYNTHESIZERS),
                        position(&SYNTHESIZERS, s.voice.synthesizer),
                    ),
                ),
                control(
                    "voice",
                    msg::settings_voice(),
                    choice(self.voice_options(), voice),
                ),
                control(
                    "rate",
                    msg::settings_rate(),
                    ControlKind::Slider {
                        min: -10,
                        max: 10,
                        value: s.voice.rate,
                    },
                ),
                control(
                    "volume",
                    msg::settings_volume(),
                    ControlKind::Slider {
                        min: 0,
                        max: 100,
                        value: i32::from(s.voice.volume),
                    },
                ),
                control(
                    "punctuation",
                    msg::settings_punctuation(),
                    choice(
                        names(&PUNCTUATIONS),
                        position(&PUNCTUATIONS, s.voice.punctuation),
                    ),
                ),
                control(
                    "keyboard",
                    msg::settings_group_keyboard(),
                    ControlKind::Group,
                ),
                control(
                    "layout",
                    msg::settings_layout(),
                    choice(names(&LAYOUTS), position(&LAYOUTS, s.keyboard.layout)),
                ),
                control(
                    "okur_key",
                    msg::settings_okur_key(),
                    choice(names(&OKUR_KEYS), position(&OKUR_KEYS, s.keyboard.okur_key)),
                ),
                control("reading", msg::settings_group_reading(), ControlKind::Group),
                control(
                    "typing_echo",
                    msg::settings_typing_echo(),
                    choice(names(&ECHOES), position(&ECHOES, s.typing_echo)),
                ),
                control(
                    "verbosity",
                    msg::settings_verbosity(),
                    choice(names(&LEVELS), position(&LEVELS, s.verbosity.level)),
                ),
                control(
                    "live_region_whole_text",
                    msg::settings_live_region_whole_text(),
                    ControlKind::CheckBox {
                        checked: s.live_region_append == LiveRegionAppend::WholeText,
                    },
                ),
                control(
                    "viewer_on_screen",
                    msg::settings_viewer_on_screen(),
                    ControlKind::CheckBox {
                        checked: s.viewer_display == ViewerDisplay::OnScreen,
                    },
                ),
                control(
                    "confirm_exit",
                    msg::settings_confirm_exit(),
                    ControlKind::CheckBox {
                        checked: s.confirm_exit,
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
                control(
                    "apply",
                    msg::settings_apply(),
                    ControlKind::Button { default: false },
                ),
            ],
        }
    }

    /// Handles what the user did to the dialog; events of other dialogs are ignored.
    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                self.change(control, value.clone())
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" => Outcome::Apply {
                    settings: self.draft.clone(),
                    close: true,
                },
                "apply" => Outcome::Apply {
                    settings: self.draft.clone(),
                    close: false,
                },
                _ => Outcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }

    fn change(&mut self, control: &str, value: Value) -> Outcome {
        let s = &mut self.draft;
        match (control, value) {
            ("synthesizer", Value::Selected(i)) => {
                if let Some(synthesizer) = pick(&SYNTHESIZERS, i) {
                    // Another synthesizer has other voices: its default until it is applied.
                    if synthesizer != s.voice.synthesizer {
                        s.voice.voice = None;
                    }
                    s.voice.synthesizer = synthesizer;
                }
            }
            ("voice", Value::Selected(i)) => {
                s.voice.voice = i.checked_sub(1).and_then(|v| self.voices.get(v)).cloned();
            }
            ("rate", Value::Number(n)) => s.voice.rate = n.clamp(-10, 10),
            ("volume", Value::Number(n)) => {
                s.voice.volume = u16::try_from(n.clamp(0, 100)).unwrap_or(100);
            }
            ("punctuation", Value::Selected(i)) => {
                if let Some(level) = pick(&PUNCTUATIONS, i) {
                    s.voice.punctuation = level;
                }
            }
            ("layout", Value::Selected(i)) => {
                if let Some(layout) = pick(&LAYOUTS, i) {
                    if layout != s.keyboard.layout {
                        // A layout comes with its own JAWS key, as in JAWS.
                        s.keyboard = KeyboardSettings::for_layout(layout);
                        return Outcome::Send(vec![ToUi::DialogSet {
                            id: self.id,
                            control: "okur_key".to_owned(),
                            value: Value::Selected(position(&OKUR_KEYS, s.keyboard.okur_key)),
                        }]);
                    }
                }
            }
            ("okur_key", Value::Selected(i)) => {
                if let Some(key) = pick(&OKUR_KEYS, i) {
                    s.keyboard.okur_key = key;
                }
            }
            ("typing_echo", Value::Selected(i)) => {
                if let Some(echo) = pick(&ECHOES, i) {
                    s.typing_echo = echo;
                }
            }
            ("verbosity", Value::Selected(i)) => {
                if let Some(level) = pick(&LEVELS, i) {
                    s.verbosity.level = level;
                }
            }
            ("live_region_whole_text", Value::Checked(checked)) => {
                s.live_region_append = if checked {
                    LiveRegionAppend::WholeText
                } else {
                    LiveRegionAppend::Appended
                };
            }
            ("viewer_on_screen", Value::Checked(checked)) => {
                s.viewer_display = if checked {
                    ViewerDisplay::OnScreen
                } else {
                    ViewerDisplay::SpeechOnly
                };
            }
            ("confirm_exit", Value::Checked(checked)) => s.confirm_exit = checked,
            _ => {}
        }
        Outcome::Nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changed(control: &str, value: Value) -> FromUi {
        FromUi::DialogChanged {
            id: 4,
            control: control.to_owned(),
            value,
        }
    }

    #[test]
    fn the_dialog_shows_the_settings_with_jaws_names() {
        let mut settings = Settings {
            typing_echo: TypingEcho::None,
            ..Default::default()
        };
        settings.voice.voice = Some("Microsoft Zira".to_owned());
        let dialog = SettingsDialog::new(
            4,
            &settings,
            vec!["Microsoft David".into(), "Microsoft Zira".into()],
        );
        let spec = dialog.spec();
        assert_eq!(spec.title, "Okur Settings");
        let find = |id: &str| {
            spec.controls
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.kind.clone())
        };
        assert_eq!(
            find("voice"),
            Some(ControlKind::Choice {
                options: vec![
                    "Default".into(),
                    "Microsoft David".into(),
                    "Microsoft Zira".into()
                ],
                selected: 2
            })
        );
        assert_eq!(
            find("typing_echo"),
            Some(ControlKind::Choice {
                options: vec!["Off".into(), "Characters".into()],
                selected: 0
            })
        );
        assert_eq!(
            find("verbosity"),
            Some(ControlKind::Choice {
                options: vec!["Beginner".into(), "Intermediate".into(), "Advanced".into()],
                selected: 0
            })
        );
        assert_eq!(
            find("viewer_on_screen"),
            Some(ControlKind::CheckBox { checked: true })
        );
        // JAWS's default: Insert+F4 asks first.
        assert_eq!(
            find("confirm_exit"),
            Some(ControlKind::CheckBox { checked: true })
        );
        // Every label's access key is its own.
        let keys: Vec<char> = spec
            .controls
            .iter()
            .filter_map(|c| c.label.split_once('&'))
            .filter_map(|(_, after)| after.chars().next())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        let mut unique = keys.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), keys.len(), "{keys:?}");
    }

    #[test]
    fn changes_are_drafted_until_applied_and_cancel_drops_them() {
        let mut dialog = SettingsDialog::new(4, &Settings::default(), Vec::new());
        assert_eq!(
            dialog.handle(&changed("rate", Value::Number(3))),
            Outcome::Nothing
        );
        assert_eq!(
            dialog.handle(&changed("typing_echo", Value::Selected(0))),
            Outcome::Nothing
        );
        assert_eq!(
            dialog.handle(&changed("viewer_on_screen", Value::Checked(false))),
            Outcome::Nothing
        );
        assert_eq!(
            dialog.handle(&changed("punctuation", Value::Selected(3))),
            Outcome::Nothing
        );
        // Another dialog's events change nothing.
        let other = FromUi::DialogChanged {
            id: 9,
            control: "rate".into(),
            value: Value::Number(-5),
        };
        assert_eq!(dialog.handle(&other), Outcome::Nothing);
        let applied = dialog.handle(&FromUi::DialogPressed {
            id: 4,
            control: "apply".into(),
        });
        let mut expected = Settings {
            typing_echo: TypingEcho::None,
            viewer_display: ViewerDisplay::SpeechOnly,
            ..Default::default()
        };
        expected.voice.rate = 3;
        expected.voice.punctuation = crate::settings::Punctuation::All;
        assert_eq!(
            applied,
            Outcome::Apply {
                settings: expected.clone(),
                close: false
            }
        );
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 4,
                control: "ok".into()
            }),
            Outcome::Apply {
                settings: expected,
                close: true
            }
        );
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 4 }),
            Outcome::Closed
        );
    }

    #[test]
    fn a_layout_brings_its_own_jaws_key() {
        let mut dialog = SettingsDialog::new(4, &Settings::default(), Vec::new());
        assert_eq!(
            dialog.handle(&changed("layout", Value::Selected(1))),
            Outcome::Send(vec![ToUi::DialogSet {
                id: 4,
                control: "okur_key".into(),
                value: Value::Selected(1),
            }])
        );
        assert_eq!(
            dialog.draft().keyboard,
            KeyboardSettings::for_layout(KeyboardLayout::Laptop)
        );
        // Choosing another key afterwards keeps the layout.
        dialog.handle(&changed("okur_key", Value::Selected(2)));
        assert_eq!(dialog.draft().keyboard.okur_key, OkurKey::ScrollLock);
        assert_eq!(dialog.draft().keyboard.layout, KeyboardLayout::Laptop);
    }
}
