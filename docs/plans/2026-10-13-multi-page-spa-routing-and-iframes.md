# Development Plan: SPA Routing, Frame / IFrame Navigation, and In-Page Anchor Transitions

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 10 – SPA Client-Side Routing, Frame / IFrame Navigation, and In-Page Anchor Transitions  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-complex-aria-widgets-combobox-treeview.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-complex-aria-widgets-combobox-treeview.md)  
**Corpus Evidence**: [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/) (Scenarios `01` – `45` passing with 100% parity)

---

## 1. Goal & Scope

With complex ARIA widgets (ComboBoxes, hierarchical TreeViews, Sliders, and ProgressBars) verified at 100% parity against JAWS 2025, Milestone 10 addresses real-world multi-frame and dynamic single-page web application (SPA) architectures:

1. **Frames and IFrames (`<iframe>`, `<frame>`, `role="region"`, `Role::IFrame`)**:
   - Linearize iframe embedded documents into virtual buffers with distinct frame boundaries.
   - Quick Navigation via `M` / `Shift+M` (`MoveToNextFrame` / `MoveToPriorFrame`).
   - Frame boundary announcements matching JAWS 2025:
     - Entry: `"Inline Frame <Title|Name>"`
     - Exit: Seamless return to host document or `"Leaving Frame"`.
2. **In-Page Anchor & Skip Navigation ("Skip to content")**:
   - Detection of internal same-page fragment links (`<a href="#id">`).
   - Activation via `Enter` jumps virtual cursor directly to target node ID.
   - Announcement of target element role, name, and heading level upon arrival.
3. **SPA Client-Side Route Transitions**:
   - Dynamic document title modification (`document.title` updates) triggering title speech announcements:
     - `"Page has loaded: <Title>"` or `"<Title>"` depending on virtual cursor focus state.
   - Virtual buffer reload / subtree invalidation on router view swapping.
   - Smooth virtual cursor retargeting when route focus moves to `<main>` or primary `<h1>`.

---

## 2. Technical Design

1. **AOM Extensions (`sr-core::aom`)**:
   - Add `Role::IFrame` (or `Role::Frame`).
   - Support `target_id: Option<String>` on link nodes for fragment anchor targets.
   - Support embedded sub-documents within frame nodes.
2. **Buffer Linearization & Frame Indexing (`sr-core::buffer`)**:
   - Track `frames: Vec<usize>` in `VirtualBuffer`.
   - Implement `next_frame_from()` and `prior_frame_from()`.
   - Store frame boundary intervals (`start_offset`, `end_offset`, `title`).
3. **Cursor & Interactive Navigation (`sr-core::cursor`, `sr-core::engine`)**:
   - Add `move_next_frame()` and `move_prior_frame()` triggered by `Key::Alpha('M')`.
   - Implement link activation (`activate_current_link()` / `activate_link_target()`):
     - Resolve fragment `#id` against node IDs in the buffer and move cursor directly to the destination.
   - Implement route transition handler:
     - `handle_route_transition(new_title: &str, target_node_id: Option<&str>)`.
4. **Speech Formatting (`sr-core::speech`)**:
   - `format_frame()`: Formats frame enter/exit announcements.
   - `format_page_title()`: Formats SPA route change announcements.
5. **Differential Verification Harness & Corpus (`sr-corpus`, `corpus/`)**:
   - Fixture: `corpus/fixtures/spa-and-frames.html` and `.aom.json`.
   - Author Scenarios 46–50 in `corpus/scenarios/` and canonical evidence in `corpus/evidence/`:
     - `46-iframe-quicknav`: Navigate to frame using `M` and announce `"Inline Frame <name>"`.
     - `47-iframe-boundary-exit`: Traverse past frame content and resume outer document.
     - `48-inpage-skip-link-jump`: Press `Enter` on skip link and land on main content heading.
     - `49-spa-route-title-change`: Simulate route update and verify page title announcement.
     - `50-spa-focus-shift-main`: Route change with focus shift to main landmark.

---

## 3. Implementation Steps

1. Add `Role::IFrame` and link target resolution in `sr-core::aom`.
2. Add frame boundaries, `M` quick navigation, and anchor jumping in `sr-core::buffer` and `sr-core::cursor`.
3. Add route transition and dynamic title handling in `sr-core::engine`.
4. Author fixture `corpus/fixtures/spa-and-frames.html` and `corpus/fixtures/spa-and-frames.aom.json`.
5. Author scenarios 46–50 in `corpus/scenarios/` and reference evidence in `corpus/evidence/`.
6. Update `corpus/CORPUS_MANIFEST.json` and integration tests in `sr-corpus`.
7. Verify 100% parity across all 50 scenarios and execute full quality gate (`cargo fmt --check`, `cargo clippy`, `cargo test`).
