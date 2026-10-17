//! Whether Okur runs with administrator rights.
//!
//! Windows keeps a process from reading, and from sending keys to, the windows of a process at
//! a higher integrity level (UIPI). An elevated application therefore shows nothing to an Okur
//! that is not elevated: Windows Settings' "Rename this PC" dialog runs in
//! `SystemSettingsAdminFlows` at High integrity, and UIA gave Okur only its empty panes, where
//! JAWS read "Rename your PC | Current PC name: … Edit". JAWS's own process (`jfw`) runs at
//! High integrity (dev box, 2026-10-17).

/// Whether this process runs elevated (always true off Windows, where nothing is kept out).
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        windows_impl::is_elevated()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

#[cfg(windows)]
mod windows_impl {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    pub(super) fn is_elevated() -> bool {
        let mut token = HANDLE::default();
        // SAFETY: the current process's pseudo-handle and a valid out-pointer.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut written = 0u32;
        let size = u32::try_from(std::mem::size_of::<TOKEN_ELEVATION>()).unwrap_or(0);
        // SAFETY: the buffer is a TOKEN_ELEVATION of `size` bytes, for the token just opened.
        let read = unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                Some(std::ptr::from_mut(&mut elevation).cast()),
                size,
                &mut written,
            )
        }
        .is_ok();
        // SAFETY: closes the handle opened above.
        let _ = unsafe { CloseHandle(token) };
        read && elevation.TokenIsElevated != 0
    }
}
