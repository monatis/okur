//! The verification harness's tools, shared by its programs (`okur-lab`, `okur-lab-keys`,
//! `okur-lab-notify-probe`): key chords sent to other programs and JAWS's speech history
//! (`harness`), UI Automation probes (`uia_probe`), the sound card's meter (`audio`), the
//! trace file writer (`trace`) and the reads made right away for research and recording
//! (`probes`). None of it is in the screen reader's programs: what sends keys, reads the
//! clipboard, writes traces and meters the sound card stays out of `okur.exe`, which users run
//! (check-crate-layers.ps1, rule R3).

#[cfg(windows)]
pub mod audio;
#[cfg(windows)]
pub mod harness;
#[cfg(windows)]
pub mod probes;
#[cfg(windows)]
pub mod trace;
#[cfg(windows)]
pub mod uia_probe;

#[cfg(windows)]
pub use audio::{AudioActivityReport, AudioEndpointMeter, SilenceReport};

#[cfg(all(test, windows))]
pub(crate) static TEST_AUDIO_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
