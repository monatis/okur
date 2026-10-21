//! Every setting, declared once (`docs/references/jaws-customization-map.md` §7): its path in
//! [`Settings`], the group and label a dialog shows it with, its kind and values, where it may
//! be set, and the JAWS settings file key it is read from. The settings dialog and the import of
//! JAWS's settings files are made from this table, so they cannot drift apart (the dialog once
//! offered two of typing echo's four values); a test keeps the table and [`Settings`] in step.
//!
//! The engine still reads [`Settings`]' typed fields: the table is for what works on settings
//! by name (dialogs, files, imports), never for a key's hot path.

use serde::Serialize;
use serde_json::{Map, Value};

use super::{
    KeyboardLayout, KeyboardSettings, LessSpeech, OkurKey, OutputType, Punctuation, SayAllUnit,
    Settings, SpeechMode, Synthesizer, TypingEcho, Verbosity, VerbosityLevel,
};
use crate::messages::{msg, StaticMessage};
use crate::settings_layers::Scope;

/// A catalog label.
pub type Label = fn() -> StaticMessage;

/// An enum setting's values, in the order a dialog lists them, with their labels.
pub trait Choices: Copy + PartialEq + Serialize + 'static {
    const ALL: &'static [Self];
    /// The value's label in a dialog.
    fn label(self) -> StaticMessage;
}

/// One value of a choice: as the settings file writes it, and as a dialog shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub value: Value,
    pub label: StaticMessage,
}

/// `T`'s values as choices.
pub fn choices<T: Choices>() -> Vec<Choice> {
    T::ALL
        .iter()
        .map(|c| Choice {
            value: json(*c),
            label: c.label(),
        })
        .collect()
}

/// A value as the settings file writes it.
fn json<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// Where a dialog shows a setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Speech,
    Keyboard,
    Reading,
}

impl Group {
    pub const ALL: [Group; 3] = [Group::Speech, Group::Keyboard, Group::Reading];

    /// The group's control id in a dialog.
    pub fn id(self) -> &'static str {
        match self {
            Self::Speech => "speech",
            Self::Keyboard => "keyboard",
            Self::Reading => "reading",
        }
    }

    pub fn label(self) -> StaticMessage {
        match self {
            Self::Speech => msg::settings_group_speech(),
            Self::Keyboard => msg::settings_group_keyboard(),
            Self::Reading => msg::settings_group_reading(),
        }
    }
}

/// A value a check box stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Literal {
    Bool(bool),
    Text(&'static str),
}

impl Literal {
    pub fn value(self) -> Value {
        match self {
            Self::Bool(b) => Value::Bool(b),
            Self::Text(t) => Value::String(t.to_owned()),
        }
    }
}

/// What kind of value a setting holds, and so how a dialog shows it.
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    /// One of an enum's values (a combo box).
    Choice(fn() -> Vec<Choice>),
    /// One of two values (a check box): checked is `on`.
    Check { on: Literal, off: Literal },
    /// A whole number from `min` to `max` (a slider).
    Range { min: i32, max: i32 },
    /// One of the synthesizer's voices by name, or none for its default.
    Voice,
    /// A table of values (per output type, per verbosity level), set in the settings file until
    /// its own dialog comes (JAWS's Configure Verbosity Levels).
    Structured,
}

/// Where a setting may be set, besides for the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scopes {
    /// For all applications, one application, or one site.
    Everywhere,
    /// For all applications only, as JAWS keeps its key in the default file only ("JAWS Key for
    /// Current Layout", Settings Center help).
    AllApplications,
}

impl Scopes {
    pub fn allows(self, scope: &Scope) -> bool {
        match self {
            Self::Everywhere => true,
            Self::AllApplications => matches!(scope, Scope::Global | Scope::Session),
        }
    }
}

/// One of JAWS's settings files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JawsFile {
    /// The user's `DEFAULT.JCF`: the settings for all applications.
    DefaultJcf,
}

