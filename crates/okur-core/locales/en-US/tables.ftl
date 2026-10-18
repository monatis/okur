## Tables: entering one and moving between its cells.

# Entering a table (T).
table-entry = table with { $columns } columns and { $rows } rows
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
