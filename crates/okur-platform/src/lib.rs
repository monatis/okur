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
pub mod mouse_point;
pub mod notification_listener;
pub mod ocr;
pub mod privacy;
pub mod process;
pub mod registry;
pub mod screen_lines;
pub mod settings_file;
pub mod signature;
pub mod sound;
pub mod text_context;
pub mod top_windows;
pub mod tts;
pub mod ui_host;
pub mod uia_probe;
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

/// Waits for `thread` to end, at most `limit`; past it the thread is left to end with the
/// process. Stopping joined the keyboard hook's and the accessibility events' threads, and one
/// stuck in a read of a hung application held Okur from quitting (performance review
/// 2026-10-23, T9).
pub(crate) fn join_within(thread: std::thread::JoinHandle<()>, limit: std::time::Duration) {
    let until = std::time::Instant::now() + limit;
    while !thread.is_finished() {
        if std::time::Instant::now() >= until {
            tracing::warn!(thread = ?thread.thread().name(), "thread not ended in time: left");
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = thread.join();
}

#[cfg(test)]
mod join_tests {
    /// A thread that ends is joined; one that does not is left once the time is up.
    #[test]
    fn a_stuck_thread_is_left_after_the_limit() {
        let quick = std::thread::spawn(|| {});
        super::join_within(quick, std::time::Duration::from_secs(5));
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let stuck = std::thread::spawn(move || {
            let _ = rx.recv();
        });
        let started = std::time::Instant::now();
        super::join_within(stuck, std::time::Duration::from_millis(50));
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        drop(tx);
    }
}