/// A key's value in JAWS's settings files, by file, section and key; `None` when the file does
/// not name it, which means JAWS's default.
pub type JawsLookup<'a> = &'a dyn Fn(JawsFile, &str, &str) -> Option<String>;

/// How a setting is read from JAWS's settings files.
#[derive(Debug, Clone, Copy)]
pub enum Jaws {
    /// JAWS's files do not hold it, or Okur does not read it yet.
    Unread,
    /// One key, decoded into the setting's value. A key the file leaves out is JAWS's default,
    /// which is Okur's.
    Key {
        file: JawsFile,
        section: &'static str,
        key: &'static str,
        decode: fn(&str) -> Value,
    },
    /// Read by a function of its own, from several keys.
    Import(fn(JawsLookup<'_>, &mut Settings)),
}

/// One setting.
#[derive(Debug, Clone, Copy)]
pub struct SettingSpec {
    /// Its path in [`Settings`] and in the settings file, also its control id in a dialog:
    /// "voice.punctuation", "typing_echo".
    pub id: &'static str,
    pub group: Group,
    /// Its label in the settings dialog; none while it is set in the settings file only.
    pub label: Option<Label>,
    pub kind: Kind,
    pub scopes: Scopes,
    pub jaws: Jaws,
    /// What changes with it: a keyboard layout brings its own Okur key, a synthesizer its own
    /// default voice.
    pub follows: Option<fn(&mut Settings)>,
}

/// A spec with the defaults most settings share.
const fn spec(id: &'static str, group: Group, label: Option<Label>, kind: Kind) -> SettingSpec {
    SettingSpec {
        id,
        group,
        label,
        kind,
        scopes: Scopes::Everywhere,
        jaws: Jaws::Unread,
        follows: None,
    }
}

/// `DEFAULT.JCF`'s key `key` in `section`, decoded by `decode`.
const fn default_jcf(section: &'static str, key: &'static str, decode: fn(&str) -> Value) -> Jaws {
    Jaws::Key {
        file: JawsFile::DefaultJcf,
        section,
        key,
        decode,
    }
}

/// Every setting, in the order the settings dialog shows them within each group.
pub static SETTINGS: &[SettingSpec] = &[
    SettingSpec {
        follows: Some(|s| s.voice.voice = None),
        ..spec(
            "voice.synthesizer",
            Group::Speech,
            Some(msg::settings_synthesizer),
            Kind::Choice(choices::<Synthesizer>),
        )
    },
    spec(
        "voice.voice",
        Group::Speech,
        Some(msg::settings_voice),
        Kind::Voice,
    ),
    spec(
        "voice.rate",
        Group::Speech,
        Some(msg::settings_rate),
        Kind::Range { min: -10, max: 10 },
    ),
    spec(
        "voice.volume",
        Group::Speech,
        Some(msg::settings_volume),
        Kind::Range { min: 0, max: 100 },
    ),
    spec(
        "voice.punctuation",
        Group::Speech,
        Some(msg::settings_punctuation),
        Kind::Choice(choices::<Punctuation>),
    ),
    // Full Speech or less speech, and less speech's kind (`SpeechMode`, `LessSpeechMode`;
    // 700 and 707 record them); Insert+Space, S and Shift+S change them for the session.
    SettingSpec {
        jaws: default_jcf("Options", "SpeechMode", |v| json(SpeechMode::from_jcf(v))),
        ..spec(
            "speech_mode",
            Group::Speech,
            None,
            Kind::Choice(choices::<SpeechMode>),
        )
    },
    SettingSpec {
        jaws: default_jcf("Options", "LessSpeechMode", |v| {
            json(LessSpeech::from_jcf(v))
        }),
        ..spec(
            "less_speech",
            Group::Speech,
            None,
            Kind::Choice(choices::<LessSpeech>),
        )
    },
    SettingSpec {
        scopes: Scopes::AllApplications,
        jaws: default_jcf("Options", "KeyboardType", |v| {
            json(KeyboardLayout::from_jcf(v))
        }),
        follows: Some(|s| s.keyboard = KeyboardSettings::for_layout(s.keyboard.layout)),
        ..spec(
            "keyboard.layout",
            Group::Keyboard,
            Some(msg::settings_layout),
            Kind::Choice(choices::<KeyboardLayout>),
        )
    },
    SettingSpec {
        scopes: Scopes::AllApplications,
        ..spec(
            "keyboard.okur_key",
            Group::Keyboard,
            Some(msg::settings_okur_key),
            Kind::Choice(choices::<OkurKey>),
        )
    },
    SettingSpec {
        jaws: default_jcf("Options", "TypingEcho", |v| json(TypingEcho::from_jcf(v))),
        ..spec(
            "typing_echo",
            Group::Reading,
            Some(msg::settings_typing_echo),
            Kind::Choice(choices::<TypingEcho>),
        )
    },
    // Say All Reads (`SayAllMode`; 677-681 record each).
    SettingSpec {
        jaws: default_jcf("Options", "SayAllMode", |v| json(SayAllUnit::from_jcf(v))),
        ..spec(
            "say_all_unit",
            Group::Reading,
            Some(msg::settings_say_all_unit),
            Kind::Choice(choices::<SayAllUnit>),
        )
    },
    // The user verbosity level (`Verbosity`; 548-555 record each).
    SettingSpec {
        jaws: default_jcf("Options", "Verbosity", |v| {
            json(VerbosityLevel::from_jcf(v))
        }),
        ..spec(
            "verbosity.level",
            Group::Reading,
            Some(msg::settings_verbosity),
            Kind::Choice(choices::<VerbosityLevel>),
        )
    },
    SettingSpec {
        jaws: Jaws::Import(import_output_modes),
        ..spec(
            "verbosity.output_modes",
            Group::Reading,
            None,
            Kind::Structured,
        )
    },
    spec(
        "verbosity.message_length",
        Group::Reading,
        None,
        Kind::Structured,
    ),
    // Auto Forms Mode (`[FormsMode] AutoFormsMode`, on unless "0"); its three-way Forms Mode
    // (Auto, Semi-Auto, Manual) comes with its dialog control.
    SettingSpec {
        jaws: default_jcf("FormsMode", "AutoFormsMode", |v| {
            Value::Bool(v.trim() != "0")
        }),
        ..spec(
            "auto_forms_mode",
            Group::Reading,
            None,
            Kind::Check {
                on: Literal::Bool(true),
                off: Literal::Bool(false),
            },
        )
    },
    spec(
        "live_region_append",
        Group::Reading,
        Some(msg::settings_live_region_whole_text),
        Kind::Check {
            on: Literal::Text("whole_text"),
            off: Literal::Text("appended"),
        },
    ),
    spec(
        "viewer_display",
        Group::Reading,
        Some(msg::settings_viewer_on_screen),
        Kind::Check {
            on: Literal::Text("on_screen"),
            off: Literal::Text("speech_only"),
        },
    ),
    spec(
        "confirm_exit",
        Group::Reading,
        Some(msg::settings_confirm_exit),
        Kind::Check {
            on: Literal::Bool(true),
            off: Literal::Bool(false),
        },
    ),
];

/// The setting with `id`.
pub fn find(id: &str) -> Option<&'static SettingSpec> {
    SETTINGS.iter().find(|s| s.id == id)
}

/// The setting's value in `settings`, as the settings file writes it.
pub fn get(settings: &Settings, id: &str) -> Option<Value> {
    let mut value = &json(settings);
    for name in id.split('.') {
        value = value.get(name)?;
    }
    Some(value.clone())
}

/// Sets the setting `id` to `value`, and what follows it when it changed. `false` when `value`
/// is not one of its values, and `settings` stays as it was.
pub fn set(settings: &mut Settings, id: &str, value: Value) -> bool {
    let Some(spec) = find(id) else {
        return false;
    };
    if get(settings, id).as_ref() == Some(&value) {
        return true;
    }
    let mut tree = json(&*settings);
    let Some(slot) = id
        .split('.')
        .try_fold(&mut tree, |node, name| node.get_mut(name))
    else {
        return false;
    };
    *slot = value;
    let Ok(changed) = serde_json::from_value::<Settings>(tree) else {
        return false;
    };
    *settings = changed;
    if let Some(follows) = spec.follows {
        follows(settings);
    }
    true
}

/// The value `text` names for a setting, as a command line gives it: a choice or a check box
/// as the settings file writes it ("words", "whole_text", "true"), a number, a voice's name. An
/// error says which values the setting takes.
pub fn parse(spec: &SettingSpec, text: &str) -> Result<Value, String> {
    let named = |value: &Value| match value {
        Value::String(s) => s.eq_ignore_ascii_case(text.trim()),
        Value::Bool(b) => text
            .trim()
            .eq_ignore_ascii_case(if *b { "true" } else { "false" }),
        _ => false,
    };
    let offered = |values: &[Value]| {
        let names: Vec<String> = values
            .iter()
            .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned))
            .collect();
        format!("{} takes {}, not {text:?}", spec.id, names.join(", "))
    };
    match spec.kind {
        Kind::Choice(choices) => {
            let values: Vec<Value> = choices().into_iter().map(|c| c.value).collect();
            values
                .iter()
                .find(|v| named(v))
                .cloned()
                .ok_or_else(|| offered(&values))
        }
        Kind::Check { on, off } => {
            let values = [on.value(), off.value()];
            values
                .iter()
                .find(|v| named(v))
                .cloned()
                .ok_or_else(|| offered(&values))
        }
        Kind::Range { min, max } => text
            .trim()
            .parse::<i32>()
            .ok()
            .filter(|n| (min..=max).contains(n))
            .map(Value::from)
            .ok_or_else(|| {
                format!(
                    "{} takes a number from {min} to {max}, not {text:?}",
                    spec.id
                )
            }),
        Kind::Voice => Ok(Value::String(text.to_owned())),
        Kind::Structured => Err(format!("{} is set in the settings file", spec.id)),
    }
}

