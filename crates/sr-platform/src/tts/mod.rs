pub mod no_speech;
pub mod onecore;
pub mod sapi5;

pub use no_speech::NoSpeechEngine;
pub use onecore::OneCoreEngine;
pub use sapi5::Sapi5Engine;

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

    /// Synthesizes and outputs speech.
    /// If `interrupt` is true, cancels any ongoing speech immediately before speaking.
    fn speak(&mut self, text: &str, interrupt: bool) -> Result<(), TtsError>;

    /// Speaks `text` with its pitch changed by `pitch` on SAPI's scale (-10 to 10): a capital
    /// letter read alone. Synthesizers without pitch control speak it unchanged.
    fn speak_pitched(&mut self, text: &str, pitch: i32, interrupt: bool) -> Result<(), TtsError> {
        let _ = pitch;
        self.speak(text, interrupt)
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

/// Available TTS engine drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsEngineType {
    Auto,
    NoSpeech,
    Sapi5,
    OneCore,
}

impl std::str::FromStr for TtsEngineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(TtsEngineType::Auto),
            "no-speech" | "nospeech" | "silent" | "silence" | "none" => Ok(TtsEngineType::NoSpeech),
            "sapi5" | "sapi" => Ok(TtsEngineType::Sapi5),
            "onecore" | "one-core" => Ok(TtsEngineType::OneCore),
            _ => Err(format!(
                "Unknown TTS engine '{}'. Available: auto, onecore, sapi5, no-speech",
                s
            )),
        }
    }
}

/// Synthesizer manager managing the active TTS engine and runtime fallback.
pub struct TtsManager {
    engine: Box<dyn TtsEngine>,
    speech_log: Option<SpeechLog>,
}

impl TtsManager {
    /// Creates a new TTS manager with the specified engine driver.
    pub fn new(engine_type: TtsEngineType) -> Result<Self, TtsError> {
        let engine: Box<dyn TtsEngine> = match engine_type {
            TtsEngineType::NoSpeech => Box::new(NoSpeechEngine::new()),
            TtsEngineType::Sapi5 => {
                let sapi = Sapi5Engine::new()?;
                Box::new(sapi)
            }
            TtsEngineType::OneCore => Box::new(OneCoreEngine::new()?),
            TtsEngineType::Auto => {
                // OneCore voices respond fastest; SAPI 5 where they are missing.
                #[cfg(windows)]
                {
                    match OneCoreEngine::new() {
                        Ok(onecore) => Box::new(onecore),
                        Err(e) => {
                            tracing::warn!("OneCore unavailable ({e}), trying SAPI 5");
                            match Sapi5Engine::new() {
                                Ok(sapi) => Box::new(sapi),
                                Err(e) => {
                                    tracing::warn!("SAPI 5 unavailable ({e}), no speech");
                                    Box::new(NoSpeechEngine::new())
                                }
                            }
                        }
                    }
                }
                #[cfg(not(windows))]
                {
                    Box::new(NoSpeechEngine::new())
                }
            }
        };

        Ok(Self {
            engine,
            speech_log: None,
        })
    }

    /// Records every utterance (and stop) to `log`, the screen reader's equivalent of
    /// JAWS speech history. Off unless explicitly enabled (e.g. `sr-cli run --speech-log`).
    pub fn with_speech_log(mut self, log: SpeechLog) -> Self {
        self.speech_log = Some(log);
        self
    }

    /// Speaks text using the active synthesizer.
    pub fn speak(&mut self, text: &str, interrupt: bool) -> Result<(), TtsError> {
        if let Some(log) = self.speech_log.as_mut() {
            log.record(SpeechLogKind::Speak, text, interrupt);
        }
        self.engine.speak(text, interrupt)
    }

    /// Immediately stops speech.
    pub fn stop(&mut self) -> Result<(), TtsError> {
        if let Some(log) = self.speech_log.as_mut() {
            log.record(SpeechLogKind::Stop, "", true);
        }
        self.engine.stop()
    }

    /// Pauses ongoing speech.
    pub fn pause(&mut self) -> Result<(), TtsError> {
        self.engine.pause()
    }

    /// Resumes paused speech.
    pub fn resume(&mut self) -> Result<(), TtsError> {
        self.engine.resume()
    }

    /// Sets speech rate (-10 to 10).
    pub fn set_rate(&mut self, rate: i32) -> Result<(), TtsError> {
        self.engine.set_rate(rate)
    }

    /// Gets current speech rate.
    pub fn get_rate(&self) -> i32 {
        self.engine.get_rate()
    }

    /// Sets volume (0 to 100).
    pub fn set_volume(&mut self, volume: u16) -> Result<(), TtsError> {
        self.engine.set_volume(volume)
    }

    /// Gets current volume (0 to 100).
    pub fn get_volume(&self) -> u16 {
        self.engine.get_volume()
    }

    /// Returns whether the synthesizer is currently outputting speech.
    pub fn is_speaking(&self) -> bool {
        self.engine.is_speaking()
    }

    /// Returns the name of the currently active TTS driver.
    pub fn engine_name(&self) -> &str {
        self.engine.name()
    }

    /// Returns available voices.
    pub fn get_voices(&self) -> Vec<VoiceInfo> {
        self.engine.get_voices()
    }

    /// Sets active voice by identifier.
    pub fn set_voice(&mut self, voice_id: &str) -> Result<(), TtsError> {
        self.engine.set_voice(voice_id)
    }

    /// Blocks until ongoing speech finishes rendering or until timeout.
    pub fn wait_until_done(&mut self, timeout_ms: u32) -> Result<(), TtsError> {
        self.engine.wait_until_done(timeout_ms)
    }

