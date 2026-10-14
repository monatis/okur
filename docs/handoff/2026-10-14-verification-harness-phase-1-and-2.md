# Handoff: Verification Harness, Phases 1 and 2 (2026-10-14)

**Plan**: `docs/plans/2026-10-14-verification-harness-rebuild.md`.
**Preceded by**: `docs/handoff/2026-10-14-full-audit-of-prior-development.md`.
**Branch**: `audit-remediation`. Not merged to `master`; awaiting the project owner.

## Numbers (tool output, pasted verbatim)

```
$ cargo run -q -p sr-cli -- parity | tail -1
JAWS parity: 53/61 steps match exactly (0 scenarios could not run)
$ cargo run -q -p sr-cli -- invariants | tail -1
Buffer invariants: 22 violation(s)
$ cargo test --workspace
tests: 52 passed, 0 failed, 4 ignored   (ignored = 3 live audio/SAPI tests + 1 known gap)
```

**How to read these numbers.**
- The 53/61 is real: an exact match against JAWS transcripts, over Chrome's real tree.
- But it covers only shallow steps: most scenarios take 1–2 keystrokes from the top of a page, and the speech formatter was previously written against these exact strings.
- The invariants show what that hides: on `doc-reading`, Down Arrow never speaks any paragraph, and words merge across elements on every page.

## Phase 1: Clean slate (done)

The details are in commit `8b8b427`:
- Scenarios 31–65 are quarantined in `corpus/unverified/`.
- `cargo test` is hermetic.
- The manifest is generated (`sr-cli corpus-manifest`), with a freshness test.
- The commit gate runs `cargo test`, and reference data may not be committed together with `crates/`.
- The speech log is opt-in (`run --speech-log`, JSON Lines).
- The known-gap test pattern was introduced.
- The README and AGENTS overclaims were corrected.
- AGENTS.md gained the "Verification Integrity Rules".

## Phase 2: Real differential harness (core done)

- **UIA snapshots.**
  - `sr-platform::uia_snapshot` captures about 40 UIA properties per node in one `BuildUpdatedCache` call: 97–158 ms per page on `old-pc`.
  - Capture is run with `sr-cli snapshot` on old-pc through `skills/jaws-reference/scripts/capture-snapshot.ps1`, which loads the page exactly like `run-scenario.ps1`.
  - The five snapshots for scenarios 01–30 are in `corpus/snapshots/` (commit `9f07d55`), each hash-bound to its fixture.
- **One mapping.**
  - `uia_snapshot::to_aom` is a pure UIA→AOM conversion that relies on control types, ARIA roles and UIA properties, never localized text. old-pc's localized names are Turkish.
  - New AOM roles: `Group`, `RadioButton`, `Image`.
  - The live path does not use this mapping yet (phase 4).
- **Parity.**
  - `sr-corpus::parity` replays only `keyboard`/`jaws_script` steps, compares the normalized speech against every JAWS variant, and is ratcheted in `corpus/parity-baseline.json`.
  - `sr-cli parity [--all]` prints diffs.
  - A mutation check confirmed it: renaming "heading level" made the ratchet fail and name every affected step.
- **Invariants.**
  - `sr-corpus::invariants` checks five properties: text reachable, lines ordered, no repeated lines, words within nodes, and links listed.
  - They are ratcheted in `corpus/invariants-baseline.json`, and `sr-cli invariants` prints them.
- **Mapping test**: `uia_mapping_preserves_chrome_semantics` covers headings, link URLs, landmarks, checkbox state, edit values and table header scopes.

## Findings from real Chrome data

- `<p>` is exposed as a UIA `Group` with `aria_role` "group", holding `Text` children that carry `aria_role` "description". Hand-written trees used a `Paragraph` role Chrome never produces, which is why the critical buffer bug was invisible to tests.
- A heading has a raw-view-only text child that duplicates its name. The mapping drops raw-only text leaves.
- JAWS itself varied across trials in scenarios 20, 28 and 29: in some trials the page-load speech leaked into the first step. The parity comparator accepts any recorded variant.

## OpenDesk issues observed

- After a dropped connection, the MCP `opendesk_exec` kept failing with `websocket send error ... (os error 10054)` and never reconnected. The `opendesk exec` CLI kept working.

## Next session

Phase 2 remaining items:
- Capture-tooling upgrades: a provenance block in `run-scenario.ps1`, speech-quiescence settle, and scan-code keystrokes.
- Record **longer scenarios**: full-page Down Arrow reads and cycling every heading, link and form field, so that parity measures real reading.
- Periodic `cargo mutants`.

Then phase 3, buffer remediation, driven by the invariants baseline and the parity ratchet. Retire the hand-written `*.aom.json` and the authored-assertion tests once their coverage is replaced.
