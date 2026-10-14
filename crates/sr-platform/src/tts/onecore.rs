//! OneCore voices: Windows 10 and 11's `Windows.Media.SpeechSynthesis`, the voices NVDA
//! defaults to.
//!
//! SAPI 5 adds noticeable latency between a key press and speech. Here each utterance is
//! synthesized to PCM in memory, its leading silence is trimmed, and it is played through our
//! own WASAPI shared-mode stream. So speech starts as soon as synthesis returns, and stopping
//! resets the stream's buffer at once. A generation counter makes an interrupting utterance or
//! a stop discard everything queued or playing before it.

#[cfg(not(windows))]
use super::{TtsEngine, TtsError, VoiceInfo};

/// A WAV stream's PCM format and samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavFormat {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub block_align: u16,
}

/// The format and the `data` chunk of a RIFF WAVE file (what OneCore synthesizes).
pub fn parse_wav(bytes: &[u8]) -> Option<(WavFormat, &[u8])> {
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let u16_at = |at: usize| Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let mut format = None;
    let mut at = 12;
    while let (Some(id), Some(len)) = (bytes.get(at..at + 4), u32_at(at + 4)) {
        let body = at + 8;
        let len = usize::try_from(len).ok()?;
        match id {
            b"fmt " => {
                format = Some(WavFormat {
                    channels: u16_at(body + 2)?,
                    sample_rate: u32_at(body + 4)?,
                    block_align: u16_at(body + 12)?,
                    bits_per_sample: u16_at(body + 14)?,
                });
            }
            b"data" => {
                // A streaming writer may leave the size unset; take what is there.
                let end = body.saturating_add(len).min(bytes.len());
                return Some((format?, bytes.get(body..end)?));
            }
            _ => {}
        }
        // Chunks are padded to an even size.
        at = body + len + (len & 1);
    }
    None
}

/// `pcm` without its leading near-silence (16-bit samples quieter than `threshold`), so
/// speech starts at once. Whole frames are kept.
pub fn trim_leading_silence(pcm: &[u8], format: WavFormat, threshold: i16) -> &[u8] {
    if format.bits_per_sample != 16 || format.block_align == 0 {
        return pcm;
    }
    let frame = usize::from(format.block_align);
    let first_loud = pcm
        .as_chunks::<2>()
        .0
        .iter()
        .position(|s| i16::from_le_bytes(*s).unsigned_abs() >= threshold.unsigned_abs())
        .map_or(pcm.len(), |sample| sample * 2);
    pcm.get(first_loud - first_loud % frame..)
        .unwrap_or_default()
}

/// Our -10..10 rate scale as OneCore's speaking rate (0.5 to 6, 1 is normal).
pub fn speaking_rate(rate: i32) -> f64 {
    let rate = f64::from(rate.clamp(-10, 10));
    if rate >= 0.0 {
        1.0 + rate * 0.2
    } else {
        1.0 + rate * 0.05
    }
}

#[cfg(windows)]
pub use engine::OneCoreEngine;

#[cfg(windows)]
mod engine {
    use super::{parse_wav, speaking_rate, trim_leading_silence, WavFormat};
    use crate::tts::{TtsEngine, TtsError, VoiceInfo};
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use windows::core::HSTRING;
    use windows::Media::SpeechSynthesis::SpeechSynthesizer;
    use windows::Storage::Streams::DataReader;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioClient, IAudioRenderClient, IMMDeviceEnumerator,
        MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
        AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, WAVEFORMATEX,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    /// The playback buffer: 60 ms, in 100 ns units.
    const BUFFER_DURATION: i64 = 600_000;
    const POLL: Duration = Duration::from_millis(4);
    /// Samples quieter than this before speech are dropped.
    const SILENCE_THRESHOLD: i16 = 256;

    enum Command {
        Speak { text: String, generation: u64 },
        Voice(String),
        Rate(f64),
        Volume(f64),
    }

    #[derive(Default)]
    struct Shared {
        /// Raised by every interrupt and stop; work from an older generation is dropped.
        generation: AtomicU64,
        /// Utterances sent and not yet played or dropped.
        pending: AtomicUsize,
    }

    /// The OneCore speech driver: a worker thread synthesizes and plays utterances in order.
    pub struct OneCoreEngine {
        commands: Mutex<Sender<Command>>,
        shared: Arc<Shared>,
        voices: Vec<VoiceInfo>,
        rate: i32,
        volume: u16,
    }

