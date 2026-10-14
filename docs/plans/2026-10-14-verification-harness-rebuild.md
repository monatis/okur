# Plan: Verification Harness Rebuild, then Core Remediation (2026-10-14)

**Prerequisite reading**: `docs/handoff/2026-10-14-full-audit-of-prior-development.md`.
**Supersedes**: `2026-10-13-complex-web-apps-google-docs-and-sheets.md`, deferred until phase 5 is done.

## Principle

The agent that implements a feature must not be able to declare it verified. Every parity
statement must be computed by tooling from evidence the implementer did not author, and must be
reproducible with one command.

## Phase 1: Clean slate (done 2026-10-14)

- Quarantine scenarios 31–65 in `corpus/unverified/`.
- Make `cargo test` hermetic: no live UIA, speakers, or audio devices (live tests are `#[ignore]`).
- Generate `CORPUS_MANIFEST.json` from disk, with the `manifest_is_current` test.
- Commit gate: run `cargo test --workspace`; evidence edits may not be committed together with `crates/` edits.
- Make the speech log opt-in (`run --speech-log`).
- Use the known-gap test pattern: `#[ignore = "known gap: ..."]` tests assert correct JAWS behavior and document the gap.

## Phase 2: Real differential harness

1. **Raw platform snapshots.**
   - `sr-platform::uia_snapshot` defines a serializable `UiaNode` with every UIA property the screen reader needs. Examples: control type, localized type, name, value, ARIA role and properties, heading level, landmark type, toggle, expand/collapse and selection state, PositionInSet/SizeOfSet, GridItem row/column and spans, LiveSetting, IsOffscreen, and the help/full description.
   - It is captured in one cross-process call with a subtree `CacheRequest`.
   - `sr-cli snapshot --process chrome --out <file>` writes it.
   - On `old-pc`, it is captured from the exact fixture bytes JAWS was recorded on, and stored at `corpus/snapshots/<fixture>.chrome.uia.json` with the fixture hash, Chrome version and capture tool version.
2. **One mapping function.**
   - `uia_snapshot -> AomNode` is a pure function that compiles on every OS.
   - The live path must use the same function (live capture produces a `UiaNode` tree first), so tests exercise production mapping code on real Chrome data.
   - The hand-written `*.aom.json` files are deleted once snapshots exist.
3. **Differential comparison.**
   - For every scenario step that has genuine evidence, feed the scenario's real keystrokes to the engine and compare its speech against **every distinct variant** JAWS produced across trials. Some scenarios vary between trials; see the manifest's `deterministic` flag.
   - The normalizer is small and documented: unify line endings, collapse whitespace runs, and trim.
   - The `\u{1d}`/`\u{1e}` markers are kept, because they encode JAWS speech structure.
   - Output per step: exact match, or a token diff.
4. **Ratchet baseline.**
   - `corpus/parity-baseline.json` records each step's status. It is generated, never hand-edited.
   - `cargo test` fails if a step that matched JAWS regresses, or if the baseline is stale.
   - Improvements require regenerating the baseline in a commit that touches no evidence.
   - The parity percentage is printed by `sr-cli parity`. Docs must quote the tool's output, never a hand-written number.
5. **Universal buffer invariants** (property tests over every snapshot and over random trees):
   - All visible text is reachable with Down Arrow.
   - Lines are in document order and do not overlap.
   - Word navigation never merges text from different elements.
   - Every link and form control in the tree is in its list.
   - Next/prior moves are inverses away from the boundaries.
6. **Mutation testing.** Run `cargo mutants` on `sr-core` periodically, and treat surviving mutants in tested code as missing tests.
7. **Capture tooling upgrades.**
   - `run-scenario.ps1` writes a `provenance` block: script hash, fixture hash, Chrome version, host, and OpenDesk session.
   - Settle detection waits for speech-history quiescence instead of a fixed sleep (fixes the page-load leakage seen in scenarios 20, 28 and 29).
   - Keystrokes are sent as scan codes, so Insert/numpad/extended keys are faithful.

## Phase 3: Fix the virtual buffer against real trees

Fix the audit's critical buffer findings, driven by the invariants and the parity ratchet:
- Inline text gets no lines.
- Words merge across elements.
- Line order is wrong and lines overlap.
- Table cell content is lost.
- Line-length wrapping is missing.
- Word segmentation is wrong.
- Quick-key boundary messages and key bindings (K, U, A, G, Q, N, P, V) don't match JAWS.

Then re-record scenarios 31–65, starting with those that use only real keystrokes, using the capture tooling.

## Phase 4: Make interaction real

- Browse-mode Tab, Space, Enter and forms mode act on the real application (UIA focus, invoke, toggle, and setting values).
- Insert and CapsLock act as the JAWS key and are consumed, with laptop layout support.
- Bind Say All (with cursor tracking), Say Line, Insert+Tab, Insert+T and the other core JAWS keys.
- Switch to event-driven buffer updates: StructureChanged, TextChanged, LiveRegionChanged and Notification events through an MTA event thread. Evaluate xa11y `App::subscribe` first.
- Keep the user's place across updates.
- Echo the caret and typing in edit fields.
- Keep the keyboard hook callback minimal: hand work to the engine thread, and add a watchdog that reinstalls the hook.

## Phase 5: Live end-to-end differential on old-pc

- Run the same keystrokes on the same page, once with JAWS and once with our screen reader. JAWS is stopped while ours runs, and our run is logged with `--speech-log`. Then compare the transcripts with the phase 2 comparator.
- This is the only layer that catches event ordering, interruption, timing and real focus behavior.

## Done criteria for phase 2

- `sr-cli parity` prints per-step results for all 30 genuine scenarios, computed from snapshots captured on `old-pc`.
- The ratchet baseline is enforced by `cargo test`.
- The invariant suite runs on every snapshot.
- Known failures are listed as generated output, not prose.
