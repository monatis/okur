# Development Plan: Structural Navigation & Quick Navigation Keys Parity

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 2 – Structural Navigation (`11-struct-next-heading` through `15-struct-next-landmark`)  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-basic-document-reading-parity.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-basic-document-reading-parity.md)  
**Corpus Evidence**: [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/) (Scenarios `11` – `15`)  
**Fixture**: [`corpus/fixtures/structural-navigation.html`](file:///c:/Users/Gigabyte/screen-reader/corpus/fixtures/structural-navigation.html)  

---

## 1. Goal & Scope

Extend `sr-core` virtual cursor and buffer navigation to support JAWS quick navigation keys and structural commands:
- `11-struct-next-heading`: `H` key / `MoveToNextHeading` navigation across all heading levels.
- `12-struct-heading-level-2`: `2` key / `MoveToNextHeadingLevelTwo` level-specific filtering.
- `13-struct-next-link`: `U` key (`MoveToNextUnvisitedLink`) / link navigation.
- `14-struct-next-list`: `L` key / `MoveToNextList` list container navigation.
- `15-struct-next-landmark`: `R` / `;` key / `MoveToNextLandmark` landmark boundary navigation.

---

## 2. Technical Design

1. **OS Accessibility Standard & Delegation**:
   - All structural objects (headings, links, lists, landmarks) are resolved from live operating system accessibility trees via **[`PlatformBridge`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-platform/src/bridge.rs)** delegating to **[`xa11y`](https://github.com/xa11y/xa11y)** (Windows UI Automation `IUIAutomation`).
   - For offline CI runs, canonical AOM JSON snapshots are loaded directly without any HTML parsing.
2. **AOM Structural Indices in `VirtualBuffer`**:
   - Maintain indexed lists of node offsets by role in `VirtualBuffer`:
     - `headings: Vec<(usize, u32)>` (offset, level)
     - `links: Vec<usize>` (offset)
     - `lists: Vec<usize>` (offset)
     - `landmarks: Vec<usize>` (offset)
3. **Quick Navigation Primitives in `VirtualCursor`**:
   - `move_next_heading(&mut self) -> Option<String>`
   - `move_prior_heading(&mut self) -> Option<String>`
   - `move_next_heading_level(&mut self, level: u32) -> Option<String>`
   - `move_prior_heading_level(&mut self, level: u32) -> Option<String>`
   - `move_next_link(&mut self) -> Option<String>`
   - `move_next_list(&mut self) -> Option<String>`
   - `move_next_landmark(&mut self) -> Option<String>`
4. **JAWS Speech Announcements for Structures**:
   - Headings: `" \u001d heading level {level} \u001e {text}"`
   - Links: `" \u001d link \u001e {text}"`
   - Lists: `"List of {count} items"`
   - Landmarks: `"{landmark_type} landmark"`
5. **Differential Test Runner Extension**:
   - Wire structural keystrokes (`H`, `2`, `U`, `L`, `R`) and JAWS scripts (`MoveToNextHeading`, `MoveToNextHeadingLevelTwo`, etc.) in `sr-corpus::runner`.

---

## 3. Implementation Steps

1. **Index Structural Nodes in `sr-core::VirtualBuffer`**:
   - Collect indices during linearization for quick binary searches from current cursor offset.
2. **Implement Structural Navigation on `VirtualCursor`**:
   - Support forward/backward structural navigation with wrapping or boundary checks.
3. **Extend `sr-corpus::DifferentialRunner`**:
   - Map scenario commands for scenarios 11–15 to corresponding cursor methods.
4. **Verification**:
   - Run `cargo test -p sr-corpus` to verify scenarios 11–15 pass with 100% parity.
   - Run `sr-cli test --scenarios 11..15`.
   - Ensure `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` pass.
