//! The live screen reader: `okur run` and `okur-sr.exe` both run it.
//!
//! As a process it follows `docs/references/startup-and-secure-screens-proposal.md`, §5.6:
//! one copy per desktop (an automatic start steps aside for a copy already running, a manual
//! start replaces it), a control pipe through which it is asked to quit, the system's
//! screen-reader flag raised while it runs, last in Windows' shutdown order, and a hidden
//! window that hears the session end. Every way out (Ctrl+C in the console, the control pipe's
//! `quit`, the end of the session) goes down one exit path ([`ExitReason`]).

use okur_core::{
    AccessibilityEvent, AomNode, Role, ScreenReaderEngine, VirtualBuffer, VirtualCursor,
};
use okur_platform::{
    create_event_listener, create_keyboard_hook, KeyAction, PlatformBridge, SpeechLog,
    TtsEngineType, TtsManager,
};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use crate::startup::{self, StartFacts, StartKind};

/// `okur run`'s options that change a setting for the run.
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub engine: Option<String>,
    pub voice: Option<String>,
    pub rate: Option<i32>,
    pub volume: Option<u16>,
    pub keyboard_layout: Option<String>,
    pub okur_key: Option<String>,
    pub live_region_whole_text: bool,
    pub typing_echo: Option<String>,
    /// Settings by name, "id=value" ("typing_echo=words"), as the settings table names them.
    pub set: Vec<String>,
}

impl RunOptions {
    /// Changes `settings` as the options ask; an error names an option's bad value.
    pub fn apply(self, settings: &mut okur_core::settings::Settings) -> Result<(), String> {
        use okur_core::settings::registry::set_named;
        use okur_core::settings::{LiveRegionAppend, Synthesizer};
        if let Some(engine) = self.engine {
            settings.voice.synthesizer = match TtsEngineType::from_str(&engine)? {
                TtsEngineType::Auto => Synthesizer::Auto,
                TtsEngineType::OneCore => Synthesizer::OneCore,
                TtsEngineType::Sapi5 => Synthesizer::Sapi5,
                TtsEngineType::NoSpeech => Synthesizer::NoSpeech,
            };
        }
        if self.voice.is_some() {
            settings.voice.voice = self.voice;
        }
        if let Some(rate) = self.rate {
            settings.voice.rate = rate.clamp(-10, 10);
        }
        if let Some(volume) = self.volume {
            settings.voice.volume = volume.min(100);
        }
        if self.keyboard_layout.is_some() || self.okur_key.is_some() {
            let layout = match (&self.keyboard_layout, settings.keyboard.layout) {
                (Some(layout), _) => layout.clone(),
                (None, okur_core::settings::KeyboardLayout::Desktop) => "desktop".to_owned(),
                (None, okur_core::settings::KeyboardLayout::Laptop) => "laptop".to_owned(),
            };
            settings.keyboard = keyboard_settings(&layout, self.okur_key.as_deref())?;
        }
        if self.live_region_whole_text {
            settings.live_region_append = LiveRegionAppend::WholeText;
        }
        if let Some(echo) = self.typing_echo {
            set_named(settings, "typing_echo", &echo).map_err(|e| format!("--typing-echo: {e}"))?;
        }
        for named in self.set {
            let (id, value) = named
                .split_once('=')
                .ok_or_else(|| format!("--set takes id=value, not {named:?}"))?;
            set_named(settings, id.trim(), value).map_err(|e| format!("--set: {e}"))?;
        }
        Ok(())
    }
}

fn keyboard_settings(
    layout: &str,
    okur_key: Option<&str>,
) -> Result<okur_core::settings::KeyboardSettings, String> {
    use okur_core::settings::{KeyboardLayout, KeyboardSettings, OkurKey};
    let layout = match layout.to_ascii_lowercase().as_str() {
        "desktop" => KeyboardLayout::Desktop,
        "laptop" => KeyboardLayout::Laptop,
        other => {
            return Err(format!(
                "unknown keyboard layout {other:?} (desktop, laptop)"
            ))
        }
    };
    let mut settings = KeyboardSettings::for_layout(layout);
    if let Some(key) = okur_key {
        settings.okur_key = match key.to_ascii_lowercase().as_str() {
            "insert" => OkurKey::Insert,
            "capslock" => OkurKey::CapsLock,
            "scrolllock" => OkurKey::ScrollLock,
            other => {
                return Err(format!(
                    "unknown Okur key {other:?} (insert, capslock, scrolllock)"
                ))
            }
        };
    }
    Ok(settings)
}

/// What the live screen reader runs with.
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub options: RunOptions,
    /// A UIA snapshot to load instead of the active application.
    pub fixture: Option<String>,
    pub speech_log: Option<String>,
    /// The settings folder to read and save instead of the user's own.
    pub settings: Option<String>,
    pub start: StartKind,
    /// Print the banner and what is said (the console program).
    pub console: bool,
}

/// Why the screen reader stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    /// Ctrl+C in the console.
    Interrupted,
    /// Insert+F4, answered (or not asked, with `confirm_exit` off).
    Command,
    /// The control pipe's `quit`: the installer, `okur quit`, a copy taking over.
    ControlPipe,
    /// Windows is ending the session: out fast, nothing said.
    EndSession,
}