    /// Replaces the active engine.
    pub fn set_engine(&mut self, engine: Box<dyn TtsEngine>) {
        self.engine = engine;
    }
}

impl sr_core::speech::SpeechOutput for TtsManager {
    fn speak(&mut self, text: &str, interrupt: bool) {
        let _ = self.speak(text, interrupt);
    }

    /// The speech log keeps the character as JAWS's history would; the synthesizer says its
    /// name.
    fn speak_character(&mut self, text: &str, interrupt: bool) {
        if let Some(log) = self.speech_log.as_mut() {
            log.record(SpeechLogKind::Speak, text, interrupt);
        }
        let _ = self.engine.speak_pitched(
            &sr_core::pronounce::character(text),
            sr_core::pronounce::character_pitch(text),
            interrupt,
        );
    }

    fn stop(&mut self) {
        let _ = self.stop();
    }

    fn is_speaking(&self) -> bool {
        self.is_speaking()
    }
}

/// Kind of entry in the speech log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechLogKind {
    Speak,
    Stop,
}

/// One speech log entry. `text` is the exact engine output, including the
/// `\u{1d}`/`\u{1e}` role markers, so it can be compared against JAWS speech history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechLogEntry {
    pub t_ms: u128,
    pub kind: SpeechLogKind,
    pub interrupt: bool,
    pub text: String,
}

/// Append-only JSON Lines speech log, kept open for the lifetime of the manager.
#[derive(Debug)]
pub struct SpeechLog {
    file: std::fs::File,
}

impl SpeechLog {
    /// Opens (creating if needed) the log at `path` for appending.
    pub fn open(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self { file })
    }

    fn record(&mut self, kind: SpeechLogKind, text: &str, interrupt: bool) {
        use std::io::Write;
        let entry = SpeechLogEntry {
            t_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            kind,
            interrupt,
            text: text.to_owned(),
        };
        if let Ok(mut line) = serde_json::to_string(&entry) {
            line.push('\n');
            let _ = self.file.write_all(line.as_bytes());
        }
    }

    /// Reads every entry from the log at `path`, skipping malformed lines.
    pub fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<SpeechLogEntry>> {
        let content = std::fs::read_to_string(path)?;
        Ok(content
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_speech_engine_lifecycle() {
        let mut engine = NoSpeechEngine::new();
        assert_eq!(engine.name(), "No Speech");
        assert_eq!(engine.last_spoken(), None);

        // Speak utterance with JAWS control formatting tokens
        assert!(engine
            .speak(" \u{001d} heading level 1 \u{001e} System Overview", false)
            .is_ok());
        assert_eq!(
            engine.last_spoken(),
            Some("  heading level 1  System Overview")
        );
        assert_eq!(engine.history().len(), 1);

        // Second utterance with interruption
        assert!(engine.speak("Second Line", true).is_ok());
        assert_eq!(engine.last_spoken(), Some("Second Line"));
        assert_eq!(engine.history().len(), 2);

        // Rate and volume clamping
        assert!(engine.set_rate(15).is_ok());
        assert_eq!(engine.get_rate(), 10);
        assert!(engine.set_rate(-20).is_ok());
        assert_eq!(engine.get_rate(), -10);

        assert!(engine.set_volume(150).is_ok());
        assert_eq!(engine.get_volume(), 100);

        // Clear history
        engine.clear_history();
        assert_eq!(engine.history().len(), 0);
        assert_eq!(engine.last_spoken(), None);
    }

    #[test]
    fn test_tts_engine_type_parsing() {
        use std::str::FromStr;

        assert_eq!(TtsEngineType::from_str("auto"), Ok(TtsEngineType::Auto));
        assert_eq!(
            TtsEngineType::from_str("no-speech"),
            Ok(TtsEngineType::NoSpeech)
        );
        assert_eq!(
            TtsEngineType::from_str("silent"),
            Ok(TtsEngineType::NoSpeech)
        );
        assert_eq!(TtsEngineType::from_str("sapi5"), Ok(TtsEngineType::Sapi5));
        assert!(TtsEngineType::from_str("nonexistent").is_err());
    }

    #[test]
    fn test_tts_manager_with_no_speech() {
        let mgr = TtsManager::new(TtsEngineType::NoSpeech);
        assert!(mgr.is_ok(), "Manager creation should succeed");
        if let Ok(mut mgr) = mgr {
            assert_eq!(mgr.engine_name(), "No Speech");

            assert!(mgr.speak("Hello world", false).is_ok());
            assert!(mgr.stop().is_ok());
            assert!(mgr.pause().is_ok());
            assert!(mgr.resume().is_ok());
        }
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "live: speaks aloud through SAPI 5"]
    fn test_sapi5_engine_initialization_and_speech() {
        let _lock = crate::TEST_AUDIO_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let sapi = Sapi5Engine::new();
        assert!(sapi.is_ok(), "SAPI 5 should initialize on Windows");
        if let Ok(mut sapi) = sapi {
            assert_eq!(sapi.name(), "SAPI 5");

            // Speak a test sentence (asynchronous non-blocking)
            assert!(sapi
                .speak("Screen reader SAPI 5 test speech output.", true)
                .is_ok());

            // Immediate stop to test purging
            assert!(sapi.stop().is_ok());

            // Volume and rate
            assert!(sapi.set_rate(0).is_ok());
            assert!(sapi.set_volume(80).is_ok());
            assert_eq!(sapi.get_volume(), 80);
        }
    }
}
