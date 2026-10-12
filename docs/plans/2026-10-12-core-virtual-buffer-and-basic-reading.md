# Development Plan: Core Virtual Buffer & Basic Document Reading Parity

**Plan Date**: 2026-10-12  
**Target Milestone**: Milestone 1 – Basic Document Reading (`01-doc-say-all` through `10-doc-next-paragraph`)  
**Prerequisite Handoff**: [`docs/handoff/2026-10-12-corpus-audit-and-v1.1.0-verification.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-12-corpus-audit-and-v1.1.0-verification.md)  
**Corpus Evidence**: [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/) (Scenarios `01` – `10`)  

---

## 1. Goal & Scope

Build the foundational virtual buffer and virtual cursor text-navigation engine in `crates/sr-core`, and implement the differential test evaluator in `crates/sr-corpus` to verify 100% speech output and navigation parity against JAWS 2025 canonical recordings for the first 10 scenarios:
- `01-doc-say-all`: Full document sequential speech streaming
- `02-doc-next-line`: Down Arrow line navigation
- `03-doc-prior-line`: Up Arrow line navigation
- `04-doc-next-word`: Ctrl+Right word navigation
- `05-doc-prior-word`: Ctrl+Left word navigation
- `06-doc-next-char`: Right Arrow character navigation
- `07-doc-prior-char`: Left Arrow character navigation
- `08-doc-say-current-line`: Say Current Line command
- `09-doc-say-current-word`: Say Current Word command
- `10-doc-next-paragraph`: Ctrl+Down block/paragraph navigation

---

## 2. Research & Investigation Steps

1. **NVDA Buffer & Text Architecture Cross-Reference**:
   - Inspect NVDA's `NVDAObject` text providers and virtual buffer implementation (`virtualBuffers/`, `textInfos/offsets.py`) for line boundary normalization, word boundary tokenization (Unicode UAX #29 rules), and block element separation.
2. **JAWS Speech Token Formatting**:
   - Examine raw observation tokens in `corpus/evidence/01-doc-say-all-evidence.json` through `10-doc-next-paragraph-evidence.json`.
   - Replicate JAWS's speech phrasing for block headings (`heading level 1 <title>`), punctuation boundaries, and whitespace handling.
3. **JAWS Scripts & Help Manuals on `old-pc`**:
   - Use OpenDesk to inspect JAWS default reading scripts (`Default.jss`, `SayNextLine`, `SayPriorLine`, `SayNextWord`, `SayPriorWord`, `SayAll`) if boundary edge-cases arise.

---

## 3. Implementation Tasks

### Phase A: `crates/sr-core` Core Models
- [ ] Define `AomNode` / `BufferNode`: semantic node representation with role, name, text content, and hierarchy.
- [ ] Implement `VirtualBuffer`:
  - Linearize node tree into a continuous text buffer with offset spans mapping to source nodes.
  - Calculate line offsets, word offsets, and paragraph boundaries.
- [ ] Implement `VirtualCursor`:
  - Track current character offset within the linearized buffer.
  - Implement navigation primitives:
    - `move_next_line()`, `move_prior_line()`, `current_line()`
    - `move_next_word()`, `move_prior_word()`, `current_word()`
    - `move_next_char()`, `move_prior_char()`, `current_char()`
    - `move_next_paragraph()`, `move_prior_paragraph()`
    - `say_all()`: Yield speech runs from current cursor position to document end.
- [ ] Implement `SpeechFormatter`:
  - Formats node and text transitions into canonical speech sequences matching JAWS transcripts.

### Phase B: `crates/sr-corpus` Differential Test Runner
- [ ] Parse declarative scenario files (`corpus/scenarios/01-*.json` .. `10-*.json`).
- [ ] Ingest HTML fixture (`corpus/fixtures/doc-reading.html`) or construct matching buffer.
- [ ] Execute scenario steps against `sr-core::VirtualCursor`.
- [ ] Assert produced speech matches canonical observations in `corpus/evidence/*-evidence.json`.

### Phase C: `crates/sr-cli` Integration
- [ ] Wire CLI command `sr-cli test --scenarios 01..10` to execute the differential suite.

---

## 4. Verification & Quality Gates

1. **Compilation & Quality Gate**:
   - `cargo fmt --check` must pass.
   - `cargo clippy --all-targets --all-features -- -D warnings` must pass with 0 errors and 0 warnings.
2. **Unit & Differential Tests**:
   - `cargo test --all` must pass.
   - All 10 basic document reading scenarios must achieve 100% parity with JAWS canonical evidence.

---

## 5. Session Exit & Handoff Checklist

- [ ] Write `docs/handoff/2026-10-13-basic-document-reading-parity.md` documenting verified behavior and test results.
- [ ] Create next plan in `docs/plans/` targeting Milestone 2 (Headings & Structural Navigation, scenarios `11`–`15`).
- [ ] Update `AGENTS.md` to reference the new plan.
