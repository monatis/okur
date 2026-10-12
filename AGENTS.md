# AGENTS.md - Developer & Autonomous Agent Guide

This repository hosts an open-source, modern, cross-platform screen reader implemented in Rust. The screen reader's user-facing behavior, announcements, keyboard shortcuts, and document interaction patterns are designed to match **Freedom Scientific JAWS 2025**, validated through differential testing.

This is an **autonomous, agent-driven project with a "verification harness in the loop"**.

> [!IMPORTANT]
> **Development Notice – No Backward Compatibility Tricks**:
> This project is in early and fast development. Our ultimate goal is correctness, parity with JAWS, high performance, and real-world usefulness (it should be able to be actually used on Windows and macOS for serious tasks). Until this notice is removed from `AGENTS.md`, **we do not need any backward compatibility tricks or legacy artifacts** in this project. Everything can be changed, updated, or removed cleanly without preserving legacy scaffolding or shims just for the sake of backward compatibility. Implementations must be clean, high-performance, and strictly match JAWS in functionality and behavior.

---

## 1. Mission & Engineering Principles

1. **Parity Through Autonomous Verification ("Verification Harness in the Loop")**:
   - Talk is cheap; demonstrable behavior verified against evidence is ground truth.
   - Every reading command, virtual cursor movement, mode transition, and speech announcement must be verified automatically against canonical transcripts in [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/).
   - The agent autonomously discovers features and behavior of JAWS, chooses a task, implements it, conducts research (online, JAWS documentation, help manuals, JAWS scripts, NVDA repository, OS accessibility API documentation), records reference trials from live JAWS on `old-pc`, and implements code until full functional parity is verified.

2. **User-Facing Parity Target: JAWS**:
   - While internal architecture and algorithms may borrow best practices from other screen readers (e.g., NVDA), **all user-facing behavior must match JAWS**.
   - Keyboard navigation defaults, quick-nav keys (e.g., `H` for headings, `1`–`6` for levels, `K` for links, `C` for checkboxes, `E` for edits), announcement phrasing, and browse/forms mode switching must follow JAWS conventions.

3. **Cross-Platform Target Tiers**:
   - **First-Citizen Targets (Tier 1)**:
     - **Windows**: Primary reference environment. Windows UI Automation (UIA) and IAccessible2.
     - **macOS**: Native `AXUIElement` / NSAccessibility.
   - **Best-Effort / Nice-to-Have (Tier 2)**:
     - **Linux**: AT-SPI2 / D-Bus.

---

## 2. Active Session Plan & Session Lifecycle

To ensure seamless context passing across autonomous sessions, all engineering work is orchestrated through the `docs/` tree:

