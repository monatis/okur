//! Okur's start on Windows, around the neutral run ([`crate::run`]), as
//! `docs/references/startup-and-secure-screens-proposal.md`, §5.6 has it: one copy per desktop
//! (an automatic start steps aside for a copy already running, a manual start replaces it), a
//! control pipe through which it is asked to quit, the system's screen-reader flag raised while
//! it runs, last in Windows' shutdown order, and a hidden window that hears the session end.
//! The platform is Windows' driver, okur-windows.

use std::sync::Mutex;

use okur_runtime::host::{ExitReason, ExitSender, Exits, Platform};

use crate::startup::{self, StartFacts, StartKind};

/// The crates whose logs are the screen reader's, besides the runtime's and the engine's.
pub const LOG_TARGETS: &[&str] = &["okur_windows", "okur"];

/// What the process sets once, before anything else: DPI awareness.
pub fn init() {
    okur_windows::init();
}

/// The synthesizer an option names (auto, onecore, sapi5, no-speech).
pub fn synthesizer_named(name: &str) -> Result<okur_core::settings::Synthesizer, String> {
    okur_windows::synthesizer_named(name)
}

/// Windows' synthesizers, for `okur voices`.
pub fn synthesizers() -> std::sync::Arc<dyn okur_runtime::SynthesizerFactory> {
    std::sync::Arc::new(okur_windows::WindowsSynthesizers)
}

/// What Okur holds on Windows while it runs, let go as it stops.
pub struct System {
    /// Settings Center's start options, handed to the platform.
    start_options: Option<okur_windows::start_options::RegistryStartOptions>,
    instance_lock: Option<okur_windows::instance::InstanceLock>,
    control: Option<okur_windows::instance::ControlServer>,
    session_end: Option<okur_windows::process::EndSessionWatcher>,
    /// Told when the exit path is over, for the session's end to let Windows go on.
    ended: Option<std::sync::mpsc::Sender<()>>,
    screen_reader_flag: Option<okur_windows::process::ScreenReaderFlag>,
}

impl System {
    /// Whether this start goes on: the sign-in start only as the user, or the machine, chose;
    /// this desktop taken for the screen reader. None says why it ends, without a sound.
    pub fn start(start: StartKind, console: bool) -> Option<Self> {
        let start_options = okur_windows::start_options::RegistryStartOptions::system();
        if start == StartKind::SignIn && !start_options.starts_after_sign_in() {
            let why = startup::SilentExit::NotAfterSignIn;
            tracing::info!(reason = why.describe(), "not started");
            if console {
                eprintln!("Okur does not start: {}.", why.describe());
            }
            return None;
        }
        let instance_lock = match take_desktop(start) {
            Ok(lock) => lock,
            Err(why) => {
                if console {
                    eprintln!("Okur does not start: {}.", why.describe());
                }
                return None;
            }
        };
        okur_windows::process::shut_down_last();
        Some(Self {
            start_options: Some(start_options),
            instance_lock: Some(instance_lock),
            control: None,
            session_end: None,
            ended: None,
            screen_reader_flag: None,
        })
    }

    /// The control pipe, the session's end, the elevation note and the screen-reader flag,
    /// for the run whose exits are `exits`.
    pub fn serve(&mut self, exits: &Exits, console: bool) {
        // The control pipe: `quit` from the installer, `okur quit`, or a copy taking over.
        let session = okur_windows::desktop::session_id().unwrap_or(0);
        let desktop = okur_windows::desktop::thread_desktop().unwrap_or_default();
        let status = crate::control::Status::of_this_process(session, &desktop);
        self.control = serve_control(session, &desktop, &status, &exits.sender());
        // Windows ending the session: the window's thread waits, briefly, for the exit path,
        // since Windows may end the process once it returns.
        let (ended_tx, ended_rx) = std::sync::mpsc::channel::<()>();
        let ended_rx = Mutex::new(ended_rx);
        let session_exit = exits.sender();
        self.session_end = okur_windows::process::EndSessionWatcher::start(Box::new(move || {
            session_exit.send(ExitReason::EndSession);
            if let Ok(ended) = ended_rx.lock() {
                let _ = ended.recv_timeout(std::time::Duration::from_secs(3));
            }
        }));
        self.ended = Some(ended_tx);
        if self.session_end.is_none() {
            tracing::warn!("no window hears the session end");
        }
        // An application running as administrator shows nothing to an Okur below High
        // integrity (Windows Settings' Rename dialog); JAWS runs elevated, the installed Okur
        // with UI Access.
        if !okur_windows::elevation::reads_elevated_windows() {
            let note = "Okur is not running as administrator: windows of applications \
                        that are (Settings' \"Rename this PC\", an elevated terminal) cannot \
                        be read. Start Okur from an administrator terminal to read them.";
            if console {
                println!("[Okur] {note}");
            }
            tracing::warn!("{note}");
        }
        // Applications that adapt to a screen reader know one runs.
        self.screen_reader_flag = Some(okur_windows::process::ScreenReaderFlag::raise());
    }

