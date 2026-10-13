# Development Plan: Complex Text Selection, Font Formatting Queries & Web Dialog Modal Traps

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 8 – Complex Text Selection, Font Formatting Queries & Web Dialog Modal Traps  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-live-focus-and-aria-live.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-live-focus-and-aria-live.md)  
**Corpus Evidence**: [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/) (Scenarios `01` – `35` passing with 100% parity)

---

## 1. Goal & Scope

With live OS focus tracking, automatic forms mode entry/exit, and ARIA live regions verified at 100% parity, Milestone 8 equips the screen reader with advanced document reading capabilities and complex web application containment:

1. **Text Selection & Cursor Markers**:
   - Support extended selection commands (`Shift+RightArrow`, `Shift+LeftArrow`, `Shift+DownArrow`, `Shift+UpArrow`).
   - Emit canonical selection feedback matching JAWS 2025:
     - Selecting: `"selected <text>"`
     - Unselecting (collapsing selection): `"unselected <text>"`
   - Implement `SaySelection` query command (`JAWSKey+Shift+DownArrow`) to speak the entire active selection or report `"No selection"` if empty.

2. **Font & Text Formatting Query (`Insert+F`)**:
   - Implement JAWS `WhatFont` / formatting inspection shortcut (`Insert+F` / `Caps+F`).
   - Query inline text run attributes from AOM and platform accessibility elements:
     - Font family (e.g. `Arial`, `Times New Roman`)
     - Font size in points (e.g. `12 pt`, `16 pt`)
     - Style attributes (e.g. `bold`, `italic`, `underline`)
     - Text alignment and colors when specified.
   - Synthesize canonical JAWS formatting description strings:
     - e.g. `"16 pt, bold, Arial on default background, left aligned"`

3. **Web Dialogs & Modal Confinement (`role="dialog"`, `aria-modal="true"`)**:
   - Recognize modal boundaries (`Role::Dialog`, `Role::AlertDialog`, and `aria-modal="true"`).
   - Dialog entry announcement: Announce `"Dialog <Title>"` when focus or cursor enters a modal dialog.
   - Virtual cursor modal trap: When trapped in a modal, boundary wrapping and quick navigation keys (`H`, `K`, `F`, `Tab`, arrow keys) remain strictly confined to the dialog's subtree and cannot escape into inactive backdrop nodes.
   - Dialog exit announcement: When dismissing or exiting the modal dialog, announce `"Leaving Dialog"` and restore global virtual buffer navigation scope.

---

## 2. Technical Design

1. **Selection Subsystem (`sr-core::cursor`)**:
   - Add selection anchor and active cursor offsets to `VirtualCursor`:
     ```rust
     pub struct TextSelection {
         pub anchor: usize,
         pub active: usize,
     }
     ```
   - Add selection movement methods: `select_next_char()`, `select_prior_char()`, `select_next_line()`, `select_prior_line()`, `say_selection()`.
   - Compute delta strings and format `"selected <str>"` / `"unselected <str>"`.

2. **Formatting Run Subsystem (`sr-core::aom` & `sr-core::cursor`)**:
   - Extend `AomNode` with optional `formatting`:
     ```rust
     pub struct TextFormatting {
         pub font_family: Option<String>,
         pub font_size_pt: Option<f32>,
         pub bold: bool,
         pub italic: bool,
         pub underline: bool,
         pub alignment: Option<String>,
     }
     ```
   - Implement formatting query handler `say_font_formatting()` producing JAWS-formatted speech.

3. **Modal Dialog Subtree Confinement (`sr-core::buffer` & `sr-core::cursor`)**:
   - Track active modal boundary root node ID (`active_modal_root: Option<String>`).
   - Filter virtual buffer linearized navigation to nodes within the subtree of `active_modal_root`.
   - Prevent quick-nav and line movement from escaping the dialog container.

4. **Differential Verification Harness (`corpus/`)**:
   - Fixture: `corpus/fixtures/dialog-and-formatting.html` and `.aom.json`.
   - Author Scenarios 36–40:
     - `36-text-select-char.json`: Character selection and unselection via `Shift+Arrows`.
     - `37-text-say-selection.json`: Query active selection text via `SaySelection`.
     - `38-font-query-formatting.json`: Query font formatting runs via `Insert+F`.
     - `39-dialog-entry-modal-trap.json`: Enter modal dialog and constrain navigation within boundaries.
     - `40-dialog-exit-restore.json`: Exit modal dialog and restore full buffer navigation.
   - Record and verify canonical evidence in `corpus/evidence/`.

---

## 3. Implementation Steps

1. **Selection State in `sr-core`**:
   - Add selection tracking to `VirtualCursor`.
   - Implement selection commands in `execute_command`.
2. **Text Formatting in `sr-core`**:
   - Add formatting fields to `AomNode` and linearize formatting spans into `VirtualBuffer`.
   - Implement `Insert+F` formatting query command.
3. **Modal Dialog Confinement in `sr-core`**:
   - Add modal scope filtering in `VirtualCursor` traversal.
4. **Author Corpus Scenarios (36–40)**:
   - Create fixtures, scenario JSONs, and evidence files.
   - Run differential verification against JAWS 2025.
5. **Quality Gate**:
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all`
