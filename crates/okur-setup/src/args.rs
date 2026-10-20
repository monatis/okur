//! `okur-setup`'s command line.
//!
//! ```text
//! okur-setup install [--from <folder>] [--silent] [--no-start] [--no-start-after-sign-in]
//! okur-setup uninstall [--silent]
//! ```
//!
//! - `--from`: the folder holding the programs (default: the installer's own).
//! - `--silent`: no message at the end (the exit code says how it went).
//! - `--no-start`: Okur is not started once installed.
//! - `--no-start-after-sign-in`: no Run-key start at sign-in. Until M4 brings JAWS's three
//!   start choices, the Run key starts Okur at every user's sign-in, JAWS's default, beside
//!   any other screen reader that starts then (on old-pc, JAWS): leave it out there.
//!
//! The installer runs unelevated and starts itself elevated for the machine's part, with
//! `--worker <report file>`; the user's own part (HKCU) and Okur's start stay unelevated, so
//! that they are the signed-in user's even when an administrator elevated over the shoulder.

use std::path::PathBuf;

/// What `okur-setup` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Task {
    Install {
        from: Option<PathBuf>,
        start: bool,
        start_after_sign_in: bool,
    },
    Uninstall,
}

/// The whole command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub task: Task,
    pub silent: bool,
    /// Set by the unelevated installer for its elevated worker: where to write its report.
    pub worker: Option<PathBuf>,
}

/// Reads the arguments (without the program's name).
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Args, String> {
    let mut args = args.into_iter();
    let task = args
        .next()
        .ok_or_else(|| "say install or uninstall".to_owned())?;
    let mut from = None;
    let mut silent = false;
    let mut start = true;
    let mut start_after_sign_in = true;
    let mut worker = None;
    let install = match task.to_ascii_lowercase().as_str() {
        "install" => true,
        "uninstall" => false,
        other => return Err(format!("unknown task {other:?}: install or uninstall")),
    };
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .map(PathBuf::from)
                .ok_or_else(|| format!("{name} needs a folder or file"))
        };
        match arg.as_str() {
            "--silent" => silent = true,
            "--worker" => worker = Some(value("--worker")?),
            "--from" if install => from = Some(value("--from")?),
            "--no-start" if install => start = false,
            "--no-start-after-sign-in" if install => start_after_sign_in = false,
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let task = if install {
        Task::Install {
            from,
            start,
            start_after_sign_in,
        }
    } else {
        Task::Uninstall
    };
    Ok(Args {
        task,
        silent,
        worker,
    })
}

/// The worker's command line: the same task and choices, and where to write its report.
pub fn worker_line(args: &Args, report: &std::path::Path) -> String {
    let quote = |path: &std::path::Path| format!("\"{}\"", path.display());
    let mut line = match &args.task {
        Task::Install {
            from,
            start: _,
            start_after_sign_in,
        } => {
            let mut line = "install".to_owned();
            if let Some(from) = from {
                line.push_str(&format!(" --from {}", quote(from)));
            }
            if !start_after_sign_in {
                line.push_str(" --no-start-after-sign-in");
            }
            line
        }
        Task::Uninstall => "uninstall".to_owned(),
    };
    line.push_str(&format!(" --worker {}", quote(report)));
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Args, String> {
        parse(list.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn install_takes_its_choices_with_jaws_defaults() {
        assert_eq!(
            args(&["install"]),
            Ok(Args {
                task: Task::Install {
                    from: None,
                    start: true,
                    start_after_sign_in: true
                },
                silent: false,
                worker: None
            })
        );
        assert_eq!(
            args(&[
                "install",
                "--from",
                r"C:\build",
                "--silent",
                "--no-start",
                "--no-start-after-sign-in"
            ]),
            Ok(Args {
                task: Task::Install {
                    from: Some(PathBuf::from(r"C:\build")),
                    start: false,
                    start_after_sign_in: false
                },
                silent: true,
                worker: None
            })
        );
    }

    #[test]
    fn uninstall_takes_only_silent() {
        assert_eq!(
            args(&["uninstall", "--silent"]).map(|a| (a.task, a.silent)),
            Ok((Task::Uninstall, true))
        );
        assert!(args(&["uninstall", "--no-start"]).is_err());
        assert!(args(&[]).is_err());
        assert!(args(&["repair"]).is_err());
        assert!(args(&["install", "--from"]).is_err());
    }

    /// The elevated worker gets the same machine choices back, with spaces kept in paths.
    #[test]
    fn the_worker_line_reads_back_as_the_same_task() {
        let asked = Args {
            task: Task::Install {
                from: Some(PathBuf::from(r"C:\Users\ailabs\sr harness\bin")),
                start: false,
                start_after_sign_in: false,
            },
            silent: true,
            worker: None,
        };
        let report = PathBuf::from(r"C:\Temp\okur setup.txt");
        let line = worker_line(&asked, &report);
        assert_eq!(
            line,
            r#"install --from "C:\Users\ailabs\sr harness\bin" --no-start-after-sign-in --worker "C:\Temp\okur setup.txt""#
        );
        // As Windows would split it.
        let split = vec![
            "install".to_owned(),
            "--from".to_owned(),
            r"C:\Users\ailabs\sr harness\bin".to_owned(),
            "--no-start-after-sign-in".to_owned(),
            "--worker".to_owned(),
            r"C:\Temp\okur setup.txt".to_owned(),
        ];
        assert_eq!(
            parse(split).map(|a| (a.task, a.worker)),
            Ok((
                Task::Install {
                    from: Some(PathBuf::from(r"C:\Users\ailabs\sr harness\bin")),
                    start: true,
                    start_after_sign_in: false
                },
                Some(report)
            ))
        );
    }
}