    /// Windows' own: the low-level keyboard hook, WinEvents and UI Automation, OneCore's and
    /// SAPI 5's voices, okur-ui below UI Access when Okur has it, the application actions, Say
    /// System Time (Insert+F12) in the user's locale formats and a notification rule's sound.
    /// Windows' driver keeps no event trace of its own (`okur-lab trace` records one).
    pub fn platform(&mut self, _event_trace: Option<String>) -> Platform {
        Platform {
            keyboard_hook: Box::new(okur_windows::hook::windows::WindowsKeyboardHook::new()),
            event_source: Box::new(
                okur_windows::events::windows::WindowsAccessibilityEventListener::new(),
            ),
            synthesizers: synthesizers(),
            ui_spawner: Box::new(okur_windows::ui_spawner::WindowsUiSpawner::for_this_process()),
            actions: Some(Box::new(|events| {
                Box::new(okur_windows::actions::WindowsActions::start(Some(events)))
            })),
            clock: Some(Box::new(okur_windows::clock::WindowsClock)),
            sounds: Some(Box::new(okur_windows::sound::WindowsSounds)),
            start_options: self
                .start_options
                .take()
                .map(|options| Box::new(options) as Box<dyn okur_core::settings::StartOptions>),
        }
    }

    /// The flag lowered, the desktop let go, the session's end told the exit is over. The
    /// control pipe and the session's window stay until the process exits, which comes next.
    pub fn stop(&mut self) {
        drop(self.screen_reader_flag.take());
        if let Some(lock) = self.instance_lock.take() {
            lock.release();
        }
        if let Some(ended) = self.ended.take() {
            let _ = ended.send(());
        }
    }
}

/// Serves this desktop's control pipe. A copy this one replaced lets the desktop's lock go
/// moments before its process, and its pipe, are gone: the pipe is tried again for a second.
fn serve_control(
    session: u32,
    desktop: &str,
    status: &crate::control::Status,
    exit: &ExitSender,
) -> Option<okur_windows::instance::ControlServer> {
    let name = okur_windows::instance::pipe_name(session, desktop);
    let mut last_error = String::new();
    for _ in 0..10 {
        let (status, exit) = (status.clone(), exit.clone());
        match okur_windows::instance::ControlServer::start(
            &name,
            Box::new(move |line| {
                crate::control::answer(line, &status, &|| exit.send(ExitReason::ControlPipe))
            }),
        ) {
            Ok(server) => return Some(server),
            Err(e) => last_error = e,
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    tracing::warn!("no control pipe: {last_error}");
    None
}

/// Takes this desktop for the screen reader, or says why it ends without a sound.
fn take_desktop(
    start: StartKind,
) -> Result<okur_windows::instance::InstanceLock, startup::SilentExit> {
    let desktop = okur_windows::desktop::thread_desktop().unwrap_or_default();
    let facts = StartFacts {
        kind: start,
        both_shifts_held: okur_windows::process::both_shifts_held(
            &okur_windows::process::SystemKeyState,
        ),
        secure_desktop: okur_windows::desktop::is_secure_desktop(&desktop),
        system_account: okur_windows::elevation::current()
            .sid
            .is_some_and(|sid| sid == "S-1-5-18"),
    };
    let taken = startup::preflight(&facts).and_then(|()| {
        startup::take_desktop(
            start,
            &mut crate::control::SystemInstances::for_this_desktop(),
        )
    });
    if let Err(why) = &taken {
        tracing::info!(?start, %desktop, "Okur does not start: {}", why.describe());
    }
    taken
}
