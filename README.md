# Open-Source Rust Screen Reader

A lightweight, modern, open-source, and cross-platform screen reader implemented in Rust.

## Architecture & Crates

This project is organized as a Cargo workspace with dedicated modular crates:

- **crates/sr-core**: Core semantic data structures, Accessibility Object Model (AOM), virtual buffer, cursor navigation, interaction modes (Browse mode vs. Forms mode), and speech presentation abstractions.
- **crates/sr-platform**: OS-level accessibility bridge adapters (Windows UIA/IA2, Linux AT-SPI2, macOS NSAccessibility) and input injection hooks.
- **crates/sr-corpus**: Differential testing engine, JAWS reference corpus parser, transcript normalizer, and 4-stage quality gate evaluator.
- **crates/sr-cli**: Command-line interface and daemon runner for interactive accessibility execution and differential regression testing.

## Corpus & Differential Testing

Reference speech data and behavioral scenarios are located in the `corpus/` directory:
- `corpus/scenarios/`: Declarative JSON test scenarios.
- `corpus/fixtures/`: Synthetic HTML/web test fixtures.
- `corpus/evidence/`: Canonical JAWS 2025 speech transcripts and trial execution recordings.
- `corpus/CORPUS_MANIFEST.json`: Catalog of all recorded scenarios and repeatability status.
- `corpus/review.txt`: Peer auditor review detailing the 4-stage quality gate requirements, invalid scenario script fixes, and semantic assertions.

## Skill Integration

The `skills/jaws-reference/` directory contains the automation skill for Freedom Scientific JAWS COM API control, script inventory discovery, and speech history capture.

## Build & Check

```bash
cargo build
cargo test
cargo check
```
