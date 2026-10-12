use super::{TtsEngine, TtsError, VoiceInfo};

#[cfg(windows)]
use windows::{
    core::PCWSTR,
    Win32::{
        Media::Speech::{
            ISpVoice, SpVoice, SPF_ASYNC, SPF_IS_NOT_XML, SPF_PURGEBEFORESPEAK, SPVOICESTATUS,
        },
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
    },
};

/// Genuine Windows SAPI 5 Text-To-Speech driver using COM `ISpVoice`.
#[derive(Debug)]
pub struct Sapi5Engine {
    #[cfg(windows)]
    voice: ISpVoice,
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

    fn stop(&mut self) -> Result<(), TtsError> {
        #[cfg(windows)]
        {
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

    fn get_voices(&self) -> Vec<VoiceInfo> {
        vec![VoiceInfo {
            id: "sapi5-default".to_string(),
            name: "SAPI 5 Default Voice".to_string(),
            language: Some("en-US".to_string()),
        }]
    }

    fn set_voice(&mut self, voice_id: &str) -> Result<(), TtsError> {
        if voice_id == "sapi5-default" {
            Ok(())
        } else {
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
