//! User settings, modeled on JAWS's options and defaulting to JAWS's defaults.
//!
//! The reference machine's JAWS is not on defaults (its `DEFAULT.JCF` sets the laptop
//! keyboard layout, typing echo off and manual forms mode), so parity replay runs with
//! [`Settings::reference_machine`], and every behavior that depends on a setting reads it from
//! here instead of assuming the reference configuration.

use serde::{Deserialize, Serialize};

use crate::input::{Key, KeyEvent};

pub mod registry;

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

/// The keys that can be the Okur key (JAWS's Settings Center: JAWS Key for Current Layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OkurKey {
    Insert,
    CapsLock,
    ScrollLock,
}

/// Which keys act as the JAWS key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardSettings {
    pub layout: KeyboardLayout,
    /// The JAWS key chosen for `layout`, in Okur's name: the Okur key (AGENTS.md §1.5), as
    /// the settings file and `okur run --okur-key` name it.
    pub okur_key: OkurKey,
}

impl KeyboardLayout {
    /// The layout a `DEFAULT.JCF` `KeyboardType` value sets: "Laptop", else the desktop's.
    pub fn from_jcf(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("Laptop") {
            Self::Laptop
        } else {
            Self::Desktop
        }
    }
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        Self::for_layout(KeyboardLayout::Desktop)
    }
}

impl KeyboardSettings {
    /// A layout with its default JAWS key: Insert for desktop, CapsLock for laptop.
    pub fn for_layout(layout: KeyboardLayout) -> Self {
        let okur_key = match layout {
            KeyboardLayout::Desktop => OkurKey::Insert,
            KeyboardLayout::Laptop => OkurKey::CapsLock,
        };
        Self { layout, okur_key }
    }

    /// Whether `key` acts as the Okur key: the chosen one, and Insert in the laptop layout
    /// [obs, 309: CapsLock+T and Insert+T both speak the title in the laptop layout].
    pub fn is_okur_key(&self, key: Key) -> bool {
        match key {
            Key::Insert => {
                self.okur_key == OkurKey::Insert || self.layout == KeyboardLayout::Laptop
            }
            Key::CapsLock => self.okur_key == OkurKey::CapsLock,
            Key::ScrollLock => self.okur_key == OkurKey::ScrollLock,
            _ => false,
        }
    }

    /// `event` with the JAWS modifier (`modifiers.insert`) set when any JAWS key is held,
    /// and cleared when only a key that is not a JAWS key is held.
    pub fn resolve(&self, event: &KeyEvent) -> KeyEvent {
        let mut resolved = event.clone();
        let m = &event.modifiers;
        resolved.modifiers.insert = (m.insert && self.is_okur_key(Key::Insert))
            || (m.caps_lock && self.is_okur_key(Key::CapsLock))
            || (m.scroll_lock && self.is_okur_key(Key::ScrollLock));
        // Kept when CapsLock is the JAWS key held: the laptop layout's keys are CapsLock's, and
        // Insert keeps the desktop's with them (Insert+N toggles the quick keys where CapsLock+N
        // is the next sentence [obs, 298/s1, 510/s2]).
        resolved.modifiers.caps_lock = m.caps_lock && self.is_okur_key(Key::CapsLock);
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

/// What Say All gives the synthesizer at a time, and what Left and Right during it move by:
/// JAWS's Say All Reads, `SayAllMode` in `DEFAULT.JCF`'s Options (0 line, its default; 1
/// sentence; 2 paragraph).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SayAllUnit {
    #[default]
    Line,
    Sentence,
    Paragraph,
}

impl SayAllUnit {
    /// The unit a `DEFAULT.JCF` `SayAllMode` value sets; any other value is a line.
    pub fn from_jcf(value: &str) -> Self {
        match value.trim() {
            "1" => Self::Sentence,
            "2" => Self::Paragraph,
            _ => Self::Line,
        }
    }
}

/// How much mouse echo says of what is under the mouse: JAWS's Mouse Echo Unit,
/// `MouseSpeechEchoUnit` in `DEFAULT.JCF`'s Options (0 character, 1 word, 2 line, its default; 3
/// paragraph).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseEchoUnit {
    Character,
    Word,
    #[default]
    Line,
    Paragraph,
}

impl MouseEchoUnit {
    /// The unit a `DEFAULT.JCF` `MouseSpeechEchoUnit` value sets; any other value is a line.
    pub fn from_jcf(value: &str) -> Self {
        match value.trim() {
            "0" => Self::Character,
            "1" => Self::Word,
            "3" => Self::Paragraph,
            _ => Self::Line,
        }
    }
}

