//! The live screen reader, on every system Okur has a driver for: `okur run` runs it, and on
//! Windows `okur-sr.exe` too. What is the same everywhere is here: the settings folder and its
//! layers with the run's options over them, the speech log, a fixture, and the runtime's
//! `okur_runtime::host::run`, which is the screen reader itself, from its speech to its last
//! words. What a system adds around it is its module's [`System`]: on Windows
//! ([`crate::windows`]) one copy per desktop, the control pipe, the screen-reader flag and the
//! session's end (`docs/references/startup-and-secure-screens-proposal.md`, §5.6); on Linux
//! ([`crate::linux`]) the X11 keyboard, the desktop's AT-SPI events and speech-dispatcher. Every
//! way out goes down one exit path ([`ExitReason`]).

use okur_core::{AomNode, Role, VirtualBuffer};
use okur_runtime::host::{say_to_console, Config, Exits};
use okur_runtime::SpeechLog;
use std::path::Path;

pub use okur_runtime::host::ExitReason;

use crate::startup::StartKind;
use crate::system::{self, System};

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
            settings.voice.synthesizer = system::synthesizer_named(&engine)?;
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
    /// Where the system's driver writes what it read for each event, where it keeps such a
    /// record (Linux: `okur-lab live-replay linux` replays it).
    pub event_trace: Option<String>,
}

/// Logs as [`okur_runtime::logging::init_logging`] does, with the system's driver and the
/// programs among the screen reader's crates, and the installed reader's (`kept`) log in
/// [`crate::folders::kept_log`].
pub fn init_logging(debug_log: Option<&str>, kept: bool) {
    let kept = kept.then(crate::folders::kept_log).flatten();
    okur_runtime::logging::init_logging(debug_log, kept, system::LOG_TARGETS);
}

/// Ends the process after an error before the screen reader ran.
pub(crate) fn fail(console: bool, message: &str) -> ! {
    if console {
        eprintln!("{message}");
    }
    tracing::error!("{message}");
    std::process::exit(1)
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
        event_trace,
    } = config;
    okur_runtime::host::log_panics();
    // The system's say on whether this start goes on (on Windows, the sign-in start as the
    // user chose, one copy per desktop).
    let Some(mut system) = System::start(start, console) else {
        return;
    };

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
    } else {
        // Live, the event listener starts with the focus as it is (a page is loaded
        // when that focus is in one), as any later focus change.
        let empty_doc = AomNode::new(0, Role::Document);
        (VirtualBuffer::from_aom(&empty_doc), false)
    };

    // Every way out comes here: the runtime takes it from the first.
    let exits = Exits::new();
    // What the system asks to stop Okur by, and tells while it runs.
    system.serve(&exits, console);
    let platform = system.platform(event_trace);
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
    system.stop();
    if console {
        println!("Okur stopped.");
    }
    std::process::exit(0);
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