/// Sets the setting `id` to the value `text` names ([`parse`]).
pub fn set_named(settings: &mut Settings, id: &str, text: &str) -> Result<(), String> {
    let spec = find(id).ok_or_else(|| format!("no setting {id:?}"))?;
    let value = parse(spec, text)?;
    if set(settings, id, value) {
        Ok(())
    } else {
        Err(format!("{id} cannot be {text:?}"))
    }
}

/// The settings JAWS's files give, over `base` (what the files cannot say: the reference
/// machine's voice rate, Okur's own choices): every key the table reads, a key the files leave
/// out being JAWS's default.
pub fn import_jaws(base: &Settings, lookup: JawsLookup<'_>) -> Settings {
    let defaults = Settings::default();
    let mut settings = base.clone();
    for spec in SETTINGS {
        match spec.jaws {
            Jaws::Unread => {}
            Jaws::Key {
                file,
                section,
                key,
                decode,
            } => {
                let value = match lookup(file, section, key) {
                    Some(found) => decode(&found),
                    None => get(&defaults, spec.id).unwrap_or(Value::Null),
                };
                if !set(&mut settings, spec.id, value) {
                    tracing::warn!(setting = spec.id, key, "JAWS's value not understood");
                }
            }
            Jaws::Import(import) => import(lookup, &mut settings),
        }
    }
    settings
}

