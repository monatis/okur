//! Okur's start on Linux, around the neutral run ([`crate::run`]): Linux's driver, okur-linux,
//! is the platform (the X11 keyboard hook, the desktop's AT-SPI events, speech-dispatcher, and
//! Enter and Space on a page's elements). A terminate signal stops Okur as Ctrl+C does, so the
//! desktop's accessibility status is put back as it was (the Linux lab's runner ends its runs
//! so).

use okur_runtime::host::{ExitReason, Exits, Platform};

use crate::startup::StartKind;

/// The crates whose logs are the screen reader's, besides the runtime's and the engine's.
pub const LOG_TARGETS: &[&str] = &["okur_linux", "okur"];

/// What the process sets once, before anything else: nothing on Linux.
pub fn init() {}

/// The synthesizer an option names: auto (speech-dispatcher) or no-speech.
pub fn synthesizer_named(name: &str) -> Result<okur_core::settings::Synthesizer, String> {
    use okur_core::settings::Synthesizer;
    match name.to_lowercase().as_str() {
        "auto" | "speech-dispatcher" | "speechd" => Ok(Synthesizer::Auto),
        "no-speech" | "nospeech" | "silent" | "silence" | "none" => Ok(Synthesizer::NoSpeech),
        _ => Err(format!(
            "Unknown TTS engine '{name}'. Available: auto (speech-dispatcher), no-speech"
        )),
    }
}

/// Linux's synthesizers, for `okur voices`.
pub fn synthesizers() -> std::sync::Arc<dyn okur_runtime::SynthesizerFactory> {
    std::sync::Arc::new(okur_linux::speech::LinuxSynthesizers)
}

/// What Okur holds on Linux while it runs: nothing yet but the start's say.
pub struct System;

impl System {
    /// Every start goes on: one copy per desktop and the sign-in start are Windows' for now.
    pub fn start(_start: StartKind, _console: bool) -> Option<Self> {
        Some(Self)
    }

    /// A terminate signal, or its terminal closing (a hang-up), stops Okur as Ctrl+C does, so
    /// the desktop's accessibility status is put back.
    pub fn serve(&mut self, exits: &Exits, _console: bool) {
        use tokio::signal::unix::SignalKind;
        for kind in [SignalKind::terminate(), SignalKind::hangup()] {
            let exit = exits.sender();
            tokio::spawn(async move {
                match tokio::signal::unix::signal(kind) {
                    Ok(mut signal) => {
                        if signal.recv().await.is_some() {
                            exit.send(ExitReason::Interrupted);
                        }
                    }
                    Err(e) => tracing::warn!("a signal not heard: {e}"),
                }
            });
        }
    }

    /// okur-linux's platform; its event source writing what it read for each event to
    /// `event_trace` where one is given.
    pub fn platform(&mut self, event_trace: Option<String>) -> Platform {
        let event_source = match event_trace {
            Some(path) => okur_linux::events::AtspiEventSource::with_trace(path.into()),
            None => okur_linux::events::AtspiEventSource::new(),
        };
        let objects = event_source.objects();
        Platform {
            keyboard_hook: Box::new(okur_linux::keyboard::X11KeyboardHook::new()),
            event_source: Box::new(event_source),
            synthesizers: synthesizers(),
            ui_spawner: Box::new(okur_runtime::ui_host::PlainSpawner),
            // Enter and Space on a page's elements, and the focus given to them.
            actions: Some(Box::new(move |_sink| {
                Box::new(okur_linux::actions::LinuxActions::new(objects))
                    as Box<dyn okur_core::actions::ApplicationActions>
            })),
            clock: None,
            sounds: None,
            start_options: None,
        }
    }

    /// Nothing to let go.
    pub fn stop(&mut self) {}
}