    impl OneCoreEngine {
        /// Starts the worker; fails when OneCore speech or its voices are unavailable.
        pub fn new() -> Result<Self, TtsError> {
            let (tx, rx) = channel::<Command>();
            let (ready_tx, ready_rx) = channel::<Result<Vec<VoiceInfo>, TtsError>>();
            let shared = Arc::new(Shared::default());
            let worker_shared = shared.clone();
            std::thread::Builder::new()
                .name("onecore-speech".into())
                .spawn(move || worker(rx, worker_shared, ready_tx))
                .map_err(|e| TtsError::InitFailed(format!("OneCore worker: {e}")))?;
            let voices = ready_rx
                .recv_timeout(Duration::from_secs(5))
                .map_err(|_| TtsError::InitFailed("OneCore did not start".into()))??;
            Ok(Self {
                commands: Mutex::new(tx),
                shared,
                voices,
                rate: 0,
                volume: 100,
            })
        }

        fn send(&self, command: Command) -> Result<(), TtsError> {
            self.commands
                .lock()
                .ok()
                .and_then(|tx| tx.send(command).ok())
                .ok_or_else(|| TtsError::SynthesisFailed("OneCore worker stopped".into()))
        }
    }

    fn voice_list(
        voices: impl IntoIterator<Item = windows::Media::SpeechSynthesis::VoiceInformation>,
    ) -> Vec<VoiceInfo> {
        voices
            .into_iter()
            .filter_map(|v| {
                Some(VoiceInfo {
                    id: v.Id().ok()?.to_string(),
                    name: v.DisplayName().ok()?.to_string(),
                    language: v.Language().ok().map(|l| l.to_string()),
                })
            })
            .collect()
    }

    /// The synthesizer's WAV for `text`.
    fn synthesize(synth: &SpeechSynthesizer, text: &str) -> windows::core::Result<Vec<u8>> {
        let stream = synth
            .SynthesizeTextToStreamAsync(&HSTRING::from(text))?
            .join()?;
        let size = u32::try_from(stream.Size()?).unwrap_or(u32::MAX);
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
        let loaded = reader.LoadAsync(size)?.join()?;
        let mut bytes = vec![0u8; usize::try_from(loaded).unwrap_or(0)];
        reader.ReadBytes(&mut bytes)?;
        Ok(bytes)
    }

    /// A WASAPI shared-mode stream on the default output, for one PCM format.
    struct Output {
        format: WavFormat,
        client: IAudioClient,
        render: IAudioRenderClient,
        frames: u32,
    }

    impl Output {
        fn open(format: WavFormat) -> windows::core::Result<Self> {
            let wave = WAVEFORMATEX {
                wFormatTag: 1, // WAVE_FORMAT_PCM
                nChannels: format.channels,
                nSamplesPerSec: format.sample_rate,
                nAvgBytesPerSec: format.sample_rate * u32::from(format.block_align),
                nBlockAlign: format.block_align,
                wBitsPerSample: format.bits_per_sample,
                cbSize: 0,
            };
            // SAFETY: WASAPI activation and initialization with a valid format.
            unsafe {
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
                let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
                client.Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                    BUFFER_DURATION,
                    0,
                    &wave,
                    None,
                )?;
                let frames = client.GetBufferSize()?;
                let render: IAudioRenderClient = client.GetService()?;
                Ok(Self {
                    format,
                    client,
                    render,
                    frames,
                })
            }
        }

