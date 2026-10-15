//! The current virtual desktop's name, which JAWS speaks as the desktop's window title
//! ("Title is | Desktop 1" with focus on the desktop) [obs, 209 trial 1].
//!
//! Windows keeps the virtual desktops in the user's registry under
//! `Software\Microsoft\Windows\CurrentVersion\Explorer`:
//! - `VirtualDesktops\VirtualDesktopIDs`: the desktops' GUIDs in order, 16 bytes each;
//! - `VirtualDesktops\CurrentVirtualDesktop` (Windows 11), or
//!   `SessionInfo\<session>\VirtualDesktops\CurrentVirtualDesktop` (Windows 10);
//! - `VirtualDesktops\Desktops\{GUID}\Name`, only for desktops the user renamed.
//!
//! An unnamed desktop is "Desktop N" by its position; with no virtual desktops recorded,
//! the one desktop is "Desktop 1".

/// The name of desktop `index` (0-based) among `ids`, given the user-set `names`.
pub fn desktop_name(
    ids: &[[u8; 16]],
    current: Option<[u8; 16]>,
    name_of: impl Fn(&[u8; 16]) -> Option<String>,
) -> String {
    let index = current
        .and_then(|c| ids.iter().position(|id| *id == c))
        .unwrap_or(0);
    current
        .and_then(|c| name_of(&c))
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("Desktop {}", index + 1))
}

/// A GUID's registry form, `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`, from its 16 bytes
/// (the first three fields little-endian).
pub fn guid_string(bytes: &[u8; 16]) -> String {
    let [a0, a1, a2, a3, b0, b1, c0, c1, rest @ ..] = *bytes;
    let tail: String = rest.iter().map(|b| format!("{b:02X}")).collect();
    format!(
        "{{{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{}-{}}}",
        a3,
        a2,
        a1,
        a0,
        b1,
        b0,
        c1,
        c0,
        tail.get(..4).unwrap_or_default(),
        tail.get(4..).unwrap_or_default()
    )
}

#[cfg(windows)]
pub use reader::current_name;

#[cfg(windows)]
mod reader {
    use super::{desktop_name, guid_string};
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_BINARY, RRF_RT_REG_SZ,
    };

    const EXPLORER: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer";

    /// A registry value of the current user, as bytes.
    fn value(key: &str, name: &str, flags: u32) -> Option<Vec<u8>> {
        let key = HSTRING::from(key);
        let name = HSTRING::from(name);
        let mut size = 0u32;
        // SAFETY: a size query, then a read into a buffer of that size.
        unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
                windows::Win32::System::Registry::REG_ROUTINE_FLAGS(flags),
                None,
                None,
                Some(&mut size),
            )
            .ok()
            .ok()?;
            let mut buffer = vec![0u8; usize::try_from(size).ok()?];
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
                windows::Win32::System::Registry::REG_ROUTINE_FLAGS(flags),
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            )
            .ok()
            .ok()?;
            buffer.truncate(usize::try_from(size).ok()?);
            Some(buffer)
        }
    }

    fn guid(bytes: &[u8]) -> Option<[u8; 16]> {
        bytes.get(..16)?.try_into().ok()
    }

    /// The current virtual desktop's name ("Desktop 1" when none are recorded).
    pub fn current_name() -> String {
        use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
        use windows::Win32::System::Threading::GetCurrentProcessId;
        let desktops = format!(r"{EXPLORER}\VirtualDesktops");
        let ids: Vec<[u8; 16]> = value(&desktops, "VirtualDesktopIDs", RRF_RT_REG_BINARY.0)
            .map(|b| b.as_chunks::<16>().0.to_vec())
            .unwrap_or_default();
        let mut session = 0u32;
        // SAFETY: plain queries with a valid out-pointer.
        let _ = unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) };
        let current = value(&desktops, "CurrentVirtualDesktop", RRF_RT_REG_BINARY.0)
            .or_else(|| {
                value(
                    &format!(r"{EXPLORER}\SessionInfo\{session}\VirtualDesktops"),
                    "CurrentVirtualDesktop",
                    RRF_RT_REG_BINARY.0,
                )
            })
            .and_then(|b| guid(&b));
        desktop_name(&ids, current, |id| {
            let bytes = value(
                &format!(r"{desktops}\Desktops\{}", guid_string(id)),
                "Name",
                RRF_RT_REG_SZ.0,
            )?;
            let units: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .take_while(|u| *u != 0)
                .collect();
            Some(String::from_utf16_lossy(&units))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unnamed_desktops_are_numbered_and_names_win() {
        let a = [1u8; 16];
        let b = [2u8; 16];
        assert_eq!(desktop_name(&[], None, |_| None), "Desktop 1");
        assert_eq!(desktop_name(&[a, b], Some(b), |_| None), "Desktop 2");
        assert_eq!(
            desktop_name(&[a, b], Some(a), |_| Some("Work".into())),
            "Work"
        );
    }

    #[test]
    fn guids_print_in_registry_form() {
        let bytes = [
            0x78, 0x56, 0x34, 0x12, 0x34, 0x12, 0x78, 0x56, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34,
            0x56, 0x78,
        ];
        assert_eq!(
            guid_string(&bytes),
            "{12345678-1234-5678-9ABC-DEF012345678}"
        );
    }
}
