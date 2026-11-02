//! Which session and desktop Okur runs on.
//!
//! A low-level keyboard hook sees only the input of the desktop its thread is on, so each
//! desktop needs its own Okur (`docs/references/startup-and-secure-screens-proposal.md`,
//! §2.1). A user's applications live on `WinSta0\Default`; the sign-in screen, the
//! Ctrl+Alt+Delete options, the credential screen after a lock and secure UAC prompts on
//! `WinSta0\Winlogon`, the secure desktop. Off Windows there is one desktop, "Default".

/// The secure desktop's name.
pub const SECURE_DESKTOP: &str = "Winlogon";

/// Whether a desktop is the secure one (names are compared without regard to case).
pub fn is_secure_desktop(name: &str) -> bool {
    name.eq_ignore_ascii_case(SECURE_DESKTOP)
}

/// The desktop the calling thread is on: a process started on `Winlogon` has `Winlogon` for
/// its threads, one started by the user `Default`.
pub fn thread_desktop() -> Option<String> {
    #[cfg(windows)]
    {
        windows_impl::thread_desktop()
    }
    #[cfg(not(windows))]
    {
        Some("Default".to_owned())
    }
}

/// The process's window station ("WinSta0" for an interactive session).
pub fn window_station() -> Option<String> {
    #[cfg(windows)]
    {
        windows_impl::window_station()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// The desktop that receives the user's input now; `None` when it cannot be opened, which is
/// what a user's process gets while the secure desktop has the input.
pub fn input_desktop() -> Option<String> {
    #[cfg(windows)]
    {
        windows_impl::input_desktop()
    }
    #[cfg(not(windows))]
    {
        Some("Default".to_owned())
    }
}

/// The Windows session this process runs in (0 off Windows).
pub fn session_id() -> Option<u32> {
    #[cfg(windows)]
    {
        windows_impl::session_id()
    }
    #[cfg(not(windows))]
    {
        Some(0)
    }
}

#[cfg(windows)]
mod windows_impl {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, GetProcessWindowStation, GetThreadDesktop, GetUserObjectInformationW,
        OpenInputDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_READOBJECTS, UOI_NAME,
    };
    use windows::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};

    /// A desktop's or window station's name.
    fn object_name(object: HANDLE) -> Option<String> {
        let mut name = [0u16; 256];
        let mut needed = 0u32;
        let size = u32::try_from(std::mem::size_of_val(&name)).ok()?;
        // SAFETY: a buffer of `size` bytes for the name.
        unsafe {
            GetUserObjectInformationW(
                object,
                UOI_NAME,
                Some(name.as_mut_ptr().cast()),
                size,
                Some(&mut needed),
            )
        }
        .ok()?;
        let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        Some(String::from_utf16_lossy(name.get(..len)?))
    }

    pub(super) fn thread_desktop() -> Option<String> {
        // SAFETY: the calling thread's desktop; the handle is not to be closed.
        let desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) }.ok()?;
        object_name(HANDLE(desktop.0))
    }

    pub(super) fn window_station() -> Option<String> {
        // SAFETY: the process's window station; the handle is not to be closed.
        let station = unsafe { GetProcessWindowStation() }.ok()?;
        object_name(HANDLE(station.0))
    }

    pub(super) fn input_desktop() -> Option<String> {
        // SAFETY: opens the input desktop for reading its name; closed below.
        let desktop =
            unsafe { OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) }
                .ok()?;
        let name = object_name(HANDLE(desktop.0));
        // SAFETY: the handle opened above.
        let _ = unsafe { CloseDesktop(desktop) };
        name
    }

    pub(super) fn session_id() -> Option<u32> {
        let mut session = 0u32;
        // SAFETY: a valid out-pointer.
        unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) }.ok()?;
        Some(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_secure_desktop_is_winlogon_in_any_case() {
        assert!(is_secure_desktop("Winlogon"));
        assert!(is_secure_desktop("winlogon"));
        assert!(!is_secure_desktop("Default"));
    }

    /// The test's own thread is on a desktop with a name.
    #[test]
    fn this_thread_has_a_desktop() {
        assert!(thread_desktop().is_some_and(|name| !name.is_empty()));
        assert!(session_id().is_some());
    }
}
