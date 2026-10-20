//! The engine's [`UiHost`]: `okur-ui`, the process that draws Okur's windows, started beside
//! Okur's own executable and kept running, so that a window opens without a process start.
//! Messages go to its standard input from a thread of their own, so the engine never waits
//! on it; what the user did to a window comes back from its standard output as
//! [`AccessibilityEvent::Ui`]. If it exits, it is started again for the next message.
//!
//! An Okur with UI Access (the installed `okur-sr.exe`) starts it with a lowered token, UI
//! Access cleared and the integrity Medium, so that it stays the normal process
//! `docs/references/helper-ui-architecture.md` decided it is; and lets it take the foreground
//! for each dialog it shows, which a Medium process could not do by itself.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use okur_core::event::AccessibilityEvent;
use okur_core::ui::{FromUi, ToUi, UiHost};
use tokio::sync::mpsc::UnboundedSender;

/// What the writer thread is asked to do.
enum Message {
    Send(ToUi),
    /// End `okur-ui` (its input closes, and it exits), then say so.
    Close(mpsc::Sender<()>),
}

pub struct ProcessUiHost {
    messages: mpsc::Sender<Message>,
}

/// Ends `okur-ui` as Okur exits, from outside the engine that owns the host.
#[derive(Clone)]
pub struct UiCloser {
    messages: mpsc::Sender<Message>,
}

impl UiCloser {
    /// Closes `okur-ui`'s input, so that it exits, and waits up to `wait` for it to go.
    pub fn close(&self, wait: Duration) {
        let (done_tx, done_rx) = mpsc::channel();
        if self.messages.send(Message::Close(done_tx)).is_ok() {
            let _ = done_rx.recv_timeout(wait);
        }
    }
}

impl ProcessUiHost {
    /// Starts `okur-ui` from Okur's own folder; `None` when it is not there.
    pub fn start(events: UnboundedSender<AccessibilityEvent>) -> Option<Self> {
        let program = program_path()?;
        if !program.exists() {
            tracing::warn!(?program, "okur-ui not found: Okur's windows will not show");
            return None;
        }
        // Lowered only below UI Access: an Okur started as administrator from a terminal
        // keeps its okur-ui at its own level, as before.
        let lowered = crate::elevation::has_ui_access();
        let (messages, received) = mpsc::channel::<Message>();
        std::thread::Builder::new()
            .name("okur-ui writer".to_owned())
            .spawn(move || {
                let mut running = Running::start(&program, &events, lowered);
                for message in received {
                    let message = match message {
                        Message::Send(message) => message,
                        Message::Close(done) => {
                            if let Some(running) = running.take() {
                                running.close(Duration::from_millis(500));
                            }
                            let _ = done.send(());
                            break;
                        }
                    };
                    let Ok(line) = okur_ui_model::encode(&message) else {
                        continue;
                    };
                    let shows_dialog = matches!(message, ToUi::DialogShow { .. });
                    // A process that exited is started again, once per message.
                    if !running
                        .as_mut()
                        .is_some_and(|r| r.write(&line, shows_dialog))
                    {
                        running = Running::start(&program, &events, lowered);
                        if !running
                            .as_mut()
                            .is_some_and(|r| r.write(&line, shows_dialog))
                        {
                            tracing::warn!("okur-ui did not take a message");
                        }
                    }
                }
            })
            .ok()?;
        Some(Self { messages })
    }

    /// What ends `okur-ui` as Okur exits.
    pub fn closer(&self) -> UiCloser {
        UiCloser {
            messages: self.messages.clone(),
        }
    }
}

impl UiHost for ProcessUiHost {
    fn send(&self, message: ToUi) {
        let _ = self.messages.send(Message::Send(message));
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

/// The `okur-ui` process, started the plain way or with the lowered token.
enum Process {
    Plain(Child),
    #[cfg(windows)]
    Lowered(crate::lowered_process::LoweredChild),
}

impl Process {
    fn has_exited(&mut self) -> bool {
        match self {
            Process::Plain(child) => matches!(child.try_wait(), Ok(Some(_))),
            #[cfg(windows)]
            Process::Lowered(child) => child.has_exited(),
        }
    }

    fn id(&self) -> u32 {
        match self {
            Process::Plain(child) => child.id(),
            #[cfg(windows)]
            Process::Lowered(child) => child.id(),
        }
    }
}

/// A running `okur-ui` and its input.
struct Running {
    process: Process,
    input: Box<dyn Write + Send>,
    lowered: bool,
}

impl Running {
    fn start(
        program: &PathBuf,
        events: &UnboundedSender<AccessibilityEvent>,
        lowered: bool,
    ) -> Option<Self> {
        let (process, input, output) = if lowered {
            Self::spawn_lowered(program)?
        } else {
            Self::spawn_plain(program)?
        };
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
        tracing::debug!(pid = process.id(), lowered, "okur-ui started");
        Some(Self {
            process,
            input,
            lowered,
        })
    }

    #[allow(clippy::type_complexity)]
    fn spawn_plain(
        program: &PathBuf,
    ) -> Option<(Process, Box<dyn Write + Send>, Box<dyn Read + Send>)> {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| tracing::warn!(error = %e, "okur-ui did not start"))
            .ok()?;
        let input = child.stdin.take()?;
        let output = child.stdout.take()?;
        Some((Process::Plain(child), Box::new(input), Box::new(output)))
    }

    #[allow(clippy::type_complexity)]
    fn spawn_lowered(
        program: &PathBuf,
    ) -> Option<(Process, Box<dyn Write + Send>, Box<dyn Read + Send>)> {
        #[cfg(windows)]
        {
            match crate::lowered_process::spawn(program) {
                Ok((child, pipes)) => Some((
                    Process::Lowered(child),
                    Box::new(pipes.input),
                    Box::new(pipes.output),
                )),
                // Okur's windows still show, at Okur's own level.
                Err(e) => {
                    tracing::warn!(error = %e, "okur-ui not started lowered; started as Okur");
                    Self::spawn_plain(program)
                }
            }
        }
        #[cfg(not(windows))]
        {
            Self::spawn_plain(program)
        }
    }

    /// Writes one message; false when the process is gone. A dialog to show is first
    /// allowed to take the foreground.
    fn write(&mut self, line: &str, shows_dialog: bool) -> bool {
        if self.process.has_exited() {
            return false;
        }
        if shows_dialog && self.lowered {
            crate::process::allow_foreground(self.process.id());
        }
        writeln!(self.input, "{line}").is_ok() && self.input.flush().is_ok()
    }

    /// Closes the input, so that `okur-ui` exits, and waits up to `wait` for it.
    fn close(mut self, wait: Duration) {
        drop(std::mem::replace(
            &mut self.input,
            Box::new(std::io::sink()),
        ));
        let until = std::time::Instant::now() + wait;
        while !self.process.has_exited() && std::time::Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