/// Mouse echo: what the mouse points to said as it moves, as JAWS's Mouse Echo group sets it
/// (Settings Center help; `DEFAULT.JCF`'s Options, JAWS's defaults here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MouseEcho {
    /// Enable Mouse Echo (`MouseSpeechEnabled`), off by default; Insert+Space, E, O turns it
    /// on and off.
    pub enabled: bool,
    /// Mouse Echo Unit (`MouseSpeechEchoUnit`).
    pub unit: MouseEchoUnit,
    /// Mouse Echo Instant (`MouseSpeechInstant`): said while the mouse moves, not after it
    /// rests for `delay_ms`. Off by default.
    pub instant: bool,
    /// Mouse Echo Delay (`MouseSpeechDelay`): how long the mouse rests before what it points
    /// to is said, 400 milliseconds by default.
    pub delay_ms: u32,
    /// Mouse Echo Interrupt (`MouseMovementStopsSpeech`): the mouse moving onto text stops
    /// speech. On by default.
    pub interrupt: bool,
    /// Speak Control Type and State of Item (`MouseEchoSpeaksControlTypeAndState`), after
    /// the text. On by default.
    pub control_type_and_state: bool,
    /// Speak Description of Item (`MouseEchoSpeaksHelpAndDescription`), after the text. Off
    /// by default.
    pub description: bool,
}

impl Default for MouseEcho {
    fn default() -> Self {
        Self {
            enabled: false,
            unit: MouseEchoUnit::Line,
            instant: false,
            delay_ms: 400,
            interrupt: true,
            control_type_and_state: true,
            description: false,
        }
    }
}

/// How much is spoken: JAWS's Full Speech, or less speech (`SpeechMode` in `DEFAULT.JCF`'s
/// Options: 0 full, its default; 1 less), the kind of less speech being [`LessSpeech`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechMode {
    #[default]
    Full,
    Less,
}

impl SpeechMode {
    /// The mode a `DEFAULT.JCF` `SpeechMode` value sets; any other value is full speech.
    pub fn from_jcf(value: &str) -> Self {
        if value.trim() == "1" {
            Self::Less
        } else {
            Self::Full
        }
    }
}

/// Less speech's kind (`LessSpeechMode`: 0 on demand, its default; 1 muted). On demand, only
/// the reading commands of the key map's [Speech On Demand] list speak (Say Line, Say Word,
/// Say Window Title, Say Color, ...); what the others say is still written to the speech
/// history ("banner" for a silent Down [obs, 700/s1]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LessSpeech {
    #[default]
    OnDemand,
    Muted,
}

impl LessSpeech {
    /// The kind a `DEFAULT.JCF` `LessSpeechMode` value sets; any other value is on demand.
    pub fn from_jcf(value: &str) -> Self {
        if value.trim() == "1" {
            Self::Muted
        } else {
            Self::OnDemand
        }
    }
}

/// When forms mode turns on by itself (Settings Center: Forms Mode; `[FormsMode]
/// AutoFormsMode`: 0 manual, 1 auto, 2 semi-auto).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormsMode {
    /// A field the focus or an arrow reaches is in forms mode: JAWS's default. Off, a field
    /// takes Enter first, and arrows in a page's menu stay the virtual cursor's (old-pc,
    /// 2026-10-18).
    #[default]
    Auto,
    /// Only Tab and Shift+Tab onto a field turn forms mode on ("only when you move to a form
    /// control using Tab or Shift+Tab", What's New in JAWS 2018): Tab reads as with Auto
    /// [obs, 714 against 287].
    SemiAuto,
    /// Enter turns forms mode on: old-pc's `AutoFormsMode=0`.
    Manual,
}

impl FormsMode {
    /// The mode a `DEFAULT.JCF` `AutoFormsMode` value sets: 0 manual, 2 semi-auto [obs,
    /// 714]; any other value is auto, JAWS's default.
    pub fn from_jcf(value: &str) -> Self {
        match value.trim() {
            "0" => Self::Manual,
            "2" => Self::SemiAuto,
            _ => Self::Auto,
        }
    }
}

/// How a heading's role is said: JAWS's `[HTML] HeadingIndication` (2 by default): its text
/// alone at 0, "heading" at 1, "heading level 1" at 2 [obs, 730, 731; 11].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadingIndication {
    Text,
    Heading,
    #[default]
    Level,
}

impl HeadingIndication {
    /// The indication a `DEFAULT.JCF` `HeadingIndication` value sets: 0 the text alone, 1
    /// "heading"; any other value the level, JAWS's default.
    pub fn from_jcf(value: &str) -> Self {
        match value.trim() {
            "0" => Self::Text,
            "1" => Self::Heading,
            _ => Self::Level,
        }
    }

