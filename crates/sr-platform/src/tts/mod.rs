pub mod no_speech;
pub mod sapi5;

pub use no_speech::NoSpeechEngine;
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
}

impl std::str::FromStr for TtsEngineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(TtsEngineType::Auto),
            "no-speech" | "nospeech" | "silent" | "silence" | "none" => Ok(TtsEngineType::NoSpeech),
            "sapi5" | "sapi" => Ok(TtsEngineType::Sapi5),
            _ => Err(format!(
                "Unknown TTS engine '{}'. Available: auto, no-speech, sapi5",
                s
            )),
        }
    }
}

/// Synthesizer manager managing the active TTS engine and runtime fallback.
pub struct TtsManager {
    engine: Box<dyn TtsEngine>,
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
            TtsEngineType::Auto => {
                #[cfg(windows)]
                {
                    match Sapi5Engine::new() {
                        Ok(sapi) => Box::new(sapi),
                        Err(e) => {
                            tracing::warn!(
                                "Failed to initialize SAPI 5, falling back to NoSpeech: {}",
                                e
                            );
                            Box::new(NoSpeechEngine::new())
                        }
                    }
                }
                #[cfg(not(windows))]
                {
                    Box::new(NoSpeechEngine::new())
                }
            }
        };

        Ok(Self { engine })
    }

    /// Speaks text using the active synthesizer.
    pub fn speak(&mut self, text: &str, interrupt: bool) -> Result<(), TtsError> {
        log_speech_utterance(text, interrupt, self.engine.name());
        self.engine.speak(text, interrupt)
    }

    /// Immediately stops speech.
    pub fn stop(&mut self) -> Result<(), TtsError> {
        log_speech_utterance("[SPEECH_STOPPED]", false, self.engine.name());
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

    fn stop(&mut self) {
        let _ = self.stop();
    }

    fn is_speaking(&self) -> bool {
        self.is_speaking()
    }
}

/// Appends a speech utterance to the programmatic speech history log (`target/speech_history.log`).
pub fn log_speech_utterance(text: &str, interrupt: bool, engine_name: &str) {
    let clean = text.replace(['\u{001d}', '\u{001e}'], "");
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let line = format!(
        "[{} ms] [{}] [interrupt: {}] {}\n",
        millis,
        engine_name,
        interrupt,
        clean.trim()
    );

    let _ = std::fs::create_dir_all("target");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("target/speech_history.log")
    {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
}

/// Reads all recorded utterances from the programmatic speech history log.
pub fn read_speech_history() -> Vec<String> {
    if let Ok(content) = std::fs::read_to_string("target/speech_history.log") {
        content.lines().map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    }
}

/// Clears the programmatic speech history log.
pub fn clear_speech_history() {
    let _ = std::fs::remove_file("target/speech_history.log");
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
    fn test_sapi5_engine_initialization_and_speech() {
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
