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

/// Longest piece of text synthesized at once: the voice starts after the first piece.
const MAX_CHUNK_CHARS: usize = 160;

/// An utterance split for synthesis: by line, and long lines after sentence ends (or at a
/// space). Synthesis takes time in proportion to the text, so a long utterance (Say All, a
/// whole page) synthesized in one piece would delay its first word, and an interruption
/// could not stop the synthesis already running.
pub fn speech_chunks(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    for line in text.split(['\n', '\r', '\u{2028}', '\u{2029}']) {
        let mut current = String::new();
        let mut count = 0usize;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            current.push(c);
            count += 1;
            let at_space = chars.peek().is_some_and(|n| n.is_whitespace());
            let sentence_end = matches!(c, '.' | '!' | '?' | ';' | ':') && at_space;
            if (sentence_end && count >= MAX_CHUNK_CHARS / 3)
                || (count >= MAX_CHUNK_CHARS && c.is_whitespace())
            {
                chunks.push(std::mem::take(&mut current));
                count = 0;
            }
        }
        chunks.push(current);
    }
    chunks.retain(|c| !c.trim().is_empty());
    chunks
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

/// A pitch change on SAPI's scale (-10 to 10) as OneCore's audio pitch (0 to 2, 1 is
/// normal): +4, a capital letter, is 1.4.
pub fn audio_pitch(pitch: i32) -> f64 {
    1.0 + f64::from(pitch.clamp(-10, 10)) / 10.0
}

#[cfg(windows)]
pub use engine::OneCoreEngine;

#[cfg(windows)]
mod engine {
    use super::{
        audio_pitch, parse_wav, speaking_rate, speech_chunks, trim_leading_silence, WavFormat,
    };
    use crate::tts::{TtsEngine, TtsError, VoiceInfo};
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use windows::core::HSTRING;
    use windows::Media::SpeechSynthesis::{
        SpeechAppendedSilence, SpeechPunctuationSilence, SpeechSynthesisStream, SpeechSynthesizer,
    };
    use windows::Storage::Streams::DataReader;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioClient, IAudioRenderClient, IMMDeviceEnumerator,
        MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
        AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, WAVEFORMATEX,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows_future::AsyncStatus;

    /// The playback buffer: 60 ms, in 100 ns units.
    const BUFFER_DURATION: i64 = 600_000;
    const POLL: Duration = Duration::from_millis(4);
    /// Samples quieter than this before speech are dropped.
    const SILENCE_THRESHOLD: i16 = 256;
    /// How often a synthesis in flight checks for an interruption.
    const SYNTHESIS_POLL: Duration = Duration::from_millis(2);

    enum Command {
        Speak {
            text: String,
            /// The pitch change, on SAPI's scale.
            pitch: i32,
            generation: u64,
        },
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

    type Synthesis = windows_future::IAsyncOperation<SpeechSynthesisStream>;

    /// Starts synthesizing `text` in the background.
    fn start(synth: &SpeechSynthesizer, text: &str) -> Option<Synthesis> {
        synth
            .SynthesizeTextToStreamAsync(&HSTRING::from(text))
            .map_err(|e| tracing::warn!("OneCore synthesis failed to start: {e}"))
            .ok()
    }

    /// The WAV a synthesis produced, or `None` when it failed or speech was interrupted
    /// first: an interrupted synthesis is cancelled instead of waited for, so a long stale
    /// utterance never holds up the next one.
    fn finish(op: &Synthesis, cancelled: &dyn Fn() -> bool) -> Option<Vec<u8>> {
        loop {
            match op.Status() {
                Ok(AsyncStatus::Started) => {
                    if cancelled() {
                        let _ = op.Cancel();
                        return None;
                    }
                    std::thread::sleep(SYNTHESIS_POLL);
                }
                Ok(AsyncStatus::Completed) => break,
                other => {
                    tracing::warn!("OneCore synthesis ended with {other:?}");
                    return None;
                }
            }
        }
        let read = || -> windows::core::Result<Vec<u8>> {
            let stream = op.GetResults()?;
            let size = u32::try_from(stream.Size()?).unwrap_or(u32::MAX);
            let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
            let loaded = reader.LoadAsync(size)?.join()?;
            let mut bytes = vec![0u8; usize::try_from(loaded).unwrap_or(0)];
            reader.ReadBytes(&mut bytes)?;
            Ok(bytes)
        };
        read()
            .map_err(|e| tracing::warn!("OneCore stream read failed: {e}"))
            .ok()
    }

    /// Speaks an utterance chunk by chunk: the next chunk is synthesized while the current
    /// one plays, and an interruption stops playback and cancels the synthesis in flight.
    fn speak_chunks(
        synth: &SpeechSynthesizer,
        output: &mut Option<Output>,
        chunks: &[String],
        cancelled: &dyn Fn() -> bool,
    ) {
        let started = Instant::now();
        let mut next = chunks.first().and_then(|c| start(synth, c));
        for index in 0..chunks.len() {
            let Some(op) = next.take() else {
                break;
            };
            let Some(wav) = finish(&op, cancelled) else {
                break;
            };
            next = chunks.get(index + 1).and_then(|c| start(synth, c));
            let Some((format, pcm)) = parse_wav(&wav) else {
                continue;
            };
            let pcm = trim_leading_silence(pcm, format, SILENCE_THRESHOLD);
            if index == 0 {
                tracing::debug!(ms = started.elapsed().as_millis(), "OneCore first chunk");
            }
            if output.as_ref().is_none_or(|o| o.format != format) {
                if let Some(old) = output.take() {
                    old.stop();
                }
                *output = Output::open(format)
                    .map_err(|e| tracing::warn!("audio output: {e}"))
                    .ok();
            }
            let Some(out) = output.as_ref() else {
                break;
            };
            let last = index + 1 == chunks.len();
            if let Err(e) = out.play(pcm, cancelled, last) {
                tracing::warn!("audio playback: {e}");
                *output = None;
                break;
            }
            if cancelled() {
                break;
            }
        }
        if let Some(op) = next {
            let _ = op.Cancel();
        }
        if let Some(out) = output.as_ref() {
            out.stop();
        }
    }

