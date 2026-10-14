//! A SAPI 5 voice for the reference machine that records what a screen reader sends its
//! synthesizer.
//!
//! JAWS's speech history keeps text from before JAWS's own symbol and punctuation processing
//! (`$`), while the user hears the processed text (`dollar`). With this engine registered as
//! a SAPI 5 voice and chosen as JAWS's synthesizer on `old-pc`, every `Speak` call appends
//! one [`SpokenRecord`] (its text fragments as SAPI parsed them) to a JSON-lines log. The call
//! is then handed to a real voice (the token's `Delegate` value), so speech stays audible and
//! its timing realistic for the harness's silence detection.
//!
//! Registration (elevated, once): `skills/jaws-reference/scripts/install-speech-recorder.ps1`.

// COM's `DllGetClassObject` and `DllCanUnloadNow` exports draw LNK4104 ("should be PRIVATE"),
// which matters only for import libraries: COM loads this DLL, nothing links against it.
#![allow(linker_messages)]

use serde::{Deserialize, Serialize};

/// One `ISpTTSEngine::Speak` call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpokenRecord {
    /// Milliseconds since the Unix epoch when the call arrived.
    pub t: u64,
    pub frags: Vec<Fragment>,
}

/// One SAPI text fragment: a run of text with one voice state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    /// SAPI's action: `speak`, `silence`, `pronounce`, `bookmark`, `spell`, `section` or
    /// `unknown_tag`.
    pub action: String,
    pub text: String,
    /// Pitch adjustment (`SPVPITCH::MiddleAdj`, -10 to 10), e.g. for capital letters.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pitch: i32,
    /// Rate adjustment (-10 to 10).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rate: i32,
    /// Volume (0 to 100).
    pub volume: u32,
    /// Silence before the fragment, for `silence`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub silence_ms: u32,
}

fn is_zero<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

impl SpokenRecord {
    /// The text the synthesizer was asked to speak, fragments separated by spaces.
    pub fn text(&self) -> String {
        self.frags
            .iter()
            .filter(|f| matches!(f.action.as_str(), "speak" | "spell" | "pronounce"))
            .map(|f| f.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The SAPI action name for `SPVACTIONS` values.
pub fn action_name(action: i32) -> &'static str {
    match action {
        0 => "speak",
        1 => "silence",
        2 => "pronounce",
        3 => "bookmark",
        4 => "spell",
        5 => "section",
        6 => "unknown_tag",
        _ => "other",
    }
}

#[cfg(windows)]
mod engine;

#[cfg(windows)]
pub use engine::CLSID_RECORDER;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip_and_skip_neutral_adjustments() {
        let record = SpokenRecord {
            t: 1,
            frags: vec![
                Fragment {
                    action: "speak".into(),
                    text: "left paren".into(),
                    pitch: 0,
                    rate: 0,
                    volume: 100,
                    silence_ms: 0,
                },
                Fragment {
                    action: "speak".into(),
                    text: "B".into(),
                    pitch: 4,
                    rate: 0,
                    volume: 100,
                    silence_ms: 0,
                },
            ],
        };
        let line = serde_json::to_string(&record).unwrap_or_default();
        assert_eq!(
            line,
            r#"{"t":1,"frags":[{"action":"speak","text":"left paren","volume":100},{"action":"speak","text":"B","pitch":4,"volume":100}]}"#
        );
        let back: Option<SpokenRecord> = serde_json::from_str(&line).ok();
        assert_eq!(back.as_ref(), Some(&record));
        assert_eq!(record.text(), "left paren B");
    }

    #[test]
    fn sapi_actions_have_names() {
        assert_eq!(action_name(0), "speak");
        assert_eq!(action_name(4), "spell");
        assert_eq!(action_name(42), "other");
    }
}
