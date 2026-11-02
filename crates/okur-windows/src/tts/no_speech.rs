use super::{TtsEngine, TtsError, VoiceInfo};

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
