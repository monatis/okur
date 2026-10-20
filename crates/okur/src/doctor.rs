//! `okur doctor`: what Windows grants a copy of Okur, read only
//! (`docs/references/startup-and-secure-screens-proposal.md`, M0 and experiment E1). For this
//! process, or another by process id (`okur doctor --pid`, to see `okur-ui`'s lowered token
//! or the installed screen reader's):
//!
//! - the session, the desktop it runs on and the desktop that has the input;
//! - the account, the integrity level, `TokenUIAccess` and elevation;
//! - the executable: whether it is under Program Files, its signature, its manifest's
//!   execution level and `uiAccess`;
//! - the system's screen-reader flag, and the copy of Okur running on this desktop;
//! - Okur's Ease of Access registration: its key, and the two `Configuration` lists.
//!
//! Nothing is changed: every query reads.

use std::path::{Path, PathBuf};

use okur_platform::ease_of_access::{self, AtRegistration};
use okur_platform::elevation::{integrity, ElevationType, TokenFacts};
use okur_platform::registry::SystemRegistry;
use okur_platform::signature::{ManifestFacts, SignatureStatus};

/// Everything `okur doctor` reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub pid: u32,
    pub executable: Option<PathBuf>,
    /// The process's token, or why it could not be read.
    pub token: Result<TokenFacts, String>,
    /// `WinSta0\Default`: this process's only.
    pub desktop: Option<String>,
    pub input_desktop: Option<String>,
    pub in_program_files: Option<bool>,
    pub signature: SignatureStatus,
    pub manifest: Option<ManifestFacts>,
    pub screen_reader_flag: Option<bool>,
    /// The `status` reply of the copy running on this desktop, or why there is none.
    pub running: Result<String, String>,
    pub registration: AtRegistration,
}

/// Whether `path` is under one of `roots` (Program Files' folders), without regard to case.
pub fn under_any(path: &Path, roots: &[PathBuf]) -> bool {
    let path = path.to_string_lossy().to_lowercase();
    roots.iter().any(|root| {
        let root = root.to_string_lossy().to_lowercase();
        let root = root.trim_end_matches('\\');
        !root.is_empty()
            && path
                .strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('\\'))
    })
}

/// Program Files' folders, as this machine names them.
fn program_files() -> Vec<PathBuf> {
    ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"]
        .iter()
        .filter_map(|name| std::env::var_os(name).map(PathBuf::from))
        .collect()
}

/// Reads the report for this process, or for the process `pid`.
pub fn gather(pid: Option<u32>) -> Report {
    let own = pid.is_none_or(|p| p == std::process::id());
    let pid = pid.unwrap_or_else(std::process::id);
    let executable = if own {
        std::env::current_exe().ok()
    } else {
        okur_platform::process::image_path(pid)
    };
    let token = if own {
        Ok(okur_platform::elevation::current())
    } else {
        okur_platform::elevation::of_process(pid)
    };
    let desktop = own
        .then(|| {
            let desktop = okur_platform::desktop::thread_desktop()?;
            Some(match okur_platform::desktop::window_station() {
                Some(station) => format!("{station}\\{desktop}"),
                None => desktop,
            })
        })
        .flatten();
    let signature = executable.as_deref().map_or(
        SignatureStatus::Unavailable,
        okur_platform::signature::verify_file,
    );
    let manifest = executable
        .as_deref()
        .and_then(okur_platform::signature::embedded_manifest)
        .map(|text| ManifestFacts::parse(&text));
    Report {
        pid,
        in_program_files: executable
            .as_deref()
            .map(|path| under_any(path, &program_files())),
        executable,
        token,
        desktop,
        input_desktop: okur_platform::desktop::input_desktop(),
        signature,
        manifest,
        screen_reader_flag: okur_platform::process::screen_reader_flag(),
        running: crate::control::send(crate::control::Request::Status),
        registration: AtRegistration::read(&SystemRegistry),
    }
}

fn yes_no(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    }
}

fn or_unknown(value: Option<String>) -> String {
    value.unwrap_or_else(|| "unknown".to_owned())
}

