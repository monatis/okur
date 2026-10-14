use crate::PlatformError;
use std::time::{Duration, Instant};

/// Audio activity report captured directly from the hardware audio endpoint / soundcard.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioActivityReport {
    /// Maximum peak amplitude recorded during the sampling window (0.0 to 1.0).
    pub max_peak: f32,
    /// Average peak amplitude across non-zero samples.
    pub average_peak: f32,
    /// Number of samples where audio amplitude exceeded the silence threshold.
    pub active_samples: usize,
    /// Total number of samples evaluated.
    pub total_samples: usize,
    /// Whether audible sound was actively playing through the soundcard.
    pub detected_audio: bool,
}

/// Outcome of [`AudioEndpointMeter::wait_for_silence`].
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SilenceReport {
    /// Silence was reached (or no audio started within the onset window) before the deadline.
    pub settled: bool,
    /// Any audio above the threshold was observed.
    pub heard_audio: bool,
    pub elapsed_ms: u128,
}

/// Peak meter on the default render endpoint (the whole system mix, not one process).
#[cfg(windows)]
struct PeakMeter(windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation);

#[cfg(windows)]
impl PeakMeter {
    fn open() -> Result<Self, PlatformError> {
        use windows::Win32::Media::Audio::{
            eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
        };
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
        };
        let audio_err =
            |what: &str, e: windows::core::Error| PlatformError::AudioError(format!("{what}: {e}"));
        // SAFETY: standard COM activation of the default render endpoint's meter; a repeated
        // CoInitializeEx on the same thread returns S_FALSE, which is fine.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| audio_err("Failed to create MMDeviceEnumerator", e))?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| audio_err("Failed to get default audio endpoint", e))?;
            let meter = device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| audio_err("Failed to activate IAudioMeterInformation", e))?;
            Ok(Self(meter))
        }
    }

    fn peak(&self) -> f32 {
        // SAFETY: the meter interface stays valid for the lifetime of `self`.
        unsafe { self.0.GetPeakValue() }.unwrap_or(0.0)
    }
}

/// Programmatic audio endpoint meter querying the default playback device via WASAPI.
pub struct AudioEndpointMeter;

impl AudioEndpointMeter {
    /// Polls the default output every millisecond from `start` until its peak reaches
    /// `threshold`, returning the time to the first sound: speech latency, measured.
    pub fn time_to_sound(
        start: Instant,
        threshold: f32,
        timeout: Duration,
    ) -> Result<Option<Duration>, PlatformError> {
        #[cfg(windows)]
        {
            let meter = PeakMeter::open()?;
            while start.elapsed() < timeout {
                if meter.peak() >= threshold {
                    return Ok(Some(start.elapsed()));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(None)
        }
        #[cfg(not(windows))]
        {
            let _ = (start, threshold, timeout);
            Err(PlatformError::Unsupported(
                "Audio endpoint metering only supported on Windows.".to_string(),
            ))
        }
    }

    /// Queries the instantaneous peak amplitude (0.0 to 1.0) of the default output endpoint.
    pub fn get_peak_amplitude() -> Result<f32, PlatformError> {
        #[cfg(windows)]
        {
            Ok(PeakMeter::open()?.peak())
        }
        #[cfg(not(windows))]
        {
            Err(PlatformError::Unsupported(
                "Audio endpoint metering only supported on Windows.".to_string(),
            ))
        }
    }

    /// Samples the default output endpoint for `duration` and summarizes audio activity.
    pub fn monitor_activity(
        duration: Duration,
        sample_interval: Duration,
        silence_threshold: f32,
    ) -> Result<AudioActivityReport, PlatformError> {
        #[cfg(windows)]
        {
            let meter = PeakMeter::open()?;
            let start = Instant::now();
            let mut max_peak = 0.0f32;
            let mut peak_sum = 0.0f32;
            let mut active_samples = 0;
            let mut total_samples = 0;
            while start.elapsed() < duration {
                let peak = meter.peak();
                total_samples += 1;
                max_peak = max_peak.max(peak);
                if peak > silence_threshold {
                    active_samples += 1;
                    peak_sum += peak;
                }
                std::thread::sleep(sample_interval);
            }
            Ok(AudioActivityReport {
                max_peak,
                average_peak: if active_samples > 0 {
                    peak_sum / (active_samples as f32)
                } else {
                    0.0
                },
                active_samples,
                total_samples,
                detected_audio: active_samples > 0,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = (duration, sample_interval, silence_threshold);
            Err(PlatformError::Unsupported(
                "Audio endpoint metering only supported on Windows.".to_string(),
            ))
        }
    }

    /// Blocks until the output endpoint has been silent for `quiet` after audio was heard, or
    /// until `onset` passes with no audio at all (an action that produced no speech), or until
    /// `max` elapses (`settled: false`).
    pub fn wait_for_silence(
        onset: Duration,
        quiet: Duration,
        max: Duration,
        silence_threshold: f32,
    ) -> Result<SilenceReport, PlatformError> {
        #[cfg(windows)]
        {
            let meter = PeakMeter::open()?;
            let start = Instant::now();
            let mut heard_audio = false;
            let mut last_sound = start;
            loop {
                let now = Instant::now();
                if meter.peak() > silence_threshold {
                    heard_audio = true;
                    last_sound = now;
                }
                let settled = if heard_audio {
                    now.duration_since(last_sound) >= quiet
                } else {
                    now.duration_since(start) >= onset
                };
                if settled || now.duration_since(start) >= max {
                    return Ok(SilenceReport {
                        settled,
                        heard_audio,
                        elapsed_ms: start.elapsed().as_millis(),
                    });
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (onset, quiet, max, silence_threshold);
            Err(PlatformError::Unsupported(
                "Audio endpoint metering only supported on Windows.".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    #[ignore = "live: needs a default audio render endpoint"]
    fn test_audio_endpoint_meter_query() {
        let _lock = crate::TEST_AUDIO_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let peak = AudioEndpointMeter::get_peak_amplitude();
        assert!(
            peak.is_ok(),
            "Audio endpoint meter should successfully query default playback device: {:?}",
            peak
        );
        if let Ok(val) = peak {
            assert!(
                (0.0..=1.0).contains(&val),
                "Peak value should be within [0.0, 1.0], got {}",
                val
            );
        }
    }

    /// Live: speaks aloud and meters the whole system mix on the default endpoint,
    /// so it only proves that *some* audio played. Run with `cargo test -- --ignored`
    /// on an unmuted machine with no other audio playing.
    #[test]
    #[cfg(windows)]
    #[ignore = "live: speaks aloud and needs an unmuted audio endpoint"]
    fn test_audio_endpoint_meter_detects_speech() {
        let _lock = crate::TEST_AUDIO_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        use crate::tts::{Sapi5Engine, TtsEngine};

        let sapi = Sapi5Engine::new();
        assert!(sapi.is_ok(), "SAPI 5 must initialize for this live test");
        let Ok(mut sapi) = sapi else { return };
        assert!(sapi
            .speak("Audio device verification test phrase.", true)
            .is_ok());

        let report = AudioEndpointMeter::monitor_activity(
            Duration::from_millis(500),
            Duration::from_millis(25),
            0.001,
        );
        let _ = sapi.stop();

        assert!(
            report.as_ref().is_ok_and(|rep| rep.detected_audio),
            "No audio detected on the default endpoint while speaking: {report:?}"
        );
    }
}