    /// The role said for a heading of `level`, if any.
    pub fn role(self, level: u32) -> Option<String> {
        use crate::messages::msg;
        match self {
            Self::Text => None,
            Self::Heading => Some(msg::role_heading().to_string()),
            Self::Level => Some(msg::role_heading_level(level).render()),
        }
    }
}

/// What is read of the text an application writes to the screen: JAWS's Screen Echo
/// (`[Options] ScreenEcho`: 0 none, 1 highlighted, 2 all; 1 in its shipped `DEFAULT.JCF`).
/// Insert+S moves it on for the session [obs, 913/s1-s3].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenEcho {
    None,
    #[default]
    Highlighted,
    All,
}

impl ScreenEcho {
    /// The echo a `DEFAULT.JCF` `ScreenEcho` value sets: 0 none, 2 all; any other value
    /// highlighted text, JAWS's default.
    pub fn from_jcf(value: &str) -> Self {
        match value.trim() {
            "0" => Self::None,
            "2" => Self::All,
            _ => Self::Highlighted,
        }
    }

    /// The echo Insert+S moves to: from highlighted, all, none and highlighted again
    /// [obs, 913/s1-s3].
    pub fn next(self) -> Self {
        match self {
            Self::Highlighted => Self::All,
            Self::All => Self::None,
            Self::None => Self::Highlighted,
        }
    }

    /// The echo's name, as Insert+S says it [obs, 913/s1-s3].
    pub fn name(self) -> String {
        use crate::messages::msg;
        match self {
            Self::None => msg::screen_echo_none(),
            Self::Highlighted => msg::screen_echo_highlighted(),
            Self::All => msg::screen_echo_all(),
        }
        .to_string()
    }
}

/// What the virtual cursor's character and word keys treat as one unit: JAWS's Smart
/// Navigation (`[HTML] SmartNavigation`, a bit pattern its `DEFAULT.JCF` describes: 1 a
/// control, 2 a table cell; 0 by default).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartNavigation {
    #[default]
    Off,
    Controls,
    Tables,
    ControlsAndTables,
}

impl SmartNavigation {
    /// The navigation a `DEFAULT.JCF` `SmartNavigation` value sets; any value but 1, 2 and
    /// 3 is off, JAWS's default.
    pub fn from_jcf(value: &str) -> Self {
        match value.trim() {
            "1" => Self::Controls,
            "2" => Self::Tables,
            "3" => Self::ControlsAndTables,
            _ => Self::Off,
        }
    }

    /// What Insert+X turns it to: off, from off controls and tables [obs, 913/s5-s8].
    pub fn toggled(self) -> Self {
        match self {
            Self::Off => Self::ControlsAndTables,
            _ => Self::Off,
        }
    }

    /// What Insert+X says of it [obs, 913/s5, s6].
    pub fn name(self) -> String {
        use crate::messages::msg;
        match self {
            Self::Off => msg::smart_navigation_off(),
            Self::Controls => msg::smart_navigation_controls(),
            Self::Tables => msg::smart_navigation_tables(),
            Self::ControlsAndTables => msg::smart_navigation_controls_and_tables(),
        }
        .to_string()
    }
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
    /// (`okur-lab wait-silence`, 2026-10-17) [obs, 403/s1-s6, s29-s34].
    None,
}

