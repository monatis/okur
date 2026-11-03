//! Okur's driver for Windows: its keyboard hook, its accessibility events and reads (UI
//! Automation, IAccessible2, MSAA), application actions, SAPI 5 and OneCore speech, OCR, the
//! Office object models and the installer's plumbing. It is built on Windows alone; elsewhere
//! the crate is empty, and the programs take it as a Windows-only dependency.
#![cfg(windows)]

pub mod actions;
pub mod bridge;
pub mod clock;
pub mod com;
pub mod desktop;
pub mod ease_of_access;
pub mod elevation;
pub mod event_capture;
pub mod events;
pub mod excel_om;
pub mod file_text;
pub mod hook;
pub mod ia2_capture;
pub mod instance;
pub mod key_input;
pub mod lowered_process;
pub mod mouse_point;
pub mod notification_listener;
pub mod ocr;
pub mod process;
pub mod registry;
pub mod screen_lines;
pub mod signature;
pub mod sound;
pub mod start_options;
pub mod text_context;
pub mod top_windows;
pub mod tts;
pub mod ui_spawner;
pub mod uia_capture;
pub mod virtual_desktop;
pub mod window_text;
pub mod word_om;

pub use bridge::{PlatformBridge, PlatformError};
pub use tts::{
    synthesizer_named, NoSpeechEngine, Sapi5Engine, TtsEngine, TtsError, VoiceInfo,
    WindowsSynthesizers,
};

/// Process-wide setup, before any thread reads the screen. On Windows the process is aware of
/// each monitor's DPI: positions and sizes are real pixels, as JAWS gives them ("PC Cursor
/// active | 1045, 544" for a button whose middle a DPI-unaware process read as 668, 348 at
/// 125% [obs, 516/s8]), and the mouse goes where the element is.
pub fn init() {
    use windows::Win32::UI::HiDpi::{
        GetAwarenessFromDpiAwarenessContext, GetThreadDpiAwarenessContext,
    };
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    // SAFETY: a process-wide setting with a constant argument, made at startup; it fails
    // harmlessly when the awareness is already set (the programs' manifests set it).
    let set = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    // SAFETY: reads of this thread's own awareness; no preconditions.
    let awareness = unsafe { GetAwarenessFromDpiAwarenessContext(GetThreadDpiAwarenessContext()) };
    // 2 is per-monitor: positions and sizes are real pixels.
    tracing::info!(set = set.is_ok(), awareness = awareness.0, "DPI awareness");
    tracing::info!("okur-windows initialized");
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
