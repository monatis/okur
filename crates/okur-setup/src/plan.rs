//! What `okur-setup` installs and removes: the files and the machine's registry values of
//! `docs/references/startup-and-secure-screens-proposal.md`, §5.4, as data. M3 leaves out
//! the sign-in screen start (Okur's name in HKLM's `Configuration`), which comes with the
//! secure desktop's mode (M6); uninstall removes it all the same.

use std::path::{Path, PathBuf};

use okur_platform::ease_of_access;
use okur_platform::registry::{Hive, RegValue};

/// The product's folder name, under Program Files.
pub const FOLDER: &str = "Okur";

/// The installed screen reader.
pub const SCREEN_READER: &str = "okur-sr.exe";
/// The screen reader without UI Access, installed under [`SCREEN_READER`]'s name where that
/// one's signature does not verify.
pub const SCREEN_READER_NO_UI_ACCESS: &str = "okur-sr-no-uiaccess.exe";
/// The process that draws Okur's windows.
pub const UI: &str = "okur-ui.exe";
/// This installer, which stays as the uninstaller.
pub const SETUP: &str = "okur-setup.exe";
/// Documents installed beside the programs when they are found beside the installer.
pub const DOCUMENTS: [&str; 2] = ["LICENSE", "NOTICE"];
/// The read-only settings of the secure desktop's copy (M6), removed on uninstall.
pub const SECURE_SETTINGS: &str = "secure-settings";

/// Okur's own machine key.
pub const OKUR_KEY: &str = r"SOFTWARE\Okur";
/// Win+R's names: `okur` starts the screen reader, as `JAWS2025` starts JAWS.
pub const APP_PATH_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\okur.exe";
/// The programs every user's sign-in starts.
pub const RUN_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
/// The Run key's value for Okur.
pub const RUN_VALUE: &str = "Okur";
/// Apps & features' entry.
pub const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Okur";

/// Where Okur is installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallDir(pub PathBuf);

impl InstallDir {
    /// `%ProgramFiles%\Okur`: a folder only administrators can write, as UI Access requires.
    pub fn program_files() -> Option<Self> {
        std::env::var_os("ProgramW6432")
            .or_else(|| std::env::var_os("ProgramFiles"))
            .map(|root| Self(PathBuf::from(root).join(FOLDER)))
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    pub fn screen_reader(&self) -> PathBuf {
        self.file(SCREEN_READER)
    }
}

/// One value to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Write {
    pub hive: Hive,
    pub key: String,
    pub name: String,
    pub value: RegValue,
}

fn write(key: &str, name: &str, value: RegValue) -> Write {
    Write {
        hive: Hive::LocalMachine,
        key: key.to_owned(),
        name: name.to_owned(),
        value,
    }
}

fn text(value: impl Into<String>) -> RegValue {
    RegValue::String(value.into())
}

