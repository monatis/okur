//! The verification harness's tools, shared by its programs (`okur-lab`, `okur-lab-keys`,
//! `okur-lab-notify-probe`): key chords sent to other programs and JAWS's speech history
//! (`harness`), UI Automation probes (`uia_probe`), the sound card's meter (`audio`), the
//! trace file writer (`trace`) and the reads made right away for research and recording
//! (`probes`). None of it is in the screen reader's programs: what sends keys, reads the
//! clipboard, writes traces and meters the sound card stays out of `okur.exe`, which users run
//! (check-crate-layers.ps1, rule R3).

pub mod audio;
pub mod harness;
pub mod probes;
pub mod trace;
pub mod uia_probe;

pub use audio::{AudioActivityReport, AudioEndpointMeter, SilenceReport};

#[cfg(test)]
pub(crate) static TEST_AUDIO_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
