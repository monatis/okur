//! Okur's programs, as a library: the live screen reader ([`run`]) and what starts, stops and
//! describes it. These programs call it (`docs/references/startup-and-secure-screens-proposal.md`,
//! §5.2):
//!
//! - `okur.exe`, the screen reader's console program (`src/main.rs`): `okur run`, `okur
//!   doctor`, `okur quit` and `okur voices`;
//! - `okur-sr.exe`, the installed screen reader without a console ([`sr`]), with UI Access in
//!   its manifest, and `okur-sr-no-uiaccess.exe`, the same without it;
//! - `okur-lab.exe`, the verification harness (crates/okur-lab), for its live runs' settings
//!   and pages, never installed for users.

pub mod control;
pub mod doctor;
pub mod run;
pub mod sr;
pub mod startup;
