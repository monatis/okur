//! `okur-lab.exe`: Okur's verification harness, apart from the screen reader (`okur.exe`):
//! replay against JAWS's evidence (`parity`, `live-parity`, `invariants`, `corpus-manifest`),
//! the reference machine's capture tools (`trace`, `snapshot`, `wait-silence`, `focus`,
//! `keys`, `jaws-history`, `front`, `meter`) and debugging views (`evidence`, `lines`,
//! `window-text`, `history`, `latency`). What sends keys to other programs, reads the
//! clipboard and traces every window lives here, never in the program users run.

use clap::{Parser, Subcommand};
use okur::run::load_aom_from_path;
use okur_core::{AomNode, Role, VirtualBuffer, VirtualCursor};
use okur_platform::{SpeechLog, TtsEngineType, TtsManager};
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(
    name = "okur-lab",
    about = "Okur's verification harness: replay against JAWS's evidence, and the reference machine's capture tools"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
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

        /// Replay only the scenarios whose ids start with this (repeatable): with RUST_LOG,
        /// one scenario's engine debug log. Not with --write-baseline.
        #[arg(long)]
        only: Vec<String>,
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
    /// List the taskbar's notification area icons as the system tray list (Insert+F11) gives
    /// them, with the points their clicks go to (Windows)
    Tray {
        /// Print the taskbar's whole control tree instead, to find its icons
        #[arg(long)]
        dump: bool,
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

        /// Also record the page in front's objects shown, hidden and reordered, with their
        /// object ids (Chromium pages; nothing is read for them)
        #[arg(long)]
        page_changes: bool,

        /// Log each event's read, with its stages' times, to this file (for `okur-lab latency`
        /// and the performance review's measurements)
        #[arg(long)]
        debug_log: Option<String>,
    },
    /// Send key chords to the application in front, each only while it is one of --allow:
    /// for live checks on a machine the user lets the agent drive ("Insert+Alt+W",
    /// "down:Alt", "up:Alt", "wait:500")
    Keys {
        /// Process names allowed in front (comma-separated, without .exe)
        #[arg(long, value_delimiter = ',', required = true)]
        allow: Vec<String>,

        /// Text the foreground window's title must contain (case-insensitive), for an
        /// application whose process also owns the user's windows (Windows Terminal)
        #[arg(long)]
        title: Option<String>,

        /// Steps, in order
        #[arg(required = true)]
        steps: Vec<String>,
    },
    /// Bring the visible window whose title contains this text (case-insensitive) to the
    /// front, for live checks: prints its whole title (Windows)
    Front {
        /// Text the window's title contains
        title: String,
    },
    /// Print the element UI Automation says has the keyboard focus, as one JSON line: its
    /// name, control type, class, and whether it is in a document (a web page's). The harness
    /// checks a web trial starts in its page (Windows)
    Focus,
    /// Watch a window for UI Automation notifications, registered several ways at once, and
    /// print which registration heard what (Windows; research, never evidence)
    ProbeNotifications {
        /// The window's title
        #[arg(long)]
        window: String,
        /// How long to watch, in milliseconds
        #[arg(long, default_value_t = 15000)]
        millis: u64,
    },
    /// Print whether Windows lets this program read its list of notifications, how many toasts
    /// it keeps, and those whose text contains `--only` (Windows; research, never evidence)
    ListNotifications {
        /// Text a listed toast's application or text contains for it to be printed; the
        /// others are only counted, so that no one's own notifications are printed
        #[arg(long)]
        only: String,
    },
    /// Print JAWS's speech history (through FSAPI; the clipboard's text is put back)
    JawsHistory {
        /// Empty the history after reading it
        #[arg(long)]
        clear: bool,
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
    /// Print the virtual buffer a page snapshot builds, line by line: offset, node, role,
    /// the line's text and its speech (for debugging replay and quick navigation)
    Lines {
        /// Snapshot file (corpus/snapshots/*.chrome.ia2.json or .uia.json)
        snapshot: String,

        /// Only lines whose text contains this
        #[arg(long)]
        grep: Option<String>,

        /// Also print each line's spans: offsets, node, role and text
        #[arg(long)]
        spans: bool,
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
        /// UIA snapshot to read (okur snapshot output, e.g. corpus/snapshots/*.json)
        fixture: String,

        /// TTS synthesizer engine: auto (default: OneCore, else SAPI 5, else none), onecore,
        /// sapi5, or no-speech
        #[arg(short, long, default_value = "auto")]
        engine: String,

        /// Reading command: say-all (default), line, or table
        #[arg(short, long, default_value = "say-all")]
        mode: String,
    },
    /// Print the Virtual Viewer's lines for the window in front, as Virtualize Window builds
    /// them (Windows): for comparing with JAWS's viewer
    WindowText {
        /// The focused control only, as Virtualize Current Control reads it
        #[arg(long)]
        control: bool,
    },
    /// Print the Okur cursor's lines for the window in front, as its experimental screen text
    /// reads them (Windows): the text UI Automation places on it, with --recognize the words
    /// Windows' OCR reads in it too. Research, never evidence
    ScreenText {
        /// Recognize the window's pixels too (Insert+Space, O, W)
        #[arg(long)]
        recognize: bool,
        /// Print each line's pieces: their source and place
        #[arg(long)]
        pieces: bool,
        /// Append the window's text, as JSON, to this file instead of printing its lines: the
        /// recorder's snapshot after a step of a scenario with "trace_screen" (replay gives it
        /// to Okur's screen reads)
        #[arg(long)]
        append_to: Option<String>,
        /// With --append-to, the step the snapshot is taken after
        #[arg(long, default_value = "")]
        step: String,
    },
    /// Print a speech log recorded with `run --speech-log`
    History {
        /// Path of the speech log file
        path: String,
    },
    /// Summarize a live run's responsiveness from its debug log (`run --debug-log`): keys,
    /// event reads and handling, page captures and text reads, as percentiles in ms
    Latency {
        /// Path of the debug log
        path: String,
    },
    /// Compare Okur's live run of a scenario (`run-scenario.ps1 -Reader okur` on the reference
    /// machine) with JAWS's evidence for it, step by step: what replay cannot measure, such
    /// as Okur's own dialogs
    LiveParity {
        /// The run file
        run: String,
        #[arg(short, long, default_value = "corpus")]
        corpus_dir: String,
        /// Print every step, not only mismatches
        #[arg(long)]
        all: bool,
    },
    /// Write the reference machine's settings (old-pc's JAWS configuration) as a settings
    /// folder, for `run --settings` in a live run there
    ReferenceSettings {
        /// The folder to write them in
        path: String,
    },
    /// Query hardware soundcard / default playback audio meter
    Meter {
        /// Monitor duration in milliseconds (default 500ms)
        #[arg(long, default_value_t = 500)]
        duration_ms: u64,
    },
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
            let s = okur_platform::uia_snapshot::capture_foreground_document(
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
            let s = okur_platform::ia2_snapshot::capture_foreground_document(
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
fn trace_line(event: &okur_platform::event_trace::TraceEvent, start_ms: u128) -> String {
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
            okur_platform::event_trace::read_trace(corpus.join("traces").join(file))?;
        println!(
            "trace {file} (format {}, {} events)",
            header.trace_format_version,
            events.len()
        );
        events
    } else {
        Vec::new()
    };
    // As replay places them: from a tick before the step's key, after the step before it.
    let mut previous_end: Option<u128> = None;
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
            let from = start.saturating_sub(okur_corpus::parity::TICK_MS);
            let from = previous_end.map_or(from, |e| from.max(e + 1));
            for event in events.iter().filter(|e| (from..=end).contains(&e.t_ms)) {
                println!("{:>21} {}", "", trace_line(event, start));
            }
            previous_end = Some(end);
        }
    }
    Ok(())
}

/// Logs to stderr at the RUST_LOG level, or, for `run --debug-log <file>` and
/// `trace --debug-log <file>`, everything from the screen reader's crates at debug level to
/// that file.
fn init_logging(cli: &Cli) {
    let debug_log = match &cli.command {
        Some(Commands::Trace { debug_log, .. }) => debug_log.as_deref(),
        _ => None,
    };
    okur::run::init_logging(debug_log);
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    init_logging(&cli);
    okur_platform::init();

    match cli.command {
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
                match okur_platform::audio::AudioEndpointMeter::time_to_sound(
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
                "=== Okur Virtual Speech Output ===\n{}\n===========================================",
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
        Some(Commands::WindowText { control }) => {
            #[cfg(windows)]
            match okur_platform::window_text::foreground_lines(control) {
                Ok(lines) => {
                    for line in lines {
                        println!("{line}");
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            #[cfg(not(windows))]
            {
                let _ = control;
                eprintln!("window-text is only available on Windows");
                std::process::exit(1);
            }
        }
        Some(Commands::ScreenText {
            recognize,
            pieces,
            append_to,
            step,
        }) => {
            #[cfg(windows)]
            match okur_platform::window_text::foreground_screen(recognize) {
                Ok(model) if append_to.is_some() => {
                    let t_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or_default();
                    let snapshot = serde_json::json!({
                        "step": step,
                        "t_ms": t_ms,
                        "accessible": model.accessible,
                        "recognized": model.recognized,
                    });
                    let path = append_to.unwrap_or_default();
                    let written = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&path)
                        .and_then(|mut file| {
                            use std::io::Write as _;
                            writeln!(file, "{snapshot}")
                        });
                    if let Err(e) = written {
                        eprintln!("{path}: {e}");
                        std::process::exit(1);
                    }
                }
                Ok(model) => {
                    for line in model.lines() {
                        println!("{}", line.text);
                        if pieces {
                            let chars: Vec<char> = line.text.chars().collect();
                            for piece in &line.pieces {
                                let text: String =
                                    chars.iter().skip(piece.start).take(piece.chars).collect();
                                println!(
                                    "    {:?} {},{} {},{}: {text}",
                                    piece.source, piece.left, piece.top, piece.right, piece.bottom
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            #[cfg(not(windows))]
            {
                let _ = (recognize, pieces, append_to, step);
                eprintln!("screen-text is only available on Windows");
                std::process::exit(1);
            }
        }
        Some(Commands::Latency { path }) => match fs::read_to_string(&path) {
            Ok(log) => {
                let categories = okur_corpus::latency::summarize(&log);
                if categories.is_empty() {
                    println!("No timings in {path:?} (was it written by `run --debug-log`?)");
                } else {
                    print!("{}", okur_corpus::latency::render(&categories));
                }
            }
            Err(e) => {
                eprintln!("Failed to read debug log {path:?}: {e}");
                std::process::exit(1);
            }
        },
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
            let report = okur_platform::AudioEndpointMeter::monitor_activity(
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
        Some(Commands::LiveParity {
            run,
            corpus_dir,
            all,
        }) => {
            let read = |path: &Path| -> serde_json::Value {
                let text = fs::read_to_string(path).unwrap_or_else(|e| {
                    eprintln!("Error: {path:?}: {e}");
                    std::process::exit(1);
                });
                serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap_or_else(|e| {
                    eprintln!("Error: {path:?}: {e}");
                    std::process::exit(1);
                })
            };
            let run = read(Path::new(&run));
            let scenario = run
                .get("scenario_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let evidence = read(
                &Path::new(&corpus_dir)
                    .join("evidence")
                    .join(format!("{scenario}-evidence.json")),
            );
            match okur_corpus::live::compare(&run, &evidence) {
                Ok(steps) => {
                    for step in steps.iter().filter(|s| all || !(s.matched || s.accepted)) {
                        let tag = match (step.matched, step.accepted) {
                            (true, _) => "MATCH",
                            (false, true) => "ACCEPTED",
                            (false, false) => "DIFF ",
                        };
                        let unsettled = if step.settled { "" } else { "  unsettled" };
                        println!("[{tag}] {}  ({}){unsettled}", step.key, step.command);
                        println!("    okur: {:?}", okur_corpus::parity::normalize(&step.okur));
                        if !step.matched {
                            for v in &step.jaws_variants {
                                println!("    jaws: {:?}", okur_corpus::parity::normalize(v));
                            }
                        }
                    }
                    // What each sent its synthesizer, when both went through the recorder.
                    let spoken: Vec<_> = steps
                        .iter()
                        .filter(|s| !s.accepted)
                        .filter_map(|s| s.spoken.as_ref().map(|spoken| (s, spoken)))
                        .collect();
                    for (step, said) in spoken.iter().filter(|(_, s)| all || !s.matched) {
                        let tag = if said.matched { "MATCH" } else { "DIFF " };
                        println!("[{tag}] spoken {}  ({})", step.key, step.command);
                        println!(
                            "    okur: {:?}",
                            okur_corpus::parity::normalize(&said.engine)
                        );
                        if !said.matched {
                            for v in &said.jaws_variants {
                                println!("    jaws: {:?}", okur_corpus::parity::normalize(v));
                            }
                        }
                    }
                    let matched = steps.iter().filter(|s| s.matched).count();
                    let accepted = steps.iter().filter(|s| s.accepted).count();
                    println!(
                        "Okur live: {matched}/{} steps match JAWS exactly; {accepted} accepted \
                         divergences not counted",
                        steps.len() - accepted
                    );
                    if !spoken.is_empty() {
                        let said = spoken.iter().filter(|(_, s)| s.matched).count();
                        println!(
                            "Okur live, spoken: {said}/{} steps send the synthesizer what JAWS \
                             sent its own",
                            spoken.len()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::ReferenceSettings { path }) => {
            use okur_core::settings::SettingsStore;
            let layers = okur_core::settings_layers::SettingsLayers::with_global(
                &okur_core::settings::Settings::reference_machine(),
            );
            let folder = okur_platform::settings_file::SettingsFolder::at(&path);
            folder.save(&layers);
            if folder.load() != layers {
                eprintln!("Error: {path}: the settings were not written");
                std::process::exit(1);
            }
            println!("Wrote {path:?}");
        }
        Some(Commands::Parity {
            corpus_dir,
            all,
            write_baseline,
            api,
            spoken,
            only,
        }) => {
            use okur_corpus::snapshots::{Api, PARITY_API};
            let dir = Path::new(&corpus_dir);
            let api = match api.as_deref().map(Api::parse) {
                None => PARITY_API,
                Some(Some(api)) => api,
                Some(None) => {
                    eprintln!("--api must be uia or ia2");
                    std::process::exit(2);
                }
            };
            if write_baseline && !only.is_empty() {
                eprintln!("The baseline covers every scenario; drop --only to write it.");
                std::process::exit(2);
            }
            if write_baseline && api != PARITY_API {
                eprintln!("The baseline ratchets {PARITY_API:?} parity; drop --api to write it.");
                std::process::exit(2);
            }
            let report = okur_corpus::parity::run_matching(dir, api, &only);
            // Steps without JAWS's history (Keyboard Help's) are compared on the spoken layer.
            for step in report
                .steps
                .iter()
                .filter(|s| s.history && (all || !(s.matched || s.accepted)))
            {
                let tag = match (step.matched, step.accepted) {
                    (true, _) => "MATCH",
                    (false, true) => "ACCEPTED",
                    (false, false) => "DIFF ",
                };
                println!("[{tag}] {}  ({})", step.key, step.command);
                println!(
                    "    ours: {:?}",
                    okur_corpus::parity::normalize(&step.engine)
                );
                if !step.matched {
                    for v in &step.jaws_variants {
                        println!("    jaws: {:?}", okur_corpus::parity::normalize(v));
                    }
                }
            }
            if spoken {
                for step in &report.steps {
                    let Some(s) = step
                        .spoken
                        .as_ref()
                        .filter(|s| all || !(s.matched || step.accepted))
                    else {
                        continue;
                    };
                    let tag = if s.matched { "MATCH" } else { "DIFF " };
                    println!("[{tag}] spoken {}  ({})", step.key, step.command);
                    println!("    ours: {:?}", okur_corpus::parity::normalize(&s.engine));
                    if !s.matched {
                        for v in &s.jaws_variants {
                            println!("    jaws: {:?}", okur_corpus::parity::normalize(v));
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
                "JAWS parity ({api:?}): {}/{} steps match exactly; {} accepted divergences not \
                 counted ({} scenarios could not run)",
                report.matched(),
                report.compared(),
                report.accepted(),
                report.errors.len()
            );
            if write_baseline {
                if let Err(e) = okur_corpus::parity::write_baseline(dir, &report) {
                    eprintln!("Failed to write baseline: {e}");
                    std::process::exit(1);
                }
                println!("Wrote {:?}", dir.join(okur_corpus::parity::BASELINE_FILE));
            }
        }
        Some(Commands::Act { name, action }) => {
            #[cfg(windows)]
            {
                fn find<'a>(node: &'a AomNode, name: &str) -> Option<&'a AomNode> {
                    if node.name.as_deref().map(str::trim) == Some(name)
                        && node.role != Role::Text
                        && okur_platform::actions::unique_id(node).is_some()
                    {
                        return Some(node);
                    }
                    node.children.iter().find_map(|c| find(c, name))
                }
                let page = match okur_platform::actions::capture_foreground_page() {
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
                    "focus" => okur_platform::actions::perform_now(target, false),
                    "activate" => okur_platform::actions::perform_now(target, true),
                    other => {
                        eprintln!("--action must be focus or activate, not {other:?}");
                        std::process::exit(2);
                    }
                };
                std::thread::sleep(std::time::Duration::from_millis(500));
                let focused = okur_platform::uia_snapshot::focused_element_summary();
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
            use okur_core::editing::EditKey;
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
                let context = okur_platform::actions::read_text_now();
                let spoken = context
                    .as_ref()
                    .and_then(|c| okur_core::editing::speech(key, None, c, ""));
                println!("{key:?}: {spoken:?} (field: {context:?})");
            }
            #[cfg(not(windows))]
            {
                let _ = key;
                eprintln!("caret is only available on Windows");
            }
        }
        Some(Commands::Tray { dump }) => {
            #[cfg(windows)]
            if dump {
                for line in okur_platform::actions::taskbar_dump_now() {
                    println!("{line}");
                }
            } else {
                for icon in okur_platform::actions::tray_icons_now() {
                    println!("{} @ {}, {}", icon.name, icon.x, icon.y);
                }
            }
            #[cfg(not(windows))]
            {
                let _ = dump;
                eprintln!("tray is only available on Windows");
            }
        }
        Some(Commands::Trace {
            out,
            stop_file,
            max_ms,
            page_changes,
            debug_log: _,
        }) => {
            #[cfg(windows)]
            match okur_platform::event_trace::run_trace(
                Path::new(&out),
                stop_file.as_deref().map(Path::new),
                max_ms,
                page_changes,
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
        Some(Commands::Keys {
            allow,
            title,
            steps,
        }) => {
            let parsed: Result<Vec<_>, _> = steps
                .iter()
                .map(|s| okur_platform::harness::parse_step(s))
                .collect();
            let result = parsed.and_then(|steps| {
                #[cfg(windows)]
                {
                    okur_platform::harness::send_steps(&steps, &allow, title.as_deref())
                }
                #[cfg(not(windows))]
                {
                    let _ = (steps, allow, title);
                    Err("sending keys is only available on Windows".to_owned())
                }
            });
            if let Err(e) = result {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        Some(Commands::Front { title }) => {
            #[cfg(windows)]
            let result = okur_platform::harness::bring_to_front(&title);
            #[cfg(not(windows))]
            let result: Result<String, String> = {
                let _ = title;
                Err("bringing a window to the front is only available on Windows".to_owned())
            };
            match result {
                Ok(title) => println!("{title}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Focus) => {
            #[cfg(windows)]
            println!("{}", okur_platform::uia_snapshot::focused_element_json());
            #[cfg(not(windows))]
            {
                eprintln!("the focus is only read on Windows");
                std::process::exit(1);
            }
        }
        Some(Commands::ProbeNotifications { window, millis }) => {
            match okur_platform::uia_probe::notification_registrations(&window, millis) {
                Ok(lines) => lines.iter().for_each(|line| println!("{line}")),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::ListNotifications { only }) => {
            #[cfg(windows)]
            {
                use okur_platform::notification_listener as listener;
                match listener::access() {
                    Ok(access) => println!("access: {access}"),
                    Err(e) => println!("access: {e}"),
                }
                match listener::toasts() {
                    Ok(toasts) => {
                        println!("toasts: {}", toasts.len());
                        for toast in toasts.iter().filter(|t| {
                            t.app.contains(&only) || t.texts.iter().any(|x| x.contains(&only))
                        }) {
                            println!("{} {}: {}", toast.id, toast.app, toast.texts.join(" | "));
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                }
            }
            #[cfg(not(windows))]
            {
                let _ = only;
                eprintln!("list-notifications is for Windows");
                std::process::exit(1);
            }
        }
        Some(Commands::JawsHistory { clear }) => {
            #[cfg(windows)]
            match okur_platform::harness::jaws_history(clear) {
                Ok(history) => println!("{history}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            #[cfg(not(windows))]
            {
                let _ = clear;
                eprintln!("JAWS runs only on Windows");
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
            match okur_platform::AudioEndpointMeter::wait_for_silence(
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
        Some(Commands::Lines {
            snapshot,
            grep,
            spans,
        }) => {
            let loaded = okur_corpus::snapshots::load(Path::new(&snapshot)).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let buffer = VirtualBuffer::from_aom(&loaded.aom);
            for line in &buffer.lines {
                if grep.as_deref().is_some_and(|g| !line.text.contains(g)) {
                    continue;
                }
                println!(
                    "{:>5} {:>6}  node {:<5} {:<16} {:?}  speech {:?}",
                    line.line_index,
                    line.start_offset,
                    line.node_id,
                    format!("{:?}", line.role),
                    line.text,
                    line.speech
                );
                if !spans {
                    continue;
                }
                for span in buffer.spans.iter().filter(|s| {
                    s.start_offset >= line.start_offset && s.end_offset <= line.end_offset
                }) {
                    println!(
                        "      {:>6}..{:<6} node {:<5} {:<16} {:?}",
                        span.start_offset,
                        span.end_offset,
                        span.node_id,
                        format!("{:?}", span.role),
                        buffer
                            .text
                            .get(span.start_offset..span.end_offset)
                            .unwrap_or_default()
                    );
                }
            }
        }
        Some(Commands::Invariants {
            corpus_dir,
            write_baseline,
        }) => {
            let dir = Path::new(&corpus_dir);
            let results = okur_corpus::invariants::run_all(dir).unwrap_or_else(|e| {
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
                if let Err(e) = okur_corpus::invariants::write_baseline(dir, &results) {
                    eprintln!("Failed to write baseline: {e}");
                    std::process::exit(1);
                }
                println!(
                    "Wrote {:?}",
                    dir.join(okur_corpus::invariants::BASELINE_FILE)
                );
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
                    Ok(bytes) => Some(okur_corpus::manifest::sha256_hex(&bytes)),
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
                okur_corpus::manifest::write(dir)
            } else {
                okur_corpus::manifest::check(dir)
            };
            match result {
                Ok(()) if write => {
                    println!("Wrote {:?}", dir.join(okur_corpus::manifest::MANIFEST_FILE))
                }
                Ok(()) => println!("Corpus manifest is current."),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("No command specified. Use `okur-lab --help` for available options.");
        }
    }
}
