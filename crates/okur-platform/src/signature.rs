//! An executable's Authenticode signature and its embedded manifest, read only.
//!
//! UI Access needs both (`docs/references/startup-and-secure-screens-proposal.md`, §2.6): a
//! manifest asking for `uiAccess="true"`, and a signature that chains to a root in the local
//! machine's Trusted Root store. `okur doctor` reports them, and `okur-setup` installs the UI
//! Access build only when its signature verifies on the machine.

use std::path::Path;

/// What `WinVerifyTrust` said of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureStatus {
    /// Signed, and the chain ends in a trusted root.
    Valid,
    /// No signature (`TRUST_E_NOSIGNATURE`).
    NotSigned,
    /// Signed, but the root is not trusted here (`CERT_E_UNTRUSTEDROOT`): a development
    /// certificate on a machine that does not trust it.
    UntrustedRoot,
    /// The certificate has expired, and the signature has no timestamp (`CERT_E_EXPIRED`).
    Expired,
    /// The file changed after it was signed (`TRUST_E_BAD_DIGEST`).
    BadDigest,
    /// The certificate is explicitly distrusted (`TRUST_E_EXPLICIT_DISTRUST`).
    Distrusted,
    /// Any other answer, as its HRESULT.
    Other(i32),
    /// Not checked: off Windows.
    Unavailable,
}

impl SignatureStatus {
    /// The status `WinVerifyTrust`'s answer means.
    pub fn from_hresult(hr: i32) -> Self {
        // The HRESULTs as unsigned, as Windows' headers write them.
        match hr as u32 {
            0 => Self::Valid,
            0x800B_0100 => Self::NotSigned,
            0x800B_0109 => Self::UntrustedRoot,
            0x800B_0101 => Self::Expired,
            0x8009_6010 => Self::BadDigest,
            0x800B_0111 => Self::Distrusted,
            _ => Self::Other(hr),
        }
    }

    pub fn is_valid(self) -> bool {
        self == Self::Valid
    }

    /// A line for `okur doctor`.
    pub fn describe(self) -> String {
        match self {
            Self::Valid => "valid".to_owned(),
            Self::NotSigned => "not signed".to_owned(),
            Self::UntrustedRoot => "signed, root not trusted on this machine".to_owned(),
            Self::Expired => "signed, certificate expired".to_owned(),
            Self::BadDigest => "signed, but the file changed after signing".to_owned(),
            Self::Distrusted => "signed, certificate explicitly distrusted".to_owned(),
            Self::Other(hr) => format!("not verified (0x{:08X})", hr as u32),
            Self::Unavailable => "not checked on this platform".to_owned(),
        }
    }
}

/// What an executable's manifest asks of UAC.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManifestFacts {
    /// `requestedExecutionLevel`'s `level`: `asInvoker`, `requireAdministrator`, ...
    pub level: Option<String>,
    /// Its `uiAccess`.
    pub ui_access: Option<bool>,
}

impl ManifestFacts {
    /// Reads the `requestedExecutionLevel` element of a manifest's text.
    pub fn parse(manifest: &str) -> Self {
        let Some((_, after)) = manifest.split_once("requestedExecutionLevel") else {
            return Self::default();
        };
        let element = after.split('>').next().unwrap_or_default();
        Self {
            level: attribute(element, "level"),
            ui_access: attribute(element, "uiAccess").map(|v| v.eq_ignore_ascii_case("true")),
        }
    }

    /// A line for `okur doctor`.
    pub fn describe(&self) -> String {
        format!(
            "level={} uiAccess={}",
            self.level.as_deref().unwrap_or("(none)"),
            self.ui_access
                .map_or("(none)".to_owned(), |ui| ui.to_string())
        )
    }
}

/// An attribute's value in an element's text, quoted either way.
fn attribute(element: &str, name: &str) -> Option<String> {
    let mut rest = element;
    while let Some(at) = rest.find(name) {
        let before = rest.get(..at)?;
        let after = rest.get(at + name.len()..)?;
        rest = after;
        // A whole attribute name, not the end of a longer one.
        if !before.ends_with(char::is_whitespace) {
            continue;
        }
        let Some(value) = after.trim_start().strip_prefix('=') else {
            continue;
        };
        let value = value.trim_start();
        let quote = value.chars().next()?;
        if quote != '"' && quote != '\'' {
            continue;
        }
        let inner = value.get(1..)?;
        return inner.split(quote).next().map(str::to_owned);
    }
    None
}

