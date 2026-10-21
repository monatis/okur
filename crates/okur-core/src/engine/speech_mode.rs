//! Full Speech and less speech (JAWS 2025 help, Speech On Demand). Less speech is on demand
//! or muted, as the settings choose (`SpeechMode`, `LessSpeechMode`). On demand, only the
//! reading commands of the key map's [Speech On Demand] list speak
//! ([`crate::command::Command::speaks_on_demand`]): Down, H and Tab say nothing, while Say
//! Line, Say Window Title and Say Color speak [obs, 700/s1-s10]. Muted, nothing speaks, Say
//! Line neither [obs, 707/s4]. Either way what is said is still written to the speech history
//! ("banner" for a silent Down [obs, 700/s1, 707/s5]), so the synthesizer is told what not to
//! speak ([`crate::speech::SpeechOutput::set_unspoken`]) and the rest of Okur speaks as always.
//!
//! The speech layer's S (MuteSynthesizer) goes from full speech to less speech and back:
//! "Speech On Demand", then "Full Speech" [obs, 706/s3, s7; 700/s12, s15]. Shift+S
//! (SpeechOnDemandToggle) turns less speech from on demand to muted and back, "Speech off set
//! to speech mute", "speech off set to speech on demand" [obs, 707/s3, s7]; in full speech it
//! is "Not available in Full Speech mode" [obs, 566/s3, 707 trial 2].

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

    /// Less speech muted is on: nothing speaks.
    fn less_speech_muted(&self) -> bool {
        self.speech_mode() == SpeechMode::Less && self.less_speech() == LessSpeech::Muted
    }

    /// Tells the synthesizer what it speaks now: on demand, nothing until a command of the list
    /// (or the Say All it started) speaks; muted, nothing.
    pub(super) fn sync_speech_mode(&mut self) {
        let unspoken = self.less_speech_muted() || (self.on_demand() && self.say_all.is_none());
        self.speech.set_unspoken(unspoken);
    }

    /// Speaks one of the mode's own messages, which speak whatever the mode.
    fn say_mode(&mut self, said: &str) {
        self.speech.set_unspoken(false);
        self.speech.speak(said, true);
        self.sync_speech_mode();
    }

    /// Insert+Space, S (MuteSynthesizer): full speech or less speech.
    pub(super) fn mute_synthesizer(&mut self) {
        if self.speech_mode() == SpeechMode::Less {
            self.speech_mode_session = Some(SpeechMode::Full);
            self.say_mode(msg::speech_mode_full().as_str());
            return;
        }
        self.speech_mode_session = Some(SpeechMode::Less);
        match self.less_speech() {
            LessSpeech::OnDemand => self.say_mode(msg::speech_mode_on_demand().as_str()),
            LessSpeech::Muted => self.sync_speech_mode(),
        }
    }

    /// Insert+Space, Shift+S (SpeechOnDemandToggle): less speech on demand or muted; in full
    /// speech, not available.
    pub(super) fn speech_on_demand_toggle(&mut self) {
        if self.speech_mode() == SpeechMode::Full {
            self.say_mode(msg::speech_on_demand_unavailable().as_str());
            return;
        }
        let (next, said) = match self.less_speech() {
            LessSpeech::OnDemand => (LessSpeech::Muted, msg::less_speech_set_muted()),
            LessSpeech::Muted => (LessSpeech::OnDemand, msg::less_speech_set_on_demand()),
        };
        self.less_speech_session = Some(next);
        self.say_mode(said.as_str());
    }
}
