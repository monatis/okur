# Open-Source Rust Screen Reader

A lightweight, modern, open-source, and cross-platform screen reader implemented in Rust, designed for behavioral parity with **Freedom Scientific JAWS 2025**.

This is an autonomous, agent-driven project with an automated **verification harness in the loop**, validated against differential speech and interaction recordings from live JAWS on Windows 10.

---

## Architecture & Crates

This project is organized as a Cargo workspace with modular crates:

- **[`crates/sr-core`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-core)**: Core semantic data structures, Accessibility Object Model (AOM), virtual buffer linearization, cursor navigation, interaction modes (Browse mode vs. Forms mode), and speech presentation formatting.
- **[`crates/sr-platform`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-platform)**: OS-level accessibility bridge adapters (Windows UIA/IA2, Linux AT-SPI2, macOS NSAccessibility) and input injection hooks.
- **[`crates/sr-corpus`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-corpus)**: Differential testing engine, JAWS reference corpus parser, transcript normalizer, and 4-stage quality gate evaluator.
- **[`crates/sr-cli`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-cli)**: Command-line interface and daemon runner for interactive accessibility execution and differential regression testing.

---

## Corpus & Differential Testing

Reference speech data and behavioral scenarios are located in the [`corpus/`](file:///c:/Users/Gigabyte/screen-reader/corpus/) directory:
- `corpus/scenarios/`: Declarative JSON test scenarios.
- `corpus/fixtures/`: Synthetic HTML/web test fixtures.
- `corpus/evidence/`: Canonical JAWS 2025 speech transcripts and trial execution recordings (150/150 passed trials).
- `corpus/CORPUS_MANIFEST.json`: Machine-readable catalog of all recorded scenarios, environment metadata, and fixture SHA-256 hashes.
- `corpus/quality-report.md`: Quality gate validation report certifying 100% trial success and zero unhandled script errors.
- `corpus/evidence-v1.0.0/`: Preserved archive of initial pre-audit reference traces.

---

## Documentation & Session Workflow (`docs/`)

Engineering sessions are documented continuously in `docs/`:
- **`docs/plans/`**: Milestone implementation plans for upcoming sessions.
- **`docs/handoff/`**: Dated session handoff summaries with verification evidence.
- **`docs/references/`**: User-facing documentation and architectural specifications.

See [`AGENTS.md`](file:///c:/Users/Gigabyte/screen-reader/AGENTS.md) for the active session plan and agent development protocol.

---

## Build, Test & Quality Gates

```bash
# Code formatting check
cargo fmt --check

# Strict lint check (must pass with 0 warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Run all test suites
cargo test

# Run screen reader CLI
cargo run --bin sr-cli -- --help
```
