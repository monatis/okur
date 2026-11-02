//! Whether Okur runs with administrator rights or with UI Access, and at which integrity level.
//!
//! Windows keeps a process from reading, and from sending keys to, the windows of a process at
//! a higher integrity level (UIPI). An elevated application therefore shows nothing to an Okur
//! that is not elevated: Windows Settings' "Rename this PC" dialog runs in
//! `SystemSettingsAdminFlows` at High integrity, and UIA gave Okur only its empty panes, where
//! JAWS read "Rename your PC | Current PC name: … Edit". JAWS's own process (`jfw`) runs at
//! High integrity (dev box, 2026-10-17).
//!
//! The installed screen reader, `okur-sr.exe`, is a UI Access process
//! (`docs/references/startup-and-secure-screens-proposal.md`, §2.6): its token is the limited
//! one, so it is not "elevated", yet an administrator's copy runs at High integrity and reads
//! elevated windows. What counts is the token's integrity level and its `TokenUIAccess`.

/// Mandatory integrity levels, as the relative ids of their labels (`S-1-16-<rid>`).
pub mod integrity {
    pub const UNTRUSTED: u32 = 0;
    pub const LOW: u32 = 0x1000;
    pub const MEDIUM: u32 = 0x2000;
    pub const HIGH: u32 = 0x3000;
    pub const SYSTEM: u32 = 0x4000;
    pub const PROTECTED: u32 = 0x5000;

    /// A level's name, with what it is above its base: "Medium", "Medium+16" (a standard
    /// user's UI Access process, in Microsoft's words "medium+"), "High", "System".
    pub fn name(rid: u32) -> String {
        const NAMED: [(u32, &str); 6] = [
            (PROTECTED, "Protected"),
            (SYSTEM, "System"),
            (HIGH, "High"),
            (MEDIUM, "Medium"),
            (LOW, "Low"),
            (UNTRUSTED, "Untrusted"),
        ];
        NAMED
            .iter()
            .find(|(base, _)| rid >= *base)
            .map(|(base, name)| match rid - base {
                0 => (*name).to_owned(),
                above => format!("{name}+{above}"),
            })
            .unwrap_or_default()
    }
}

/// How a token stands to elevation (`TOKEN_ELEVATION_TYPE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElevationType {
    /// No split token: UAC is off, or the account is not an administrator, or it is SYSTEM.
    Default,
    /// The administrator's full token: elevated.
    Full,
    /// The administrator's limited token: what a non-elevated program runs with.
    Limited,
}

/// What a process's token says about it. A fact that could not be read is `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TokenFacts {
    /// The Windows session (0 for services, 1 for the first user signed in).
    pub session_id: Option<u32>,
    /// `DOMAIN\name`, or the SID where the account has no name.
    pub account: Option<String>,
    /// The account's SID, `S-1-5-...`.
    pub sid: Option<String>,
    /// The integrity level's relative id ([`integrity`]).
    pub integrity: Option<u32>,
    /// `TokenUIAccess`: the process may drive and read windows of every integrity level.
    pub ui_access: Option<bool>,
    /// `TokenElevation`.
    pub elevated: Option<bool>,
    pub elevation_type: Option<ElevationType>,
}

impl TokenFacts {
    /// Whether the windows of applications running as administrator can be read: an elevated
    /// process, or one at High integrity or above (an administrator's UI Access process). A
    /// standard user's UI Access process (Medium+16) "cannot access elevated UI" (Microsoft,
    /// Security Considerations for Assistive Technologies).
    pub fn reads_elevated_windows(&self) -> bool {
        self.elevated == Some(true) || self.integrity.is_some_and(|rid| rid >= integrity::HIGH)
    }
}

/// Whether this process runs elevated (always true off Windows, where nothing is kept out).
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        current().elevated.unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        true
    }
}

