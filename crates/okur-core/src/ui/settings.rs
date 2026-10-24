//! The settings dialog (Insert+V, Insert+6): every setting Okur has, in one dialog, until
//! there are enough for JAWS's Quick Settings and Settings Center. Its controls are the
//! settings table's ([`crate::settings::registry`]), group by group. It keeps a draft of the
//! settings as the user changes them; Apply and OK make the draft Okur's settings, Cancel
//! drops what was not applied.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use crate::messages::{msg, StaticMessage};
use crate::settings::registry::{self, Group, Kind, SettingSpec, SETTINGS};
use crate::settings::{MessageLength, OutputMode, OutputType, Settings};
use crate::settings_layers::Scope;

/// The verbosity level whose items the dialog customizes, its spoken items and its message
/// length, as JAWS's Settings Center customizes a level.
const CUSTOMIZE: &str = "verbosity.customize";
const LENGTH: &str = "verbosity.length";
/// A spoken item's control: this, then the output type's id ("verbosity.output.hint").
const OUTPUT: &str = "verbosity.output.";

/// An output type's id in its control's.
fn output_id(output: OutputType) -> String {
    serde_json::to_value(output)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// A setting's value, as the settings file writes it.
type Json = serde_json::Value;

/// What the engine does after the dialog handled an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Send these to the dialog.
    Send(Vec<ToUi>),
    /// Make these Okur's settings for `scope`, where they were `before`; with `close`, close
    /// the dialog too.
    Apply {
        scope: Scope,
        before: Box<Settings>,
        settings: Box<Settings>,
        close: bool,
    },
    /// The dialog is gone; nothing more is applied.
    Closed,
    /// Open the dialog that customizes the verbosity levels, from Settings Center's
    /// "Configure Verbosity Levels" (JAWS's opens its own dialog over Settings Center).
    OpenVerbosityLevels,
}

/// Where the dialog's settings are kept, as its "Settings for" box offers it: all
/// applications, the application in front, the site of its page (JAWS's Settings Center opens
/// on the application in front, and its Application box chooses another or "Default (All
/// Applications)"; Quick Settings changes the application in front, or the site in
/// Personalize Web Settings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub scope: Scope,
    /// As the box shows it.
    pub name: String,
    /// The settings in force there.
    pub settings: Settings,
}

/// The settings dialog's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDialog {
    pub id: u32,
    places: Vec<Place>,
    place: usize,
    draft: Settings,
    voices: Vec<String>,
    /// The verbosity level customized, Beginner's 0.
    customized: usize,
}

fn control(id: &str, label: StaticMessage, kind: ControlKind) -> Control {
    Control {
        id: id.to_owned(),
        label: label.to_string(),
        kind,
        enabled: true,
    }
}

/// The settings the dialog shows: those with a label.
fn shown_settings() -> impl Iterator<Item = (&'static SettingSpec, StaticMessage)> {
    SETTINGS
        .iter()
        .filter_map(|spec| spec.label.map(|label| (spec, label())))
}

impl SettingsDialog {
    /// The dialog over `places`, showing the settings of `place` among them, offering `voices`
    /// (the synthesizer's, by name).
    pub fn new(id: u32, places: Vec<Place>, place: usize, voices: Vec<String>) -> Self {
        let place = place.min(places.len().saturating_sub(1));
        let draft = places
            .get(place)
            .map(|p| p.settings.clone())
            .unwrap_or_default();
        let customized = draft.verbosity.level.index();
        Self {
            id,
            places,
            place,
            draft,
            voices,
            customized,
        }
    }

    /// The settings shown when the dialog's place was chosen.
    fn shown(&self) -> Settings {
        self.places
            .get(self.place)
            .map(|p| p.settings.clone())
            .unwrap_or_default()
    }

    /// Where the place shown keeps its settings.
    fn scope(&self) -> Scope {
        self.places
            .get(self.place)
            .map_or(Scope::Global, |p| p.scope.clone())
    }

    /// Every control's value, as the dialog is to show it.
    fn values(&self) -> Vec<ToUi> {
        self.spec()
            .controls
            .into_iter()
            .filter(|c| c.id != "scope")
            .filter_map(|c| {
                let value = match c.kind {
                    ControlKind::Choice { selected, .. } => Value::Selected(selected),
                    ControlKind::Slider { value, .. } => Value::Number(value),
                    ControlKind::CheckBox { checked } => Value::Checked(checked),
                    _ => return None,
                };
                Some(ToUi::DialogSet {
                    id: self.id,
                    control: c.id,
                    value,
                })
            })
            .collect()
    }

