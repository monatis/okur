//! Okur's own windows on Linux (okur-ui) started as everywhere, what they tell the engine kept
//! in the trace beside the accessibility events: a replay then plays a dialog's choices, its
//! buttons and its close as the live run had them (Escape from a dialog, and the quiet return to
//! the page after it, could not be replayed: the handoff of 2026-11-09, evening).

use std::io::Read;
use std::path::Path;

use okur_linux_model::convert::AtspiRecord;
use okur_runtime::ui_host::{spawn_plain, UiProcess, UiSpawner};

use crate::events::{trace_only, unix_ms, TraceOut};

/// Starts okur-ui as an ordinary child process, its messages to the engine also written to the
/// trace as the UI host reads them.
pub struct TracedSpawner {
    trace: TraceOut,
}

impl TracedSpawner {
    pub fn new(trace: TraceOut) -> Self {
        Self { trace }
    }
}

impl UiSpawner for TracedSpawner {
    fn spawn(&self, program: &Path) -> Option<UiProcess> {
        let mut process = spawn_plain(program)?;
        process.output = Box::new(TracedOutput {
            inner: process.output,
            line: Vec::new(),
            trace: std::sync::Arc::clone(&self.trace),
        });
        Some(process)
    }
}

/// okur-ui's output as the UI host reads it, each whole line that is a message also written to
/// the trace.
struct TracedOutput {
    inner: Box<dyn Read + Send>,
    line: Vec<u8>,
    trace: TraceOut,
}

impl TracedOutput {
    fn line_done(&mut self) {
        let line = std::mem::take(&mut self.line);
        let Ok(text) = std::str::from_utf8(&line) else {
            return;
        };
        if let Ok(message) = serde_json::from_str::<okur_core::ui::FromUi>(text.trim_end()) {
            trace_only(&self.trace, AtspiRecord::ui(unix_ms(), message));
        }
    }
}

impl Read for TracedOutput {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        for &byte in buf.get(..read).unwrap_or_default() {
            if byte == b'\n' {
                self.line_done();
            } else {
                self.line.push(byte);
            }
        }
        Ok(read)
    }
}
