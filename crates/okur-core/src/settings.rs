//! User settings, modeled on JAWS's options and defaulting to JAWS's defaults.
//!
//! The reference machine's JAWS is not on defaults (its `DEFAULT.JCF` sets the laptop
//! keyboard layout, typing echo off and manual forms mode), so parity replay runs with
//! [`Settings::reference_machine`], and every behavior that depends on a setting reads it from
//! here instead of assuming the reference configuration.

use serde::{Deserialize, Serialize};

use crate::input::{Key, KeyEvent};

/// JAWS's keyboard layouts (Basic Settings: Keyboard Layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardLayout {
    /// The default: Insert is the JAWS key.
    #[default]
    Desktop,
    /// CapsLock is the JAWS key, and "for most of those keystrokes, the INSERT key works as
    /// well" (JAWS help).
    Laptop,
}

/// The keys that can be the JAWS key (Settings Center: JAWS Key for Current Layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JawsKey {
    Insert,
    CapsLock,
    ScrollLock,
}

/// Which keys act as the JAWS key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardSettings {
    pub layout: KeyboardLayout,
    /// The JAWS key chosen for `layout`.
    pub jaws_key: JawsKey,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        Self::for_layout(KeyboardLayout::Desktop)
    }
}

impl KeyboardSettings {
    /// A layout with its default JAWS key: Insert for desktop, CapsLock for laptop.
    pub fn for_layout(layout: KeyboardLayout) -> Self {
        let jaws_key = match layout {
            KeyboardLayout::Desktop => JawsKey::Insert,
            KeyboardLayout::Laptop => JawsKey::CapsLock,
        };
        Self { layout, jaws_key }
    }

    /// Whether `key` acts as the JAWS key: the chosen one, and Insert in the laptop layout
    /// [obs, 309: CapsLock+T and Insert+T both speak the title in the laptop layout].
    pub fn is_jaws_key(&self, key: Key) -> bool {
        match key {
            Key::Insert => {
                self.jaws_key == JawsKey::Insert || self.layout == KeyboardLayout::Laptop
            }
            Key::CapsLock => self.jaws_key == JawsKey::CapsLock,
            Key::ScrollLock => self.jaws_key == JawsKey::ScrollLock,
            _ => false,
        }
    }

    /// `event` with the JAWS modifier (`modifiers.insert`) set when any JAWS key is held,
    /// and cleared when only a key that is not a JAWS key is held.
    pub fn resolve(&self, event: &KeyEvent) -> KeyEvent {
        let mut resolved = event.clone();
        let m = &event.modifiers;
        resolved.modifiers.insert = (m.insert && self.is_jaws_key(Key::Insert))
            || (m.caps_lock && self.is_jaws_key(Key::CapsLock))
            || (m.scroll_lock && self.is_jaws_key(Key::ScrollLock));
        // Kept when CapsLock is the JAWS key held: the laptop layout's keys are CapsLock's, and
        // Insert keeps the desktop's with them (Insert+N toggles the quick keys where CapsLock+N
        // is the next sentence [obs, 298/s1, 510/s2]).
        resolved.modifiers.caps_lock = m.caps_lock && self.is_jaws_key(Key::CapsLock);
        resolved.modifiers.scroll_lock = false;
        resolved
    }
}

/// What a live region's change speaks when text was appended to what it already said.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveRegionAppend {
    /// Only the appended text, from the start of a word the old text ended in: a streamed
    /// chat reply is read as it grows, once.
    #[default]
    Appended,
    /// The region's whole text again on every change, as JAWS 2025 does: a 40-word stream
    /// gave its growing paragraph 37 times (dev box, 2026-10-15). Appended lines (a new chat
    /// message) are read alone in both modes [obs, 206/s2-s3].
    WholeText,
}

/// What typing a character says (Basic Settings: Typing Echo).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypingEcho {
    /// Each character as it is typed, Space and Enter by name: JAWS's default (no
    /// `TypingEcho` in `DEFAULT.JCF`) [obs, 403/s8-s13].
    #[default]
    Characters,
    /// Each word once it is ended by Space or Enter; Enter by name [obs, 403/s15-s20].
    Words,
    /// Characters, and each word at its end, Space then not named [obs, 403/s22-s27].
    Both,
    /// Nothing, Enter neither: `TypingEcho=0`, the reference machine's and the dev box's. On
    /// the dev box no sound followed four letters typed in the Run box, while Backspace spoke
    /// (`okur wait-silence`, 2026-10-17) [obs, 403/s1-s6, s29-s34].
    None,
}

