//! Okur on Linux, live, for the Linux lab (`skills/linux-lab/scripts/probe-okur.sh`): the
//! runtime's event loop (`host::run`) with Linux's platform, the X11 keyboard hook, the desktop's
//! AT-SPI events and speech-dispatcher, printing what it says. `--no-speech` speaks through the
//! runtime's silent synthesizer instead. Runs for the seconds given (20 by default).

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
    let seconds = args.iter().find_map(|a| a.parse().ok()).unwrap_or(20u64);
    let mut settings = okur_core::settings::Settings::default();
    if args.iter().any(|a| a == "--no-speech") {
        settings.voice.synthesizer = okur_core::settings::Synthesizer::NoSpeech;
    }
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
        layers: Default::default(),
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

#[cfg(not(target_os = "linux"))]
fn main() {}
