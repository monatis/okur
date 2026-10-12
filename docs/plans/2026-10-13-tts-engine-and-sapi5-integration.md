# Development Plan: TTS Engine Subsystem & SAPI 5 / No-Speech Integration

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 5 (Prioritized) – TTS Engine Subsystem & Synthetic Speech Pipeline  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-table-navigation-and-grid-movement.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-table-navigation-and-grid-movement.md)  
**Deferred Milestone**: Boundaries & Dynamic Updates (deferred to Milestone 6 per user priority)  

---

## 1. Goal & Scope

Implement the core Text-To-Speech (TTS) synthesizer subsystem for the screen reader:
1. **Cross-Platform Synthesizer Trait (`TtsEngine`)**:
   - `speak(&mut self, text: &str, interrupt: bool)`: Asynchronous speech output with interruption support (`SPF_PURGEBEFORESPEAK`).
   - `stop(&mut self)`: Instant speech cancellation.
   - `pause(&mut self)` / `resume(&mut self)`: Speech flow control.
   - `set_rate(&mut self, rate: i32)` / `get_rate(&self)`: Speed control.
   - `set_volume(&mut self, volume: u16)` / `get_volume(&self)`: Volume control (0–100).
   - `get_voices(&self)` / `set_voice(&mut self, voice_id: &str)`: Voice selection.
2. **`NoSpeechEngine` ("No Speech" Driver)**:
   - Universal silent driver (matches JAWS "No Speech" and NVDA "Silence" driver).
   - Isolates automated differential tests from audio noise and speaker conflicts.
   - Captures speech utterances in an inspectable memory buffer for verification and future braille display synchronization.
3. **`Sapi5Engine` (Windows SAPI 5 Driver)**:
   - Genuine Windows SAPI 5 synthesizer driver using COM `ISpVoice` (`CLSID_SpVoice`).
   - Asynchronous speech delivery via `SPF_ASYNC`, instant interruption via `SPF_PURGEBEFORESPEAK`.
   - Voice enumeration and selection from installed Windows SAPI 5 voices.
4. **CLI & Interactive Integration (`sr-cli`)**:
   - `sr-cli speak "Hello world" --engine [sapi5|no-speech]`: Speaks arbitrary text or speech sequences.
   - `sr-cli read --fixture <path> [--engine sapi5|no-speech]`: Reads document fixtures aloud using the virtual cursor.
   - `sr-cli test --tts [no-speech|sapi5]`: Allows running test suite with audio speech enabled or muted.

---

## 2. Technical Design

### A. Module Layout in `crates/sr-platform/src/tts/`
- `crates/sr-platform/src/tts/mod.rs`:
  - `pub trait TtsEngine: Send + Sync`
  - `pub struct VoiceInfo { pub id: String, pub name: String, pub language: Option<String> }`
  - `pub enum TtsEngineType { Sapi5, NoSpeech, Auto }`
  - `pub struct TtsManager`
- `crates/sr-platform/src/tts/no_speech.rs`:
  - `pub struct NoSpeechEngine`: Silent driver with history tracking.
- `crates/sr-platform/src/tts/sapi5.rs`:
  - `#[cfg(windows)] pub struct Sapi5Engine`: COM `ISpVoice` driver.

### B. Windows SAPI 5 Architecture
- Initialized on COM MTA or STA thread via `CoCreateInstance(&CLSID_SpVoice, None, CLSCTX_ALL)`.
- Speech text is stripped of internal control codes (e.g. `\u{001d}`, `\u{001e}`) and converted to wide UTF-16.
- Calls `voice.Speak(PCWSTR, flags, None)`.

---

## 3. Implementation Steps

1. Create `crates/sr-platform/src/tts/` module hierarchy with `TtsEngine` trait and `VoiceInfo`.
2. Implement `NoSpeechEngine` with speech buffer recording and inspection.
3. Implement `Sapi5Engine` on Windows using `windows::Win32::Media::Speech` (`ISpVoice`).
4. Implement `TtsManager` with engine discovery and factory creation.
5. Extend `sr-cli` with `speak` command and `--engine` flags.
6. Verify audio delivery with SAPI 5 and silence verification with `NoSpeechEngine`.
7. Pass strict quality gates (`cargo fmt --check`, `cargo clippy`, `cargo test`).
8. Author reference guide `docs/references/tts-synthesizers.md` and handoff doc.
