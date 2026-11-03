//! What runs the screen reader on every system: the ports a platform's driver implements (its
//! keyboard hook, its accessibility events, its synthesizers), with the headless mocks that
//! stand in for them in tests (`docs/references/crate-structure-proposal.md`, §13.1, step 4).
//! No platform API is called here.

pub mod error;
pub mod events;
pub mod hook;
pub mod host;
pub mod logging;
pub mod settings_file;
pub mod speech;
pub mod synthesizer;
pub mod ui_host;

pub use error::RuntimeError;
pub use events::{EventSink, EventSource, MockEventSource};
pub use hook::{HookCallback, KeyboardHook, MockKeyboardHook};
pub use speech::{
    SilentSynthesizers, SpeechIdle, SpeechLog, SpeechLogEntry, SpeechLogKind, SpeechThread,
    TtsManager,
};
pub use synthesizer::{NoSpeechEngine, SynthesizerFactory, TtsEngine, TtsError, VoiceInfo};
