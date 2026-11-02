//! The Windows registry, as `okur doctor` reads it and `okur-setup` writes it, always in its
//! 64-bit view. The screen reader itself only reads. [`MemoryRegistry`] stands in for it in
//! tests, so that nothing a test does reaches the machine's registry.

use std::collections::BTreeMap;

/// A hive Okur reads or the installer writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Hive {
    LocalMachine,
    CurrentUser,
}

impl Hive {
    /// Its short name, as `reg.exe` writes it.
    pub fn short_name(self) -> &'static str {
        match self {
            Hive::LocalMachine => "HKLM",
            Hive::CurrentUser => "HKCU",
        }
    }
}

/// A value: the kinds Okur writes, and any other kind as `Other`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    String(String),
    Dword(u32),
    Other,
}

impl RegValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            RegValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_dword(&self) -> Option<u32> {
        match self {
            RegValue::Dword(d) => Some(*d),
            _ => None,
        }
    }

    /// The value as `okur doctor` prints it.
    pub fn display(&self) -> String {
        match self {
            RegValue::String(s) => s.clone(),
            RegValue::Dword(d) => d.to_string(),
            RegValue::Other => "(another type)".to_owned(),
        }
    }
}

/// The registry, by hive, key path and value name ("" is a key's default value). Names are
/// compared without regard to case, as Windows compares them.
pub trait Registry {
    fn value(&self, hive: Hive, key: &str, name: &str) -> Option<RegValue>;
    /// Every value of a key, in the registry's order; `None` when the key does not exist.
    fn values(&self, hive: Hive, key: &str) -> Option<Vec<(String, RegValue)>>;
    /// Sets a value, creating its key.
    fn set(&mut self, hive: Hive, key: &str, name: &str, value: &RegValue) -> Result<(), String>;
    /// Deletes a value; one that does not exist is no error.
    fn delete_value(&mut self, hive: Hive, key: &str, name: &str) -> Result<(), String>;
    /// Deletes a key with everything under it; one that does not exist is no error.
    fn delete_tree(&mut self, hive: Hive, key: &str) -> Result<(), String>;

    fn string(&self, hive: Hive, key: &str, name: &str) -> Option<String> {
        self.value(hive, key, name)
            .and_then(|v| v.as_str().map(str::to_owned))
    }
}

/// A registry in memory, for tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryRegistry {
    /// Keys by hive and lowercase path; each key's values by lowercase name, with the name as
    /// written.
    keys: BTreeMap<(Hive, String), BTreeMap<String, (String, RegValue)>>,
}

impl MemoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn path(key: &str) -> String {
        key.trim_matches('\\').to_lowercase()
    }

    /// Every key, as `HKLM\path`, for assertions.
    pub fn key_paths(&self) -> Vec<String> {
        self.keys
            .keys()
            .map(|(hive, path)| format!("{}\\{path}", hive.short_name()))
            .collect()
    }
}

impl Registry for MemoryRegistry {
    fn value(&self, hive: Hive, key: &str, name: &str) -> Option<RegValue> {
        self.keys
            .get(&(hive, Self::path(key)))?
            .get(&name.to_lowercase())
            .map(|(_, value)| value.clone())
    }

    fn values(&self, hive: Hive, key: &str) -> Option<Vec<(String, RegValue)>> {
        self.keys
            .get(&(hive, Self::path(key)))
            .map(|values| values.values().cloned().collect())
    }

    fn set(&mut self, hive: Hive, key: &str, name: &str, value: &RegValue) -> Result<(), String> {
        self.keys
            .entry((hive, Self::path(key)))
            .or_default()
            .insert(name.to_lowercase(), (name.to_owned(), value.clone()));
        Ok(())
    }

    fn delete_value(&mut self, hive: Hive, key: &str, name: &str) -> Result<(), String> {
        if let Some(values) = self.keys.get_mut(&(hive, Self::path(key))) {
            values.remove(&name.to_lowercase());
        }
        Ok(())
    }

    fn delete_tree(&mut self, hive: Hive, key: &str) -> Result<(), String> {
        let path = Self::path(key);
        let below = format!("{path}\\");
        self.keys
            .retain(|(h, p), _| *h != hive || (*p != path && !p.starts_with(&below)));
        Ok(())
    }
}

/// The machine's registry. Off Windows, reads find nothing and writes fail.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRegistry;