impl TypingEcho {
    /// The echo a `DEFAULT.JCF` `TypingEcho` value sets: "0" is none; anything else is read
    /// as characters (which values JAWS writes for words and for both is not known yet).
    pub fn from_jcf(value: &str) -> Self {
        if value.trim() == "0" {
            Self::None
        } else {
            Self::Characters
        }
    }

    /// The echo Typing Echo (Insert+2) moves to: from None, Characters, Words, Both and None
    /// again [obs, 403/s7, s14, s21, s28].
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Characters,
            Self::Characters => Self::Words,
            Self::Words => Self::Both,
            Self::Both => Self::None,
        }
    }

    /// The echo's name, as JAWS says it when it changes [obs, 403/s7].
    pub fn name(self) -> String {
        use crate::messages::msg;
        match self {
            Self::None => msg::typing_echo_none(),
            Self::Characters => msg::typing_echo_characters(),
            Self::Words => msg::typing_echo_words(),
            Self::Both => msg::typing_echo_both(),
        }
        .to_string()
    }

    /// Whether typed characters are said.
    pub fn says_characters(self) -> bool {
        matches!(self, Self::Characters | Self::Both)
    }

    /// Whether a word is said when it ends.
    pub fn says_words(self) -> bool {
        matches!(self, Self::Words | Self::Both)
    }
}

/// Whether the Virtual Viewer also shows in a window, for someone watching the screen:
/// JAWS's "Show Virtual Viewer on Screen" (`bVirtViewer`, on in its default settings). The
/// keys stay with the viewer either way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewerDisplay {
    #[default]
    OnScreen,
    SpeechOnly,
}

/// The speech synthesizers Okur can use.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Synthesizer {
    /// OneCore, else SAPI 5, else none.
    #[default]
    Auto,
    #[serde(rename = "onecore")]
    OneCore,
    #[serde(rename = "sapi5")]
    Sapi5,
    NoSpeech,
}

/// JAWS's punctuation level: from which level a symbol in running text is spoken by its name
/// (`docs/references/jaws-punctuation-levels.md`). Each symbol has the level it is named
/// from; a level names its own symbols and those of the levels below it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Punctuation {
    None,
    Some,
    /// JAWS's default.
    #[default]
    Most,
    All,
}

impl Punctuation {
    /// The level Set Punctuation Level (Insert+Shift+2) moves to: from Most, All, None, Some
    /// and Most again [obs, 400/s6, s12, s18, s24].
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Some,
            Self::Some => Self::Most,
            Self::Most => Self::All,
            Self::All => Self::None,
        }
    }

    /// The level's name, as JAWS says it when the level changes [obs, 400/s6].
    pub fn name(self) -> String {
        use crate::messages::msg;
        match self {
            Self::None => msg::punctuation_none(),
            Self::Some => msg::punctuation_some(),
            Self::Most => msg::punctuation_most(),
            Self::All => msg::punctuation_all(),
        }
        .to_string()
    }
}

/// The voice Okur speaks with (JAWS: Voice Adjustment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceSettings {
    pub synthesizer: Synthesizer,
    /// The voice's name; the synthesizer's own default when none.
    pub voice: Option<String>,
    /// -10 to 10.
    pub rate: i32,
    /// 0 to 100.
    pub volume: u16,
    /// Which symbols in running text are spoken by name. JAWS keeps it in the voice profile
    /// too, per voice context (Voice Adjustment's Punctuation).
    pub punctuation: Punctuation,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            synthesizer: Synthesizer::Auto,
            voice: None,
            rate: 0,
            volume: 100,
            punctuation: Punctuation::default(),
        }
    }
}

/// JAWS's user verbosity levels (Settings Center: Verbosity). Each level has its own output
/// modes and message length; JAWS starts at Beginner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerbosityLevel {
    #[default]
    Beginner,
    Intermediate,
    Advanced,
}

