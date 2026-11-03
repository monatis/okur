//! Windows' synthesizers: OneCore's voices and SAPI 5's, made for the runtime's speech thread
//! (`okur_runtime::SynthesizerFactory`).

pub mod onecore;
pub mod sapi5;

pub use okur_runtime::{NoSpeechEngine, SynthesizerFactory, TtsEngine, TtsError, VoiceInfo};
pub use onecore::OneCoreEngine;
pub use sapi5::Sapi5Engine;

use okur_core::settings::Synthesizer;

/// The synthesizer by the name the command line gives it (`--engine`).
pub fn synthesizer_named(name: &str) -> Result<Synthesizer, String> {
    match name.to_lowercase().as_str() {
        "auto" => Ok(Synthesizer::Auto),
        "no-speech" | "nospeech" | "silent" | "silence" | "none" => Ok(Synthesizer::NoSpeech),
        "sapi5" | "sapi" => Ok(Synthesizer::Sapi5),
        "onecore" | "one-core" => Ok(Synthesizer::OneCore),
        _ => Err(format!(
            "Unknown TTS engine '{name}'. Available: auto, onecore, sapi5, no-speech"
        )),
    }
}

/// Windows' synthesizers: OneCore's voices, SAPI 5's, or none.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsSynthesizers;

impl SynthesizerFactory for WindowsSynthesizers {
    fn make(&self, kind: Synthesizer) -> Result<Box<dyn TtsEngine>, TtsError> {
        let engine: Box<dyn TtsEngine> = match kind {
            Synthesizer::NoSpeech => Box::new(NoSpeechEngine::new()),
            Synthesizer::Sapi5 => {
                let sapi = Sapi5Engine::new()?;
                Box::new(sapi)
            }
            Synthesizer::OneCore => Box::new(OneCoreEngine::new()?),
            Synthesizer::Auto => {
                // OneCore voices respond fastest; SAPI 5 where they are missing.
                #[cfg(windows)]
                {
                    match OneCoreEngine::new() {
                        Ok(onecore) => Box::new(onecore),
                        Err(e) => {
                            tracing::warn!("OneCore unavailable ({e}), trying SAPI 5");
                            match Sapi5Engine::new() {
                                Ok(sapi) => Box::new(sapi),
                                Err(e) => {
                                    tracing::warn!("SAPI 5 unavailable ({e}), no speech");
                                    Box::new(NoSpeechEngine::new())
                                }
                            }
                        }
                    }
                }
                #[cfg(not(windows))]
                {
                    Box::new(NoSpeechEngine::new())
                }
            }
        };
        Ok(engine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_synthesizer_is_named_on_the_command_line() {
        assert_eq!(synthesizer_named("auto"), Ok(Synthesizer::Auto));
        assert_eq!(synthesizer_named("no-speech"), Ok(Synthesizer::NoSpeech));
        assert_eq!(synthesizer_named("silent"), Ok(Synthesizer::NoSpeech));
        assert_eq!(synthesizer_named("SAPI5"), Ok(Synthesizer::Sapi5));
        assert_eq!(synthesizer_named("one-core"), Ok(Synthesizer::OneCore));
        assert!(synthesizer_named("nonexistent").is_err());
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "live: speaks aloud through SAPI 5"]
    fn test_sapi5_engine_initialization_and_speech() {
        let _lock = crate::TEST_AUDIO_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let sapi = Sapi5Engine::new();
        assert!(sapi.is_ok(), "SAPI 5 should initialize on Windows");
        if let Ok(mut sapi) = sapi {
            assert_eq!(sapi.name(), "SAPI 5");

            // Speak a test sentence (asynchronous non-blocking)
            assert!(sapi
                .speak("Screen reader SAPI 5 test speech output.", true)
                .is_ok());

            // Immediate stop to test purging
            assert!(sapi.stop().is_ok());

            // Volume and rate
            assert!(sapi.set_rate(0).is_ok());
            assert!(sapi.set_volume(80).is_ok());
            assert_eq!(sapi.get_volume(), 80);
        }
    }
}
