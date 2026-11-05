//! Speech on Linux: speech-dispatcher, through its protocol (SSIP) on its Unix socket, as the
//! Linux lab's probe 4 heard it: espeak-ng's audio, and begin, end and index-mark notifications
//! for a marked text. The protocol's text and lines are `okur_linux_model::ssip`'s.
//!
//! One connection: commands are answered in order, and notifications come between the answers,
//! so a reader thread sorts the lines, answers to the command waiting, notifications into the
//! state ([`Heard`]) that `is_speaking` and `reached` read.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use okur_linux_model::ssip::{self, Notification};
use okur_runtime::synthesizer::{TtsEngine, TtsError, VoiceInfo};

/// How long a command waits for the server's answer.
const ANSWER: Duration = Duration::from_secs(2);

/// A reply: its code and its lines, the last included.
struct Reply {
    code: u16,
    lines: Vec<String>,
}

/// What the notifications say.
#[derive(Default)]
struct Heard {
    speaking: bool,
    /// Index marks reached since the last `reached`, in order.
    marks: Vec<u64>,
}

/// speech-dispatcher's socket: `SPEECHD_ADDRESS` ("unix_socket:<path>") when set, else its place
/// under the user's runtime directory.
fn socket_path() -> PathBuf {
    if let Some(path) = std::env::var("SPEECHD_ADDRESS")
        .ok()
        .and_then(|a| a.strip_prefix("unix_socket:").map(PathBuf::from))
    {
        return path;
    }
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_owned());
    PathBuf::from(runtime).join("speech-dispatcher/speechd.sock")
}

/// The server's socket, starting the server as its clients do (`--spawn`) when it is not up.
fn connect() -> Result<UnixStream, TtsError> {
    let path = socket_path();
    if let Ok(stream) = UnixStream::connect(&path) {
        return Ok(stream);
    }
    let _ = std::process::Command::new("speech-dispatcher")
        .arg("--spawn")
        .status();
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        match UnixStream::connect(&path) {
            Ok(stream) => return Ok(stream),
            Err(_) if Instant::now() < until => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => {
                return Err(TtsError::InitFailed(format!(
                    "speech-dispatcher at {}: {error}",
                    path.display()
                )))
            }
        }
    }
}

/// speech-dispatcher, through SSIP.
pub struct SsipEngine {
    stream: UnixStream,
    /// Behind a lock only because a synthesizer is `Sync`: one thread speaks.
    replies: Mutex<Receiver<Reply>>,
    heard: Arc<Mutex<Heard>>,
    rate: i32,
    volume: u16,
}

impl SsipEngine {
    pub fn new() -> Result<Self, TtsError> {
        let stream = connect()?;
        let reader = stream
            .try_clone()
            .map_err(|e| TtsError::InitFailed(format!("speech-dispatcher: {e}")))?;
        let (tx, replies) = mpsc::channel();
        let heard = Arc::new(Mutex::new(Heard::default()));
        {
            let heard = Arc::clone(&heard);
            std::thread::spawn(move || {
                let mut lines = Vec::new();
                for line in BufReader::new(reader).lines() {
                    let Ok(line) = line else {
                        return;
                    };
                    let Some((code, more, text)) = ssip::parse_line(&line) else {
                        continue;
                    };
                    lines.push(text.to_owned());
                    if more {
                        continue;
                    }
                    let done = std::mem::take(&mut lines);
                    if (700..800).contains(&code) {
                        if let (Some(n), Ok(mut heard)) =
                            (ssip::notification(code, &done), heard.lock())
                        {
                            match n {
                                Notification::Begin | Notification::Resume => heard.speaking = true,
                                Notification::End | Notification::Cancel | Notification::Pause => {
                                    heard.speaking = false
                                }
                                Notification::Mark(name) => {
                                    if let Ok(mark) = name.parse() {
                                        heard.marks.push(mark);
                                    }
                                }
                            }
                        }
                    } else if tx.send(Reply { code, lines: done }).is_err() {
                        return;
                    }
                }
            });
        }
        let mut engine = Self {
            stream,
            replies: Mutex::new(replies),
            heard,
            rate: 0,
            volume: 100,
        };
        engine.command("SET self CLIENT_NAME user:okur:main")?;
        engine.command("SET self SSML_MODE on")?;
        engine.command("SET self NOTIFICATION all on")?;
        engine.command("SET self PRIORITY message")?;
        Ok(engine)
    }

    /// Sends a command line and waits for its answer, an error for a code from 300 up.
    fn command(&mut self, line: &str) -> Result<Reply, TtsError> {
        self.stream
            .write_all(format!("{line}\r\n").as_bytes())
            .map_err(|e| TtsError::SynthesisFailed(format!("speech-dispatcher: {e}")))?;
        self.answer(line)
    }

