//! Sounds Okur plays: a notification rule's sound in the notification's place (JAWS's Play
//! Sound action). The platform plays them ([`SoundOutput`]); tests record them.

/// Plays sound files.
pub trait SoundOutput: Send {
    /// Plays the sound file at `path` once, without waiting for it to end. False when it
    /// cannot (no such file, no sound device, unsupported).
    fn play_file(&mut self, path: &str) -> bool;
}

/// Sounds kept as they were asked for, for tests.
#[derive(Debug, Clone, Default)]
pub struct RecordingSounds {
    played: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl RecordingSounds {
    /// The files played so far, emptied.
    pub fn take(&self) -> Vec<String> {
        self.played
            .lock()
            .map(|mut played| std::mem::take(&mut *played))
            .unwrap_or_default()
    }
}

impl SoundOutput for RecordingSounds {
    fn play_file(&mut self, path: &str) -> bool {
        if let Ok(mut played) = self.played.lock() {
            played.push(path.to_owned());
        }
        true
    }
}
