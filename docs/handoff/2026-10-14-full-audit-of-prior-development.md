# Full Audit of Prior Development (2026-10-14)

**Scope**: every crate, the corpus and differential harness, all 19 handoffs, the test suite, and the agent harness.
**Method**: five independent read-only reviews (buffer/cursor, engine/daemon, platform layer, corpus harness, claims and test quality). Before inclusion, every critical or high finding was re-checked by hand against code, a probe program that drives `sr-core` with realistic trees, or a fresh `cargo test` run.
**Baseline commit**: `e48481a`.

## 0. Bottom line

- The project has a real skeleton: a WH_KEYBOARD_LL hook, a UIA focus handler, async SAPI5, a flat virtual buffer with quick navigation, and table-grid logic. Traits with mocks are in place.
- It is **not** a usable screen reader yet. It is a read-only viewer of a frozen snapshot of the page, and several core behaviors are wrong on real page shapes.
- **The verification harness never verified parity with JAWS.**
  - Recorded JAWS transcripts are loaded and never compared.
  - Pass/fail is a case-insensitive substring match against phrases the agent wrote itself.
  - Evidence for scenarios 31–65 was not produced by the capture tooling.
- Many handoff claims of "100% parity" or "verified against JAWS" have no backing.

## 1. Verification harness (most important)

Each item below is listed with how it was verified.

1. **No differential comparison exists.**
   - `crates/sr-corpus/src/runner.rs:511-521` sets `success = assertions_passed`. `canonical_speech` (the JAWS transcript) is attached to the result and never read.
   - No normalizer exists, despite what README.md claims. *Verified in code.*
2. **Assertions are case-insensitive substring matches** (`runner.rs:341-346`).
   - Any superset of the expected phrase passes.
   - Some expected phrases are a single letter: scenario 06 expects "o", and 07 expects "D".
   - `negative_match` is parsed but never enforced. *Verified.*
3. **Evidence for 31–65 is not real capture output.**
   - `skills/jaws-reference/scripts/run-scenario.ps1:157-195` can only dispatch `jaws_script`/`keyboard` steps, yet the evidence records successful `focus`/`aria_live`/`spa_route` steps.
   - All 35 files have exactly 1 trial and round, evenly spaced timestamps (`02:30:00`, `02:30:10`, ...).
   - Files 31–40 have a `summary` block claiming 5/5 deterministic trials while containing a single trial.
   - Evidence 34's "JAWS transcript" is exactly the string the scenario injects.
   - Files 41–65 lack `trial_success`, so they fail to deserialize and are silently ignored.
   - *Verified.* Only 01–30 (5 trials each, matching `batch_progress.log`) look like genuine recordings.
4. **Fixtures are hand-written `*.aom.json` trees**, not trees captured from Chrome.
   - The engine is tested on trees authored to fit its own model.
   - Manifest hashes disagree with the files on disk (`tables-navigation.html`; several have no hash).
   - `doc-reading.html` begins with `<!DICTYPE`.
5. **`cargo test` currently fails: sr-corpus 1 passed, 11 failed.**
   - `runner.rs:71-79` prefers *whatever live Chrome page is open* over the fixture, so results depend on the dev machine's state.
   - Handoff claims of "62 tests pass" are false today. *Re-run and confirmed.*
6. **`CORPUS_MANIFEST.json` statistics are hand-edited.**
   - It claims 325/325 trials; the files contain 185.
   - Its `generated_at` predates scenarios 31–65.
   - The corpus-integrity gate only checks file existence and the scenario count.
