# Vendored xa11y

These five crates were copied from **xa11y 0.15.2** (<https://github.com/xa11y/xa11y>), MIT
licensed; see [`LICENSE`](LICENSE). They are now part of this project and are extended here as
our cross-platform accessibility API layer. Upstream changes are not tracked automatically.

| Crate | Role |
|---|---|
| `xa11y` | Public facade, plus a CLI (`xa11y` binary) and MCP server for tree inspection |
| `xa11y-core` | Platform-neutral types: `ElementData`, `StateSet`, `Role`, events, selectors |
| `xa11y-windows` | UI Automation backend |
| `xa11y-macos` | AXUIElement backend |
| `xa11y-linux` | AT-SPI2 backend |

## Changes made when importing (2026-10-14)

- The manifests use the original (pre-publish) dependency layout, wired to this workspace with path dependencies. Crate names and the 0.15.2 version are kept as a record of the base.
- `#![allow(clippy::indexing_slicing)]` is set at each crate root and in each integration test, because the workspace bounds-safety gate predates this code.
- Unit tests in `xa11y-windows/src/uia.rs` that query the live desktop's UI Automation tree (`try_provider()`) are `#[ignore = "live: ..."]`, to keep `cargo test` hermetic. Run them with `--ignored`.
- `xa11y-windows/src/screenshot.rs`: `chunks_exact_mut(4)` became `as_chunks_mut::<4>()`, to satisfy a newer clippy lint.

## Hardening ratchet

The pre-commit gate exempts this directory from the zero-`unwrap`/zero-`panic!` rules and instead enforces [`gate-baseline.json`](gate-baseline.json): the counts may only go down. New code must not add unchecked indexing, `unwrap`/`expect` or panicking macros. When hardening a module, remove its violations, lower the baseline, and eventually drop the crate-level `indexing_slicing` allowance.

## Planned extensions (phase 4)

- Screen-reader properties in `ElementData`, added through the `ElementParts` completeness guard so every backend must map them: heading level, landmark type, PositionInSet/SizeOfSet, table cell row/column/spans, ARIA role and properties, live setting, and link URL.
- Cached subtree capture as a first-class provider API, replacing `sr-platform::uia_snapshot`'s parallel UIA code. The snapshot format becomes serialized `ElementData` trees, which gives macOS the same harness.
- The live event pipeline for the screen reader (focus, structure, text, live region and notifications), replacing `sr-platform/src/events/windows.rs`.

## Open items

- The facade's 299 unit tests occasionally take 20–40 s instead of under 1 s (observed 2026-10-14; not reproduced on demand). This is unexplained so far: the slow time is measured inside the test harness, so it isn't binary startup.
