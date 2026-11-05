//! `okur-lab trace`: the accessibility events of every window, written as a trace file (JSON
//! Lines: its header, then one event a line), the capture the live screen reader uses.

/// Records accessibility events to `out` until `stop_file` exists or `max_ms` elapses, and
/// returns the number of events recorded. With `page_changes`, the page in front's objects
/// shown, hidden and reordered are recorded too, as `page_change` events with their object
/// ids and nothing read.
#[cfg(windows)]
pub fn run_trace(
    out: &std::path::Path,
    stop_file: Option<&std::path::Path>,
    max_ms: u64,
    page_changes: bool,
) -> Result<usize, String> {
    use okur_windows_model::event_trace::{TraceEvent, TraceHeader, TRACE_FORMAT_VERSION};
    use okur_windows_model::privacy;
    use std::io::Write;
    let io_err = |e: std::io::Error| format!("{out:?}: {e}");
    let mut file = std::fs::File::create(out).map_err(io_err)?;
    let header = TraceHeader {
        trace_format_version: TRACE_FORMAT_VERSION,
        tool_version: env!("CARGO_PKG_VERSION").to_owned(),
        host: std::env::var("COMPUTERNAME").unwrap_or_default(),
        started_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        screen: okur_windows::actions::primary_screen_size().map(|(w, h)| [w, h]),
    };
    let header = serde_json::to_string(&header).map_err(|e| e.to_string())?;
    file.write_all(format!("{header}\n").as_bytes())
        .map_err(io_err)?;

    // A trace goes into the public corpus: no one's e-mail address goes with it.
    let sink = move |mut event: TraceEvent| {
        if privacy::internet_feed(&event) || privacy::unvisited_row_under_mouse(&event) {
            return;
        }
        // Nor JAWS's own dictionary, which its Dictionary Manager shows.
        privacy::without_shipped_dictionary(&mut event);
        if let Ok(line) = serde_json::to_string(&event) {
            let mut line = privacy::redact_addresses(&line);
            line.push('\n');
            let _ = file.write_all(line.as_bytes());
        }
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(max_ms);
    Ok(okur_windows::event_capture::run_recording(
        Box::new(sink),
        page_changes,
        || std::time::Instant::now() < deadline && !stop_file.is_some_and(|p| p.exists()),
    ))
}
