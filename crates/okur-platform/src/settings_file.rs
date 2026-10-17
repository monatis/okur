//! Okur's settings file: what the settings dialog changed, kept between runs as JSON.
//! A setting missing from the file has its default, so the file only grows as settings do.

use std::path::{Path, PathBuf};

use okur_core::settings::{Settings, SettingsStore};

pub struct SettingsFile {
    path: PathBuf,
}

impl SettingsFile {
    /// The user's settings file: `%APPDATA%\Okur\settings.json` on Windows,
    /// `~/Library/Application Support/Okur/settings.json` on macOS, and
    /// `$XDG_CONFIG_HOME/okur/settings.json` (else `~/.config/okur/settings.json`) elsewhere.
    pub fn user() -> Option<Self> {
        let env = |name: &str| std::env::var_os(name).map(PathBuf::from);
        let folder = if cfg!(windows) {
            env("APPDATA")?.join("Okur")
        } else if cfg!(target_os = "macos") {
            env("HOME")?.join("Library/Application Support/Okur")
        } else {
            env("XDG_CONFIG_HOME")
                .or_else(|| env("HOME").map(|home| home.join(".config")))?
                .join("okur")
        };
        Some(Self::at(folder.join("settings.json")))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved settings; `None` when there is no file, or it cannot be read (logged).
    pub fn load(&self) -> Option<Settings> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        serde_json::from_str(&text)
            .map_err(|e| tracing::warn!(path = ?self.path, error = %e, "settings file not read"))
            .ok()
    }
}

impl SettingsStore for SettingsFile {
    /// Writes the whole file anew, through a temporary file, so that a failure never leaves
    /// half a file.
    fn save(&self, settings: &Settings) {
        let written = (|| -> std::io::Result<()> {
            if let Some(folder) = self.path.parent() {
                std::fs::create_dir_all(folder)?;
            }
            let text = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
            let temporary = self.path.with_extension("json.tmp");
            std::fs::write(&temporary, text)?;
            std::fs::rename(&temporary, &self.path)
        })();
        if let Err(e) = written {
            tracing::warn!(path = ?self.path, error = %e, "settings not saved");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use okur_core::settings::{KeyboardLayout, KeyboardSettings, TypingEcho, ViewerDisplay};

    #[test]
    fn settings_survive_a_save_and_missing_ones_take_their_defaults() {
        let folder =
            std::env::temp_dir().join(format!("okur-settings-test-{}", std::process::id()));
        let file = SettingsFile::at(folder.join("settings.json"));
        assert_eq!(file.load(), None);
        let mut settings = Settings {
            keyboard: KeyboardSettings::for_layout(KeyboardLayout::Laptop),
            typing_echo: TypingEcho::None,
            viewer_display: ViewerDisplay::SpeechOnly,
            ..Default::default()
        };
        settings.voice.rate = 4;
        file.save(&settings);
        assert_eq!(file.load(), Some(settings));
        // A file from an older Okur, with fewer settings.
        let _ = std::fs::write(file.path(), r#"{"typing_echo":"none"}"#);
        let expected = Settings {
            typing_echo: TypingEcho::None,
            ..Default::default()
        };
        assert_eq!(file.load(), Some(expected));
        let _ = std::fs::remove_dir_all(&folder);
    }
}
