//! A system's speech synthesizers, and the silent one that stands in for them.

use okur_core::settings::Synthesizer;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    #[error("Synthesis failed: {0}")]
    SynthesisFailed(String),
    #[error("Initialization failed: {0}")]
    InitFailed(String),
    #[error("Voice not found: {0}")]
    VoiceNotFound(String),
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}

/// Metadata description for an installed synthesizer voice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub language: Option<String>,
}

/// Common abstraction for Text-To-Speech synthesizers.
pub trait TtsEngine: Send + Sync {
    /// The human-readable name of the synthesizer (e.g., "SAPI 5", "No Speech").
    fn name(&self) -> &str;

    /// The voice the synthesizer started with, its system default, by id: a voice context
    /// without a voice of its own goes back to it. `None` where the synthesizer cannot tell,
    /// and it is then started anew.
    fn default_voice(&self) -> Option<String> {
        None
    }

    /// Synthesizes and outputs speech.
    /// If `interrupt` is true, cancels any ongoing speech immediately before speaking.
    fn speak(&mut self, text: &str, interrupt: bool) -> Result<(), TtsError>;

    /// Speaks `text` with its pitch changed by `pitch` on SAPI's scale (-10 to 10): a capital
    /// letter read alone. Synthesizers without pitch control speak it unchanged.
    fn speak_pitched(&mut self, text: &str, pitch: i32, interrupt: bool) -> Result<(), TtsError> {
        let _ = pitch;
        self.speak(text, interrupt)
    }

    /// Speaks one part of a continuous reading (Say All) behind what is speaking, at the voice
    /// context's `pitch`; [`TtsEngine::reached`] gives `mark` back once the synthesizer starts
    /// it.
    fn speak_marked(
        &mut self,
        text: &str,
        mark: u64,
        pitch: i32,
        interrupt: bool,
    ) -> Result<(), TtsError> {
        let _ = mark;
        self.speak_pitched(text, pitch, interrupt)
    }

    /// The mark of the latest part the synthesizer has started since the last call, for a
    /// synthesizer that [`TtsEngine::follows_marks`].
    fn reached(&mut self) -> Option<u64> {
        None
    }

    /// Whether [`TtsEngine::reached`] follows the parts as they are spoken. Otherwise a part
    /// counts as reached once it is handed over.
    fn follows_marks(&self) -> bool {
        false
    }

    /// Cancels/stops any ongoing speech immediately.
    fn stop(&mut self) -> Result<(), TtsError>;

    /// Pauses ongoing speech.
    fn pause(&mut self) -> Result<(), TtsError>;

    /// Resumes paused speech.
    fn resume(&mut self) -> Result<(), TtsError>;

    /// Sets speech rate (-10 to 10 for SAPI 5, or normalized scale).
    fn set_rate(&mut self, rate: i32) -> Result<(), TtsError>;

    /// Gets the current speech rate.
    fn get_rate(&self) -> i32;

    /// Sets volume (0 to 100).
    fn set_volume(&mut self, volume: u16) -> Result<(), TtsError>;

    /// Gets the current volume (0 to 100).
    fn get_volume(&self) -> u16;

    /// Returns whether the synthesizer is currently speaking audio.
    fn is_speaking(&self) -> bool;

    /// Retrieves available voices.
    fn get_voices(&self) -> Vec<VoiceInfo>;

    /// Sets active voice by ID or name.
    fn set_voice(&mut self, voice_id: &str) -> Result<(), TtsError>;

    /// Blocks until any ongoing speech is completely rendered or until timeout.
    fn wait_until_done(&mut self, timeout_ms: u32) -> Result<(), TtsError>;
}

/// Makes a system's synthesizers, on the thread that calls them: the speech thread.
pub trait SynthesizerFactory: Send + Sync {
    /// A synthesizer of `kind`; `Auto` is the system's first that starts.
    fn make(&self, kind: Synthesizer) -> Result<Box<dyn TtsEngine>, TtsError>;
}

/// A silent TTS driver matching JAWS "No Speech" and NVDA "Silence" driver.
///
/// This engine is cross-platform, produces no audio, and records spoken utterances
/// into an in-memory buffer. It isolates automated differential tests from speaker noise
/// and provides an inspection buffer for braille display synchronization or testing.
#[derive(Debug, Clone)]
pub struct NoSpeechEngine {
    rate: i32,
    volume: u16,
    is_paused: bool,
    history: Vec<String>,
    current_voice: String,
}

impl Default for NoSpeechEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl NoSpeechEngine {
    pub fn new() -> Self {
        Self {
            rate: 0,
            volume: 100,
            is_paused: false,
            history: Vec::new(),
            current_voice: "No Speech".to_string(),
        }
    }

    /// Returns the most recently spoken utterance, if any.
    pub fn last_spoken(&self) -> Option<&str> {
        self.history.last().map(|s| s.as_str())
    }

    /// Returns all spoken utterances since the engine was created or cleared.
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Clears the recorded speech history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

impl TtsEngine for NoSpeechEngine {
    fn name(&self) -> &str {
        "No Speech"
    }

    fn speak(&mut self, text: &str, interrupt: bool) -> Result<(), TtsError> {
        if interrupt {
            // Interruption clears current active output
        }
        // Clean speech text of internal JAWS delimiters (\u{001d}, \u{001e})
        let clean = text.replace(['\u{001d}', '\u{001e}'], "");
        self.history.push(clean);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), TtsError> {
        Ok(())
    }

    fn pause(&mut self) -> Result<(), TtsError> {
        self.is_paused = true;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), TtsError> {
        self.is_paused = false;
        Ok(())
    }

    fn set_rate(&mut self, rate: i32) -> Result<(), TtsError> {
        self.rate = rate.clamp(-10, 10);
        Ok(())
    }

    fn get_rate(&self) -> i32 {
        self.rate
    }

    fn set_volume(&mut self, volume: u16) -> Result<(), TtsError> {
        self.volume = volume.min(100);
        Ok(())
    }

    fn get_volume(&self) -> u16 {
        self.volume
    }

    fn is_speaking(&self) -> bool {
        false
    }

    fn get_voices(&self) -> Vec<VoiceInfo> {
        vec![VoiceInfo {
            id: "no-speech".to_string(),
            name: "No Speech (Silent Output)".to_string(),
            language: Some("en-US".to_string()),
        }]
    }

    fn set_voice(&mut self, voice_id: &str) -> Result<(), TtsError> {
        if voice_id == "no-speech" {
            self.current_voice = voice_id.to_string();
            Ok(())
        } else {
            Err(TtsError::VoiceNotFound(voice_id.to_string()))
        }
    }

    fn wait_until_done(&mut self, _timeout_ms: u32) -> Result<(), TtsError> {
        Ok(())
    }
}