/// Logs to stderr at the RUST_LOG level, or, with `debug_log`, everything from the screen
/// reader's crates at debug level to that file.
pub fn init_logging(debug_log: Option<&str>) {
    if let Some(path) = debug_log {
        match std::fs::File::create(path) {
            Ok(file) => {
                let lines = spawn_log_writer(file);
                tracing_subscriber::fmt()
                    .with_writer(move || LogLines(lines.clone()))
                    .with_ansi(false)
                    .with_env_filter(tracing_subscriber::EnvFilter::new(
                        "okur_core=debug,okur_platform=debug,okur=debug",
                    ))
                    .init();
                return;
            }
            Err(e) => eprintln!("Cannot write debug log {path}: {e}"),
        }
    }
    tracing_subscriber::fmt::init();
}

/// Loads a page from a UIA snapshot (`okur-lab snapshot` output) through the production
/// UIA-to-AOM mapping.
pub fn load_aom_from_path(path: &Path) -> Result<AomNode, Box<dyn std::error::Error>> {
    let snapshot = okur_platform::uia_snapshot::UiaSnapshot::load(path)?;
    Ok(okur_platform::uia_snapshot::to_aom(&snapshot.root))
}

/// Engine work longer than this, on a key or an event, is logged as slow: the keyboard hook
/// waits on the engine, and a key that waits 150 ms passes to the application unread.
const SLOW_ENGINE: std::time::Duration = std::time::Duration::from_millis(20);

/// Pages of at least this many nodes are built again off the engine after a control on them
/// changed: 5,032 nodes took 15-17 ms to build, 25,032 nodes 114-155 ms (performance review
/// P3, 2026-10-20).
const PAGE_BUILDS_ELSEWHERE_FROM: usize = 5_000;

/// Page parts spliced into a page that changed meanwhile are spliced into the page as it is,
/// off the engine, this many times in all; after that under it.
const PARTS_ATTEMPTS: u32 = 3;

/// Page parts spliced and built off the engine.
struct BuiltParts {
    /// The page they were spliced into.
    base: std::sync::Arc<AomNode>,
    /// The page's buffer with them; `None` when a part was not found in the page.
    buffer: Option<VirtualBuffer>,
    parts: Vec<AomNode>,
    /// Which try this was, from 1.
    attempt: u32,
}

/// Splices `parts` into `base` and builds the buffer on the blocking pool, without the
/// engine, and sends the result to `built`.
fn splice_off_engine(
    base: std::sync::Arc<AomNode>,
    parts: Vec<AomNode>,
    attempt: u32,
    built: tokio::sync::mpsc::UnboundedSender<BuiltParts>,
) {
    tokio::task::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let buffer = VirtualBuffer::spliced(&base, &parts).map(VirtualBuffer::from_aom_owned);
        tracing::debug!(
            parts = parts.len(),
            attempt,
            lines = buffer.as_ref().map(|b| b.lines.len()),
            ms = started.elapsed().as_millis(),
            "page parts spliced"
        );
        let _ = built.send(BuiltParts {
            base,
            buffer,
            parts,
            attempt,
        });
    });
}

/// Keys waiting for the engine, which the event loop lets go first. A plain mutex takes no
/// turns: the loop took the engine again within microseconds for each queued event, and the
/// keyboard hook, trying every millisecond, could miss every gap until its 150 ms were over
/// and the key went to the application (responsiveness review, 2026-10-19).
static KEYS_WAITING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Takes `engine` for a key that arrived at `arrived`: at once, or as soon as the event loop
/// gives way, within 150 ms; `None` after that, and the key goes to the application.
fn take_for_key<T>(
    engine: &std::sync::Mutex<T>,
    arrived: std::time::Instant,
) -> Option<std::sync::MutexGuard<'_, T>> {
    use std::sync::TryLockError;
    let deadline = arrived + std::time::Duration::from_millis(150);
    let mut guard = engine.try_lock();
    if matches!(guard, Err(TryLockError::WouldBlock)) {
        KEYS_WAITING.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        while matches!(guard, Err(TryLockError::WouldBlock)) && std::time::Instant::now() < deadline
        {
            // A Windows sleep can last a timer tick (up to 15.6 ms): the first 2 ms only yield.
            if arrived.elapsed() < std::time::Duration::from_millis(2) {
                std::thread::yield_now();
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            guard = engine.try_lock();
        }
        KEYS_WAITING.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
    match guard {
        Ok(guard) => Some(guard),
        Err(TryLockError::Poisoned(poisoned)) => Some(recovered(engine, poisoned)),
        Err(TryLockError::WouldBlock) => None,
    }
}

/// The engine, also after a panic while it was held: a poisoned lock refused every later key
/// and event, and the reader went silent while it seemed to run (performance review
/// 2026-10-23, T8). The panic itself is logged where it happened ([`log_panics`]).
fn lock_engine<T>(engine: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    engine
        .lock()
        .unwrap_or_else(|poisoned| recovered(engine, poisoned))
}

/// A poisoned lock's guard, the lock cleared for the next taker.
fn recovered<'a, T>(
    engine: &'a Mutex<T>,
    poisoned: std::sync::PoisonError<std::sync::MutexGuard<'a, T>>,
) -> std::sync::MutexGuard<'a, T> {
    tracing::error!("the engine was held by a panic; it goes on");
    engine.clear_poison();
    poisoned.into_inner()
}

/// Logs each panic, with where it happened, before the default report (on stderr, which the
/// installed reader has none of).
fn log_panics() {
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(%info, "panic");
        report(info);
    }));
}

