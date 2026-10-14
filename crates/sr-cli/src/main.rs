use clap::{Parser, Subcommand};
use sr_core::{AomNode, Role, ScreenReaderEngine, VirtualBuffer, VirtualCursor};
use sr_platform::{
    create_event_listener, create_keyboard_hook, KeyAction, PlatformBridge, SpeechLog,
    TtsEngineType, TtsManager,
};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(name = "sr", about = "Open-Source Rust Screen Reader CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the interactive live screen reader daemon
    Run {
        /// TTS synthesizer engine: auto (default: OneCore, else SAPI 5, else none), onecore,
        /// sapi5, or no-speech
        #[arg(short, long, default_value = "auto")]
        engine: String,

        /// Specific synthesizer voice name or ID
        #[arg(long)]
        voice: Option<String>,

        /// Speech rate (-10 to 10)
        #[arg(short, long, default_value_t = 0)]
        rate: i32,

        /// Volume (0 to 100)
        #[arg(long, default_value_t = 100)]
        volume: u16,

        /// UIA snapshot to load instead of the active application (sr-cli snapshot output)
        #[arg(short, long)]
        fixture: Option<String>,

        /// Record every utterance as JSON Lines to this file (off by default)
        #[arg(long)]
        speech_log: Option<String>,

        /// Write a debug log of keys, engine decisions, OS events, caret reads and speech
        /// to this file, for diagnosing live sessions (off by default)
        #[arg(long)]
        debug_log: Option<String>,

        /// JAWS keyboard layout: desktop (default; Insert is the JAWS key) or laptop (CapsLock
        /// is the JAWS key, and Insert works as well)
        #[arg(long, default_value = "desktop")]
        keyboard_layout: String,

        /// The JAWS key for the layout: insert, capslock or scrolllock (default: the
        /// layout's own)
        #[arg(long)]
        jaws_key: Option<String>,
    },
    /// Compare engine speech with recorded JAWS speech for every genuine corpus scenario
    Parity {
        #[arg(short, long, default_value = "corpus")]
        corpus_dir: String,

        /// Print every step, not only mismatches
        #[arg(long)]
        all: bool,

        /// Regenerate corpus/parity-baseline.json from the current results
        #[arg(long)]
        write_baseline: bool,

        /// Snapshot API to drive the engine with: ia2 (the ratchet's API) or uia
        #[arg(long)]
        api: Option<String>,

        /// List spoken-layer differences (what the synthesizer is given, against what JAWS
        /// sent its synthesizer in evidence recorded through the speech recorder voice)
        #[arg(long)]
        spoken: bool,
    },
    /// Act on the foreground browser page like Enter/Space would (Windows): find a control by
    /// name in the page's IA2 capture, request focus or its default action, then report the
    /// element UIA says has keyboard focus. A live check of the engine's action path.
    Act {
        /// Accessible name of the target control (first match in document order)
        #[arg(long)]
        name: String,

        /// focus or activate
        #[arg(long, default_value = "focus")]
        action: String,
    },
    /// Read the focused edit's text at its caret, as caret keys do in an edit (Windows)
    Caret {
        /// char, word or line
        #[arg(long, default_value = "char")]
        unit: String,
    },
    /// Record system-wide accessibility events as JSON Lines (Windows)
    Trace {
        /// Output trace file
        #[arg(long)]
        out: String,

        /// Stop when this file appears (lets a capture script end the trace cleanly)
        #[arg(long)]
        stop_file: Option<String>,

        /// Hard limit on the trace duration (milliseconds)
        #[arg(long, default_value_t = 600_000)]
        max_ms: u64,
    },
    /// Print an evidence file's steps and JAWS transcripts readably
    Evidence {
        /// Evidence file path, or a scenario id looked up in corpus/evidence
        evidence: String,

        /// Trial to print (1-based); variants across all trials are always listed
        #[arg(long, default_value_t = 1)]
        trial: usize,

        /// Also list the accessibility events traced during each step of the trial
        #[arg(long)]
        trace: bool,
    },
    /// Wait until the default audio output is silent; prints a JSON SilenceReport
    WaitSilence {
        /// Give up waiting for any audio to start after this long (no speech)
        #[arg(long, default_value_t = 800)]
        onset_ms: u64,
        /// Silence required after the last audio
        #[arg(long, default_value_t = 500)]
        quiet_ms: u64,
        /// Hard deadline
        #[arg(long, default_value_t = 4000)]
        max_ms: u64,
        #[arg(long, default_value_t = 0.001)]
        threshold: f32,
    },
    /// Check universal virtual-buffer invariants on every captured snapshot
    Invariants {
        #[arg(short, long, default_value = "corpus")]
        corpus_dir: String,

        /// Regenerate corpus/invariants-baseline.json from the current results
        #[arg(long)]
        write_baseline: bool,
    },
    /// Capture the foreground window's document as a raw UIA snapshot (Windows)
    Snapshot {
        /// Output JSON file
        #[arg(long)]
        out: String,

        /// Accessibility API to capture: uia (default) or ia2 (Chromium/Firefox)
        #[arg(long, default_value = "uia")]
        api: String,

        /// Fixture file the page was loaded from; its SHA-256 is recorded in provenance
        #[arg(long)]
        fixture: Option<String>,

        /// Version of the captured application (e.g. Chrome's file version)
        #[arg(long)]
        process_version: Option<String>,

        /// Wait before capturing, to let the page settle (milliseconds)
        #[arg(long, default_value_t = 0)]
        delay_ms: u64,
    },
    /// Check (default) or regenerate the generated corpus manifest
    CorpusManifest {
        #[arg(short, long, default_value = "corpus")]
        corpus_dir: String,

        /// Regenerate CORPUS_MANIFEST.json from the files on disk instead of checking it
        #[arg(long)]
        write: bool,
    },
    /// Speak arbitrary text aloud using the TTS synthesizer
    Speak {
        /// Text utterance to synthesize
        text: String,

        /// TTS synthesizer engine: auto (default: OneCore, else SAPI 5, else none), onecore,
        /// sapi5, or no-speech
        #[arg(short, long, default_value = "auto")]
        engine: String,

        /// Speech rate (-10 to 10)
        #[arg(short, long, default_value_t = 0)]
        rate: i32,

        /// Volume (0 to 100)
        #[arg(short, long, default_value_t = 100)]
        volume: u16,

        /// Measure the time from the speak call to the first sound on the default output
        /// (the system must be otherwise silent)
        #[arg(long)]
        measure_latency: bool,

        /// First start a long utterance (a page read aloud) and interrupt it after this many
        /// milliseconds with the text: how fast speech recovers from an interruption
        #[arg(long)]
        after_long_ms: Option<u64>,
    },
    /// Load and read a document fixture aloud using the virtual cursor
    Read {
        /// UIA snapshot to read (sr-cli snapshot output, e.g. corpus/snapshots/*.json)
        fixture: String,

        /// TTS synthesizer engine: auto (default: OneCore, else SAPI 5, else none), onecore,
        /// sapi5, or no-speech
        #[arg(short, long, default_value = "auto")]
        engine: String,

        /// Reading command: say-all (default), line, or table
        #[arg(short, long, default_value = "say-all")]
        mode: String,
    },
    /// List available voices from the synthesizer
    Voices {
        /// TTS synthesizer engine: auto (default: OneCore, else SAPI 5, else none), onecore,
        /// sapi5, or no-speech
        #[arg(short, long, default_value = "auto")]
        engine: String,
    },
    /// Print a speech log recorded with `run --speech-log`
    History {
        /// Path of the speech log file
        path: String,
    },
    /// Query hardware soundcard / default playback audio meter
    Meter {
        /// Monitor duration in milliseconds (default 500ms)
        #[arg(long, default_value_t = 500)]
        duration_ms: u64,
    },
}

