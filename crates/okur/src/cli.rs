//! The console program `okur`: `okur run` starts Okur; `okur voices` lists the synthesizers'
//! voices; on Windows `okur quit` asks the Okur on this desktop to quit and `okur doctor` says
//! what Windows grants it. The verification harness is `okur-lab` (crates/okur-lab).

use clap::{Parser, Subcommand};
use okur_runtime::TtsManager;

#[derive(Parser, Debug)]
#[command(
    name = "okur",
    about = "Okur: an open screen reader that keeps your JAWS muscle memory"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

// Run carries every option of a run and Quit none; the enum is parsed once, as the program
// starts.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
enum Commands {
    /// Run Okur, the live screen reader. Its settings are the ones its settings dialog
    /// (Insert+6 or Insert+V) saved; the options below change them for this run only.
    Run {
        /// TTS synthesizer engine: auto (on Windows OneCore, else SAPI 5, else none; on Linux
        /// speech-dispatcher), no-speech, and on Windows onecore or sapi5
        #[arg(short, long)]
        engine: Option<String>,

        /// Synthesizer voice, by name
        #[arg(long)]
        voice: Option<String>,

        /// Speech rate (-10 to 10)
        #[arg(short, long)]
        rate: Option<i32>,

        /// Volume (0 to 100)
        #[arg(long)]
        volume: Option<u16>,

        /// UIA snapshot to load instead of the active application (okur snapshot output)
        #[arg(short, long)]
        fixture: Option<String>,

        /// Record every utterance as JSON Lines to this file (off by default)
        #[arg(long)]
        speech_log: Option<String>,

        /// Write a debug log of keys, engine decisions, OS events, caret reads and speech
        /// to this file, for diagnosing live sessions (off by default)
        #[arg(long)]
        debug_log: Option<String>,

        /// Keyboard layout, as JAWS's: desktop (Insert is the Okur key) or laptop (CapsLock is
        /// the Okur key, and Insert works as well)
        #[arg(long)]
        keyboard_layout: Option<String>,

        /// The Okur key for the layout: insert, capslock or scrolllock (default: the layout's
        /// own)
        #[arg(long)]
        okur_key: Option<String>,

        /// Read a growing live region's whole text again on every change, as JAWS does
        /// (default: only what was appended, so a streamed chat reply is read once)
        #[arg(long)]
        live_region_whole_text: bool,

        /// Typing echo: characters (as JAWS's default), words, both or none
        #[arg(long)]
        typing_echo: Option<String>,

        /// Any setting, by its name in the settings file: --set typing_echo=words, --set
        /// voice.punctuation=all (repeatable)
        #[arg(long = "set", value_name = "ID=VALUE")]
        set: Vec<String>,

        /// The settings folder to read and save instead of the user's own, so that a test run
        /// never changes the user's settings
        #[arg(long)]
        settings: Option<String>,

        /// Write what the driver read for each accessibility event to this file, a JSON line
        /// each, for `okur-lab live-replay linux` (off by default)
        #[cfg(target_os = "linux")]
        #[arg(long)]
        trace: Option<String>,
    },
    /// List available voices from the synthesizer
    Voices {
        /// TTS synthesizer engine: auto (the default: on Windows OneCore, else SAPI 5, else none;
        /// on Linux speech-dispatcher), no-speech, and on Windows onecore or sapi5
        #[arg(short, long, default_value = "auto")]
        engine: String,
    },
    #[cfg(windows)]
    /// Print what Windows grants this process, or another: session, desktop, account,
    /// integrity, UI Access, elevation, the executable's signature and manifest, and Okur's
    /// Ease of Access registration (read only)
    Doctor {
        /// Report on this process instead (its desktop is not shown)
        #[arg(long)]
        pid: Option<u32>,
    },
    #[cfg(windows)]
    /// Ask the Okur running on this desktop to quit, without its question
    Quit,
}

fn init_logging(cli: &Cli) {
    let debug_log = match &cli.command {
        Some(Commands::Run { debug_log, .. }) => debug_log.as_deref(),
        _ => None,
    };
    crate::run::init_logging(debug_log, false);
}

/// Runs the console program's command; the exit code.
pub async fn main() -> i32 {
    let cli = Cli::parse();
    init_logging(&cli);
    crate::system::init();

    match cli.command {
        Some(Commands::Run {
            engine,
            voice,
            rate,
            volume,
            fixture,
            speech_log,
            debug_log: _,
            keyboard_layout,
            okur_key,
            live_region_whole_text,
            typing_echo,
            set,
            settings: settings_folder,
            #[cfg(target_os = "linux")]
            trace,
        }) => {
            #[cfg(target_os = "linux")]
            let event_trace = trace;
            #[cfg(not(target_os = "linux"))]
            let event_trace = None;
            // The live screen reader, as `okur-sr.exe` runs it, with this console.
            crate::run::run(crate::run::RunConfig {
                options: crate::run::RunOptions {
                    engine,
                    voice,
                    rate,
                    volume,
                    keyboard_layout,
                    okur_key,
                    live_region_whole_text,
                    typing_echo,
                    set,
                },
                fixture,
                speech_log,
                settings: settings_folder,
                start: crate::startup::StartKind::Manual,
                console: true,
                event_trace,
            })
            .await;
        }
        #[cfg(windows)]
        Some(Commands::Doctor { pid }) => {
            print!("{}", crate::doctor::render(&crate::doctor::gather(pid)));
        }
        #[cfg(windows)]
        Some(Commands::Quit) => match crate::control::quit_this_desktop() {
            Ok(said) => println!("{said}"),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        Some(Commands::Voices { engine }) => {
            let engine_type = crate::system::synthesizer_named(&engine).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });

            let tts =
                TtsManager::new(crate::system::synthesizers(), engine_type).unwrap_or_else(|e| {
                    eprintln!("Failed to initialize TTS engine: {}", e);
                    std::process::exit(1);
                });

            println!("Available Voices for [{}] synthesizer:", tts.engine_name());
            for (idx, voice) in tts.get_voices().iter().enumerate() {
                println!(
                    "  {}. ID: '{}', Name: '{}', Lang: {:?}",
                    idx + 1,
                    voice.id,
                    voice.name,
                    voice.language
                );
            }
        }
        None => {
            println!("No command specified. Use `okur --help` for available options.");
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// `okur run` names Okur's modifier in Okur's name (AGENTS.md §1.5): `--okur-key`, the
    /// "Okur key" in its help, never "JAWS key".
    #[test]
    fn run_names_the_okur_key() {
        let parsed = Cli::try_parse_from(["okur", "run", "--okur-key", "scrolllock"]);
        assert!(matches!(
            parsed,
            Ok(Cli {
                command: Some(Commands::Run {
                    okur_key: Some(key),
                    ..
                })
            }) if key == "scrolllock"
        ));
        assert!(Cli::try_parse_from(["okur", "run", "--jaws-key", "scrolllock"]).is_err());
        let mut cli = Cli::command();
        let help = cli
            .find_subcommand_mut("run")
            .map(|run| run.render_long_help().to_string())
            .unwrap_or_default();
        assert!(help.contains("--okur-key"), "{help}");
        assert!(help.contains("Okur key"), "{help}");
        assert!(!help.to_lowercase().contains("jaws key"), "{help}");
    }
}
