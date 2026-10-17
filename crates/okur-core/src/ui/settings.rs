//! The settings dialog (Insert+V, Insert+6): every setting Okur has, in one dialog, until
//! there are enough for JAWS's Quick Settings and Settings Center. It keeps a draft of the
//! settings as the user changes them; Apply and OK make the draft Okur's settings, Cancel
//! drops what was not applied.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use crate::settings::{
    JawsKey, KeyboardLayout, KeyboardSettings, LiveRegionAppend, Settings, Synthesizer, TypingEcho,
    ViewerDisplay,
};

/// The dialog's title.
pub const TITLE: &str = "Okur Settings";

const SYNTHESIZERS: [(Synthesizer, &str); 4] = [
    (Synthesizer::Auto, "Automatic"),
    (Synthesizer::OneCore, "OneCore"),
    (Synthesizer::Sapi5, "SAPI 5"),
    (Synthesizer::NoSpeech, "No speech"),
];
const LAYOUTS: [(KeyboardLayout, &str); 2] = [
    (KeyboardLayout::Desktop, "Desktop"),
    (KeyboardLayout::Laptop, "Laptop"),
];
const JAWS_KEYS: [(JawsKey, &str); 3] = [
    (JawsKey::Insert, "Insert"),
    (JawsKey::CapsLock, "Caps Lock"),
    (JawsKey::ScrollLock, "Scroll Lock"),
];
/// JAWS's names for its typing echo choices (Settings Center, User).
const ECHOES: [(TypingEcho, &str); 2] = [
    (TypingEcho::None, "Off"),
    (TypingEcho::Characters, "Characters"),
];
/// The voice list's first entry: the synthesizer's own default voice.
const DEFAULT_VOICE: &str = "Default";

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

fn position<T: PartialEq + Copy, const N: usize>(table: &[(T, &str); N], value: T) -> usize {
    table.iter().position(|(v, _)| *v == value).unwrap_or(0)
}

fn names<T, const N: usize>(table: &[(T, &str); N]) -> Vec<String> {
    table.iter().map(|(_, name)| (*name).to_owned()).collect()
}

fn pick<T: Copy, const N: usize>(table: &[(T, &str); N], index: usize) -> Option<T> {
    table.get(index).map(|(v, _)| *v)
}

fn control(id: &str, label: &str, kind: ControlKind) -> Control {
    Control {
        id: id.to_owned(),
        label: label.to_owned(),
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
        std::iter::once(DEFAULT_VOICE.to_owned())
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
            title: TITLE.to_owned(),
            controls: vec![
                control("speech", "Speech", ControlKind::Group),
                control(
                    "synthesizer",
                    "S&ynthesizer:",
                    choice(
                        names(&SYNTHESIZERS),
                        position(&SYNTHESIZERS, s.voice.synthesizer),
                    ),
                ),
                control("voice", "&Voice:", choice(self.voice_options(), voice)),
                control(
                    "rate",
                    "&Rate:",
                    ControlKind::Slider {
                        min: -10,
                        max: 10,
                        value: s.voice.rate,
                    },
                ),
                control(
                    "volume",
                    "V&olume:",
                    ControlKind::Slider {
                        min: 0,
                        max: 100,
                        value: i32::from(s.voice.volume),
                    },
                ),
                control("keyboard", "Keyboard", ControlKind::Group),
                control(
                    "layout",
                    "Keyboard &Layout:",
                    choice(names(&LAYOUTS), position(&LAYOUTS, s.keyboard.layout)),
                ),
                control(
                    "jaws_key",
                    "&JAWS Key:",
                    choice(names(&JAWS_KEYS), position(&JAWS_KEYS, s.keyboard.jaws_key)),
                ),
                control("reading", "Reading", ControlKind::Group),
                control(
                    "typing_echo",
                    "Typing &Echo:",
                    choice(names(&ECHOES), position(&ECHOES, s.typing_echo)),
                ),
                control(
                    "live_region_whole_text",
                    "Read the &Whole Text of a Growing Live Region",
                    ControlKind::CheckBox {
                        checked: s.live_region_append == LiveRegionAppend::WholeText,
                    },
                ),
                control(
                    "viewer_on_screen",
                    "&Show Virtual Viewer on Screen",
                    ControlKind::CheckBox {
                        checked: s.viewer_display == ViewerDisplay::OnScreen,
                    },
                ),
                control("ok", "OK", ControlKind::Button { default: true }),
                control("cancel", "Cancel", ControlKind::Button { default: false }),
                control("apply", "&Apply", ControlKind::Button { default: false }),
            ],
        }
    }

    /// Handles what the user did to the dialog; events of other dialogs are ignored.
    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                self.change(control, *value)
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
            ("layout", Value::Selected(i)) => {
                if let Some(layout) = pick(&LAYOUTS, i) {
                    if layout != s.keyboard.layout {
                        // A layout comes with its own JAWS key, as in JAWS.
                        s.keyboard = KeyboardSettings::for_layout(layout);
                        return Outcome::Send(vec![ToUi::DialogSet {
                            id: self.id,
                            control: "jaws_key".to_owned(),
                            value: Value::Selected(position(&JAWS_KEYS, s.keyboard.jaws_key)),
                        }]);
                    }
                }
            }
            ("jaws_key", Value::Selected(i)) => {
                if let Some(key) = pick(&JAWS_KEYS, i) {
                    s.keyboard.jaws_key = key;
                }
            }
            ("typing_echo", Value::Selected(i)) => {
                if let Some(echo) = pick(&ECHOES, i) {
                    s.typing_echo = echo;
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
            find("viewer_on_screen"),
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
                control: "jaws_key".into(),
                value: Value::Selected(1),
            }])
        );
        assert_eq!(
            dialog.draft().keyboard,
            KeyboardSettings::for_layout(KeyboardLayout::Laptop)
        );
        // Choosing another key afterwards keeps the layout.
        dialog.handle(&changed("jaws_key", Value::Selected(2)));
        assert_eq!(dialog.draft().keyboard.jaws_key, JawsKey::ScrollLock);
        assert_eq!(dialog.draft().keyboard.layout, KeyboardLayout::Laptop);
    }
}
