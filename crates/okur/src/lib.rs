//! Okur's programs, as a library: the live screen reader ([`run`]) and what starts, stops and
//! describes it. These programs call it (`docs/references/startup-and-secure-screens-proposal.md`,
//! §5.2):
//!
//! - `okur.exe`, the screen reader's console program ([`cli`]): `okur run`, `okur doctor`,
//!   `okur quit` and `okur voices`;
//! - `okur-sr.exe`, the installed screen reader without a console ([`sr`]), with UI Access in
//!   its manifest, and `okur-sr-no-uiaccess.exe`, the same without it.
//!
//! They run on Windows for now: elsewhere they say so (`not_on_this_system`) and end.

#[cfg(windows)]
pub mod cli;
#[cfg(windows)]
pub mod control;
#[cfg(windows)]
pub mod doctor;
pub mod folders;
#[cfg(windows)]
pub mod run;
#[cfg(windows)]
pub mod sr;
pub mod startup;

/// What the programs say on a system without a driver yet (macOS's will be `okur-macos`), and
/// the exit code they end with.
#[cfg(not(windows))]
pub fn not_on_this_system() -> i32 {
    eprintln!("Okur runs on Windows for now.");
    1
}
