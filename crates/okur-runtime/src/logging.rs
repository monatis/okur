//! The logs: the debug log a run is asked for, and the installed reader's kept log, each
//! written from a thread of its own.

use std::path::{Path, PathBuf};

/// Logs to stderr at the RUST_LOG level, or, with `debug_log`, everything from the screen
/// reader's crates (the core's, the runtime's and `platform_targets`) at debug level to that
/// file. The installed reader, which has no stderr, keeps its warnings and errors in its
/// `kept` log instead: without it a caught panic, a synthesizer that failed or settings not
/// saved left no trace in the field (performance review 2026-11-02, speech finding 6).
pub fn init_logging(debug_log: Option<&str>, kept: Option<PathBuf>, platform_targets: &[&str]) {
    if let Some(path) = debug_log {
        match std::fs::File::create(path) {
            Ok(file) => {
                let lines = spawn_log_writer(file, None);
                let targets = ["okur_core", "okur_runtime"]
                    .iter()
                    .chain(platform_targets)
                    .map(|target| format!("{target}=debug"))
                    .collect::<Vec<_>>()
                    .join(",");
                tracing_subscriber::fmt()
                    .with_writer(move || LogLines(lines.clone()))
                    .with_ansi(false)
                    .with_env_filter(tracing_subscriber::EnvFilter::new(targets))
                    .init();
                return;
            }
            Err(e) => eprintln!("Cannot write debug log {path}: {e}"),
        }
    }
    let kept_log = kept.and_then(|path| Some((open_kept_log(&path).ok()?, path)));
    if let Some((file, path)) = kept_log {
        let lines = spawn_log_writer(file, Some(path));
        tracing_subscriber::fmt()
            .with_writer(move || LogLines(lines.clone()))
            .with_ansi(false)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
            )
            .init();
        return;
    }
    tracing_subscriber::fmt::init();
}

/// The most of the kept log written before it starts again, the one before kept beside it
/// (`okur.old.log`).
const KEPT_LOG_CAP: u64 = 1 << 20;

