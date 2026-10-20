//! `okur-setup.exe install | uninstall`: see [`okur_setup::args`] for the command line.
//!
//! Started unelevated, it starts itself elevated (`runas`) for the machine's part and waits;
//! the user's part and Okur's start stay with the unelevated copy, so that Okur runs as the
//! user and not with the administrator's token. Started elevated, it does the machine's part
//! itself and leaves Okur to be started by the user. What it did is said in the console that
//! started it, or in a message box.

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::Path;

use okur_platform::registry::SystemRegistry;
use okur_setup::args::{self, Args, Task};
use okur_setup::plan::InstallDir;
use okur_setup::setup::{self, InstallOptions, Outcome, ThisMachine};

const TITLE: &str = "Okur Setup";

fn main() {
    std::process::exit(run());
}

/// What a finished machine part says.
fn describe(task: &Task, outcome: &Outcome) -> String {
    let mut lines = Vec::new();
    match task {
        Task::Install { .. } if outcome.ui_access => {
            lines.push("Okur is installed.".to_owned());
        }
        Task::Install { .. } => lines.push(
            "Okur is installed without UI Access: its signature does not verify on this \
             computer, so windows of programs running as administrator stay silent."
                .to_owned(),
        ),
        Task::Uninstall => {
            lines.push("Okur is uninstalled. Your settings stay in %APPDATA%\\Okur.".to_owned())
        }
    }
    if !outcome.at_reboot.is_empty() {
        lines.push(format!(
            "{} file(s) in use go when Windows next starts.",
            outcome.at_reboot.len()
        ));
    }
    lines.join("\n")
}

/// The machine's part, in this (elevated) process.
fn machine_part(args: &Args, dir: &InstallDir) -> Result<String, String> {
    let mut registry = SystemRegistry;
    let mut machine = ThisMachine;
    let outcome = match &args.task {
        Task::Install {
            from,
            start_after_sign_in,
            ..
        } => {
            let source = match from {
                Some(from) => from.clone(),
                None => std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent().map(Path::to_path_buf))
                    .ok_or_else(|| "the installer's folder is unknown".to_owned())?,
            };
            setup::install(
                dir,
                &InstallOptions {
                    source,
                    start_after_sign_in: *start_after_sign_in,
                },
                &mut registry,
                &mut machine,
            )?
        }
        Task::Uninstall => setup::uninstall(dir, &mut registry, &mut machine)?,
    };
    Ok(describe(&args.task, &outcome))
}

/// The machine's part in an elevated copy of this program, waited for: its report.
fn elevated_part(args: &Args) -> Result<String, String> {
    let report = std::env::temp_dir().join(format!("okur-setup-{}.txt", std::process::id()));
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let started = okur_platform::process::shell_execute(
        &exe,
        &args::worker_line(args, &report),
        Some("runas"),
        true,
    );
    let said = std::fs::read_to_string(&report).unwrap_or_default();
    let _ = std::fs::remove_file(&report);
    match started {
        Ok(Some(0)) => Ok(said),
        Ok(Some(_)) => Err(said),
        Ok(None) => Err("The elevated installer did not run.".to_owned()),
        Err(e) => Err(format!(
            "The installer was not allowed to run elevated: {e}"
        )),
    }
}

fn run() -> i32 {
    let console = okur_platform::process::attach_parent_console();
    let args = match args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(e) => {
            say(
                console,
                false,
                &format!("{e}\nokur-setup install [--from <folder>] [--silent] [--no-start] [--no-start-after-sign-in]\nokur-setup uninstall [--silent]"),
            );
            return 2;
        }
    };
    let Some(dir) = InstallDir::program_files() else {
        say(console, args.silent, "Program Files was not found.");
        return 1;
    };
    // The elevated worker: the machine's part, its report to the file it was given.
    if let Some(report) = &args.worker {
        let result = machine_part(&args, &dir);
        let (text, code) = match &result {
            Ok(text) => (text.clone(), 0),
            Err(e) => (e.clone(), 1),
        };
        let _ = std::fs::write(report, text);
        return code;
    }
    let elevated = okur_platform::elevation::is_elevated();
    let result = if elevated {
        machine_part(&args, &dir)
    } else {
        elevated_part(&args)
    };
    let mut text = match &result {
        Ok(text) | Err(text) => text.clone(),
    };
    if result.is_ok() {
        match &args.task {
            Task::Install { start: true, .. } if !elevated => {
                // Through the shell, as a UI Access program must be started, as the user.
                if let Err(e) =
                    okur_platform::process::shell_execute(&dir.screen_reader(), "", None, false)
                {
                    text.push_str(&format!("\nOkur did not start: {e}"));
                }
            }
            Task::Install { start: true, .. } => text.push_str(
                "\nStart Okur yourself (Windows+R, okur): started from here, it would run as \
                 administrator.",
            ),
            Task::Install { .. } => {}
            Task::Uninstall => {
                if let Err(e) = setup::uninstall_for_user(&mut SystemRegistry) {
                    text.push_str(&format!("\nYour sign-in start was not removed: {e}"));
                }
            }
        }
    }
    say(console, args.silent, &text);
    i32::from(result.is_err())
}

/// Says what happened: in the console that started the installer, else in a message box,
/// unless silent.
fn say(console: bool, silent: bool, text: &str) {
    if console {
        println!("{text}");
    } else if !silent {
        okur_platform::process::message_box(TITLE, text);
    }
}
