# Development Plan: Complex ARIA Widgets: ComboBoxes, TreeViews, and Hierarchical Navigation

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 9 – Complex ARIA Widgets: ComboBoxes, TreeViews, and Hierarchical Navigation  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-complex-text-and-web-dialogs.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-complex-text-and-web-dialogs.md)  
**Corpus Evidence**: [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/) (Scenarios `01` – `40` passing with 100% parity)

---

## 1. Goal & Scope

With text selection, formatting runs, and web dialog modal containment verified at 100% parity against JAWS 2025, Milestone 9 tackles advanced interactive ARIA design patterns commonly found in modern web applications:

1. **ARIA ComboBox (`role="combobox"`)**:
   - Detect `Role::ComboBox` with associated popup (`aria-expanded`, `aria-autocomplete`, `aria-haspopup`).
   - Announcements matching JAWS:
     - Entry: `"<Label>  \u{001d} combo box \u{001e} <Value> \u{001d} <collapsed|expanded> \u{001e}"`
     - State transitions: Announce `"expanded"` / `"collapsed"` upon `Alt+DownArrow` / `Escape`.
     - Active-descendant option tracking: Announce highlighted popup options during vertical arrow traversal.

2. **ARIA TreeView (`role="tree"`, `role="treeitem"`)**:
   - Linearize tree structures into the virtual buffer preserving hierarchical nesting levels (`level: u32`).
   - Announcements matching JAWS:
     - Level reporting: `"<Name>  \u{001d} level <N> \u{001e} <collapsed|expanded>"`
     - Expansion/collapse interaction:
       - `RightArrow` on collapsed node: Expands and announces `"expanded"`.
       - `LeftArrow` on expanded node: Collapses and announces `"collapsed"`.
       - `LeftArrow` on collapsed node: Jumps to parent tree item.

3. **ARIA Slider & Range Widgets (`role="slider"`, `role="progressbar"`)**:
   - Value and percentage announcements (`"<Name>  \u{001d} slider \u{001e} <value>"`).
   - Range boundary enforcement (`aria-valuemin`, `aria-valuemax`).

---

## 2. Technical Design

1. **AOM Extensions (`sr-core::aom`)**:
   - Add `Role::ComboBox`, `Role::Tree`, `Role::TreeItem`, `Role::Slider`, `Role::ProgressBar`.
   - Add `level: Option<u32>`, `value_min: Option<f64>`, `value_max: Option<f64>`, `value_now: Option<f64>` to `AomNode`.
2. **Buffer Linearization (`sr-core::buffer`)**:
   - Track hierarchical depth during tree linearization.
   - Maintain interactive widget positions and quick navigation targets.
3. **Speech Formatting (`sr-core::speech`)**:
   - `format_combobox()`, `format_tree_item()`, `format_slider()`.
4. **Interactive Handlers (`sr-core::engine`)**:
   - Tree item expansion/collapse handlers (`RightArrow`, `LeftArrow`).
   - ComboBox toggle and option navigation handlers (`Alt+DownArrow`, `Alt+UpArrow`).
5. **Corpus Scenarios (41–45)**:
   - `41-combobox-interaction`: ComboBox focus, expansion toggle, and option selection.
   - `42-treeview-level-announcement`: TreeView navigation and level announcements.
   - `43-treeview-expand-collapse`: Expand and collapse tree items via Right/Left arrows.
   - `44-slider-value-change`: Slider focus and value adjustment.
   - `45-progressbar-status`: Progress bar state and percentage monitoring.

---

## 3. Implementation Steps

1. Add ARIA widget roles and attributes in `sr-core::aom`.
2. Implement speech formatting helpers in `sr-core::speech`.
3. Update buffer builder and cursor navigation for tree and combobox semantics in `sr-core::buffer` and `sr-core::cursor`.
4. Wire interactive commands into `sr-core::engine`.
5. Author fixture `corpus/fixtures/aria-widgets.html` and `.aom.json`.
6. Author Scenarios 41–45 and verify against canonical evidence in `corpus/evidence/`.
7. Execute full quality gates (`cargo fmt --check`, `cargo clippy`, `cargo test --all`).
