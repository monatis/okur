# Implementation Plan: Milestone 12 – Complex Table Headers, Coordinate Reading & Multi-Span Navigation

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 12  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-virtual-buffer-search-and-element-lists.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-virtual-buffer-search-and-element-lists.md)  
**Parity Target**: Freedom Scientific JAWS 2025 Differential Behavior  

---

## 1. Overview & Objective

In this milestone, we will extend the table navigation subsystem to support full JAWS 2025 parity for complex 2D data tables:
1. **Explicit & Inferred Header Association**: Associating column and row headers (`<th>` with `scope="col"`, `scope="row"`, or `headers` IDs) with data cells (`<td>`).
2. **Cell Coordinate Announcements**: Including row/column header context when traversing cells (`"Column {col_hdr} Row {row_hdr}: {cell_text}"`).
3. **Table Navigation Shortcuts**: Supporting full JAWS table layer commands:
   - `Ctrl+Alt+RightArrow` / `Ctrl+Alt+LeftArrow`: Next/prior column in current row.
   - `Ctrl+Alt+DownArrow` / `Ctrl+Alt+UpArrow`: Next/prior row in current column.
   - `Ctrl+Alt+Home`: Jump to first cell of table (`r1c1`).
   - `Ctrl+Alt+End`: Jump to last cell of table (`rNcM`).
   - `Ctrl+Alt+Shift+H`: Speak current column header without moving.
   - `Ctrl+Alt+Shift+R`: Speak current row header without moving.
4. **Spanning Cell Semantics**: Handling `colspan` and `rowspan` with JAWS announcement tokens (`"spans {n} columns"`, `"spans {n} rows"`).
5. **Differential Corpus Scenarios**: Authoring and executing Scenarios 56 through 60 against live JAWS 2025 on `old-pc`.

---

## 2. Technical Architecture & Component Changes

### 2.1 Core Subsystem (`crates/sr-core`)

- **Table Grid & Header Association (`buffer.rs`)**:
  - Enhance `TableCellInfo` to store:
    - `is_header: bool`
    - `col_span: usize`, `row_span: usize`
    - `col_header: Option<String>`, `row_header: Option<String>`
    - `headers_attr: Option<Vec<String>>`
  - Enhance `TableGrid` resolution:
    - Map `colspan` and `rowspan` to populate 2D cell coordinate matrix across spanned positions.
    - Resolve explicit `headers` attribute links by referencing header cell IDs.
    - Infer column headers from topmost row(s) and row headers from leftmost column(s).

- **Speech Formatting (`speech.rs`)**:
  - `format_table_cell_with_headers(row, col, col_hdr, row_hdr, cell_text, col_span, row_span)` matching JAWS format:
    `"{ColHdr} {CellText}\ncolumn {col} row {row}"` or with span announcements.
  - `format_column_header(col_hdr, col)`: Announcement for `Ctrl+Alt+Shift+H`.
  - `format_row_header(row_hdr, row)`: Announcement for `Ctrl+Alt+Shift+R`.

- **Cursor Navigation Commands (`cursor.rs`, `engine.rs`, `input.rs`)**:
  - Implement `move_table_first_cell()`, `move_table_last_cell()`, `read_current_col_header()`, `read_current_row_header()`.
  - Bind keyboard combinations in `input.rs`:
    - `^!{HOME}` (`Ctrl+Alt+Home`)
    - `^!{END}` (`Ctrl+Alt+End`)
    - `^!+{H}` (`Ctrl+Alt+Shift+H`)
    - `^!+{R}` (`Ctrl+Alt+Shift+R`)

---

## 3. Autonomous Verification Plan (`old-pc` & Corpus)

1. **Remote Research & Reference Capture**:
   - Create HTML fixture with complex data tables (headers, colspans, rowspans) or use existing `tables-navigation.html`.
   - Use `skills/jaws-reference/` and OpenDesk to capture live JAWS 2025 speech transcripts for:
     - Navigation with column/row header announcements.
     - `Ctrl+Alt+Home` and `Ctrl+Alt+End`.
     - `Ctrl+Alt+Shift+H` / `Ctrl+Alt+Shift+R`.
     - Spanning cell announcements.
2. **Corpus Scenarios & Evidence (56–60)**:
   - Scenario 56: Table cell navigation with explicit column and row headers.
   - Scenario 57: Jump to first cell (`Ctrl+Alt+Home`) and last cell (`Ctrl+Alt+End`).
   - Scenario 58: Direct column header query (`Ctrl+Alt+Shift+H`).
   - Scenario 59: Direct row header query (`Ctrl+Alt+Shift+R`).
   - Scenario 60: Multi-span cell traversal (`colspan` / `rowspan`).
3. **Execution & Regression Testing**:
   - Run `cargo test` across all 60 scenarios.
   - Verify `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`.
