//! What a running Okur is asked through its control pipe, and what it answers
//! (`docs/references/startup-and-secure-screens-proposal.md`, §5.6). One line each way:
//!
//! - `ping`: `pong`.
//! - `status`: `okur pid=<pid> session=<n> desktop=<name> version=<v> exe=<path>`.
//! - `quit`: `quitting`, and the copy exits without asking (the installer, `okur quit`,
//!   `okur-sr.exe --quit`, and a manual start taking over).
//!
//! The transport, the pipe and its security, is `okur_platform::instance`'s.

use std::time::Duration;

use okur_platform::instance;

use crate::startup::Instances;

/// A request a running copy answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Ping,
    Status,
    Quit,
}

impl Request {
    pub fn parse(line: &str) -> Option<Self> {
        match line.trim().to_ascii_lowercase().as_str() {
            "ping" => Some(Request::Ping),
            "status" => Some(Request::Status),
            "quit" => Some(Request::Quit),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Request::Ping => "ping",
            Request::Status => "status",
            Request::Quit => "quit",
        }
    }
}

pub const PONG: &str = "pong";
pub const QUITTING: &str = "quitting";
pub const UNKNOWN: &str = "unknown request";

/// What a copy says of itself to `status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub pid: u32,
    pub session: u32,
    pub desktop: String,
    pub version: String,
    pub exe: String,
}

impl Status {
    /// This process's.
    pub fn of_this_process(session: u32, desktop: &str) -> Self {
        Self {
            pid: std::process::id(),
            session,
            desktop: desktop.to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            exe: std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        }
    }

    pub fn line(&self) -> String {
        format!(
            "okur pid={} session={} desktop={} version={} exe={}",
            self.pid, self.session, self.desktop, self.version, self.exe
        )
    }

    /// A `status` reply read back; the executable's path may hold spaces, and comes last.
    pub fn parse(line: &str) -> Option<Self> {
        let rest = line.trim().strip_prefix("okur ")?;
        let (fields, exe) = rest.split_once(" exe=")?;
        let mut pid = None;
        let mut session = None;
        let mut desktop = None;
        let mut version = None;
        for field in fields.split_whitespace() {
            match field.split_once('=') {
                Some(("pid", v)) => pid = v.parse().ok(),
                Some(("session", v)) => session = v.parse().ok(),
                Some(("desktop", v)) => desktop = Some(v.to_owned()),
                Some(("version", v)) => version = Some(v.to_owned()),
                _ => {}
            }
        }
        Some(Self {
            pid: pid?,
            session: session?,
            desktop: desktop?,
            version: version?,
            exe: exe.to_owned(),
        })
    }
}

/// The reply to a request line; `on_quit` runs for `quit`, after which the copy exits.
pub fn answer(line: &str, status: &Status, on_quit: &dyn Fn()) -> String {
    match Request::parse(line) {
        Some(Request::Ping) => PONG.to_owned(),
        Some(Request::Status) => status.line(),
        Some(Request::Quit) => {
            on_quit();
            QUITTING.to_owned()
        }
        None => UNKNOWN.to_owned(),
    }
}

/// The control pipe of the copy on this process's desktop.
pub fn this_desktop_pipe() -> String {
    let session = okur_platform::desktop::session_id().unwrap_or(0);
    let desktop = okur_platform::desktop::thread_desktop().unwrap_or_else(|| "Default".into());
    instance::pipe_name(session, &desktop)
}

/// The lock of this process's desktop.
pub fn this_desktop_lock() -> String {
    let desktop = okur_platform::desktop::thread_desktop().unwrap_or_else(|| "Default".into());
    instance::lock_name(&desktop)
}

/// How long a request waits for a running copy's reply.
pub const REPLY_WAIT: Duration = Duration::from_secs(2);

/// Sends a request to the copy on this desktop.
pub fn send(request: Request) -> Result<String, String> {
    instance::request(&this_desktop_pipe(), request.as_str(), REPLY_WAIT)
}

/// What `okur quit` and `okur-sr.exe --quit` print: whether a copy was asked and agreed.
pub fn quit_this_desktop() -> Result<String, String> {
    match send(Request::Quit) {
        Ok(reply) if reply == QUITTING => Ok("Okur was asked to quit, and is quitting.".into()),
        Ok(reply) => Err(format!("Okur answered {reply:?}")),
        Err(e) => Err(format!("No Okur runs on this desktop ({e}).")),
    }
}

/// This desktop's copies, through the lock and the control pipe.
pub struct SystemInstances {
    lock: String,
}

impl SystemInstances {
    pub fn for_this_desktop() -> Self {
        Self {
            lock: this_desktop_lock(),
        }
    }
}

impl Instances for SystemInstances {
    type Lock = instance::InstanceLock;

    fn try_lock(&mut self, wait: Duration) -> Option<instance::InstanceLock> {
        instance::InstanceLock::acquire(&self.lock, wait)
    }

    fn ask_to_quit(&mut self) -> Option<u32> {
        let pid = send(Request::Status)
            .ok()
            .and_then(|reply| Status::parse(&reply))
            .map(|status| status.pid);
        match send(Request::Quit) {
            Ok(reply) => tracing::info!(%reply, "the running copy was asked to quit"),
            Err(e) => tracing::warn!(error = %e, "the running copy did not take quit"),
        }
        pid
    }

    fn terminate(&mut self, pid: u32) {
        tracing::warn!(pid, "the running copy did not quit: it is ended");
        if let Err(e) = okur_platform::process::terminate(pid) {
            tracing::warn!(pid, error = %e, "the running copy could not be ended");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_read_without_case_or_spaces() {
        assert_eq!(Request::parse("quit\r\n"), Some(Request::Quit));
        assert_eq!(Request::parse(" PING "), Some(Request::Ping));
        assert_eq!(Request::parse("status"), Some(Request::Status));
        assert_eq!(Request::parse("shutdown"), None);
        for request in [Request::Ping, Request::Status, Request::Quit] {
            assert_eq!(Request::parse(request.as_str()), Some(request));
        }
    }

    #[test]
    fn a_status_line_reads_back_with_spaces_in_the_path() {
        let status = Status {
            pid: 4321,
            session: 1,
            desktop: "Default".into(),
            version: "0.1.0".into(),
            exe: r"C:\Program Files\Okur\okur-sr.exe".into(),
        };
        let line = status.line();
        assert_eq!(
            line,
            r"okur pid=4321 session=1 desktop=Default version=0.1.0 exe=C:\Program Files\Okur\okur-sr.exe"
        );
        assert_eq!(Status::parse(&line), Some(status));
        assert_eq!(Status::parse("pong"), None);
    }

    #[test]
    fn quit_is_answered_and_acted_on_and_nothing_else_is() {
        let status = Status::of_this_process(1, "Default");
        let quit = std::cell::Cell::new(0);
        let on_quit = || quit.set(quit.get() + 1);
        assert_eq!(answer("ping", &status, &on_quit), PONG);
        assert_eq!(answer("status", &status, &on_quit), status.line());
        assert_eq!(answer("rm -rf", &status, &on_quit), UNKNOWN);
        assert_eq!(quit.get(), 0);
        assert_eq!(answer("quit", &status, &on_quit), QUITTING);
        assert_eq!(quit.get(), 1);
    }
}
