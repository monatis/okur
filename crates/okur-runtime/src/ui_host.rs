//! The engine's [`UiHost`]: `okur-ui`, the process that draws Okur's windows, started beside
//! Okur's own executable and kept running, so that a window opens without a process start.
//! Messages go to its standard input from a thread of their own, so the engine never waits
//! on it; what the user did to a window comes back from its standard output as
//! [`AccessibilityEvent::Ui`]. If it exits, it is started again for the next message, and the
//! windows it showed are reported closed, as each one's own closing is: Okur no longer believes
//! them open (performance review 2026-10-25, finding 9).
//!
//! The platform starts it ([`UiSpawner`]): Windows' Okur with UI Access (the installed
//! `okur-sr.exe`) with a lowered token, UI Access cleared and the integrity Medium, so that it
//! stays the normal process `docs/references/helper-ui-architecture.md` decided it is, letting
//! it take the foreground for each dialog it shows, which a Medium process could not do by
//! itself.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use okur_core::event::AccessibilityEvent;
use okur_core::ui::{FromUi, ToUi, UiHost};

use crate::EventSink;

/// A started `okur-ui`, as the host asks after it.
pub trait UiChild: Send {
    fn has_exited(&mut self) -> bool;
    fn id(&self) -> u32;
}

impl UiChild for Child {
    fn has_exited(&mut self) -> bool {
        matches!(self.try_wait(), Ok(Some(_)))
    }

    fn id(&self) -> u32 {
        Child::id(self)
    }
}

/// A started `okur-ui`: the process, and its standard input, output and error.
pub struct UiProcess {
    pub child: Box<dyn UiChild>,
    pub input: Box<dyn Write + Send>,
    pub output: Box<dyn Read + Send>,
    pub error: Box<dyn Read + Send>,
}

/// How a system starts `okur-ui` and lets the windows it shows come to the front.
pub trait UiSpawner: Send + Sync {
    /// Starts the program at `program`, its standard streams piped; `None` when it does not.
    fn spawn(&self, program: &Path) -> Option<UiProcess>;

    /// Lets the process `pid` bring the window it shows next to the front.
    fn allow_foreground(&self, pid: u32) {
        let _ = pid;
    }
}

/// Starts `okur-ui` as an ordinary child process, at Okur's own level.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlainSpawner;

impl UiSpawner for PlainSpawner {
    fn spawn(&self, program: &Path) -> Option<UiProcess> {
        spawn_plain(program)
    }
}

/// `program` started as an ordinary child process, its standard streams piped.
pub fn spawn_plain(program: &Path) -> Option<UiProcess> {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| tracing::warn!(error = %e, "okur-ui did not start"))
        .ok()?;
    let input = child.stdin.take()?;
    let output = child.stdout.take()?;
    let error = child.stderr.take()?;
    Some(UiProcess {
        child: Box::new(child),
        input: Box::new(input),
        output: Box::new(output),
        error: Box::new(error),
    })
}

/// What the writer thread is asked to do.
enum Message {
    Send(ToUi),
    /// End `okur-ui` (its input closes, and it exits), then say so.
    Close(mpsc::Sender<()>),
}

pub struct ProcessUiHost {
    messages: mpsc::Sender<Message>,
}

/// The windows `okur-ui` shows: its dialogs and wizards by id, oldest first, and whether the
/// viewer's window is up. Kept from what it is told and what it tells, so that its exit can
/// report them closed.
#[derive(Debug, Default)]
struct OpenWindows {
    dialogs: Vec<u32>,
    viewer: bool,
}

impl OpenWindows {
    /// Notes what `message` shows or closes.
    fn told(&mut self, message: &ToUi) {
        let mut opened = |id: u32| {
            if !self.dialogs.contains(&id) {
                self.dialogs.push(id);
            }
        };
        match message {
            ToUi::DialogShow { dialog } => opened(dialog.id),
            ToUi::WizardShow { wizard } => opened(wizard.id),
            ToUi::DialogClose { id } => self.dialogs.retain(|d| d != id),
            ToUi::ViewerShow { .. } => self.viewer = true,
            ToUi::ViewerClose => self.viewer = false,
            _ => {}
        }
    }

    /// Notes what the user closed.
    fn heard(&mut self, message: &FromUi) {
        match message {
            FromUi::DialogClosed { id } => self.dialogs.retain(|d| d != id),
            FromUi::ViewerDismissed => self.viewer = false,
            _ => {}
        }
    }