/// The verbosity output types `DEFAULT.JCF`'s `[OutputModes]` names: one mode per level and the
/// type's name ("0|0|0|Access Key"). A type the file leaves out has JAWS's shipped modes.
fn import_output_modes(lookup: JawsLookup<'_>, settings: &mut Settings) {
    for output in OutputType::ALL {
        let Some(key) = output.jcf_key() else {
            continue;
        };
        let modes = lookup(JawsFile::DefaultJcf, "OutputModes", key)
            .and_then(|v| Verbosity::modes_from_jcf(&v))
            .unwrap_or(output.default_modes());
        settings.verbosity.set_modes(output, modes);
    }
}

/// The ids of the settings `values` holds that `scope` may not hold, which a layer drops.
pub fn refused(values: &Map<String, Value>, scope: &Scope) -> Vec<&'static str> {
    SETTINGS
        .iter()
        .filter(|s| !s.scopes.allows(scope))
        .filter(|s| {
            let mut names = s.id.split('.');
            let first = names.next().and_then(|name| values.get(name));
            names
                .fold(first, |node, name| node.and_then(|n| n.get(name)))
                .is_some()
        })
        .map(|s| s.id)
        .collect()
}

impl Choices for Synthesizer {
    const ALL: &'static [Self] = &[Self::Auto, Self::OneCore, Self::Sapi5, Self::NoSpeech];
    fn label(self) -> StaticMessage {
        match self {
            Self::Auto => msg::settings_synthesizer_automatic(),
            Self::OneCore => msg::settings_synthesizer_onecore(),
            Self::Sapi5 => msg::settings_synthesizer_sapi5(),
            Self::NoSpeech => msg::settings_synthesizer_none(),
        }
    }
}

