//! The engine's [`UiHost`]: `okur-ui`, the process that draws Okur's windows, started beside
//! Okur's own executable and kept running, so that a window opens without a process start.
//! Messages go to its standard input from a thread of their own, so the engine never waits
//! on it; what the user did to a window comes back from its standard output as
//! [`AccessibilityEvent::Ui`]. If it exits, it is started again for the next message.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;

use okur_core::event::AccessibilityEvent;
use okur_core::ui::{FromUi, ToUi, UiHost};
use tokio::sync::mpsc::UnboundedSender;

pub struct ProcessUiHost {
    messages: mpsc::Sender<ToUi>,
}

impl ProcessUiHost {
    /// Starts `okur-ui` from Okur's own folder; `None` when it is not there.
    pub fn start(events: UnboundedSender<AccessibilityEvent>) -> Option<Self> {
        let program = program_path()?;
        if !program.exists() {
            tracing::warn!(?program, "okur-ui not found: Okur's windows will not show");
            return None;
        }
        let (messages, received) = mpsc::channel::<ToUi>();
        std::thread::Builder::new()
            .name("okur-ui writer".to_owned())
            .spawn(move || {
                let mut running = Running::start(&program, &events);
                for message in received {
                    let Ok(line) = okur_ui_model::encode(&message) else {
                        continue;
                    };
                    // A process that exited is started again, once per message.
                    if !running.as_mut().is_some_and(|r| r.write(&line)) {
                        running = Running::start(&program, &events);
                        if !running.as_mut().is_some_and(|r| r.write(&line)) {
                            tracing::warn!("okur-ui did not take a message");
                        }
                    }
                }
            })
            .ok()?;
        Some(Self { messages })
    }
}

impl UiHost for ProcessUiHost {
    fn send(&self, message: ToUi) {
        let _ = self.messages.send(message);
    }
}

fn program_path() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "okur-ui.exe"
    } else {
        "okur-ui"
    };
    Some(std::env::current_exe().ok()?.with_file_name(name))
}

/// A running `okur-ui` and its input.
struct Running {
    child: Child,
    input: ChildStdin,
}

impl Running {
    fn start(program: &PathBuf, events: &UnboundedSender<AccessibilityEvent>) -> Option<Self> {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| tracing::warn!(error = %e, "okur-ui did not start"))
            .ok()?;
        let input = child.stdin.take()?;
        if let Some(output) = child.stdout.take() {
            let events = events.clone();
            let _ = std::thread::Builder::new()
                .name("okur-ui reader".to_owned())
                .spawn(move || {
                    for line in BufReader::new(output).lines() {
                        let Ok(line) = line else { break };
                        match okur_ui_model::decode::<FromUi>(&line) {
                            Ok(message) => {
                                let _ = events.send(AccessibilityEvent::Ui(message));
                            }
                            Err(e) => {
                                tracing::warn!(%line, error = %e, "unreadable okur-ui message")
                            }
                        }
                    }
                });
        }
        tracing::debug!(pid = child.id(), "okur-ui started");
        Some(Self { child, input })
    }

    /// Writes one message; false when the process is gone.
    fn write(&mut self, line: &str) -> bool {
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            return false;
        }
        writeln!(self.input, "{line}").is_ok() && self.input.flush().is_ok()
    }
}
