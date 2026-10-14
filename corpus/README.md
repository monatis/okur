# JAWS 2025 Reference Speech Corpus

Autonomously collected, audited, and verified reference speech corpus from **Freedom Scientific JAWS 2025 (Build 2025.2505.43.400)** with **Google Chrome 154** on Windows 10, captured via the OpenDesk automation harness on remote peer `old-pc`.

## Status and Rules (updated 2026-10-14 after audit)

- **Genuine evidence**: only scenarios 01–30 in `evidence/` were produced by the capture tooling
  (`skills/jaws-reference/scripts/run-scenario.ps1`, 5 trials each, see `batch_progress.log`).
- **Quarantined**: scenarios 31–65 live in [`unverified/`](unverified/README.md). Their "evidence"
  was not produced by the capture tooling and must not be cited as JAWS ground truth.
- **Differential parity** (`sr-cli parity`): for every genuine scenario step, the engine is driven over
  the UIA snapshot Chrome produced for the same fixture on `old-pc` (`snapshots/`) and its speech is
  compared exactly (after whitespace normalization) with every transcript variant JAWS produced.
  Results are ratcheted in the generated `parity-baseline.json`.
- **Buffer invariants** (`sr-cli invariants`): universal properties (all text reachable, lines
  ordered, no merged words, links listed) checked on every snapshot, ratcheted in the generated
  `invariants-baseline.json`. Run both commands for current numbers; never quote numbers by hand.
- The older `assertions` in scenario files and the hand-written `*.aom.json` trees remain only as
  engine regression checks until phase 3 retires them.
- **`CORPUS_MANIFEST.json` is generated**: never edit it by hand. Regenerate with
  `cargo run -p sr-cli -- corpus-manifest --write`; the `manifest_is_current` test fails when stale.
- **Evidence is written only by capture tooling**, and is never committed together with
  `crates/` changes (enforced by the pre-commit gate).
- `fixtures/tables-navigation.html` is intentionally the exact byte-for-byte file JAWS was recorded
  on (it contains a `<!DICTYPE` typo and a corrupted cell). Fix it only together with re-recording.

## Corpus Version 1.1.0 (Audited Quality Gate Passed)

Capture run history for scenarios 01–30 (2026-10-12).