        /// Plays `pcm` to the end, or until `cancelled` (then the buffer is dropped at once).
        fn play(&self, pcm: &[u8], cancelled: &dyn Fn() -> bool) -> windows::core::Result<()> {
            let frame = usize::from(self.format.block_align.max(1));
            let mut chunks = pcm.chunks(frame * 256);
            let mut pending: &[u8] = &[];
            // SAFETY: the render buffer is written within the frames WASAPI granted.
            unsafe {
                self.client.Start()?;
                loop {
                    if cancelled() {
                        break;
                    }
                    if pending.is_empty() {
                        match chunks.next() {
                            Some(chunk) => pending = chunk,
                            None => {
                                // Drained once the device has played what is queued.
                                if self.client.GetCurrentPadding()? == 0 {
                                    break;
                                }
                                std::thread::sleep(POLL);
                                continue;
                            }
                        }
                    }
                    let free = self.frames - self.client.GetCurrentPadding()?;
                    let n = (pending.len() / frame).min(usize::try_from(free).unwrap_or(0));
                    if n == 0 {
                        std::thread::sleep(POLL);
                        continue;
                    }
                    let count = u32::try_from(n).unwrap_or(0);
                    let buffer = self.render.GetBuffer(count)?;
                    let (now, rest) = pending.split_at(n * frame);
                    std::ptr::copy_nonoverlapping(now.as_ptr(), buffer, now.len());
                    self.render.ReleaseBuffer(count, 0)?;
                    pending = rest;
                }
                self.client.Stop()?;
                self.client.Reset()?;
            }
            Ok(())
        }
    }

    fn worker(
        commands: Receiver<Command>,
        shared: Arc<Shared>,
        ready: Sender<Result<Vec<VoiceInfo>, TtsError>>,
    ) {
        // SAFETY: COM (and so WinRT) initialization for this worker thread.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let started = SpeechSynthesizer::new().and_then(|synth| {
            let voices = voice_list(SpeechSynthesizer::AllVoices()?);
            Ok((synth, voices))
        });
        let synth = match started {
            Ok((synth, voices)) if !voices.is_empty() => {
                let _ = ready.send(Ok(voices));
                synth
            }
            Ok(_) => {
                let _ = ready.send(Err(TtsError::InitFailed("no OneCore voices".into())));
                return;
            }
            Err(e) => {
                let _ = ready.send(Err(TtsError::InitFailed(format!("OneCore: {e}"))));
                return;
            }
        };
        let mut output: Option<Output> = None;
        for command in commands {
            match command {
                Command::Speak { text, generation } => {
                    let cancelled = || shared.generation.load(Ordering::SeqCst) != generation;
                    if cancelled() {
                        shared.pending.fetch_sub(1, Ordering::SeqCst);
                        continue;
                    }
                    let started = Instant::now();
                    let result = synthesize(&synth, &text).map(|wav| {
                        let Some((format, pcm)) = parse_wav(&wav) else {
                            tracing::warn!("OneCore produced no PCM for {text:?}");
                            return;
                        };
                        let pcm = trim_leading_silence(pcm, format, SILENCE_THRESHOLD).to_vec();
                        tracing::debug!(ms = started.elapsed().as_millis(), "OneCore synthesis");
                        if output.as_ref().is_none_or(|o| o.format != format) {
                            output = Output::open(format)
                                .map_err(|e| tracing::warn!("audio output: {e}"))
                                .ok();
                        }
                        if let Some(out) = &output {
                            if let Err(e) = out.play(&pcm, &cancelled) {
                                tracing::warn!("audio playback: {e}");
                                output = None;
                            }
                        }
                    });
                    if let Err(e) = result {
                        tracing::warn!("OneCore synthesis failed: {e}");
                    }
                    shared.pending.fetch_sub(1, Ordering::SeqCst);
                }
                Command::Voice(id) => {
                    let found = SpeechSynthesizer::AllVoices().ok().and_then(|voices| {
                        voices.into_iter().find(|v| {
                            v.Id().is_ok_and(|i| i == id.as_str())
                                || v.DisplayName().is_ok_and(|n| {
                                    n.to_string().to_lowercase().contains(&id.to_lowercase())
                                })
                        })
                    });
                    match found.map(|v| synth.SetVoice(&v)) {
                        Some(Ok(())) => {}
                        Some(Err(e)) => tracing::warn!("OneCore voice {id}: {e}"),
                        None => tracing::warn!("OneCore voice {id} not found"),
                    }
                }
                Command::Rate(rate) => {
                    if let Err(e) = synth.Options().and_then(|o| o.SetSpeakingRate(rate)) {
                        tracing::warn!("OneCore rate: {e}");
                    }
                }
                Command::Volume(volume) => {
                    if let Err(e) = synth.Options().and_then(|o| o.SetAudioVolume(volume)) {
                        tracing::warn!("OneCore volume: {e}");
                    }
                }
            }
        }
    }

    impl TtsEngine for OneCoreEngine {
        fn name(&self) -> &str {
            "OneCore"
        }

        fn speak(&mut self, text: &str, interrupt: bool) -> Result<(), TtsError> {
            let generation = if interrupt {
                self.shared.generation.fetch_add(1, Ordering::SeqCst) + 1
            } else {
                self.shared.generation.load(Ordering::SeqCst)
            };
            let clean = text.replace(['\u{001d}', '\u{001e}'], "");
            if clean.trim().is_empty() {
                return Ok(());
            }
            self.shared.pending.fetch_add(1, Ordering::SeqCst);
            let sent = self.send(Command::Speak {
                text: clean,
                generation,
            });
            if sent.is_err() {
                self.shared.pending.fetch_sub(1, Ordering::SeqCst);
            }
            sent
        }

        fn stop(&mut self) -> Result<(), TtsError> {
            self.shared.generation.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn pause(&mut self) -> Result<(), TtsError> {
            Err(TtsError::Unsupported("OneCore pause".into()))
        }

        fn resume(&mut self) -> Result<(), TtsError> {
            Err(TtsError::Unsupported("OneCore resume".into()))
        }

        fn set_rate(&mut self, rate: i32) -> Result<(), TtsError> {
            self.rate = rate.clamp(-10, 10);
            self.send(Command::Rate(speaking_rate(self.rate)))
        }

        fn get_rate(&self) -> i32 {
            self.rate
        }

        fn set_volume(&mut self, volume: u16) -> Result<(), TtsError> {
            self.volume = volume.min(100);
            self.send(Command::Volume(f64::from(self.volume) / 100.0))
        }

        fn get_volume(&self) -> u16 {
            self.volume
        }

        fn is_speaking(&self) -> bool {
            self.shared.pending.load(Ordering::SeqCst) > 0
        }

        fn get_voices(&self) -> Vec<VoiceInfo> {
            self.voices.clone()
        }

        fn set_voice(&mut self, voice_id: &str) -> Result<(), TtsError> {
            let wanted = voice_id.to_lowercase();
            if !self
                .voices
                .iter()
                .any(|v| v.id == voice_id || v.name.to_lowercase().contains(&wanted))
            {
                return Err(TtsError::VoiceNotFound(voice_id.to_owned()));
            }
            self.send(Command::Voice(voice_id.to_owned()))
        }

        fn wait_until_done(&mut self, timeout_ms: u32) -> Result<(), TtsError> {
            let deadline = Instant::now() + Duration::from_millis(u64::from(timeout_ms));
            while self.is_speaking() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(())
        }
    }
}

