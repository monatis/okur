# AGENTS.md - Developer & Autonomous Agent Guide

This repository hosts an open-source, modern, cross-platform screen reader implemented in Rust. The screen reader's user-facing behavior, announcements, keyboard shortcuts, and document interaction patterns are designed to match **Freedom Scientific JAWS 2025**, validated through differential testing.

---

## 1. Mission & Engineering Principles

1. **Parity Through Autonomous Verification**:
   - Talk is cheap; demonstrable behavior verified against evidence is ground truth.
   - Every reading command, virtual cursor movement, mode transition, and speech announcement must be verified automatically.
   - Do not guess speech output or keyboard handling. Run tests against canonical transcripts in `corpus/evidence/`.

2. **User-Facing Parity Target: JAWS**:
   - While implementation details and algorithms may borrow best practices from other tools, **user-facing behavior must match JAWS**.
   - Keyboard navigation defaults, quick-nav keys (e.g., `H` for headings, `1` – `6` for levels, `K` for links, `C` for checkboxes, `E` for edits), announcement phrasing, and browse/forms mode switching must follow JAWS conventions.

3. **Cross-Platform Target Tiers**:
   - **First-Citizen Targets (Tier 1)**:
     - **Windows**: Primary reference environment. Windows UI Automation (UIA) and IAccessible2.
     - **macOS**: Native `AXUIElement` / NSAccessibility.
   - **Best-Effort / Nice-to-Have (Tier 2)**:
     - **Linux**: AT-SPI2 / D-Bus.

---

## 2. Accessibility APIs, `xa11y`, & NVDA Cross-Reference

- **Core Accessibility Engine (`xa11y`)**:
  - Use `xa11y` as the primary Rust accessibility abstraction layer for platform accessibility APIs.
  - **Self-Sufficiency**: If `xa11y` lacks an accessibility API, property, pattern, or event hook required by the screen reader, **implement and extend the missing accessibility features directly**.
- **Learning from NVDA**:
   - Agents are encouraged to consult and study the open-source **NVDA** codebase for accessibility APIs, complex text-range normalization, line/word boundaries, formatting runs, and virtual buffer linearization.
  - **Crucial Distinction**: Learn architecture and API interaction from NVDA, but keep the **user-facing speech strings, announcement structures, and keyboard UX matched to JAWS**.
---

## 3. Autonomous Verification Harness: OpenDesk & `old-pc`

When developing or verifying screen reader behavior against canonical JAWS execution:
- **`old-pc` Remote Peer**:
   - Connect to the paired remote peer `old-pc` using OpenDesk (`opendesk-use` / MCP peer routing).
  - `old-pc` hosts a live Freedom Scientific JAWS environment with the Freedom Scientific COM API enabled.
- **Collecting Reference Evidence**:
  - Use the `skills/jaws-reference/` automation skill to send JAWS script calls, keyboard input, query scripts, and retrieve speech history buffers.
  - Save multi-trial evidence in `corpus/evidence/`.

---

## 4. Corpus Lifecycle & 4-Stage Quality Gate

All behavioral scenarios in `corpus/` must adhere to the 4-stage quality gate lifecycle outlined in `corpus/review.txt`:
1. **Stage 1 (Generated)**: Controlled scenario specification and HTML fixture.
2. **Stage 2 (Captured)**: Multi-trial execution against reference screen reader.
3. **Stage 3 (Validated)**: Assertion validation; application-state transitions verified. No unhandled script calls allowed.
4. **Stage 4 (Stability Checked)**: Characterized consistency across repeated trials.

### Key Known Repairs from Auditor Review (`corpus/review.txt`):
- `03-doc-prior-line`: `PriorLine` is not an exposed COM script. Must use `{UP}` keyboard command.
- `07-doc-prior-char`: `PriorCharacter` is not an exposed COM script. Must use `{LEFT}` keyboard command.
- `12-struct-heading-level-2`: `MoveToNextHeadingLevel2` failed. Must use quick key `2`.
- `19-form-mode-entry-exit`: Must explicitly assert Forms mode entry on Enter and exit on Esc.
- `20-form-checkbox-toggle`: Must toggle checkbox via Space and assert state transition.
- `29-dynamic-tab-selection`: Must assert that active tab selection actually changes.

---

## 5. Repository Structure

- `crates/sr-core/`: AOM, virtual cursor, reading commands, mode transitions.
- `crates/sr-platform/`: Platform accessibility bindings (xa11y, UIA, AXUIElement).
- `crates/sr-corpus/`: Test harness, normalizer, differential comparison.
- `crates/sr-cli/`: CLI daemon and regression test binary.
- `corpus/`: Reference speech corpus (manifest, fixtures, scenarios, evidence, auditor review).n
- `skills/jaws-reference/`: Reusable automation skill for JAWS COM and speech extraction.

---

## 6. Build, Check, and Test Workflow

```bash
cargo check
cargo test
cargo run --bin sr-cli -- --help
```
