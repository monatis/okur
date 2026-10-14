# Handoff: Realistic Evidence, Buffer Rewrite, IA2 and Event Traces (2026-10-14)

**Branch**: `longer-evidence-and-buffer-fixes` (based on `master` @ `9808fe6`).
**Preceded by**: `2026-10-14-verification-harness-phase-1-and-2.md`.
**Plan**: `docs/plans/2026-10-14-verification-harness-rebuild.md`.

## Numbers (tool output, pasted verbatim)

```
$ cargo run -q -p sr-cli -- parity | tail -1
JAWS parity: 294/449 steps match exactly (0 scenarios could not run)
$ cargo run -q -p sr-cli -- invariants | tail -1
Buffer invariants: 0 violation(s)
$ cargo test --workspace
tests: 787 passed, 0 failed, 199 ignored   (ignored: live-desktop/audio tests and known gaps)
```

At the start of this session the corpus (including the 17 new scenarios) measured 78/449.

## What was done

### Evidence and capture tooling

**`run-scenario.ps1` format 2:**
- Keys are sent as scan-coded `SendInput` chords, written readably ("Down", "Shift+H", "Insert+Down").
- Each step waits for **audio silence** (`sr-cli wait-silence`) and for a **silent baseline** before the key press.
- The speech history is copied exactly once per step. JAWS speaks and records a "Copy speech history to clipboard" confirmation on every copy, so polling the history corrupts it.
- Every trial reloads the page identically.
- Each run writes a provenance block (script, scenario and fixture hashes, plus Chrome and JAWS versions and the keyboard layout).
- No agent-authored assertions or summaries are written into evidence.

**`lab-push.ps1` / `lab-pull.ps1`:** hash-verified deployment to `old-pc` through Dropbox, using a mandatory **push ID**. A stale manifest once installed an old capture script; the evidence provenance caught it.

**`run-batch.ps1`:** unattended serial batches with `batch.log` and a `batch.done` marker.

**17 realistic scenarios (101–113, 201–204)** on `news-article.html` and `signup-form.html`:
- full-page Down and Up reads, and Say All
- heading, level, link, list, list-item, region, graphic and paragraph navigation, including wraps
- word and character navigation over punctuation
- table navigation including the edges
- form fields, form quick keys, and Tab order

16 of the 17 are deterministic across 5 trials.

**`.gitattributes -text`:** reference data is stored byte-for-byte. A fresh clone now passes the manifest check; this was verified with a real clone.

**`sr-cli evidence <id>`:** a readable view of transcripts and variants.

### Specifications from evidence

- **`docs/references/jaws-virtual-buffer-spec.md`:** about 27 rules, each citing `scenario/step`.
- **`docs/references/jaws-quick-navigation-keys.md`:** the `[Quick Navigation Keys]` table read from JAWS's `Default.JKM` (key-to-command interface only). JAWS binds by physical key.

### Engine

- **`sr-core::linearize`** replaces the 1,260-line builder. It produces JAWS lines: the title as line 0, container start and end lines, links, graphics and controls on their own lines, inline flow, and JAWS word segmentation. Each line carries its exact speech.
- **Quick navigation** shares one routine:
  - wrap and "none" messages
  - focus phrasing for F, E, X, A, C and B
  - non-wrapping heading-level keys
- **New keys:** G, A, O (article), Q (main region), `'` (tab) and `;` (onMouseOver).
- **Corrected bindings:** K is no longer bound to links, O is no longer MathML, and `;` is no longer landmarks.
- **Line reading:** Down at the bottom and Up at the top repeat the current line. Insert+Up and Insert+Down are bound.
- **UIA mapping:**
  - keeps raw text leaves
  - adds radio "checked" (from SelectionItem), password, multi-line, description, DOM ID, set position, page URL and tab panel

### Accessibility API layer

- **xa11y 0.15.2 is vendored** at `crates/xa11y` (MIT). See `crates/xa11y/VENDORED.md`, which covers the import changes, the hardening ratchet, and 16 live-desktop tests marked `#[ignore]`.
- **`xa11y-windows::ia2`:** IAccessible2 and IAccessibleText bindings in IDL vtable order, plus Chromium document capture (attributes, states, hypertext, rendered line ranges), exposed as `sr-cli snapshot --api ia2`. **Not yet run against a live Chrome.**
- **`sr-platform::event_trace`:** `sr-cli trace` records WinEvents with MSAA objects and UIA focus chains as JSON Lines. Use `run-scenario.ps1 -TraceDir` or `run-batch.ps1 -Trace`. **Not yet run on old-pc.**
- **Trace replay in parity:** focus events recorded during each step are replayed into `handle_focus_event` through `focus_event_from_chain`. That is one mapping, shared with the future live listener.

### Gates

- The panic-macro check never matched (a regex bug) and is now fixed. Our own crates have zero panic macros.
- The vendored crates are covered by a ratchet in `crates/xa11y/gate-baseline.json`.
- Reference data (evidence and snapshots) may not be committed together with `crates/` changes.
- The commit hook judges the whole command, so run generators and `git add` in a separate command (noted in CLAUDE.md).

## Removed until rebuilt against recorded evidence

These features were removed because they were never verified against JAWS:
- MathML linearization
- iframe navigation (M)
- tree views
- spanned table cells

The hand-written `*.aom.json` fixtures, the authored-assertion runner and `sr-cli test` are retired, superseded by parity. The quarantined scenarios 31–65 in `corpus/unverified/` remain as a list of features to re-record.

## Open questions and next steps (in order)

1. **Import the v2 re-recording** of 101–113 and 201–204, captured with the race-free script (batch running on `old-pc` at handoff time). Commit the evidence separately, then regenerate the manifest and baselines.
2. **First live runs:**
   - `sr-cli snapshot --api ia2` and `run-batch.ps1 -Trace` on `old-pc`.
   - Fix whatever the real data shows.
   - Commit IA2 snapshots and traces as reference data. Add `corpus/traces/` to the separation rule.
3. **Line wrapping (largest remaining parity loss):** JAWS wraps long paragraphs at about 152–155 characters.
   - Hypothesis: it follows Chrome's rendered lines.
   - Test: compare the IA2 `lines` of the long paragraphs with JAWS's lines, then re-record at a second window width.
4. **IA2 → AOM mapping:** blockquote (`tag:blockquote`), the block-versus-inline display rule, and visited links (`TRAVERSED`) for U and V.
5. **JAWS behaviors still needing experiments:**
   - The `2` key from the top also speaks the level-1 heading (104/s1, legacy 12/s1).
   - `I` skips nested list items (108/s8).
   - Scan code versus virtual-key code on layouts where they differ (for example, the key at the US `,` position on Turkish Q).
6. **Event-replay scenarios:** re-record 204 (Tab order) and the forms scenarios with `-Trace`, then build the deterministic desktop test app for native-UI verification.