impl TypingEcho {
    /// The echo a `DEFAULT.JCF` `TypingEcho` value sets: 0 none, 1 characters, 2 words, 3
    /// both [obs, 709-711: each value set in the file, "ab cd" typed]; any other value is
    /// characters, JAWS's default.
    pub fn from_jcf(value: &str) -> Self {
        match value.trim() {
            "0" => Self::None,
            "2" => Self::Words,
            "3" => Self::Both,
            _ => Self::Characters,
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

/// How running text is given to the synthesizer: JAWS's Text Processing, `DEFAULT.JCF`'s
/// Options `MixedCase`, `Numbers` and `SayDollars` [obs, 721-723 against 407].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextProcessing {
    /// A word is split where a capital follows a small letter ("get Element By Id"); off, it
    /// is given whole, while letters and digits are still split ("System 32") [obs, 721].
    pub mixed_case: bool,
    /// Every number digit by digit ("4 2"), its point, comma and colon by name ("2 point 5",
    /// "1 Comma 0 0 0", "1 0 Colon 3 0"), an ordinal whole ("1st") [obs, 722].
    pub numbers_as_digits: bool,
    /// Dollar and euro amounts go to the synthesizer as written ("$0.02", "€"), for it to say
    /// [obs, 723].
    pub say_dollars: bool,
}

impl Default for TextProcessing {
    fn default() -> Self {
        Self {
            mixed_case: true,
            numbers_as_digits: false,
            say_dollars: false,
        }
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
    /// A capital read alone or spelled is said at a raised pitch: JAWS's Caps Indicate
    /// (`IndicateCaps` in `DEFAULT.JCF`'s Options, on by default) [obs, 408, 717].
    pub indicate_caps: bool,
    /// The voice's pitch, -10 to 10 on SAPI's scale around the voice's own (0). Voice
    /// Adjustment sets it per voice context, as JAWS's shows Pitch for a context only [obs, the
    /// walk of Voice Adjustment's contexts].
    pub pitch: i32,
    /// How running text is given to the voice.
    pub text: TextProcessing,
    /// The voice contexts' own voices, where they differ from the one above (JAWS's All
    /// Contexts).
    pub contexts: VoiceContexts,
    /// The voice profile this voice was chosen from, by name (Select a Voice Profile); none,
    /// the synthesizer's own.
    pub profile: Option<String>,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            synthesizer: Synthesizer::Auto,
            voice: None,
            rate: 0,
            volume: 100,
            punctuation: Punctuation::default(),
            indicate_caps: true,
            pitch: 0,
            text: TextProcessing::default(),
            contexts: VoiceContexts::default(),
            profile: None,
        }
    }
}

impl VoiceSettings {
    /// This voice made `other`'s: its synthesizer, person, rate, volume, punctuation,
    /// contexts and profile; Caps Indicate and the text processing stay as they are.
    pub fn with_voice_of(&self, other: &VoiceSettings) -> VoiceSettings {
        VoiceSettings {
            synthesizer: other.synthesizer,
            voice: other.voice.clone(),
            rate: other.rate,
            volume: other.volume,
            punctuation: other.punctuation,
            pitch: other.pitch,
            contexts: other.contexts.clone(),
            profile: other.profile.clone(),
            ..self.clone()
        }
    }

    /// This voice made profile `name`'s, `profile`, as [`VoiceSettings::with_voice_of`].
    pub fn with_profile(&self, name: &str, profile: &VoiceSettings) -> VoiceSettings {
        VoiceSettings {
            profile: Some(name.to_owned()),
            ..self.with_voice_of(profile)
        }
    }

    /// The voice `context` speaks with: All Contexts' with the context's own parts.
    pub fn in_context(&self, context: VoiceContext) -> VoiceSettings {
        let own = self.contexts.get(context);
        VoiceSettings {
            voice: own.voice.clone().or_else(|| self.voice.clone()),
            rate: own.rate.unwrap_or(self.rate),
            punctuation: own.punctuation.unwrap_or(self.punctuation),
            pitch: own.pitch.unwrap_or(self.pitch),
            ..self.clone()
        }
    }
}

/// Who is speaking, as JAWS's voice contexts (Voice Adjustment's Adjust box) tell them apart:
/// each can have its own voice, so that the voice says which cursor is reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceContext {
    /// The focus, and what the PC cursor reads.
    PcCursor,
    /// What is read while the Okur cursor is active (JAWS's JAWS Cursor voice).
    OkurCursor,
    /// Typing echo.
    Keyboard,
    /// Tutor messages and Okur's own messages.
    Message,
}

impl VoiceContext {
    /// In Voice Adjustment's order, after All Contexts.
    pub const ALL: [VoiceContext; 4] = [
        VoiceContext::PcCursor,
        VoiceContext::OkurCursor,
        VoiceContext::Keyboard,
        VoiceContext::Message,
    ];
}

/// A voice context's own voice: each part it leaves out is All Contexts'. The volume is All
/// Contexts' alone, as Voice Adjustment shows it [obs, the walk of Voice Adjustment's contexts].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContextVoice {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punctuation: Option<Punctuation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch: Option<i32>,
}

/// The voice contexts' own voices.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceContexts {
    pub pc_cursor: ContextVoice,
    pub okur_cursor: ContextVoice,
    pub keyboard: ContextVoice,
    pub message: ContextVoice,
}

impl VoiceContexts {
    pub fn get(&self, context: VoiceContext) -> &ContextVoice {
        match context {
            VoiceContext::PcCursor => &self.pc_cursor,
            VoiceContext::OkurCursor => &self.okur_cursor,
            VoiceContext::Keyboard => &self.keyboard,
            VoiceContext::Message => &self.message,
        }
    }

