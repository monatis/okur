//! The screen reader running: the engine with its speech, its keyboard hook and its
//! accessibility events, from start to exit, on any system ([`run`]). The system's own start
//! (one copy per desktop, its control pipe, the session's end) is the program's: it hands this
//! its ports ([`Platform`]) and every way out ([`Exits`]), and ends the process once [`run`]
//! returns.

use okur_core::actions::ApplicationActions;
use okur_core::settings::{Settings, StartOptions};
use okur_core::settings_layers::SettingsLayers;
use okur_core::{input::KeyAction, AccessibilityEvent, AomNode, ScreenReaderEngine, VirtualBuffer};
use std::sync::{Arc, Mutex};

use crate::settings_file::{BackgroundStore, SettingsFolder};
use crate::ui_host::{ProcessUiHost, UiSpawner};
use crate::{
    EventSink, EventSource, KeyboardHook, RuntimeError, SpeechLog, SpeechThread,
    SynthesizerFactory, TtsManager,
};

/// What starts a system's application actions, given where their answers go.
pub type ActionsStart = Box<dyn FnOnce(EventSink) -> Box<dyn ApplicationActions> + Send>;

/// What a system gives the runtime: its ports, and what the engine acts and hears through.
pub struct Platform {
    pub keyboard_hook: Box<dyn KeyboardHook>,
    pub event_source: Box<dyn EventSource>,
    pub synthesizers: Arc<dyn SynthesizerFactory>,
    pub ui_spawner: Box<dyn UiSpawner>,
    /// What acts on applications for the engine (Enter on an edit focuses it, Enter or Space on
    /// a control activates it), started with where its answers go.
    pub actions: Option<ActionsStart>,
    /// Say System Time in the user's locale formats.
    pub clock: Option<Box<dyn okur_core::clock::Clock>>,
    /// A notification rule's sound.
    pub sounds: Option<Box<dyn okur_core::sound::SoundOutput>>,
    /// Settings Center's start options (Okur after sign-in, at the sign-in screen).
    pub start_options: Option<Box<dyn StartOptions>>,
}

/// What a run starts with.
pub struct Config {
    /// The settings for this run: the saved ones, the run's options over them.
    pub settings: Settings,
    pub layers: SettingsLayers,
    /// Where the settings dialog, custom labels and the rest are kept, if anywhere.
    pub settings_folder: Option<SettingsFolder>,
    /// The first start for this user: the Startup Wizard opens.
    pub first_start: bool,
    pub speech_log: Option<SpeechLog>,
    /// The page read first, and whether it is a fixture (a snapshot) rather than the system.
    pub buffer: VirtualBuffer,
    pub is_fixture: bool,
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
    /// The system is ending the session: out fast, nothing said.
    EndSession,
}

/// Every way out of the screen reader: [`run`] takes the first that comes.
pub struct Exits {
    sender: ExitSender,
    received: tokio::sync::mpsc::UnboundedReceiver<ExitReason>,
}

impl Exits {
    pub fn new() -> Self {
        let (sender, received) = tokio::sync::mpsc::unbounded_channel();
        Self {
            sender: ExitSender(sender),
            received,
        }
    }

    /// What tells the runtime to stop, and why.
    pub fn sender(&self) -> ExitSender {
        self.sender.clone()
    }
}

impl Default for Exits {
    fn default() -> Self {
        Self::new()
    }
}

/// Tells [`run`] to stop.
#[derive(Debug, Clone)]
pub struct ExitSender(tokio::sync::mpsc::UnboundedSender<ExitReason>);