    fn answer(&mut self, what: &str) -> Result<Reply, TtsError> {
        let replies = self
            .replies
            .lock()
            .map_err(|_| TtsError::SynthesisFailed("speech-dispatcher: the answers".to_owned()))?;
        let reply = replies.recv_timeout(ANSWER).map_err(|_| {
            TtsError::SynthesisFailed(format!("speech-dispatcher: no answer to {what}"))
        })?;
        if reply.code >= 300 {
            return Err(TtsError::SynthesisFailed(format!(
                "speech-dispatcher: {what}: {} {}",
                reply.code,
                reply.lines.last().map(String::as_str).unwrap_or("")
            )));
        }
        Ok(reply)
    }

    /// A message: SPEAK, then its text.
    fn message(&mut self, text: &str, mark: Option<u64>, interrupt: bool) -> Result<(), TtsError> {
        if interrupt {
            self.stop()?;
        }
        self.command("SPEAK")?;
        self.stream
            .write_all(ssip::speak_data(&ssip::ssml(text, mark)).as_bytes())
            .map_err(|e| TtsError::SynthesisFailed(format!("speech-dispatcher: {e}")))?;
        self.answer("the message")?;
        Ok(())
    }

    /// A message at a pitch of its own: the pitch is taken with the message, then set back.
    fn pitched(
        &mut self,
        text: &str,
        pitch: i32,
        mark: Option<u64>,
        interrupt: bool,
    ) -> Result<(), TtsError> {
        if pitch == 0 {
            return self.message(text, mark, interrupt);
        }
        self.command(&format!("SET self PITCH {}", ssip::ssip_scale(pitch)))?;
        let spoken = self.message(text, mark, interrupt);
        self.command("SET self PITCH 0")?;
        spoken
    }
}

impl TtsEngine for SsipEngine {
    fn name(&self) -> &str {
        "speech-dispatcher"
    }

    fn speak(&mut self, text: &str, interrupt: bool) -> Result<(), TtsError> {
        self.message(text, None, interrupt)
    }

    fn speak_pitched(&mut self, text: &str, pitch: i32, interrupt: bool) -> Result<(), TtsError> {
        self.pitched(text, pitch, None, interrupt)
    }

    fn speak_marked(
        &mut self,
        text: &str,
        mark: u64,
        pitch: i32,
        interrupt: bool,
    ) -> Result<(), TtsError> {
        self.pitched(text, pitch, Some(mark), interrupt)
    }

    fn reached(&mut self) -> Option<u64> {
        let mut heard = self.heard.lock().ok()?;
        let last = heard.marks.last().copied();
        heard.marks.clear();
        last
    }

    fn follows_marks(&self) -> bool {
        true
    }

    fn stop(&mut self) -> Result<(), TtsError> {
        self.command("CANCEL self")?;
        if let Ok(mut heard) = self.heard.lock() {
            heard.speaking = false;
        }
        Ok(())
    }

    fn pause(&mut self) -> Result<(), TtsError> {
        self.command("PAUSE self").map(|_| ())
    }

    fn resume(&mut self) -> Result<(), TtsError> {
        self.command("RESUME self").map(|_| ())
    }

    fn set_rate(&mut self, rate: i32) -> Result<(), TtsError> {
        self.command(&format!("SET self RATE {}", ssip::ssip_scale(rate)))?;
        self.rate = rate.clamp(-10, 10);
        Ok(())
    }

    fn get_rate(&self) -> i32 {
        self.rate
    }

    fn set_volume(&mut self, volume: u16) -> Result<(), TtsError> {
        self.command(&format!("SET self VOLUME {}", ssip::ssip_volume(volume)))?;
        self.volume = volume.min(100);
        Ok(())
    }

    fn get_volume(&self) -> u16 {
        self.volume
    }

    fn is_speaking(&self) -> bool {
        self.heard.lock().map(|h| h.speaking).unwrap_or(false)
    }

    fn get_voices(&self) -> Vec<VoiceInfo> {
        // `command` needs the stream mutably; a second connection lists the voices.
        let Ok(mut other) = SsipEngine::new() else {
            return Vec::new();
        };
        let Ok(reply) = other.command("LIST SYNTHESIS_VOICES") else {
            return Vec::new();
        };
        // Each line but the last: name, language, variant, by tabs.
        let count = reply.lines.len().saturating_sub(1);
        reply
            .lines
            .iter()
            .take(count)
            .filter_map(|line| {
                let mut parts = line.split('\t');
                let name = parts.next()?.to_owned();
                let language = parts.next().map(str::to_owned);
                Some(VoiceInfo {
                    id: name.clone(),
                    name,
                    language,
                })
            })
            .collect()
    }

    fn set_voice(&mut self, voice_id: &str) -> Result<(), TtsError> {
        self.command(&format!("SET self SYNTHESIS_VOICE {voice_id}"))
            .map(|_| ())
            .map_err(|_| TtsError::VoiceNotFound(voice_id.to_owned()))
    }

    fn wait_until_done(&mut self, timeout_ms: u32) -> Result<(), TtsError> {
        let until = Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        while self.is_speaking() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }
}

impl Drop for SsipEngine {
    fn drop(&mut self) {
        let _ = self.stream.write_all(b"QUIT\r\n");
    }
}