/// JAWS's punctuation levels, as Voice Adjustment lists them.
impl Choices for Punctuation {
    const ALL: &'static [Self] = &[Self::None, Self::Some, Self::Most, Self::All];
    fn label(self) -> StaticMessage {
        match self {
            Self::None => msg::punctuation_none(),
            Self::Some => msg::punctuation_some(),
            Self::Most => msg::punctuation_most(),
            Self::All => msg::punctuation_all(),
        }
    }
}

impl Choices for SpeechMode {
    const ALL: &'static [Self] = &[Self::Full, Self::Less];
    fn label(self) -> StaticMessage {
        match self {
            Self::Full => msg::settings_speech_mode_full(),
            Self::Less => msg::settings_speech_mode_less(),
        }
    }
}

impl Choices for LessSpeech {
    const ALL: &'static [Self] = &[Self::OnDemand, Self::Muted];
    fn label(self) -> StaticMessage {
        match self {
            Self::OnDemand => msg::settings_less_speech_on_demand(),
            Self::Muted => msg::settings_less_speech_muted(),
        }
    }
}

impl Choices for KeyboardLayout {
    const ALL: &'static [Self] = &[Self::Desktop, Self::Laptop];
    fn label(self) -> StaticMessage {
        match self {
            Self::Desktop => msg::settings_layout_desktop(),
            Self::Laptop => msg::settings_layout_laptop(),
        }
    }
}

impl Choices for OkurKey {
    const ALL: &'static [Self] = &[Self::Insert, Self::CapsLock, Self::ScrollLock];
    fn label(self) -> StaticMessage {
        match self {
            Self::Insert => msg::settings_okur_key_insert(),
            Self::CapsLock => msg::settings_okur_key_caps_lock(),
            Self::ScrollLock => msg::settings_okur_key_scroll_lock(),
        }
    }
}

/// JAWS's typing echo choices, as Settings Center names them (User Options).
impl Choices for TypingEcho {
    const ALL: &'static [Self] = &[Self::None, Self::Characters, Self::Words, Self::Both];
    fn label(self) -> StaticMessage {
        match self {
            Self::None => msg::settings_typing_echo_off(),
            Self::Characters => msg::settings_typing_echo_characters(),
            Self::Words => msg::settings_typing_echo_words(),
            Self::Both => msg::settings_typing_echo_both(),
        }
    }
}

/// JAWS's Say All Reads choices (Settings Center, Say All).
impl Choices for SayAllUnit {
    const ALL: &'static [Self] = &[Self::Line, Self::Sentence, Self::Paragraph];
    fn label(self) -> StaticMessage {
        match self {
            Self::Line => msg::settings_say_all_unit_line(),
            Self::Sentence => msg::settings_say_all_unit_sentence(),
            Self::Paragraph => msg::settings_say_all_unit_paragraph(),
        }
    }
}