Original uncorrected v1.0.0 evidence traces are preserved in [`evidence-v1.0.0/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence-v1.0.0/).

### Suite Statistics

- **Total Scenarios**: 30
- **Total Executed Trials**: 150
- **Successful Trials**: 150 (100.0%)
- **Deterministic Scenarios**: 27 (90.0%)
- **Variable Scenarios**: 3 (10.0%) (`20-form-checkbox-toggle`, `28-dynamic-tab-navigation`, `29-dynamic-tab-selection`)
- **Unhandled / Unknown JAWS Scripts**: 0 (0.0%)

### Audited Repairs Summary

1. **`03-doc-prior-line`**: Replaced unexposed `PriorLine` COM script with `{UP}` keyboard command.
2. **`07-doc-prior-char`**: Replaced unexposed `PriorCharacter` COM script with `{LEFT}` keyboard command.
3. **`12-struct-heading-level-2`**: Fixed script name to canonical JAWS COM script `MoveToNextHeadingLevelTwo`.
4. **`19-form-mode-entry-exit`**: Executed full Forms mode entry (`{ENTER}`), typing, and exit (`{ESC}`), asserting mode transitions.
5. **`20-form-checkbox-toggle`**: Implemented Win32 hardware scan-code spacebar generation to toggle checkbox state and assert state transition.
6. **`28-dynamic-tab-navigation` & `29-dynamic-tab-selection`**: Upgraded fixture with accessible ARIA tablist and explicit boundaries; asserted dynamic selection transitions.
7. **Semantic Assertion & Quality Gate**: Upgraded `skills/jaws-reference/scripts/run-scenario.ps1` to reject unknown scripts, validate multi-attribute semantic assertions, and enforce DOM reset before each repetition.
8. **Evidence Representation**: Separated action metadata, raw speech observations (preserving control characters), and semantic assertion records.

### Catalog

| ID | Scenario | Category | Stability | Success Rate |
| :--- | :--- | :--- | :--- | :--- |
| `01-doc-say-all` | Full Document Reading (Say All) | Document Reading | deterministic | 5/5 trials (100%) |
| `02-doc-next-line` | Next Line Navigation (Down Arrow) | Document Reading | deterministic | 5/5 trials (100%) |
| `03-doc-prior-line` | Prior Line Navigation (Up Arrow) | Document Reading | deterministic | 5/5 trials (100%) |
| `04-doc-next-word` | Next Word Navigation (Ctrl+Right) | Document Reading | deterministic | 5/5 trials (100%) |
| `05-doc-prior-word` | Prior Word Navigation (Ctrl+Left) | Document Reading | deterministic | 5/5 trials (100%) |
| `06-doc-next-char` | Next Character Navigation (Right Arrow) | Document Reading | deterministic | 5/5 trials (100%) |
| `07-doc-prior-char` | Prior Character Navigation (Left Arrow) | Document Reading | deterministic | 5/5 trials (100%) |
| `08-doc-say-current-line` | Say Current Line | Document Reading | deterministic | 5/5 trials (100%) |
| `09-doc-say-current-word` | Say Current Word | Document Reading | deterministic | 5/5 trials (100%) |
| `10-doc-next-paragraph` | Next Paragraph Navigation (Ctrl+Down) | Document Reading | deterministic | 5/5 trials (100%) |
| `11-struct-next-heading` | Next Heading Navigation | Headings & Structure | deterministic | 5/5 trials (100%) |
| `12-struct-heading-level-2` | Next Level 2 Heading Navigation | Headings & Structure | deterministic | 5/5 trials (100%) |
| `13-struct-next-link` | Next Link Navigation | Headings & Structure | deterministic | 5/5 trials (100%) |
| `14-struct-next-list` | Next List Navigation | Headings & Structure | deterministic | 5/5 trials (100%) |
| `15-struct-next-landmark` | Next ARIA Landmark Navigation | Headings & Structure | deterministic | 5/5 trials (100%) |
| `16-form-next-control` | Next Form Control Navigation | Forms & Controls | deterministic | 5/5 trials (100%) |
| `17-form-next-button` | Next Button Navigation | Forms & Controls | deterministic | 5/5 trials (100%) |
| `18-form-next-edit` | Next Edit Field Navigation | Forms & Controls | deterministic | 5/5 trials (100%) |
| `19-form-mode-entry-exit` | Forms Mode Enter, Type & Exit | Forms & Controls | deterministic | 5/5 trials (100%) |
| `20-form-checkbox-toggle` | Checkbox Navigation & Space Toggle | Forms & Controls | variable | 5/5 trials (100%) |
| `21-table-next-table` | Next Table Navigation | Tables & Cells | deterministic | 5/5 trials (100%) |
| `22-table-cell-right` | Cell Navigation Right (Ctrl+Alt+Right) | Tables & Cells | deterministic | 5/5 trials (100%) |
| `23-table-cell-left` | Cell Navigation Left (Ctrl+Alt+Left) | Tables & Cells | deterministic | 5/5 trials (100%) |
| `24-table-row-down` | Cell Navigation Down (Ctrl+Alt+Down) | Tables & Cells | deterministic | 5/5 trials (100%) |
| `25-table-row-up` | Cell Navigation Up (Ctrl+Alt+Up) | Tables & Cells | deterministic | 5/5 trials (100%) |
| `26-boundary-heading-wrap` | Heading Boundary Wrap | Boundaries & Dynamics | deterministic | 5/5 trials (100%) |
| `27-boundary-no-tables` | Empty Structure Navigation Boundary | Boundaries & Dynamics | deterministic | 5/5 trials (100%) |
| `28-dynamic-tab-navigation` | Dynamic Tablist Focus Navigation | Boundaries & Dynamics | variable | 5/5 trials (100%) |
| `29-dynamic-tab-selection` | Dynamic Tab Selection & State Change | Boundaries & Dynamics | variable | 5/5 trials (100%) |
| `30-dynamic-hidden-skip` | Hidden Content Skipping Boundary | Boundaries & Dynamics | deterministic | 5/5 trials (100%) |

See [`quality-report.md`](file:///c:/Users/Gigabyte/screen-reader/corpus/quality-report.md) and [`CORPUS_MANIFEST.json`](file:///c:/Users/Gigabyte/screen-reader/corpus/CORPUS_MANIFEST.json) for the full machine-readable validation manifest and SHA-256 fixture hashes.
