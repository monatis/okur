pub mod bridge;
pub mod events;
pub mod hook;
pub mod tts;

pub use bridge::{PlatformBridge, PlatformError};
pub use events::{
    create_event_listener, AccessibilityEvent, AccessibilityEventListener, FocusEvent,
    LiveRegionEvent, Politeness,
};
pub use hook::{create_keyboard_hook, Key, KeyAction, KeyEvent, KeyboardHook, Modifiers};
pub use tts::{
    NoSpeechEngine, Sapi5Engine, TtsEngine, TtsEngineType, TtsError, TtsManager, VoiceInfo,
};

pub fn init() {
    tracing::info!("sr-platform initialized");
}