    pub fn get_mut(&mut self, context: VoiceContext) -> &mut ContextVoice {
        match context {
            VoiceContext::PcCursor => &mut self.pc_cursor,
            VoiceContext::OkurCursor => &mut self.okur_cursor,
            VoiceContext::Keyboard => &mut self.keyboard,
            VoiceContext::Message => &mut self.message,
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
    /// The level a `DEFAULT.JCF` `Verbosity` value sets: 0 Beginner, 1 Intermediate, 2
    /// Advanced; any other value is Beginner.
    pub fn from_jcf(value: &str) -> Self {
        match value.trim() {
            "1" => Self::Intermediate,
            "2" => Self::Advanced,
            _ => Self::Beginner,
        }
    }

    /// The level's place in a per-level setting: Beginner first.
    pub fn index(self) -> usize {
        match self {
            Self::Beginner => 0,
            Self::Intermediate => 1,
            Self::Advanced => 2,
        }
    }
}

/// Whether an output type is spoken at a level, and where: JAWS's Access Key choices (Off,
/// Speak all, Speak Menus Only, Speak Dialogs Only) and Tutor Messages' (Turn Off, Announce
/// Menu and Control Help, Announce Custom Messages Only) [the walk of the Startup Wizard,
/// `docs/references/okur-settings-redesign.md`; documented as menus' and dialogs' access keys,
/// and the custom messages a trainer writes].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    Off,
    On,
    /// Access keys in menus only.
    MenusOnly,
    /// Access keys in dialogs only.
    DialogsOnly,
    /// Tutor messages written for the user only: Okur has none, so none is said.
    CustomOnly,
}

impl OutputMode {
    /// The Tutor Messages choices, in JAWS's order.
    pub const TUTOR: [OutputMode; 3] = [Self::Off, Self::On, Self::CustomOnly];
    /// The Access Key choices, in JAWS's order.
    pub const ACCESS_KEY: [OutputMode; 4] =
        [Self::Off, Self::On, Self::MenusOnly, Self::DialogsOnly];

    /// Whether the output is said, in the places its mode names.
    pub fn speaks(self) -> bool {
        matches!(self, Self::On | Self::MenusOnly | Self::DialogsOnly)
    }
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
    /// 599/s3, 609/s3, 608/s9]; "Pasted", "cut", "Undo" likewise. JAWS's "JAWS Message",
    /// which the settings file names in Okur's name (AGENTS.md §1.5).
    #[serde(rename = "okur_message")]
    OkurMessage,
    /// A control's type: "Edit", "radio button", "Button", "same page link"; off, the control
    /// is its name and value alone [obs, 737]. JAWS's "Control Type".
    ControlType,
    /// A control's states: "checked", "not checked", "Required" [obs, 738]. JAWS's "Item
    /// State".
    ItemState,
    /// A control's place among its siblings: "1 of 3" [obs, 739]. JAWS's "Position
    /// Information".
    Position,
}

impl OutputType {
    pub const ALL: [OutputType; 10] = [
        OutputType::AccessKey,
        OutputType::Hint,
        OutputType::ListPosition,
        OutputType::Description,
        OutputType::ControlGroupName,
        OutputType::Select,
        OutputType::OkurMessage,
        OutputType::ControlType,
        OutputType::ItemState,
        OutputType::Position,
    ];

    /// JAWS's key for the type in `DEFAULT.JCF` `[OutputModes]`, where it is known.
    pub fn jcf_key(self) -> Option<&'static str> {
        match self {
            Self::AccessKey => Some("ACCESS_KEY"),
            // The hints are JAWS's tutor messages [obs, 736].
            Self::Hint => Some("TUTOR"),
            Self::Description => None,
            Self::ListPosition => Some("ANNOUNCE_POSITION_AND_COUNT"),
            Self::ControlGroupName => Some("CONTROL_GROUP_NAME"),
            Self::Select => Some("SELECT"),
            Self::OkurMessage => Some("JAWS_MESSAGE"),
            Self::ControlType => Some("CONTROL_TYPE"),
            Self::ItemState => Some("ITEM_STATE"),
            Self::Position => Some("POSITION"),
        }
    }

    /// Its label in the settings dialog, as JAWS's verbosity items name it where the help does
    /// (its JAWS Messages are Okur's).
    pub fn label(self) -> crate::messages::StaticMessage {
        use crate::messages::msg;
        match self {
            Self::AccessKey => msg::settings_output_access_key(),
            Self::Hint => msg::settings_output_hint(),
            Self::ListPosition => msg::settings_output_list_position(),
            Self::Description => msg::settings_output_description(),
            Self::ControlGroupName => msg::settings_output_control_group_name(),
            Self::Select => msg::settings_output_select(),
            Self::OkurMessage => msg::settings_output_okur_message(),
            Self::ControlType => msg::settings_output_control_type(),
            Self::ItemState => msg::settings_output_item_state(),
            Self::Position => msg::settings_output_position(),
        }
    }

