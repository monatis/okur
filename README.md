# Okur

**An open screen reader that keeps your JAWS muscle memory.**

Okur ("reader" in Turkish) is a modern, open-source screen reader written in Rust. It speaks,
navigates and behaves like **JAWS 2025**, down to its keystrokes, phrasing and reading of web
pages and Windows applications. It is being built by AI agents in a **clean room**: they
never see JAWS's code, only what JAWS says when keys are pressed on a separate lab machine.
Every behavior they implement must match recorded JAWS evidence before it counts.

> **Status (October 2026):** on Windows it is approaching everyday use. Web pages in Chrome,
> File Explorer, Notepad, Windows Terminal, dialog boxes, the desktop, Alt+Tab and Windows
> search are read as JAWS reads them. macOS and Linux have their accessibility layer; their
> keyboard and event plumbing comes next. All figures below are tool output, not estimates.

| Measure | Result | Command |
|---|---|---|
| Steps matching JAWS's speech history, word for word | **818 of 844** | `okur parity` |
| Steps matching what JAWS sent its synthesizer, pitch included | **171 of 172** | `okur parity` |
| Recorded JAWS scenarios / trials | **65 / 307** | `corpus/CORPUS_MANIFEST.json` |
| Tests | **877 passed, 0 failed** | `cargo test --workspace` |
| Virtual buffer invariant violations | **0** | `okur invariants` |
| `unwrap`, `expect`, `panic!` in the screen reader's own crates | **0**, tests included | pre-commit gate |

---

## Why re-implement JAWS?

Blind people use the computer with the keyboard. With daily use, the keystrokes become instant:
Insert+F7 for the list of links, H for the next heading, Insert+T for the window title,
CapsLock+Alt+W for the Virtual Viewer. Nobody thinks about them, just as sighted people don't
think about using the mouse.

Now suppose someone changed your mouse overnight, and it behaved differently from what you
were used to. That is how changing screen readers feels after years, often decades, of daily
use. The alternatives are good, but they use other keys, other phrasing and other ideas of
where the cursor is. So people stay with what they know, and for many that is JAWS: a
commercial product, three decades old, that runs only on Windows.

Okur keeps the muscle memory and replaces everything underneath:
- **Open source.**
- **Memory-safe and fast**, written in Rust.
- **Built to run on Windows and macOS** as first-class platforms, with Linux as a second tier.

The keystrokes, the announcements and the navigation model are the interface people rely on,
so they follow JAWS, checked step by step against recordings of it. Everything behind them is
our own.

---

## How can AI agents build a screen reader?

Clean-room design is an old idea: one team describes what a system does, and another, which has
never seen its insides, builds a new one from that description. Here the description is not a
document. It is **JAWS itself, observed as a black box**, and the builders are AI coding
agents (Claude Code, among others) working in a loop with a verification harness.

That only works if the agents' claims can be checked. An agent can confidently report "parity
achieved" about code that does nothing of the kind, and on 2026-10-14 an audit of this project
found exactly that: parity claims no tool had ever computed. Since then, the rule is simple.
**Talk is cheap; the harness decides.** No behavior counts until a tool compares it with
recorded JAWS evidence, and every number in a commit or handoff is pasted tool output
([`AGENTS.md`](AGENTS.md), "Verification Integrity Rules").

### The lab: two machines

- **The development machine** builds and tests the code, replays recorded evidence, and runs
  the screen reader live.
- **`old-pc`, the reference lab:** a separate Windows 10 machine running **JAWS
  2025.2505.43.400** with Chrome, used only as a black box. No JAWS code, script or binary is
  ever copied, decompiled or read; the harness only presses keys there and records what JAWS
  says.