/// Runs `work`, a key's or an event's handling, logging a panic in it instead of letting it
/// end the hook (an abort, unwinding out of the hook's callback) or the event loop (the reader
/// deaf to events). The engine may be part-way through a change; it goes on.
fn unless_it_panics<R>(what: &'static str, work: impl FnOnce() -> R) -> Option<R> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
        Ok(result) => Some(result),
        Err(_) => {
            tracing::error!(what, "handling ended by a panic");
            None
        }
    }
}

/// Waits while a key waits for the engine: at most 200 ms, in case a key never takes it.
fn give_way_to_keys() {
    let until = std::time::Instant::now() + std::time::Duration::from_millis(200);
    while KEYS_WAITING.load(std::sync::atomic::Ordering::SeqCst) > 0
        && std::time::Instant::now() < until
    {
        std::thread::yield_now();
    }
}

/// A short name for an event, for logs (a page's tree is too big to print).
fn event_kind(event: Option<&AccessibilityEvent>) -> &'static str {
    match event {
        None => "idle",
        Some(AccessibilityEvent::Focus(_)) => "focus",
        Some(AccessibilityEvent::Menu(_)) => "menu",
        Some(AccessibilityEvent::Change { .. }) => "change",
        Some(AccessibilityEvent::TextContext(_)) => "text context",
        Some(AccessibilityEvent::ConsoleText(_)) => "console text",
        Some(AccessibilityEvent::Selected(_)) => "selected",
        Some(AccessibilityEvent::LiveRegion(_)) => "live region",
        Some(AccessibilityEvent::ViewerText(_)) => "viewer text",
        Some(AccessibilityEvent::ScreenText(_)) => "screen text",
        Some(AccessibilityEvent::Recognized(_)) => "recognized",
        Some(AccessibilityEvent::FileRecognized(_)) => "file recognized",
        Some(AccessibilityEvent::FileRecognitionStarted) => "file recognition started",
        Some(AccessibilityEvent::TopWindows(_)) => "top windows",
        Some(AccessibilityEvent::TrayIcons(_)) => "tray icons",
        Some(AccessibilityEvent::PageElements(_)) => "page elements",
        Some(AccessibilityEvent::DocumentHeadings(_)) => "document headings",
        Some(AccessibilityEvent::DocumentLinks(_)) => "document links",
        Some(AccessibilityEvent::LineSpacing(_)) => "line spacing",
        Some(AccessibilityEvent::WordKeyState(_)) => "word key state",
        Some(AccessibilityEvent::DocumentFields(_)) => "document fields",
        Some(AccessibilityEvent::DocumentNotes(..)) => "document notes",
        Some(AccessibilityEvent::DocumentParagraphs(_)) => "document paragraphs",
        Some(AccessibilityEvent::DocumentView(_)) => "document view",
        Some(AccessibilityEvent::DocumentLoaded { .. }) => "document loaded",
        Some(AccessibilityEvent::Ui(_)) => "okur window",
        Some(AccessibilityEvent::PageChanged) => "page changed",
        Some(AccessibilityEvent::PagePartsLoaded(_)) => "page parts loaded",
        Some(AccessibilityEvent::WindowRenamed(_)) => "window renamed",
        Some(AccessibilityEvent::WindowShown(_)) => "window shown",
        Some(AccessibilityEvent::MouseMoved { .. }) => "mouse moved",
        Some(AccessibilityEvent::Toast { .. }) => "toast",
        Some(AccessibilityEvent::AppNotification { .. }) => "application notification",
        Some(AccessibilityEvent::MousePoint(_)) => "mouse point",
    }
}

/// Prints utterances from a thread of its own: a console paused in QuickEdit selection
/// blocks writes, and the event loop must not wait on it.
fn spawn_console() -> std::sync::mpsc::Sender<String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let spawned = std::thread::Builder::new()
        .name("okur-console".to_owned())
        .spawn(move || {
            for line in rx {
                println!("{line}");
            }
        });
    if let Err(e) = spawned {
        tracing::warn!("no console thread, utterances are not printed: {e}");
    }
    tx
}

/// Debug-log output, handed to the writer thread ([`spawn_log_writer`]).
struct LogLines(std::sync::mpsc::Sender<Vec<u8>>);

