# Okur

[![CI status](https://github.com/monatis/okur/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/monatis/okur/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Written in Rust](https://img.shields.io/badge/rust-stable-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Platforms: Windows; macOS in progress](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%28in%20progress%29-lightgrey.svg)](#platforms)
[![JAWS parity: 818 of 844 recorded steps](https://img.shields.io/badge/JAWS%20parity-818%2F844%20steps-brightgreen.svg)](#where-it-stands)

**An open screen reader that keeps your JAWS muscle memory.**

Okur ("reader" in Turkish) is an open-source screen reader written in Rust. It speaks,
navigates and behaves like **JAWS 2025**: the same keystrokes, the same phrasing, the same
reading of web pages and Windows applications. It is built by AI agents in a **clean room**:
they never see JAWS's code, only what JAWS says when keys are pressed on a separate lab
machine, and nothing they build counts until it matches recorded JAWS speech.

## Why re-implement JAWS?

Blind people use the computer with the keyboard. With daily use, the keystrokes become
instant: Insert+F7 for the list of links, H for the next heading, Insert+T for the window
title. Nobody thinks about them, just as sighted people don't think about using the mouse.

Now suppose someone changed your mouse overnight, and it behaved differently from what you
were used to. That is how changing screen readers feels after years, often decades, of daily
use. The alternatives are good, but they use other keys, other phrasing and other ideas of
where the cursor is. So people stay with what they know, and for many that is JAWS: a
commercial product, three decades old, that runs only on Windows.

Okur keeps the muscle memory and replaces everything underneath:
- **Open source**, under the Apache License 2.0.
- **Memory-safe and fast**, written in Rust.
- **Built to run on Windows and macOS** as first-class platforms, with Linux as a second tier.

The keystrokes, the announcements and the navigation model are the interface people rely on,
so they follow JAWS, checked step by step against recordings of it. Everything behind them is
our own.

Where to go from here:
- **If you use a screen reader** and want to try Okur, go to
  [For screen reader users](#for-screen-reader-users).
- **If you are a developer** and want to contribute, go to [For developers](#for-developers).
- **If you are curious** how AI agents can build a screen reader, read
  [How AI agents build it](#how-ai-agents-build-it) further down.

## Where it stands

> **Status (October 2026): a preview on Windows.** Web pages in Chrome, File Explorer,
> Notepad, Windows Terminal, dialog boxes, the desktop, Alt+Tab and Windows search are read as
> JAWS reads them, and web apps such as GitHub and VS Code for the web are coming along. It
> is not yet a replacement for your screen reader. macOS has its accessibility layer; its
> keyboard and speech come next.

Every figure is tool output, rerun for this README.

| Measure | Result | Command |
|---|---|---|
| Steps matching JAWS's speech history, word for word | **818 of 844** | `okur parity` |
| Steps matching what JAWS sent its synthesizer, pitch included | **171 of 172** | `okur parity --spoken` |
| Recorded JAWS scenarios, trials | **65, 307** | `corpus/CORPUS_MANIFEST.json` |
| Tests | **913 passed, 0 failed** | `cargo test --workspace` |
| Virtual buffer invariant violations | **0** | `okur invariants` |
| `unwrap`, `expect`, `panic!` in the screen reader's own crates | **0**, tests included | the commit gate and CI |

---

## For screen reader users

### What works today (Windows)

- **Web pages in browse mode** with the Virtual PC Cursor:
  - quick navigation keys: headings and their levels, links, form fields, buttons, tables,
    lists, landmarks, and more;
  - the links, headings and form field lists (Insert+F7, Insert+F6, Insert+F5);
  - table navigation with Ctrl+Alt+arrows, frames, and single-page apps changing route;
  - forms mode, entered automatically or with Enter, and left with Escape;
  - Tab moves on from where you are reading, as with JAWS.
- **Web apps:** autocompletes and command palettes read their suggestions ("Apple, 1 of 3");
  regions with role "application" (such as VS Code's) get every key; GitHub's code view reads
  line by line.
- **Live regions** announced as JAWS announces them. A streamed chat reply is read once, as
  it arrives, not again from the start on every update; `--live-region-whole-text` gives
  JAWS's own behavior.
- **Windows applications:** dialog boxes with their text and buttons; lists, combo boxes,
  trees and menus; the desktop, the Alt+Tab switcher, Windows search and File Explorer.
- **Editing** in text fields: characters, words, lines, selections, and Say Font.
- **The Virtual Viewer** (Insert+Alt+W) for windows, controls and consoles, Windows Terminal
  included.
- **Both keyboard layouts,** desktop and laptop, with Insert or CapsLock as the JAWS key.
- **OneCore and SAPI 5 voices.**

The [reference documentation](docs/references/) describes these behaviors in detail.

### Trying it

There are no ready-made downloads yet: Okur is built from its source code.

1. Install [Rust](https://www.rust-lang.org/tools/install) and the Visual Studio C++ build
   tools it asks for, on Windows 10 or 11.
2. In a command prompt in this folder, build it:

   ```
   cargo build --release --bin okur
   ```

3. Run it from the same prompt. Okur speaks as soon as it starts:

   ```
   target\release\okur run
   ```

   With a laptop keyboard (CapsLock as the JAWS key):

   ```
   target\release\okur run --keyboard-layout laptop
   ```

4. To stop it, Alt+Tab to the command prompt it runs in and press Ctrl+C.

Two screen readers speaking at once is hard to follow. Unload your usual screen reader first,
or if you keep it running, give Okur another JAWS key so the two do not both act on the same
command: `--jaws-key scrolllock` (then ScrollLock+T says the window title, and so on).

Other options:

| Option | What it does |
|---|---|
| `--engine onecore` or `--engine sapi5` | which speech engine to use (OneCore by default) |
| `--voice <name>`, `--rate <-10 to 10>`, `--volume <0 to 100>` | the voice |
| `--keyboard-layout desktop` or `laptop` | the JAWS keyboard layout |
| `--jaws-key insert`, `capslock` or `scrolllock` | the JAWS key |
| `--live-region-whole-text` | read a growing live region whole each time, as JAWS does |
| `--debug-log <file>` | record keys, events and timings, for reporting a problem |

### Reporting a problem

Please [open an issue](https://github.com/monatis/okur/issues). The most useful reports say:
- what you did, key by key;
- what JAWS says there, and what Okur said instead;
- the application or web page.

A debug log helps: run Okur with `--debug-log okur.log`, repeat the problem, stop Okur, and
attach the file. It records keys and what is on screen, so check it before you share it.

---

## For developers

### Build and test

Requires stable Rust. The screen reader runs on Windows 10 and 11; the engine
(`okur-core`) builds and tests on any platform.

```
cargo build --release --bin okur
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::indexing_slicing -D clippy::string_slice
cargo test --workspace
cargo run --bin okur -- parity
```

`cargo test` is hermetic: no live UI, speech, audio or network. It includes the parity replay
of every recorded JAWS scenario, the corpus manifest check and the ratcheted baselines, so a
step that matched JAWS cannot silently stop matching.

### Quality gates

A screen reader that crashes leaves a blind user without a computer, so crashing is treated
as the worst possible bug.

- **No panics in the screen reader's own crates.** `unwrap`, `expect`, `panic!`, `todo!`,
  `unimplemented!` and `dbg!` are rejected, in tests too. Unchecked indexing and string
  slicing are clippy errors (`indexing_slicing`, `string_slice`).
- **The vendored accessibility layer is held to the same standard** through each crate's
  `[lints]` table; only its tests may panic, and its remaining panicking calls may only
  decrease (a ratchet).
- **Every commit passes the gate** ([`.agents/scripts/check-pre-commit.ps1`](.agents/scripts/check-pre-commit.ps1)):
  the panic scan, clippy with warnings denied, the full test suite, and a check that recorded
  JAWS evidence never lands in the same commit as engine code.
- **CI runs the same checks** on every push and pull request
  ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)), on Windows. It also runs the
  engine's tests on macOS and Linux, reported without failing the build until those ports are
  built there.
- **Responsive by construction.** The keyboard hook never waits more than 150 ms for the
  engine; slow work (building a page's buffer, reading text, writing logs) runs off the engine
  lock and the event loop.

### Performance

Measured on the development machine with `okur latency` (from a live run's debug log),
release build, Okur running next to JAWS:

| What | Result |
|---|---|
| Arrow key to speech in a Windows text field (median) | **19 ms** |
| Ctrl+Right to speech in VS Code's editor on the web (median) | **25 ms**, from 41 ms |
| A GitHub repository page readable after it loads | **160 ms**, from 3 s |
| Reading a focus change in a 300-item list (median) | **55 ms**, from 1,792 ms |
| Building a 9,866-node page's virtual buffer | **14 ms**, from 59 ms |
| The engine handling a key (90th percentile) | **0.1 ms** or less |

Most gains came from measuring first: every stage of an accessibility event read and every
key's way to speech is timed in the debug log.

### Repository map

- [`crates/okur-core`](crates/okur-core): the engine. The accessibility object model, the
  virtual buffer, keys, focus, live regions, and speech in JAWS's phrasing. Platform-neutral.
- [`crates/okur-platform`](crates/okur-platform): Windows bindings (and the future macOS and
  Linux ones): the event tracer, keyboard hook, speech engines and application actions.
- [`crates/okur-corpus`](crates/okur-corpus): parity replay, invariants, the corpus manifest
  and the latency summary.
- [`crates/okur`](crates/okur): the screen reader and the harness commands (`run`, `parity`,
  `evidence`, `trace`, `snapshot`, `latency`, `keys`, `jaws-history`, and more).
- [`crates/okur-speech-recorder`](crates/okur-speech-recorder): the SR Speech Recorder voice,
  which records what JAWS sends its synthesizer.
- [`crates/xa11y`](crates/xa11y): the cross-platform accessibility layer, vendored from
  [xa11y](https://github.com/xa11y/xa11y) and extended here
  ([`VENDORED.md`](crates/xa11y/VENDORED.md)).
- [`corpus/`](corpus): fixtures, scenarios, recorded JAWS evidence, event traces, snapshots,
  the generated manifest and baselines. [`corpus/pending/`](corpus/pending) holds scenarios
  waiting to be recorded.
- [`skills/jaws-reference/`](skills/jaws-reference): the recording toolkit for the lab
  machine.
- [`docs/`](docs): plans, handoffs and references.

### How the work is organized

The project is developed by AI coding agents, session after session.
[`AGENTS.md`](AGENTS.md) is their guide and points to the current plan and the latest
handoff. The [verification loop](docs/references/verification-loop.md) describes the lab
machines, recording JAWS evidence and live checks.

---

## How AI agents build it

Clean-room design is an old idea: one team describes what a system does, and another, which
has never seen its insides, builds a new one from that description. Here the description is
**JAWS itself, observed as a black box**, and the builders are AI coding agents (Claude Code,
among others) working in a loop with a verification harness.

That only works if the agents' claims can be checked. An agent can confidently report
"parity achieved" about code that does nothing of the kind, and on 2026-10-14 an audit of
this project found exactly that: parity claims no tool had ever computed. Since then the rule
is simple: **talk is cheap; the harness decides.** No behavior counts until a tool compares it
with recorded JAWS evidence, and every number in a commit or handoff is pasted tool output.

### The lab

- **The development machine** builds and tests the code, replays recorded evidence, and runs
  the screen reader live.
- **`old-pc`, the reference lab,** a separate Windows 10 machine running **JAWS
  2025.2505.43.400** with Chrome, used only as a black box. The harness presses keys there and
  records what JAWS says.

The agents reach the lab through **[OpenDesk](https://github.com/monatis/opendesk)**, our
open-source remote-desktop harness for AI agents. Files move between the machines through a
shared folder, and every transfer is checked by hash.

JAWS's speech history keeps text before JAWS processes symbols, so it is not quite what a
user hears. The **SR Speech Recorder** is a SAPI 5 voice JAWS speaks through on the lab
machine: it logs every utterance, with pitch changes such as JAWS's higher pitch for capital
letters, and hands the text on to a real voice. A tracer also records the accessibility events
Windows raised during each trial, so replay feeds the engine the same events JAWS saw.

### The loop

1. **Discover** a gap, from a user report, a JAWS manual or keystroke list, or JAWS's
   observed behavior.
2. **Record the reference:** a fixture (a web page or a small Windows app) and a scenario of
   keystrokes, run on `old-pc` several times, keeping JAWS's speech, synthesizer text, event
   traces and provenance.
3. **Implement** the behavior from scratch, with per-application behavior in named modules,
   as JAWS keeps it in application scripts.
4. **Test:** replay every recorded scenario through the engine (`okur parity`) and compare
   each step with JAWS. Results are ratcheted.
5. **Prove it live:** run the screen reader against real applications with a debug log, and
   measure its responsiveness (`okur latency`).

### Integrity rules

From [`AGENTS.md`](AGENTS.md), enforced by tests, the commit gate and CI where a machine can
enforce them:
1. Evidence is written only by capture tooling on the lab machine; never by hand.
2. Evidence and engine code never land in the same commit.
3. Generated artifacts (the corpus manifest, the parity and invariant baselines) are
   regenerated, never edited.
4. Every parity number or "verified" claim quotes the command that produced it.
5. `cargo test` is hermetic. Live checks are separate.
6. A known but unimplemented JAWS behavior becomes a test asserting it, marked as a known gap.
7. No fixture-shaped special cases: code never branches on a test page's text or a machine's
   window titles.

## Platforms

| Platform | Tier | Status |
|---|---|---|
| Windows 10 and 11 | 1 | Runs today: UI Automation, IAccessible2, MSAA and WinEvents; low-level keyboard hook; OneCore and SAPI 5 speech |
| macOS | 1 | The accessibility layer (AXUIElement) is in place; the keyboard event tap, event listener and speech are next |
| Linux | 2 | The accessibility layer (AT-SPI2) is in place; best effort |

Every OS-dependent part is a trait in the core (keyboard hook, speech output, accessibility
events, application actions), with a mock for tests. The engine never knows which platform it
runs on. Platform patterns were learned from NVDA's open source code; everything the user
hears follows JAWS.

## Clean room and interoperability

- No source code, scripts, binaries or decompiled code from Freedom Scientific are copied,
  bundled or examined. Everything here was written from scratch.
- JAWS is observed only from the outside: keystrokes in, speech out, on fixtures this project
  wrote.
- The project's position is that keystrokes, announcement phrasing and navigation models are
  functional methods of operation, an interoperability interface users depend on, and that
  observing a program's behavior to build a compatible one is legitimate.

JAWS is a trademark of Freedom Scientific (Vispero). Okur is independent and is not
affiliated with or endorsed by them.

## License

Okur is licensed under the [Apache License, Version 2.0](LICENSE); see also [`NOTICE`](NOTICE).
The vendored xa11y crates in [`crates/xa11y`](crates/xa11y) keep their MIT license
([`crates/xa11y/LICENSE`](crates/xa11y/LICENSE)).