impl ExitSender {
    pub fn send(&self, reason: ExitReason) {
        let _ = self.0.send(reason);
    }
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

/// The keys acting as the Okur key, as the engine's keyboard settings last said ([`okur_keys`]):
/// the hook decides by them alone when the engine is busy.
static OKUR_KEYS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(OKUR_INSERT);
const OKUR_INSERT: u8 = 1;
const OKUR_CAPS_LOCK: u8 = 2;
const OKUR_SCROLL_LOCK: u8 = 4;

/// Which keys act as the Okur key under `keyboard`, as [`OKUR_KEYS`] keeps them.
fn okur_keys(keyboard: &okur_core::settings::KeyboardSettings) -> u8 {
    use okur_core::input::Key;
    [
        (Key::Insert, OKUR_INSERT),
        (Key::CapsLock, OKUR_CAPS_LOCK),
        (Key::ScrollLock, OKUR_SCROLL_LOCK),
    ]
    .into_iter()
    .filter(|(key, _)| keyboard.is_okur_key(*key))
    .fold(0, |keys, (_, bit)| keys | bit)
}

/// What becomes of a key the engine was too busy to take: the Okur key and every chord with it
/// are Okur's, never the application's, as the engine has them (Insert reaching Word toggled
/// its overtype, Insert+T typed "t"; performance review 2026-11-02, live path finding 5);
/// any other key goes on to the application.
fn busy_key_action(event: &okur_core::input::KeyEvent, okur: u8) -> KeyAction {
    use okur_core::input::Key;
    let held = |bit: u8, down: bool| down && okur & bit != 0;
    let okur_key = match event.key {
        Key::Insert => okur & OKUR_INSERT != 0,
        Key::CapsLock => okur & OKUR_CAPS_LOCK != 0,
        Key::ScrollLock => okur & OKUR_SCROLL_LOCK != 0,
        _ => false,
    };
    let m = &event.modifiers;
    if okur_key
        || held(OKUR_INSERT, m.insert)
        || held(OKUR_CAPS_LOCK, m.caps_lock)
        || held(OKUR_SCROLL_LOCK, m.scroll_lock)
    {
        KeyAction::Consume
    } else {
        KeyAction::PassThrough
    }
}

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
pub fn log_panics() {
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

/// How long Okur's exit waits for the engine before it goes without it.
const EXIT_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// `engine`, taken within `wait`, as it is after a panic too; `None` where it stays held.
fn take_within<T>(
    engine: &std::sync::Mutex<T>,
    wait: std::time::Duration,
) -> Option<std::sync::MutexGuard<'_, T>> {
    use std::sync::TryLockError;
    let until = std::time::Instant::now() + wait;
    loop {
        match engine.try_lock() {
            Ok(guard) => return Some(guard),
            Err(TryLockError::Poisoned(poisoned)) => return Some(recovered(engine, poisoned)),
            Err(TryLockError::WouldBlock) if std::time::Instant::now() >= until => return None,
            Err(TryLockError::WouldBlock) => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
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
    event.map_or("idle", crate::logging::event_kind)
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

/// Prints a line in the console program; logs it either way.
pub fn say_to_console(console: bool, line: &str) {
    if console {
        println!("{line}");
    }
    tracing::info!("{line}");
}

/// Runs the screen reader until a way out comes ([`Exits`]), and returns why, its last words
/// said: the program then ends the process. An error is a keyboard hook that did not start.
pub async fn run(
    platform: Platform,
    config: Config,
    exits: Exits,
) -> Result<ExitReason, RuntimeError> {
    let Platform {
        keyboard_hook: mut hook,
        event_source: mut event_listener,
        synthesizers,
        ui_spawner,
        actions,
        clock,
        sounds,
        start_options,
    } = platform;
    let Config {
        settings,
        layers,
        settings_folder,
        first_start,
        speech_log,
        buffer,
        is_fixture,
        console,
    } = config;
    let Exits {
        sender: exit_tx,
        received: mut exit_rx,
    } = exits;
    // Speech on its own thread, which makes the synthesizer it calls: the keyboard hook never
    // waits for a synthesizer.
    let synthesizer = settings.voice.synthesizer;
    let mut speech = SpeechThread::start(speech_log, move || {
        TtsManager::starting(synthesizers, synthesizer)
    });
    okur_core::speech::SpeechOutput::configure(&mut speech, &settings.voice);

    if console {
        println!("============================================================");
        println!("  Okur, an open screen reader");
        println!("  TTS Driver: {}", speech.engine_name());
        println!("============================================================");
    }

    let mut cursor = okur_core::VirtualCursor::new(buffer);
    cursor.move_to_start();

    // OS accessibility events and caret text both reach the engine through this channel.
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
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
    if let Some(start) = actions {
        engine.set_actions(start(EventSink::new(event_tx.clone())));
    }
    // Okur's own windows (the Virtual Viewer on screen), drawn by okur-ui.
    let mut ui_closer = None;
    if let Some(ui) = ProcessUiHost::start(EventSink::new(event_tx.clone()), ui_spawner) {
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
        // Written on a thread of its own, off the engine's, which the keyboard hook waits on
        // (performance review 2026-10-25, finding 7; 2026-11-02, speech finding 9).
        let store = BackgroundStore::new(folder.clone());
        engine.set_custom_labels(folder.load_labels(), Some(Box::new(store.clone())));
        engine.set_placemarkers(folder.load_placemarkers(), Some(Box::new(store.clone())));
        engine.set_kept_titles(folder.load_excel_titles(), Some(Box::new(store.clone())));
        engine.set_flexible_web_rules(
            folder.load_flexible_web_rules(),
            Some(Box::new(store.clone())),
        );
        engine.set_notification_rules(
            folder.load_notification_rules(),
            Some(Box::new(store.clone())),
        );
        engine.set_settings_history(&folder.load_settings_history());
        if let Some(options) = start_options {
            engine.set_start_options(options);
        }
        if first_start {
            if let Err(e) = std::fs::create_dir_all(folder.folder()) {
                tracing::warn!(folder = ?folder.folder(), error = %e, "settings folder not made");
            }
            engine.open_startup_wizard();
        }
        engine.set_settings_store(Box::new(store));
    }
    // Say System Time (Insert+F12) in the user's locale formats; a notification rule's sound.
    if let Some(clock) = clock {
        engine.set_clock(clock);
    }
    if let Some(sounds) = sounds {
        engine.set_sounds(sounds);
    }
    // Insert+F4: the engine asks; the runtime stops.
    let command_exit = exit_tx.clone();
    engine.set_exit(Box::new(move || command_exit.send(ExitReason::Command)));
    let engine = Arc::new(Mutex::new(engine));

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
            say_to_console(console, "[Okur] Ready in live native navigation mode.");
        }
    }

    // Install low-level keyboard hook
    let engine_cb = Arc::clone(&engine);

    hook.start(Box::new(move |event| {
        // Windows silently removes a low-level keyboard hook that once exceeds
        // LowLevelHooksTimeout, so the hook never waits long for the engine: a key
        // that finds it busy for 150 ms passes to the application, unless it is Okur's.
        let arrived = std::time::Instant::now();
        let guard = take_for_key(&engine_cb, arrived);
        let Some(mut eng) = guard else {
            let action =
                busy_key_action(&event, OKUR_KEYS.load(std::sync::atomic::Ordering::Relaxed));
            tracing::warn!(key = ?event.key, ?action, "engine busy: key not handled");
            return action;
        };
        {
            let waited = arrived.elapsed();
            let started = std::time::Instant::now();
            let action = unless_it_panics("key", || eng.handle_key_event(&event))
                .unwrap_or(KeyAction::PassThrough);
            OKUR_KEYS.store(
                okur_keys(&eng.settings.keyboard),
                std::sync::atomic::Ordering::Relaxed,
            );
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
        }
    }))?;

    // Start OS-level accessibility event listener
    let mut event_task = None;
    if let Err(e) = event_listener.start(EventSink::new(event_tx)) {
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
            // Every engine call in this loop runs in `unless_it_panics`: a panic unwinding
            // out of it would end the task, and Okur would hear no event again, keys still
            // working (performance review 2026-11-02, live path finding 6).
            loop {
                let received = tokio::select! {
                    biased;
                    Some(mark) = marks_rx.recv() => {
                        give_way_to_keys();
                        let mut eng = lock_engine(&engine_events);
                        unless_it_panics("speech mark", || eng.speech_reached(mark));
                        continue;
                    }
                    Some((number, title, buffer)) = built_rx.recv() => {
                        if number > refresh_applied {
                            refresh_applied = number;
                            give_way_to_keys();
                            let mut eng = lock_engine(&engine_events);
                            unless_it_panics("page refreshed", || {
                                eng.handle_page_refreshed(&title, buffer);
                            });
                        }
                        continue;
                    }
                    Some((generation, buffer)) = rebuilt_rx.recv() => {
                        give_way_to_keys();
                        let mut eng = lock_engine(&engine_events);
                        unless_it_panics("page rebuilt", || {
                            eng.install_page_build(generation, buffer);
                        });
                        continue;
                    }
                    // Installed if the page is still the one they were spliced into;
                    // else the parts go into the page as it is now, under the lock.
                    Some(BuiltParts { base, buffer, parts, attempt }) = parts_rx.recv() => {
                        give_way_to_keys();
                        let mut eng = lock_engine(&engine_events);
                        let taken = unless_it_panics("page parts", || match buffer {
                            Some(buffer) => eng.install_page_parts(&base, buffer),
                            None => eng.page_parts_not_found(&base),
                        })
                        .unwrap_or(true);
                        if taken {
                            continue;
                        }
                        // The page is another by now: the parts go into it as it is, off
                        // the engine again (performance review P3 step 3); after a few
                        // tries, under it.
                        if attempt >= PARTS_ATTEMPTS {
                            unless_it_panics("page parts", || {
                                eng.handle_accessibility_event(
                                    &AccessibilityEvent::PagePartsLoaded(parts),
                                )
                            });
                            continue;
                        }
                        let base = unless_it_panics("page parts", || eng.page_for_parts()).flatten();
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
                        // A page the platform read again, unloaded, is the held one's refresh
                        // wherever the focus is (okur_core::aom::READ_AGAIN_ATTR).
                        let read_again = root
                            .attributes
                            .contains_key(okur_core::aom::READ_AGAIN_ATTR);
                        give_way_to_keys();
                        let read = {
                            let eng = lock_engine(&engine_events);
                            unless_it_panics("page loaded", || {
                                (
                                    eng.is_refresh(title, url.as_deref(), read_again),
                                    eng.reading_marks(),
                                )
                            })
                        };
                        let Some((refresh, marks)) = read else {
                            continue;
                        };
                        let mut root = std::mem::replace(
                            root,
                            okur_core::AomNode::new(0, okur_core::Role::Document),
                        );
                        // Built as the reading choices read it, so the engine need not build
                        // it again.
                        if unless_it_panics("page marked", || marks.mark(&mut root)).is_none() {
                            continue;
                        }
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
                        let Some(buffer) =
                            unless_it_panics("page built", || VirtualBuffer::from_aom_owned(root))
                        else {
                            continue;
                        };
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
                    let base = {
                        let eng = lock_engine(&engine_events);
                        unless_it_panics("page parts", || eng.page_for_parts()).flatten()
                    };
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
                    if let Some((generation, page)) =
                        unless_it_panics("page build", || eng.take_page_build()).flatten()
                    {
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
    // The engine, within a while: a handler stuck holding it left a process with no keyboard
    // and no speech that held the desktop's lock, until a new start ended it (performance
    // review 2026-11-02, speech finding 11).
    let said = match take_within(&engine, EXIT_WAIT) {
        Some(mut eng) => {
            // The settings applied last are kept before Okur goes.
            eng.flush_settings();
            if reason == ExitReason::EndSession {
                eng.speech.stop();
                false
            } else {
                let words = eng.exit_words();
                eng.speech.speak(&words, true);
                true
            }
        }
        None => {
            tracing::error!("the engine is held: Okur goes without its last words");
            false
        }
    };
    if said {
        let _ = speech_idle.wait(std::time::Duration::from_millis(1500));
    }
    if let Some(ui) = ui_closer {
        ui.close(std::time::Duration::from_millis(500));
    }
    Ok(reason)
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

    /// Okur's exit takes the engine when it is free, and goes without it when a handler holds
    /// it past the wait.
    #[test]
    fn the_exit_does_not_wait_for_a_held_engine_for_good() {
        let engine = Arc::new(Mutex::new(7_u32));
        assert_eq!(
            take_within(&engine, Duration::from_millis(50)).map(|e| *e),
            Some(7)
        );
        let held = engine.lock();
        let started = Instant::now();
        let other = Arc::clone(&engine);
        let taken =
            std::thread::spawn(move || take_within(&other, Duration::from_millis(50)).is_some())
                .join();
        assert_eq!(taken.ok(), Some(false));
        assert!(started.elapsed() < Duration::from_secs(2));
        drop(held);
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

    /// A key the busy engine could not take: the Okur key and its chords stay Okur's, with the
    /// keys that act as it in each layout; other keys go to the application.
    #[test]
    fn a_busy_engine_keeps_the_okur_keys_from_the_application() {
        use okur_core::input::{Key, KeyEvent, Modifiers};
        use okur_core::settings::{KeyboardLayout, KeyboardSettings, OkurKey};
        let key = |key: Key, modifiers: Modifiers| KeyEvent {
            key,
            modifiers,
            is_down: true,
            is_repeat: false,
            text: None,
            time_ms: 0,
            caps_lock_on: None,
        };
        let insert = Modifiers {
            insert: true,
            ..Modifiers::none()
        };
        let caps = Modifiers {
            caps_lock: true,
            ..Modifiers::none()
        };
        let desktop = okur_keys(&KeyboardSettings::default());
        assert_eq!(desktop, OKUR_INSERT);
        let consumed = |event: &KeyEvent, okur| busy_key_action(event, okur) == KeyAction::Consume;
        assert!(consumed(&key(Key::Insert, Modifiers::none()), desktop));
        assert!(consumed(&key(Key::Alpha('t'), insert), desktop));
        assert!(consumed(&key(Key::DownArrow, insert), desktop));
        assert!(!consumed(&key(Key::Alpha('t'), Modifiers::none()), desktop));
        assert!(!consumed(&key(Key::CapsLock, Modifiers::none()), desktop));
        assert!(!consumed(&key(Key::Alpha('t'), caps), desktop));
        // The laptop layout: CapsLock is the Okur key, Insert still one.
        let laptop = okur_keys(&KeyboardSettings {
            layout: KeyboardLayout::Laptop,
            okur_key: OkurKey::CapsLock,
        });
        assert!(consumed(&key(Key::Alpha('t'), caps), laptop));
        assert!(consumed(&key(Key::CapsLock, Modifiers::none()), laptop));
        assert!(consumed(&key(Key::Alpha('t'), insert), laptop));
    }
}
