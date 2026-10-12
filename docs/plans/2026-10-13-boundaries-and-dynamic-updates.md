# Development Plan: Boundaries & Dynamic Updates Parity

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 5 – Boundaries & Dynamic Updates (`26-boundary-heading-wrap` through `30-dynamic-hidden-skip`)  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-table-navigation-and-grid-movement.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-table-navigation-and-grid-movement.md)  
**Corpus Evidence**: [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/) (Scenarios `26` – `30`)  
**Fixture**: [`corpus/fixtures/boundaries-dynamic.html`](file:///c:/Users/Gigabyte/screen-reader/corpus/fixtures/boundaries-dynamic.html)  

---

## 1. Goal & Scope

Extend `sr-core` virtual cursor and buffer navigation to support JAWS boundary wrapping, negative search announcements, interactive Tab navigation, ARIA tab selection, and skipping hidden DOM subtrees:
- `26-boundary-heading-wrap`: `H` key / `MoveToNextHeading` wrap announcement when reaching end of document (`"wrapping to top"`).
- `27-boundary-no-tables`: `T` key / `MoveToNextTable` negative search announcement (`"There are no tables on this page."`).
- `28-dynamic-tab-navigation`: `{TAB}` keyboard traversal across focusable controls (`Button`, `Tab`).
- `29-dynamic-tab-selection`: `{TAB}` traversal into tablist, `{SPACE}` tab selection, and `"selected"` state announcement.
- `30-dynamic-hidden-skip`: Sequential line reading (`{DOWN}`) skips elements marked with `aria-hidden="true"` or `hidden`.

---

## 2. Technical Design

1. **Heading Boundary Wrap in `VirtualCursor`**:
   - When `move_next_heading()` reaches the end of the document, wrap to the first heading and prefix speech with `"wrapping to top\n"`.
   - When `move_prior_heading()` reaches the beginning, wrap to the last heading and prefix speech with `"wrapping to bottom\n"`.
2. **Negative Search Announcement**:
   - When quick-nav commands (`T`, `L`, `F`, etc.) find no targets in the document, announce negative search matching JAWS:
     - Tables: `"There are no tables on this page."`
     - Headings: `"There are no headings on this page."`
     - Lists: `"There are no lists on this page."`
3. **Interactive Control Tab Navigation (`{TAB}` / `Shift+{TAB}`)**:
   - Maintain focusable elements list in `VirtualBuffer`: buttons, links, edits, checkboxes, tabs.
   - `move_next_tab_stop()` / `move_prior_tab_stop()`.
   - Announce control name, role, and states (e.g. `"System Details Button"`, `"General Tab selected"`, `"Advanced Tab"`).
4. **ARIA Tab Selection (`Spacebar`)**:
   - `toggle_active_control()` / `select_active_control()`: when active control has `Role::Tab`, mark `selected: true` (and other sibling tabs `selected: false`) and announce `"selected"`.
5. **Aria-Hidden Subtree Filtering**:
   - In `BufferBuilder::traverse_node`: skip any node whose `states.hidden` is true or has `aria-hidden="true"`.
6. **Fixture**:
   - Create canonical `corpus/fixtures/boundaries-dynamic.aom.json` mirroring `boundaries-dynamic.html`.

---

## 3. Implementation Steps

1. **Create Canonical AOM JSON Fixture**:
   - Generate `corpus/fixtures/boundaries-dynamic.aom.json`.
2. **Implement Boundary Wrapping in `VirtualCursor`**:
   - Update `move_next_heading()` and other structural navigation primitives to support wrapping announcements.
3. **Implement Tab Navigation & Selection in `VirtualCursor`**:
   - Implement `move_next_tab_stop()` / `move_prior_tab_stop()`.
   - Implement `Role::Tab` formatting in `SpeechFormatter`.
   - Implement tab selection logic on `Spacebar`.
4. **Wire Differential Runner for Scenarios 26–30**:
   - Map `{TAB}`, `+TAB` in `runner.rs`.
   - Add `target_selected` assertions to `runner.rs`.
5. **Verification**:
   - Run `sr-cli test --scenarios 01..30` and verify 30/30 (100% parity).
   - Ensure all quality gates (`cargo fmt --check`, `cargo clippy`, `cargo test`) pass cleanly.
