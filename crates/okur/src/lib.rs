//! Okur's programs, as a library: the live screen reader ([`run`]) and what starts, stops and
//! describes it. Two programs call it (`docs/references/startup-and-secure-screens-proposal.md`,
//! §5.2):
//!
//! - `okur.exe`, the developer's console program (`src/main.rs`): `okur run`, the harness
//!   commands, `okur doctor` and `okur quit`;
//! - `okur-sr.exe`, the installed screen reader without a console ([`sr`]), with UI Access in
//!   its manifest, and `okur-sr-no-uiaccess.exe`, the same without it.

pub mod control;
pub mod doctor;
pub mod run;
pub mod sr;
pub mod startup;