### Active Plan for Current / Next Session
- **Active Plan**: [`docs/plans/2026-10-13-boundaries-and-dynamic-updates.md`](file:///c:/Users/Gigabyte/screen-reader/docs/plans/2026-10-13-boundaries-and-dynamic-updates.md)
- **Latest Handoff**: [`docs/handoff/2026-10-13-table-navigation-and-grid-movement.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-table-navigation-and-grid-movement.md)


### Standard Session Execution Protocol
Every agent session must follow this exact loop:
1. **Inception & Context Ingestion**:
   - Read this `AGENTS.md` file and inspect the **Active Plan** linked above.
   - Review the prerequisite handoff document in [`docs/handoff/`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/).
2. **Autonomous Research & Discovery**:
   - Investigate JAWS behavior using documentation, scripts (`Default.jss`, `Default.jkm`), the NVDA codebase, or OS accessibility specifications.
   - When necessary, connect to `old-pc` via OpenDesk to test commands or capture new reference evidence.
3. **Implementation & In-Loop Verification**:
   - Implement the planned features in the appropriate crates.
   - Execute differential tests against the reference evidence in `corpus/evidence/`.
4. **Code Quality Gate**:
   - Ensure `cargo fmt --check` passes.
   - Run `cargo clippy --all-targets --all-features -- -D warnings` and verify **0 errors and 0 warnings**.
   - Run `cargo test` and verify all tests pass.
   - *(Note: An automated `Stop` hook in `.agents/hooks.json` enforces formatting and clippy clean passes on loop completion).*
5. **Documentation & Handoff**:
   - Write a dated handoff document in [`docs/handoff/`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/) (referencing previous steps and test results).
   - Author user-facing documentation in [`docs/references/`](file:///c:/Users/Gigabyte/screen-reader/docs/references/) when applicable.
   - Create the next dated plan in [`docs/plans/`](file:///c:/Users/Gigabyte/screen-reader/docs/plans/).
   - Update `AGENTS.md` to point to the newly created plan for the next session.

---

## 3. Autonomous Verification Harness: OpenDesk & `old-pc`

When developing or verifying screen reader behavior against canonical JAWS execution:
- **`old-pc` Remote Peer**:
  - Connect to the paired remote peer `old-pc` using OpenDesk (`opendesk-use` / MCP peer routing).
  - `old-pc` hosts a live Freedom Scientific JAWS environment (`2025.2505.43.400`) with Google Chrome.
- **Collecting Reference Evidence**:
  - Use the `skills/jaws-reference/` automation skill to send JAWS script calls, keyboard input, query scripts, and retrieve speech history buffers.
  - Multi-trial evidence is recorded in `corpus/evidence/` and cataloged in `corpus/CORPUS_MANIFEST.json`.
- **Differential Verification**:
  - Compare the output of the Rust engine with JAWS observations. Parity is defined as semantic equivalence in announcement tokens, state changes, and navigation boundaries.

---

## 4. Accessibility APIs, `xa11y`, & NVDA Cross-Reference

- **Strict Production Standard**:
  - This is a production screen reader project: **NO toy, mocked, or handcrafted custom toy parsers (e.g. ad-hoc HTML parsers) are acceptable**.
  - A screen reader is not simply about parsing and announcing strings; it is an active medium for interacting with the operating system and running applications. All UI consumption must occur through genuine OS accessibility APIs.
- **Core Accessibility Engine (`xa11y`)**:
  - Repository: [`https://github.com/xa11y/xa11y`](https://github.com/xa11y/xa11y)
  - Use `xa11y` as the primary cross-platform accessibility abstraction layer.
  - **Delegation Principle**: If an accessibility feature, query, pattern, or action is covered in `xa11y`, **delegate that work directly to `xa11y`**.
  - **OS-Level Self-Sufficiency**: If `xa11y` lacks an accessibility API, pattern, event hook, or complex text-range support required by the screen reader, **implement and extend it directly at the OS level** (e.g., Windows UI Automation `IUIAutomation`, IAccessible2, Win32 events, macOS `AXUIElement`, Linux AT-SPI2).
- **Learning from NVDA**:
  - Repository: [`https://github.com/nvaccess/nvda`](https://github.com/nvaccess/nvda)
  - Consult the open-source **NVDA** and **`xa11y`** codebases for accessibility API patterns, complex text-range normalization, line/word boundaries, formatting runs, and virtual buffer linearization.
  - **Crucial Distinction**: Learn architecture and OS API interaction from NVDA and `xa11y`, but keep the **user-facing speech strings, announcement structures, and keyboard UX strictly matched to JAWS**.

---

## 5. Documentation Structure (`docs/`)

- `docs/plans/`: Future implementation plans (dated, milestone-oriented).
- `docs/handoff/`: Session handoff documents created after verifying implemented features against JAWS.
- `docs/references/`: User-facing canonical documentation, architecture guides, and API specs.

---

## 6. Repository Structure

- `crates/sr-core/`: AOM, virtual buffer, virtual cursor, reading commands, mode transitions.
- `crates/sr-platform/`: Platform accessibility bindings (xa11y, UIA, AXUIElement).
- `crates/sr-corpus/`: Test harness, normalizer, differential comparison engine.
- `crates/sr-cli/`: CLI daemon and regression test binary.
- `corpus/`: Reference speech corpus (manifest, fixtures, scenarios, evidence, quality report).
- `skills/jaws-reference/`: Reusable automation skill for JAWS COM and speech extraction.
- `.agents/`: Project agent configurations, MCP setup (`mcp_config.json`), and lifecycle hooks (`hooks.json`).

---

## 7. Build, Check, and Quality Workflow

```bash
# Code formatting
cargo fmt --check

# Strict lint check (must have 0 errors, 0 warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Test execution
cargo test

# Run CLI
cargo run --bin sr-cli -- --help
```
