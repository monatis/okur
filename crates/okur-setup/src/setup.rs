//! Install and uninstall (`docs/references/startup-and-secure-screens-proposal.md`, §5.10),
//! the machine's part: what the elevated worker does. The registry goes through
//! [`Registry`] and everything else that touches the machine beyond the install folder
//! through [`Machine`], so that tests run both against stand-ins.

use std::path::{Path, PathBuf};

use okur_platform::ease_of_access::{self, ACCESSIBILITY_KEY, AT_NAME, CONFIGURATION};
use okur_platform::registry::{Hive, RegValue, Registry};

use crate::plan::{self, InstallDir};

/// What install and uninstall do to the machine besides its registry and the install folder.
pub trait Machine {
    /// Whether a program's signature verifies on this machine, as UI Access requires.
    fn signature_valid(&self, program: &Path) -> bool;
    /// Asks every running Okur to quit, and ends those that stay and Okur's programs still
    /// running from `dir`.
    fn stop_okur(&mut self, dir: &InstallDir);
    /// Deletes a file, or an empty folder, when Windows next starts.
    fn delete_at_reboot(&mut self, path: &Path) -> Result<(), String>;
}

/// Install's choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallOptions {
    /// The folder holding the programs to install.
    pub source: PathBuf,
    /// The Run key's `--sign-in` start for every user (JAWS's default, "Always", until M4's
    /// choices).
    pub start_after_sign_in: bool,
}

/// What install did, for the user.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The screen reader installed is the UI Access build.
    pub ui_access: bool,
    /// Files in use that Windows deletes at its next start.
    pub at_reboot: Vec<PathBuf>,
}

/// A free name beside `path` for a file put aside: `okur-sr.exe.old`, `okur-sr.exe.old2`, ...
fn aside(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    (1..)
        .map(|n| {
            let suffix = if n == 1 {
                ".old".to_owned()
            } else {
                format!(".old{n}")
            };
            path.with_file_name(format!("{name}{suffix}"))
        })
        .find(|candidate| !candidate.exists())
        .unwrap_or_else(|| path.with_extension("old"))
}

/// Copies `from` to `to`. A file in use there (a running program) is renamed aside first, and
/// deleted when Windows next starts, as NVDA's installer does.
pub fn place(
    from: &Path,
    to: &Path,
    machine: &mut dyn Machine,
    outcome: &mut Outcome,
) -> Result<(), String> {
    if to.exists() && std::fs::remove_file(to).is_err() {
        let aside = aside(to);
        std::fs::rename(to, &aside).map_err(|e| format!("{} is in use: {e}", to.display()))?;
        machine.delete_at_reboot(&aside)?;
        outcome.at_reboot.push(aside);
    }
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| format!("{} to {}: {e}", from.display(), to.display()))
}

/// Removes a file; one in use is deleted when Windows next starts.
fn remove(path: &Path, machine: &mut dyn Machine, outcome: &mut Outcome) -> Result<(), String> {
    if !path.exists() || std::fs::remove_file(path).is_ok() {
        return Ok(());
    }
    machine.delete_at_reboot(path)?;
    outcome.at_reboot.push(path.to_path_buf());
    Ok(())
}

fn same_folder(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Installs Okur from `options.source` into `dir`: every running copy stopped, the programs
/// copied (the UI Access build only if its signature verifies here, else the build without
/// it under the same name), the registry of §5.4 written.
pub fn install(
    dir: &InstallDir,
    options: &InstallOptions,
    registry: &mut dyn Registry,
    machine: &mut dyn Machine,
) -> Result<Outcome, String> {
    let source = &options.source;
    let ui_access_build = source.join(plan::SCREEN_READER);
    let plain_build = source.join(plan::SCREEN_READER_NO_UI_ACCESS);
    let ui_access = ui_access_build.exists() && machine.signature_valid(&ui_access_build);
    let screen_reader = if ui_access {
        ui_access_build
    } else {
        plain_build
    };
    for needed in [&screen_reader, &source.join(plan::UI)] {
        if !needed.exists() {
            return Err(format!("{} is missing", needed.display()));
        }
    }
    machine.stop_okur(dir);
    std::fs::create_dir_all(&dir.0).map_err(|e| format!("{}: {e}", dir.0.display()))?;
    let mut outcome = Outcome {
        ui_access,
        at_reboot: Vec::new(),
    };
    place(&screen_reader, &dir.screen_reader(), machine, &mut outcome)?;
    place(
        &source.join(plan::UI),
        &dir.file(plan::UI),
        machine,
        &mut outcome,
    )?;
    // The installer stays as the uninstaller, unless it already runs from there.
    if !same_folder(source, &dir.0) {
        let setup = source.join(plan::SETUP);
        if setup.exists() {
            place(&setup, &dir.file(plan::SETUP), machine, &mut outcome)?;
        }
        for document in plan::DOCUMENTS {
            let from = source.join(document);
            if from.exists() {
                place(&from, &dir.file(document), machine, &mut outcome)?;
            }
        }
    }
    let size: u64 = std::fs::read_dir(&dir.0)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|e| e.metadata().ok())
                .filter(std::fs::Metadata::is_file)
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0);
    let size_kb = u32::try_from(size / 1024).unwrap_or(u32::MAX);
    for value in plan::machine_values(
        dir,
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS"),
        size_kb,
        options.start_after_sign_in,
    ) {
        registry.set(value.hive, &value.key, &value.name, &value.value)?;
    }
    Ok(outcome)
}

