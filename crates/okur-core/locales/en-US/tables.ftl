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
# entered is placed by its column or row [obs, 1034/s12]; Say Current Focus places it fully
# [obs, 1034/s13].
word-column-of = column { $column } of { $columns }
word-row-of = row { $row } of { $rows }
# A table's next row as Say All reaches it: "Hours | Row 2 | Central" [obs, 1038/s12].
word-row = Row { $row }
# A row of a table without row titles, on moving along a column and in Say Cell; the first row,
# in Say Cell [obs, 1040/s13, s14].
word-row-no-title = Row { $row } no title
word-title-row = This is the title row, containing column titles.
# Tab selects the cell's text [obs, 518/s15].
word-cell-selected = Selected
# The table's last cell, where Tab adds a row [obs, 518/s15, 1034/s10].
word-last-cell = last cell in table
word-tab-adds-row = Warning! Pressing tab here adds an extra row.
# Ctrl+Alt+Home to the table's first cell [obs, 1035/s12].
word-table-start = beginning of table
# Say Cell and Say Current Focus at the last cell [obs, 1035/s10, s11].
word-last-cell-said = Last cell in table.
# A table key outside a table [obs, 1035/s15].
word-not-in-table = Not in a Table

## Say Font in Word's document [obs, 1034/s1, s3].

word-style = { $style } style
# Say Font's first words for bold, italic and underlined text, JAWS's own, its two spaces kept
# [obs, 1040/s3, s5, s7].
word-bolded = bolded
word-italicized = italicized
word-underline-single = Single  underline
word-paragraph-aligned = Paragraph formatting: Aligned { $alignment }
# Say Font's line spacing in Word, after the style, as Word keeps it [obs, 1052, 1040/s17, s19]:
# single, one and a half, double, at least and exactly so many points, so many lines.
word-line-spacing = Line spacing: { $spacing }
word-line-spacing-single = Single
word-line-spacing-one-and-a-half = one point five lines.
word-line-spacing-double = Double
word-line-spacing-at-least = At Least { $points } points
word-line-spacing-exactly = Exactly { $points } points
word-line-spacing-lines = { $lines } lines.
word-outline-level = Outline level: { $level }
word-outline-body = Outline level: body text

# Say Window Title in Word adds the view [obs, 519/s1].
word-view-print = Print View
# Ctrl+Alt+N and Ctrl+Alt+O in Word, and Say Window Title in those views [obs, 1039].
word-view-draft = Draft View
word-view-outline = Outline View
# Say Window Title in Web Layout and Read Mode: JAWS's lower case "reading layout" [obs,
# 1042/s5, s9].
word-view-web = Web Layout View
word-view-read = reading layout View
# The Links List in a Word document without links, after "no links"; Select a Form Field
# (Insert+F5), which lists Word's fields, without any [obs, 1044/s1, 1047/s1].
word-links-list-hint = Check for link type fields with Insert+F5.
word-fields-list-none = no fields