impl std::io::Write for LogLines {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = self.0.send(buf.to_vec());
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Writes the debug log from a thread of its own: the keyboard hook and the event loop log
/// on every key and event, and must not wait on the disk.
fn spawn_log_writer(file: std::fs::File) -> std::sync::mpsc::Sender<Vec<u8>> {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let spawned = std::thread::Builder::new()
        .name("okur-debug-log".to_owned())
        .spawn(move || {
            use std::io::Write;
            let mut out = std::io::BufWriter::new(file);
            while let Ok(first) = rx.recv() {
                let _ = out.write_all(&first);
                for more in rx.try_iter() {
                    let _ = out.write_all(&more);
                }
                // Flushed whenever the queue empties, so the log is current on disk.
                let _ = out.flush();
            }
        });
    if let Err(e) = spawned {
        eprintln!("No debug-log thread, the debug log stays empty: {e}");
    }
    tx
}

/// Prints a line in the console program; logs it either way.
fn say_to_console(console: bool, line: &str) {
    if console {
        println!("{line}");
    }
    tracing::info!("{line}");
}

/// Ends the process after an error before the screen reader ran.
fn fail(console: bool, message: &str) -> ! {
    if console {
        eprintln!("{message}");
    }
    tracing::error!("{message}");
    std::process::exit(1)
}

/// Serves this desktop's control pipe. A copy this one replaced lets the desktop's lock go
/// moments before its process, and its pipe, are gone: the pipe is tried again for a second.
fn serve_control(
    session: u32,
    desktop: &str,
    status: &crate::control::Status,
    exit: &tokio::sync::mpsc::UnboundedSender<ExitReason>,
) -> Option<okur_platform::instance::ControlServer> {
    let name = okur_platform::instance::pipe_name(session, desktop);
    let mut last_error = String::new();
    for _ in 0..10 {
        let (status, exit) = (status.clone(), exit.clone());
        match okur_platform::instance::ControlServer::start(
            &name,
            Box::new(move |line| {
                crate::control::answer(line, &status, &|| {
                    let _ = exit.send(ExitReason::ControlPipe);
                })
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
) -> Result<okur_platform::instance::InstanceLock, startup::SilentExit> {
    let desktop = okur_platform::desktop::thread_desktop().unwrap_or_default();
    let facts = StartFacts {
        kind: start,
        both_shifts_held: okur_platform::process::both_shifts_held(
            &okur_platform::process::SystemKeyState,
        ),
        secure_desktop: okur_platform::desktop::is_secure_desktop(&desktop),
        system_account: okur_platform::elevation::current()
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

/// Runs the screen reader until it is asked to stop, then exits the process.
pub async fn run(config: RunConfig) {
    let RunConfig {
        options,
        fixture,
        speech_log,
        settings: settings_folder,
        start,
        console,
    } = config;
    log_panics();
    // The sign-in start goes on only as the user, or the machine, chose.
    let start_options = okur_platform::start_options::RegistryStartOptions::system();
    if start == crate::startup::StartKind::SignIn && !start_options.starts_after_sign_in() {
        let why = crate::startup::SilentExit::NotAfterSignIn;
        tracing::info!(reason = why.describe(), "not started");
        if console {
            eprintln!("Okur does not start: {}.", why.describe());
        }
        return;
    }
    let instance_lock = match take_desktop(start) {
        Ok(lock) => lock,
        Err(why) => {
            if console {
                eprintln!("Okur does not start: {}.", why.describe());
            }
            return;
        }
    };
    okur_platform::process::shut_down_last();

    // The first start for this user, before Okur has a settings folder of theirs, opens the
    // Startup Wizard, as JAWS's runs once it is installed; the folder is made then, so the
    // wizard opens once. A folder given with --settings-folder (the lab's runs) never opens it.
    let first_start = settings_folder.is_none()
        && okur_platform::settings_file::SettingsFolder::user()
            .is_some_and(|folder| !folder.folder().exists());
    // The saved settings, then this run's options over them, for this run only.
    let settings_folder = match settings_folder {
        Some(path) => Some(okur_platform::settings_file::SettingsFolder::at(path)),
        None => okur_platform::settings_file::SettingsFolder::user(),
    };
    let mut layers = settings_folder
        .as_ref()
        .map(|folder| folder.load())
        .unwrap_or_default();
    let saved = layers.resolve(None, None);
    let mut settings = saved.clone();
    if let Err(e) = options.apply(&mut settings) {
        fail(console, &format!("Error: {e}"));
    }
    layers.change(
        &okur_core::settings_layers::Scope::Session,
        &saved,
        &settings,
        None,
    );
    let engine_type = TtsEngineType::from(settings.voice.synthesizer);

    let mut tts = TtsManager::new(engine_type).unwrap_or_else(|e| {
        fail(console, &format!("Failed to initialize TTS engine: {e}"));
    });
    if let Some(ref path) = speech_log {
        match SpeechLog::open(path) {
            Ok(log) => tts = tts.with_speech_log(log),
            Err(e) => fail(console, &format!("Failed to open speech log {path:?}: {e}")),
        }
    }

    okur_core::speech::SpeechOutput::configure(&mut tts, &settings.voice);

    if console {
        println!("============================================================");
        println!("  Okur, an open screen reader");
        println!("  TTS Driver: {}", tts.engine_name());
        println!("============================================================");
    }

    // Determine active buffer
    let (buffer, is_fixture) = if let Some(ref f) = fixture {
        let fixture_path = Path::new(f);
        match load_aom_from_path(fixture_path) {
            Ok(aom) => {
                say_to_console(
                    console,
                    &format!("Loaded document fixture from {fixture_path:?}"),
                );
                (VirtualBuffer::from_aom(&aom), true)
            }
            Err(e) => fail(console, &format!("Failed to load fixture: {e}")),
        }
    } else if cfg!(windows) {
        // Live, the event listener starts with the focus as it is (a page is loaded
        // when that focus is in one), as any later focus change.
        let empty_doc = AomNode::new(0, Role::Document);
        (VirtualBuffer::from_aom(&empty_doc), false)
    } else {
        match PlatformBridge::new().get_active_document(None) {
            Ok(aom) => {
                say_to_console(
                    console,
                    "Attached to live active foreground document via OS accessibility APIs.",
                );
                (VirtualBuffer::from_aom(&aom), true)
            }
            Err(_) => {
                say_to_console(console, "Operating in live native navigation mode.");
                let empty_doc = AomNode::new(0, Role::Document);
                (VirtualBuffer::from_aom(&empty_doc), false)
            }
        }
    };

    let mut cursor = VirtualCursor::new(buffer);
    cursor.move_to_start();

    // OS accessibility events and caret text both reach the engine through this channel.
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    // Speech on its own thread: the keyboard hook never waits for a synthesizer.
    let speech = okur_platform::tts::SpeechThread::start(tts);
    // What tells the exit path that Okur's last words were said.
    let speech_idle = speech.idle();
    // Say All's progress: the mark of each part the synthesizer starts comes back to
    // the event loop, which moves the virtual cursor to its line.
    let (marks_tx, mut marks_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();
    speech.report_marks(Box::new(move |mark| {
        let _ = marks_tx.send(mark);
    }));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech));
    // Enter on an edit focuses it in the page, Enter/Space on a control activates it.
    #[cfg(windows)]
    engine.set_actions(Box::new(okur_platform::actions::WindowsActions::start(
        Some(event_tx.clone()),
    )));
    // Okur's own windows (the Virtual Viewer on screen), drawn by okur-ui.
    let mut ui_closer = None;
    if let Some(ui) = okur_platform::ui_host::ProcessUiHost::start(event_tx.clone()) {
        ui_closer = Some(ui.closer());
        engine.set_ui(Box::new(ui));
    }
    engine.set_settings_layers(layers);
    // A big page whose control changed is built again off the engine, which the keyboard
    // hook waits on (performance review P3); a smaller one at once, in a few milliseconds.
    engine.set_page_builds_elsewhere(PAGE_BUILDS_ELSEWHERE_FROM);
    // The settings dialog keeps what it changes in the settings folder, and so do custom
    // labels.
    if let Some(folder) = settings_folder {
        engine.set_custom_labels(folder.load_labels(), Some(Box::new(folder.clone())));
        engine.set_placemarkers(folder.load_placemarkers(), Some(Box::new(folder.clone())));
        engine.set_flexible_web_rules(
            folder.load_flexible_web_rules(),
            Some(Box::new(folder.clone())),
        );
        engine.set_notification_rules(
            folder.load_notification_rules(),
            Some(Box::new(folder.clone())),
        );
        engine.set_settings_history(&folder.load_settings_history());
        engine.set_start_options(Box::new(start_options));
        if first_start {
            if let Err(e) = std::fs::create_dir_all(folder.folder()) {
                tracing::warn!(folder = ?folder.folder(), error = %e, "settings folder not made");
            }
            engine.open_startup_wizard();
        }
        engine.set_settings_store(Box::new(folder));
    }
    // Say System Time (Insert+F12) in the user's locale formats.
    #[cfg(windows)]
    engine.set_clock(Box::new(okur_platform::clock::WindowsClock));
    engine.set_sounds(Box::new(okur_platform::sound::WindowsSounds));
    // Every way out comes here: the exit path below takes it from the first.
    let (exit_tx, mut exit_rx) = tokio::sync::mpsc::unbounded_channel::<ExitReason>();
    // Insert+F4: the engine asks; this host ends Okur.
    let command_exit = exit_tx.clone();
    engine.set_exit(Box::new(move || {
        let _ = command_exit.send(ExitReason::Command);
    }));
    let engine = Arc::new(Mutex::new(engine));

    // The control pipe: `quit` from the installer, `okur quit`, or a copy taking over.
    let session = okur_platform::desktop::session_id().unwrap_or(0);
    let desktop = okur_platform::desktop::thread_desktop().unwrap_or_default();
    let status = crate::control::Status::of_this_process(session, &desktop);
    let _control = serve_control(session, &desktop, &status, &exit_tx);
    // Windows ending the session: the window's thread waits, briefly, for the exit path,
    // since Windows may end the process once it returns.
    let (ended_tx, ended_rx) = std::sync::mpsc::channel::<()>();
    let ended_rx = Mutex::new(ended_rx);
    let session_exit = exit_tx.clone();
    let session_end = okur_platform::process::EndSessionWatcher::start(Box::new(move || {
        let _ = session_exit.send(ExitReason::EndSession);
        if let Ok(ended) = ended_rx.lock() {
            let _ = ended.recv_timeout(std::time::Duration::from_secs(3));
        }
    }));
    if session_end.is_none() {
        tracing::warn!("no window hears the session end");
    }

    // Announce startup
    {
        let mut eng = lock_engine(&engine);
        if is_fixture {
            eng.app_mode = okur_core::engine::AppMode::VirtualBuffer;
            let doc_name = eng.cursor.buffer.document_title().unwrap_or("Document");
            let welcome = format!("{}  \u{001d} Virtual PC Cursor \u{001e}", doc_name);
            say_to_console(
                console,
                &format!("[Okur] {}", welcome.replace(['\u{001d}', '\u{001e}'], "")),
            );
            eng.speech.speak(&welcome, true);
        } else {
            eng.app_mode = okur_core::engine::AppMode::NativeControl;
            say_to_console(
                console,
                "[Okur] Ready in live native Windows navigation mode.",
            );
        }
    }
    // An application running as administrator shows nothing to an Okur below High integrity
    // (Windows Settings' Rename dialog); JAWS runs elevated, the installed Okur with UI Access.
    if !okur_platform::elevation::reads_elevated_windows() {
        let note = "Okur is not running as administrator: windows of applications \
                    that are (Settings' \"Rename this PC\", an elevated terminal) cannot \
                    be read. Start Okur from an administrator terminal to read them.";
        if console {
            println!("[Okur] {note}");
        }
        tracing::warn!("{note}");
    }

    // Install low-level keyboard hook
    let mut hook = create_keyboard_hook();
    let engine_cb = Arc::clone(&engine);

    if let Err(e) = hook.start(Box::new(move |event| {
        // Windows silently removes a low-level keyboard hook that once exceeds
        // LowLevelHooksTimeout, so the hook never waits long for the engine: a key
        // that finds it busy for 150 ms passes to the application.
        let arrived = std::time::Instant::now();
        let guard = take_for_key(&engine_cb, arrived);
        if guard.is_none() {
            tracing::warn!(key = ?event.key, "engine busy: key passed through");
        }
        if let Some(mut eng) = guard {
            let waited = arrived.elapsed();
            let started = std::time::Instant::now();
            let action = unless_it_panics("key", || eng.handle_key_event(&event))
                .unwrap_or(KeyAction::PassThrough);
            let held = started.elapsed();
            if waited > SLOW_ENGINE || held > SLOW_ENGINE {
                tracing::warn!(
                    key = ?event.key,
                    waited_ms = waited.as_millis(),
                    held_ms = held.as_millis(),
                    "slow key handling"
                );
            }
            if event.is_down {
                tracing::debug!(
                    key = ?event.key,
                    text = ?event.text,
                    ctrl = event.modifiers.ctrl,
                    shift = event.modifiers.shift,
                    alt = event.modifiers.alt,
                    insert = event.modifiers.insert,
                    ?action,
                    mode = ?eng.app_mode,
                    forms = eng.forms_mode,
                    waited_us = waited.as_micros(),
                    held_us = held.as_micros(),
                    "key"
                );
            }
            action
        } else {
            KeyAction::PassThrough
        }
    })) {
        fail(console, &format!("Failed to install keyboard hook: {e}"));
    }
    // Applications that adapt to a screen reader know one runs.
    let screen_reader_flag = okur_platform::process::ScreenReaderFlag::raise();

    // Start OS-level accessibility event listener
    let mut event_listener = create_event_listener();
    let mut event_task = None;
    if let Err(e) = event_listener.start(event_tx) {
        tracing::warn!("Could not start OS accessibility event listener: {}", e);
    } else {
        let engine_events = Arc::clone(&engine);
        let console_lines = console.then(spawn_console);
        event_task = Some(tokio::spawn(async move {
            let idle = std::time::Duration::from_millis(150);
            // Since when no event but the mouse's has come: the mouse moving, up to 20 times
            // a second, put off the idle work (speech deferred until no event supersedes it,
            // a page to apply, live regions gone quiet) for as long as it moved (performance
            // review 2026-10-23, T6).
            let mut quiet_since = tokio::time::Instant::now();
            // Captures of the page already read are built on threads of their own
            // (an IA2 capture after the quick one, a refresh after changes: 69-275 ms
            // on a page of 25,000 nodes, during which typing echo and focus events
            // waited; responsiveness review, 2026-10-19). They come back numbered, and
            // one finished after a newer one is dropped.
            let (built_tx, mut built_rx) =
                tokio::sync::mpsc::unbounded_channel::<(u64, String, VirtualBuffer)>();
            let (mut refreshes_sent, mut refresh_applied) = (0_u64, 0_u64);
            // Page parts read again are spliced into the page and built on a thread of
            // their own too (135-175 ms on a page of 25,000 nodes, during which every
            // event waited; performance review, 2026-10-20, P8).
            let (parts_tx, mut parts_rx) = tokio::sync::mpsc::unbounded_channel::<BuiltParts>();
            // A page built again after one of its controls changed, with the change's
            // generation: installed unless another change came meanwhile.
            let (rebuilt_tx, mut rebuilt_rx) =
                tokio::sync::mpsc::unbounded_channel::<(u64, VirtualBuffer)>();
            loop {
                let received = tokio::select! {
                    biased;
                    Some(mark) = marks_rx.recv() => {
                        give_way_to_keys();
                        {
                            let mut eng = lock_engine(&engine_events);
                            eng.speech_reached(mark);
                        }
                        continue;
                    }
                    Some((number, title, buffer)) = built_rx.recv() => {
                        if number > refresh_applied {
                            refresh_applied = number;
                            give_way_to_keys();
                            {
                                let mut eng = lock_engine(&engine_events);
                                eng.handle_page_refreshed(&title, buffer);
                            }
                        }
                        continue;
                    }
                    Some((generation, buffer)) = rebuilt_rx.recv() => {
                        give_way_to_keys();
                        {
                            let mut eng = lock_engine(&engine_events);
                            eng.install_page_build(generation, buffer);
                        }
                        continue;
                    }
                    // Installed if the page is still the one they were spliced into;
                    // else the parts go into the page as it is now, under the lock.
                    Some(BuiltParts { base, buffer, parts, attempt }) = parts_rx.recv() => {
                        give_way_to_keys();
                        let mut eng = lock_engine(&engine_events);
                        let taken = match buffer {
                            Some(buffer) => eng.install_page_parts(&base, buffer),
                            None => eng.page_parts_not_found(&base),
                        };
                        if taken {
                            continue;
                        }
                        // The page is another by now: the parts go into it as it is, off
                        // the engine again (performance review P3 step 3); after a few
                        // tries, under it.
                        if attempt >= PARTS_ATTEMPTS {
                            let _ = eng.handle_accessibility_event(
                                &AccessibilityEvent::PagePartsLoaded(parts),
                            );
                            continue;
                        }
                        let base = eng.page_for_parts();
                        drop(eng);
                        if let Some(base) = base {
                            splice_off_engine(base, parts, attempt + 1, parts_tx.clone());
                        }
                        continue;
                    }
                    received = tokio::time::timeout_at(quiet_since + idle, event_rx.recv()) => {
                        received
                    }
                };
                let event = match received {
                    Ok(Some(event)) => Some(event),
                    Ok(None) => break,
                    // Quiet for a moment: speech JAWS defers until no event supersedes it.
                    Err(_) => None,
                };
                if !matches!(event, Some(AccessibilityEvent::MouseMoved { .. })) {
                    quiet_since = tokio::time::Instant::now();
                }
                // A page's buffer is built before the engine is taken: on a big page
                // that takes long, and the keyboard hook waits on the engine. The
                // buffer keeps the page the event brought, without a copy.
                let mut event = event;
                let page = match &mut event {
                    Some(AccessibilityEvent::DocumentLoaded { title, root }) => {
                        let url = root.attributes.get("url").cloned();
                        give_way_to_keys();
                        let (refresh, marks) = {
                            let eng = lock_engine(&engine_events);
                            (
                                eng.is_page_in_front(title, url.as_deref()),
                                eng.reading_marks(),
                            )
                        };
                        let mut root = std::mem::replace(
                            root,
                            okur_core::AomNode::new(0, okur_core::Role::Document),
                        );
                        // Built as the reading choices read it, so the engine need not build
                        // it again.
                        marks.mark(&mut root);
                        if refresh {
                            refreshes_sent += 1;
                            let (number, title) = (refreshes_sent, std::mem::take(title));
                            let built = built_tx.clone();
                            tokio::task::spawn_blocking(move || {
                                let started = std::time::Instant::now();
                                let buffer = VirtualBuffer::from_aom_owned(root);
                                tracing::debug!(
                                    lines = buffer.lines.len(),
                                    ms = started.elapsed().as_millis(),
                                    "page refresh built"
                                );
                                let _ = built.send((number, title, buffer));
                            });
                            continue;
                        }
                        let started = std::time::Instant::now();
                        let buffer = VirtualBuffer::from_aom_owned(root);
                        tracing::debug!(
                            lines = buffer.lines.len(),
                            ms = started.elapsed().as_millis(),
                            "page buffer built"
                        );
                        Some((std::mem::take(title), buffer))
                    }
                    _ => None,
                };
                // Page parts read again are spliced into the page and built on the
                // blocking pool, the engine taken only a moment to share its page.
                if let Some(AccessibilityEvent::PagePartsLoaded(parts)) = &mut event {
                    give_way_to_keys();
                    let base = lock_engine(&engine_events).page_for_parts();
                    if let Some(base) = base {
                        splice_off_engine(base, std::mem::take(parts), 1, parts_tx.clone());
                        continue;
                    }
                }
                let spoken = {
                    give_way_to_keys();
                    let mut eng = lock_engine(&engine_events);
                    let started = std::time::Instant::now();
                    let spoken =
                        unless_it_panics(event_kind(event.as_ref()), || match (&event, page) {
                            (_, Some((title, buffer))) => {
                                eng.handle_document_loaded(&title, buffer)
                            }
                            (Some(event), None) => eng.handle_accessibility_event(event),
                            (None, None) => eng.on_idle(),
                        })
                        .flatten();
                    let held = started.elapsed();
                    if held > SLOW_ENGINE {
                        tracing::warn!(
                            event = event_kind(event.as_ref()),
                            ms = held.as_millis(),
                            "slow event handling"
                        );
                    }
                    // Idle ticks come every 150 ms of quiet; only events are timed, the
                    // mouse's many moves aside.
                    if event.is_some()
                        && !matches!(event, Some(AccessibilityEvent::MouseMoved { .. }))
                    {
                        tracing::debug!(
                            event = event_kind(event.as_ref()),
                            us = held.as_micros(),
                            "event handled"
                        );
                    }
                    // A control the event changed: its page is built again off the engine.
                    // The page is copied first and let go, so that a change meanwhile does
                    // not copy it under the lock.
                    if let Some((generation, page)) = eng.take_page_build() {
                        let rebuilt = rebuilt_tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let started = std::time::Instant::now();
                            let root = okur_core::AomNode::clone(&page);
                            drop(page);
                            let buffer = VirtualBuffer::from_aom_owned(root);
                            tracing::debug!(
                                generation,
                                lines = buffer.lines.len(),
                                ms = started.elapsed().as_millis(),
                                "page built after a change"
                            );
                            let _ = rebuilt.send((generation, buffer));
                        });
                    }
                    spoken
                };
                // Copied for the log and the console only when one of them takes it
                // (performance review 2026-10-23, T19).
                let shown = console_lines.is_some() || tracing::enabled!(tracing::Level::DEBUG);
                if let Some(msg) = spoken.filter(|_| shown) {
                    // One line per utterance, so the debug log stays line-oriented.
                    let msg = msg
                        .replace(['\u{001d}', '\u{001e}'], "")
                        .replace('\n', " | ");
                    tracing::debug!(%msg, "spoken for event");
                    if let Some(console) = &console_lines {
                        let _ = console.send(format!("[Event Spoken] {msg}"));
                    }
                }
            }
        }));
    }

    if console {
        println!("\nOkur is running.");
        println!("Navigation shortcuts active:");
        println!("  - Arrow keys: Navigate lines / characters");
        println!("  - Ctrl+Left/Right: Navigate words");
        println!("  - Ctrl+Home / Ctrl+End: Document start / end");
        println!("  - H / Shift+H: Next / prior heading");
        println!("  - 1..6: Next heading by level");
        println!("  - U / Shift+U: Next / prior link");
        println!("  - T / Shift+T: Next / prior table");
        println!("  - Tab / Shift+Tab: Interactive controls");
        println!("  - Enter: Forms mode on edits");
        println!("  - Esc: Exit forms mode to Virtual PC Cursor");
        println!("  - Ctrl: Silence speech");
        println!("Press Ctrl+C to stop Okur.\n");
    }

    // Ctrl+C in the console, or a reason from the exit channel. A program without a console
    // never hears Ctrl+C.
    let interrupted = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };
    let reason = tokio::select! {
        () = interrupted => ExitReason::Interrupted,
        Some(reason) = exit_rx.recv() => reason,
    };
    tracing::info!(?reason, "Okur stops");
    if console {
        println!("\nShutting down Okur...");
    }

    // The keyboard first, so that no key is held back while Okur goes; then nothing more
    // is read or said but Okur's last words.
    let _ = hook.stop();
    let _ = event_listener.stop();
    if let Some(task) = event_task {
        task.abort();
    }
    {
        let mut eng = lock_engine(&engine);
        if reason == ExitReason::EndSession {
            eng.speech.stop();
        } else {
            let words = eng.exit_words();
            eng.speech.speak(&words, true);
        }
    }
    if reason != ExitReason::EndSession {
        let _ = speech_idle.wait(std::time::Duration::from_millis(1500));
    }
    if let Some(ui) = ui_closer {
        ui.close(std::time::Duration::from_millis(500));
    }
    drop(screen_reader_flag);
    instance_lock.release();
    let _ = ended_tx.send(());
    if console {
        println!("Okur stopped.");
    }
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    /// The engine left poisoned by a panic while it was held: the next key and the next event
    /// still take it, as they find it (performance review 2026-10-23, T8).
    #[test]
    fn the_engine_is_taken_after_a_panic_while_it_was_held() {
        let engine = Arc::new(Mutex::new(7_u32));
        let holder = Arc::clone(&engine);
        let _ = std::thread::spawn(move || {
            let _held = holder.lock();
            std::panic::resume_unwind(Box::new("a bug while the engine was held"));
        })
        .join();
        assert!(engine.is_poisoned());
        assert_eq!(take_for_key(&engine, Instant::now()).map(|e| *e), Some(7));
        assert_eq!(*lock_engine(&engine), 7);
        assert!(!engine.is_poisoned());
    }

    /// A panic in a key's or an event's handling is caught, and nothing comes of it.
    #[test]
    fn a_panic_in_handling_is_caught() {
        let handled = unless_it_panics("test", || -> u32 {
            std::panic::resume_unwind(Box::new("a bug in a handler"))
        });
        assert_eq!(handled, None);
        assert_eq!(unless_it_panics("test", || 3), Some(3));
    }

    /// A key takes the engine quickly while the event loop takes it again and again, as it does
    /// for a stream of events, each held 1 ms: the loop gives way to a waiting key.
    #[test]
    fn a_key_gets_the_engine_between_a_stream_of_events() {
        let engine = Arc::new(Mutex::new(0_u32));
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let events = {
            let (engine, stop) = (Arc::clone(&engine), Arc::clone(&stop));
            std::thread::spawn(move || {
                while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                    give_way_to_keys();
                    if let Ok(mut count) = engine.lock() {
                        *count += 1;
                        std::thread::sleep(Duration::from_millis(1));
                    }
                }
            })
        };
        std::thread::sleep(Duration::from_millis(20));
        let mut longest = Duration::ZERO;
        for _ in 0..20 {
            let arrived = Instant::now();
            let taken = take_for_key(&engine, arrived).is_some();
            assert!(taken, "the key went to the application");
            longest = longest.max(arrived.elapsed());
            std::thread::sleep(Duration::from_millis(5));
        }
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = events.join();
        assert!(longest < Duration::from_millis(100), "{longest:?}");
    }

    #[test]
    fn run_options_change_the_settings_or_name_a_bad_value() {
        let mut settings = okur_core::settings::Settings::default();
        let options = RunOptions {
            rate: Some(40),
            keyboard_layout: Some("laptop".into()),
            ..RunOptions::default()
        };
        assert_eq!(options.apply(&mut settings), Ok(()));
        assert_eq!(settings.voice.rate, 10);
        assert_eq!(
            settings.keyboard.layout,
            okur_core::settings::KeyboardLayout::Laptop
        );
        // Typing echo takes all four of its values, and --set any setting by name.
        let options = RunOptions {
            typing_echo: Some("words".into()),
            set: vec![
                "voice.punctuation=all".into(),
                "say_all_unit = sentence".into(),
            ],
            ..RunOptions::default()
        };
        assert_eq!(options.apply(&mut settings), Ok(()));
        assert_eq!(settings.typing_echo, okur_core::settings::TypingEcho::Words);
        assert_eq!(
            settings.voice.punctuation,
            okur_core::settings::Punctuation::All
        );
        assert_eq!(
            settings.say_all_unit,
            okur_core::settings::SayAllUnit::Sentence
        );
        for bad in [
            RunOptions {
                typing_echo: Some("loud".into()),
                ..RunOptions::default()
            },
            RunOptions {
                set: vec!["typing_echo".into()],
                ..RunOptions::default()
            },
        ] {
            assert!(bad.apply(&mut settings).is_err());
        }
    }
}
