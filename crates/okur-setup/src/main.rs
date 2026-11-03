//! `okur-setup.exe install | uninstall`: see [`okur_setup::args`] for the command line, and
//! `okur_setup::program` for what it does. It runs on Windows alone.

#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    std::process::exit(okur_setup::program::run());
    #[cfg(not(windows))]
    {
        eprintln!("Okur's setup runs on Windows.");
        std::process::exit(1);
    }
}
