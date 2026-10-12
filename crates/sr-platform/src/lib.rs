pub mod bridge;
pub mod tts;

pub use bridge::{PlatformBridge, PlatformError};
pub use tts::{
    NoSpeechEngine, Sapi5Engine, TtsEngine, TtsEngineType, TtsError, TtsManager, VoiceInfo,
};

pub fn init() {
    tracing::info!("sr-platform initialized");
}
