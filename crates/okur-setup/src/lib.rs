//! `okur-setup.exe`: installs Okur under Program Files, registers it with Windows as an
//! assistive technology, and uninstalls it
//! (`docs/references/startup-and-secure-screens-proposal.md`, §5.4 and §5.10; milestone M3,
//! without the sign-in screen's start). The development signing of §5.11 is
//! `signing/dev-signing.ps1`.

pub mod args;
pub mod plan;
pub mod setup;

#[cfg(test)]
mod signing_tests;