/// Stand-in on other platforms, where OneCore does not exist.
#[cfg(not(windows))]
pub struct OneCoreEngine;

#[cfg(not(windows))]
impl OneCoreEngine {
    pub fn new() -> Result<Self, TtsError> {
        Err(TtsError::Unsupported(
            "OneCore voices exist only on Windows".into(),
        ))
    }
}

#[cfg(not(windows))]
impl TtsEngine for OneCoreEngine {
    fn name(&self) -> &str {
        "OneCore"
    }
    fn speak(&mut self, _text: &str, _interrupt: bool) -> Result<(), TtsError> {
        Err(TtsError::Unsupported("OneCore".into()))
    }
    fn stop(&mut self) -> Result<(), TtsError> {
        Ok(())
    }
    fn pause(&mut self) -> Result<(), TtsError> {
        Ok(())
    }
    fn resume(&mut self) -> Result<(), TtsError> {
        Ok(())
    }
    fn set_rate(&mut self, _rate: i32) -> Result<(), TtsError> {
        Ok(())
    }
    fn get_rate(&self) -> i32 {
        0
    }
    fn set_volume(&mut self, _volume: u16) -> Result<(), TtsError> {
        Ok(())
    }
    fn get_volume(&self) -> u16 {
        100
    }
    fn is_speaking(&self) -> bool {
        false
    }
    fn get_voices(&self) -> Vec<VoiceInfo> {
        Vec::new()
    }
    fn set_voice(&mut self, voice_id: &str) -> Result<(), TtsError> {
        Err(TtsError::VoiceNotFound(voice_id.to_owned()))
    }
    fn wait_until_done(&mut self, _timeout_ms: u32) -> Result<(), TtsError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&1u16.to_le_bytes()); // mono
        out.extend_from_slice(&22_050u32.to_le_bytes());
        out.extend_from_slice(&44_100u32.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        // An unknown odd-sized chunk before the data, padded to even.
        out.extend_from_slice(b"LIST");
        out.extend_from_slice(&3u32.to_le_bytes());
        out.extend_from_slice(&[1, 2, 3, 0]);
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn wav_data_is_found_after_other_chunks() {
        let bytes = wav(&[0, 10, -4000, 7000]);
        let (format, pcm) = parse_wav(&bytes).unwrap_or((
            WavFormat {
                channels: 0,
                sample_rate: 0,
                bits_per_sample: 0,
                block_align: 0,
            },
            &[],
        ));
        assert_eq!(
            format,
            WavFormat {
                channels: 1,
                sample_rate: 22_050,
                bits_per_sample: 16,
                block_align: 2
            }
        );
        assert_eq!(pcm.len(), 8);
        assert_eq!(trim_leading_silence(pcm, format, 256).len(), 4);
        assert!(parse_wav(b"RIFF\0\0\0\0WAVX").is_none());
    }

    #[test]
    fn rates_map_onto_onecore_speeds() {
        assert!((speaking_rate(0) - 1.0).abs() < f64::EPSILON);
        assert!((speaking_rate(10) - 3.0).abs() < 1e-9);
        assert!((speaking_rate(-10) - 0.5).abs() < 1e-9);
        assert!((speaking_rate(99) - 3.0).abs() < 1e-9);
    }
}