/// A path as Windows' Run key and uninstall strings want it, quoted.
fn quoted(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

/// What the machine's registry holds once Okur is installed in `dir`.
///
/// - The assistive-technology key, `TerminateOnDesktopSwitch=0` so that Windows does not
///   kill and restart Okur at every desktop switch; no `SecureDesktopAccommodation` (that
///   would name another AT for the secure desktop), no `CopySettingsToLockedDesktop` (§5.7),
///   no `PassiveAutoStartBehavior` (JAWS has none; J3 decides).
/// - `HKLM\SOFTWARE\Okur`, Win+R's `okur`, Apps & features' entry.
/// - With `start_after_sign_in`, the Run key's `--sign-in` start for every user.
pub fn machine_values(
    dir: &InstallDir,
    version: &str,
    publisher: &str,
    size_kb: u32,
    start_after_sign_in: bool,
) -> Vec<Write> {
    let at = ease_of_access::at_key();
    let sr = dir.screen_reader();
    let setup = dir.file(SETUP);
    let mut values = vec![
        write(&at, "ApplicationName", text("Okur")),
        write(&at, "Description", text("Okur screen reader")),
        write(
            &at,
            "Profile",
            text(
                r#"<HCIModel><Accommodation type="severe vision"/><Accommodation type="mild vision"/></HCIModel>"#,
            ),
        ),
        write(&at, "SimpleProfile", text("screenreader")),
        write(&at, "ATExe", text(SCREEN_READER)),
        write(&at, "StartExe", text(sr.display().to_string())),
        write(&at, "StartParams", text("--ease-of-access")),
        write(&at, "TerminateOnDesktopSwitch", RegValue::Dword(0)),
        write(OKUR_KEY, "InstallDir", text(dir.0.display().to_string())),
        write(OKUR_KEY, "Version", text(version)),
        // JAWS's default: off (Automatically start JAWS after log on for all users).
        write(OKUR_KEY, "StartAfterSignInAllUsers", RegValue::Dword(0)),
        write(APP_PATH_KEY, "", text(sr.display().to_string())),
        write(UNINSTALL_KEY, "DisplayName", text("Okur")),
        write(UNINSTALL_KEY, "DisplayVersion", text(version)),
        write(UNINSTALL_KEY, "Publisher", text(publisher)),
        write(
            UNINSTALL_KEY,
            "InstallLocation",
            text(dir.0.display().to_string()),
        ),
        write(
            UNINSTALL_KEY,
            "UninstallString",
            text(format!("{} uninstall", quoted(&setup))),
        ),
        write(
            UNINSTALL_KEY,
            "QuietUninstallString",
            text(format!("{} uninstall --silent", quoted(&setup))),
        ),
        write(UNINSTALL_KEY, "DisplayIcon", text(sr.display().to_string())),
        write(UNINSTALL_KEY, "EstimatedSize", RegValue::Dword(size_kb)),
        write(UNINSTALL_KEY, "NoModify", RegValue::Dword(1)),
        write(UNINSTALL_KEY, "NoRepair", RegValue::Dword(1)),
    ];
    if start_after_sign_in {
        values.push(write(
            RUN_KEY,
            RUN_VALUE,
            text(format!("{} --sign-in", quoted(&sr))),
        ));
    }
    values
}

/// The machine keys uninstall deletes, with everything under them.
pub fn machine_keys() -> Vec<(Hive, String)> {
    vec![
        (Hive::LocalMachine, ease_of_access::at_key()),
        (Hive::LocalMachine, OKUR_KEY.to_owned()),
        (Hive::LocalMachine, APP_PATH_KEY.to_owned()),
        (Hive::LocalMachine, UNINSTALL_KEY.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> InstallDir {
        InstallDir(PathBuf::from(r"C:\Program Files\Okur"))
    }

    fn find<'a>(values: &'a [Write], key: &str, name: &str) -> Option<&'a RegValue> {
        values
            .iter()
            .find(|w| w.key == key && w.name == name)
            .map(|w| &w.value)
    }

    /// The assistive-technology key as §5.4 lists it, and nothing that would keep Okur off
    /// the secure desktop or start it there yet.
    #[test]
    fn okur_registers_as_an_assistive_technology() {
        let values = machine_values(&dir(), "0.1.0", "Okur", 20_000, true);
        let at = ease_of_access::at_key();
        assert_eq!(
            at,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Accessibility\ATs\okur_okur_v1"
        );
        let string = |name: &str| find(&values, &at, name).and_then(RegValue::as_str);
        assert_eq!(string("ATExe"), Some("okur-sr.exe"));
        assert_eq!(
            string("StartExe"),
            Some(r"C:\Program Files\Okur\okur-sr.exe")
        );
        assert_eq!(string("StartParams"), Some("--ease-of-access"));
        assert_eq!(string("SimpleProfile"), Some("screenreader"));
        assert!(string("Profile").is_some_and(|p| p.contains(r#"type="severe vision""#)));
        assert_eq!(
            find(&values, &at, "TerminateOnDesktopSwitch"),
            Some(&RegValue::Dword(0))
        );
        for absent in [
            "SecureDesktopAccommodation",
            "CopySettingsToLockedDesktop",
            "PassiveAutoStartBehavior",
        ] {
            assert_eq!(find(&values, &at, absent), None, "{absent}");
        }
        // The sign-in screen's start is M6's: no Configuration value is written.
        assert!(values.iter().all(|w| w.name != "Configuration"));
        assert!(values.iter().all(|w| w.hive == Hive::LocalMachine));
    }

    #[test]
    fn win_r_apps_and_features_and_the_sign_in_start() {
        let values = machine_values(&dir(), "0.1.0", "Okur", 20_000, true);
        assert_eq!(
            find(&values, APP_PATH_KEY, "").and_then(RegValue::as_str),
            Some(r"C:\Program Files\Okur\okur-sr.exe")
        );
        assert_eq!(
            find(&values, RUN_KEY, RUN_VALUE).and_then(RegValue::as_str),
            Some(r#""C:\Program Files\Okur\okur-sr.exe" --sign-in"#)
        );
        assert_eq!(
            find(&values, UNINSTALL_KEY, "UninstallString").and_then(RegValue::as_str),
            Some(r#""C:\Program Files\Okur\okur-setup.exe" uninstall"#)
        );
        assert_eq!(
            find(&values, OKUR_KEY, "StartAfterSignInAllUsers"),
            Some(&RegValue::Dword(0))
        );
        let without = machine_values(&dir(), "0.1.0", "Okur", 20_000, false);
        assert_eq!(find(&without, RUN_KEY, RUN_VALUE), None);
    }

    /// Uninstall's keys hold every key install writes but the Run key, which is shared and
    /// loses only Okur's value.
    #[test]
    fn uninstall_removes_every_key_install_writes() {
        let keys = machine_keys();
        for w in machine_values(&dir(), "0.1.0", "Okur", 1, true) {
            let removed = keys.iter().any(|(_, key)| *key == w.key) || w.key == RUN_KEY;
            assert!(removed, "{} is left behind", w.key);
        }
        assert!(keys.iter().all(|(_, key)| key != RUN_KEY));
    }
}
