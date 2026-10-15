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

## Hardening

**Production code may not panic.** Every crate's `Cargo.toml` denies `clippy::unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` and `unreachable`; `xa11y-core`, `xa11y-windows` and `xa11y` also deny `indexing_slicing`. Tests are exempt: unit tests through [`clippy.toml`](clippy.toml) (`allow-*-in-tests`), integration tests through a crate-level `#![allow]`, and two unit-test modules that use `unreachable!()` through `#[allow(clippy::unreachable)]`. Deliberate exceptions carry an `#[allow(..., reason = "...")]`:
- `xa11y-core::role::unknown_role` panics under the test-only `strict-roles` feature;
- `xa11y-core::mock` (the `test-support` feature) indexes its fixed tree.

The non-native stubs (`WindowsProvider` off Windows, the macOS and Linux providers elsewhere) return `Error::Unsupported` or `Error::Platform` instead of `unreachable!()`: they can be built through `Default`, so their methods are reachable.

**Still to do:** `xa11y-macos` and `xa11y-linux` keep a crate-level `#![allow(clippy::indexing_slicing)]`. Their native code compiles only on their own platforms, so remove the allowance and fix what clippy reports there. Clippy has not been run on macOS or Linux since these lints were added (2026-10-15). A grep found no `unwrap`/`expect`/panicking macro outside their test modules, but clippy on those platforms is the check.

**The line-count ratchet** in [`gate-baseline.json`](gate-baseline.json) still runs in the pre-commit gate: it counts every non-comment line in this directory, tests included, and the counts may only go down. It measures mostly test code now; the lints above are what guard production code.

## Planned extensions (phase 4)

- Screen-reader properties in `ElementData`, added through the `ElementParts` completeness guard so every backend must map them: heading level, landmark type, PositionInSet/SizeOfSet, table cell row/column/spans, ARIA role and properties, live setting, and link URL.
- Cached subtree capture as a first-class provider API, replacing `sr-platform::uia_snapshot`'s parallel UIA code. The snapshot format becomes serialized `ElementData` trees, which gives macOS the same harness.
- The live event pipeline for the screen reader (focus, structure, text, live region and notifications), replacing `sr-platform/src/events/windows.rs`.

## Open items

- The facade's 299 unit tests occasionally take 20–40 s instead of under 1 s (observed 2026-10-14; not reproduced on demand). This is unexplained so far: the slow time is measured inside the test harness, so it isn't binary startup.