/// Loads a page from a UIA snapshot (`sr-cli snapshot` output) through the production
/// UIA-to-AOM mapping.
fn load_aom_from_path(path: &Path) -> Result<AomNode, Box<dyn std::error::Error>> {
    let snapshot = sr_platform::uia_snapshot::UiaSnapshot::load(path)?;
    Ok(sr_platform::uia_snapshot::to_aom(&snapshot.root))
}

#[cfg(windows)]
fn capture_snapshot(
    out: &str,
    api: &str,
    fixture_sha256: Option<String>,
    process_version: Option<String>,
) -> Result<(), String> {
    let (json, nodes, title, ms) = match api {
        "uia" => {
            let s = sr_platform::uia_snapshot::capture_foreground_document(
                fixture_sha256,
                process_version,
            )
            .map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?;
            (
                json,
                s.provenance.node_count,
                s.provenance.window_title,
                s.provenance.capture_ms,
            )
        }
        "ia2" => {
            let s = sr_platform::ia2_snapshot::capture_foreground_document(
                fixture_sha256,
                process_version,
            )
            .map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?;
            (
                json,
                s.provenance.node_count,
                s.provenance.window_title,
                s.provenance.capture_ms,
            )
        }
        other => {
            return Err(format!(
                "unknown accessibility API {other:?} (expected uia or ia2)"
            ))
        }
    };
    fs::write(out, json + "\n").map_err(|e| format!("{out:?}: {e}"))?;
    println!("Captured {nodes} {api} nodes from {title:?} in {ms} ms -> {out}");
    Ok(())
}