impl VerbosityLevel {
    /// The level's place in a per-level setting: Beginner first.
    pub fn index(self) -> usize {
        match self {
            Self::Beginner => 0,
            Self::Intermediate => 1,
            Self::Advanced => 2,
        }
    }
}

/// Whether an output type is spoken at a level. More modes only if JAWS's files show them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    Off,
    On,
}

/// Whether messages that have a short form are said long or short (JAWS's message length;
/// a message's short form is its `.short` attribute in the catalog).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageLength {
    Long,
    Short,
}

/// The kinds of speech verbosity switches on and off, as JAWS's output types: every message
/// JAWS speaks belongs to one. These are Okur's ids; each maps to JAWS's `DEFAULT.JCF`
/// `[OutputModes]` key where one is known. JAWS's full list is learned on the reference
/// machine (proposal §6.5), so only the types Okur needs are here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputType {
    /// A control's access key ("alt+Y") and a field's shortcut ("Ctrl+L"): JAWS's Access Key
    /// output takes both [obs, 220/s7].
    AccessKey,
    /// Usage hints: "To activate press Enter."
    Hint,
    /// An item's place as the arrows move within a list: "Cherries | 3 of 3" at Beginner and
    /// Intermediate, "Cherries" at Advanced; entering the list still says it [obs, 548-549/s8,
    /// s9]. JAWS's "Announce Position and Count when navigating in lists".
    ListPosition,
    /// A control's description (a page field's aria-describedby text): said at Beginner and
    /// Intermediate, not at Advanced ("Email address edit | sam@example.com | Type in text."
    /// [obs, 552-553/s4]). JAWS's key for it is not known.
    Description,
    /// The group box a dialog's control is in, said before its name: "Sort Links In Tab
    /// Order radio button" at Beginner and Intermediate, "In Tab Order radio button" at
    /// Advanced [obs, 585/s5, s6]. JAWS's "Control Group Name".
    ControlGroupName,
    /// "selected" and "unselected" as a selection grows or shrinks: said at Beginner and
    /// Intermediate, not at Advanced ("=" where Beginner says "= selected" [obs, 609/s16,
    /// 608/s2]). JAWS's "Select".
    Select,
    /// JAWS's own messages about what a key did: "Copied selection to clipboard" at
    /// Beginner, "Copied" at Intermediate (its short form), nothing at Advanced [obs,
    /// 599/s3, 609/s3, 608/s9]; "Pasted", "cut", "Undo" likewise. JAWS's "JAWS Message".
    JawsMessage,
}

impl OutputType {
    pub const ALL: [OutputType; 7] = [
        OutputType::AccessKey,
        OutputType::Hint,
        OutputType::ListPosition,
        OutputType::Description,
        OutputType::ControlGroupName,
        OutputType::Select,
        OutputType::JawsMessage,
    ];

    /// JAWS's key for the type in `DEFAULT.JCF` `[OutputModes]`, where it is known.
    pub fn jcf_key(self) -> Option<&'static str> {
        match self {
            Self::AccessKey => Some("ACCESS_KEY"),
            Self::Hint | Self::Description => None,
            Self::ListPosition => Some("ANNOUNCE_POSITION_AND_COUNT"),
            Self::ControlGroupName => Some("CONTROL_GROUP_NAME"),
            Self::Select => Some("SELECT"),
            Self::JawsMessage => Some("JAWS_MESSAGE"),
        }
    }

    /// The type's modes, Beginner first, where the user's settings name none: JAWS's shipped
    /// `DEFAULT.JCF` (`ANNOUNCE_POSITION_AND_COUNT=1|2|0`, `CONTROL_GROUP_NAME=1|2|0`,
    /// `SELECT=1|2|0`, `JAWS_MESSAGE=1|2|0`, `TUTOR=1|1|1`, `ACCESS_KEY=1|1|1`).
    pub fn default_modes(self) -> [OutputMode; 3] {
        match self {
            Self::AccessKey | Self::Hint => [OutputMode::On; 3],
            Self::ListPosition
            | Self::Description
            | Self::ControlGroupName
            | Self::Select
            | Self::JawsMessage => [OutputMode::On, OutputMode::On, OutputMode::Off],
        }
    }

    /// The output type a part of an announcement belongs to; `None` for parts verbosity
    /// does not switch.
    pub fn of_part(kind: crate::announcement::PartKind) -> Option<OutputType> {
        use crate::announcement::PartKind;
        match kind {
            PartKind::AccessKey | PartKind::Shortcut => Some(Self::AccessKey),
            PartKind::Hint => Some(Self::Hint),
            PartKind::ListPosition => Some(Self::ListPosition),
            PartKind::Description => Some(Self::Description),
            PartKind::GroupName => Some(Self::ControlGroupName),
            _ => None,
        }
    }
}

