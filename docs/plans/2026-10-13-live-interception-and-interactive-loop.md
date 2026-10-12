# Development Plan: Live System Interception & Interactive Screen Reader Loop

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 6 – Live System Interception & Interactive Screen Reader Loop  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-boundaries-and-dynamic-updates.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-boundaries-and-dynamic-updates.md)  
**Corpus Evidence**: [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/) (Scenarios `01` – `30` passing with 100% parity)

---

## 1. Goal & Scope

With virtual buffer navigation, JAWS reading parity across 30 scenarios, and synthetic speech output (Windows SAPI 5 + NoSpeech) complete, this milestone connects the components into a **live, usable screen reader daemon**:
1. **Low-Level Keyboard Interception (`sr-platform::hook`)**:
   - Install a low-level keyboard hook (`WH_KEYBOARD_LL` on Windows) to capture navigation keys (`DownArrow`, `UpArrow`, `LeftArrow`, `RightArrow`, `PageUp`, `PageDown`, `Home`, `End`, `Tab`, `Space`, `Enter`, `Esc`, quick keys `H`, `T`, `1`–`6`, `K`, `F`, `B`, `E`, `X`).
   - Cross-platform abstraction for keyboard event interception.
2. **Foreground Window & Focus Tracking**:
   - Track active application window and focus changes via `PlatformBridge`.
   - Acquire and linearize active document AOM into `VirtualBuffer`.
3. **Interactive Screen Reader Daemon (`sr-cli run`)**:
   - Background execution loop connecting keyboard hooks -> `VirtualCursor` -> `TtsManager` (`ISpVoice` SAPI 5 / NoSpeech).
   - Instant speech interruption on keystroke (`stop()` / `SPF_PURGEBEFORESPEAK`).
   - Mode toggling between Virtual PC Cursor and Forms Mode.

---

## 2. Technical Design

1. **Keyboard Hook Subsystem (`crates/sr-platform/src/hook/`)**:
   - Cross-platform trait `KeyboardHook` with `start(callback)` and `stop()`.
   - Windows implementation using `SetWindowsHookExW(WH_KEYBOARD_LL, ...)`.
   - Key translation from virtual key codes (`VK_DOWN`, `VK_UP`, etc.) to standard screen reader command strings (`{DOWN}`, `{TAB}`, `H`, etc.).
2. **Interactive Event Loop in `sr-core`**:
   - `ScreenReaderEngine` coordinating:
     - `VirtualCursor`
     - `TtsManager`
     - Input event receiver
3. **CLI Daemon Integration (`crates/sr-cli/src/main.rs`)**:
   - Add `run` subcommand:
     `sr-cli run [--engine sapi5|no-speech] [--voice <name>] [--rate <int>]`
   - Real-time logging of keystrokes, spoken utterances, and mode transitions.

---

## 3. Implementation Steps

1. **Implement `KeyboardHook` in `sr-platform`**:
   - Add Win32 low-level hook implementation behind `#[cfg(target_os = "windows")]`.
2. **Implement `ScreenReaderEngine` in `sr-core`**:
   - Encapsulate cursor state, active document, TTS manager, and key event dispatcher.
3. **Add `sr-cli run` Subcommand in `sr-cli`**:
   - Wire arguments and signal handling (`Ctrl+C` clean exit).
4. **Verification**:
   - Validate live key interception and speech streaming.
   - Run full regression tests (`cargo test --all`, `cargo fmt --check`, `cargo clippy`).
