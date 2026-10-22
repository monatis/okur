//! The platform's sound output ([`okur_core::sound::SoundOutput`]): a sound file played once,
//! asynchronously, as JAWS plays a notification rule's sound.

/// Windows' own sound playing (`PlaySoundW`, a file, asynchronously, nothing in its place when
/// the file is missing).
#[cfg(windows)]
#[derive(Debug, Default)]
pub struct WindowsSounds;

#[cfg(windows)]
impl okur_core::sound::SoundOutput for WindowsSounds {
    fn play_file(&mut self, path: &str) -> bool {
        use windows::core::PCWSTR;
        use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};
        let path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: a NUL-terminated path that outlives the call; SND_ASYNC copies what it needs.
        unsafe {
            PlaySoundW(
                PCWSTR(path.as_ptr()),
                None,
                SND_FILENAME | SND_ASYNC | SND_NODEFAULT,
            )
        }
        .as_bool()
    }
}
