//! `okur-sr.exe`, the installed screen reader: a program without a console, started by
//! Windows' Ease of Access, at sign-in, or by the user
//! (`docs/references/startup-and-secure-screens-proposal.md`, §5.2 and §5.5).
//!
//! ```text
//! okur-sr.exe                       start (a copy running on this desktop is replaced)
//! okur-sr.exe --ease-of-access      started by Windows' Assistive Technology Manager
//! okur-sr.exe --sign-in             started at sign-in (the Run key)
//! okur-sr.exe --quit                ask the copy on this desktop to quit
//! okur-sr.exe --doctor-exit [file]  print `okur doctor`'s report for this very process
//!                                   (to the console that started it, or to the file), exit
//! okur-sr.exe --debug-log <file>    the developer's debug log, as `okur run --debug-log`
//! ```
//!
//! Windows adds `/hardwarebuttonlaunch` when a hardware button started the assistive
//! technology; it is taken as `--ease-of-access`. Until the sign-in start's choices are
//! built (the proposal's M4), `--sign-in` starts Okur as JAWS's default, Always, does.

use crate::run::{RunConfig, RunOptions};
use crate::startup::StartKind;

/// What `okur-sr.exe` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Run {
        start: StartKind,
        debug_log: Option<String>,
    },
    Quit,
    /// The report, to the parent's console (`None`) or to a file.
    Doctor(Option<String>),
}

/// Reads `okur-sr.exe`'s arguments (without the program's name).
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Action, String> {
    let mut start = StartKind::Manual;
    let mut debug_log = None;
    let mut quit = false;
    let mut doctor: Option<Option<String>> = None;
    let mut args = args.into_iter().peekable();
    while let Some(arg) = args.next() {
        match arg.to_ascii_lowercase().as_str() {
            "--ease-of-access" | "/hardwarebuttonlaunch" => start = StartKind::EaseOfAccess,
            "--sign-in" => start = StartKind::SignIn,
            "--quit" => quit = true,
            "--doctor-exit" => {
                let file = args.next_if(|next| !next.starts_with("--"));
                doctor = Some(file);
            }
            "--debug-log" => {
                debug_log = Some(
                    args.next()
                        .ok_or_else(|| "--debug-log needs a file".to_owned())?,
                );
            }
            _ => return Err(format!("unknown argument {arg:?}")),
        }
    }
    Ok(match (quit, doctor) {
        (true, Some(_)) => return Err("--quit and --doctor-exit do not go together".into()),
        (true, None) => Action::Quit,
        (false, Some(file)) => Action::Doctor(file),
        (false, None) => Action::Run { start, debug_log },
    })
}

/// `okur-sr.exe`'s whole program; its exit code.
pub async fn main() -> i32 {
    let action = match parse(std::env::args().skip(1)) {
        Ok(action) => action,
        Err(e) => {
            if okur_platform::process::attach_parent_console() {
                eprintln!("okur-sr: {e}");
            }
            return 2;
        }
    };
    match action {
        Action::Quit => {
            let console = okur_platform::process::attach_parent_console();
            match crate::control::quit_this_desktop() {
                Ok(said) => {
                    if console {
                        println!("{said}");
                    }
                    0
                }
                Err(e) => {
                    if console {
                        eprintln!("{e}");
                    }
                    1
                }
            }
        }
        Action::Doctor(file) => {
            // Never on the secure desktop: a file written there could hold what was typed.
            let desktop = okur_platform::desktop::thread_desktop().unwrap_or_default();
            if okur_platform::desktop::is_secure_desktop(&desktop) {
                return 1;
            }
            let report = crate::doctor::render(&crate::doctor::gather(None));
            match file {
                Some(path) => match std::fs::write(&path, report) {
                    Ok(()) => 0,
                    Err(_) => 1,
                },
                None => {
                    if okur_platform::process::attach_parent_console() {
                        print!("{report}");
                    }
                    0
                }
            }
        }
        Action::Run { start, debug_log } => {
            crate::run::init_logging(debug_log.as_deref());
            okur_platform::init();
            crate::run::run(RunConfig {
                options: RunOptions::default(),
                fixture: None,
                speech_log: None,
                settings: None,
                start,
                console: false,
            })
            .await;
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Action, String> {
        parse(list.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn no_argument_is_a_manual_start() {
        assert_eq!(
            args(&[]),
            Ok(Action::Run {
                start: StartKind::Manual,
                debug_log: None
            })
        );
    }

    #[test]
    fn windows_and_the_sign_in_start_say_so() {
        assert_eq!(
            args(&["--ease-of-access"]),
            Ok(Action::Run {
                start: StartKind::EaseOfAccess,
                debug_log: None
            })
        );
        // Windows' own argument for a hardware button's start.
        assert_eq!(
            args(&["/hardwarebuttonlaunch"]),
            Ok(Action::Run {
                start: StartKind::EaseOfAccess,
                debug_log: None
            })
        );
        assert_eq!(
            args(&["--sign-in", "--debug-log", "C:\\okur.log"]),
            Ok(Action::Run {
                start: StartKind::SignIn,
                debug_log: Some("C:\\okur.log".into())
            })
        );
    }

    #[test]
    fn quit_and_doctor_are_actions_of_their_own() {
        assert_eq!(args(&["--quit"]), Ok(Action::Quit));
        assert_eq!(args(&["--doctor-exit"]), Ok(Action::Doctor(None)));
        assert_eq!(
            args(&["--doctor-exit", "C:\\report.txt"]),
            Ok(Action::Doctor(Some("C:\\report.txt".into())))
        );
        assert!(args(&["--quit", "--doctor-exit"]).is_err());
    }

    #[test]
    fn unknown_or_incomplete_arguments_are_errors() {
        assert!(args(&["--secure"]).is_err());
        assert!(args(&["--debug-log"]).is_err());
    }
}
