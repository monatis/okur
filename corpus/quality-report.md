# Reference Speech Corpus Quality & Validation Report

> **Note (2026-10-14)**: This report covers the 2026-10-12 capture run of scenarios 01–30 only. Its "verified" wording refers to JAWS trial capture, not to any comparison with this project's engine; see `README.md` for current status.

**Corpus Version**: 1.1.0  
**Generated At**: 2026-10-12T22:30:04.3342049+03:00  
**Reference Environment**: Freedom Scientific JAWS 2025.2505.43.400, Google Chrome 154.0.8037.98 on Windows 10 (Microsoft Windows NT 10.0.19045.0)

## Quality Gate Summary

- **Total Scenarios**: 30
- **Deterministic Scenarios**: 27 (90.0%)
- **Variable Scenarios**: 3 (10.0%)
- **Total Trials Executed**: 150
- **Successful Trials**: 150 (100.0%)
- **Unhandled / Unknown JAWS Scripts**: 0 (0.0%)

## Audited Repairs Executed

1. **`03-doc-prior-line`**: Replaced unexposed `PriorLine` COM script with `{UP}` keyboard command. Verified valid line speech output across all 5 trials.
2. **`07-doc-prior-char`**: Replaced unexposed `PriorCharacter` COM script with `{LEFT}` keyboard command. Verified valid character speech output across all 5 trials.
3. **`12-struct-heading-level-2`**: Discovered canonical JAWS script is `MoveToNextHeadingLevelTwo` (spelled-out word, matching JAWS command structure). Verified level 2 heading announcements across all 5 trials.
4. **`19-form-mode-entry-exit`**: Executed full Forms mode entry (`{ENTER}` / `"Type in text."`), text typing (`"Jane DoeABC"`), and exit (`{ESC}` / `"virtual PC Cursor"`), explicitly asserting Forms mode active/inactive state transitions.
5. **`20-form-checkbox-toggle`**: Implemented Win32 hardware scan-code spacebar generation (`keybd_event(0x20, 0x39)`). Executed checkbox navigation and activation (`{SPACE}`), asserting initial unchecked state and toggle to checked state.
6. **`28-dynamic-tab-navigation` & `29-dynamic-tab-selection`**: Upgraded fixture `boundaries-dynamic.html` to an accessible ARIA tablist structure with explicit boundary controls and activation handlers. Validated keyboard tab navigation and dynamic selection state transitions.
7. **Semantic Assertion & Quality Gate Harness**: Upgraded `skills/jaws-reference/scripts/run-scenario.ps1` to reject unknown/unhandled script calls, evaluate multi-dimensional semantic assertions (`target_role`, `target_name`, `target_selected`, `target_col`, `target_row`, `forms_mode_active`, `contains_speech`), and enforce clean DOM reloads between trials.
8. **Evidence Representation**: Separated action metadata, raw speech observations (preserving control characters `\u001d` and `\u001e`), and semantic assertion results into structured fields.
9. **Corpus Preservation**: Original v1.0.0 evidence safely preserved in `corpus/evidence-v1.0.0/` without silent overwrites.

## Fixture Verification Hashes (SHA-256)

| Fixture | SHA-256 Hash | Size (bytes) |
| :--- | :--- | :--- |
| `boundaries-dynamic.html` | `4CAAA7092DF80F075B4FD5BFA8F3FBBDBA558F90188E4F3711AEC05DD9C71953` | 2,383 |
| `doc-reading.html` | `7A68CA85FBFBC6DEAB6E587DC60A7C85A913437047C97FA9A807D3BC9786CBE8` | 532 |
| `forms-interaction.html` | `18FCE31741084133EC7F31CD746E51F3CF00A36DF7D5B92A1D5AF35410748D5F` | 862 |
| `structural-navigation.html` | `DD74A3E0285DB898CE8A785A5DA59A730A7A0E95BC6E1396D2E21E0C91D9948D` | 1,314 |
| `tables-navigation.html` | `685CA0B777B5B0184D58C505D3161EB348EC6356D7F5B27AD25EDDACCDC03C86` | 887 |

## Scenario Stability & Success Breakdown

| Category | Total Scenarios | Deterministic | Variable | Trial Success Rate |
| :--- | :--- | :--- | :--- | :--- |
| **Basic Document Reading (01–10)** | 10 | 10 | 0 | 50 / 50 (100%) |
| **Headings & Structure (11–15)** | 5 | 5 | 0 | 25 / 25 (100%) |
| **Forms & Controls (16–20)** | 5 | 4 | 1 | 25 / 25 (100%) |
| **Tables & Cells (21–25)** | 5 | 5 | 0 | 25 / 25 (100%) |
| **Boundaries & Dynamics (26–30)** | 5 | 3 | 2 | 25 / 25 (100%) |
| **Overall Corpus Suite** | **30** | **27 (90%)** | **3 (10%)** | **150 / 150 (100%)** |
