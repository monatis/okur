# Reference: TTS Synthesizers & Speech Output Pipeline

This reference document outlines the Text-To-Speech (TTS) architecture, synthesizer drivers, configuration commands, and audio output pipelines implemented in the screen reader engine.

---

## 1. Synthesizer Driver Architecture

The TTS subsystem is designed with a cross-platform driver abstraction ([`TtsEngine`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-platform/src/tts/mod.rs)), allowing transparent switching between operating system native voices, synthetic audio pipelines, and silent testing drivers.

```
                      +-------------------+
                      |   VirtualCursor   |
                      | (Reading Engine)  |
                      +---------+---------+
                                |
                                v
                      +-------------------+
                      |  SpeechFormatter  |
                      |   (JAWS Tokens)   |
                      +---------+---------+
                                |
                                v
                      +-------------------+
                      |    TtsManager     |
                      +---------+---------+
                                |
        +-----------------------+-----------------------+
        |                                               |
        v                                               v
+----------------+                             +----------------+
|  Sapi5Engine   |                             | NoSpeechEngine |
| (Windows COM)  |                             | (Silent Queue) |
+----------------+                             +----------------+
```

### Supported Drivers

| Driver Name | Identifier | Supported Platforms | Description |
|---|---|---|---|
| **OneCore** | `onecore` | Windows 10, 11 | The Windows OneCore voices (`Windows.Media.SpeechSynthesis`, NVDA's default). Each utterance is synthesized to PCM in memory, its leading silence is trimmed, and it plays through our own WASAPI stream, so a stop drops audio at once. Measured key-to-sound latency on old-pc: 106–155 ms, against 325–397 ms for SAPI 5 (`sr-cli speak --measure-latency`). |
| **SAPI 5** | `sapi5` | Windows | Native Microsoft Speech API 5 using COM `ISpVoice` (`CLSID_SpVoice`). Asynchronous audio rendering via `SPF_ASYNC`, instant interruption via `SPF_PURGEBEFORESPEAK`. |
| **No Speech** | `no-speech` | Windows, macOS, Linux, CI | Universal silent synthesizer matching Freedom Scientific JAWS "No Speech" and NVDA "Silence" driver. Stores utterances in an inspectable memory buffer for test validation, braille display synchronization, and noise-free CI pipelines. |
| **Auto** | `auto` | All | On Windows: OneCore, else SAPI 5, else `no-speech`. Elsewhere: `no-speech` until native drivers exist. |

---

## 2. Synthesizer Control API (`TtsEngine`)

The [`TtsEngine`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-platform/src/tts/mod.rs) trait exposes core screen reader synthesizer primitives:

```rust
pub trait TtsEngine: Send + Sync {
    fn name(&self) -> &str;
    fn speak(&mut self, text: &str, interrupt: bool) -> Result<(), TtsError>;
    fn stop(&mut self) -> Result<(), TtsError>;
    fn pause(&mut self) -> Result<(), TtsError>;
    fn resume(&mut self) -> Result<(), TtsError>;
    fn set_rate(&mut self, rate: i32) -> Result<(), TtsError>;
    fn get_rate(&self) -> i32;
    fn set_volume(&mut self, volume: u16) -> Result<(), TtsError>;
    fn get_volume(&self) -> u16;
    fn is_speaking(&self) -> bool;
    fn get_voices(&self) -> Vec<VoiceInfo>;
    fn set_voice(&mut self, voice_id: &str) -> Result<(), TtsError>;
    fn wait_until_done(&mut self, timeout_ms: u32) -> Result<(), TtsError>;
}
```

### Rate and Volume Ranges
- **Rate**: Clamped to `-10` (slowest) through `10` (fastest), where `0` represents normal speaking speed.
- **Volume**: Clamped to `0` (muted) through `100` (full volume).

---

## 3. CLI Audio Commands (`sr-cli`)

The screen reader CLI provides built-in commands for testing speech output:

### Speak Utterance
```bash
# Speak text using the default platform synthesizer (SAPI 5 on Windows)
cargo run --bin sr-cli -- speak "Hello, screen reader speech is active!"

# Speak text using SAPI 5 with custom rate and volume
cargo run --bin sr-cli -- speak "Faster voice" --engine sapi5 --rate 3 --volume 90

# Speak silently via No Speech engine (records in memory buffer)
cargo run --bin sr-cli -- speak "Silent test" --engine no-speech
```

### Read Document Fixture
```bash
# Read an entire HTML fixture aloud using virtual cursor SayAll
cargo run --bin sr-cli -- read corpus/fixtures/tables-navigation.html --engine sapi5

# Read fixture line-by-line
cargo run --bin sr-cli -- read corpus/fixtures/forms-interaction.html --engine sapi5 --mode line
```

### Enumerate Installed Voices
```bash
cargo run --bin sr-cli -- voices --engine sapi5
cargo run --bin sr-cli -- voices --engine no-speech
```

### Muted Test Execution
```bash
# Differential tests run silently by default using the No Speech driver
cargo run --bin sr-cli -- test --scenarios 01..25 --tts no-speech

# Optionally enable live SAPI 5 speech during differential test runs
cargo run --bin sr-cli -- test --scenarios 21..25 --tts sapi5
```
