## Tables: entering one and moving between its cells.

# Entering a table (T).
table-entry = table with { $columns } columns and { $rows } rows
# With short messages (Intermediate and Advanced verbosity) [obs, 578/s1].
table-entry-short = { $columns } columns and { $rows } rows
table-cell = Column { $column }, Row { $row }
table-column-of = Column { $column } of { $columns }
table-row-of = Row { $row } of { $rows }
table-column = Column { $column }
table-row = Row { $row }
table-spans-columns = Spans { $columns } columns
# A cell of a row that spans several rows.
table-cell-of-spanning-row = cell { $column } of row spanning { $rows } rows
table-row-start = beginning of row
table-row-end = end of row
table-column-top = top of column
table-column-bottom = bottom of column
table-not-in-table = Not in a table.
# A focus entering a table cell, or moving to another row and column at once.
table-focus-cell = column { $column } row { $row }
# With short messages [obs, 593/s11, s12].
table-focus-cell-short = col { $column } row { $row }
# The table layer (Insert+Space, T) turned on: the arrows move by cell until Escape.
table-layer = Table layer
# With short messages [obs, 895/s5].
table-layer-short = Table
# Reading the next or prior row or column whole (Windows+Alt and an arrow) found none further
# [obs, 297/s6; 615/s4, s5, s10].
table-on-last-row = You are on the last row in the table.
# With short messages [obs, 578/s6; 592/s2, s3, s6].
table-on-last-row-short = Last row
table-on-first-row = You are on the first row of the table.
table-on-first-row-short = First row
table-on-last-column = You are on the last column in the row.
table-on-last-column-short = Last column
table-on-first-column = You are on the first column of the row.
table-on-first-column-short = First column

## Microsoft Word's documents, as JAWS reads them [obs, 518].

# A table entered: its number in the document, that its rows are alike, then the cell
# [obs, 518/s13, s18].
word-table = table { $number }
word-table-uniform = uniform Table
# Another cell of the table [obs, 518/s14].
word-new-cell = new cell
# The table left [obs, 518/s16, s19].
word-out-of-table = out of table
# A list entered and left, after the line [obs, 518/s10, s12].
word-entering-list = Entering List
word-exit-list = exit list
# JAWS's table keys and Tab in Word's table: a cell without a title for the way it was
# entered is placed by its column or row [obs, 522/s12]; Say Current Focus places it fully
# [obs, 522/s13].
word-column-of = column { $column } of { $columns }
word-row-of = row { $row } of { $rows }
# Tab selects the cell's text [obs, 518/s15].
word-cell-selected = Selected
# The table's last cell, where Tab adds a row [obs, 518/s15, 522/s10].
word-last-cell = last cell in table
word-tab-adds-row = Warning! Pressing tab here adds an extra row.
