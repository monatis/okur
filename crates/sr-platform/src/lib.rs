pub mod actions;
pub mod audio;
pub mod bridge;
pub mod clock;
pub mod event_trace;
pub mod events;
pub mod hook;
pub mod ia2_snapshot;
pub mod text_context;
pub mod tts;
pub mod uia_snapshot;

pub use audio::{AudioActivityReport, AudioEndpointMeter, SilenceReport};

pub use bridge::{PlatformBridge, PlatformError};
pub use events::{
    create_event_listener, AccessibilityEvent, AccessibilityEventListener, FocusEvent,
    LiveRegionEvent, Politeness,
};
pub use hook::{create_keyboard_hook, Key, KeyAction, KeyEvent, KeyboardHook, Modifiers};
pub use tts::{
    NoSpeechEngine, Sapi5Engine, SpeechLog, SpeechLogEntry, SpeechLogKind, TtsEngine,
    TtsEngineType, TtsError, TtsManager, VoiceInfo,
};

pub fn init() {
    tracing::info!("sr-platform initialized");
}

#[cfg(test)]
pub(crate) static TEST_AUDIO_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