/// The report as `okur doctor` prints it, one fact per line.
pub fn render(report: &Report) -> String {
    let mut lines: Vec<(String, String)> = Vec::new();
    let mut add = |name: &str, value: String| lines.push((name.to_owned(), value));
    add("process", report.pid.to_string());
    add(
        "executable",
        or_unknown(report.executable.as_ref().map(|p| p.display().to_string())),
    );
    match &report.token {
        Ok(token) => {
            add(
                "session",
                or_unknown(token.session_id.map(|s| s.to_string())),
            );
            let account = match (&token.account, &token.sid) {
                (Some(account), Some(sid)) if account != sid => format!("{account} ({sid})"),
                (Some(account), _) => account.clone(),
                (None, sid) => or_unknown(sid.clone()),
            };
            add("account", account);
            add(
                "integrity",
                or_unknown(
                    token
                        .integrity
                        .map(|rid| format!("{} (0x{rid:04X})", integrity::name(rid))),
                ),
            );
            add(
                "TokenUIAccess",
                or_unknown(token.ui_access.map(|ui| u8::from(ui).to_string())),
            );
            let kind = match token.elevation_type {
                Some(ElevationType::Full) => " (full token)",
                Some(ElevationType::Limited) => " (limited token)",
                Some(ElevationType::Default) => " (no split token)",
                None => "",
            };
            add("elevated", format!("{}{kind}", yes_no(token.elevated)));
            add(
                "reads elevated windows",
                yes_no(Some(token.reads_elevated_windows())).to_owned(),
            );
        }
        Err(e) => add("token", format!("not read: {e}")),
    }
    add(
        "desktop",
        report
            .desktop
            .clone()
            .unwrap_or_else(|| "(this process's only)".to_owned()),
    );
    add(
        "input desktop",
        report
            .input_desktop
            .clone()
            .unwrap_or_else(|| "not open to this process (a secure desktop?)".to_owned()),
    );
    add(
        "in Program Files",
        yes_no(report.in_program_files).to_owned(),
    );
    add("signature", report.signature.describe());
    add(
        "manifest",
        report
            .manifest
            .as_ref()
            .map_or("none embedded".to_owned(), ManifestFacts::describe),
    );
    add(
        "screen reader flag",
        match report.screen_reader_flag {
            Some(true) => "on".to_owned(),
            Some(false) => "off".to_owned(),
            None => "unknown".to_owned(),
        },
    );
    add(
        "Okur on this desktop",
        match &report.running {
            Ok(status) => status.clone(),
            Err(_) => "none answers".to_owned(),
        },
    );
    let registration = &report.registration;
    add(
        "AT key",
        match &registration.at_values {
            Some(values) if values.is_empty() => {
                format!("{} (no values)", ease_of_access::at_key())
            }
            Some(_) => ease_of_access::at_key(),
            None => format!("{} not present", ease_of_access::AT_NAME),
        },
    );
    for (name, value) in registration.at_values.iter().flatten() {
        add(&format!("  {name}"), value.display());
    }
    let list = |value: &Option<String>, starts: bool| match value {
        Some(list) => format!(
            "{list:?} (Okur {})",
            if starts { "listed" } else { "not listed" }
        ),
        None => "not set".to_owned(),
    };
    add(
        "sign-in screen (HKLM Configuration)",
        list(
            &registration.machine_configuration,
            registration.on_sign_in_screen(),
        ),
    );
    add(
        "after sign-in (HKCU Configuration)",
        list(
            &registration.user_configuration,
            registration.after_sign_in(),
        ),
    );
    let width = lines.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    lines
        .iter()
        .map(|(name, value)| format!("{name:<width$}  {value}\n"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use okur_platform::registry::RegValue;

    #[test]
    fn program_files_is_a_folder_prefix_in_any_case() {
        let roots = [
            PathBuf::from(r"C:\Program Files"),
            PathBuf::from(r"C:\Program Files (x86)\"),
        ];
        assert!(under_any(
            Path::new(r"C:\Program Files\Okur\okur-sr.exe"),
            &roots
        ));
        assert!(under_any(
            Path::new(r"c:\program files (x86)\Okur\x.exe"),
            &roots
        ));
        assert!(!under_any(
            Path::new(r"C:\Program FilesX\okur-sr.exe"),
            &roots
        ));
        assert!(!under_any(
            Path::new(r"C:\Users\ailabs\sr-harness\bin\okur.exe"),
            &roots
        ));
        assert!(!under_any(
            Path::new(r"C:\Program Files\x"),
            &[PathBuf::new()]
        ));
    }

    /// An administrator's UI Access copy as E1's case (c) expects it, printed line by line.
    #[test]
    fn the_report_names_every_fact() {
        let report = Report {
            pid: 4321,
            executable: Some(PathBuf::from(r"C:\Program Files\Okur\okur-sr.exe")),
            token: Ok(TokenFacts {
                session_id: Some(1),
                account: Some(r"OLD-PC\ailabs".into()),
                sid: Some("S-1-5-21-1-2-3-1001".into()),
                integrity: Some(0x3000),
                ui_access: Some(true),
                elevated: Some(false),
                elevation_type: Some(ElevationType::Limited),
            }),
            desktop: Some(r"WinSta0\Default".into()),
            input_desktop: Some("Default".into()),
            in_program_files: Some(true),
            signature: SignatureStatus::Valid,
            manifest: Some(ManifestFacts {
                level: Some("asInvoker".into()),
                ui_access: Some(true),
            }),
            screen_reader_flag: Some(true),
            running: Err("no pipe".into()),
            registration: AtRegistration {
                at_values: Some(vec![(
                    "TerminateOnDesktopSwitch".into(),
                    RegValue::Dword(0),
                )]),
                machine_configuration: Some("FreedomScientific_JAWS_v2025".into()),
                user_configuration: None,
            },
        };
        let text = render(&report);
        for expected in [
            "process ",
            r"OLD-PC\ailabs (S-1-5-21-1-2-3-1001)",
            "High (0x3000)",
            "TokenUIAccess ",
            "no (limited token)",
            r"WinSta0\Default",
            "signature ",
            "  valid",
            "level=asInvoker uiAccess=true",
            "none answers",
            r"\ATs\okur_okur_v1",
            "  TerminateOnDesktopSwitch",
            "\"FreedomScientific_JAWS_v2025\" (Okur not listed)",
            "not set",
        ] {
            assert!(text.contains(expected), "{expected:?} in:\n{text}");
        }
        assert!(text
            .lines()
            .any(|line| line.starts_with("TokenUIAccess") && line.ends_with(" 1")));
    }
}
