//! `okur-ui` draws Okur's own windows. Okur starts it, writes to its standard input what to
//! show, one JSON message per line ([`okur_ui_model::ToUi`]), and reads from its standard
//! output what the user did to a window ([`okur_ui_model::FromUi`]). When Okur exits, the
//! input ends, and so does `okur-ui`.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod dialog;
#[cfg(target_os = "linux")]
mod gtk;
#[cfg(windows)]
mod main_window;
#[cfg(any(windows, test))]
// Off Windows only its tests use it.
#[cfg_attr(not(windows), allow(dead_code))]
mod template;
#[cfg(windows)]
mod win32;
#[cfg(windows)]
mod wizard;

fn main() {
    #[cfg(windows)]
    win32::run();
    #[cfg(target_os = "linux")]
    gtk::run();
    // No renderer on this platform yet: what Okur sends is read and dropped, so that Okur
    // never waits on a full pipe.
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        use std::io::BufRead;
        for _ in std::io::stdin().lock().lines() {}
    }
}
