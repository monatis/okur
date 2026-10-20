pub mod actions;
pub mod audio;
pub mod bridge;
pub mod clock;
pub mod desktop;
pub mod ease_of_access;
pub mod elevation;
pub mod event_trace;
pub mod events;
pub mod harness;
pub mod hook;
pub mod ia2_snapshot;
pub mod instance;
#[cfg(windows)]
pub mod lowered_process;
pub mod privacy;
pub mod process;
pub mod registry;
pub mod screen_lines;
pub mod settings_file;
pub mod signature;
pub mod text_context;
pub mod top_windows;
pub mod tts;
pub mod ui_host;
pub mod uia_snapshot;
pub mod virtual_desktop;
pub mod window_text;

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

/// Process-wide setup, before any thread reads the screen. On Windows the process is aware of
/// each monitor's DPI: positions and sizes are real pixels, as JAWS gives them ("PC Cursor
/// active | 1045, 544" for a button whose middle a DPI-unaware process read as 668, 348 at
/// 125% [obs, 516/s8]), and the mouse goes where the element is.
pub fn init() {
    #[cfg(windows)]
    {
        use windows::Win32::UI::HiDpi::{
            GetAwarenessFromDpiAwarenessContext, GetThreadDpiAwarenessContext,
        };
        use windows::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        // SAFETY: a process-wide setting with a constant argument, made at startup; it fails
        // harmlessly when the awareness is already set (the programs' manifests set it).
        let set =
            unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        // SAFETY: reads of this thread's own awareness; no preconditions.
        let awareness =
            unsafe { GetAwarenessFromDpiAwarenessContext(GetThreadDpiAwarenessContext()) };
        // 2 is per-monitor: positions and sizes are real pixels.
        tracing::info!(set = set.is_ok(), awareness = awareness.0, "DPI awareness");
    }
    tracing::info!("okur-platform initialized");
}

#[cfg(test)]
pub(crate) static TEST_AUDIO_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
