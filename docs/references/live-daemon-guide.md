# User & Architecture Guide: Interactive Screen Reader Daemon

This document details the architecture and operational usage of the interactive screen reader daemon (`okur run`), covering low-level keyboard interception, speech synthesis coordination, and mode transitions.

---

## 1. Overview

The screen reader daemon bridges OS accessibility trees (`PlatformBridge`), low-level keyboard event interception (`KeyboardHook`), virtual buffer navigation (`VirtualCursor`), and synthetic speech (`TtsManager`) into an interactive, real-time accessibility loop.

```
+-----------------------------------------------------------+
|                   Operating System (OS)                   |
|   (Windows WH_KEYBOARD_LL / macOS CGEventTap / xa11y UIA)  |
+-----------------------------+-----------------------------+
                              | Low-Level Keystrokes
                              v
                  +-----------------------+
                  |     KeyboardHook      |
                  +-----------+-----------+
                              | KeyEvent
                              v
                +---------------------------+
                |    ScreenReaderEngine     |
                |  (Virtual PC Cursor vs.   |
                |        Forms Mode)        |
                +-------------+-------------+
                              |
               +--------------+--------------+
               |                             |
               v                             v
     +-------------------+         +-------------------+
     |   VirtualCursor   |         |    TtsManager     |
     |  (VirtualBuffer)  |         | (ISpVoice SAPI 5) |
     +-------------------+         +-------------------+
```

---

## 2. CLI Usage

Start the interactive screen reader daemon with:

```bash
# Start daemon with default auto-detected TTS engine (SAPI 5 on Windows, NoSpeech fallback)
cargo run --bin okur-lab -- run

# Start daemon with specific TTS engine, speech rate, and volume
cargo run --bin okur-lab -- run --engine sapi5 --rate 2 --volume 90

# Start daemon loaded with a specific test document fixture
cargo run --bin okur-lab -- run --fixture corpus/fixtures/boundaries-dynamic.html
```

Okur's settings are the ones its settings dialog saved (Insert+6 or Insert+V: "Okur
Settings"; OK and Apply save, Cancel and Escape drop what was not applied):
- synthesizer, voice, rate and volume;
- keyboard layout and Okur key (JAWS's JAWS key);
- typing echo, reading a growing live region's whole text, and showing the Virtual Viewer on
  screen.

They are kept in Okur's settings folder: `%APPDATA%\Okur` on Windows,
`~/Library/Application Support/Okur` on macOS, and `$XDG_CONFIG_HOME/okur` elsewhere. Its TOML
files hold only what differs from the layer below, as JAWS's own settings files do:
- `settings.toml`: for all applications (what the settings dialog saves);
- `applications/<application>.toml`: one application's, named after its executable
  (`notepad.toml`, `chrome.toml`);
- `sites/<host>.toml`: one web site's (`mail.example.org.toml`), over its browser's.

A setting is looked up from the most specific layer down to Okur's default; Insert+Shift+2's
punctuation level lasts until Okur exits. Okur rewrites only the values it changes, so your
comments stay. The command-line options (`--engine`, `--voice`, `--rate`, `--volume`,
`--keyboard-layout`, `--okur-key`, `--typing-echo`, `--live-region-whole-text`, and
`--set <id>=<value>` for any setting the settings table declares) change a setting for one run only; tests also pass `--settings <folder>`, so that pressing OK in a test
never changes the user's settings.

To stop the daemon cleanly, press `Ctrl+C`.

---

## 3. Keyboard Navigation Shortcuts

When running in **Virtual PC Cursor** mode, the screen reader intercepts and handles the following shortcuts:

| Shortcut | Description | JAWS Script Equivalent |
|---|---|---|
| `DownArrow` / `UpArrow` | Read next / prior line | `NextLine` / `PriorLine` |
| `Ctrl+RightArrow` / `Ctrl+LeftArrow` | Read next / prior word | `NextWord` / `PriorWord` |
| `RightArrow` / `LeftArrow` | Read next / prior character | `NextCharacter` / `PriorCharacter` |
| `Ctrl+DownArrow` / `Ctrl+UpArrow` | Read next / prior paragraph | `NextParagraph` / `PriorParagraph` |
| `Ctrl+Home` / `Ctrl+End` | Jump to top / bottom of document | `Top` / `Bottom` |
| `H` / `Shift+H` | Next / prior heading | `MoveToNextHeading` / `MoveToPriorHeading` |
| `1` – `6` / `Shift+1` – `Shift+6` | Jump to heading of level 1–6 | `MoveToNextHeadingLevelX` |
| `K` or `U` / `Shift+K` or `Shift+U` | Next / prior link | `MoveToNextLink` / `MoveToPriorLink` |
| `T` / `Shift+T` | Jump to next / prior data table | `MoveToNextTable` / `MoveToPriorTable` |
| `Ctrl+Alt+Right` / `Ctrl+Alt+Left` | Move cell right / left in table | `TableCellRight` / `TableCellLeft` |
| `Ctrl+Alt+Down` / `Ctrl+Alt+Up` | Move row down / up in table | `TableCellDown` / `TableCellUp` |
| `F` / `Shift+F` | Next / prior form control | `FocusToNextField` / `FocusToPriorField` |
| `B` / `Shift+B` | Next / prior push button | `MoveToNextButton` / `MoveToPriorButton` |
| `E` / `Shift+E` | Next / prior edit box | `MoveToNextEdit` / `MoveToPriorEdit` |
| `X` / `Shift+X` | Next / prior checkbox | `MoveToNextCheckbox` / `MoveToPriorCheckbox` |
| `Tab` / `Shift+Tab` | Next / prior interactive tab stop | `NextTab` / `PriorTab` |
| `Space` | Toggle checkbox or ARIA tab selection | `ToggleControl` |
| `Enter` | Enter Forms Mode when focused on an Edit field | `EnterFormsMode` |
| `Ctrl` (tap) | Instantly silence ongoing speech | `StopSpeech` |

---

## 4. Modes and Transitions

1. **Virtual PC Cursor (Default)**:
   - Keystrokes are consumed by the screen reader for virtual buffer reading.
   - Quick navigation keys (`H`, `T`, `K`, etc.) navigate structural elements without typing characters into the underlying page.

2. **Forms Mode**:
   - Entering an edit box and pressing `Enter` transitions into Forms Mode.
   - The screen reader speaks `"Type in text."`.
   - In Forms Mode, keystrokes pass through to the OS / active application so the user can type into form inputs.
   - Pressing `Escape` exits Forms Mode back to Virtual PC Cursor mode, announcing `"Virtual PC Cursor"`.