#[cfg(not(windows))]
impl Registry for SystemRegistry {
    fn value(&self, _: Hive, _: &str, _: &str) -> Option<RegValue> {
        None
    }

    fn values(&self, _: Hive, _: &str) -> Option<Vec<(String, RegValue)>> {
        None
    }

    fn set(&mut self, _: Hive, _: &str, _: &str, _: &RegValue) -> Result<(), String> {
        Err("the registry is Windows'".to_owned())
    }

    fn delete_value(&mut self, _: Hive, _: &str, _: &str) -> Result<(), String> {
        Err("the registry is Windows'".to_owned())
    }

    fn delete_tree(&mut self, _: Hive, _: &str) -> Result<(), String> {
        Err("the registry is Windows'".to_owned())
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{Hive, RegValue, Registry, SystemRegistry};
    use windows::core::{HSTRING, PCWSTR, PWSTR};
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegDeleteValueW, RegEnumValueW,
        RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY, KEY_WRITE, REG_DWORD, REG_EXPAND_SZ,
        REG_OPTION_NON_VOLATILE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
    };

    fn root(hive: Hive) -> HKEY {
        match hive {
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
            Hive::CurrentUser => HKEY_CURRENT_USER,
        }
    }

    /// An open key, closed when dropped.
    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // SAFETY: a key this module opened.
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }

    fn open(hive: Hive, path: &str, access: REG_SAM_FLAGS) -> Option<Key> {
        let path = HSTRING::from(path);
        let mut key = HKEY::default();
        // SAFETY: a valid path and out-pointer.
        let opened = unsafe {
            RegOpenKeyExW(
                root(hive),
                PCWSTR(path.as_ptr()),
                None,
                access | KEY_WOW64_64KEY,
                &mut key,
            )
        };
        (opened == ERROR_SUCCESS).then_some(Key(key))
    }

    fn checked(result: WIN32_ERROR, what: &str) -> Result<(), String> {
        if result == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("{what}: {}", windows::core::Error::from(result)))
        }
    }

    /// A value's raw data as UTF-16 units or a DWORD.
    fn decode(kind: REG_VALUE_TYPE, data: &[u8]) -> RegValue {
        if kind == REG_SZ || kind == REG_EXPAND_SZ {
            let units: Vec<u16> = data
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_le_bytes(*pair))
                .collect();
            let len = units.iter().position(|&c| c == 0).unwrap_or(units.len());
            RegValue::String(String::from_utf16_lossy(
                units.get(..len).unwrap_or_default(),
            ))
        } else if kind == REG_DWORD {
            data.get(..4)
                .and_then(|b| <[u8; 4]>::try_from(b).ok())
                .map_or(RegValue::Other, |b| RegValue::Dword(u32::from_le_bytes(b)))
        } else {
            RegValue::Other
        }
    }

    impl Registry for SystemRegistry {
        fn value(&self, hive: Hive, key: &str, name: &str) -> Option<RegValue> {
            let key = open(hive, key, KEY_READ)?;
            let name = HSTRING::from(name);
            let mut kind = REG_VALUE_TYPE::default();
            let mut size = 0u32;
            // SAFETY: a size query on an open key.
            let sized = unsafe {
                RegQueryValueExW(
                    key.0,
                    PCWSTR(name.as_ptr()),
                    None,
                    Some(&mut kind),
                    None,
                    Some(&mut size),
                )
            };
            if sized != ERROR_SUCCESS {
                return None;
            }
            let mut data = vec![0u8; usize::try_from(size).ok()?];
            // SAFETY: a buffer of `size` bytes.
            let read = unsafe {
                RegQueryValueExW(
                    key.0,
                    PCWSTR(name.as_ptr()),
                    None,
                    Some(&mut kind),
                    Some(data.as_mut_ptr()),
                    Some(&mut size),
                )
            };
            if read != ERROR_SUCCESS {
                return None;
            }
            data.truncate(usize::try_from(size).ok()?);
            Some(decode(kind, &data))
        }

        fn values(&self, hive: Hive, key: &str) -> Option<Vec<(String, RegValue)>> {
            let key = open(hive, key, KEY_READ)?;
            let mut values = Vec::new();
            for index in 0u32.. {
                let mut name = vec![0u16; 16_384];
                let mut name_len = u32::try_from(name.len()).ok()?;
                let mut data = vec![0u8; 65_536];
                let mut data_len = u32::try_from(data.len()).ok()?;
                let mut kind = 0u32;
                // SAFETY: buffers of the lengths given.
                let read = unsafe {
                    RegEnumValueW(
                        key.0,
                        index,
                        Some(PWSTR(name.as_mut_ptr())),
                        &mut name_len,
                        None,
                        Some(&mut kind),
                        Some(data.as_mut_ptr()),
                        Some(&mut data_len),
                    )
                };
                if read != ERROR_SUCCESS {
                    break;
                }
                let name = String::from_utf16_lossy(
                    name.get(..usize::try_from(name_len).ok()?)
                        .unwrap_or_default(),
                );
                data.truncate(usize::try_from(data_len).ok()?);
                values.push((name, decode(REG_VALUE_TYPE(kind), &data)));
            }
            Some(values)
        }

        fn set(
            &mut self,
            hive: Hive,
            key: &str,
            name: &str,
            value: &RegValue,
        ) -> Result<(), String> {
            let path = HSTRING::from(key);
            let mut handle = HKEY::default();
            // SAFETY: creates or opens the key for writing; closed by `Key`.
            checked(
                unsafe {
                    RegCreateKeyExW(
                        root(hive),
                        PCWSTR(path.as_ptr()),
                        None,
                        PCWSTR::null(),
                        REG_OPTION_NON_VOLATILE,
                        KEY_WRITE | KEY_WOW64_64KEY,
                        None,
                        &mut handle,
                        None,
                    )
                },
                key,
            )?;
            let handle = Key(handle);
            let (kind, data) = match value {
                RegValue::String(text) => (
                    REG_SZ,
                    text.encode_utf16()
                        .chain(std::iter::once(0))
                        .flat_map(u16::to_le_bytes)
                        .collect::<Vec<u8>>(),
                ),
                RegValue::Dword(number) => (REG_DWORD, number.to_le_bytes().to_vec()),
                RegValue::Other => return Err(format!("{key}\\{name}: no data to write")),
            };
            let name_w = HSTRING::from(name);
            // SAFETY: the open key and the value's bytes.
            checked(
                unsafe {
                    RegSetValueExW(handle.0, PCWSTR(name_w.as_ptr()), None, kind, Some(&data))
                },
                &format!("{key}\\{name}"),
            )
        }

        fn delete_value(&mut self, hive: Hive, key: &str, name: &str) -> Result<(), String> {
            let Some(handle) = open(hive, key, KEY_WRITE) else {
                return Ok(());
            };
            let name_w = HSTRING::from(name);
            // SAFETY: the open key.
            let result = unsafe { RegDeleteValueW(handle.0, PCWSTR(name_w.as_ptr())) };
            if result == ERROR_FILE_NOT_FOUND {
                return Ok(());
            }
            checked(result, &format!("{key}\\{name}"))
        }

        fn delete_tree(&mut self, hive: Hive, key: &str) -> Result<(), String> {
            // The key's parent, in the 64-bit view, and the key's own name under it.
            let (parent, leaf) = key.rsplit_once('\\').unwrap_or(("", key));
            let Some(handle) = open(hive, parent, KEY_WRITE | KEY_READ) else {
                return Ok(());
            };
            let leaf = HSTRING::from(leaf);
            // SAFETY: the open parent key.
            let result = unsafe { RegDeleteTreeW(handle.0, PCWSTR(leaf.as_ptr())) };
            if result == ERROR_FILE_NOT_FOUND {
                return Ok(());
            }
            checked(result, key)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memory_registry_compares_names_without_case() {
        let mut registry = MemoryRegistry::new();
        let key = r"SOFTWARE\Okur";
        let _ = registry.set(
            Hive::LocalMachine,
            key,
            "InstallDir",
            &RegValue::String("C:\\Okur".into()),
        );
        assert_eq!(
            registry.string(Hive::LocalMachine, r"software\okur", "installdir"),
            Some("C:\\Okur".to_owned())
        );
        assert_eq!(registry.value(Hive::CurrentUser, key, "InstallDir"), None);
        let _ = registry.set(
            Hive::LocalMachine,
            r"SOFTWARE\Okur\Sub",
            "",
            &RegValue::Dword(1),
        );
        let _ = registry.delete_tree(Hive::LocalMachine, key);
        assert!(
            registry.key_paths().is_empty(),
            "{:?}",
            registry.key_paths()
        );
    }
}