    /// The type's modes, Beginner first, where the user's settings name none: JAWS's shipped
    /// `DEFAULT.JCF` (`ANNOUNCE_POSITION_AND_COUNT=1|2|0`, `CONTROL_GROUP_NAME=1|2|0`,
    /// `SELECT=1|2|0`, `JAWS_MESSAGE=1|2|0`, `TUTOR=1|1|1`, `ACCESS_KEY=1|1|1`,
    /// `CONTROL_TYPE=1|2|2`, `ITEM_STATE=1|2|2`, `POSITION=1|2|2`; a 2 is on, its short form
    /// not modeled yet).
    pub fn default_modes(self) -> [OutputMode; 3] {
        match self {
            Self::AccessKey | Self::Hint | Self::ControlType | Self::ItemState | Self::Position => {
                [OutputMode::On; 3]
            }
            Self::ListPosition
            | Self::Description
            | Self::ControlGroupName
            | Self::Select
            | Self::OkurMessage => [OutputMode::On, OutputMode::On, OutputMode::Off],
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
            PartKind::Role => Some(Self::ControlType),
            PartKind::State => Some(Self::ItemState),
            PartKind::Position => Some(Self::Position),
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
        self.mode(output).speaks()
    }

    /// `output`'s mode at the current level.
    pub fn mode(&self, output: OutputType) -> OutputMode {
        self.mode_at(output, self.level.index())
    }

    /// `output`'s mode at the level of index `level`, Beginner's 0.
    pub fn mode_at(&self, output: OutputType, level: usize) -> OutputMode {
        self.output_modes
            .get(&output)
            .and_then(|modes| modes.get(level))
            .copied()
            .or_else(|| output.default_modes().get(level).copied())
            .unwrap_or(OutputMode::On)
    }

    /// Sets `output`'s mode at the level of index `level`, the others' kept.
    pub fn set_mode_at(&mut self, output: OutputType, level: usize, mode: OutputMode) {
        let mut modes = [0, 1, 2].map(|i| self.mode_at(output, i));
        if let Some(m) = modes.get_mut(level) {
            *m = mode;
        }
        self.set_modes(output, modes);
    }

    /// Which of `modes` `output` has at the current level: a dialog's radio button for it
    /// (Basics' and the Startup Wizard's Tutor Messages and Access Key).
    pub fn chosen(&self, output: OutputType, modes: &[OutputMode]) -> usize {
        let mode = self.mode(output);
        modes.iter().position(|m| *m == mode).unwrap_or(0)
    }

    /// `output` set to `modes`' choice `index` at every level, as such a radio button chooses.
    pub fn choose(&mut self, output: OutputType, modes: &[OutputMode], index: usize) {
        self.set_modes(
            output,
            [modes.get(index).copied().unwrap_or(OutputMode::Off); 3],
        );
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
    /// What Say All reads at a time: a line, a sentence or a paragraph.
    pub say_all_unit: SayAllUnit,
    /// Full Speech or less speech, as Okur starts; Insert+Space, S changes it for the session.
    pub speech_mode: SpeechMode,
    /// Less speech on demand or muted.
    pub less_speech: LessSpeech,
    pub viewer_display: ViewerDisplay,
    /// When forms mode turns on by itself.
    pub forms_mode: FormsMode,
    /// What is said and how much: a control's access key is said after it ("alt+Y") while
    /// the `access_key` output type is on, as JAWS's Access Key output (`[OutputModes]
    /// ACCESS_KEY`, on by default).
    pub verbosity: Verbosity,
    /// A link to a place on its page is a "same page link": JAWS's Identify Same Page Links
    /// (`[HTML] IdentifySamePageLinks`, on by default). Off, it is a "Link" [obs, 724].
    pub identify_same_page_links: bool,
    /// A page's graphics are read: JAWS's `[HTML] IncludeGraphics` (1 by default). At 0 they are
    /// left out: G finds none, "There are no Graphics on this page." [obs, 727].
    pub show_graphics: bool,
    /// How a heading's role is said.
    pub heading_indication: HeadingIndication,
    /// A page is read from its top once it loads, after its title and summary: JAWS's
    /// "Document and Web Pages Automatically Read When Loaded" (Settings Center, Web/HTML/PDFs,
    /// Reading; `[HTML] SayAllOnDocumentLoad`), on in JAWS 2025. Off, the title and the
    /// summary are said and the cursor waits at the top [obs, 712, 713].
    pub read_page_on_load: bool,
    /// Indentation is said: JAWS's Say Indented Characters (`[Options] Indentation`, 0 by
    /// default). A line at the margin is said with "Left Margin" when it is read as a line and
    /// as a Say All starts [obs, 729/s1, s7, s8, s10].
    pub indentation: bool,
    /// What is read of the text an application writes to the screen.
    pub screen_echo: ScreenEcho,
    /// A key bound to a command stops what is being read: JAWS's Reading Interrupt
    /// (`[Options] ReadingInterrupt`, 1 by default). Off, a Say All reads on through such a
    /// key, which is answered after what is being said when it only reports (Insert+T), and
    /// changes nothing otherwise (Down) [obs, 982, 983].
    pub reading_interrupt: bool,
    /// What the virtual cursor's character and word keys treat as one unit.
    pub smart_navigation: SmartNavigation,
    /// Insert+F4 asks before Okur exits, as JAWS's "Prompt to Confirm When Exiting JAWS"
    /// (Options > Basics), checked by default: "When cleared, JAWS immediately closes when you
    /// use any of the commands to exit the program including INSERT+F4" (JAWS 2025 help).
    pub confirm_exit: bool,
    pub mouse_echo: MouseEcho,
    /// The language Insert+Space, O recognizes the screen in, a BCP 47 tag ("tr-TR"): JAWS's
    /// Microsoft OCR Language (`[OCR] MicrosoftOcrLanguage`, a locale number, 1033 by
    /// default). Empty, Okur's own language. Windows' recognizer for the user's languages is
    /// the fallback where Windows has none for it.
    pub ocr_language: String,
    /// The voice profiles saved by name (Voice Adjustment's Save As), for all applications;
    /// the synthesizers' own are not kept here.
    pub voice_profiles: std::collections::BTreeMap<String, VoiceSettings>,
    /// The applications whose own settings are kept but not used (Manage Application Settings'
    /// check boxes cleared), by their settings names.
    pub disabled_applications: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            voice: VoiceSettings::default(),
            keyboard: KeyboardSettings::default(),
            live_region_append: LiveRegionAppend::default(),
            typing_echo: TypingEcho::default(),
            say_all_unit: SayAllUnit::default(),
            speech_mode: SpeechMode::default(),
            less_speech: LessSpeech::default(),
            viewer_display: ViewerDisplay::default(),
            forms_mode: FormsMode::Auto,
            verbosity: Verbosity::default(),
            identify_same_page_links: true,
            show_graphics: true,
            heading_indication: HeadingIndication::Level,
            read_page_on_load: true,
            indentation: false,
            screen_echo: ScreenEcho::default(),
            smart_navigation: SmartNavigation::default(),
            reading_interrupt: true,
            confirm_exit: true,
            mouse_echo: MouseEcho::default(),
            ocr_language: String::new(),
            voice_profiles: std::collections::BTreeMap::new(),
            disabled_applications: Vec::new(),
        }
    }
}

