//! Speech on a thread of its own. The engine stops and starts speech while it handles a key,
//! inside the low-level keyboard hook, and a synthesizer can take long to stop: SAPI's purge
//! took 750 ms on old-pc (2026-10-18). That held the hook past Windows' limit, and the key went
//! on to the application (Space on an edit toggled the check box that had the focus). Here the
//! engine's speech calls are queued, in order, to a thread that talks to the synthesizer. The
//! speech log is still written when a call is made, so key-to-speech timings keep their meaning.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use okur_core::speech::SpeechOutput;

use super::{SpeechLog, SpeechLogKind, TtsManager};

enum Request {
    Speak(String, bool),
    /// Text with every symbol named: a selection, a word read on its own.
    Symbols(String, bool),
    Character(String, bool),
    /// A character spelled phonetically (Say Character twice).
    Phonetic(String, bool),
    Stop,
    Configure(okur_core::settings::VoiceSettings),
}

/// How long a stop waits for the speech that follows it: a key stops speech, then its command
/// speaks with an interruption, which stops speech too. Taken together, the synthesizer
/// purges once (SAPI's purge took 750 ms on old-pc).
const STOP_GRACE: std::time::Duration = std::time::Duration::from_millis(3);

impl Request {
    /// Whether the request cuts off all speech before it.
    fn interrupts(&self) -> bool {
        match self {
            Request::Stop => true,
            Request::Speak(_, interrupt)
            | Request::Symbols(_, interrupt)
            | Request::Character(_, interrupt)
            | Request::Phonetic(_, interrupt) => *interrupt,
            Request::Configure(_) => false,
        }
    }

    /// `first` and every request queued behind it; a stop alone waits briefly for the speech
    /// that usually follows it.
    fn batch(first: Request, queue: &std::sync::mpsc::Receiver<Request>) -> Vec<Request> {
        let mut batch = vec![first];
        batch.extend(queue.try_iter());
        if matches!(batch.last(), Some(Request::Stop)) {
            if let Ok(next) = queue.recv_timeout(STOP_GRACE) {
                batch.push(next);
                batch.extend(queue.try_iter());
            }
        }
        batch
    }

    /// The requests of `batch` worth carrying out: from the last one that cuts off speech on
    /// (what came before it would be cut off at once), with the voice changes before it, in
    /// order. A held arrow key queued speech faster than SAPI could stop it, and the speech
    /// lagged further and further behind the keys (responsiveness review, 2026-10-19).
    fn superseding(batch: Vec<Request>) -> impl Iterator<Item = Request> {
        let cut = batch.iter().rposition(Request::interrupts).unwrap_or(0);
        batch
            .into_iter()
            .enumerate()
            .filter(move |(i, request)| *i >= cut || matches!(request, Request::Configure(_)))
            .map(|(_, request)| request)
    }
}

/// A [`TtsManager`] driven from its own thread.
pub struct SpeechThread {
    requests: Sender<Request>,
    tts: Arc<Mutex<TtsManager>>,
    /// Requests sent and not yet carried out: speech asked for is speech going on.
    pending: Arc<AtomicUsize>,
    log: Option<SpeechLog>,
    /// The synthesizer's voices, read when it starts: the settings dialog asks for them in
    /// the keyboard hook, and reading them there waited behind the synthesizer (a SAPI purge)
    /// and then enumerated SAPI's voices (responsiveness review, 2026-10-19).
    voices: Arc<Mutex<Vec<String>>>,
}

impl SpeechThread {
    /// Moves `tts` to a new thread; its speech log stays with the caller.
    pub fn start(mut tts: TtsManager) -> Self {
        let log = tts.speech_log.take();
        let voices = Arc::new(Mutex::new(SpeechOutput::voices(&tts)));
        let tts = Arc::new(Mutex::new(tts));
        let pending = Arc::new(AtomicUsize::new(0));
        let (requests, received) = channel::<Request>();
        let (worker_tts, worker_pending) = (Arc::clone(&tts), Arc::clone(&pending));
        let worker_voices = Arc::clone(&voices);
        let started = std::thread::Builder::new()
            .name("okur-speech".to_owned())
            .spawn(move || {
                #[cfg(windows)]
                // SAFETY: COM initialization for this thread, which calls the synthesizer.
                let _ = unsafe {
                    windows::Win32::System::Com::CoInitializeEx(
                        None,
                        windows::Win32::System::Com::COINIT_MULTITHREADED,
                    )
                };
                while let Ok(first) = received.recv() {
                    let batch = Request::batch(first, &received);
                    let count = batch.len();
                    for request in Request::superseding(batch) {
                        let Ok(mut tts) = worker_tts.lock() else {
                            continue;
                        };
                        match request {
                            Request::Speak(text, interrupt) => {
                                let spoken = okur_core::pronounce::spoken(&text, tts.punctuation);
                                let _ = tts.engine.speak(&spoken, interrupt);
                            }
                            Request::Symbols(text, interrupt) => {
                                let spoken = okur_core::pronounce::spoken_symbols(&text);
                                let _ = tts.engine.speak(&spoken, interrupt);
                            }
                            Request::Character(text, interrupt) => {
                                let _ = tts.engine.speak_pitched(
                                    &okur_core::pronounce::character(&text),
                                    okur_core::pronounce::character_pitch(&text),
                                    interrupt,
                                );
                            }
                            Request::Phonetic(text, interrupt) => {
                                let _ = tts.engine.speak_pitched(
                                    &okur_core::pronounce::phonetic(&text),
                                    okur_core::pronounce::character_pitch(&text),
                                    interrupt,
                                );
                            }
                            Request::Stop => {
                                let _ = tts.engine.stop();
                            }
                            Request::Configure(voice) => {
                                let kind = tts.kind;
                                tts.configure(&voice);
                                // Another synthesizer has other voices.
                                if tts.kind != kind {
                                    if let Ok(mut known) = worker_voices.lock() {
                                        *known = SpeechOutput::voices(&*tts);
                                    }
                                }
                            }
                        }
                    }
                    worker_pending.fetch_sub(count, Ordering::SeqCst);
                }
            });
        if let Err(e) = started {
            tracing::error!(error = %e, "the speech thread did not start: no speech");
        }
        Self {
            requests,
            tts,
            pending,
            log,
            voices,
        }
    }

