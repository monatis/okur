use super::{TtsEngine, TtsError, VoiceInfo};

#[cfg(windows)]
use windows::{
    core::PCWSTR,
    Win32::{
        Media::Speech::{
            ISpVoice, SpVoice, SPF_ASYNC, SPF_IS_NOT_XML, SPF_IS_XML, SPF_PURGEBEFORESPEAK,
            SPVOICESTATUS,
        },
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
    },
};

/// Genuine Windows SAPI 5 Text-To-Speech driver using COM `ISpVoice`.
#[derive(Debug)]
pub struct Sapi5Engine {
    #[cfg(windows)]
    voice: ISpVoice,
    /// The Say All parts queued: each one's SAPI stream number and mark, oldest first.
    #[cfg(windows)]
    marks: std::collections::VecDeque<(u32, u64)>,
    /// The current stream last logged, so the debug log shows its changes only.
    #[cfg(windows)]
    last_stream: u32,
    rate: i32,
    volume: u16,
    is_paused: bool,
}

// Safety: `ISpVoice` is a free-threaded COM interface that can be marshaled across threads.
unsafe impl Send for Sapi5Engine {}
unsafe impl Sync for Sapi5Engine {}

impl Sapi5Engine {
    /// Creates and initializes a new Windows SAPI 5 voice instance.
    pub fn new() -> Result<Self, TtsError> {
        #[cfg(windows)]
        {
            unsafe {
                // Initialize COM library on the calling thread (ignoring RPC_E_CHANGED_MODE if already initialized)
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

                let voice: ISpVoice =
                    CoCreateInstance(&SpVoice, None, CLSCTX_ALL).map_err(|e| {
                        TtsError::InitFailed(format!(
                            "Failed to create SAPI 5 SpVoice COM object: {}",
                            e
                        ))
                    })?;

                let mut rate = 0;
                let _ = voice.GetRate(&mut rate);
                let mut volume = 100;
                let _ = voice.GetVolume(&mut volume);

                Ok(Self {
                    voice,
                    marks: std::collections::VecDeque::new(),
                    last_stream: 0,
                    rate,
                    volume,
                    is_paused: false,
                })
            }
        }

        #[cfg(not(windows))]
        {
            Err(TtsError::Unsupported(
                "SAPI 5 is only supported on Windows platforms.".to_string(),
            ))
        }
    }
}

impl TtsEngine for Sapi5Engine {
    fn name(&self) -> &str {
        "SAPI 5"
    }

    fn speak(&mut self, text: &str, interrupt: bool) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            if interrupt {
                self.marks.clear();
            }
            let clean = text.replace(['\u{001d}', '\u{001e}'], "");
            let mut flags = SPF_ASYNC.0 | SPF_IS_NOT_XML.0;
            if interrupt {
                flags |= SPF_PURGEBEFORESPEAK.0;
            }