/// The kept log at `path`, for appending: started again where it has grown past its cap.
fn open_kept_log(path: &Path) -> std::io::Result<std::fs::File> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    if std::fs::metadata(path).is_ok_and(|m| m.len() > KEPT_LOG_CAP) {
        let _ = std::fs::rename(path, path.with_extension("old.log"));
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
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
fn spawn_log_writer(
    file: std::fs::File,
    kept: Option<std::path::PathBuf>,
) -> std::sync::mpsc::Sender<Vec<u8>> {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let spawned = std::thread::Builder::new()
        .name("okur-debug-log".to_owned())
        .spawn(move || {
            use std::io::Write;
            let mut written = file.metadata().map(|m| m.len()).unwrap_or_default();
            let mut out = std::io::BufWriter::new(file);
            while let Ok(first) = rx.recv() {
                for lines in std::iter::once(first).chain(rx.try_iter()) {
                    written += lines.len() as u64;
                    let _ = out.write_all(&lines);
                }
                // Flushed whenever the queue empties, so the log is current on disk.
                let _ = out.flush();
                // The kept log starts again past its cap.
                if let Some(path) = kept.as_ref().filter(|_| written > KEPT_LOG_CAP) {
                    if let Ok(file) = open_kept_log(path) {
                        out = std::io::BufWriter::new(file);
                        written = 0;
                    }
                }
            }
        });
    if let Err(e) = spawned {
        eprintln!("No debug-log thread, the debug log stays empty: {e}");
    }
    tx
}

/// An event's kind in a few words, for logs: what an event is without formatting what it holds
/// (a document's whole text, a page's elements; second review, 2026-11-03).
pub fn event_kind(event: &okur_core::AccessibilityEvent) -> &'static str {
    use okur_core::AccessibilityEvent;
    match event {
        AccessibilityEvent::Focus(_) => "focus",
        AccessibilityEvent::Menu(_) => "menu",
        AccessibilityEvent::Change { .. } => "change",
        AccessibilityEvent::TextContext(_) => "text context",
        AccessibilityEvent::ConsoleText(_) => "console text",
        AccessibilityEvent::Selected(_) => "selected",
        AccessibilityEvent::LiveRegion(_) => "live region",
        AccessibilityEvent::ProgressChanged { .. } => "progress changed",
        AccessibilityEvent::ViewerText(_) => "viewer text",
        AccessibilityEvent::ScreenText(_) => "screen text",
        AccessibilityEvent::WindowControls(_) => "window controls",
        AccessibilityEvent::Recognized(_) => "recognized",
        AccessibilityEvent::FileRecognized(_) => "file recognized",
        AccessibilityEvent::FileRecognitionStarted => "file recognition started",
        AccessibilityEvent::TopWindows(_) => "top windows",
        AccessibilityEvent::TrayIcons(_) => "tray icons",
        AccessibilityEvent::PageElements(_) => "page elements",
        AccessibilityEvent::TrackedPlace { .. } => "tracked place",
        AccessibilityEvent::DocumentHeadings(_) => "document headings",
        AccessibilityEvent::DocumentLinks(_) => "document links",
        AccessibilityEvent::LineSpacing(_) => "line spacing",
        AccessibilityEvent::WordKeyState(_) => "word key state",
        AccessibilityEvent::DocumentFields(_) => "document fields",
        AccessibilityEvent::DocumentNotes(..) => "document notes",
        AccessibilityEvent::DocumentParagraphs(_) => "document paragraphs",
        AccessibilityEvent::DocumentView(_) => "document view",
        AccessibilityEvent::AppWindowTexts(_) => "app window texts",
        AccessibilityEvent::StatusBar(_) => "status bar",
        AccessibilityEvent::SheetPlace(_) => "sheet place",
        AccessibilityEvent::SheetList { .. } => "sheet list",
        AccessibilityEvent::DocumentLoaded { .. } => "document loaded",
        AccessibilityEvent::Ui(_) => "okur window",
        AccessibilityEvent::PageChanged => "page changed",
        AccessibilityEvent::PagePartsLoaded(_) => "page parts loaded",
        AccessibilityEvent::WindowRenamed(_) => "window renamed",
        AccessibilityEvent::WindowShown(_) => "window shown",
        AccessibilityEvent::MouseMoved { .. } => "mouse moved",
        AccessibilityEvent::Toast { .. } => "toast",
        AccessibilityEvent::BalloonTip(_) => "balloon tip",
        AccessibilityEvent::AppNotification { .. } => "application notification",
        AccessibilityEvent::MousePoint(_) => "mouse point",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An event is logged by its kind, not by what it holds.
    #[test]
    fn an_event_is_logged_by_its_kind() {
        let text = okur_core::AccessibilityEvent::BalloonTip("Password: Use at least 8".into());
        assert_eq!(event_kind(&text), "balloon tip");
        assert_eq!(
            event_kind(&okur_core::AccessibilityEvent::PageChanged),
            "page changed"
        );
    }

    /// The kept log grown past its cap starts again, the one before kept beside it.
    #[test]
    fn the_kept_log_starts_again_past_its_cap() {
        let folder = std::env::temp_dir().join(format!("okur-kept-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let path = folder.join("okur.log");
        assert!(open_kept_log(&path).is_ok());
        let full = vec![b'x'; usize::try_from(KEPT_LOG_CAP + 1).unwrap_or(usize::MAX)];
        assert!(std::fs::write(&path, &full).is_ok());
        assert!(open_kept_log(&path).is_ok());
        let size = |p: &Path| std::fs::metadata(p).map(|m| m.len()).ok();
        assert_eq!(size(&path), Some(0));
        assert_eq!(
            size(&path.with_extension("old.log")),
            Some(KEPT_LOG_CAP + 1)
        );
        let _ = std::fs::remove_dir_all(&folder);
    }
}