    fn send(&self, request: Request) {
        self.pending.fetch_add(1, Ordering::SeqCst);
        if self.requests.send(request).is_err() {
            self.pending.fetch_sub(1, Ordering::SeqCst);
        }
    }

    fn record(&mut self, kind: SpeechLogKind, text: &str, interrupt: bool) {
        if let Some(log) = self.log.as_mut() {
            log.record(kind, text, interrupt);
        }
    }
}

impl SpeechOutput for SpeechThread {
    fn speak(&mut self, text: &str, interrupt: bool) {
        self.record(SpeechLogKind::Speak, text, interrupt);
        self.send(Request::Speak(text.to_owned(), interrupt));
    }

    fn speak_symbols(&mut self, text: &str, interrupt: bool) {
        self.record(SpeechLogKind::Speak, text, interrupt);
        self.send(Request::Symbols(text.to_owned(), interrupt));
    }

    /// The speech log keeps the character as JAWS's history would; the synthesizer says its
    /// name.
    fn speak_character(&mut self, text: &str, interrupt: bool) {
        self.record(SpeechLogKind::Speak, text, interrupt);
        self.send(Request::Character(text.to_owned(), interrupt));
    }

    /// The speech log keeps the character; the synthesizer says its phonetic word.
    fn speak_phonetic(&mut self, text: &str, interrupt: bool) {
        self.record(SpeechLogKind::Speak, text, interrupt);
        self.send(Request::Phonetic(text.to_owned(), interrupt));
    }

    fn stop(&mut self) {
        self.record(SpeechLogKind::Stop, "", true);
        self.send(Request::Stop);
    }

    /// Queued speech counts, and so does a synthesizer busy carrying out a request.
    fn is_speaking(&self) -> bool {
        self.pending.load(Ordering::SeqCst) > 0
            || self.tts.try_lock().map_or(true, |tts| tts.is_speaking())
    }

    fn voices(&self) -> Vec<String> {
        self.voices
            .lock()
            .map(|known| known.clone())
            .unwrap_or_default()
    }

    fn configure(&mut self, voice: &okur_core::settings::VoiceSettings) {
        self.send(Request::Configure(voice.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tts::TtsEngineType;

    /// Calls are logged as they are made and carried out on the speech thread, in order.
    #[test]
    fn speech_is_logged_at_once_and_carried_out_in_order() {
        let path =
            std::env::temp_dir().join(format!("okur-speech-thread-{}.jsonl", std::process::id()));
        let log = SpeechLog::open(&path);
        assert!(log.is_ok(), "{:?}", log.as_ref().err());
        let manager = TtsManager::new(TtsEngineType::NoSpeech);
        assert!(manager.is_ok());
        let (Ok(log), Ok(manager)) = (log, manager) else {
            return;
        };
        let mut speech = SpeechThread::start(manager.with_speech_log(log));
        speech.stop();
        speech.speak("Count Button", true);
        let entries = SpeechLog::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        let said: Vec<(SpeechLogKind, &str)> =
            entries.iter().map(|e| (e.kind, e.text.as_str())).collect();
        assert_eq!(
            said,
            [
                (SpeechLogKind::Stop, ""),
                (SpeechLogKind::Speak, "Count Button")
            ]
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while speech.pending.load(Ordering::SeqCst) > 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(speech.pending.load(Ordering::SeqCst), 0);
    }

    /// Of the requests queued together, those a later interruption would cut off at once are
    /// left out; voice changes are kept, in order.
    #[test]
    fn requests_an_interruption_supersedes_are_left_out() {
        let name = |request: Request| match request {
            Request::Speak(text, _) => text,
            Request::Configure(_) => "configure".to_owned(),
            Request::Stop => "stop".to_owned(),
            _ => "other".to_owned(),
        };
        let batch = vec![
            Request::Speak("Line one".into(), true),
            Request::Configure(okur_core::settings::VoiceSettings::default()),
            Request::Speak("queued".into(), false),
            Request::Stop,
            Request::Speak("Line two".into(), true),
            Request::Speak("its hint".into(), false),
        ];
        let kept: Vec<String> = Request::superseding(batch).map(name).collect();
        assert_eq!(kept, ["configure", "Line two", "its hint"]);
        let calm = vec![
            Request::Speak("one".into(), false),
            Request::Speak("two".into(), false),
        ];
        let kept: Vec<String> = Request::superseding(calm).map(name).collect();
        assert_eq!(kept, ["one", "two"]);
    }
}
