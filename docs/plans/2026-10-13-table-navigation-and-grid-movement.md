# Development Plan: Table Navigation & 2D Grid Movement Parity

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 4 – Table Navigation & 2D Grid Movement (`21-table-next-table` through `25-table-row-up`)  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-forms-and-interaction-modes.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-forms-and-interaction-modes.md)  
**Corpus Evidence**: [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/) (Scenarios `21` – `25`)  
**Fixture**: [`corpus/fixtures/tables-navigation.html`](file:///c:/Users/Gigabyte/screen-reader/corpus/fixtures/tables-navigation.html)  

---

## 1. Goal & Scope

Extend `sr-core` virtual cursor and buffer navigation to support JAWS table quick navigation and 2D cell grid navigation:
- `21-table-next-table`: `T` key / `MoveToNextTable` navigation to data tables with dimension and summary announcement.
- `22-table-cell-right`: `Ctrl+Alt+RightArrow` (`^%{RIGHT}`) / move to cell to the right within the active row, announcing content and column position (`"Column {col} of {total_cols}"`).
- `23-table-cell-left`: `Ctrl+Alt+LeftArrow` (`^%{LEFT}`) / move to cell to the left within the active row.
- `24-table-row-down`: `Ctrl+Alt+DownArrow` (`^%{DOWN}`) / move to cell directly below in the active column, announcing content and row position (`"Row {row} of {total_rows}"`).
- `25-table-row-up`: `Ctrl+Alt+UpArrow` (`^%{UP}`) / move to cell directly above in the active column.

---

## 2. Technical Design

1. **Table Model in `VirtualBuffer`**:
   - `TableGrid`:
     - `table_id: usize`
     - `rows: usize`, `cols: usize`
     - `caption: Option<String>`
     - `cells: Vec<Vec<TableCellInfo>>` (2D grid mapping `(row_idx, col_idx)` to `TableCellInfo { node_id, offset, text }`)
   - `tables: Vec<TableGrid>` and `table_offsets: Vec<usize>` maintained during buffer linearization.
2. **2D Grid Navigation Primitives in `VirtualCursor`**:
   - `current_table_pos: Option<(usize, usize, usize)>` (table_idx, row_idx, col_idx).
   - `move_next_table(&mut self) -> Option<String>` / `move_prior_table(&mut self) -> Option<String>` (`T` / `+T`).
   - `move_table_cell_right(&mut self) -> Option<String>` (`Ctrl+Alt+RightArrow`).
   - `move_table_cell_left(&mut self) -> Option<String>` (`Ctrl+Alt+LeftArrow`).
   - `move_table_cell_down(&mut self) -> Option<String>` (`Ctrl+Alt+DownArrow`).
   - `move_table_cell_up(&mut self) -> Option<String>` (`Ctrl+Alt+UpArrow`).
3. **JAWS 2025 Table Speech Formatters**:
   - Table entry: `"table with {cols} columns and {rows} rows\n{Caption}\nColumn 1, Row 1\n{FirstCellText}"`.
   - Cell navigation horizontal: `"{CellText}\nColumn {col} of {cols}"`.
   - Cell navigation vertical: `"{CellText}\nRow {row} of {rows}"`.
4. **Fixture**:
   - Generate canonical `corpus/fixtures/tables-navigation.aom.json` mirroring `tables-navigation.html`.

---

## 3. Implementation Steps

1. **Create Canonical AOM JSON Fixture**:
   - Create `corpus/fixtures/tables-navigation.aom.json`.
2. **Build Table 2D Grid Structure in `sr-core`**:
   - Traverse `Role::Table`, `Role::TableRow`, `Role::TableCell` in `BufferBuilder` and construct `TableGrid`.
3. **Implement 2D Movement in `VirtualCursor`**:
   - Add table navigation methods and cursor coordinate updates.
4. **Wire Differential Runner for Scenarios 21–25**:
   - Map `^%{RIGHT}`, `^%{LEFT}`, `^%{DOWN}`, `^%{UP}`, and `MoveToNextTable`.
5. **Verification**:
   - Run `sr-cli test --scenarios 21..25` and confirm 100% parity against canonical evidence.
   - Run quality gates (`cargo fmt --check`, strict clippy, `cargo test --all`).