    /// A WASAPI shared-mode stream on the default output, for one PCM format.
    struct Output {
        format: WavFormat,
        client: IAudioClient,
        render: IAudioRenderClient,
        frames: u32,
        /// The stream is started (between chunks of an utterance).
        running: std::cell::Cell<bool>,
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
                    running: std::cell::Cell::new(false),
                })
            }
        }

        /// Queues `pcm` for playback, starting the stream if needed, until all of it is
        /// written, or until `cancelled` (then buffered audio is dropped at once). With
        /// `drain`, also waits until the device has played it (the utterance's last chunk);
        /// otherwise the stream keeps running for the next chunk, without a gap.
        fn play(
            &self,
            pcm: &[u8],
            cancelled: &dyn Fn() -> bool,
            drain: bool,
        ) -> windows::core::Result<()> {
            let frame = usize::from(self.format.block_align.max(1));
            let mut pending = pcm;
            // SAFETY: the render buffer is written within the frames WASAPI granted.
            unsafe {
                if !self.running.get() {
                    self.client.Start()?;
                    self.running.set(true);
                }
                loop {
                    if cancelled() {
                        self.stop();
                        return Ok(());
                    }
                    let padding = self.client.GetCurrentPadding()?;
                    if pending.len() < frame {
                        // Drained once the device has played what is queued.
                        if !drain || padding == 0 {
                            break;
                        }
                        std::thread::sleep(POLL);
                        continue;
                    }
                    let free = self.frames.saturating_sub(padding);
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
            }
            Ok(())
        }

        /// Stops the stream and drops whatever it still holds.
        fn stop(&self) {
            if self.running.replace(false) {
                // SAFETY: plain calls on a valid, started client.
                unsafe {
                    let _ = self.client.Stop();
                    let _ = self.client.Reset();
                }
            }
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
                // No silence padded after each utterance or at punctuation, as NVDA sets it:
                // the next utterance follows at once (Windows 10 1803 and later).
                if let Ok(options) = synth.Options() {
                    let _ = options.SetAppendedSilence(SpeechAppendedSilence::Min);
                    let _ = options.SetPunctuationSilence(SpeechPunctuationSilence::Min);
                }
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
                Command::Speak {
                    text,
                    pitch,
                    generation,
                } => {
                    let cancelled = || shared.generation.load(Ordering::SeqCst) != generation;
                    if cancelled() {
                        shared.pending.fetch_sub(1, Ordering::SeqCst);
                        continue;
                    }
                    // A changed pitch (a capital letter) holds for this utterance only.
                    let set_pitch = |value: f64| {
                        if let Err(e) = synth.Options().and_then(|o| o.SetAudioPitch(value)) {
                            tracing::warn!("OneCore pitch: {e}");
                        }
                    };
                    if pitch != 0 {
                        set_pitch(audio_pitch(pitch));
                    }
                    speak_chunks(&synth, &mut output, &speech_chunks(&text), &cancelled);
                    if pitch != 0 {
                        set_pitch(1.0);
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
            self.speak_pitched(text, 0, interrupt)
        }

        fn speak_pitched(
            &mut self,
            text: &str,
            pitch: i32,
            interrupt: bool,
        ) -> Result<(), TtsError> {
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
                pitch,
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
    fn utterances_split_by_line_and_long_sentences() {
        assert_eq!(
            speech_chunks("Title is\nSR Editor Fixture"),
            ["Title is", "SR Editor Fixture"]
        );
        assert_eq!(speech_chunks("left paren"), ["left paren"]);
        assert!(speech_chunks(" \n\n ").is_empty());
        let sentence = "The library will stay open late on weekdays from April onward. ";
        let long = sentence.repeat(6);
        let chunks = speech_chunks(&long);
        assert!(chunks.len() >= 3, "{chunks:?}");
        assert!(chunks
            .iter()
            .all(|c| c.chars().count() <= MAX_CHUNK_CHARS * 2));
        assert_eq!(chunks.concat(), long.trim_end());
        // A long run without sentence ends still breaks, at a space.
        let words = "word ".repeat(100);
        assert!(speech_chunks(&words).len() >= 3);
    }

    #[test]
    fn rates_map_onto_onecore_speeds() {
        assert!((speaking_rate(0) - 1.0).abs() < f64::EPSILON);
        assert!((speaking_rate(10) - 3.0).abs() < 1e-9);
        assert!((speaking_rate(-10) - 0.5).abs() < 1e-9);
        assert!((speaking_rate(99) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn pitch_changes_map_onto_onecore_pitch() {
        assert!((audio_pitch(0) - 1.0).abs() < f64::EPSILON);
        assert!((audio_pitch(4) - 1.4).abs() < 1e-9);
        assert!((audio_pitch(-10)).abs() < 1e-9);
        assert!((audio_pitch(20) - 2.0).abs() < 1e-9);
    }
}