#[cfg(not(windows))]
fn capture_snapshot(_: &str, _: &str, _: Option<String>, _: Option<String>) -> Result<(), String> {
    Err("Accessibility snapshots are only available on Windows".to_owned())
}

/// Makes JAWS speech-history structure visible on one line: `\u{1d}`/`\u{1e}` role markers
/// become `‹ ›`, line breaks become ` | `, and no-break spaces become `·`.
fn visible(speech: &str) -> String {
    speech
        .replace("\r\n", "\n")
        .replace('\u{1d}', "‹")
        .replace('\u{1e}', "›")
        .replace('\u{a0}', "·")
        .replace('\n', " | ")
}

/// One traced event as a line: its time into the step, kind, window, and the focus chain's
/// innermost elements (`control type:class:name`).
fn trace_line(event: &sr_platform::event_trace::TraceEvent, start_ms: u128) -> String {
    let chain: Vec<String> = event
        .focus_chain
        .iter()
        .rev()
        .take(3)
        .rev()
        .map(|n| {
            format!(
                "{}:{}:{}",
                n.control_type,
                n.class_name.as_deref().unwrap_or(""),
                n.name.as_deref().unwrap_or("")
            )
        })
        .collect();
    let object = event
        .object
        .as_ref()
        .and_then(|o| o.name.as_deref())
        .map(|n| format!(" obj={n:?}"))
        .unwrap_or_default();
    format!(
        "+{}ms {:<21} {} {:?}{object} | {}",
        event.t_ms.saturating_sub(start_ms),
        event.event,
        event.window_class.as_deref().unwrap_or("-"),
        event.window_title.as_deref().unwrap_or(""),
        chain.join(" > ")
    )
}

fn print_evidence(evidence: &str, trial: usize, trace: bool) -> Result<(), String> {
    let path = if Path::new(evidence).exists() {
        Path::new(evidence).to_path_buf()
    } else {
        Path::new("corpus/evidence").join(format!("{evidence}-evidence.json"))
    };
    let text = fs::read_to_string(&path).map_err(|e| format!("{path:?}: {e}"))?;
    let json: serde_json::Value =
        serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
    let trials = json
        .get("trials")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();
    let chosen = trials
        .get(trial.saturating_sub(1))
        .ok_or_else(|| format!("{path:?} has {} trial(s)", trials.len()))?;
    let speech_of = |t: &serde_json::Value, id: &str| -> Option<String> {
        t.get("step_results")?
            .as_array()?
            .iter()
            .find(|s| s.get("step_id").and_then(|v| v.as_str()) == Some(id))?
            .pointer("/observation/speech_raw")?
            .as_str()
            .map(str::to_owned)
    };
    println!(
        "{} trial {trial}/{}  success={}",
        json.get("scenario_id")
            .and_then(|v| v.as_str())
            .unwrap_or("?"),
        trials.len(),
        chosen
            .get("trial_success")
            .unwrap_or(&serde_json::Value::Null)
    );
    // The trial's trace sits in corpus/traces, next to corpus/evidence.
    let events = if trace {
        let file = chosen
            .get("trace_file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("trial {trial} of {path:?} was not traced"))?;
        let corpus = path
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("corpus"));
        let (header, events) =
            sr_platform::event_trace::read_trace(corpus.join("traces").join(file))?;
        println!(
            "trace {file} (format {}, {} events)",
            header.trace_format_version,
            events.len()
        );
        events
    } else {
        Vec::new()
    };
    for step in chosen
        .get("step_results")
        .and_then(|s| s.as_array())
        .into_iter()
        .flatten()
    {
        let id = step.get("step_id").and_then(|v| v.as_str()).unwrap_or("?");
        let command = step
            .pointer("/action/command")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let speech = step
            .pointer("/observation/speech_raw")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!("{id:>4} {command:<16} {}", visible(speech));
        let mut variants: Vec<String> = Vec::new();
        for t in &trials {
            if let Some(s) = speech_of(t, id) {
                if s != speech && !variants.contains(&s) {
                    variants.push(s);
                }
            }
        }
        for v in variants {
            println!("{:>21} variant: {}", "", visible(&v));
        }
        let time = |key: &str| {
            step.get(key)
                .and_then(serde_json::Value::as_u64)
                .map(u128::from)
        };
        if let (Some(start), Some(end)) = (time("t_start_ms"), time("t_end_ms")) {
            for event in events.iter().filter(|e| (start..=end).contains(&e.t_ms)) {
                println!("{:>21} {}", "", trace_line(event, start));
            }
        }
    }
    Ok(())
}