/// Takes Okur's name out of a `Configuration` list, leaving every other name.
fn unlist(registry: &mut dyn Registry, hive: Hive) -> Result<(), String> {
    if let Some(list) = registry.string(hive, ACCESSIBILITY_KEY, CONFIGURATION) {
        let kept = ease_of_access::without_name(&list, AT_NAME);
        if kept != list {
            registry.set(
                hive,
                ACCESSIBILITY_KEY,
                CONFIGURATION,
                &RegValue::String(kept),
            )?;
        }
    }
    Ok(())
}

/// Removes Okur from the machine: every copy stopped; its name out of the sign-in screen's
/// list; its keys, its Run value and its files gone (files in use at the next start). The
/// users' settings in `%APPDATA%\Okur` stay, as NVDA's do.
pub fn uninstall(
    dir: &InstallDir,
    registry: &mut dyn Registry,
    machine: &mut dyn Machine,
) -> Result<Outcome, String> {
    machine.stop_okur(dir);
    unlist(registry, Hive::LocalMachine)?;
    for (hive, key) in plan::machine_keys() {
        registry.delete_tree(hive, &key)?;
    }
    registry.delete_value(Hive::LocalMachine, plan::RUN_KEY, plan::RUN_VALUE)?;
    let mut outcome = Outcome::default();
    if dir.0.exists() {
        let secure = dir.file(plan::SECURE_SETTINGS);
        if secure.exists() {
            std::fs::remove_dir_all(&secure).map_err(|e| format!("{}: {e}", secure.display()))?;
        }
        let ours = [plan::SCREEN_READER, plan::UI, plan::SETUP]
            .into_iter()
            .chain(plan::DOCUMENTS)
            .map(|name| dir.file(name));
        // Files put aside by an install over a running copy go too.
        let aside: Vec<PathBuf> = std::fs::read_dir(&dir.0)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.to_string_lossy().contains(".old"))
                    .collect()
            })
            .unwrap_or_default();
        for path in ours.chain(aside) {
            remove(&path, machine, &mut outcome)?;
        }
        if std::fs::remove_dir(&dir.0).is_err() {
            machine.delete_at_reboot(&dir.0)?;
            outcome.at_reboot.push(dir.0.clone());
        }
    }
    Ok(outcome)
}

/// The signed-in user's part of uninstall, done unelevated so that it is that user's: Okur's
/// name out of the after-sign-in list.
pub fn uninstall_for_user(registry: &mut dyn Registry) -> Result<(), String> {
    unlist(registry, Hive::CurrentUser)
}

/// The machine itself.
#[derive(Debug, Default)]
pub struct ThisMachine;

impl Machine for ThisMachine {
    fn signature_valid(&self, program: &Path) -> bool {
        okur_platform::signature::verify_file(program).is_valid()
    }