    /// Which settings the place shown can keep: the others' controls are dimmed.
    fn enabling(&self) -> Vec<ToUi> {
        let scope = self.scope();
        shown_settings()
            .map(|(spec, _)| ToUi::DialogEnable {
                id: self.id,
                control: spec.id.to_owned(),
                enabled: spec.scopes.allows(&scope),
            })
            .collect()
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

    /// A setting's control, with the draft's value; none for a setting the dialog cannot show.
    fn setting_control(
        &self,
        spec: &SettingSpec,
        label: StaticMessage,
        scope: &Scope,
    ) -> Option<Control> {
        let value = registry::get(&self.draft, spec.id).unwrap_or(Json::Null);
        let kind = match spec.kind {
            Kind::Choice(choices) => {
                let choices = choices();
                ControlKind::Choice {
                    selected: choices.iter().position(|c| c.value == value).unwrap_or(0),
                    options: choices.iter().map(|c| c.label.to_string()).collect(),
                }
            }
            Kind::Check { on, .. } => ControlKind::CheckBox {
                checked: value == on.value(),
            },
            Kind::Range { min, max } => ControlKind::Slider {
                min,
                max,
                value: value
                    .as_i64()
                    .and_then(|n| i32::try_from(n).ok())
                    .unwrap_or(min),
            },
            Kind::Voice => ControlKind::Choice {
                options: self.voice_options(),
                selected: value
                    .as_str()
                    .and_then(|name| self.voices.iter().position(|v| v == name))
                    .map_or(0, |i| i + 1),
            },
            Kind::Values(_) | Kind::Structured => return None,
        };
        Some(Control {
            id: spec.id.to_owned(),
            label: label.to_string(),
            kind,
            enabled: spec.scopes.allows(scope),
        })
    }

    /// The dialog's description, with the draft's values: the place, then each group's
    /// settings, then the buttons.
    pub fn spec(&self) -> DialogSpec {
        let scope = self.scope();
        let mut controls = vec![control(
            "scope",
            msg::settings_scope(),
            ControlKind::Choice {
                options: self.places.iter().map(|p| p.name.clone()).collect(),
                selected: self.place,
            },
        )];
        for group in Group::ALL {
            controls.push(control(group.id(), group.label(), ControlKind::Group));
            controls.extend(
                shown_settings()
                    .filter(|(spec, _)| spec.group == group)
                    .filter_map(|(spec, label)| self.setting_control(spec, label, &scope)),
            );
        }
        controls.extend(self.verbosity_controls());
        controls.extend([
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
        ]);
        DialogSpec {
            id: self.id,
            title: msg::settings_dialog_title().to_string(),
            controls,
        }
    }

    /// The verbosity levels' group: the level customized, its spoken items, its message
    /// length.
    fn verbosity_controls(&self) -> Vec<Control> {
        let verbosity = &self.draft.verbosity;
        let level = self.customized;
        let mut controls = vec![
            control(
                "verbosity_levels",
                msg::settings_group_verbosity_levels(),
                ControlKind::Group,
            ),
            control(
                CUSTOMIZE,
                msg::settings_verbosity_customize(),
                ControlKind::Choice {
                    options: [
                        msg::settings_verbosity_beginner(),
                        msg::settings_verbosity_intermediate(),
                        msg::settings_verbosity_advanced(),
                    ]
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                    selected: level,
                },
            ),
        ];
        controls.extend(OutputType::ALL.iter().map(|&output| Control {
            id: format!("{OUTPUT}{}", output_id(output)),
            label: output.label().to_string(),
            kind: ControlKind::CheckBox {
                checked: verbosity.mode_at(output, level) == OutputMode::On,
            },
            enabled: true,
        }));
        let short = verbosity.message_length.get(level) == Some(&MessageLength::Short);
        controls.push(control(
            LENGTH,
            msg::settings_verbosity_length(),
            ControlKind::Choice {
                options: vec![
                    msg::settings_verbosity_length_long().to_string(),
                    msg::settings_verbosity_length_short().to_string(),
                ],
                selected: usize::from(short),
            },
        ));
        controls
    }

    /// A change to the verbosity levels' group; `None` for any other control.
    fn change_verbosity(&mut self, control: &str, value: &Value) -> Option<Outcome> {
        match (control, value) {
            (CUSTOMIZE, Value::Selected(level)) => {
                // Another level: its items and length are shown.
                self.customized = (*level).min(2);
                let shown: Vec<String> = self
                    .verbosity_controls()
                    .into_iter()
                    .map(|c| c.id)
                    .filter(|id| id != CUSTOMIZE)
                    .collect();
                let sent = self
                    .values()
                    .into_iter()
                    .filter(
                        |v| matches!(v, ToUi::DialogSet { control, .. } if shown.contains(control)),
                    )
                    .collect();
                Some(Outcome::Send(sent))
            }
            (LENGTH, Value::Selected(i)) => {
                let length = if *i == 1 {
                    MessageLength::Short
                } else {
                    MessageLength::Long
                };
                if let Some(l) = self.draft.verbosity.message_length.get_mut(self.customized) {
                    *l = length;
                }
                Some(Outcome::Nothing)
            }
            (_, Value::Checked(checked)) => {
                let id = control.strip_prefix(OUTPUT)?;
                let output = OutputType::ALL
                    .iter()
                    .copied()
                    .find(|&o| output_id(o) == id)?;
                let mode = if *checked {
                    OutputMode::On
                } else {
                    OutputMode::Off
                };
                self.draft
                    .verbosity
                    .set_mode_at(output, self.customized, mode);
                Some(Outcome::Nothing)
            }
            _ => None,
        }
    }

    /// Handles what the user did to the dialog; events of other dialogs are ignored.
    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                self.change(control, value.clone())
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" | "apply" => {
                    let before = self.shown();
                    let scope = self.scope();
                    // What was applied is what the place now holds.
                    if let Some(place) = self.places.get_mut(self.place) {
                        place.settings = self.draft.clone();
                    }
                    Outcome::Apply {
                        scope,
                        before: Box::new(before),
                        settings: Box::new(self.draft.clone()),
                        close: control == "ok",
                    }
                }
                _ => Outcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }

    fn change(&mut self, control: &str, value: Value) -> Outcome {
        // Another place: its settings are shown, and the controls it cannot keep dimmed.
        if let ("scope", Value::Selected(i)) = (control, &value) {
            if *i != self.place && *i < self.places.len() {
                self.place = *i;
                self.draft = self.shown();
                let mut sent = self.values();
                sent.extend(self.enabling());
                return Outcome::Send(sent);
            }
            return Outcome::Nothing;
        }
        if let Some(outcome) = self.change_verbosity(control, &value) {
            return outcome;
        }
        let Some(spec) = registry::find(control)
            .filter(|spec| spec.label.is_some() && spec.scopes.allows(&self.scope()))
        else {
            return Outcome::Nothing;
        };
        let Some(setting) = self.value_of(spec, value) else {
            return Outcome::Nothing;
        };
        let shown = self.values();
        registry::set(&mut self.draft, spec.id, setting);
        // What changed with it is shown too: a layout's own Okur key.
        let followed: Vec<ToUi> = self
            .values()
            .into_iter()
            .filter(|v| !shown.contains(v))
            .filter(|v| !matches!(v, ToUi::DialogSet { control: c, .. } if c == control))
            .collect();
        if followed.is_empty() {
            Outcome::Nothing
        } else {
            Outcome::Send(followed)
        }
    }

    /// The setting's value a control's value stands for.
    fn value_of(&self, spec: &SettingSpec, value: Value) -> Option<Json> {
        match (spec.kind, value) {
            (Kind::Choice(choices), Value::Selected(i)) => {
                choices().into_iter().nth(i).map(|c| c.value)
            }
            (Kind::Check { on, off }, Value::Checked(checked)) => {
                Some(if checked { on } else { off }.value())
            }
            (Kind::Range { min, max }, Value::Number(n)) => Some(Json::from(n.clamp(min, max))),
            (Kind::Voice, Value::Selected(i)) => Some(
                i.checked_sub(1)
                    .and_then(|v| self.voices.get(v))
                    .map_or(Json::Null, |name| Json::String(name.clone())),
            ),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{
        KeyboardLayout, KeyboardSettings, OkurKey, SayAllUnit, TypingEcho, ViewerDisplay,
    };

    /// A verbosity level is customized as JAWS's Settings Center does: the level, then its
    /// spoken items and message length, each kept for that level alone.
    #[test]
    fn a_verbosity_levels_items_and_length_are_customized() {
        let settings = Settings::default();
        let mut dialog = SettingsDialog::new(1, everywhere(&settings), 0, Vec::new());
        let changed = |control: &str, value: Value| FromUi::DialogChanged {
            id: 1,
            control: control.to_owned(),
            value,
        };
        // Advanced: positions in lists off by default, as JAWS ships it.
        let shown = dialog.handle(&changed(CUSTOMIZE, Value::Selected(2)));
        let list_position = format!("{OUTPUT}list_position");
        assert!(matches!(
            &shown,
            Outcome::Send(sent) if sent.contains(&ToUi::DialogSet {
                id: 1,
                control: list_position.clone(),
                value: Value::Checked(false),
            })
        ));
        dialog.handle(&changed(&list_position, Value::Checked(true)));
        dialog.handle(&changed(LENGTH, Value::Selected(0)));
        let verbosity = &dialog.draft().verbosity;
        assert_eq!(
            verbosity.mode_at(OutputType::ListPosition, 2),
            OutputMode::On
        );
        assert_eq!(
            verbosity.mode_at(OutputType::ListPosition, 1),
            OutputMode::On
        );
        assert_eq!(
            verbosity.message_length,
            [
                MessageLength::Long,
                MessageLength::Short,
                MessageLength::Long
            ]
        );
        // The level in force is another setting.
        assert_eq!(verbosity.level, settings.verbosity.level);
    }

    /// The dialog's one place, all applications, with `settings`.
    fn everywhere(settings: &Settings) -> Vec<Place> {
        vec![Place {
            scope: Scope::Global,
            name: "Default (All Applications)".into(),
            settings: settings.clone(),
        }]
    }

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
            everywhere(&settings),
            0,
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
            find("voice.voice"),
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
                options: vec![
                    "Off".into(),
                    "Characters".into(),
                    "Words".into(),
                    "Characters and Words".into()
                ],
                selected: 0
            })
        );
        assert_eq!(
            find("verbosity.level"),
            Some(ControlKind::Choice {
                options: vec!["Beginner".into(), "Intermediate".into(), "Advanced".into()],
                selected: 0
            })
        );
        assert_eq!(
            find("say_all_unit"),
            Some(ControlKind::Choice {
                options: vec!["Line".into(), "Sentence".into(), "Paragraph".into()],
                selected: 0
            })
        );
        assert_eq!(
            find("viewer_display"),
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
        let mut dialog = SettingsDialog::new(4, everywhere(&Settings::default()), 0, Vec::new());
        assert_eq!(
            dialog.handle(&changed("voice.rate", Value::Number(3))),
            Outcome::Nothing
        );
        assert_eq!(
            dialog.handle(&changed("typing_echo", Value::Selected(0))),
            Outcome::Nothing
        );
        assert_eq!(
            dialog.handle(&changed("viewer_display", Value::Checked(false))),
            Outcome::Nothing
        );
        assert_eq!(
            dialog.handle(&changed("voice.punctuation", Value::Selected(3))),
            Outcome::Nothing
        );
        assert_eq!(
            dialog.handle(&changed("say_all_unit", Value::Selected(1))),
            Outcome::Nothing
        );
        // Another dialog's events change nothing.
        let other = FromUi::DialogChanged {
            id: 9,
            control: "voice.rate".into(),
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
            say_all_unit: SayAllUnit::Sentence,
            ..Default::default()
        };
        expected.voice.rate = 3;
        expected.voice.punctuation = crate::settings::Punctuation::All;
        assert_eq!(
            applied,
            Outcome::Apply {
                scope: Scope::Global,
                before: Box::default(),
                settings: Box::new(expected.clone()),
                close: false
            }
        );
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 4,
                control: "ok".into()
            }),
            Outcome::Apply {
                scope: Scope::Global,
                before: Box::new(expected.clone()),
                settings: Box::new(expected),
                close: true
            }
        );
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 4 }),
            Outcome::Closed
        );
    }

    /// "Settings for" chooses where the dialog's changes go, as JAWS's Settings Center's
    /// Application box: the dialog opens on the application in front with its settings, shows
    /// all applications' when chosen, and applies to the place chosen.
    #[test]
    fn the_dialog_keeps_its_changes_where_chosen() {
        let global = Settings::default();
        let mut notepad = global.clone();
        notepad.voice.punctuation = crate::settings::Punctuation::All;
        let places = vec![
            everywhere(&global).remove(0),
            Place {
                scope: Scope::Application("notepad".into()),
                name: "Notepad".into(),
                settings: notepad.clone(),
            },
        ];
        let mut dialog = SettingsDialog::new(4, places, 1, Vec::new());
        let scope_box = dialog
            .spec()
            .controls
            .into_iter()
            .find(|c| c.id == "scope")
            .map(|c| (c.label, c.kind));
        assert_eq!(
            scope_box,
            Some((
                "Settings &for:".to_owned(),
                ControlKind::Choice {
                    options: vec!["Default (All Applications)".into(), "Notepad".into()],
                    selected: 1
                }
            ))
        );
        assert_eq!(dialog.draft(), &notepad);
        // All applications: their settings are shown, punctuation among them.
        let shown = dialog.handle(&changed("scope", Value::Selected(0)));
        let punctuation = ToUi::DialogSet {
            id: 4,
            control: "voice.punctuation".into(),
            // Most, JAWS's default.
            value: Value::Selected(2),
        };
        assert!(
            matches!(&shown, Outcome::Send(set) if set.contains(&punctuation)),
            "{shown:?}"
        );
        assert_eq!(dialog.draft(), &global);
        dialog.handle(&changed("typing_echo", Value::Selected(0)));
        let applied = dialog.handle(&FromUi::DialogPressed {
            id: 4,
            control: "apply".into(),
        });
        assert!(
            matches!(&applied, Outcome::Apply { scope: Scope::Global, before, .. } if **before == global),
            "{applied:?}"
        );
    }

    /// An application's place cannot keep the keyboard layout or the Okur key, which JAWS
    /// keeps for all applications only: their controls are dimmed there, and lit again for all
    /// applications.
    #[test]
    fn an_application_cannot_keep_the_okur_key() {
        let places = vec![
            everywhere(&Settings::default()).remove(0),
            Place {
                scope: Scope::Application("notepad".into()),
                name: "Notepad".into(),
                settings: Settings::default(),
            },
        ];
        let mut dialog = SettingsDialog::new(4, places, 1, Vec::new());
        let enabled = |dialog: &SettingsDialog, id: &str| {
            dialog
                .spec()
                .controls
                .into_iter()
                .find(|c| c.id == id)
                .map(|c| c.enabled)
        };
        assert_eq!(enabled(&dialog, "keyboard.okur_key"), Some(false));
        assert_eq!(enabled(&dialog, "keyboard.layout"), Some(false));
        assert_eq!(enabled(&dialog, "typing_echo"), Some(true));
        // A change sent anyway is not taken.
        dialog.handle(&changed("keyboard.okur_key", Value::Selected(2)));
        assert_eq!(dialog.draft().keyboard.okur_key, OkurKey::Insert);
        let shown = dialog.handle(&changed("scope", Value::Selected(0)));
        let lit = ToUi::DialogEnable {
            id: 4,
            control: "keyboard.okur_key".into(),
            enabled: true,
        };
        assert!(
            matches!(&shown, Outcome::Send(sent) if sent.contains(&lit)),
            "{shown:?}"
        );
    }

    #[test]
    fn a_layout_brings_its_own_jaws_key() {
        let mut dialog = SettingsDialog::new(4, everywhere(&Settings::default()), 0, Vec::new());
        assert_eq!(
            dialog.handle(&changed("keyboard.layout", Value::Selected(1))),
            Outcome::Send(vec![ToUi::DialogSet {
                id: 4,
                control: "keyboard.okur_key".into(),
                value: Value::Selected(1),
            }])
        );
        assert_eq!(
            dialog.draft().keyboard,
            KeyboardSettings::for_layout(KeyboardLayout::Laptop)
        );
        // Choosing another key afterwards keeps the layout.
        dialog.handle(&changed("keyboard.okur_key", Value::Selected(2)));
        assert_eq!(dialog.draft().keyboard.okur_key, OkurKey::ScrollLock);
        assert_eq!(dialog.draft().keyboard.layout, KeyboardLayout::Laptop);
    }
}