    /// What the windows' going says to Okur, the newest first: each dialog closed, the viewer
    /// dismissed. Forgets them.
    fn gone(&mut self) -> Vec<FromUi> {
        let mut said: Vec<FromUi> = std::mem::take(&mut self.dialogs)
            .into_iter()
            .rev()
            .map(|id| FromUi::DialogClosed { id })
            .collect();
        if std::mem::take(&mut self.viewer) {
            said.push(FromUi::ViewerDismissed);
        }
        said
    }
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
    /// Starts `okur-ui` from Okur's own folder with `spawner`; `None` when it is not there.
    pub fn start(events: EventSink, spawner: Box<dyn UiSpawner>) -> Option<Self> {
        let program = program_path()?;
        if !program.exists() {
            tracing::warn!(?program, "okur-ui not found: Okur's windows will not show");
            return None;
        }
        let (messages, received) = mpsc::channel::<Message>();
        let open = Arc::new(Mutex::new(OpenWindows::default()));
        std::thread::Builder::new()
            .name("okur-ui writer".to_owned())
            .spawn(move || {
                let mut running = Running::start(&program, &events, spawner.as_ref(), &open);
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
                    if let Ok(mut open) = open.lock() {
                        open.told(&message);
                    }
                    // A window coming to the front, which the spawner may have to let okur-ui
                    // bring there.
                    let shows_dialog = matches!(
                        message,
                        ToUi::DialogShow { .. }
                            | ToUi::MainWindowShow { .. }
                            | ToUi::WizardShow { .. }
                            | ToUi::ViewerShow { .. }
                            | ToUi::FileOpen { .. }
                            | ToUi::DialogMenu { .. }
                    );
                    // A process that exited is started again, once per message.
                    if !running
                        .as_mut()
                        .is_some_and(|r| r.write(&line, shows_dialog, spawner.as_ref()))
                    {
                        running = Running::start(&program, &events, spawner.as_ref(), &open);
                        if !running
                            .as_mut()
                            .is_some_and(|r| r.write(&line, shows_dialog, spawner.as_ref()))
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
    let name = format!("okur-ui{}", std::env::consts::EXE_SUFFIX);
    Some(std::env::current_exe().ok()?.with_file_name(name))
}

/// A running `okur-ui` and its input.
struct Running {
    process: Box<dyn UiChild>,
    input: Box<dyn Write + Send>,
}

impl Running {
    fn start(
        program: &Path,
        events: &EventSink,
        spawner: &dyn UiSpawner,
        open: &Arc<Mutex<OpenWindows>>,
    ) -> Option<Self> {
        let UiProcess {
            child: process,
            input,
            output,
            error,
        } = spawner.spawn(program)?;
        // What okur-ui writes to its standard error (a panic's message) goes into Okur's log.
        let _ = std::thread::Builder::new()
            .name("okur-ui errors".to_owned())
            .spawn(move || {
                for line in BufReader::new(error).lines() {
                    let Ok(line) = line else { break };
                    tracing::warn!(%line, "okur-ui error output");
                }
            });
        let events = events.clone();
        let open = Arc::clone(open);
        let _ = std::thread::Builder::new()
            .name("okur-ui reader".to_owned())
            .spawn(move || {
                for line in BufReader::new(output).lines() {
                    let Ok(line) = line else { break };
                    match okur_ui_model::decode::<FromUi>(&line) {
                        Ok(message) => {
                            if let Ok(mut open) = open.lock() {
                                open.heard(&message);
                            }
                            events.send(AccessibilityEvent::Ui(message));
                        }
                        Err(e) => {
                            tracing::warn!(%line, error = %e, "unreadable okur-ui message")
                        }
                    }
                }
                // okur-ui is gone, and every window it showed with it.
                let gone = open.lock().map(|mut open| open.gone()).unwrap_or_default();
                if !gone.is_empty() {
                    tracing::warn!(windows = gone.len(), "okur-ui exited with windows open");
                }
                for message in gone {
                    events.send(AccessibilityEvent::Ui(message));
                }
            });
        tracing::debug!(pid = process.id(), "okur-ui started");
        Some(Self { process, input })
    }

    /// Writes one message; false when the process is gone. A dialog to show is first
    /// allowed to take the foreground.
    fn write(&mut self, line: &str, shows_dialog: bool, spawner: &dyn UiSpawner) -> bool {
        if self.process.has_exited() {
            return false;
        }
        if shows_dialog {
            spawner.allow_foreground(self.process.id());
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

#[cfg(test)]
mod tests {
    use super::*;
    use okur_ui_model::DialogSpec;

    fn dialog(id: u32) -> ToUi {
        ToUi::DialogShow {
            dialog: DialogSpec {
                id,
                title: String::new(),
                controls: Vec::new(),
                keys: Vec::new(),
            },
        }
    }

    /// The windows okur-ui leaves as it exits are reported closed, the newest first; those
    /// Okur or the user closed are not.
    #[test]
    fn the_windows_left_open_are_reported_closed() {
        let mut open = OpenWindows::default();
        for message in [dialog(1), dialog(2), dialog(3), ToUi::DialogClose { id: 2 }] {
            open.told(&message);
        }
        open.told(&ToUi::ViewerShow {
            title: String::new(),
            lines: Vec::new(),
            mark: okur_ui_model::TextSpan::default(),
        });
        open.heard(&FromUi::DialogClosed { id: 3 });
        open.told(&dialog(4));
        assert_eq!(
            open.gone(),
            [
                FromUi::DialogClosed { id: 4 },
                FromUi::DialogClosed { id: 1 },
                FromUi::ViewerDismissed
            ]
        );
        assert!(open.gone().is_empty());
    }
}
