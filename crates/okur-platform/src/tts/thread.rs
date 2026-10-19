//! Speech on a thread of its own. The engine stops and starts speech while it handles a key,
//! inside the low-level keyboard hook, and a synthesizer can take long to stop: SAPI's purge
//! took 750 ms on old-pc (2026-10-18). That held the hook past Windows' limit, and the key went
//! on to the application (Space on an edit toggled the check box that had the focus). Here the
//! engine's speech calls are queued, in order, to a thread that talks to the synthesizer. The
//! speech log is still written when a call is made, so key-to-speech timings keep their meaning.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use okur_core::speech::SpeechOutput;

use super::{SpeechLog, SpeechLogKind, TtsManager};

enum Request {
    Speak(String, bool),
    /// Text with every symbol named: a selection, a word read on its own.
    Symbols(String, bool),
    Character(String, bool),
    /// A character spelled phonetically (Say Character twice).
    Phonetic(String, bool),
    /// A text spelled (Say Word or Say Line twice).
    Spelled(String, bool),
    /// A part of a Say All, whose mark goes back to the engine as the synthesizer starts it.
    Marked(String, u64, bool),
    Stop,
    Configure(okur_core::settings::VoiceSettings),
}

/// How often the synthesizer is asked how far a Say All has got.
const MARK_POLL: Duration = Duration::from_millis(20);
/// Polls without progress, and without speech, after which a Say All is taken to be over.
const IDLE_POLLS: u32 = 25;

/// Where a Say All part's mark goes once the synthesizer has started it.
pub type MarkReport = Box<dyn Fn(u64) + Send>;

/// The longest part of a text given to the synthesizer at once, in bytes.
const PART: usize = 2000;

/// `text` in parts of whole lines of about [`PART`] bytes (a longer line is a part of its own):
/// Say All from the top of a page was one utterance of 430 KB, pronounced whole (24 ms
/// optimized, 565 ms in a debug build) before the first word (responsiveness review,
/// 2026-10-19).
fn parts(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let mut end = 0;
        for (i, _) in rest.match_indices('\n') {
            if i > PART && end > 0 {
                break;
            }
            end = i + 1;
        }
        if end == 0 || rest.len() <= PART {
            end = rest.len();
        }
        let (part, after) = rest.split_at(end);
        rest = after;
        Some(part)
    })
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
            | Request::Phonetic(_, interrupt)
            | Request::Spelled(_, interrupt)
            | Request::Marked(_, _, interrupt) => *interrupt,
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
    /// Where a Say All part's mark goes as the synthesizer starts it.
    report_to: Arc<Mutex<Option<MarkReport>>>,
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
        let report_to: Arc<Mutex<Option<MarkReport>>> = Arc::new(Mutex::new(None));
        let worker_report = Arc::clone(&report_to);
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
                // Say All parts whose start the synthesizer reports are out, and the polls
                // since it last made progress.
                let mut following = false;
                let mut idle = 0;
                loop {
                    let next = if following {
                        match received.recv_timeout(MARK_POLL) {
                            Ok(request) => Some(request),
                            Err(RecvTimeoutError::Timeout) => None,
                            Err(RecvTimeoutError::Disconnected) => break,
                        }
                    } else {
                        match received.recv() {
                            Ok(request) => Some(request),
                            Err(_) => break,
                        }
                    };
                    if let Some(first) = next {
                        carry_out(
                            Request::batch(first, &received),
                            &worker_tts,
                            &worker_pending,
                            &worker_voices,
                            &worker_report,
                            &mut following,
                        );
                        idle = 0;
                    }
                    if following {
                        let Ok(mut tts) = worker_tts.lock() else {
                            continue;
                        };
                        match tts.engine.reached() {
                            Some(mark) => {
                                idle = 0;
                                report(&worker_report, mark);
                            }
                            None if tts.engine.is_speaking() => idle = 0,
                            None => {
                                idle += 1;
                                following = idle < IDLE_POLLS;
                            }
                        }
                    }
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
            report_to,
        }
    }

    /// Sends each Say All part's mark to `report` once the synthesizer has started the part.
    /// Until this is called the engine hands a Say All over whole.
    pub fn report_marks(&self, report: MarkReport) {
        if let Ok(mut to) = self.report_to.lock() {
            *to = Some(report);
        }
    }
}

