//! Where Okur keeps its files on each system.

use std::path::PathBuf;

fn env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

/// The user's settings folder: `%APPDATA%\Okur` on Windows,
/// `~/Library/Application Support/Okur` on macOS, and `$XDG_CONFIG_HOME/okur` (else
/// `~/.config/okur`) elsewhere.
pub fn settings() -> Option<PathBuf> {
    Some(if cfg!(windows) {
        env("APPDATA")?.join("Okur")
    } else if cfg!(target_os = "macos") {
        env("HOME")?.join("Library/Application Support/Okur")
    } else {
        env("XDG_CONFIG_HOME")
            .or_else(|| env("HOME").map(|home| home.join(".config")))?
            .join("okur")
    })
}

/// The installed reader's own log: `%LOCALAPPDATA%\Okur\okur.log` on Windows (the package's
/// own folder when Okur comes from the Store), `~/Library/Logs/Okur/okur.log` on macOS,
/// `$XDG_STATE_HOME/okur/okur.log` (else `~/.local/state`) elsewhere.
pub fn kept_log() -> Option<PathBuf> {
    let folder = if cfg!(windows) {
        env("LOCALAPPDATA")?.join("Okur")
    } else if cfg!(target_os = "macos") {
        env("HOME")?.join("Library/Logs/Okur")
    } else {
        env("XDG_STATE_HOME")
            .or_else(|| env("HOME").map(|home| home.join(".local/state")))?
            .join("okur")
    };
    Some(folder.join("okur.log"))
}