/// Whether Okur starts after this user signs in to Windows: JAWS's "Start JAWS after logon for
/// this user" (Always, Never, Using the All Users Setting), which overrides the machine's "for
/// all users" (`docs/references/startup-and-secure-screens-proposal.md` §5.5). Kept in the
/// user's registry, where the sign-in start reads it before any setting is loaded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UserStart {
    #[default]
    Always,
    Never,
    AllUsers,
}

impl UserStart {
    pub const ALL: [UserStart; 3] = [Self::Always, Self::Never, Self::AllUsers];

    /// The registry's word for it: `always`, `never`, `all-users`.
    pub fn word(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Never => "never",
            Self::AllUsers => "all-users",
        }
    }

    /// The choice a registry word names; none for another.
    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|start| start.word() == word.trim())
    }

    /// Whether a start at sign-in goes on: Always does, Never does not, and "all users"
    /// follows the machine's choice. No choice is JAWS's default, Always (the proposal's
    /// Run-key resolver).
    pub fn starts_after_sign_in(user: Option<Self>, all_users: bool) -> bool {
        match user.unwrap_or_default() {
            Self::Always => true,
            Self::Never => false,
            Self::AllUsers => all_users,
        }
    }
}

/// Where the user's choice of [`UserStart`] is kept: the user's registry on Windows.
pub trait StartOptions: Send {
    /// The user's choice; none when they made none.
    fn user_start(&self) -> Option<UserStart>;
    fn set_user_start(&self, start: UserStart);
}

/// Where changed settings are kept, so that they hold the next time Okur starts: the saved
/// layers (all applications, each application, each site), never the session's.
pub trait SettingsStore: Send {
    fn save(&self, layers: &crate::settings_layers::SettingsLayers);
    /// Last Changed Settings, kept so that it holds the next time Okur starts, as JAWS's.
    fn save_history(&self, history: &crate::ui::settings_tree::SavedHistory);
}

/// A store that keeps what it was given, for tests.
#[derive(Debug, Clone, Default)]
pub struct RecordingSettingsStore {
    saved: std::sync::Arc<std::sync::Mutex<Vec<crate::settings_layers::SettingsLayers>>>,
    history: std::sync::Arc<std::sync::Mutex<Option<crate::ui::settings_tree::SavedHistory>>>,
}

