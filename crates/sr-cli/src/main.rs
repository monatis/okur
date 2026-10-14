use clap::{Parser, Subcommand};
use sr_core::{AomNode, Role, ScreenReaderEngine, VirtualBuffer, VirtualCursor};
use sr_corpus::DifferentialRunner;
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
        /// TTS synthesizer engine: auto (default), sapi5, or no-speech
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

        /// Path to fixture file to load (optional; if not provided, queries active application)
        #[arg(short, long)]
        fixture: Option<String>,

        /// Record every utterance as JSON Lines to this file (off by default)
        #[arg(long)]
        speech_log: Option<String>,
    },
    /// Run corpus scenarios against their authored assertions (not yet a JAWS parity comparison)
    Test {
        #[arg(short, long, default_value = "corpus")]
        corpus_dir: String,

        /// Scenario range or specific ID (e.g. "01..10", "1..10", or "01-doc-say-all")
        #[arg(short, long)]
        scenarios: Option<String>,

        /// TTS output driver during tests: "no-speech" (default) or "sapi5"
        #[arg(long, default_value = "no-speech")]
        tts: String,
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
    },
    /// Print an evidence file's steps and JAWS transcripts readably
    Evidence {
        /// Evidence file path, or a scenario id looked up in corpus/evidence
        evidence: String,

        /// Trial to print (1-based); variants across all trials are always listed
        #[arg(long, default_value_t = 1)]
        trial: usize,
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

        /// TTS synthesizer engine: auto (default), sapi5, or no-speech
        #[arg(short, long, default_value = "auto")]
        engine: String,

        /// Speech rate (-10 to 10)
        #[arg(short, long, default_value_t = 0)]
        rate: i32,

        /// Volume (0 to 100)
        #[arg(short, long, default_value_t = 100)]
        volume: u16,
    },
    /// Load and read a document fixture aloud using the virtual cursor
    Read {
        /// Path to fixture file (HTML or AOM JSON)
        fixture: String,

        /// TTS synthesizer engine: auto (default), sapi5, or no-speech
        #[arg(short, long, default_value = "auto")]
        engine: String,

        /// Reading command: say-all (default), line, or table
        #[arg(short, long, default_value = "say-all")]
        mode: String,
    },
    /// List available voices from the synthesizer
    Voices {
        /// TTS synthesizer engine: auto (default), sapi5, or no-speech
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

fn parse_range(range_str: &str) -> Option<(u32, u32)> {
    let mut parts = range_str.split("..");
    let start = parts.next()?.trim().parse::<u32>().ok()?;
    let end = parts.next()?.trim().parse::<u32>().ok()?;
    if parts.next().is_none() {
        Some((start, end))
    } else {
        None
    }
}

fn load_aom_from_path(path: &Path) -> Result<AomNode, Box<dyn std::error::Error>> {
    let aom_path = if path.extension().and_then(|s| s.to_str()) == Some("json") {
        path.to_path_buf()
    } else {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("doc");
        let candidate = parent.join(format!("{}.aom.json", stem));
        if candidate.exists() {
            candidate
        } else {
            return Err(format!("No corresponding .aom.json found for {:?}", path).into());
        }
    };

    let content = fs::read_to_string(&aom_path)?;
    let clean = content.trim_start_matches('\u{feff}');
    let aom: AomNode = serde_json::from_str(clean)?;
    Ok(aom)
}

#[cfg(windows)]
fn capture_snapshot(
    out: &str,
    fixture_sha256: Option<String>,
    process_version: Option<String>,
) -> Result<(), String> {
    let snapshot =
        sr_platform::uia_snapshot::capture_foreground_document(fixture_sha256, process_version)
            .map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
    fs::write(out, json + "\n").map_err(|e| format!("{out:?}: {e}"))?;
    println!(
        "Captured {} nodes from {:?} in {} ms -> {}",
        snapshot.provenance.node_count,
        snapshot.provenance.window_title,
        snapshot.provenance.capture_ms,
        out
    );
    Ok(())
}

#[cfg(not(windows))]
fn capture_snapshot(_: &str, _: Option<String>, _: Option<String>) -> Result<(), String> {
    Err("UIA snapshots are only available on Windows".to_owned())
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

fn print_evidence(evidence: &str, trial: usize) -> Result<(), String> {
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
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run {
            engine,
            voice,
            rate,
            volume,
            fixture,
            speech_log,
        }) => {
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
                let bridge = PlatformBridge::new();
                match bridge.get_active_document(None) {
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

            let engine = Arc::new(Mutex::new(ScreenReaderEngine::new(cursor, Box::new(tts))));

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
                if let Ok(mut eng) = engine_cb.lock() {
                    eng.handle_key_event(&event)
                } else {
                    KeyAction::PassThrough
                }
            })) {
                eprintln!("Failed to install keyboard hook: {}", e);
                std::process::exit(1);
            }

            // Start OS-level accessibility event listener
            let mut event_listener = create_event_listener();
            let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
            if let Err(e) = event_listener.start(event_tx) {
                tracing::warn!("Could not start OS accessibility event listener: {}", e);
            } else {
                let engine_events = Arc::clone(&engine);
                tokio::spawn(async move {
                    while let Some(event) = event_rx.recv().await {
                        if let Ok(mut eng) = engine_events.lock() {
                            if let Some(msg) = eng.handle_accessibility_event(&event) {
                                println!(
                                    "[Event Spoken] {}",
                                    msg.replace(['\u{001d}', '\u{001e}'], "")
                                );
                            }
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

            if let Err(e) = tts.speak(&text, true) {
                eprintln!("Speech error: {}", e);
                std::process::exit(1);
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
        Some(Commands::Test {
            corpus_dir,
            scenarios,
            tts,
        }) => {
            let corpus_path = Path::new(&corpus_dir);
            if !corpus_path.exists() {
                eprintln!("Error: corpus directory not found at {:?}", corpus_path);
                std::process::exit(1);
            }

            let runner = DifferentialRunner::new(corpus_path);
            let mut total = 0;
            let mut passed = 0;
            let mut failed = 0;

            let engine_type = TtsEngineType::from_str(&tts).unwrap_or(TtsEngineType::NoSpeech);
            let mut tts_manager = TtsManager::new(engine_type).ok();

            println!("============================================================");
            println!("  Corpus Scenario Assertions (authored; not JAWS parity)     ");
            println!("  TTS Driver: {}", tts);
            println!("============================================================");

            let filter = scenarios.as_deref().unwrap_or("0..999");
            let results = if let Some((start, end)) = parse_range(filter) {
                runner.run_range(start, end)
            } else {
                vec![runner.run_scenario_by_id(filter)]
            };

            for res in results {
                total += 1;
                match res {
                    Ok(scenario_res) => {
                        let status_str = if scenario_res.passed {
                            passed += 1;
                            "PASS"
                        } else {
                            failed += 1;
                            "FAIL"
                        };

                        println!(
                            "[{}] {} - {} ({} steps)",
                            status_str,
                            scenario_res.scenario_id,
                            scenario_res.scenario_name,
                            scenario_res.steps.len()
                        );

                        for step in &scenario_res.steps {
                            if let Some(ref mut mgr) = tts_manager {
                                if !step.emitted_speech.is_empty() {
                                    let _ = mgr.speak(&step.emitted_speech, true);
                                }
                            }

                            if !step.success {
                                eprintln!(
                                    "   Step [{}]: command='{}', speech='{}'",
                                    step.step_id, step.command, step.emitted_speech
                                );
                                for fail in &step.failed_assertions {
                                    eprintln!("     * {}", fail);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        failed += 1;
                        eprintln!("[ERROR] Execution failure: {}", e);
                    }
                }
            }

            println!("------------------------------------------------------------");
            println!(
                "Summary: {} total, {} passed, {} failed ({:.1}% parity)",
                total,
                passed,
                failed,
                if total > 0 {
                    (passed as f64 / total as f64) * 100.0
                } else {
                    0.0
                }
            );
            println!("============================================================");

            if failed > 0 {
                std::process::exit(1);
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
        }) => {
            let dir = Path::new(&corpus_dir);
            let report = sr_corpus::parity::run(dir);
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
            for e in &report.errors {
                println!("[ERROR] {e}");
            }
            println!(
                "JAWS parity: {}/{} steps match exactly ({} scenarios could not run)",
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
        Some(Commands::Evidence { evidence, trial }) => {
            if let Err(e) = print_evidence(&evidence, trial) {
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
            if let Err(e) = capture_snapshot(&out, fixture_sha256, process_version) {
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