/// JAWS's user verbosity levels.
impl Choices for VerbosityLevel {
    const ALL: &'static [Self] = &[Self::Beginner, Self::Intermediate, Self::Advanced];
    fn label(self) -> StaticMessage {
        match self {
            Self::Beginner => msg::settings_verbosity_beginner(),
            Self::Intermediate => msg::settings_verbosity_intermediate(),
            Self::Advanced => msg::settings_verbosity_advanced(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The leaves of `value` under `path` that no spec declares: a value is declared when its
    /// path is a spec's id; a table that is not is looked into, and an empty one is a leaf.
    fn undeclared(path: &str, value: &Value, out: &mut Vec<String>) {
        if find(path).is_some() {
            return;
        }
        match value {
            Value::Object(fields) if !fields.is_empty() => {
                for (name, inner) in fields {
                    let inner_path = if path.is_empty() {
                        name.clone()
                    } else {
                        format!("{path}.{name}")
                    };
                    undeclared(&inner_path, inner, out);
                }
            }
            _ => out.push(path.to_owned()),
        }
    }

    /// Every setting of [`Settings`] is declared once, and every declaration names one.
    #[test]
    fn the_table_declares_every_setting_once() {
        let mut missing = Vec::new();
        undeclared("", &json(Settings::default()), &mut missing);
        assert!(missing.is_empty(), "not declared: {missing:?}");
        let mut ids: Vec<&str> = SETTINGS.iter().map(|s| s.id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "declared twice");
        for spec in SETTINGS {
            assert!(get(&Settings::default(), spec.id).is_some(), "{}", spec.id);
        }
    }

    /// Each value a spec offers can be set and read back, so the table's values are the
    /// settings' own: every choice, both sides of a check box, a range's ends.
    #[test]
    fn every_offered_value_sets_its_setting() {
        for spec in SETTINGS {
            let values: Vec<Value> = match spec.kind {
                Kind::Choice(choices) => choices().into_iter().map(|c| c.value).collect(),
                Kind::Check { on, off } => vec![on.value(), off.value()],
                Kind::Range { min, max } => vec![json(min), json(max)],
                Kind::Voice => vec![json("Microsoft Zira"), Value::Null],
                Kind::Structured => Vec::new(),
            };
            for value in values {
                let mut settings = Settings::default();
                assert!(
                    set(&mut settings, spec.id, value.clone()),
                    "{} {value}",
                    spec.id
                );
                assert_eq!(get(&settings, spec.id), Some(value.clone()), "{}", spec.id);
            }
        }
        let mut settings = Settings::default();
        assert!(!set(&mut settings, "typing_echo", json("loud")));
        assert!(!set(&mut settings, "no_such_setting", json(true)));
        assert_eq!(settings, Settings::default());
    }

    /// Every enum setting offers all of its values: typing echo's four, not two.
    #[test]
    fn choices_offer_every_value() {
        assert_eq!(choices::<TypingEcho>().len(), 4);
        let labels: Vec<String> = choices::<TypingEcho>()
            .iter()
            .map(|c| c.label.to_string())
            .collect();
        assert_eq!(
            labels,
            ["Off", "Characters", "Words", "Characters and Words"]
        );
        for spec in SETTINGS {
            if let Kind::Choice(choices) = spec.kind {
                let values: Vec<String> =
                    choices().into_iter().map(|c| c.value.to_string()).collect();
                let mut unique = values.clone();
                unique.sort_unstable();
                unique.dedup();
                assert_eq!(unique.len(), values.len(), "{}", spec.id);
            }
        }
    }

    /// A command line names values as the settings file writes them, and is told the values a
    /// setting takes when it names another.
    #[test]
    fn a_command_line_names_values_as_the_file_does() {
        let mut settings = Settings::default();
        assert_eq!(set_named(&mut settings, "typing_echo", "Words"), Ok(()));
        assert_eq!(settings.typing_echo, TypingEcho::Words);
        assert_eq!(
            set_named(&mut settings, "live_region_append", "whole_text"),
            Ok(())
        );
        assert_eq!(set_named(&mut settings, "confirm_exit", "false"), Ok(()));
        assert!(!settings.confirm_exit);
        assert_eq!(set_named(&mut settings, "voice.rate", "-4"), Ok(()));
        assert_eq!(settings.voice.rate, -4);
        assert_eq!(
            set_named(&mut settings, "voice.synthesizer", "no-speech"),
            Ok(())
        );
        assert_eq!(
            set_named(&mut settings, "typing_echo", "loud"),
            Err("typing_echo takes none, characters, words, both, not \"loud\"".into())
        );
        assert!(set_named(&mut settings, "voice.rate", "11").is_err());
        assert!(set_named(&mut settings, "verbosity.output_modes", "on").is_err());
        assert!(set_named(&mut settings, "typing", "words").is_err());
    }

    /// A layout brings its own Okur key; a synthesizer its own default voice.
    #[test]
    fn what_follows_a_setting_changes_with_it() {
        let mut settings = Settings::default();
        assert!(set(&mut settings, "keyboard.layout", json("laptop")));
        assert_eq!(settings.keyboard.okur_key, OkurKey::CapsLock);
        // The same layout again keeps a key chosen after it.
        assert!(set(&mut settings, "keyboard.okur_key", json("scroll_lock")));
        assert!(set(&mut settings, "keyboard.layout", json("laptop")));
        assert_eq!(settings.keyboard.okur_key, OkurKey::ScrollLock);
        settings.voice.voice = Some("Microsoft Zira".into());
        assert!(set(&mut settings, "voice.synthesizer", json("sapi5")));
        assert_eq!(settings.voice.voice, None);
    }

    /// The Okur key and the layout are kept for all applications only, as JAWS keeps its key.
    #[test]
    fn some_settings_are_for_all_applications_only() {
        let values = match serde_json::json!({"keyboard": {"okur_key": "insert"}, "typing_echo": "none"})
        {
            Value::Object(values) => values,
            _ => Map::new(),
        };
        assert_eq!(
            refused(&values, &Scope::Application("notepad".into())),
            ["keyboard.okur_key"]
        );
        assert!(refused(&values, &Scope::Global).is_empty());
        assert!(refused(&values, &Scope::Session).is_empty());
    }

    /// The import of JAWS's files gives old-pc's settings from its `DEFAULT.JCF`, each key the
    /// file leaves out being JAWS's default.
    #[test]
    fn the_jaws_import_reads_the_reference_machine() {
        let jcf = |file: JawsFile, section: &str, key: &str| {
            let found = match (file, section, key) {
                (JawsFile::DefaultJcf, "Options", "KeyboardType") => "Laptop",
                (JawsFile::DefaultJcf, "Options", "TypingEcho") => "0",
                (JawsFile::DefaultJcf, "FormsMode", "AutoFormsMode") => "0",
                (JawsFile::DefaultJcf, "OutputModes", "ACCESS_KEY") => "0|0|0|Access Key",
                _ => return None,
            };
            Some(found.to_owned())
        };
        let mut base = Settings::reference_machine();
        // What the files say replaces the base's.
        base.typing_echo = TypingEcho::Both;
        base.say_all_unit = SayAllUnit::Paragraph;
        let imported = import_jaws(&base, &jcf);
        let mut expected = Settings::reference_machine();
        for output in OutputType::ALL {
            if output.jcf_key().is_some() && output != OutputType::AccessKey {
                expected.verbosity.set_modes(output, output.default_modes());
            }
        }
        assert_eq!(imported, expected);
        let desktop = import_jaws(&base, &|_, _, _| None);
        assert_eq!(desktop.keyboard, KeyboardSettings::default());
        assert!(desktop.auto_forms_mode);
    }
}