/// Verbosity, modeled on JAWS's (proposal §6.5): a level, and per level the output modes and
/// the message length. An output type not listed in `output_modes` is on at every level, as
/// JAWS's types are by default unless its settings say otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Verbosity {
    pub level: VerbosityLevel,
    /// One mode per level, Beginner first.
    pub output_modes: std::collections::BTreeMap<OutputType, [OutputMode; 3]>,
    /// One length per level, Beginner first.
    pub message_length: [MessageLength; 3],
}

impl Default for Verbosity {
    fn default() -> Self {
        Self {
            level: VerbosityLevel::Beginner,
            output_modes: std::collections::BTreeMap::new(),
            // Long at Beginner, short above (research §2.4).
            message_length: [
                MessageLength::Long,
                MessageLength::Short,
                MessageLength::Short,
            ],
        }
    }
}

impl Verbosity {
    /// Whether `output` is spoken at the current level.
    pub fn is_on(&self, output: OutputType) -> bool {
        self.mode(output) == OutputMode::On
    }

    /// `output`'s mode at the current level.
    pub fn mode(&self, output: OutputType) -> OutputMode {
        self.output_modes
            .get(&output)
            .and_then(|modes| modes.get(self.level.index()))
            .copied()
            .or_else(|| output.default_modes().get(self.level.index()).copied())
            .unwrap_or(OutputMode::On)
    }

    /// Sets `output`'s mode at each level, Beginner first.
    pub fn set_modes(&mut self, output: OutputType, modes: [OutputMode; 3]) {
        self.output_modes.insert(output, modes);
    }

    /// The message length at the current level.
    pub fn message_length(&self) -> MessageLength {
        self.message_length
            .get(self.level.index())
            .copied()
            .unwrap_or(MessageLength::Long)
    }

    /// Whether a part of `kind` is said: its output type is on, or it has none.
    pub fn says(&self, kind: crate::announcement::PartKind) -> bool {
        OutputType::of_part(kind).is_none_or(|output| self.is_on(output))
    }

    /// The modes a `DEFAULT.JCF` `[OutputModes]` value gives: `0|0|0|Access Key` is one mode
    /// per level, Beginner first ("0" off, anything else on), then the type's display name.
    /// `None` when the value has fewer than three modes.
    pub fn modes_from_jcf(value: &str) -> Option<[OutputMode; 3]> {
        let mut fields = value.split('|').map(|f| match f.trim() {
            "0" => OutputMode::Off,
            _ => OutputMode::On,
        });
        Some([fields.next()?, fields.next()?, fields.next()?])
    }
}

/// All user settings: what the settings dialog shows and the settings file keeps. A setting
/// missing from the file has its default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub voice: VoiceSettings,
    pub keyboard: KeyboardSettings,
    pub live_region_append: LiveRegionAppend,
    pub typing_echo: TypingEcho,
    pub viewer_display: ViewerDisplay,
    /// Forms mode turns on by itself when the focus lands on a field, as JAWS's Auto Forms
    /// Mode (`[FormsMode] AutoFormsMode`, on by default). Off, a field takes Enter first,
    /// and arrows in a page's menu stay the virtual cursor's (old-pc, 2026-10-18).
    pub auto_forms_mode: bool,
    /// What is said and how much: a control's access key is said after it ("alt+Y") while
    /// the `access_key` output type is on, as JAWS's Access Key output (`[OutputModes]
    /// ACCESS_KEY`, on by default).
    pub verbosity: Verbosity,
    /// Insert+F4 asks before Okur exits, as JAWS's "Prompt to Confirm When Exiting JAWS"
    /// (Options > Basics), checked by default: "When cleared, JAWS immediately closes when you
    /// use any of the commands to exit the program including INSERT+F4" (JAWS 2025 help).
    pub confirm_exit: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            voice: VoiceSettings::default(),
            keyboard: KeyboardSettings::default(),
            live_region_append: LiveRegionAppend::default(),
            typing_echo: TypingEcho::default(),
            viewer_display: ViewerDisplay::default(),
            auto_forms_mode: true,
            verbosity: Verbosity::default(),
            confirm_exit: true,
        }
    }
}

