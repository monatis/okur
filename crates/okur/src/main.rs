use clap::{Parser, Subcommand};
use okur_core::{
    AccessibilityEvent, AomNode, Role, ScreenReaderEngine, VirtualBuffer, VirtualCursor,
};
use okur_platform::{
    create_event_listener, create_keyboard_hook, KeyAction, PlatformBridge, SpeechLog,
    TtsEngineType, TtsManager,
};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(
    name = "okur",
    about = "Okur: an open screen reader that keeps your JAWS muscle memory, and its verification harness"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run Okur, the live screen reader. Its settings are the ones its settings dialog
    /// (Insert+6 or Insert+V) saved; the options below change them for this run only.
    Run {
        /// TTS synthesizer engine: auto (OneCore, else SAPI 5, else none), onecore, sapi5, or
        /// no-speech
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

        /// JAWS keyboard layout: desktop (Insert is the JAWS key) or laptop (CapsLock is the
        /// JAWS key, and Insert works as well)
        #[arg(long)]
        keyboard_layout: Option<String>,

        /// The JAWS key for the layout: insert, capslock or scrolllock (default: the
        /// layout's own)
        #[arg(long)]
        jaws_key: Option<String>,

        /// Read a growing live region's whole text again on every change, as JAWS does
        /// (default: only what was appended, so a streamed chat reply is read once)
        #[arg(long)]
        live_region_whole_text: bool,

        /// Typing echo: characters (as JAWS's default) or none (JAWS's TypingEcho=0)
        #[arg(long)]
        typing_echo: Option<String>,

        /// The settings folder to read and save instead of the user's own, so that a test run
        /// never changes the user's settings
        #[arg(long)]
        settings: Option<String>,
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

        /// Also record the page in front's objects shown, hidden and reordered, with their
        /// object ids (Chromium pages; nothing is read for them)
        #[arg(long)]
        page_changes: bool,
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
    /// List available voices from the synthesizer
    Voices {
        /// TTS synthesizer engine: auto (default: OneCore, else SAPI 5, else none), onecore,
        /// sapi5, or no-speech
        #[arg(short, long, default_value = "auto")]
        engine: String,
    },
    /// Print the Virtual Viewer's lines for the window in front, as Virtualize Window builds
    /// them (Windows): for comparing with JAWS's viewer
    WindowText {
        /// The focused control only, as Virtualize Current Control reads it
        #[arg(long)]
        control: bool,
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

/// Loads a page from a UIA snapshot (`okur snapshot` output) through the production
/// UIA-to-AOM mapping.
fn load_aom_from_path(path: &Path) -> Result<AomNode, Box<dyn std::error::Error>> {
    let snapshot = okur_platform::uia_snapshot::UiaSnapshot::load(path)?;
    Ok(okur_platform::uia_snapshot::to_aom(&snapshot.root))
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

/// The JAWS keyboard settings named on the command line (`--keyboard-layout`, `--jaws-key`).
/// `okur run`'s options that change a setting for the run.
struct RunOptions {
    engine: Option<String>,
    voice: Option<String>,
    rate: Option<i32>,
    volume: Option<u16>,
    keyboard_layout: Option<String>,
    jaws_key: Option<String>,
    live_region_whole_text: bool,
    typing_echo: Option<String>,
}

impl RunOptions {
    /// Changes `settings` as the options ask; an error names an option's bad value.
    fn apply(self, settings: &mut okur_core::settings::Settings) -> Result<(), String> {
        use okur_core::settings::{LiveRegionAppend, Synthesizer, TypingEcho};
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
        if self.keyboard_layout.is_some() || self.jaws_key.is_some() {
            let layout = match (&self.keyboard_layout, settings.keyboard.layout) {
                (Some(layout), _) => layout.clone(),
                (None, okur_core::settings::KeyboardLayout::Desktop) => "desktop".to_owned(),
                (None, okur_core::settings::KeyboardLayout::Laptop) => "laptop".to_owned(),
            };
            settings.keyboard = keyboard_settings(&layout, self.jaws_key.as_deref())?;
        }
        if self.live_region_whole_text {
            settings.live_region_append = LiveRegionAppend::WholeText;
        }
        if let Some(echo) = self.typing_echo {
            settings.typing_echo = match echo.to_ascii_lowercase().as_str() {
                "characters" => TypingEcho::Characters,
                "none" => TypingEcho::None,
                other => {
                    return Err(format!(
                        "--typing-echo must be characters or none, not {other:?}"
                    ))
                }
            };
        }
        Ok(())
    }
}

fn keyboard_settings(
    layout: &str,
    jaws_key: Option<&str>,
) -> Result<okur_core::settings::KeyboardSettings, String> {
    use okur_core::settings::{JawsKey, KeyboardLayout, KeyboardSettings};
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
/// Engine work longer than this, on a key or an event, is logged as slow: the keyboard hook
/// waits on the engine, and a key that waits 150 ms passes to the application unread.
const SLOW_ENGINE: std::time::Duration = std::time::Duration::from_millis(20);

/// A short name for an event, for logs (a page's tree is too big to print).
fn event_kind(event: Option<&AccessibilityEvent>) -> &'static str {
    match event {
        None => "idle",
        Some(AccessibilityEvent::Focus(_)) => "focus",
        Some(AccessibilityEvent::Menu(_)) => "menu",
        Some(AccessibilityEvent::Change { .. }) => "change",
        Some(AccessibilityEvent::TextContext(_)) => "text context",
        Some(AccessibilityEvent::Selected(_)) => "selected",
        Some(AccessibilityEvent::LiveRegion(_)) => "live region",
        Some(AccessibilityEvent::ViewerText(_)) => "viewer text",
        Some(AccessibilityEvent::PageElements(_)) => "page elements",
        Some(AccessibilityEvent::DocumentLoaded { .. }) => "document loaded",
        Some(AccessibilityEvent::Ui(_)) => "okur window",
        Some(AccessibilityEvent::PageChanged) => "page changed",
        Some(AccessibilityEvent::PagePartLoaded(_)) => "page part loaded",
        Some(AccessibilityEvent::WindowRenamed(_)) => "window renamed",
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

fn init_logging(cli: &Cli) {
    if let Some(Commands::Run {
        debug_log: Some(path),
        ..
    }) = &cli.command
    {
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
            live_region_whole_text,
            typing_echo,
            settings: settings_folder,
        }) => {
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
            let options = RunOptions {
                engine,
                voice,
                rate,
                volume,
                keyboard_layout,
                jaws_key,
                live_region_whole_text,
                typing_echo,
            };
            if let Err(e) = options.apply(&mut settings) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
            layers.change(
                &okur_core::settings_layers::Scope::Session,
                &saved,
                &settings,
                None,
            );
            let engine_type = TtsEngineType::from(settings.voice.synthesizer);

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

            okur_core::speech::SpeechOutput::configure(&mut tts, &settings.voice);

            println!("============================================================");
            println!("  Okur, an open screen reader");
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
            } else if cfg!(windows) {
                // Live, the event listener starts with the focus as it is (a page is loaded
                // when that focus is in one), as any later focus change.
                let empty_doc = AomNode::new(0, Role::Document);
                (VirtualBuffer::from_aom(&empty_doc), false)
            } else {
                match PlatformBridge::new().get_active_document(None) {
                    Ok(aom) => {
                        println!("Attached to live active foreground document via OS accessibility APIs.");
                        (VirtualBuffer::from_aom(&aom), true)
                    }
                    Err(_) => {
                        println!("Operating in live native navigation mode.");
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
            let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech));
            // Enter on an edit focuses it in the page, Enter/Space on a control activates it.
            #[cfg(windows)]
            engine.set_actions(Box::new(okur_platform::actions::WindowsActions::start(
                Some(event_tx.clone()),
            )));
            // Okur's own windows (the Virtual Viewer on screen), drawn by okur-ui.
            if let Some(ui) = okur_platform::ui_host::ProcessUiHost::start(event_tx.clone()) {
                engine.set_ui(Box::new(ui));
            }
            engine.set_settings_layers(layers);
            // The settings dialog keeps what it changes in the settings folder.
            if let Some(folder) = settings_folder {
                engine.set_settings_store(Box::new(folder));
            }
            // Say System Time (Insert+F12) in the user's locale formats.
            #[cfg(windows)]
            engine.set_clock(Box::new(okur_platform::clock::WindowsClock));
            let engine = Arc::new(Mutex::new(engine));

            // Announce startup
            if let Ok(mut eng) = engine.lock() {
                if is_fixture {
                    eng.app_mode = okur_core::engine::AppMode::VirtualBuffer;
                    let doc_name = eng.cursor.buffer.document_title().unwrap_or("Document");
                    let welcome = format!("{}  \u{001d} Virtual PC Cursor \u{001e}", doc_name);
                    println!("[Okur] {}", welcome.replace(['\u{001d}', '\u{001e}'], ""));
                    eng.speech.speak(&welcome, true);
                } else {
                    eng.app_mode = okur_core::engine::AppMode::NativeControl;
                    println!("[Okur] Ready in live native Windows navigation mode.");
                }
            }
            // An application running as administrator shows nothing to an Okur that is not
            // (Windows Settings' Rename dialog); JAWS runs elevated.
            if !okur_platform::elevation::is_elevated() {
                let note = "Okur is not running as administrator: windows of applications \
                            that are (Settings' \"Rename this PC\", an elevated terminal) cannot \
                            be read. Start Okur from an administrator terminal to read them.";
                println!("[Okur] {note}");
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
                let deadline = arrived + std::time::Duration::from_millis(150);
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
                    let waited = arrived.elapsed();
                    let started = std::time::Instant::now();
                    let action = eng.handle_key_event(&event);
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
                eprintln!("Failed to install keyboard hook: {}", e);
                std::process::exit(1);
            }

            // Start OS-level accessibility event listener
            let mut event_listener = create_event_listener();
            if let Err(e) = event_listener.start(event_tx) {
                tracing::warn!("Could not start OS accessibility event listener: {}", e);
            } else {
                let engine_events = Arc::clone(&engine);
                let console = spawn_console();
                tokio::spawn(async move {
                    let idle = std::time::Duration::from_millis(150);
                    loop {
                        let event = match tokio::time::timeout(idle, event_rx.recv()).await {
                            Ok(Some(event)) => Some(event),
                            Ok(None) => break,
                            // Quiet for a moment: speech JAWS defers until no event supersedes it.
                            Err(_) => None,
                        };
                        // A page's buffer is built before the engine is taken: on a big page
                        // that takes long, and the keyboard hook waits on the engine.
                        let page = match &event {
                            Some(AccessibilityEvent::DocumentLoaded { title, root }) => {
                                let started = std::time::Instant::now();
                                let buffer = VirtualBuffer::from_aom(root);
                                tracing::debug!(
                                    lines = buffer.lines.len(),
                                    ms = started.elapsed().as_millis(),
                                    "page buffer built"
                                );
                                Some((title.as_str(), buffer))
                            }
                            _ => None,
                        };
                        let spoken = {
                            let Ok(mut eng) = engine_events.lock() else {
                                continue;
                            };
                            let started = std::time::Instant::now();
                            let spoken = match (&event, page) {
                                (_, Some((title, buffer))) => {
                                    eng.handle_document_loaded(title, buffer)
                                }
                                (Some(event), None) => eng.handle_accessibility_event(event),
                                (None, None) => eng.on_idle(),
                            };
                            let held = started.elapsed();
                            if held > SLOW_ENGINE {
                                tracing::warn!(
                                    event = event_kind(event.as_ref()),
                                    ms = held.as_millis(),
                                    "slow event handling"
                                );
                            }
                            // Idle ticks come every 150 ms of quiet; only events are timed.
                            if event.is_some() {
                                tracing::debug!(
                                    event = event_kind(event.as_ref()),
                                    us = held.as_micros(),
                                    "event handled"
                                );
                            }
                            spoken
                        };
                        if let Some(msg) = spoken {
                            // One line per utterance, so the debug log stays line-oriented.
                            let msg = msg
                                .replace(['\u{001d}', '\u{001e}'], "")
                                .replace('\n', " | ");
                            tracing::debug!(%msg, "spoken for event");
                            let _ = console.send(format!("[Event Spoken] {msg}"));
                        }
                    }
                });
            }

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

            // Handle clean exit via Ctrl+C
            if let Err(e) = tokio::signal::ctrl_c().await {
                eprintln!("Error waiting for shutdown signal: {}", e);
            }

            println!("\nShutting down Okur...");
            let _ = hook.stop();
            let _ = event_listener.stop();
            if let Ok(mut eng) = engine.lock() {
                eng.speech.speak("Okur exiting.", true);
            }
            println!("Okur stopped.");
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
                    let matched = steps.iter().filter(|s| s.matched).count();
                    let accepted = steps.iter().filter(|s| s.accepted).count();
                    println!(
                        "Okur live: {matched}/{} steps match JAWS exactly; {accepted} accepted \
                         divergences not counted",
                        steps.len() - accepted
                    );
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
            if write_baseline && api != PARITY_API {
                eprintln!("The baseline ratchets {PARITY_API:?} parity; drop --api to write it.");
                std::process::exit(2);
            }
            let report = okur_corpus::parity::run_with(dir, api);
            for step in report
                .steps
                .iter()
                .filter(|s| all || !(s.matched || s.accepted))
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
        Some(Commands::Trace {
            out,
            stop_file,
            max_ms,
            page_changes,
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
        Some(Commands::Lines { snapshot, grep }) => {
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
            println!("No command specified. Use `okur --help` for available options.");
        }
    }
}
