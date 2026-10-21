//! Full Speech and less speech (JAWS 2025 help, Speech On Demand). Less speech is on demand
//! or muted, as the settings choose (`SpeechMode`, `LessSpeechMode`). On demand, only the
//! reading commands of the key map's [Speech On Demand] list speak
//! ([`crate::command::Command::speaks_on_demand`]): Down, H and Tab say nothing, while Say
//! Line, Say Window Title and Say Color speak [obs, 700/s1-s10]. What the silent ones say is
//! still written to the speech history ("banner" for Down [obs, 700/s1]), so the synthesizer
//! is told what not to speak ([`crate::speech::SpeechOutput::set_unspoken`]) and the rest of
//! Okur speaks as always.
//!
//! The speech layer's S (MuteSynthesizer) and Shift+S (SpeechOnDemandToggle) change the mode
//! for the session. From full speech S mutes the synthesizer, history and all, and S again
//! brings it back, both silently [obs, 517/s6-s10]; started in less speech, S says "Full
//! Speech" and goes to full speech, and S again "Speech On Demand" [obs, 700/s12, s15].
//! Shift+S in full speech says "Not available in Full Speech mode" [obs, 566/s3, s10].

use super::ScreenReaderEngine;
use crate::messages::msg;
use crate::settings::{LessSpeech, SpeechMode};

impl ScreenReaderEngine {
    /// The speech mode in force: the session's, else the settings'.
    pub(super) fn speech_mode(&self) -> SpeechMode {
        self.speech_mode_session
            .unwrap_or(self.settings.speech_mode)
    }

    /// Less speech's kind in force.
    fn less_speech(&self) -> LessSpeech {
        self.less_speech_session
            .unwrap_or(self.settings.less_speech)
    }

    /// Speech On Demand is on.
    pub(super) fn on_demand(&self) -> bool {
        self.speech_mode() == SpeechMode::Less && self.less_speech() == LessSpeech::OnDemand
    }

    /// Tells the synthesizer what it speaks now: on demand, nothing until a command of the list
    /// (or the Say All it started) speaks; muted less speech, nothing at all.
    pub(super) fn sync_speech_mode(&mut self) {
        let muted = self.speech_muted
            || (self.speech_mode() == SpeechMode::Less && self.less_speech() == LessSpeech::Muted);
        self.speech.set_muted(muted);
        self.speech
            .set_unspoken(self.on_demand() && self.say_all.is_none());
    }

    /// Speaks one of the mode's own messages, which speak whatever the mode.
    fn say_mode(&mut self, said: &str) {
        self.speech.set_unspoken(false);
        self.speech.speak(said, true);
    }

    /// Insert+Space, S (MuteSynthesizer).
    pub(super) fn mute_synthesizer(&mut self) {
        if self.speech_muted {
            self.speech_muted = false;
        } else if self.speech_mode() == SpeechMode::Less {
            self.speech_mode_session = Some(SpeechMode::Full);
            self.sync_speech_mode();
            self.say_mode(msg::speech_mode_full().as_str());
        } else if self.settings.speech_mode == SpeechMode::Less {
            self.speech_mode_session = Some(SpeechMode::Less);
            if self.less_speech() == LessSpeech::OnDemand {
                self.say_mode(msg::speech_mode_on_demand().as_str());
            }
        } else {
            self.speech_muted = true;
        }
        self.sync_speech_mode();
    }

    /// Insert+Space, Shift+S (SpeechOnDemandToggle): less speech on demand or muted; in full
    /// speech, not available.
    pub(super) fn speech_on_demand_toggle(&mut self) {
        if self.speech_mode() == SpeechMode::Full {
            self.say_mode(msg::speech_on_demand_unavailable().as_str());
            self.sync_speech_mode();
            return;
        }
        let next = match self.less_speech() {
            LessSpeech::OnDemand => LessSpeech::Muted,
            LessSpeech::Muted => LessSpeech::OnDemand,
        };
        self.less_speech_session = Some(next);
        self.sync_speech_mode();
        if next == LessSpeech::OnDemand {
            self.say_mode(msg::speech_mode_on_demand().as_str());
            self.sync_speech_mode();
        }
    }
}
