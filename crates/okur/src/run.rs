//! The live screen reader: `okur run` and `okur-sr.exe` both run it.
//!
//! As a process it follows `docs/references/startup-and-secure-screens-proposal.md`, §5.6:
//! one copy per desktop (an automatic start steps aside for a copy already running, a manual
//! start replaces it), a control pipe through which it is asked to quit, the system's
//! screen-reader flag raised while it runs, last in Windows' shutdown order, and a hidden
//! window that hears the session end. Every way out (Ctrl+C in the console, the control pipe's
//! `quit`, the end of the session) goes down one exit path ([`ExitReason`]). The screen reader
//! itself, from its speech to its last words, is `okur_runtime::host::run`'s.

use okur_core::{AomNode, Role, VirtualBuffer};
use okur_runtime::host::{say_to_console, Config, ExitSender, Exits, Platform};
use okur_runtime::{EventSource, KeyboardHook, SpeechLog};
use okur_windows::PlatformBridge;
use std::path::Path;
use std::sync::Mutex;

pub use okur_runtime::host::ExitReason;

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
        use okur_core::settings::LiveRegionAppend;
        if let Some(engine) = self.engine {
            settings.voice.synthesizer = okur_windows::synthesizer_named(&engine)?;
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

/// Logs as [`okur_runtime::logging::init_logging`] does, with Windows' driver and the
/// programs among the screen reader's crates, and the installed reader's (`kept`) log in
/// [`crate::folders::kept_log`].
pub fn init_logging(debug_log: Option<&str>, kept: bool) {
    let kept = kept.then(crate::folders::kept_log).flatten();
    okur_runtime::logging::init_logging(debug_log, kept, &["okur_windows", "okur"]);
}

/// The platform's keyboard hook: Windows' low-level hook, or the headless mock where no driver
/// is built yet.
fn keyboard_hook() -> Box<dyn KeyboardHook> {
    #[cfg(windows)]
    {
        Box::new(okur_windows::hook::windows::WindowsKeyboardHook::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(okur_runtime::MockKeyboardHook::new())
    }
}

/// The platform's accessibility event listener: Windows' own, or the headless mock where no
/// driver is built yet.
fn event_listener() -> Box<dyn EventSource> {
    #[cfg(windows)]
    {
        Box::new(okur_windows::events::windows::WindowsAccessibilityEventListener::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(okur_runtime::MockEventSource::new())
    }
}

/// How the platform starts `okur-ui`: on Windows below UI Access when Okur has it.
fn ui_spawner() -> Box<dyn okur_runtime::ui_host::UiSpawner> {
    #[cfg(windows)]
    {
        Box::new(okur_windows::ui_spawner::WindowsUiSpawner::for_this_process())
    }
    #[cfg(not(windows))]
    {
        Box::new(okur_runtime::ui_host::PlainSpawner)
    }
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
    okur_runtime::host::log_panics();
    // The sign-in start goes on only as the user, or the machine, chose.
    let start_options = okur_windows::start_options::RegistryStartOptions::system();
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
    okur_windows::process::shut_down_last();

    // The first start for this user, before Okur has a settings folder of theirs, opens the
    // Startup Wizard, as JAWS's runs once it is installed; the folder is made then, so the
    // wizard opens once. A folder given with --settings-folder (the lab's runs) never opens it.
    let first_start = settings_folder.is_none()
        && crate::folders::settings().is_some_and(|folder| !folder.exists());
    // The saved settings, then this run's options over them, for this run only.
    let settings_folder = match settings_folder {
        Some(path) => Some(okur_runtime::settings_file::SettingsFolder::at(path)),
        None => crate::folders::settings().map(okur_runtime::settings_file::SettingsFolder::at),
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
    let log = speech_log.as_ref().map(|path| {
        SpeechLog::open(path)
            .unwrap_or_else(|e| fail(console, &format!("Failed to open speech log {path:?}: {e}")))
    });

    // Determine active buffer
    let (buffer, is_fixture) = if let Some(ref f) = fixture {
        let fixture_path = Path::new(f);
        match okur_windows_model::uia_snapshot::load_aom(fixture_path) {
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

    // Every way out comes here: the runtime takes it from the first.
    let exits = Exits::new();
    // The control pipe: `quit` from the installer, `okur quit`, or a copy taking over.
    let session = okur_windows::desktop::session_id().unwrap_or(0);
    let desktop = okur_windows::desktop::thread_desktop().unwrap_or_default();
    let status = crate::control::Status::of_this_process(session, &desktop);
    let _control = serve_control(session, &desktop, &status, &exits.sender());
    // Windows ending the session: the window's thread waits, briefly, for the exit path,
    // since Windows may end the process once it returns.
    let (ended_tx, ended_rx) = std::sync::mpsc::channel::<()>();
    let ended_rx = Mutex::new(ended_rx);
    let session_exit = exits.sender();
    let session_end = okur_windows::process::EndSessionWatcher::start(Box::new(move || {
        session_exit.send(ExitReason::EndSession);
        if let Ok(ended) = ended_rx.lock() {
            let _ = ended.recv_timeout(std::time::Duration::from_secs(3));
        }
    }));
    if session_end.is_none() {
        tracing::warn!("no window hears the session end");
    }
    // An application running as administrator shows nothing to an Okur below High integrity
    // (Windows Settings' Rename dialog); JAWS runs elevated, the installed Okur with UI Access.
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
    let screen_reader_flag = okur_windows::process::ScreenReaderFlag::raise();

    let platform = Platform {
        keyboard_hook: keyboard_hook(),
        event_source: event_listener(),
        synthesizers: std::sync::Arc::new(okur_windows::WindowsSynthesizers),
        ui_spawner: ui_spawner(),
        actions: windows_actions(),
        clock: windows_clock(),
        sounds: windows_sounds(),
        start_options: Some(Box::new(start_options)),
    };
    let config = Config {
        settings,
        layers,
        settings_folder,
        first_start,
        speech_log: log,
        buffer,
        is_fixture,
        console,
    };
    let reason = match okur_runtime::host::run(platform, config, exits).await {
        Ok(reason) => reason,
        Err(e) => fail(console, &format!("Failed to install keyboard hook: {e}")),
    };
    tracing::debug!(?reason, "the screen reader stopped");
    drop(screen_reader_flag);
    instance_lock.release();
    let _ = ended_tx.send(());
    if console {
        println!("Okur stopped.");
    }
    std::process::exit(0);
}

/// Windows' application actions, started with where their answers go.
fn windows_actions() -> Option<okur_runtime::host::ActionsStart> {
    #[cfg(windows)]
    {
        Some(Box::new(|events| {
            Box::new(okur_windows::actions::WindowsActions::start(Some(events)))
                as Box<dyn okur_core::actions::ApplicationActions>
        }))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Say System Time (Insert+F12) in the user's locale formats.
fn windows_clock() -> Option<Box<dyn okur_core::clock::Clock>> {
    #[cfg(windows)]
    {
        Some(Box::new(okur_windows::clock::WindowsClock))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// A notification rule's sound.
fn windows_sounds() -> Option<Box<dyn okur_core::sound::SoundOutput>> {
    #[cfg(windows)]
    {
        Some(Box::new(okur_windows::sound::WindowsSounds))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
