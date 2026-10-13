pub mod audio;
pub mod bridge;
pub mod events;
pub mod hook;
pub mod tts;

pub use audio::{AudioActivityReport, AudioEndpointMeter};

pub use bridge::{PlatformBridge, PlatformError};
pub use events::{
    create_event_listener, AccessibilityEvent, AccessibilityEventListener, FocusEvent,
    LiveRegionEvent, Politeness,
};
pub use hook::{create_keyboard_hook, Key, KeyAction, KeyEvent, KeyboardHook, Modifiers};
pub use tts::{
    clear_speech_history, log_speech_utterance, read_speech_history, NoSpeechEngine, Sapi5Engine,
    TtsEngine, TtsEngineType, TtsError, TtsManager, VoiceInfo,
};

pub fn init() {
    tracing::info!("sr-platform initialized");
}

#[cfg(test)]
pub(crate) static TEST_AUDIO_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