/// Where changed settings are kept, so that they hold the next time Okur starts: the saved
/// layers (all applications, each application, each site), never the session's.
pub trait SettingsStore: Send {
    fn save(&self, layers: &crate::settings_layers::SettingsLayers);
}

/// A store that keeps what it was given, for tests.
#[derive(Debug, Clone, Default)]
pub struct RecordingSettingsStore {
    saved: std::sync::Arc<std::sync::Mutex<Vec<crate::settings_layers::SettingsLayers>>>,
}

impl RecordingSettingsStore {
    /// The layers saved so far, emptied.
    pub fn take(&self) -> Vec<crate::settings_layers::SettingsLayers> {
        self.saved
            .lock()
            .map(|mut saved| std::mem::take(&mut *saved))
            .unwrap_or_default()
    }
}

impl SettingsStore for RecordingSettingsStore {
    fn save(&self, layers: &crate::settings_layers::SettingsLayers) {
        if let Ok(mut saved) = self.saved.lock() {
            saved.push(layers.clone());
        }
    }
}

impl Settings {
    /// The reference machine's JAWS configuration (`old-pc`'s `DEFAULT.JCF`, unchanged since
    /// 2025-08-19, before any evidence was recorded): `KeyboardType=Laptop`, `TypingEcho=0`,
    /// `AutoFormsMode=0`, `ACCESS_KEY=0|0|0`; its voice profile's rate, 90 in every context,
    /// which the voice receives as SAPI's fastest rate, 10 [obs, 500, the recorder's fragments];
    /// and JAWS's own reading of growing live regions, so replay and live runs compare like
    /// with like.
    pub fn reference_machine() -> Self {
        let mut verbosity = Verbosity::default();
        verbosity.set_modes(OutputType::AccessKey, [OutputMode::Off; 3]);
        Self {
            voice: VoiceSettings {
                rate: 10,
                ..VoiceSettings::default()
            },
            keyboard: KeyboardSettings::for_layout(KeyboardLayout::Laptop),
            live_region_append: LiveRegionAppend::WholeText,
            typing_echo: TypingEcho::None,
            viewer_display: ViewerDisplay::OnScreen,
            auto_forms_mode: false,
            verbosity,
            // Not in old-pc's JCF: JAWS's default.
            confirm_exit: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Modifiers;

    fn chord(key: Key, insert: bool, caps_lock: bool) -> KeyEvent {
        let mut e = KeyEvent::new(key, Modifiers::none(), true);
        e.modifiers.insert = insert;
        e.modifiers.caps_lock = caps_lock;
        e
    }

    /// JAWS's defaults turn Auto Forms Mode and the Access Key output on; old-pc's JCF turns
    /// both off, and a settings file that names neither keeps the defaults.
    #[test]
    fn forms_mode_and_access_keys_default_to_jaws() {
        let access_keys = |s: &Settings| s.verbosity.is_on(OutputType::AccessKey);
        let defaults = Settings::default();
        assert!(defaults.auto_forms_mode && access_keys(&defaults));
        let reference = Settings::reference_machine();
        assert!(!reference.auto_forms_mode && !access_keys(&reference));
        let older: Result<Settings, _> = serde_json::from_str(r#"{"typing_echo":"none"}"#);
        assert!(older.is_ok_and(|s| s.auto_forms_mode && access_keys(&s)));
    }

    /// Verbosity starts at Beginner with every output type on and long messages; a JCF
    /// output mode gives one mode per level, and the level picks the one in force.
    #[test]
    fn verbosity_follows_its_level_and_the_jcf_modes() {
        let mut verbosity = Verbosity::default();
        assert_eq!(verbosity.level, VerbosityLevel::Beginner);
        assert!(OutputType::ALL.iter().all(|t| verbosity.is_on(*t)));
        assert_eq!(verbosity.message_length(), MessageLength::Long);
        // A list's position as the arrows move in it is JAWS's 1|2|0: off at Advanced alone,
        // where the settings name no mode [obs, 549/s9].
        for (level, on) in [
            (VerbosityLevel::Intermediate, true),
            (VerbosityLevel::Advanced, false),
        ] {
            let at = Verbosity {
                level,
                ..Verbosity::default()
            };
            assert_eq!(at.is_on(OutputType::ListPosition), on, "{level:?}");
            assert!(at.is_on(OutputType::Hint));
        }

        assert_eq!(
            Verbosity::modes_from_jcf("0|0|0|Access Key"),
            Some([OutputMode::Off; 3])
        );
        assert_eq!(Verbosity::modes_from_jcf("1|0"), None);
        let modes = Verbosity::modes_from_jcf("1|0|0|Access Key");
        verbosity.set_modes(OutputType::AccessKey, modes.unwrap_or([OutputMode::On; 3]));
        assert!(verbosity.is_on(OutputType::AccessKey));
        verbosity.level = VerbosityLevel::Advanced;
        assert!(!verbosity.is_on(OutputType::AccessKey));
        assert!(verbosity.is_on(OutputType::Hint));
        assert_eq!(verbosity.message_length(), MessageLength::Short);

        use crate::announcement::PartKind;
        assert!(!verbosity.says(PartKind::AccessKey) && !verbosity.says(PartKind::Shortcut));
        assert!(verbosity.says(PartKind::Hint) && verbosity.says(PartKind::Name));
        assert_eq!(OutputType::AccessKey.jcf_key(), Some("ACCESS_KEY"));
    }

    /// The settings file keeps verbosity under Okur's ids.
    #[test]
    fn verbosity_round_trips_through_the_settings_file() {
        let reference = Settings::reference_machine();
        let json = serde_json::to_string(&reference).unwrap_or_default();
        assert!(
            json.contains(r#""verbosity":{"level":"beginner","output_modes":{"access_key":["off","off","off"]}"#),
            "{json}"
        );
        let back: Result<Settings, _> = serde_json::from_str(&json);
        assert!(back.is_ok_and(|s| s == reference));
    }

    #[test]
    fn desktop_layout_uses_insert_only() {
        let desktop = KeyboardSettings::default();
        assert!(desktop.is_jaws_key(Key::Insert));
        assert!(!desktop.is_jaws_key(Key::CapsLock));
        assert!(
            desktop
                .resolve(&chord(Key::Alpha('T'), true, false))
                .modifiers
                .insert
        );
        assert!(
            !desktop
                .resolve(&chord(Key::Alpha('T'), false, true))
                .modifiers
                .insert
        );
    }

    #[test]
    fn laptop_layout_uses_caps_lock_and_insert() {
        let laptop = KeyboardSettings::for_layout(KeyboardLayout::Laptop);
        assert!(laptop.is_jaws_key(Key::CapsLock));
        assert!(laptop.is_jaws_key(Key::Insert));
        let resolved = laptop.resolve(&chord(Key::Alpha('T'), false, true));
        assert!(resolved.modifiers.insert);
        // CapsLock is told apart: the laptop keys are its own.
        assert!(resolved.modifiers.caps_lock);
        assert!(
            laptop
                .resolve(&chord(Key::Alpha('T'), true, false))
                .modifiers
                .insert
        );
    }

    #[test]
    fn caps_lock_can_be_the_desktop_jaws_key() {
        let desktop_caps = KeyboardSettings {
            layout: KeyboardLayout::Desktop,
            jaws_key: JawsKey::CapsLock,
        };
        assert!(!desktop_caps.is_jaws_key(Key::Insert));
        assert!(
            desktop_caps
                .resolve(&chord(Key::F(12), false, true))
                .modifiers
                .insert
        );
    }
}
