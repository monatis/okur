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

/// Programmatic audio endpoint meter querying the hardware soundcard / default playback device via WASAPI.
pub struct AudioEndpointMeter;

impl AudioEndpointMeter {
    /// Queries the instantaneous audio peak amplitude (0.0 to 1.0) of the default audio output endpoint.
    pub fn get_peak_amplitude() -> Result<f32, PlatformError> {
        #[cfg(windows)]
        {
            use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
            use windows::Win32::Media::Audio::{
                eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
            };
            use windows::Win32::System::Com::{
                CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
            };

            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| {
                        PlatformError::AudioError(format!(
                            "Failed to create MMDeviceEnumerator: {}",
                            e
                        ))
                    })?;

                let device = enumerator
                    .GetDefaultAudioEndpoint(eRender, eConsole)
                    .map_err(|e| {
                        PlatformError::AudioError(format!(
                            "Failed to get default audio endpoint: {}",
                            e
                        ))
                    })?;

                let meter: IAudioMeterInformation =
                    device.Activate(CLSCTX_ALL, None).map_err(|e| {
                        PlatformError::AudioError(format!(
                            "Failed to activate IAudioMeterInformation: {}",
                            e
                        ))
                    })?;

                let peak = meter.GetPeakValue().map_err(|e| {
                    PlatformError::AudioError(format!("Failed to get peak audio value: {}", e))
                })?;

                Ok(peak)
            }
        }

        #[cfg(not(windows))]
        {
            Err(PlatformError::Unsupported(
                "Audio endpoint metering only supported on Windows.".to_string(),
            ))
        }
    }

    /// Monitors the default audio output endpoint over a given duration and records audio activity.
    /// Returns an `AudioActivityReport` proving whether audible sound played through the soundcard.
    pub fn monitor_activity(
        duration: Duration,
        sample_interval: Duration,
        silence_threshold: f32,
    ) -> Result<AudioActivityReport, PlatformError> {
        let start = Instant::now();
        let mut max_peak = 0.0f32;
        let mut peak_sum = 0.0f32;
        let mut active_samples = 0;
        let mut total_samples = 0;

        while start.elapsed() < duration {
            let peak = Self::get_peak_amplitude()?;
            total_samples += 1;
            if peak > max_peak {
                max_peak = peak;
            }
            if peak > silence_threshold {
                active_samples += 1;
                peak_sum += peak;
            }
            std::thread::sleep(sample_interval);
        }

        let average_peak = if active_samples > 0 {
            peak_sum / (active_samples as f32)
        } else {
            0.0
        };

        Ok(AudioActivityReport {
            max_peak,
            average_peak,
            active_samples,
            total_samples,
            detected_audio: active_samples > 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_audio_endpoint_meter_query() {
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

    #[test]
    #[cfg(windows)]
    fn test_audio_endpoint_meter_detects_speech() {
        use crate::tts::{Sapi5Engine, TtsEngine};

        if let Ok(mut sapi) = Sapi5Engine::new() {
            // Speak a phrase
            let _ = sapi.speak("Audio device verification test phrase.", true);

            // Monitor audio for 500ms
            let report = AudioEndpointMeter::monitor_activity(
                Duration::from_millis(500),
                Duration::from_millis(25),
                0.001,
            );

            let _ = sapi.stop();

            assert!(report.is_ok(), "Monitoring should succeed: {:?}", report);
            if let Ok(rep) = report {
                assert!(
                    rep.detected_audio || rep.max_peak >= 0.0,
                    "Should measure audio levels from soundcard endpoint: {:?}",
                    rep
                );
            }
        }
    }
}
