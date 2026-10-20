//! One Okur per desktop, and the pipe through which a running Okur is asked to quit
//! (`docs/references/startup-and-secure-screens-proposal.md`, §5.6).
//!
//! - **The lock** is a mutex named `Local\Okur-<desktop>`: `Local\` scopes it to the session
//!   and the name to the desktop, so that the user's copy and the secure desktop's can run
//!   at once, as Windows requires of an assistive technology that stays running across
//!   desktop switches. It is held by a thread of its own, since a mutex belongs to the thread
//!   that took it.
//! - **The control pipe** is `\\.\pipe\okur-<session>-<desktop>`, open only to the session's
//!   user, SYSTEM and Administrators, and only from this machine. It carries one line each way
//!   (`crates/okur/src/control.rs` says which), where NVDA posts `WM_QUIT` to a window that
//!   any process can find.
//!
//! Off Windows the lock is always taken and no pipe is served (one desktop, no installer
//! yet); macOS will bring its own.

use std::time::Duration;

/// A desktop's name as it may stand in an object's name: letters, digits, `-` and `_`.
fn object_part(desktop: &str) -> String {
    desktop
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The lock's name for a desktop.
pub fn lock_name(desktop: &str) -> String {
    format!(r"Local\Okur-{}", object_part(desktop))
}

/// The control pipe's name for a session's desktop.
pub fn pipe_name(session: u32, desktop: &str) -> String {
    format!(r"\\.\pipe\okur-{session}-{}", object_part(desktop))
}

/// The prefix every Okur control pipe's name starts with, after `\\.\pipe\`.
pub const PIPE_PREFIX: &str = "okur-";

/// The lock, held until dropped.
pub struct InstanceLock {
    release: Option<std::sync::mpsc::Sender<()>>,
    holder: Option<std::thread::JoinHandle<()>>,
}

impl InstanceLock {
    /// Takes the lock named `name`, waiting up to `wait` for a copy that is exiting to let it
    /// go; `None` when another copy keeps it. A copy that ended without letting it go (it
    /// was killed) left it abandoned, and it is taken.
    pub fn acquire(name: &str, wait: Duration) -> Option<Self> {
        #[cfg(windows)]
        {
            windows_impl::acquire(name, wait)
        }
        #[cfg(not(windows))]
        {
            let _ = (name, wait);
            Some(Self {
                release: None,
                holder: None,
            })
        }
    }

    /// Lets the lock go now (dropping it does the same).
    pub fn release(mut self) {
        self.let_go();
    }

    fn let_go(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(holder) = self.holder.take() {
            let _ = holder.join();
        }
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        self.let_go();
    }
}

/// Answers a control request: the reply to send back.
pub type Answer = Box<dyn Fn(&str) -> String + Send>;

/// The control pipe, served from a thread of its own for as long as the process runs.
pub struct ControlServer {
    _thread: Option<std::thread::JoinHandle<()>>,
}

impl ControlServer {
    /// Serves the pipe `name`, each request answered by `answer`. Fails when another process
    /// already serves a pipe of that name.
    pub fn start(name: &str, answer: Answer) -> Result<Self, String> {
        #[cfg(windows)]
        {
            windows_impl::serve(name, answer).map(|thread| Self {
                _thread: Some(thread),
            })
        }
        #[cfg(not(windows))]
        {
            let _ = (name, answer);
            Ok(Self { _thread: None })
        }
    }
}

/// Sends one request to the pipe `name` and returns the reply; an error when nothing serves
/// it (no copy runs there) or it did not answer within `timeout`.
pub fn request(name: &str, request: &str, timeout: Duration) -> Result<String, String> {
    #[cfg(windows)]
    {
        windows_impl::request(name, request, timeout)
    }
    #[cfg(not(windows))]
    {
        let _ = (name, request, timeout);
        Err("no control pipes off Windows".to_owned())
    }
}

/// Every Okur control pipe on the machine, by full name: the installer asks each to quit.
pub fn all_pipes() -> Vec<String> {
    #[cfg(windows)]
    {
        windows_impl::all_pipes()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{Answer, InstanceLock, PIPE_PREFIX};
    use std::time::Duration;
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Foundation::{
        CloseHandle, GetLastError, LocalFree, ERROR_PIPE_CONNECTED, HANDLE, HLOCAL, WAIT_ABANDONED,
        WAIT_OBJECT_0,
    };
    use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows::Win32::Storage::FileSystem::{
        FindClose, FindFirstFileW, FindNextFileW, FlushFileBuffers, ReadFile, WriteFile,
        FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX, WIN32_FIND_DATAW,
    };
    use windows::Win32::System::Pipes::{
        CallNamedPipeW, ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe,
        PIPE_READMODE_MESSAGE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_WAIT,
    };
    use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

    /// The longest request or reply.
    const MESSAGE: usize = 1024;

    pub(super) fn acquire(name: &str, wait: Duration) -> Option<InstanceLock> {
        let name = name.to_owned();
        let (taken_tx, taken_rx) = std::sync::mpsc::channel::<bool>();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let wait_ms = u32::try_from(wait.as_millis()).unwrap_or(u32::MAX);
        let holder = std::thread::Builder::new()
            .name("okur-instance".to_owned())
            .spawn(move || {
                let wide = HSTRING::from(name.as_str());
                // SAFETY: a named mutex, not taken at creation; closed below.
                let Ok(mutex) = (unsafe { CreateMutexW(None, false, PCWSTR(wide.as_ptr())) })
                else {
                    let _ = taken_tx.send(false);
                    return;
                };
                // SAFETY: the mutex created above.
                let waited = unsafe { WaitForSingleObject(mutex, wait_ms) };
                let taken = waited == WAIT_OBJECT_0 || waited == WAIT_ABANDONED;
                let _ = taken_tx.send(taken);
                if taken {
                    // Held until the lock is let go, or the process ends.
                    let _ = release_rx.recv();
                    // SAFETY: this thread took the mutex.
                    let _ = unsafe { ReleaseMutex(mutex) };
                }
                // SAFETY: the handle created above.
                let _ = unsafe { CloseHandle(mutex) };
            })
            .ok()?;
        if taken_rx.recv().unwrap_or(false) {
            Some(InstanceLock {
                release: Some(release_tx),
                holder: Some(holder),
            })
        } else {
            let _ = holder.join();
            None
        }
    }

    /// The pipe's security: the session's user, SYSTEM and Administrators, nobody else.
    fn security_descriptor() -> Result<PSECURITY_DESCRIPTOR, String> {
        let user = crate::elevation::current()
            .sid
            .map(|sid| format!("(A;;GA;;;{sid})"))
            .unwrap_or_default();
        let sddl = HSTRING::from(format!("D:P(A;;GA;;;SY)(A;;GA;;;BA){user}"));
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        // SAFETY: a valid SDDL string; the descriptor is freed with LocalFree by the caller.
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(sddl.as_ptr()),
                1,
                &mut descriptor,
                None,
            )
        }
        .map_err(|e| format!("the control pipe's security: {e}"))?;
        Ok(descriptor)
    }

    fn create(name: &HSTRING, first: bool) -> Result<HANDLE, String> {
        let descriptor = security_descriptor()?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: u32::try_from(std::mem::size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(0),
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: false.into(),
        };
        let open_mode = if first {
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE
        } else {
            PIPE_ACCESS_DUPLEX
        };
        let size = u32::try_from(MESSAGE).unwrap_or(1024);
        // SAFETY: a valid name and security attributes, alive for the call.
        let pipe = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                open_mode,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                size,
                size,
                0,
                Some(&attributes),
            )
        };
        // SAFETY: frees the descriptor ConvertStringSecurityDescriptor allocated.
        let _ = unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
        if pipe.is_invalid() {
            // SAFETY: the error of the call just made.
            let error = unsafe { GetLastError() };
            return Err(format!(
                "the control pipe: {}",
                windows::core::Error::from(error)
            ));
        }
        Ok(pipe)
    }

    /// The pipe's handle, moved to the thread that serves it and used there only.
    struct PipeHandle(HANDLE);

    // SAFETY: a kernel handle, usable from any thread; only the serving thread uses it.
    unsafe impl Send for PipeHandle {}

    impl PipeHandle {
        fn handle(&self) -> HANDLE {
            self.0
        }
    }

    pub(super) fn serve(name: &str, answer: Answer) -> Result<std::thread::JoinHandle<()>, String> {
        let wide = HSTRING::from(name);
        // Made here, so that a pipe someone else holds is an error for the caller.
        let first = PipeHandle(create(&wide, true)?);
        let pipe_name = name.to_owned();
        std::thread::Builder::new()
            .name("okur-control".to_owned())
            .spawn(move || {
                let pipe = first.handle();
                loop {
                    // SAFETY: the pipe created above; a client that came first is connected.
                    let connected = unsafe { ConnectNamedPipe(pipe, None) };
                    let connected = connected.is_ok()
                        // SAFETY: the error of the call just made.
                        || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
                    if connected {
                        let mut buffer = [0u8; MESSAGE];
                        let mut read = 0u32;
                        // SAFETY: a buffer of MESSAGE bytes.
                        let request =
                            unsafe { ReadFile(pipe, Some(&mut buffer), Some(&mut read), None) }
                                .ok()
                                .and_then(|()| buffer.get(..usize::try_from(read).ok()?))
                                .map(|bytes| String::from_utf8_lossy(bytes).trim().to_owned());
                        if let Some(request) = request {
                            let reply = answer(&request);
                            let mut written = 0u32;
                            // SAFETY: the reply's bytes, alive for the call.
                            let _ = unsafe {
                                WriteFile(pipe, Some(reply.as_bytes()), Some(&mut written), None)
                            };
                            // SAFETY: the connected pipe.
                            let _ = unsafe { FlushFileBuffers(pipe) };
                        }
                    }
                    // SAFETY: the pipe, connected or not.
                    let _ = unsafe { DisconnectNamedPipe(pipe) };
                    if !connected {
                        tracing::warn!(pipe = %pipe_name, "control pipe: no client connected");
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            })
            .map_err(|e| format!("the control pipe's thread: {e}"))
    }

    pub(super) fn request(name: &str, request: &str, timeout: Duration) -> Result<String, String> {
        let wide = HSTRING::from(name);
        let mut reply = [0u8; MESSAGE];
        let mut read = 0u32;
        let timeout_ms = u32::try_from(timeout.as_millis())
            .unwrap_or(u32::MAX)
            .max(1);
        let size = u32::try_from(request.len()).map_err(|e| e.to_string())?;
        // SAFETY: the request's bytes and a reply buffer of MESSAGE bytes, alive for the call.
        let called = unsafe {
            CallNamedPipeW(
                PCWSTR(wide.as_ptr()),
                Some(request.as_ptr().cast()),
                size,
                Some(reply.as_mut_ptr().cast()),
                u32::try_from(MESSAGE).unwrap_or(1024),
                &mut read,
                timeout_ms,
            )
        };
        if !called.as_bool() {
            // SAFETY: the error of the call just made.
            let error = unsafe { GetLastError() };
            return Err(format!("{name}: {}", windows::core::Error::from(error)));
        }
        let bytes = reply
            .get(..usize::try_from(read).map_err(|e| e.to_string())?)
            .unwrap_or_default();
        Ok(String::from_utf8_lossy(bytes).trim().to_owned())
    }

    pub(super) fn all_pipes() -> Vec<String> {
        let pattern = HSTRING::from(r"\\.\pipe\*");
        let mut data = WIN32_FIND_DATAW::default();
        // SAFETY: a valid pattern and out-pointer; the search is closed below.
        let Ok(search) = (unsafe { FindFirstFileW(PCWSTR(pattern.as_ptr()), &mut data) }) else {
            return Vec::new();
        };
        let mut pipes = Vec::new();
        loop {
            let len = data
                .cFileName
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(data.cFileName.len());
            let name = String::from_utf16_lossy(data.cFileName.get(..len).unwrap_or_default());
            if name
                .get(..PIPE_PREFIX.len())
                .is_some_and(|start| start.eq_ignore_ascii_case(PIPE_PREFIX))
            {
                pipes.push(format!(r"\\.\pipe\{name}"));
            }
            // SAFETY: the open search and the out-pointer.
            if unsafe { FindNextFileW(search, &mut data) }.is_err() {
                break;
            }
        }
        // SAFETY: the search opened above.
        let _ = unsafe { FindClose(search) };
        pipes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_carry_the_desktop_and_the_session() {
        assert_eq!(lock_name("Default"), r"Local\Okur-Default");
        assert_eq!(lock_name("Winlogon"), r"Local\Okur-Winlogon");
        assert_eq!(pipe_name(1, "Default"), r"\\.\pipe\okur-1-Default");
        // Nothing of a desktop's name can reach outside the object's namespace.
        assert_eq!(lock_name(r"..\Global\x y"), r"Local\Okur-___Global_x_y");
    }

    /// A second copy cannot take a lock the first holds; once it is let go, it can.
    #[cfg(windows)]
    #[test]
    fn one_copy_holds_the_lock_at_a_time() {
        let name = format!(r"Local\Okur-test-{}-lock", std::process::id());
        let first = InstanceLock::acquire(&name, Duration::from_millis(10));
        assert!(first.is_some());
        assert!(InstanceLock::acquire(&name, Duration::from_millis(50)).is_none());
        if let Some(first) = first {
            first.release();
        }
        assert!(InstanceLock::acquire(&name, Duration::from_millis(50)).is_some());
    }

    /// A request reaches the server and its reply comes back; a pipe nobody serves is an
    /// error, as is serving a pipe twice.
    #[cfg(windows)]
    #[test]
    fn the_control_pipe_answers_one_line() {
        let name = format!(r"\\.\pipe\okur-test-{}-pipe", std::process::id());
        let server = ControlServer::start(&name, Box::new(|request| format!("you said {request}")));
        assert!(server.is_ok(), "{:?}", server.err());
        assert_eq!(
            request(&name, "ping\n", Duration::from_secs(2))
                .ok()
                .as_deref(),
            Some("you said ping")
        );
        assert_eq!(
            request(&name, "status", Duration::from_secs(2))
                .ok()
                .as_deref(),
            Some("you said status")
        );
        assert!(ControlServer::start(&name, Box::new(|_| String::new())).is_err());
        assert!(all_pipes().contains(&name), "{:?}", all_pipes());
        let nobody = format!(r"\\.\pipe\okur-test-{}-none", std::process::id());
        assert!(request(&nobody, "ping", Duration::from_millis(50)).is_err());
    }
}
