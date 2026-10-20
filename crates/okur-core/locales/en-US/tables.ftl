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
# A table move found no cell further that way.
table-edge = edge of table
# A focus entering a table cell, or moving to another row and column at once.
table-focus-cell = column { $column } row { $row }
# The table layer (Insert+Space, T) turned on: the arrows move by cell until Escape.
table-layer = Table layer
# Reading the next or prior row or column whole (Windows+Alt and an arrow) found none further.
table-on-last-row = You are on the last row in the table.
# With short messages [obs, 578/s6; 592/s2, s3, s6].
table-on-last-row-short = Last row
table-on-first-row = You are on the first row in the table.
table-on-first-row-short = First row
table-on-last-column = You are on the last column in the table.
table-on-last-column-short = Last column
table-on-first-column = You are on the first column in the table.
table-on-first-column-short = First column
