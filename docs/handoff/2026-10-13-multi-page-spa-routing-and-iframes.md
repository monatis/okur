# Milestone Handoff: Multi-Page SPA Routing, In-Page Anchors & Inline Frames

**Handoff Date**: 2026-10-13  
**Milestone Completed**: Milestone 10 – SPA Routing, In-Page Skip Links & Inline Frame (`<iframe>`) Navigation  
**Previous Handoff**: [`docs/handoff/2026-10-13-complex-aria-widgets-combobox-treeview.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-complex-aria-widgets-combobox-treeview.md)  
**Corpus Scenarios**: 50 total, 50 passing (100.0% parity against JAWS 2025)

---

## 1. Summary of Completed Engineering

We implemented complete support for Single Page Application (SPA) client-side routing, in-page fragment jump anchors (skip-to-content links), and inline frame (`<iframe>`) boundary navigation across `sr-core`, `sr-platform`, `sr-corpus`, and `corpus`, strictly validated against Freedom Scientific JAWS 2025:

1. **Accessibility Object Model (AOM) Extensions (`sr-core::aom`)**:
   - Added `Role::IFrame` variant to `Role` enum.
   - Updated `Role::is_block()` to ensure `Role::IFrame` is treated as a block-level boundary element with guaranteed newline separation during virtual buffer linearization.

2. **Speech Formatting Subsystem (`sr-core::speech`)**:
   - `format_frame(name)`: Formats inline frame entry tokens matching JAWS convention (`"{name} frame"` or `"frame"` if unnamed).
   - `format_frame_end(name)`: Formats inline frame exit tokens matching JAWS convention (`"{name} frame end"` or `"frame end"`).
   - `format_page_title_change(new_title)`: Formats SPA route transitions matching JAWS (`"Page has loaded: {new_title}"`).
   - Extended `format_role_and_text()`:
     - Links with fragment hrefs starting with `#` are formatted with JAWS control tokens as `" \u{001d} same page link \u{001e} {text}"`.
     - Preserves existing `"frame"` / `"frame end"` suffixes to prevent duplicate tokens during line announcements.

3. **Virtual Buffer Indexing & In-Page Anchors (`sr-core::buffer`)**:
   - Added `FrameInfo` struct with `node_id` and `name`.
   - Added indexed positions on `VirtualBuffer`:
     - `frames: Vec<FrameInfo>`: Metadata for all frames in the document.
     - `frame_offsets: Vec<usize>`: Quick-nav byte offsets for frame entry points.
     - `target_anchors: HashMap<String, usize>`: Lookup table mapping HTML element IDs and names to virtual buffer byte offsets.
   - Implemented bidirectional frame navigation queries:
     - `next_frame_from()`, `prior_frame_from()`, `first_frame()`, `last_frame()`, `find_frame_node()`.
   - Linearized inline frames with explicit boundaries:
     - Entry line: `"{name} frame"` with `Role::IFrame`.
     - Frame child nodes recursively linearized within inner context.
     - Exit line: `"{name} frame end"` with `Role::Generic`.
   - Linearization formatting:
     - Added `block_depth` tracking to `BufferBuilder` to separate standalone block links from inline list item / paragraph links.

4. **Interactive Navigation & Mechanics (`sr-core::cursor`, `sr-core::engine`, `sr-core::input`)**:
   - Frame Quick Navigation (`M` / `Shift+M`):
     - `M`: Moves forward to next inline frame (`move_next_frame()`). Wraps to top (`"wrapping to top\n{name} frame"`) when past the last frame.
     - `Shift+M`: Moves backward to prior inline frame (`move_prior_frame()`). Wraps to bottom (`"wrapping to bottom\n{name} frame"`) when before the first frame.
   - Same-Page Skip Link Jump (`Enter`):
     - `activate_current_link()` detects `#target-id` hrefs on active links.
     - Looks up target offset in `buffer.target_anchors` and updates cursor position to destination element.
     - Immediately announces destination line content (e.g. `" \u{001d} heading level 2 \u{001e} Main Content Area"`).
   - SPA Route Transitions (`handle_route_transition`):
     - Handles route changes: announces `"Page has loaded: {new_title}"`.
     - When route includes target focus container (e.g. `#main-content`), repositions cursor and announces targeted view.
   - Key Mapping: Added `Key::Alpha('M')` to `input.rs` and bound `M` / `+M` in `engine.execute_command()`.

5. **Differential Verification Harness & Corpus (`sr-corpus`, `corpus/`)**:
   - HTML Fixture: [`corpus/fixtures/spa-and-frames.html`](file:///c:/Users/Gigabyte/screen-reader/corpus/fixtures/spa-and-frames.html) and [`corpus/fixtures/spa-and-frames.aom.json`](file:///c:/Users/Gigabyte/screen-reader/corpus/fixtures/spa-and-frames.aom.json).
   - Scenarios 46 through 50 authored in [`corpus/scenarios/`](file:///c:/Users/Gigabyte/screen-reader/corpus/scenarios/) with reference evidence in [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/):
     - `46-iframe-quicknav`: Quick-nav to inline frame via `M`, verifying frame title announcement.
     - `47-iframe-boundary-exit`: Sequential reading `{DOWN}` through iframe, verifying entry announcement, child content, exit boundary (`"frame end"`), and outer document resumption.
     - `48-inpage-skip-link-jump`: Identification of `"same page link"`, activation via `Enter`, and virtual cursor jump to destination anchor.
     - `49-spa-route-title-change`: SPA client-side route navigation triggering title announcement.
     - `50-spa-focus-shift-main`: SPA route transition with virtual focus shift to primary view container.
   - Extended `runner.rs` to support `Role::IFrame` assertions and `"route_transition"` / `"spa_route"` step types.
   - Added `test_milestone_10_scenarios_46_through_50` in `crates/sr-corpus/src/lib.rs`.
   - Updated [`corpus/CORPUS_MANIFEST.json`](file:///c:/Users/Gigabyte/screen-reader/corpus/CORPUS_MANIFEST.json) to 50 total scenarios (100% pass rate).

---

## 2. Verification & Test Evidence

### Full Corpus Regression (Scenarios 01–50)
```text
============================================================
  Differential Screen Reader Test Suite (JAWS Parity)        
============================================================
[PASS] 01-doc-say-all - Read Entire Document via SayAll (1 steps)
[PASS] 02-doc-next-line - Navigate Next Line via DownArrow (1 steps)
[PASS] 03-doc-prior-line - Navigate Prior Line via UpArrow (2 steps)
[PASS] 04-doc-next-word - Navigate Next Word via Ctrl+RightArrow (1 steps)
[PASS] 05-doc-prior-word - Navigate Prior Word via Ctrl+LeftArrow (2 steps)
[PASS] 06-doc-next-char - Navigate Next Character via RightArrow (1 steps)
[PASS] 07-doc-prior-char - Navigate Prior Character via LeftArrow (2 steps)
[PASS] 08-doc-say-current-line - Speak Current Line (2 steps)
[PASS] 09-doc-say-current-word - Speak Current Word (2 steps)
[PASS] 10-doc-next-paragraph - Navigate Next Paragraph (1 steps)
[PASS] 11-struct-next-heading - Navigate Between Headings (2 steps)
[PASS] 12-struct-heading-level-2 - Navigate Level 2 Headings (2 steps)
[PASS] 13-struct-next-link - Navigate Between Links (4 steps)
[PASS] 14-struct-next-list - Navigate Between Lists (4 steps)
[PASS] 15-struct-next-landmark - Navigate Landmarks (3 steps)
[PASS] 16-struct-next-form-field - Navigate Form Fields (3 steps)
[PASS] 17-struct-next-button - Navigate Buttons (2 steps)
[PASS] 18-struct-next-edit - Navigate Edit Fields (2 steps)
[PASS] 19-forms-mode-toggle - Forms Mode Entry and Exit (3 steps)
[PASS] 20-checkbox-toggle - Checkbox Navigation and State Toggle (2 steps)
[PASS] 21-table-navigation - Enter Table and Read Headers (2 steps)
[PASS] 22-table-cell-right - Navigate Table Cell Right (2 steps)
[PASS] 23-table-cell-left - Navigate Table Cell Left (2 steps)
[PASS] 24-table-row-down - Navigate Table Row Down (2 steps)
[PASS] 25-table-row-up - Navigate Table Row Up (2 steps)
[PASS] 26-tab-stop-navigation - Tab Key Form Control Traversal (4 steps)
[PASS] 27-table-not-found - Table Search Negative Feedback (1 steps)
[PASS] 28-wrap-navigation - Quick Navigation Boundary Wrap (3 steps)
[PASS] 29-dynamic-live-region - ARIA Live Region Mutation Announcement (2 steps)
[PASS] 30-dynamic-hidden-skip - Skipped Hidden and Collapsed Content (4 steps)
[PASS] 31-dialog-confinement - Modal Dialog Focus Trap and Confinement (4 steps)
[PASS] 32-dialog-escape - Modal Dialog Dismissal via Escape (2 steps)
[PASS] 33-tab-cycle-trap - Tab Cycle Trapping within Modal Dialog (5 steps)
[PASS] 34-auto-forms-mode - Automatic Forms Mode on Edit Field Focus (3 steps)
[PASS] 35-live-polite-assertive - Polite vs Assertive Live Region Prioritization (2 steps)
[PASS] 36-text-select-char - Character Selection via Shift+RightArrow (3 steps)
[PASS] 37-text-say-selection - Announcement of Selected Text via Shift+Alt+Down (3 steps)
[PASS] 38-font-query-formatting - Query Text Formatting Attributes via JAWSKey+F (3 steps)
[PASS] 39-multi-range-selection - Selection Across Words and Lines (3 steps)
[PASS] 40-clear-selection - Deselection and Restoration of Virtual Cursor (3 steps)
[PASS] 41-combobox-interaction - ARIA ComboBox Navigation, Expansion, and Selection (4 steps)
[PASS] 42-treeview-level-announcement - ARIA TreeView Level Depth Announcement (3 steps)
[PASS] 43-treeview-expand-collapse - TreeView Expand and Collapse State Changes (3 steps)
[PASS] 44-slider-value-change - ARIA Slider Value Adjustment via Arrow Keys (3 steps)
[PASS] 45-progressbar-status - ARIA ProgressBar Value and Percentage Reading (2 steps)
[PASS] 46-iframe-quicknav - Navigate to Inline Frame via M Quick Navigation (1 steps)
[PASS] 47-iframe-boundary-exit - Inline Frame Traversal and Boundary Exit Announcement (5 steps)
[PASS] 48-inpage-skip-link-jump - In-Page Skip Link Identification and Target Anchor Jump (2 steps)
[PASS] 49-spa-route-title-change - SPA Client-Side Route Transition and Title Update Announcement (1 steps)
[PASS] 50-spa-focus-shift-main - SPA Route Change with Virtual Focus Retargeting to Primary View Content (1 steps)
============================================================
Suite Summary: 50 passed, 0 failed, 0 skipped (100.0% parity)
============================================================
```

### Cargo Quality Gate Results
- `cargo fmt --check`: Clean pass (0 diffs).
- `cargo clippy --all-targets --all-features -- -D warnings`: 0 errors, 0 warnings.
- `cargo test`: 44 tests passed across `sr-core` (26 tests), `sr-corpus` (9 tests), and `sr-platform` (9 tests).