/// Carries out `batch` on the synthesizer, but for what an interruption in it supersedes.
/// `following`: Say All parts whose start the synthesizer reports are out.
fn carry_out(
    batch: Vec<Request>,
    worker_tts: &Mutex<TtsManager>,
    worker_pending: &AtomicUsize,
    worker_voices: &Mutex<Vec<String>>,
    worker_report: &Mutex<Option<MarkReport>>,
    following: &mut bool,
) {
    let count = batch.len();
    for request in Request::superseding(batch) {
        let Ok(mut tts) = worker_tts.lock() else {
            continue;
        };
        // Speech that interrupts ends the Say All before it.
        if request.interrupts() {
            *following = false;
        }
        match request {
            Request::Marked(text, mark, interrupt) => {
                let spoken = okur_core::pronounce::spoken(&text, tts.punctuation);
                let _ = tts.engine.speak_marked(&spoken, mark, interrupt);
                if tts.engine.follows_marks() {
                    *following = true;
                } else {
                    report(worker_report, mark);
                }
            }
            Request::Speak(text, interrupt) => {
                // Long text (Say All) in parts, so that speech starts after the
                // first part's pronunciation, not the whole text's.
                for (i, part) in parts(&text).enumerate() {
                    let spoken = okur_core::pronounce::spoken(part, tts.punctuation);
                    let _ = tts.engine.speak(&spoken, interrupt && i == 0);
                }
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
            Request::Spelled(text, interrupt) => {
                let parts = okur_core::pronounce::spelled_parts(&text);
                for (i, (part, pitch)) in parts.into_iter().enumerate() {
                    let first = interrupt && i == 0;
                    let _ = tts.engine.speak_pitched(&part, pitch, first);
                }
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

/// Hands `mark` to the engine's side.
fn report(to: &Mutex<Option<MarkReport>>, mark: u64) {
    if let Ok(report) = to.lock() {
        if let Some(report) = report.as_ref() {
            report(mark);
        }
    }
}

impl SpeechThread {
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
    /// Typing echo: kept out of the speech log, as JAWS keeps it out of its history; a capital
    /// at a raised pitch [obs, 406/s13].
    fn speak_echo(&mut self, text: &str) {
        self.send(Request::Character(text.to_owned(), true));
    }

    fn speak(&mut self, text: &str, interrupt: bool) {
        self.record(SpeechLogKind::Speak, text, interrupt);
        self.send(Request::Speak(text.to_owned(), interrupt));
    }

    fn speak_marked(&mut self, text: &str, mark: u64, interrupt: bool) {
        self.record(SpeechLogKind::Speak, text, interrupt);
        self.send(Request::Marked(text.to_owned(), mark, interrupt));
    }

    fn reports_marks(&self) -> bool {
        self.report_to.lock().is_ok_and(|to| to.is_some())
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

    /// The speech log keeps the spelled characters; the synthesizer says them in parts.
    fn speak_spelled(&mut self, text: &str, interrupt: bool) {
        let spelled = okur_core::pronounce::spelled(text);
        self.record(SpeechLogKind::Speak, &spelled, interrupt);
        self.send(Request::Spelled(text.to_owned(), interrupt));
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

    /// Long text is given in parts of whole lines that make the text again; short text whole.
    #[test]
    fn long_text_is_spoken_in_parts_of_whole_lines() {
        let line = "A line of a page read with Say All, long enough to count.\n";
        let text = line.repeat(200);
        let split: Vec<&str> = parts(&text).collect();
        assert!(split.len() > 1);
        assert_eq!(split.concat(), text);
        assert!(split
            .iter()
            .all(|p| p.ends_with('\n') && p.len() <= PART + line.len()));
        let long_line = "x".repeat(PART * 2);
        let with_long = format!("one\n{long_line}\ntwo");
        let split: Vec<&str> = parts(&with_long).collect();
        assert_eq!(split.concat(), with_long);
        assert_eq!(
            parts("Hello, world.").collect::<Vec<_>>(),
            ["Hello, world."]
        );
    }
}

#[cfg(test)]
mod mark_tests {
    use super::*;
    use crate::tts::{TtsEngine, TtsEngineType, TtsError, VoiceInfo};
    use std::collections::VecDeque;
    use std::time::Duration;

    /// A synthesizer that starts one queued part each time it is asked how far it got.
    #[derive(Default)]
    struct Following {
        queued: VecDeque<u64>,
    }

    impl TtsEngine for Following {
        fn name(&self) -> &str {
            "following"
        }
        fn speak(&mut self, _text: &str, interrupt: bool) -> Result<(), TtsError> {
            if interrupt {
                self.queued.clear();
            }
            Ok(())
        }
        fn speak_marked(
            &mut self,
            _text: &str,
            mark: u64,
            interrupt: bool,
        ) -> Result<(), TtsError> {
            if interrupt {
                self.queued.clear();
            }
            self.queued.push_back(mark);
            Ok(())
        }
        fn reached(&mut self) -> Option<u64> {
            self.queued.pop_front()
        }
        fn follows_marks(&self) -> bool {
            true
        }
        fn stop(&mut self) -> Result<(), TtsError> {
            self.queued.clear();
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
            !self.queued.is_empty()
        }
        fn get_voices(&self) -> Vec<VoiceInfo> {
            Vec::new()
        }
        fn set_voice(&mut self, _voice_id: &str) -> Result<(), TtsError> {
            Ok(())
        }
        fn wait_until_done(&mut self, _timeout_ms: u32) -> Result<(), TtsError> {
            Ok(())
        }
    }

    fn thread(
        engine: Option<Box<dyn TtsEngine>>,
    ) -> Option<(SpeechThread, std::sync::mpsc::Receiver<u64>)> {
        let mut manager = TtsManager::new(TtsEngineType::NoSpeech).ok()?;
        if let Some(engine) = engine {
            manager.engine = engine;
        }
        let speech = SpeechThread::start(manager);
        let (tx, rx) = channel();
        speech.report_marks(Box::new(move |mark| {
            let _ = tx.send(mark);
        }));
        Some((speech, rx))
    }

    fn received(rx: &std::sync::mpsc::Receiver<u64>, count: usize) -> Vec<u64> {
        (0..count)
            .map_while(|_| rx.recv_timeout(Duration::from_secs(5)).ok())
            .collect()
    }

    /// A synthesizer that follows its parts gives each mark back as it starts the part.
    #[test]
    fn marks_come_back_as_parts_are_started() {
        let Some((mut speech, rx)) = thread(Some(Box::<Following>::default())) else {
            return;
        };
        assert!(speech.reports_marks());
        speech.speak_marked("Line 1", 7, true);
        speech.speak_marked("Line 2", 8, false);
        speech.speak_marked("Line 3", 9, false);
        assert_eq!(received(&rx, 3), [7, 8, 9]);
    }

    /// Without a synthesizer that follows its parts, a part counts as reached when it is
    /// handed over, so a Say All still reads on.
    #[test]
    fn marks_come_back_at_once_without_progress() {
        let Some((mut speech, rx)) = thread(None) else {
            return;
        };
        speech.speak_marked("Line 1", 1, true);
        speech.speak_marked("Line 2", 2, false);
        assert_eq!(received(&rx, 2), [1, 2]);
    }

    /// No mark comes back before a synthesizer reports progress at all.
    #[test]
    fn marks_are_not_reported_without_a_listener() {
        let Ok(manager) = TtsManager::new(TtsEngineType::NoSpeech) else {
            return;
        };
        let speech = SpeechThread::start(manager);
        assert!(!speech.reports_marks());
    }
}