The agents reach the lab through **[OpenDesk](https://github.com/monatis/opendesk)**, our own
open-source remote-desktop harness for AI agents. It runs commands on a remote machine, sends
input, and reads its screen and UI, over a paired connection (`opendesk exec --peer old-pc
"..."`). Files move between the machines through a shared folder, and every transfer is
checked by hash.

### Hearing exactly what JAWS says

JAWS's speech history keeps text before JAWS processes symbols, so it is not quite what a user
hears. To capture the real thing, the project includes the **SR Speech Recorder**
([`crates/okur-speech-recorder`](crates/okur-speech-recorder)). It is a SAPI 5 voice that JAWS speaks
through on the lab machine:
- it logs every utterance JAWS sends it, with pitch changes such as JAWS's higher pitch for
  capital letters;
- it hands the text on to a real voice, so speech stays audible and its timing realistic.

Scenarios recorded since the voice was installed keep both layers, the speech history and the
spoken text. A tracer also records the accessibility events Windows raised during each trial,
so replay feeds the engine the same events JAWS saw.

### The loop

Every feature goes around the same loop
([`docs/references/verification-loop.md`](docs/references/verification-loop.md) has the full
lifecycle):

1. **Discover.** Start from a gap the user reports, a JAWS manual or keystroke list, or JAWS's
   observed behavior, and write down what JAWS should say.
2. **Record the reference.** Write a fixture (a web page or a small Windows app) and a
   scenario of keystrokes, run it on `old-pc` several times, and keep JAWS's speech,
   synthesizer text, event traces and provenance (OS, JAWS version, settings, script hash).
3. **Implement** the behavior from scratch in Rust, with per-application behavior in named
   modules, as JAWS keeps it in application scripts.
4. **Test.** Replay every recorded scenario through the engine (`okur parity`) and compare
   each step with JAWS, on both layers. Results are ratcheted: a baseline file records them,
   and a test fails if any step that matched stops matching.
5. **Prove it live.** Run the screen reader against real applications with a debug log, and
   measure its responsiveness (`okur latency`).

Each round ends with a dated handoff and the next plan in [`docs/`](docs/), so the next agent
session starts where this one stopped.

### Integrity rules

From [`AGENTS.md`](AGENTS.md), enforced by tests and the commit gate where a machine can
enforce them:
1. Evidence is written only by capture tooling on the lab machine; never by hand.
2. Evidence and engine code never land in the same commit.
3. Generated artifacts (the corpus manifest, the parity and invariant baselines) are
   regenerated, never edited.
4. Every parity number or "verified" claim quotes the command that produced it.
5. `cargo test` is hermetic: no live UI, speech, audio or network. Live checks are separate.
6. A known but unimplemented JAWS behavior becomes a test asserting it, marked as a known gap.
7. No fixture-shaped special cases: code never branches on a test page's text or a machine's
   window titles.

---

## Rust, safety and quality gates

A screen reader that crashes leaves a blind user without a computer, so crashing is treated as
the worst possible bug.

- **No panics in the screen reader's own crates.** `unwrap`, `expect`, `panic!`, `todo!`,
  `unimplemented!` and `dbg!` are rejected at commit, in tests too. Unchecked indexing and string
  slicing are clippy errors (`indexing_slicing`, `string_slice`).
- **The vendored accessibility layer is held to the same standard.** Its production code
  denies panicking calls in production code through each crate's `[lints]` table; only its
  tests may panic.
- **A gate on every agent commit.** `git commit` runs clippy with warnings denied, the
  full test suite (which includes the parity and manifest checks), and the evidence/code
  separation check. A stop hook keeps an agent from finishing its turn with code that is
  unformatted or fails clippy.
- **Responsive by construction.**
  - The keyboard hook never waits more than 150 ms for the engine.
  - Slow work (building a page's buffer, reading text, writing logs, printing to the console)
    runs off the engine lock and off the event loop.
  - A test fails if building a page's buffer stops being linear in its depth.

### Performance, measured

Measured on the development machine with `okur latency` (from a live run's debug log) and
`buffer_bench`, release build:

| What | Before | After |
|---|---|---|
| Reading a focus change in a 300-item list (median) | 1,792 ms | **55 ms** |
| The same, slowest 10% | 2,086 ms | **78 ms** |
| Switching to a Windows Terminal window | 6.6 s stall | **no stall** |
| Building a 9,866-node page's virtual buffer | 59 ms | **14 ms** |
| A streamed chat reply (37 live-region updates) | a page capture per update (by design) | **2 captures** |
| Speak call to audio reaching the device (OneCore, median) | | **5 ms** |
| The engine handling a key (90th percentile) | | **0.1 ms or less** |

Most of the gain came from measuring before optimizing. Each stage of an accessibility event
read is timed in the debug log, which showed exactly where the cross-process calls went.

---

## What it does today (Windows)

- **Web pages in browse mode** with the JAWS Virtual PC Cursor:
  - quick navigation keys (headings and levels, links, form fields, tables, lists, landmarks,
    and more);
  - element lists, tables and complex headers, frames, single-page-app route changes;
  - forms mode, automatic or manual.
- **Live regions** announced as JAWS does. Streamed chat replies are read once, in phrases
  paced by speech, rather than reread whole on every update (JAWS's own way remains available
  with `--live-region-whole-text`).
- **Windows applications** announced in JAWS's words:
  - dialogs with their text and buttons;
  - lists, combo boxes, trees, menus, the desktop, the Alt+Tab switcher, Windows search, File
    Explorer;
  - editing in text fields, with characters, words, lines, selections and Say Font.
- **Virtual Viewer** (Insert+Alt+W) for windows, controls and consoles, including Windows
  Terminal's text.
- **Both keyboard layouts**, desktop and laptop, with Insert or CapsLock as the JAWS key.
- **OneCore and SAPI 5 voices.**

Reference documentation for these behaviors is in
[`docs/references/`](docs/references/).

---

## Cross-platform

| Platform | Tier | Status |
|---|---|---|
| Windows 10/11 | 1 | Runs today: UI Automation, IAccessible2, MSAA and WinEvents; low-level keyboard hook; OneCore and SAPI 5 speech |
| macOS | 1 | The accessibility layer (AXUIElement) is in place; the keyboard event tap, event listener and speech are next |
| Linux | 2 | The accessibility layer (AT-SPI2) is in place; best effort |

Every OS-dependent part is a trait in the core (keyboard hook, speech output, accessibility
events, application actions), with a mock implementation for tests. The engine never knows
which platform it runs on.

The cross-platform accessibility layer is **[xa11y](https://github.com/xa11y/xa11y)**,
vendored in [`crates/xa11y`](crates/xa11y) and extended here
([`VENDORED.md`](crates/xa11y/VENDORED.md)). Platform patterns were learned from NVDA's open
source code; everything the user hears follows JAWS.

---

## Getting started

Requires a recent stable Rust, on Windows 10 or 11.

```bash
cargo build --release --bin okur
```

Run the screen reader (stop it with Ctrl+C in its console):

```bash
target/release/okur run --keyboard-layout laptop
```

Useful options:

| Option | Effect |
|---|---|
| `--engine onecore` or `--engine sapi5` | the speech engine (default: OneCore, else SAPI 5) |
| `--keyboard-layout desktop` or `laptop` | the JAWS keyboard layout |
| `--live-region-whole-text` | read growing live regions as JAWS does |
| `--debug-log <file>` | log keys, events and timings; summarize them with `okur latency <file>` |

Developer commands:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run --bin okur -- parity
```

---

## Repository map

- [`crates/okur-core`](crates/okur-core): the engine. It holds the accessibility object model and
  the virtual buffer, and handles keys, focus and live regions, with speech in JAWS's phrasing.
- [`crates/okur-platform`](crates/okur-platform): Windows (and future macOS and Linux) bindings.
  These are the event tracer, keyboard hook, speech engines and application actions.
- [`crates/okur-corpus`](crates/okur-corpus): parity replay, invariants, the corpus manifest and
  the latency summary.
- [`crates/okur`](crates/okur): the screen reader and every harness command (`run`,
  `parity`, `evidence`, `trace`, `snapshot`, `latency`, `keys`, `jaws-history`, and more).
- [`crates/okur-speech-recorder`](crates/okur-speech-recorder): the SR Speech Recorder voice.
- [`crates/xa11y`](crates/xa11y): the vendored cross-platform accessibility layer.
- [`corpus/`](corpus): fixtures, scenarios, recorded JAWS evidence, event traces, snapshots,
  and the generated manifest and baselines.
- [`skills/jaws-reference/`](skills/jaws-reference): the recording toolkit that runs on the
  lab machine.
- [`docs/`](docs): plans, handoffs and references. Start with [`AGENTS.md`](AGENTS.md) and the
  [verification loop](docs/references/verification-loop.md).

---

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

The workspace declares `MIT OR Apache-2.0` in [`Cargo.toml`](Cargo.toml). The vendored
xa11y crates are MIT licensed ([`crates/xa11y/LICENSE`](crates/xa11y/LICENSE)).
