//! Okur on Linux, live, for the Linux lab (`skills/linux-lab/scripts/probe-okur.sh`): the
//! runtime's event loop (`host::run`) with Linux's platform, the X11 keyboard hook, the desktop's
//! AT-SPI events and speech-dispatcher, printing what it says. `--no-speech` speaks through the
//! runtime's silent synthesizer instead. Runs for the seconds given (20 by default).
//! `--web-verbosity <level>` reads pages at a JAWS `VirtualCursorVerbosityLevel` (2 High, 1
//! Medium, 0 Low): the lab's runs read at the level JAWS's evidence was recorded at.

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() {
    use okur_runtime::host::{self, Config, ExitReason, Exits, Platform};

    // RUST_LOG, as the runtime's programs take it (okur_core=debug for the engine's choices).
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seconds = run_seconds(&args);
    // This run's options over the default settings, in the session's layer, as `okur run`
    // puts them: the engine takes its settings from the layers, and the options set on
    // Config's settings alone reached only the voice (the lab's web rerun of 2026-11-08 read
    // every page at Medium with --web-verbosity 2).
    let mut layers = okur_core::settings_layers::SettingsLayers::default();
    let defaults = layers.resolve(None, None);
    let mut settings = defaults.clone();
    if args.iter().any(|a| a == "--no-speech") {
        settings.voice.synthesizer = okur_core::settings::Synthesizer::NoSpeech;
    }
    if let Some(level) = args
        .iter()
        .position(|a| a == "--web-verbosity")
        .and_then(|i| args.get(i + 1))
    {
        settings.web_verbosity = okur_core::settings::WebVerbosity::from_jcf(level);
    }
    layers.change(
        &okur_core::settings_layers::Scope::Session,
        &defaults,
        &settings,
        None,
    );
    let event_source = okur_linux::events::AtspiEventSource::new();
    let objects = event_source.objects();
    let platform = Platform {
        keyboard_hook: Box::new(okur_linux::keyboard::X11KeyboardHook::new()),
        event_source: Box::new(event_source),
        synthesizers: std::sync::Arc::new(okur_linux::speech::LinuxSynthesizers),
        ui_spawner: Box::new(okur_runtime::ui_host::PlainSpawner),
        // Enter and Space on a page's elements, and the focus given to them.
        actions: Some(Box::new(move |_sink| {
            Box::new(okur_linux::actions::LinuxActions::new(objects))
                as Box<dyn okur_core::actions::ApplicationActions>
        })),
        clock: None,
        sounds: None,
        start_options: None,
    };
    let config = Config {
        settings,
        layers,
        settings_folder: None,
        first_start: false,
        // --speech-log <path>: every utterance as JSON lines, keys' speech too.
        speech_log: args
            .iter()
            .position(|a| a == "--speech-log")
            .and_then(|i| args.get(i + 1))
            .and_then(|path| okur_runtime::SpeechLog::open(path).ok()),
        buffer: Default::default(),
        is_fixture: false,
        console: true,
    };
    let exits = Exits::new();
    let stop = exits.sender();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
        stop.send(ExitReason::Interrupted);
    });
    match host::run(platform, config, exits).await {
        Ok(reason) => println!("Okur stopped: {reason:?}"),
        Err(error) => println!("Okur did not run: {error}"),
    }
}

/// The seconds to run, the first number among the arguments that is no option's value
/// (`--speech-log <path>`, `--web-verbosity <level>`); the lab's runner passes
/// `--no-speech 3600 --speech-log <path>`, a flag before the number.
#[cfg(target_os = "linux")]
fn run_seconds(args: &[String]) -> u64 {
    const WITH_VALUE: [&str; 2] = ["--speech-log", "--web-verbosity"];
    let mut skip = false;
    for arg in args {
        if std::mem::take(&mut skip) {
            continue;
        }
        if WITH_VALUE.contains(&arg.as_str()) {
            skip = true;
        } else if let Ok(seconds) = arg.parse() {
            return seconds;
        }
    }
    20
}

#[cfg(not(target_os = "linux"))]
fn main() {}