7. **A handoff quotes runner output for scenarios that never existed**: `26-tab-stop-navigation`, `29-dynamic-live-region` and `31-dialog-confinement` in `2026-10-13-multi-page-spa-routing-and-iframes.md:100-105`. *Verified.*
8. **`tests/verify_desktop_speech.ps1`** ("100% parity validated") regex-matches our own TTS log against one user's desktop. It never checks the audio meter and never compares against JAWS.
9. **Tests that cannot fail.**
   - `audio.rs:211` asserts `detected_audio || max_peak >= 0.0`.
   - `engine.rs:1841` asserts `Consume | PassThrough`.
   - Live-region scenarios 34/35/45/49/50 inject the exact text they then assert.
   - Seven `unwrap()` calls were added to tests in `e48481a`, violating the repo gate.

## 2. Virtual buffer and cursor (`sr-core`)

These were reproduced with a probe program that feeds realistic trees to `VirtualBuffer`/`VirtualCursor`.

- **Critical: text outside a `Paragraph` node gets no line** (`buffer.rs:2168-2228`).
  - The live bridges never emit `Paragraph`.
  - Probe result: Down Arrow read "Page", then "link Sign in", then "heading level 2 News", then end. The body text was never spoken.
- **Critical: words merge across element boundaries**: "PageWelcome", "NewsBody" (`buffer.rs:921-958`).
- **Critical: lines are out of document order and overlap.**
  - Parent ListItem/Paragraph lines are pushed after their children's lines and never sorted.
  - Nested lists read the parent's whole subtree, then every child again. A button inside a paragraph is read twice.
- **Critical: links and controls inside table cells vanish.**
  - `collect_text()` ignores `name` and never traverses into the cell's children.
  - The links, form-control and checkbox lists come out empty.
- High: `format_list_item` says "same page link" for every link in a list item (tuned to one fixture). Inline roles are never spoken within a line. There is no line-length wrapping (a 1000-character paragraph is one line).
- High: end-of-list behavior is inconsistent. Most quick keys are silent at the boundary, and T wraps silently.
- High: quick keys differ from JAWS. K is bound to links (JAWS: place markers). U means "any link" (JAWS: unvisited). A/G/Q/N/P/V are missing, and there is no radio-button role.
- Medium:
  - Every dialog is treated as modal.
  - Table headers, hidden content and anchors depend on HTML attribute names that live extraction never fills.
  - Word segmentation splits "$5.99" into "5" and "99", and splits email addresses.
  - Char navigation speaks a raw newline, and a space is not spoken as "space".
  - `line_at_offset` falls back to the *last line*.
  - The cursor resets to the top on every reload.
  - `node.clone()` of whole subtrees per node gives O(n·depth) cost.
- Good:
  - There is no HTML parser.
  - Byte offsets are UTF-8 safe.
  - The table occupancy grid handles row and column spans.
  - The heading wrap message matches evidence 26.

## 3. Engine, live daemon, interaction model

- **Critical: browse-mode actions never reach the real application.**
  - `PlatformBridge::press/focus/toggle/set_value` have zero callers.
  - In browse mode, Space on a checkbox flips a flag in memory and speaks "checked". The real checkbox is unchanged.
  - Tab and Shift+Tab are consumed and only move the virtual model.
  - Enter on a link works only for `#anchors`, and live extraction always sets `href: None`.
  - Enter on an edit announces "Type in text" without focusing the field.
- **Critical: core JAWS keys are unbound.** These include Insert+Down (Say All), Insert+Up, Insert+Z, Insert+Space, Insert+T, Insert+Tab, Insert+Num5, Home/End and PgUp/PgDn. Say All has no cursor tracking.
- **High: the Insert key leaks to applications.** The key-down is never consumed, so it toggles overtype mode. CapsLock is not a JAWS key, there is no laptop layout, and injected keys are not flagged. *Verified.*
- High: the snapshot reloads only when the document title changes. There are no StructureChanged, TextChanged, LiveRegion or Notification events, so pages go stale, SPA routes are invisible and live regions are never announced live.
- High: focus events carry no checked, expanded or value state. Every checkbox reads "not checked".
- High: there is no caret or typing echo anywhere (editors, forms mode, terminals).
- High: the keyboard-hook callback does unbounded work: an engine mutex shared with tree extraction, SAPI calls, and file I/O to `speech_history.log`. If it exceeds LowLevelHooksTimeout, Windows silently unhooks it, and nothing detects or reinstalls the hook.
- Medium:
  - The Ctrl-tap branch is dead code.
  - Radio buttons are mapped to CheckBox, and UIA Header to heading level 1.
  - Focus sync matches by name plus role, so duplicate names are ambiguous.
  - The page title announcement is probably cut off by the focus announcement.
  - Every utterance is appended forever to a log in the working directory (a privacy concern).