/// Whether this process has UI Access (never off Windows).
pub fn has_ui_access() -> bool {
    #[cfg(windows)]
    {
        current().ui_access.unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Whether this process can read the windows of applications running as administrator
/// ([`TokenFacts::reads_elevated_windows`]; always true off Windows).
pub fn reads_elevated_windows() -> bool {
    #[cfg(windows)]
    {
        current().reads_elevated_windows()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

/// This process's token.
pub fn current() -> TokenFacts {
    #[cfg(windows)]
    {
        windows_impl::current()
    }
    #[cfg(not(windows))]
    {
        TokenFacts::default()
    }
}

/// Another process's token, by process id: an error names what could not be opened.
pub fn of_process(pid: u32) -> Result<TokenFacts, String> {
    #[cfg(windows)]
    {
        windows_impl::of_process(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        Err("process tokens are only read on Windows".to_owned())
    }
}

#[cfg(windows)]
pub(crate) mod windows_impl {
    use super::{ElevationType, TokenFacts};
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL};
    use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows::Win32::Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, LookupAccountSidW,
        TokenElevation, TokenElevationType, TokenElevationTypeFull, TokenElevationTypeLimited,
        TokenIntegrityLevel, TokenSessionId, TokenUIAccess, TokenUser, PSID, SID_NAME_USE,
        TOKEN_ELEVATION, TOKEN_ELEVATION_TYPE, TOKEN_INFORMATION_CLASS, TOKEN_MANDATORY_LABEL,
        TOKEN_QUERY, TOKEN_USER,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    /// A handle closed when dropped.
    pub(crate) struct Owned(pub(crate) HANDLE);

    impl Drop for Owned {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                // SAFETY: a handle this module opened and nothing else closes.
                let _ = unsafe { CloseHandle(self.0) };
            }
        }
    }

    pub(super) fn current() -> TokenFacts {
        let mut token = HANDLE::default();
        // SAFETY: the current process's pseudo-handle and a valid out-pointer.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
            return TokenFacts::default();
        }
        facts(&Owned(token))
    }

    pub(super) fn of_process(pid: u32) -> Result<TokenFacts, String> {
        // SAFETY: plain opens with valid arguments; the handles are closed by `Owned`.
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
            .map(Owned)
            .map_err(|e| format!("process {pid}: {e}"))?;
        let mut token = HANDLE::default();
        // SAFETY: a process handle opened above and a valid out-pointer.
        unsafe { OpenProcessToken(process.0, TOKEN_QUERY, &mut token) }
            .map_err(|e| format!("process {pid}'s token: {e}"))?;
        Ok(facts(&Owned(token)))
    }

    /// A token information class read into a buffer aligned for any of its structures.
    fn information(token: &Owned, class: TOKEN_INFORMATION_CLASS) -> Option<Vec<u64>> {
        let mut needed = 0u32;
        // SAFETY: a size query (it fails with the size needed).
        let _ = unsafe { GetTokenInformation(token.0, class, None, 0, &mut needed) };
        if needed == 0 {
            return None;
        }
        let words = usize::try_from(needed).ok()?.div_ceil(8);
        let mut buffer = vec![0u64; words];
        // SAFETY: the buffer holds `needed` bytes, eight-byte aligned.
        unsafe {
            GetTokenInformation(
                token.0,
                class,
                Some(buffer.as_mut_ptr().cast()),
                needed,
                &mut needed,
            )
        }
        .ok()?;
        Some(buffer)
    }

    /// A DWORD-sized token information class.
    fn dword(token: &Owned, class: TOKEN_INFORMATION_CLASS) -> Option<u32> {
        let mut value = 0u32;
        let mut written = 0u32;
        // SAFETY: the buffer is one u32, which these classes fill.
        unsafe {
            GetTokenInformation(
                token.0,
                class,
                Some(std::ptr::from_mut(&mut value).cast()),
                4,
                &mut written,
            )
        }
        .ok()?;
        Some(value)
    }

    fn facts(token: &Owned) -> TokenFacts {
        let session_id = dword(token, TokenSessionId);
        let ui_access = dword(token, TokenUIAccess).map(|v| v != 0);
        let elevated = {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut written = 0u32;
            let size = u32::try_from(std::mem::size_of::<TOKEN_ELEVATION>()).unwrap_or(0);
            // SAFETY: the buffer is a TOKEN_ELEVATION of `size` bytes.
            unsafe {
                GetTokenInformation(
                    token.0,
                    TokenElevation,
                    Some(std::ptr::from_mut(&mut elevation).cast()),
                    size,
                    &mut written,
                )
            }
            .ok()
            .map(|()| elevation.TokenIsElevated != 0)
        };
        let elevation_type = dword(token, TokenElevationType).map(|v| {
            let kind = TOKEN_ELEVATION_TYPE(i32::try_from(v).unwrap_or(0));
            if kind == TokenElevationTypeFull {
                ElevationType::Full
            } else if kind == TokenElevationTypeLimited {
                ElevationType::Limited
            } else {
                ElevationType::Default
            }
        });
        let integrity = information(token, TokenIntegrityLevel).and_then(|buffer| {
            // SAFETY: the buffer holds a TOKEN_MANDATORY_LABEL whose SID points into it.
            unsafe {
                let label = &*buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>();
                last_sub_authority(label.Label.Sid)
            }
        });
        let user = information(token, TokenUser);
        let (account, sid) = user
            .as_ref()
            .map(|buffer| {
                // SAFETY: the buffer holds a TOKEN_USER whose SID points into it.
                let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
                (account_name(sid), sid_string(sid))
            })
            .unwrap_or_default();
        TokenFacts {
            session_id,
            account: account.or_else(|| sid.clone()),
            sid,
            integrity,
            ui_access,
            elevated,
            elevation_type,
        }
    }

    /// A SID's last sub-authority: an integrity label's level.
    ///
    /// # Safety
    /// `sid` must point to a valid SID.
    unsafe fn last_sub_authority(sid: PSID) -> Option<u32> {
        // SAFETY: the caller's valid SID.
        let count = unsafe { *GetSidSubAuthorityCount(sid) };
        let last = u32::from(count).checked_sub(1)?;
        // SAFETY: `last` is within the SID's sub-authorities.
        Some(unsafe { *GetSidSubAuthority(sid, last) })
    }

    /// `S-1-5-...` for a SID.
    pub(crate) fn sid_string(sid: PSID) -> Option<String> {
        let mut text = PWSTR::null();
        // SAFETY: a valid SID; the string is freed with LocalFree below.
        unsafe { ConvertSidToStringSidW(sid, &mut text) }.ok()?;
        // SAFETY: the string the call allocated, NUL-terminated.
        let value = unsafe { text.to_string() }.ok();
        // SAFETY: frees what ConvertSidToStringSidW allocated.
        let _ = unsafe { LocalFree(Some(HLOCAL(text.0.cast()))) };
        value
    }

    /// `DOMAIN\name` for a SID; `NT AUTHORITY\SYSTEM` for the system account.
    fn account_name(sid: PSID) -> Option<String> {
        let mut name = [0u16; 256];
        let mut domain = [0u16; 256];
        let (mut name_len, mut domain_len) = (256u32, 256u32);
        let mut kind = SID_NAME_USE::default();
        // SAFETY: buffers of the lengths given, and a valid SID.
        unsafe {
            LookupAccountSidW(
                None,
                sid,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                Some(PWSTR(domain.as_mut_ptr())),
                &mut domain_len,
                &mut kind,
            )
        }
        .ok()?;
        let text = |buffer: &[u16], len: u32| -> Option<String> {
            Some(String::from_utf16_lossy(
                buffer.get(..usize::try_from(len).ok()?)?,
            ))
        };
        let name = text(&name, name_len)?;
        let domain = text(&domain, domain_len)?;
        Some(if domain.is_empty() {
            name
        } else {
            format!("{domain}\\{name}")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity_levels_are_named_with_what_they_are_above() {
        assert_eq!(integrity::name(0x2000), "Medium");
        assert_eq!(integrity::name(0x2010), "Medium+16");
        assert_eq!(integrity::name(0x3000), "High");
        assert_eq!(integrity::name(0x4000), "System");
        assert_eq!(integrity::name(0x1000), "Low");
        assert_eq!(integrity::name(0), "Untrusted");
    }

    /// An administrator's UI Access copy (High, not elevated) reads elevated windows; a
    /// standard user's (Medium+16) does not, nor does a plain Medium process.
    #[test]
    fn elevated_windows_are_read_from_high_integrity() {
        let facts = |integrity: u32, elevated: bool, ui_access: bool| TokenFacts {
            integrity: Some(integrity),
            elevated: Some(elevated),
            ui_access: Some(ui_access),
            ..TokenFacts::default()
        };
        assert!(facts(0x3000, false, true).reads_elevated_windows());
        assert!(facts(0x3000, true, false).reads_elevated_windows());
        assert!(!facts(0x2010, false, true).reads_elevated_windows());
        assert!(!facts(0x2000, false, false).reads_elevated_windows());
    }

    /// This process's own token can be read (whatever its rights).
    #[cfg(windows)]
    #[test]
    fn the_current_token_is_read() {
        let facts = current();
        assert!(facts.integrity.is_some(), "{facts:?}");
        assert!(facts.sid.as_deref().is_some_and(|s| s.starts_with("S-1-")));
        assert!(facts.session_id.is_some());
        assert!(facts.ui_access.is_some());
    }
}