            let wide: Vec<u16> = clean.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                self.voice
                    .Speak(PCWSTR(wide.as_ptr()), flags as u32, None)
                    .map_err(|e| {
                        TtsError::SynthesisFailed(format!("SAPI 5 Speak failed: {}", e))
                    })?;
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = (text, interrupt);
            Err(TtsError::Unsupported(
                "SAPI 5 is only supported on Windows.".to_string(),
            ))
        }
    }

    fn speak_pitched(&mut self, text: &str, pitch: i32, interrupt: bool) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            if pitch == 0 {
                return self.speak(text, interrupt);
            }
            // SAPI XML: the pitch element changes the voice's middle pitch, as JAWS's own
            // fragments carry it (SPVPITCH::MiddleAdj).
            let mut escaped = String::new();
            for c in text.replace(['\u{001d}', '\u{001e}'], "").chars() {
                match c {
                    '&' => escaped.push_str("&amp;"),
                    '<' => escaped.push_str("&lt;"),
                    '>' => escaped.push_str("&gt;"),
                    '"' => escaped.push_str("&quot;"),
                    '\'' => escaped.push_str("&apos;"),
                    c => escaped.push(c),
                }
            }
            let xml = format!(
                "<pitch middle=\"{}\">{escaped}</pitch>",
                pitch.clamp(-10, 10)
            );
            let mut flags = SPF_ASYNC.0 | SPF_IS_XML.0;
            if interrupt {
                flags |= SPF_PURGEBEFORESPEAK.0;
                self.marks.clear();
            }
            let wide: Vec<u16> = xml.encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: a null-terminated string that outlives the asynchronous call (SAPI
            // copies it before returning).
            unsafe {
                self.voice
                    .Speak(PCWSTR(wide.as_ptr()), flags as u32, None)
                    .map_err(|e| TtsError::SynthesisFailed(format!("SAPI 5 Speak failed: {e}")))?;
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = pitch;
            self.speak(text, interrupt)
        }
    }

    /// SAPI numbers every asynchronous Speak call's stream; the part's stream number is kept
    /// with its mark.
    fn speak_marked(&mut self, text: &str, mark: u64, interrupt: bool) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            if interrupt {
                self.marks.clear();
            }
            let clean = text.replace(['\u{001d}', '\u{001e}'], "");
            let mut flags = SPF_ASYNC.0 | SPF_IS_NOT_XML.0;
            if interrupt {
                flags |= SPF_PURGEBEFORESPEAK.0;
            }
            let wide: Vec<u16> = clean.encode_utf16().chain(std::iter::once(0)).collect();
            let mut stream = 0u32;
            // SAFETY: a null-terminated string SAPI copies before returning, and a stream
            // number written before it returns.
            unsafe {
                self.voice
                    .Speak(PCWSTR(wide.as_ptr()), flags as u32, Some(&mut stream))
                    .map_err(|e| TtsError::SynthesisFailed(format!("SAPI 5 Speak failed: {e}")))?;
            }
            tracing::debug!(stream, mark, "SAPI Say All part queued");
            self.marks.push_back((stream, mark));
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = mark;
            self.speak(text, interrupt)
        }
    }

    /// The latest part whose stream SAPI has started (`SPVOICESTATUS::ulCurrentStream`).
    fn reached(&mut self) -> Option<u64> {
        #[cfg(windows)]
        {
            let front = self.marks.front()?.0;
            let mut status = SPVOICESTATUS::default();
            // SAFETY: plain COM call writing into a local status.
            unsafe { self.voice.GetStatus(&mut status, std::ptr::null_mut()) }.ok()?;
            if status.ulCurrentStream != self.last_stream {
                self.last_stream = status.ulCurrentStream;
                tracing::debug!(
                    current = status.ulCurrentStream,
                    last_queued = status.ulLastStreamQueued,
                    running = status.dwRunningState,
                    front,
                    "SAPI stream"
                );
            }
            if status.ulCurrentStream < front {
                return None;
            }
            let mut latest = None;
            while let Some(&(stream, mark)) = self.marks.front() {
                if stream > status.ulCurrentStream {
                    break;
                }
                latest = Some(mark);
                self.marks.pop_front();
            }
            latest
        }

        #[cfg(not(windows))]
        {
            None
        }
    }

    fn follows_marks(&self) -> bool {
        cfg!(windows)
    }

    fn stop(&mut self) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            self.marks.clear();
            let empty: [u16; 1] = [0];
            let flags = (SPF_PURGEBEFORESPEAK.0 | SPF_ASYNC.0) as u32;
            unsafe {
                let _ = self.voice.Speak(PCWSTR(empty.as_ptr()), flags, None);
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            Err(TtsError::Unsupported(
                "SAPI 5 is only supported on Windows.".to_string(),
            ))
        }
    }

    fn pause(&mut self) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            unsafe {
                self.voice.Pause().map_err(|e| {
                    TtsError::SynthesisFailed(format!("SAPI 5 Pause failed: {}", e))
                })?;
            }
            self.is_paused = true;
            Ok(())
        }

        #[cfg(not(windows))]
        {
            Err(TtsError::Unsupported(
                "SAPI 5 is only supported on Windows.".to_string(),
            ))
        }
    }

    fn resume(&mut self) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            unsafe {
                self.voice.Resume().map_err(|e| {
                    TtsError::SynthesisFailed(format!("SAPI 5 Resume failed: {}", e))
                })?;
            }
            self.is_paused = false;
            Ok(())
        }

        #[cfg(not(windows))]
        {
            Err(TtsError::Unsupported(
                "SAPI 5 is only supported on Windows.".to_string(),
            ))
        }
    }

    fn set_rate(&mut self, rate: i32) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            let clamped = rate.clamp(-10, 10);
            unsafe {
                self.voice.SetRate(clamped).map_err(|e| {
                    TtsError::SynthesisFailed(format!("SAPI 5 SetRate failed: {}", e))
                })?;
            }
            self.rate = clamped;
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = rate;
            Err(TtsError::Unsupported(
                "SAPI 5 is only supported on Windows.".to_string(),
            ))
        }
    }

    fn get_rate(&self) -> i32 {
        self.rate
    }

    fn set_volume(&mut self, volume: u16) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            let clamped = volume.min(100);
            unsafe {
                self.voice.SetVolume(clamped).map_err(|e| {
                    TtsError::SynthesisFailed(format!("SAPI 5 SetVolume failed: {}", e))
                })?;
            }
            self.volume = clamped;
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = volume;
            Err(TtsError::Unsupported(
                "SAPI 5 is only supported on Windows.".to_string(),
            ))
        }
    }

    fn get_volume(&self) -> u16 {
        self.volume
    }

    fn is_speaking(&self) -> bool {
        #[cfg(windows)]
        {
            unsafe {
                let mut status = SPVOICESTATUS::default();
                if self
                    .voice
                    .GetStatus(&mut status, std::ptr::null_mut())
                    .is_ok()
                {
                    // dwRunningState == 2 indicates SPRS_IS_SPEAKING
                    return status.dwRunningState == 2;
                }
            }
            false
        }

        #[cfg(not(windows))]
        {
            false
        }
    }

    /// The voices SAPI lists (its voice tokens): the token id, and the description as the
    /// name. On the reference machine this is where the speech recorder voice is chosen, so
    /// that a live run of Okur records what it sends its synthesizer, as JAWS's runs do.
    fn get_voices(&self) -> Vec<VoiceInfo> {
        #[cfg(windows)]
        {
            voice_tokens()
                .into_iter()
                .map(|(id, name, _)| VoiceInfo {
                    id,
                    name,
                    language: None,
                })
                .collect()
        }
        #[cfg(not(windows))]
        {
            Vec::new()
        }
    }

    /// The voice whose token id or name is `voice_id`.
    fn set_voice(&mut self, voice_id: &str) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            let found = voice_tokens()
                .into_iter()
                .find(|(id, name, _)| id == voice_id || name == voice_id);
            let Some((_, _, token)) = found else {
                return Err(TtsError::VoiceNotFound(voice_id.to_string()));
            };
            // SAFETY: a voice token SAPI enumerated, handed to the voice that uses it.
            unsafe { self.voice.SetVoice(&token) }
                .map_err(|e| TtsError::SynthesisFailed(format!("SAPI 5 SetVoice failed: {}", e)))
        }
        #[cfg(not(windows))]
        {
            Err(TtsError::VoiceNotFound(voice_id.to_string()))
        }
    }

    fn wait_until_done(&mut self, timeout_ms: u32) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
            unsafe {
                self.voice.WaitUntilDone(timeout_ms).map_err(|e| {
                    TtsError::SynthesisFailed(format!("SAPI 5 WaitUntilDone failed: {}", e))
                })?;
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = timeout_ms;
            Ok(())
        }
    }
}

