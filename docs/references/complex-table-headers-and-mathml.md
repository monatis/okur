# Reference: Complex Table Headers, Coordinates & MathML Linearization

**Status**: Verified & Implemented in `sr-core`  
**Parity Target**: Freedom Scientific JAWS 2025  

---

## 1. Overview & Architecture

Modern web content and documents contain complex 2D tables (multi-level column headers, row headers, spanning cells `colspan` and `rowspan`, and explicit `headers` attribute linkages) as well as mathematical expressions rendered with MathML (`<math>`, `<mfrac>`, `<msqrt>`, `<mroot>`, `<msup>`, `<msub>`, `<mo>`, `<mi>`, `<mn>`).

In this screen reader architecture, table navigation and MathML processing are integrated into `VirtualBuffer`, `AomNode`, and `VirtualCursor`, achieving user-facing parity with JAWS 2025.

---

## 2. Table Header Resolution & Coordinate Tracking

### 2.1 Multi-Dimensional Grid Representation
Tables are linearized into `VirtualBuffer` lines with semantic table annotations, while maintaining an underlying `TableGrid` representation:
- **Matrix Mapping**: Each cell occupies a bounding rectangle `[row, row + row_span) x [col, col + col_span)` in the 0-indexed coordinate space.
- **Span Traversal**: Moving right (`Ctrl+Alt+RightArrow`) or down (`Ctrl+Alt+DownArrow`) advances by `cell.col_span` or `cell.row_span`, seamlessly skipping phantom grid positions occupied by the span.

### 2.2 Header Association Logic (Cross-Referenced with NVDA & JAWS)
1. **Explicit Header Associations (`headers` attribute)**:
   - If a cell has an HTML/ARIA `headers` attribute specifying ID tokens, these tokens are looked up in the table's cell dictionary.
   - All referenced header cell strings are joined in sequence (`col_h = Some(explicit_hdrs.join(" "))`).
2. **Inferred Column Headers**:
   - For cells without an explicit column header, the grid is scanned upwards in column `c` from row `0` to `r - 1`.
   - Headers marked with `scope="col"`, `scope="colgroup"`, `role="columnheader"`, or `<th>` on row 0 are accumulated.
   - The most immediate/specific sub-header (`col_hdrs.last()`) is selected when navigating horizontally across columns.
3. **Inferred Row Headers**:
   - For cells without an explicit row header, the grid is scanned leftwards in row `r` from column `0` to `c - 1`.
   - Headers marked with `scope="row"`, `scope="rowgroup"`, `role="rowheader"`, or `<th>` on column 0 are accumulated.
   - The most immediate/specific row header (`row_hdrs.last()`) is selected when navigating vertically across rows.

### 2.3 User-Facing Speech Announcements (JAWS 2025 Parity)
- **Horizontal Move (`Ctrl+Alt+Right` / `Ctrl+Alt+Left`)**:
  `"{ColHeader} \n{CellText}\nColumn {col} of {total_cols}"`
- **Vertical Move (`Ctrl+Alt+Down` / `Ctrl+Alt+Up`)**:
  `"{RowHeader}\n{CellText}\nRow {row} of {total_rows}"`
- **Jump to First Cell (`Ctrl+Alt+Home`)**:
  Moves to row 1, col 1. Does not repeat cell text if the cell is itself a header:
  `"{CellText}\nColumn 1, Row 1"`
- **Jump to Last Cell (`Ctrl+Alt+End`)**:
  Announces column header and row header separated by two spaces, followed by cell text and coordinates:
  `"{ColHeader}  {RowHeader}\n{CellText}\nColumn {col}, Row {row}"`
- **Query Column Header (`Ctrl+Alt+Shift+H`)**:
  `"{ColHeader} column {col}"` (or `"no column header"` if none exists).
- **Query Row Header (`Ctrl+Alt+Shift+R`)**:
  `"{RowHeader} row {row}"` (or `"no row header"` if none exists).
- **Boundary Notifications**:
  - `Ctrl+Alt+Shift+Left`: `"beginning of row\n{CellText}"`
  - `Ctrl+Alt+Shift+Right`: `"end of row\n{CellText}"`
  - `Ctrl+Alt+Shift+Up`: `"top of column\n{CellText}"`
  - `Ctrl+Alt+Shift+Down`: `"bottom of column\n{CellText}"`

---

## 3. MathML Linearization & Quick-Nav Navigation

### 3.1 Role & AST Linearization
- Math containers (`<math role="math">`) are classified with `Role::Math`.
- Linearization parses MathML AST nodes recursively into standard mathematical speech strings:
  - `<mfrac>`: `"fraction {numerator} over {denominator} end fraction"`
  - `<msqrt>`: `"square root of {radicand} end root"`
  - `<mroot>`: `"{index} root of {radicand} end root"`
  - `<msup>`:
    - Exponent 2: `"{base} squared"`
    - Exponent 3: `"{base} cubed"`
    - General exponent: `"{base} to the power of {exponent}"`
  - `<msub>`: `"{base} sub {subscript}"`
  - `<mo>`: Standardized verbalization (`+` -> `"plus"`, `-` -> `"minus"`, `=` -> `"equals"`, `±` -> `"plus or minus"`, `×` -> `"times"`, `/` -> `"divided by"`).
  - `<mi>`, `<mn>`: Direct text representation.

### 3.2 Quick-Navigation (`O` / `Shift+O`) & Math Viewer (`Enter`)
- `O`: Move to next mathematical expression (`"math {expression}"`).
- `Shift+O`: Move to prior mathematical expression (`"math {expression}"`).
- `Enter` on a Math expression: Enters MathML interactive viewer mode (`"Math Viewer: {expression}"`).

---

## 4. Keyboard Shortcuts Reference

| Shortcut | Command | Announcement Pattern |
| :--- | :--- | :--- |
| `Ctrl+Alt+Right` | Next Column | `{ColHeader} \n{CellText}\nColumn {c} of {M}` |
| `Ctrl+Alt+Left` | Prior Column | `{ColHeader} \n{CellText}\nColumn {c} of {M}` |
| `Ctrl+Alt+Down` | Next Row | `{RowHeader}\n{CellText}\nRow {r} of {N}` |
| `Ctrl+Alt+Up` | Prior Row | `{RowHeader}\n{CellText}\nRow {r} of {N}` |
| `Ctrl+Alt+Home` | First Cell | `{CellText}\nColumn 1, Row 1` |
| `Ctrl+Alt+End` | Last Cell | `{ColHeader}  {RowHeader}\n{CellText}\nColumn {c}, Row {r}` |
| `Ctrl+Alt+Shift+H` | Read Column Header | `{ColHeader} column {c}` |
| `Ctrl+Alt+Shift+R` | Read Row Header | `{RowHeader} row {r}` |
| `Ctrl+Alt+Shift+Left` | Row Start | `beginning of row\n{CellText}` |
| `Ctrl+Alt+Shift+Right` | Row End | `end of row\n{CellText}` |
| `Ctrl+Alt+Shift+Up` | Column Top | `top of column\n{CellText}` |
| `Ctrl+Alt+Shift+Down` | Column Bottom | `bottom of column\n{CellText}` |
| `Ctrl+Alt+Numpad5` / `Ctrl+Alt+5` | Read Current Cell | `{CellText}\nColumn {c}, Row {r}` |
| `O` | Next Math | `math {linearized_expr}` |
| `Shift+O` | Prior Math | `math {linearized_expr}` |
| `Enter` (on Math) | Math Viewer | `Math Viewer: {linearized_expr}` |