/// The JAWS keyboard settings named on the command line (`--keyboard-layout`, `--jaws-key`).
fn keyboard_settings(
    layout: &str,
    jaws_key: Option<&str>,
) -> Result<sr_core::settings::KeyboardSettings, String> {
    use sr_core::settings::{JawsKey, KeyboardLayout, KeyboardSettings};
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
    if let Some(key) = jaws_key {
        settings.jaws_key = match key.to_ascii_lowercase().as_str() {
            "insert" => JawsKey::Insert,
            "capslock" => JawsKey::CapsLock,
            "scrolllock" => JawsKey::ScrollLock,
            other => {
                return Err(format!(
                    "unknown JAWS key {other:?} (insert, capslock, scrolllock)"
                ))
            }
        };
    }
    Ok(settings)
}

/// Logs to stderr at the RUST_LOG level, or, for `run --debug-log <file>`, everything from
/// the screen reader's crates at debug level to that file.
fn init_logging(cli: &Cli) {
    if let Some(Commands::Run {
        debug_log: Some(path),
        ..
    }) = &cli.command
    {
        match std::fs::File::create(path) {
            Ok(file) => {
                tracing_subscriber::fmt()
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false)
                    .with_env_filter(tracing_subscriber::EnvFilter::new(
                        "sr_core=debug,sr_platform=debug,sr_cli=debug",
                    ))
                    .init();
                return;
            }
            Err(e) => eprintln!("Cannot write debug log {path}: {e}"),
        }
    }
    tracing_subscriber::fmt::init();
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    init_logging(&cli);

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
            jaws_key,
        }) => {
            let keyboard =
                keyboard_settings(&keyboard_layout, jaws_key.as_deref()).unwrap_or_else(|e| {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                });
            let engine_type = TtsEngineType::from_str(&engine).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });

            let mut tts = TtsManager::new(engine_type).unwrap_or_else(|e| {
                eprintln!("Failed to initialize TTS engine: {}", e);
                std::process::exit(1);
            });
            if let Some(ref path) = speech_log {
                match SpeechLog::open(path) {
                    Ok(log) => tts = tts.with_speech_log(log),
                    Err(e) => {
                        eprintln!("Failed to open speech log {path:?}: {e}");
                        std::process::exit(1);
                    }
                }
            }

            let _ = tts.set_rate(rate);
            let _ = tts.set_volume(volume);
            if let Some(ref v) = voice {
                if let Err(e) = tts.set_voice(v) {
                    eprintln!("Warning: failed to set voice '{}': {}", v, e);
                }
            }

            println!("============================================================");
            println!("  Interactive Screen Reader Daemon");
            println!("  TTS Driver: {}", tts.engine_name());
            println!("============================================================");

            // Determine active buffer
            let (buffer, is_fixture) = if let Some(ref f) = fixture {
                let fixture_path = Path::new(f);
                match load_aom_from_path(fixture_path) {
                    Ok(aom) => {
                        println!("Loaded document fixture from {:?}", fixture_path);
                        (VirtualBuffer::from_aom(&aom), true)
                    }
                    Err(e) => {
                        eprintln!("Failed to load fixture: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                // The foreground browser page through IA2 (the pipeline parity verifies),
                // else any document xa11y finds.
                #[cfg(windows)]
                let page = sr_platform::actions::capture_foreground_page()
                    .or_else(|_| PlatformBridge::new().get_active_document(None));
                #[cfg(not(windows))]
                let page = PlatformBridge::new().get_active_document(None);
                match page {
                    Ok(aom) => {
                        println!("Attached to live active foreground document via OS accessibility APIs.");
                        (VirtualBuffer::from_aom(&aom), true)
                    }
                    Err(_) => {
                        println!("Operating in live native Windows navigation mode.");
                        let empty_doc = AomNode::new(0, Role::Document);
                        (VirtualBuffer::from_aom(&empty_doc), false)
                    }
                }
            };

            let mut cursor = VirtualCursor::new(buffer);
            cursor.move_to_start();

            // OS accessibility events and caret text both reach the engine through this channel.
            let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
            let mut engine = ScreenReaderEngine::new(cursor, Box::new(tts));
            // Enter on an edit focuses it in the page, Enter/Space on a control activates it.
            #[cfg(windows)]
            engine.set_actions(Box::new(sr_platform::actions::WindowsActions::start(Some(
                event_tx.clone(),
            ))));
            engine.settings.keyboard = keyboard;
            // Say System Time (Insert+F12) in the user's locale formats.
            #[cfg(windows)]
            engine.set_clock(Box::new(sr_platform::clock::WindowsClock));
            let engine = Arc::new(Mutex::new(engine));

            // Announce startup
            if let Ok(mut eng) = engine.lock() {
                if is_fixture {
                    eng.app_mode = sr_core::engine::AppMode::VirtualBuffer;
                    let doc_name = eng.cursor.buffer.document_title().unwrap_or("Document");
                    let welcome = format!("{}  \u{001d} Virtual PC Cursor \u{001e}", doc_name);
                    println!(
                        "[Screen Reader] {}",
                        welcome.replace(['\u{001d}', '\u{001e}'], "")
                    );
                    eng.speech.speak(&welcome, true);
                } else {
                    eng.app_mode = sr_core::engine::AppMode::NativeControl;
                    println!("[Screen Reader] Ready in live native Windows navigation mode.");
                }
            }

            // Install low-level keyboard hook
            let mut hook = create_keyboard_hook();
            let engine_cb = Arc::clone(&engine);

            if let Err(e) = hook.start(Box::new(move |event| {
                // Windows silently removes a low-level keyboard hook that once exceeds
                // LowLevelHooksTimeout, so the hook never waits long for the engine: a key
                // that finds it busy for 150 ms passes to the application.
                let deadline = std::time::Instant::now() + std::time::Duration::from_millis(150);
                let mut guard = engine_cb.try_lock();
                while matches!(guard, Err(std::sync::TryLockError::WouldBlock))
                    && std::time::Instant::now() < deadline
                {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    guard = engine_cb.try_lock();
                }
                if guard.is_err() {
                    tracing::warn!(key = ?event.key, "engine busy: key passed through");
                }
                if let Ok(mut eng) = guard {
                    let action = eng.handle_key_event(&event);
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
                            "key"
                        );
                    }
                    action
                } else {
                    KeyAction::PassThrough
                }
            })) {
                eprintln!("Failed to install keyboard hook: {}", e);
                std::process::exit(1);
            }

            // Start OS-level accessibility event listener
            let mut event_listener = create_event_listener();
            if let Err(e) = event_listener.start(event_tx) {
                tracing::warn!("Could not start OS accessibility event listener: {}", e);
            } else {
                let engine_events = Arc::clone(&engine);
                tokio::spawn(async move {
                    let idle = std::time::Duration::from_millis(150);
                    loop {
                        let event = match tokio::time::timeout(idle, event_rx.recv()).await {
                            Ok(Some(event)) => Some(event),
                            Ok(None) => break,
                            // Quiet for a moment: speech JAWS defers until no event supersedes it.
                            Err(_) => None,
                        };
                        // The engine lock is released before printing: a console in QuickEdit
                        // selection blocks writes, which must not hold the keyboard hook up.
                        let spoken = {
                            let Ok(mut eng) = engine_events.lock() else {
                                continue;
                            };
                            match &event {
                                Some(event) => eng.handle_accessibility_event(event),
                                None => eng.on_idle(),
                            }
                        };
                        if let Some(msg) = spoken {
                            // One line per utterance, so the debug log stays line-oriented.
                            let msg = msg
                                .replace(['\u{001d}', '\u{001e}'], "")
                                .replace('\n', " | ");
                            tracing::debug!(%msg, "spoken for event");
                            println!("[Event Spoken] {msg}");
                        }
                    }
                });
            }

            println!("\nScreen Reader daemon is running.");
            println!("Navigation shortcuts active:");
            println!("  - Arrow keys: Navigate lines / characters");
            println!("  - Ctrl+Left/Right: Navigate words");
            println!("  - Ctrl+Home / Ctrl+End: Document start / end");
            println!("  - H / Shift+H: Next / prior heading");
            println!("  - 1..6: Next heading by level");
            println!("  - K / U: Next link");
            println!("  - T / Shift+T: Next / prior table");
            println!("  - Tab / Shift+Tab: Interactive controls");
            println!("  - Enter: Forms mode on edits");
            println!("  - Esc: Exit forms mode to Virtual PC Cursor");
            println!("  - Ctrl: Silence speech");
            println!("Press Ctrl+C to stop the daemon.\n");

            // Handle clean exit via Ctrl+C
            if let Err(e) = tokio::signal::ctrl_c().await {
                eprintln!("Error waiting for shutdown signal: {}", e);
            }

            println!("\nShutting down screen reader daemon...");
            let _ = hook.stop();
            let _ = event_listener.stop();
            if let Ok(mut eng) = engine.lock() {
                eng.speech.speak("Screen reader exiting.", true);
            }
            println!("Screen reader stopped cleanly.");
        }
        Some(Commands::Speak {
            text,
            engine,
            rate,
            volume,
            measure_latency,
            after_long_ms,
        }) => {
            let engine_type = TtsEngineType::from_str(&engine).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });

            let mut tts = TtsManager::new(engine_type).unwrap_or_else(|e| {
                eprintln!("Failed to initialize TTS engine: {}", e);
                std::process::exit(1);
            });

            let _ = tts.set_rate(rate);
            let _ = tts.set_volume(volume);

            println!(
                "[TTS: {}] Speaking: \"{}\"",
                tts.engine_name(),
                text.replace(['\u{001d}', '\u{001e}'], "")
            );

            if let Some(ms) = after_long_ms {
                let long = "The Riverside City Library will stay open until nine in the evening on weekdays starting in April, the board announced on Tuesday. ".repeat(25);
                let _ = tts.speak(&long, true);
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
            let started = std::time::Instant::now();
            if let Err(e) = tts.speak(&text, true) {
                eprintln!("Speech error: {}", e);
                std::process::exit(1);
            }
            if measure_latency {
                match sr_platform::audio::AudioEndpointMeter::time_to_sound(
                    started,
                    0.01,
                    std::time::Duration::from_secs(5),
                ) {
                    Ok(Some(latency)) => println!("latency_ms={}", latency.as_millis()),
                    Ok(None) => println!("latency_ms=none (no sound within 5 s)"),
                    Err(e) => eprintln!("cannot meter the output: {e}"),
                }
            }

            // Wait for audio rendering to finish
            let _ = tts.wait_until_done(15000);
        }
        Some(Commands::Read {
            fixture,
            engine,
            mode,
        }) => {
            let fixture_path = Path::new(&fixture);
            let aom = load_aom_from_path(fixture_path).unwrap_or_else(|e| {
                eprintln!("Failed to load fixture AOM: {}", e);
                std::process::exit(1);
            });

            let buffer = VirtualBuffer::from_aom(&aom);
            let mut cursor = VirtualCursor::new(buffer);
            cursor.move_to_start();

            let speech_text = match mode.as_str() {
                "say-all" => cursor.say_all(),
                "line" => cursor
                    .move_next_line()
                    .unwrap_or_else(|| cursor.current_line()),
                "table" => cursor.move_next_table().unwrap_or_default(),
                _ => cursor.say_all(),
            };

            println!(
                "=== Screen Reader Virtual Speech Output ===\n{}\n===========================================",
                speech_text
            );

            let engine_type = TtsEngineType::from_str(&engine).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });

            let mut tts = TtsManager::new(engine_type).unwrap_or_else(|e| {
                eprintln!("Failed to initialize TTS engine: {}", e);
                std::process::exit(1);
            });

            println!("[TTS: {}] Speaking fixture output...", tts.engine_name());
            if let Err(e) = tts.speak(&speech_text, true) {
                eprintln!("Speech error: {}", e);
                std::process::exit(1);
            }

            let _ = tts.wait_until_done(30000);
        }
        Some(Commands::Voices { engine }) => {
            let engine_type = TtsEngineType::from_str(&engine).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });

            let tts = TtsManager::new(engine_type).unwrap_or_else(|e| {
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
        Some(Commands::History { path }) => match SpeechLog::read(&path) {
            Ok(entries) => {
                for e in entries {
                    println!(
                        "[{} ms] {:?} interrupt={} {:?}",
                        e.t_ms, e.kind, e.interrupt, e.text
                    );
                }
            }
            Err(e) => {
                eprintln!("Failed to read speech log {path:?}: {e}");
                std::process::exit(1);
            }
        },
        Some(Commands::Meter { duration_ms }) => {
            println!(
                "Sampling hardware soundcard / default playback endpoint for {} ms...",
                duration_ms
            );
            let report = sr_platform::AudioEndpointMeter::monitor_activity(
                std::time::Duration::from_millis(duration_ms),
                std::time::Duration::from_millis(25),
                0.001,
            );
            match report {
                Ok(rep) => {
                    println!("Audio Output Report:");
                    println!("  - Detected Sound: {}", rep.detected_audio);
                    println!("  - Max Peak Amplitude: {:.4}", rep.max_peak);
                    println!("  - Average Active Peak: {:.4}", rep.average_peak);
                    println!(
                        "  - Active Audio Samples: {} / {}",
                        rep.active_samples, rep.total_samples
                    );
                }
                Err(e) => {
                    eprintln!("Failed to query audio endpoint: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Parity {
            corpus_dir,
            all,
            write_baseline,
            api,
            spoken,
        }) => {
            use sr_corpus::snapshots::{Api, PARITY_API};
            let dir = Path::new(&corpus_dir);
            let api = match api.as_deref().map(Api::parse) {
                None => PARITY_API,
                Some(Some(api)) => api,
                Some(None) => {
                    eprintln!("--api must be uia or ia2");
                    std::process::exit(2);
                }
            };
            if write_baseline && api != PARITY_API {
                eprintln!("The baseline ratchets {PARITY_API:?} parity; drop --api to write it.");
                std::process::exit(2);
            }
            let report = sr_corpus::parity::run_with(dir, api);
            for step in report.steps.iter().filter(|s| all || !s.matched) {
                println!(
                    "[{}] {}  ({})",
                    if step.matched { "MATCH" } else { "DIFF " },
                    step.key,
                    step.command
                );
                println!("    ours: {:?}", sr_corpus::parity::normalize(&step.engine));
                if !step.matched {
                    for v in &step.jaws_variants {
                        println!("    jaws: {:?}", sr_corpus::parity::normalize(v));
                    }
                }
            }
            if spoken {
                for step in &report.steps {
                    let Some(s) = step.spoken.as_ref().filter(|s| all || !s.matched) else {
                        continue;
                    };
                    let tag = if s.matched { "MATCH" } else { "DIFF " };
                    println!("[{tag}] spoken {}  ({})", step.key, step.command);
                    println!("    ours: {:?}", sr_corpus::parity::normalize(&s.engine));
                    if !s.matched {
                        for v in &s.jaws_variants {
                            println!("    jaws: {:?}", sr_corpus::parity::normalize(v));
                        }
                    }
                }
            }
            for e in &report.errors {
                println!("[ERROR] {e}");
            }
            let (spoken_matched, spoken_total) = report.spoken();
            println!(
                "JAWS spoken parity: {spoken_matched}/{spoken_total} steps match exactly (evidence recorded through the speech recorder voice)"
            );
            println!(
                "JAWS parity ({api:?}): {}/{} steps match exactly ({} scenarios could not run)",
                report.matched(),
                report.steps.len(),
                report.errors.len()
            );
            if write_baseline {
                if let Err(e) = sr_corpus::parity::write_baseline(dir, &report) {
                    eprintln!("Failed to write baseline: {e}");
                    std::process::exit(1);
                }
                println!("Wrote {:?}", dir.join(sr_corpus::parity::BASELINE_FILE));
            }
        }
        Some(Commands::Act { name, action }) => {
            #[cfg(windows)]
            {
                fn find<'a>(node: &'a AomNode, name: &str) -> Option<&'a AomNode> {
                    if node.name.as_deref().map(str::trim) == Some(name)
                        && node.role != Role::Text
                        && sr_platform::actions::unique_id(node).is_some()
                    {
                        return Some(node);
                    }
                    node.children.iter().find_map(|c| find(c, name))
                }
                let page = match sr_platform::actions::capture_foreground_page() {
                    Ok(page) => page,
                    Err(e) => {
                        eprintln!("No foreground page: {e}");
                        std::process::exit(1);
                    }
                };
                let Some(target) = find(&page, &name) else {
                    eprintln!("No control named {name:?} with an IA2 uniqueID in the page");
                    std::process::exit(1);
                };
                let requested = match action.as_str() {
                    "focus" => sr_platform::actions::perform_now(target, false),
                    "activate" => sr_platform::actions::perform_now(target, true),
                    other => {
                        eprintln!("--action must be focus or activate, not {other:?}");
                        std::process::exit(2);
                    }
                };
                std::thread::sleep(std::time::Duration::from_millis(500));
                let focused = sr_platform::uia_snapshot::focused_element_summary();
                println!(
                    "{action} {name:?} ({:?}): {requested:?}; keyboard focus now: {focused}",
                    target.role
                );
            }
            #[cfg(not(windows))]
            {
                let _ = (name, action);
                eprintln!("act is only available on Windows");
                std::process::exit(1);
            }
        }
        Some(Commands::Caret { unit }) => {
            use sr_core::editing::EditKey;
            let key = match unit.as_str() {
                "char" => EditKey::Character,
                "word" => EditKey::Word,
                "line" => EditKey::Line,
                other => {
                    eprintln!("--unit must be char, word or line, not {other:?}");
                    std::process::exit(2);
                }
            };
            #[cfg(windows)]
            {
                let context = sr_platform::actions::read_text_now();
                let spoken = context
                    .as_ref()
                    .and_then(|c| sr_core::editing::speech(key, None, c, ""));
                println!("{key:?}: {spoken:?} (field: {context:?})");
            }
            #[cfg(not(windows))]
            {
                let _ = key;
                eprintln!("caret is only available on Windows");
            }
        }
        Some(Commands::Trace {
            out,
            stop_file,
            max_ms,
        }) => {
            #[cfg(windows)]
            match sr_platform::event_trace::run_trace(
                Path::new(&out),
                stop_file.as_deref().map(Path::new),
                max_ms,
            ) {
                Ok(n) => println!("Recorded {n} events -> {out}"),
                Err(e) => {
                    eprintln!("Trace failed: {e}");
                    std::process::exit(1);
                }
            }
            #[cfg(not(windows))]
            {
                let _ = (out, stop_file, max_ms);
                eprintln!("Event traces are only available on Windows");
                std::process::exit(1);
            }
        }
        Some(Commands::Evidence {
            evidence,
            trial,
            trace,
        }) => {
            if let Err(e) = print_evidence(&evidence, trial, trace) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        Some(Commands::WaitSilence {
            onset_ms,
            quiet_ms,
            max_ms,
            threshold,
        }) => {
            use std::time::Duration;
            match sr_platform::AudioEndpointMeter::wait_for_silence(
                Duration::from_millis(onset_ms),
                Duration::from_millis(quiet_ms),
                Duration::from_millis(max_ms),
                threshold,
            ) {
                Ok(report) => println!("{}", serde_json::to_string(&report).unwrap_or_default()),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Invariants {
            corpus_dir,
            write_baseline,
        }) => {
            let dir = Path::new(&corpus_dir);
            let results = sr_corpus::invariants::run_all(dir).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let mut total = 0;
            for (snapshot, violations) in &results {
                println!("{snapshot}: {} violation(s)", violations.len());
                for v in violations {
                    println!("    {v}");
                }
                total += violations.len();
            }
            println!("Buffer invariants: {total} violation(s)");
            if write_baseline {
                if let Err(e) = sr_corpus::invariants::write_baseline(dir, &results) {
                    eprintln!("Failed to write baseline: {e}");
                    std::process::exit(1);
                }
                println!("Wrote {:?}", dir.join(sr_corpus::invariants::BASELINE_FILE));
            }
        }
        Some(Commands::Snapshot {
            out,
            api,
            fixture,
            process_version,
            delay_ms,
        }) => {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            let fixture_sha256 = match fixture {
                Some(path) => match fs::read(&path) {
                    Ok(bytes) => Some(sr_corpus::manifest::sha256_hex(&bytes)),
                    Err(e) => {
                        eprintln!("Failed to read fixture {path:?}: {e}");
                        std::process::exit(1);
                    }
                },
                None => None,
            };
            if let Err(e) = capture_snapshot(&out, &api, fixture_sha256, process_version) {
                eprintln!("Snapshot failed: {e}");
                std::process::exit(1);
            }
        }
        Some(Commands::CorpusManifest { corpus_dir, write }) => {
            let dir = Path::new(&corpus_dir);
            let result = if write {
                sr_corpus::manifest::write(dir)
            } else {
                sr_corpus::manifest::check(dir)
            };
            match result {
                Ok(()) if write => {
                    println!("Wrote {:?}", dir.join(sr_corpus::manifest::MANIFEST_FILE))
                }
                Ok(()) => println!("Corpus manifest is current."),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("No command specified. Use `sr-cli --help` for available options.");
        }
    }
}
