# Reference: Table Navigation & 2D Grid Movement (JAWS 2025 Parity)

This reference document details table quick navigation keys, 2D cell grid navigation commands, announcement syntax, and virtual buffer linearization behaviors matching Freedom Scientific JAWS 2025.

---

## 1. Table Quick Navigation Keys & JAWS Script Functions

| Key / Command | JAWS Script Function | Target | Direction | Announcement Format |
|---|---|---|---|---|
| `T` | `MoveToNextTable` | Data Table (`Role::Table`) | Forward | `table with {cols} columns and {rows} rows\n{Caption}\nColumn 1, Row 1\n{FirstCellText}` |
| `Shift + T` | `MoveToPriorTable` | Data Table (`Role::Table`) | Prior | `table with {cols} columns and {rows} rows\n{Caption}\nColumn 1, Row 1\n{FirstCellText}` |

### Negative Search
When no tables are present on the active page:
- **Speech**: `"There are no tables on this page."`

---

## 2. 2D Cell Movement Keys (`Ctrl + Alt + Arrow Keys`)

Table cell navigation in Freedom Scientific JAWS uses `Ctrl + Alt` compound keystrokes to navigate along rows and columns while remaining within the virtual buffer.

| Key Keystroke | SendKeys Code | JAWS Script Function | Target Cell | Announcement Format |
|---|---|---|---|---|
| `Ctrl + Alt + RightArrow` | `^%{RIGHT}` | `TableCellRight` / `NextCell` | Cell to right in active row | `{CellText}\nColumn {col} of {cols}` |
| `Ctrl + Alt + LeftArrow` | `^%{LEFT}` | `TableCellLeft` / `PriorCell` | Cell to left in active row | `{CellText}\nColumn {col} of {cols}` |
| `Ctrl + Alt + DownArrow` | `^%{DOWN}` | `TableCellDown` / `NextRow` | Cell directly below in active column | `{CellText}\nRow {row} of {rows}` |
| `Ctrl + Alt + UpArrow` | `^%{UP}` | `TableCellUp` / `PriorRow` | Cell directly above in active column | `{CellText}\nRow {row} of {rows}` |

### Boundary Announcements
When attempting to move beyond the bounds of the table:
- **At column boundaries** (attempting left at column 1 or right at column `total_cols`): `"edge of table"`
- **At row boundaries** (attempting up at row 1 or down at row `total_rows`): `"edge of table"`
- **Outside any table**: `"Not in a table."`

---

## 3. Structural Model & Linearization

1. **`TableGrid`**:
   - Maintains `rows: usize`, `cols: usize`, `caption: Option<String>`, `start_offset: usize`, and `end_offset: usize`.
   - Maps cells as a 2D matrix `cells: Vec<Vec<TableCellInfo>>` where `cells[row_idx][col_idx]` stores `(node_id, offset, row, col, text)`.
2. **Buffer Spans & Lines**:
   - Each `Role::TableCell` generates a `BufferSpan` covering its text slice.
   - Each `Role::TableRow` generates a `BufferSpan`, `BufferLine`, and `BufferParagraph` encompassing all row cells.
   - The `Role::Table` generates an outer `BufferSpan` encompassing all rows.
3. **Cursor State**:
   - `VirtualCursor::current_table_pos`: Tracks active `(table_idx, row_idx, col_idx)`.
   - `VirtualCursor::current_table_col()`: Returns 1-based column coordinate.
   - `VirtualCursor::current_table_row()`: Returns 1-based row coordinate.