/// SAPI's voice tokens: id, description and the token, in SAPI's order.
#[cfg(windows)]
fn voice_tokens() -> Vec<(
    String,
    String,
    windows::Win32::Media::Speech::ISpObjectToken,
)> {
    use windows::Win32::Media::Speech::{
        ISpObjectToken, ISpObjectTokenCategory, SpObjectTokenCategory, SPCAT_VOICES,
    };
    use windows::Win32::System::Com::CoTaskMemFree;
    // A string SAPI allocated, copied and freed.
    fn owned(value: windows::core::PWSTR) -> String {
        // SAFETY: SAPI returns a CoTaskMemAlloc'd string, freed here after copying.
        unsafe {
            let text = value.to_string().unwrap_or_default();
            CoTaskMemFree(Some(value.0 as *const std::ffi::c_void));
            text
        }
    }
    let mut voices = Vec::new();
    // SAFETY: SAPI's token category and enumeration, used as documented.
    unsafe {
        let Ok(category) =
            CoCreateInstance::<_, ISpObjectTokenCategory>(&SpObjectTokenCategory, None, CLSCTX_ALL)
        else {
            return voices;
        };
        if category.SetId(SPCAT_VOICES, false).is_err() {
            return voices;
        }
        let Ok(tokens) = category.EnumTokens(PCWSTR::null(), PCWSTR::null()) else {
            return voices;
        };
        loop {
            let mut token: [Option<ISpObjectToken>; 1] = [None];
            let mut fetched = 0u32;
            if tokens
                .Next(1, token.as_mut_ptr(), Some(&mut fetched))
                .is_err()
                || fetched == 0
            {
                break;
            }
            let Some(token) = token.into_iter().next().flatten() else {
                break;
            };
            let id = token.GetId().map(owned).unwrap_or_default();
            let name = token
                .GetStringValue(PCWSTR::null())
                .map(owned)
                .unwrap_or_default();
            voices.push((id, name, token));
        }
    }
    voices
}
