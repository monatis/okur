# JAWS Speech History & Output Capture Mechanics

## 1. Speech History Buffer Architecture
JAWS maintains an internal ring buffer of all spoken text strings dispatched to the software synthesizer. This buffer is accessible programmatically via two built-in JAWS scripts:

- **`ClearSpeechHistory`**: Clears the internal speech history buffer so subsequent captures contain only newly emitted speech.
- **`CopySpeechHistoryToClipboard`**: Transfers the entire buffered text history to the standard Windows clipboard (`CF_UNICODE_TEXT`).

### Configuration Preconditions
Speech History capture requires the `SpeechHistory` setting to be enabled in `Default.JCF`:
```ini
[Options]
SpeechHistory=1
```
On standard JAWS 2025 installations, this setting is enabled (`1`) by default.

---

## 2. Capture Workflow & Timing Pipeline

Every capture operation follows this deterministic pipeline:

```
[Pre-Action Settle] (100ms)
       ↓
[ClearSpeechHistory] (FSAPI RunScript)
       ↓
[Inject Sentinel to Clipboard] (e.g. __SENTINEL_GUID__)
       ↓
[Dispatch Action] (FSAPI RunScript OR Keyboard SendKeys)
       ↓
[Action Settle Delay] (300ms - 500ms)
       ↓
[CopySpeechHistoryToClipboard] (FSAPI RunScript)
       ↓
[Clipboard Transfer Settle] (150ms)
       ↓
[Read Clipboard with Retries] (up to 3 retries, 100ms interval)
       ↓
[Validate Non-Sentinel & Non-Empty Content]
       ↓
[Canonical JSON Output Record]
```

---

## 3. Verbatim Preservation
- **No Silent Normalization**: Raw transcripts must be preserved exactly as JAWS produced them, including all whitespace, control characters (e.g. `\u001d`, `\u001e`), and newlines.
- Differential testing and cross-screen-reader parity benchmarks depend on canonical character-for-character fidelity.

---

## 4. Speech History vs Audible Speech
- **Synthesizer Dispatch vs Acoustic Playback**: Speech History records text strings submitted to the speech queue. It does *not* prove that acoustic playback completed through the audio output device before another action interrupted it.
- **Interruption Testing**: Speech History reflects queued announcements. When testing speech interruption (e.g., pressing `Ctrl` to silence speech), verify with `StopSpeech` and audio measurement if hardware-level playback validation is required.

---

## 5. Timing & Settle Recommendations

| Phase | Duration | Rationale |
| :--- | :--- | :--- |
| Pre-Clear Sleep | 100 ms | Allows `ClearSpeechHistory` to flush internal buffers |
| Action Settle | 300 - 500 ms | Allows browser DOM events and JAWS virtual cursor positioning to settle |
| Clipboard Settle | 150 - 200 ms | Allows `CopySpeechHistoryToClipboard` to populate Windows clipboard |
| Clipboard Retry | 100 ms (x3) | Recovers from transient Win32 clipboard locking contention |
