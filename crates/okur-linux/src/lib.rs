//! Okur's driver for Linux, Okur's first port after Windows (docs/plans/2026-11-05-linux-first.md):
//! the keyboard (an X11 hook of XInput2 grabs on every application window, then the
//! compositor's `KeyboardMonitor` on Wayland), AT-SPI events and reads through `crates/xa11y`,
//! and speech through speech-dispatcher. What each part rests on was proved in the Linux lab
//! (`skills/linux-lab/`). It is built on Linux alone; elsewhere the crate is empty.
#![cfg(target_os = "linux")]

pub mod keyboard;

pub use okur_linux_model::ssip;