/// Verifies a file's Authenticode signature against the machine's trusted roots, without
/// any user interface and without revocation checks.
pub fn verify_file(path: &Path) -> SignatureStatus {
    #[cfg(windows)]
    {
        windows_impl::verify_file(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        SignatureStatus::Unavailable
    }
}

/// The text of an executable's embedded manifest (resource 1 of type `RT_MANIFEST`), loaded
/// as data only.
pub fn embedded_manifest(path: &Path) -> Option<String> {
    #[cfg(windows)]
    {
        windows_impl::embedded_manifest(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::SignatureStatus;
    use std::path::Path;
    use windows::core::{GUID, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{FreeLibrary, HANDLE, HWND};
    use windows::Win32::Security::WinTrust::{
        WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0,
        WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE,
        WTD_STATEACTION_VERIFY, WTD_UI_NONE,
    };
    use windows::Win32::System::LibraryLoader::{
        FindResourceW, LoadLibraryExW, LoadResource, LockResource, SizeofResource,
        LOAD_LIBRARY_AS_DATAFILE, LOAD_LIBRARY_AS_IMAGE_RESOURCE,
    };
    use windows::Win32::UI::WindowsAndMessaging::RT_MANIFEST;

    pub(super) fn verify_file(path: &Path) -> SignatureStatus {
        let path = HSTRING::from(path.as_os_str());
        let mut file = WINTRUST_FILE_INFO {
            cbStruct: u32::try_from(std::mem::size_of::<WINTRUST_FILE_INFO>()).unwrap_or(0),
            pcwszFilePath: PCWSTR(path.as_ptr()),
            hFile: HANDLE::default(),
            pgKnownSubject: std::ptr::null_mut(),
        };
        let mut data = WINTRUST_DATA {
            cbStruct: u32::try_from(std::mem::size_of::<WINTRUST_DATA>()).unwrap_or(0),
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_NONE,
            dwUnionChoice: WTD_CHOICE_FILE,
            Anonymous: WINTRUST_DATA_0 { pFile: &mut file },
            dwStateAction: WTD_STATEACTION_VERIFY,
            ..WINTRUST_DATA::default()
        };
        let mut action: GUID = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        // SAFETY: the structures above outlive both calls; no window is shown (WTD_UI_NONE).
        let result = unsafe {
            WinVerifyTrust(
                HWND(-1isize as *mut _),
                &mut action,
                std::ptr::from_mut(&mut data).cast(),
            )
        };
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        // SAFETY: closes the state the verification opened.
        let _ = unsafe {
            WinVerifyTrust(
                HWND(-1isize as *mut _),
                &mut action,
                std::ptr::from_mut(&mut data).cast(),
            )
        };
        SignatureStatus::from_hresult(result)
    }

    pub(super) fn embedded_manifest(path: &Path) -> Option<String> {
        let path = HSTRING::from(path.as_os_str());
        // SAFETY: the file is mapped as data only: none of its code runs.
        let module = unsafe {
            LoadLibraryExW(
                PCWSTR(path.as_ptr()),
                None,
                LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE,
            )
        }
        .ok()?;
        // SAFETY: resource lookups in the module mapped above, freed below; the bytes are
        // copied before it is.
        let text = unsafe {
            let resource = FindResourceW(Some(module), PCWSTR(1 as _), RT_MANIFEST);
            if resource.is_invalid() {
                None
            } else {
                let size = usize::try_from(SizeofResource(Some(module), resource)).ok()?;
                let loaded = LoadResource(Some(module), resource).ok()?;
                let bytes = LockResource(loaded).cast::<u8>();
                (!bytes.is_null()).then(|| {
                    let bytes = std::slice::from_raw_parts(bytes, size);
                    String::from_utf8_lossy(bytes)
                        .trim_start_matches('\u{feff}')
                        .to_owned()
                })
            }
        };
        // SAFETY: the module mapped above.
        let _ = unsafe { FreeLibrary(module) };
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_answers_are_named() {
        assert_eq!(SignatureStatus::from_hresult(0), SignatureStatus::Valid);
        assert_eq!(
            SignatureStatus::from_hresult(0x800B_0100_u32 as i32),
            SignatureStatus::NotSigned
        );
        assert_eq!(
            SignatureStatus::from_hresult(0x800B_0109_u32 as i32),
            SignatureStatus::UntrustedRoot
        );
        assert_eq!(
            SignatureStatus::from_hresult(0x8009_6010_u32 as i32).describe(),
            "signed, but the file changed after signing"
        );
        assert_eq!(
            SignatureStatus::from_hresult(0x8000_4005_u32 as i32).describe(),
            "not verified (0x80004005)"
        );
    }

    #[test]
    fn a_manifests_execution_level_and_ui_access_are_read() {
        let manifest = r#"<?xml version="1.0"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3"><security><requestedPrivileges>
    <requestedExecutionLevel level='asInvoker' uiAccess='true' />
  </requestedPrivileges></security></trustInfo>
</assembly>"#;
        let facts = ManifestFacts::parse(manifest);
        assert_eq!(facts.level.as_deref(), Some("asInvoker"));
        assert_eq!(facts.ui_access, Some(true));
        assert_eq!(facts.describe(), "level=asInvoker uiAccess=true");
        let plain = ManifestFacts::parse(
            r#"<requestedExecutionLevel uiAccess="false" level="requireAdministrator"/>"#,
        );
        assert_eq!(plain.level.as_deref(), Some("requireAdministrator"));
        assert_eq!(plain.ui_access, Some(false));
        assert_eq!(
            ManifestFacts::parse("<assembly/>"),
            ManifestFacts::default()
        );
    }
}
