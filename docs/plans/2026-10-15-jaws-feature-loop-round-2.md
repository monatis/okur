# Plan: JAWS Feature Loop, Round 2 (2026-10-15)

**Branch**: start a new branch from `master` (round 1's `jaws-feature-loop` was merged into it).
**Preceded by**: handoff `docs/handoff/2026-10-15-jaws-feature-loop-round-1.md`.

## Goal

Keep the loop running:
1. Discover a JAWS feature.
2. Record it on old-pc.
3. Implement it.
4. Prove it with `sr-cli parity` (both layers) and live.

Live usability gaps the user reports come first.

How the loop runs (machines, OpenDesk, recording, importing evidence, dev-box rules) is in
[`docs/references/verification-loop.md`](../references/verification-loop.md). old-pc stays
offline until the user relaunches `opendesk serve` there. Until then, work on the
**priorities without old-pc** below, and check old-pc first at the start of each session:

```bash
opendesk exec --peer old-pc --timeout 20 "Get-Date -Format HH:mm:ss"
```

## Priorities without old-pc (the user's list, 2026-10-15)

The screen reader is close to usable for the user's daily work. These come next, in this order
when old-pc is offline, and interleaved with recording when it is back:

1. **Missing features that bite the user.** Take them from:
   - the user's own reports, which come first and must be proven live;
   - the JAWS manual, tutorial and keystroke lists (work item 5 below);
   - discovery on the dev box, **only when the user allows it** (verification-loop §7).

   A feature found on the dev box gets an old-pc scenario in `corpus/pending/` so it becomes
   evidence later.
2. **No panics in xa11y.** `crates/xa11y/gate-baseline.json` allows 1151 `unwrap`/`expect` and
   120 panicking macros. The pre-commit gate counts non-comment lines, so tests count too.
   - First separate test code from production code in the count, so the baseline measures what
     can crash the screen reader.
   - Then harden the code the screen reader runs, worst first: `xa11y-windows/src/uia.rs`,
     `xa11y-core/src/{element,locator,app,selector}.rs`, then `xa11y-macos`.
   - Return `Result` and use `get()` instead of indexing; lower the baseline in the same commit.
   - The CLI and MCP front ends (`xa11y/src/cli.rs`, `xa11y/src/mcp/`, the `cli` feature) are not used by
     `sr-cli run`, so they come last or are dropped from the vendored tree.
   - Finally remove the crate-level `indexing_slicing` allowance (`VENDORED.md`, "Hardening
     ratchet").
3. **Review the async machinery so nothing blocks the event loop or the keyboard hook.** What is
   there today (`crates/sr-cli/src/main.rs`, `run`):
   - The hook thread waits up to 150 ms for the engine's `std::sync::Mutex`, polling every 1 ms,
     then passes the key through ("engine busy: key passed through" in the debug log).
   - The accessibility-event task is a `tokio::spawn` task that takes the same blocking mutex
     and runs the engine on a runtime worker thread.
   - The engine calls speech (`SpeechOutput`) and application actions while it holds the lock.

   To do:
   - Measure first: log the time the engine holds its lock per key and per event, and find
     every OS call made under it (speech start, clipboard, `actions`).
   - Move blocking work off the lock and off runtime threads: a dedicated engine thread fed by
     channels, or `spawn_blocking` for OS reads.
   - Check that speech `speak`/`stop` never wait on the synthesizer. OneCore and audio
     (`tts/onecore.rs`, `audio.rs`) poll with `thread::sleep`.
   - Bound the event channel (it is unbounded) and coalesce floods of events.
   - The tracer's reads (`event_trace.rs`) already log "slow event read" past 250 ms; give the
     live listener (`events/windows.rs`) the same instrumentation.
4. **Code quality, refactoring, performance.**
   - `sr-core/src/engine.rs` keeps growing. Split by concern: keys, events, window
     announcements, and layout commands into modules beside `editing.rs` and `native_focus.rs`,
     without changing behavior. Parity and the tests must stay identical, and the commit says
     so with the tool output.
   - Look for duplicated UIA code between `sr-platform::uia_snapshot`, `event_trace` and xa11y
     (VENDORED.md, "Planned extensions"), and move it into xa11y.
   - Profile `sr-cli parity` and the live focus path (debug-log stage timings) before
     optimizing anything.

## Done on branch `code-quality` (2026-10-15), and what is left

Done, each commit through the gate with tests and parity unchanged:
- xa11y production code denies panicking calls (clippy `[lints]`); the gate denies string
  slicing.
- Nothing slow runs under the engine lock or on the event loop; the page buffer is built
  before the lock, in linear time (4× faster on deep pages).
- `engine.rs` is split by concern (3,271 lines to 397).
- Live latency is measured: `sr-cli latency <debug-log>`; every event read logs its stages.
- Focus reads: 1,792 ms to 55 ms per item in a 300-item list view (p50, `sr-cli latency`).
  - The position comes from the list view.
  - Siblings are read only when needed, and subtrees only where announcements use them.
  - Repeated focus events and a list's echo events are not read.
- Windows Terminal: no 6.6 s stall when it comes to the front; the Virtual Viewer shows a
  console's text.
- Live regions: the page is captured again once changes settle, not once per change.

Left, in order of expected effect:
1. **UIA focus events instead of `GetFocusedElement`.** The largest part of a focus read now
   (24 of 55 ms in the list view) is asking UIA which element has the focus. A UIA
   focus-changed handler registered with a cache request delivers the element with its
   properties, as NVDA does.
   - Check WinForms menus first: UIA keeps naming the control behind the menu [obs, 302].
   - Check Chrome's first focus too.
2. **Speech start.** OneCore synthesizes a whole chunk before playing it. Stream the audio as
   it is produced, and measure first-audio latency with `sr-cli speak --measure-latency`.
3. **Release profile:** `lto` and `codegen-units = 1`, measured with `buffer_bench` and
   `sr-cli latency`.
4. **Responsiveness guarded like parity:** a recorded baseline for `buffer_bench` and the
   replay time of `sr-cli parity`, with a check that fails on a large slowdown.
5. **Incremental page updates:** patch a changed live region's subtree into the buffer
   instead of capturing the page again.
   - Observed on the dev box: JAWS 2025 speaks a growing live region's whole text on every
     change (a 40-word stream gave the paragraph 37 times in its history), as the engine
     does.

## Work items for old-pc, in order

1. **Record what round 1 left unrecorded** (old-pc was offline). The scenarios wait in `corpus/pending/` (see its README); move each into `corpus/scenarios/` (and its fixture into `corpus/fixtures/`) before pushing it.
   - Deploy a release `sr-cli` built with trace format 7 and the UIA focus handler: `lab-push.ps1 -SrCli`, then `lab-pull.ps1` on old-pc.
   - Record:
     - **317**, Alt held while Tab is pressed three times. Check `sr-cli evidence 317 --trace` for `uia_focus` events. On Windows 10 the switcher (`MultitaskingViewFrame`) sits behind `ForegroundStaging`.
     - **316**, Win+M.
     - **318**, Virtualize in a page.
     - **319**, the Run dialog: its announcement, Virtualize Window, and Insert+Tab could join.
     - **320**, Notepad in the laptop layout: CapsLock+I, K, O, L, J, M, U, Insert+Up, and Ctrl+Right over code punctuation. It settles classic Notepad's word breaks against the UIA word unit used on Windows 11.
     - **321**, Virtualize Window in a console (Windows 10's classic console on old-pc): the opening line (the cursor's), Insert+T ("Virtual Viewer"), the first and last lines, and Escape. JAWS 2025 in Windows Terminal on the dev box gave "Virtualizing current window | C:UsersGigabyte>", "Virtual Viewer", the first output line, and the prompt line on Ctrl+End and on Escape.
     - **315**, a RichEdit field: lines and Say Font with paragraph formatting.
     - **208**, a web textarea, character by character (the fixed tracer).
   - Re-record **306**, **314** and **209** with trace format 7, which closes the steps whose traces predate it.
   - Commit the evidence, then regenerate the manifest and baselines. Evidence and code never share a commit.
   - Where Windows 10 JAWS differs from what the dev box showed, the old-pc evidence decides, unless the behavior is specific to Windows 11 (`apps::task_switcher`, Notepad's RichEdit).
2. ~~Make the dev-box tools a harness tool~~: done (`sr-cli keys`, `sr-cli jaws-history`, b2a0ecf).
3. **Windows 11 XAML menus** (Notepad's menu bar). UIA's MenuItem control type maps to `Generic`, so ours reads bare names ("Edit", "Undo"). JAWS 2025 on the dev box:
   - Alt: "menu | | File | 1 of 3 | To move through items press left or right arrow. | Alt followed by F"
   - Down: "Leaving menus | Text editor edit | Contains text | Context Menu | | New window Ctrl+Shift+N | 2 of 13 | To move through items press up or down arrow. | Alt followed by W"
   - Down: "New Markdown tab | 3 of 13 | Alt followed by M"
   - Right: "Undo Ctrl+Z | 1 of 15 | Alt followed by U"
   - Escape: "Leaving menus"

   Mapping MenuItem also changes web pages' buffers, so check it against parity, and against 302's Win32 menu evidence on old-pc.
4. **Virtualize Window in Notepad:** JAWS includes the menu bar and status bar (314/s10, s11).
5. **Discovery,** from the JAWS keystroke list. Candidates:
   - Insert+Tab in pages;
   - Insert+B in windows other than dialog boxes;
   - Insert+Num5, Say Character, pressed twice (phonetic), three times (character value);
   - Insert+Up pressed twice (spell the line);
   - Say All by line;
   - Insert+Ctrl+Up and Insert+Ctrl+Down for the speech rate.
6. **Gaps without an explanation yet:**
   - 312's missing `·😀`;
   - 207/s87's location after the title;
   - 303/s6;
   - 305/s13;
   - the Claude app announcing twice on arrival (fixed for Chrome by d8235b8; recheck in the Claude app);
   - Windows search on Windows 10 (old-pc): whether its results raise the same ElementSelected events (scenario still to write).