impl RecordingSettingsStore {
    /// The layers saved so far, emptied.
    pub fn take(&self) -> Vec<crate::settings_layers::SettingsLayers> {
        self.saved
            .lock()
            .map(|mut saved| std::mem::take(&mut *saved))
            .unwrap_or_default()
    }

    /// The history saved last, if any.
    pub fn history(&self) -> Option<crate::ui::settings_tree::SavedHistory> {
        self.history.lock().ok().and_then(|history| history.clone())
    }
}

impl SettingsStore for RecordingSettingsStore {
    fn save(&self, layers: &crate::settings_layers::SettingsLayers) {
        if let Ok(mut saved) = self.saved.lock() {
            saved.push(layers.clone());
        }
    }

    fn save_history(&self, history: &crate::ui::settings_tree::SavedHistory) {
        if let Ok(mut kept) = self.history.lock() {
            *kept = Some(history.clone());
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
            say_all_unit: SayAllUnit::Line,
            // Not in old-pc's user JCF: the shared one's, JAWS's defaults.
            speech_mode: SpeechMode::Full,
            less_speech: LessSpeech::OnDemand,
            viewer_display: ViewerDisplay::OnScreen,
            forms_mode: FormsMode::Manual,
            verbosity,
            // None is in old-pc's JCF: JAWS's defaults.
            identify_same_page_links: true,
            show_graphics: true,
            heading_indication: HeadingIndication::Level,
            read_page_on_load: true,
            indentation: false,
            screen_echo: ScreenEcho::default(),
            smart_navigation: SmartNavigation::default(),
            reading_interrupt: true,
            confirm_exit: true,
            // Not in old-pc's user JCF: the shared one's, JAWS's defaults.
            mouse_echo: MouseEcho::default(),
            ocr_language: String::new(),
            voice_profiles: std::collections::BTreeMap::new(),
            disabled_applications: Vec::new(),
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
        assert!(defaults.forms_mode == FormsMode::Auto && access_keys(&defaults));
        let reference = Settings::reference_machine();
        assert!(reference.forms_mode == FormsMode::Manual && !access_keys(&reference));
        let older: Result<Settings, _> = serde_json::from_str(r#"{"typing_echo":"none"}"#);
        assert!(older.is_ok_and(|s| s.forms_mode == FormsMode::Auto && access_keys(&s)));
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

    /// The settings file names Okur's settings in Okur's name (AGENTS.md §1.5): the Okur key,
    /// and JAWS's "JAWS Message" output type as Okur's.
    #[test]
    fn the_settings_file_says_okur_where_jaws_names_itself() {
        let mut settings = Settings::reference_machine();
        settings
            .verbosity
            .output_modes
            .insert(OutputType::OkurMessage, [OutputMode::On; 3]);
        let json = serde_json::to_string(&settings).unwrap_or_default();
        assert!(json.contains(r#""okur_key":"caps_lock""#), "{json}");
        assert!(
            json.contains(r#""okur_message":["on","on","on"]"#),
            "{json}"
        );
        assert!(!json.to_lowercase().contains("jaws"), "{json}");
        let back: Result<Settings, _> = serde_json::from_str(&json);
        assert!(back.is_ok_and(|s| s == settings));
    }

    /// `TypingEcho` in `DEFAULT.JCF`: 0 none, 1 characters, 2 words, 3 both [obs, 709-711];
    /// a value JAWS was not seen to write is its default, characters.
    #[test]
    fn typing_echo_reads_jaws_values() {
        let read: Vec<TypingEcho> = ["0", "1", "2", "3", "7"]
            .into_iter()
            .map(TypingEcho::from_jcf)
            .collect();
        assert_eq!(
            read,
            [
                TypingEcho::None,
                TypingEcho::Characters,
                TypingEcho::Words,
                TypingEcho::Both,
                TypingEcho::Characters
            ]
        );
    }

    #[test]
    fn desktop_layout_uses_insert_only() {
        let desktop = KeyboardSettings::default();
        assert!(desktop.is_okur_key(Key::Insert));
        assert!(!desktop.is_okur_key(Key::CapsLock));
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
        assert!(laptop.is_okur_key(Key::CapsLock));
        assert!(laptop.is_okur_key(Key::Insert));
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
            okur_key: OkurKey::CapsLock,
        };
        assert!(!desktop_caps.is_okur_key(Key::Insert));
        assert!(
            desktop_caps
                .resolve(&chord(Key::F(12), false, true))
                .modifiers
                .insert
        );
    }
}
