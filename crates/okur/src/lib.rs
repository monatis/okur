//! Okur's programs, as a library: the live screen reader ([`run`]) and what starts, stops and
//! describes it. These programs call it (`docs/references/startup-and-secure-screens-proposal.md`,
//! §5.2):
//!
//! - `okur.exe`, the screen reader's console program ([`cli`]): `okur run`, `okur doctor`,
//!   `okur quit` and `okur voices`;
//! - `okur-sr.exe`, the installed screen reader without a console ([`sr`]), with UI Access in
//!   its manifest, and `okur-sr-no-uiaccess.exe`, the same without it.
//!
//! `okur` runs on Windows and on Linux, each system's start in its own module ([`windows`],
//! [`linux`]) around the neutral run; elsewhere the programs say so (`not_on_this_system`) and
//! end.

#[cfg(any(windows, target_os = "linux"))]
pub mod cli;
#[cfg(windows)]
pub mod control;
#[cfg(windows)]
pub mod doctor;
pub mod folders;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(any(windows, target_os = "linux"))]
pub mod run;
#[cfg(windows)]
pub mod sr;
pub mod startup;
#[cfg(windows)]
pub mod windows;

/// The system this build runs on, as the neutral run and the console program use it.
#[cfg(target_os = "linux")]
use linux as system;
#[cfg(windows)]
use windows as system;

/// What a program says where it does not run (`okur` without a driver for the system, macOS's
/// to be `okur-macos`; `okur-sr`, Windows' installed reader, anywhere but Windows), and the
/// exit code it ends with.
#[cfg(not(windows))]
pub fn not_on_this_system() -> i32 {
    if cfg!(target_os = "linux") {
        eprintln!("This program is Windows' installed reader: run `okur run` here.");
    } else {
        eprintln!("Okur runs on Windows and Linux for now.");
    }
    1
}