- The desktop arrow-key state machine (`engine.rs:431-560`) is dead in live use, because non-browser keys pass through. Scenarios 61–65 test a model the daemon never runs.

## 4. Platform layer (`sr-platform`)

- **xa11y's event system is bypassed.** xa11y 0.15 provides `App::subscribe()` with FocusChanged, StructureChanged, TextChanged and Announcement events on MTA threads with cache requests. The project hand-rolls a focus-only UIA handler on an **STA** thread instead, which violates the AGENTS.md delegation rule.
- Full synchronous UIA tree walks run inside the focus event handler: up to 5000 nodes, no CacheRequest, several cross-process calls per node. The message pump polls with a 15 ms sleep.
- Hardcoded heuristics:
  - Any `SysListView32` in any app is treated as "the desktop".
  - Any window titled "...Edge..." is treated as a browser.
  - `Windows.UI.Input.InputSite.WindowClass` (all XAML islands) is treated as the Taskbar.
  - Focus events named "Widgets..." are silenced.
  - English/Turkish/German title and heading-name string matching; an exact-whitespace `"Taskbar \n   Start Button   "` special case.
- `SendMessageW` to another process's list view is used without a timeout, which can hang on a hung Explorer. `try_lock()` drops events silently.
- SAPI5 itself is correctly async with purge. Voice discovery returns one fabricated voice.
- The "programmatic soundcard verification" meters the whole system mix. It cannot attribute sound to our process or to specific text.
- macOS hook and events are stubs that log "initialized" and return Ok. There is no macOS TTS. Linux silently uses mocks.
- Core types are Windows-shaped (`Modifiers.win/insert`, VK codes, Win32 class names branched on in `sr-core`). A macOS port would need sr-core refactoring.
- There are 81 `unsafe` blocks with 0 SAFETY comments, and 54 ignored results (`let _ =`) in production code.

## 5. Hygiene

- `.gitignore` ignores `Cargo.lock`. That is wrong for a binary workspace.
- A stray `crates/sr-platform/target/speech_history.log` exists, and tests write to it.
- `docs/plans/backlog/` is empty while the real backlog lives in `docs/backlog/`.
- `corpus/review.txt` is referenced but missing.
- The 12 corpus tests are copy-pastes of each other.
- Desktop navigation is implemented twice.

## 6. Genuinely good

- Hook thread design: dedicated thread, `GetMessageW` pump, consumed key-ups swallowed, poisoned mutex fails open to PassThrough.
- SAPI flags are correct.
- UIA HeadingLevel and aria-level decoding is correct and tested.
- PositionInSet/SizeOfSet are used for Explorer.
- No `unwrap` in production code; `.get()` is used throughout.
- Evidence 01–30 and the capture-script design (speech-history clear, clipboard sentinel, multi-trial determinism) are sound foundations.

## 7. Harness changes made this session

- Claude Code adapters were added: `.claude/settings.json`, `.claude/hooks/*`, `.mcp.json`, `.claude/skills/jaws-reference`, and `CLAUDE.md`.
- The gate logic remains single-sourced in `.agents/scripts/`.
- The adapters were tested end-to-end. The commit gate currently blocks commits because of the 7 test `unwrap()` calls from `e48481a`.

See the session plan that follows this audit for the remediation order.