    fn stop_okur(&mut self, dir: &InstallDir) {
        for pipe in okur_platform::instance::all_pipes() {
            let _ =
                okur_platform::instance::request(&pipe, "quit", std::time::Duration::from_secs(2));
        }
        let names = [plan::SCREEN_READER, plan::UI, "okur.exe"];
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
        loop {
            let ours: Vec<u32> = okur_platform::process::processes_named(&names)
                .into_iter()
                .filter(|(_, path)| {
                    path.as_deref()
                        .is_some_and(|p| p.parent().is_some_and(|d| same_folder(d, &dir.0)))
                })
                .map(|(pid, _)| pid)
                .collect();
            if ours.is_empty() {
                return;
            }
            if std::time::Instant::now() >= deadline {
                for pid in ours {
                    let _ = okur_platform::process::terminate(pid);
                }
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }

    fn delete_at_reboot(&mut self, path: &Path) -> Result<(), String> {
        okur_platform::process::delete_at_reboot(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use okur_platform::registry::MemoryRegistry;

    /// A machine that records what it was asked, with a signature verdict and files that
    /// cannot be deleted.
    #[derive(Default)]
    struct FakeMachine {
        signed: bool,
        stopped: u32,
        at_reboot: Vec<PathBuf>,
    }

    impl Machine for FakeMachine {
        fn signature_valid(&self, _: &Path) -> bool {
            self.signed
        }

        fn stop_okur(&mut self, _: &InstallDir) {
            self.stopped += 1;
        }

        fn delete_at_reboot(&mut self, path: &Path) -> Result<(), String> {
            self.at_reboot.push(path.to_path_buf());
            Ok(())
        }
    }

    /// A fresh folder under the system's temporary folder, removed when dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("okur-setup-test-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            let _ = std::fs::create_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A source folder as a release build leaves it.
    fn source(scratch: &Scratch) -> PathBuf {
        let source = scratch.0.join("source");
        let _ = std::fs::create_dir_all(&source);
        for (name, body) in [
            (plan::SCREEN_READER, "ui access"),
            (plan::SCREEN_READER_NO_UI_ACCESS, "no ui access"),
            (plan::UI, "ui"),
            (plan::SETUP, "setup"),
            ("LICENSE", "license"),
        ] {
            let _ = std::fs::write(source.join(name), body);
        }
        source
    }

    fn options(source: PathBuf) -> InstallOptions {
        InstallOptions {
            source,
            start_after_sign_in: true,
        }
    }

    #[test]
    fn a_signed_build_is_installed_with_ui_access() {
        let scratch = Scratch::new("signed");
        let dir = InstallDir(scratch.0.join("Okur"));
        let mut registry = MemoryRegistry::new();
        let mut machine = FakeMachine {
            signed: true,
            ..FakeMachine::default()
        };
        let outcome = install(
            &dir,
            &options(source(&scratch)),
            &mut registry,
            &mut machine,
        );
        assert_eq!(
            outcome,
            Ok(Outcome {
                ui_access: true,
                at_reboot: Vec::new()
            })
        );
        assert_eq!(machine.stopped, 1, "running copies are stopped first");
        assert_eq!(
            std::fs::read_to_string(dir.screen_reader()).ok().as_deref(),
            Some("ui access")
        );
        for name in [plan::UI, plan::SETUP, "LICENSE"] {
            assert!(dir.file(name).exists(), "{name}");
        }
        assert!(!dir.file(plan::SCREEN_READER_NO_UI_ACCESS).exists());
        assert_eq!(
            registry.string(Hive::LocalMachine, &ease_of_access::at_key(), "StartExe"),
            Some(dir.screen_reader().display().to_string())
        );
        assert!(registry
            .value(Hive::LocalMachine, plan::UNINSTALL_KEY, "EstimatedSize")
            .is_some());
    }

    /// Where the signature does not verify, Windows would refuse UI Access: the build without
    /// it goes in under the same name (NVDA's rule).
    #[test]
    fn an_unverified_build_is_installed_without_ui_access() {
        let scratch = Scratch::new("unsigned");
        let dir = InstallDir(scratch.0.join("Okur"));
        let mut registry = MemoryRegistry::new();
        let mut machine = FakeMachine::default();
        let outcome = install(
            &dir,
            &options(source(&scratch)),
            &mut registry,
            &mut machine,
        );
        assert_eq!(outcome.map(|o| o.ui_access), Ok(false));
        assert_eq!(
            std::fs::read_to_string(dir.screen_reader()).ok().as_deref(),
            Some("no ui access")
        );
    }

    #[test]
    fn a_source_without_the_programs_is_refused_before_anything_changes() {
        let scratch = Scratch::new("empty");
        let dir = InstallDir(scratch.0.join("Okur"));
        let mut registry = MemoryRegistry::new();
        let mut machine = FakeMachine::default();
        let empty = scratch.0.join("empty");
        let _ = std::fs::create_dir_all(&empty);
        assert!(install(&dir, &options(empty), &mut registry, &mut machine).is_err());
        assert_eq!(machine.stopped, 0);
        assert!(registry.key_paths().is_empty());
        assert!(!dir.0.exists());
    }

    /// Uninstall leaves the machine as before install, but for the other assistive
    /// technologies' names and the user's settings; the Run key keeps its other values.
    #[test]
    fn uninstall_removes_what_install_wrote_and_only_that() {
        let scratch = Scratch::new("uninstall");
        let dir = InstallDir(scratch.0.join("Okur"));
        let mut registry = MemoryRegistry::new();
        let jaws = "FreedomScientific_JAWS_v2025";
        let _ = registry.set(
            Hive::LocalMachine,
            ACCESSIBILITY_KEY,
            CONFIGURATION,
            &RegValue::String(format!("{jaws},okur_okur_v1")),
        );
        let _ = registry.set(
            Hive::LocalMachine,
            plan::RUN_KEY,
            "JAWS",
            &RegValue::String("jfw.exe /run".into()),
        );
        let _ = registry.set(
            Hive::CurrentUser,
            ACCESSIBILITY_KEY,
            CONFIGURATION,
            &RegValue::String("freedomscientific_jaws_v2025,okur_okur_v1".into()),
        );
        let mut machine = FakeMachine::default();
        let installed = install(
            &dir,
            &options(source(&scratch)),
            &mut registry,
            &mut machine,
        );
        assert!(installed.is_ok(), "{installed:?}");
        let _ = std::fs::create_dir_all(dir.file(plan::SECURE_SETTINGS));
        let removed = uninstall(&dir, &mut registry, &mut machine);
        assert!(removed.is_ok(), "{removed:?}");
        assert_eq!(uninstall_for_user(&mut registry), Ok(()), "the user's part");
        assert!(!dir.0.exists());
        assert_eq!(
            registry.string(Hive::LocalMachine, ACCESSIBILITY_KEY, CONFIGURATION),
            Some(jaws.to_owned())
        );
        assert_eq!(
            registry.string(Hive::CurrentUser, ACCESSIBILITY_KEY, CONFIGURATION),
            Some("freedomscientific_jaws_v2025".to_owned())
        );
        assert!(registry
            .string(Hive::LocalMachine, plan::RUN_KEY, "JAWS")
            .is_some());
        assert_eq!(
            registry.value(Hive::LocalMachine, plan::RUN_KEY, plan::RUN_VALUE),
            None
        );
        let left: Vec<String> = registry
            .key_paths()
            .into_iter()
            .filter(|key| key.to_lowercase().contains("okur"))
            .collect();
        assert!(left.is_empty(), "{left:?}");
    }

    /// A file that can be neither removed nor renamed (held open without delete sharing) is
    /// an error that names it, with nothing scheduled; a file put aside gets a free name.
    #[cfg(windows)]
    #[test]
    fn a_file_held_open_is_an_error_and_aside_names_are_free() {
        use std::os::windows::fs::OpenOptionsExt;
        let scratch = Scratch::new("in-use");
        let from = scratch.0.join("new.exe");
        let to = scratch.0.join("okur-sr.exe");
        let _ = std::fs::write(&from, "new");
        let _ = std::fs::write(&to, "old");
        // Open without sharing deletion, as a running program's image holds it.
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0x1 | 0x2) // FILE_SHARE_READ | FILE_SHARE_WRITE
            .open(&to);
        assert!(held.is_ok());
        let mut machine = FakeMachine::default();
        let mut outcome = Outcome::default();
        let placed = place(&from, &to, &mut machine, &mut outcome);
        // Without FILE_SHARE_DELETE the file can be neither removed nor renamed: the error
        // names it, and nothing was scheduled.
        assert!(placed.is_err());
        assert!(machine.at_reboot.is_empty());
        drop(held);
        assert_eq!(aside(&to), scratch.0.join("okur-sr.exe.old"));
        let _ = std::fs::write(scratch.0.join("okur-sr.exe.old"), "older");
        assert_eq!(aside(&to), scratch.0.join("okur-sr.exe.old2"));
    }
}
