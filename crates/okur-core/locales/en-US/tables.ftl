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
# Ctrl+Shift+E in Word: the state it leaves Track Changes in [obs, 1068/s1, s8].
word-track-changes-on = Track Changes on
word-track-changes-off = Track Changes off
# Ctrl+E, Ctrl+R, Ctrl+J and Ctrl+L in Word: the alignment they leave the paragraph in, JAWS's
# capitals [obs, 1071/s2-s5].
word-aligned-centered = Centered
word-aligned-right = aligned right
word-aligned-justified = justified
word-aligned-left = Aligned left
# Ctrl+M and Ctrl+Shift+M in Word: the way, then where the paragraph starts from the page's edge, its
# margin and indent: "indent | 1 inch",
# "out dent | 1.5 inches" [obs, 1080/s2-s4].
word-indent = indent
word-outdent = out dent
word-one-inch = 1 inch
word-inches = { $inches } inches
# A comment's start in a line of Word's [obs, 1081/s1].
word-has-comment = has comment
# A footnote reference in a line of Word's, before its number: "every day footnote 1." [obs,
# 1083/s4].
word-footnote = footnote
word-endnote = end note
# What a revision of Word's changed, after its text in the Virtual Viewer: "eight inserted text
# Lab Sun, Jan 04, 26" [obs, 1104/s7, s8].
word-revision-inserted = inserted text
word-revision-deleted = deleted text
# Ctrl+Insert+R in Word: the revision at the caret, after its text, or none [obs, 1107/s10,
# 1108/s3, s8, s10].
word-revision-by = { $change } revised by { $author } on { $date }
word-no-revision = No Revision
# A word read in a revision while Word tracks changes [obs, 1107/s8, 1108/s7], and a line read
# with the caret on one, after JAWS's mark [obs, 1107/s9, 1111/s10].
word-revision-inserted-mark = Inserted
word-revision-deleted-mark = Deleted
word-revision-line = revision
# A page and a section of Word's reached, before the line [obs, 1112/s2, s5, s9].
word-page = page { $page }
word-section = Section { $section }
# Ctrl+PageDown and Ctrl+PageUp in Word, Word's next and previous page [obs, 1113/s1-s4].
word-next-page = Next page
word-previous-page = Previous page
# Insert+Delete in Word [obs, 1112/s7, 1113/s6, s8].
word-line-and-column = line { $line } column { $column }
# A footnote's or endnote's reference selected, by its formatting, and the line read after it
# [obs, 1092/s8, s10].
word-superscript = superscript
word-end-superscript = end superscript
# Alt+Shift+Apostrophe in Word: the comment at the caret, then its author [obs, 1081/s8], or
# none [obs, 1081/s3].
word-comment-by = by { $author }
word-no-comment = No comment mark at cursor.
# Alt+Shift+Apostrophe pressed twice: the Virtual Viewer's line before the text the comment is
# on [obs, 1095/s6].
word-comment-document-text = Document text:
# OK in Items to Display in Virtual Viewer: what is gathered, then the viewer's first line
# [obs, 1096/s2, 1097/s3, 1099/s4].
word-viewer-processing-comments = Please wait: Processing Comments
word-viewer-processing-footnotes = Please wait: Processing Footnotes
word-viewer-processing-endnotes = Please wait: Processing Endnotes
word-viewer-processing-revisions = Please wait: Processing Revisions
word-viewer-one-comment = There is only one Comment to display.
word-viewer-one-footnote = There is only one Footnote to display.
word-viewer-one-endnote = There is only one Endnote to display.
# One revision as one of the others [hyp: no document with one revision recorded yet].
word-viewer-one-revision = There is only one Revision to display.
# Several [obs, 1101/s2, 1102/s3, 1104/s5, 1105/s4].
word-viewer-comments = There are { $count } Comments in this document.
word-viewer-footnotes = There are { $count } Footnotes in this document.
word-viewer-endnotes = There are { $count } Endnotes in this document.
word-viewer-revisions = There are { $count } Revisions in this document.
# The same with none: no viewer opens [obs, 1100/s2].
word-viewer-none = There are no items to display.
# OK on a footnote in JAWS's list of Word's footnotes: the footnote as the list shows it, before
# the document's "Edit" [obs, 1088/s6].
word-footnote-reference = footnote reference: { $footnote }
# The same for an endnote, by its mark: "end note reference: i Renewals are free." [obs, 1090/s6].
word-endnote-reference = end note reference: { $endnote }
# Ctrl+Shift+Apostrophe, Insert+Shift+F, Insert+Shift+E and Insert+Alt+B in a document without
# comments, footnotes, endnotes or bookmarks [obs, 1089/s1, s3; 1093/s1, s3].
word-no-comments = There are no comments in the document.
word-no-footnotes = There are no footnotes in the document.
word-no-endnotes = There are no end notes in the document.
word-no-bookmarks = There are no bookmarks in the document.
# Insert+Shift+R without revisions [hyp: 1107 records it].
word-no-revisions = There are no revisions in the document.
# Insert+Shift+R and Ctrl+Insert+R in Word while it does not track changes [obs, 1109/s1, s3].
word-revisions-list-untracked = Track Changes is disabled. Press the Word command, CONTROL+SHIFT+E, to enable the feature.
word-revisions-untracked = You must enable Track Changes in order to use this feature.
# Alt+Shift+E in Word away from a footnote's or endnote's reference [obs, 1092/s1].
word-no-note-at-cursor = No footnote or EndNote reference at cursor.
# Excel's Ctrl+End and Ctrl+Home, before the cell they reach, and a Ctrl arrow's, after it
# [obs, 1115/s1-s6].
excel-last-cell = Last Cell
excel-first-cell = First cell
excel-data-region-edge = edge of current data region
# Excel's lists with nothing to list: a column's [obs, 521/s9 in round-521e]; formulas' and
# worksheets' [hyp: not recorded].
excel-no-column-cells = No cells with data in current column.
excel-no-formulas = No cells with formulas.
excel-no-worksheets = No worksheets.
# Say Window Title in Web Layout and Read Mode: JAWS's lower case "reading layout" [obs,
# 1042/s5, s9].
word-view-web = Web Layout View
word-view-read = reading layout View
# Read Mode entered: JAWS's warning, in Okur's name [obs, 1042/s8, 1059/s3].
word-read-mode-warning = WARNING! Okur does not support this view. Press Okur Key+F1 to learn how to disable Reading Layout view. To exit Reading Layout view, press ESC.
# The Links List in a Word document without links, after "no links"; Select a Form Field
# (Insert+F5), which lists Word's fields, without any [obs, 1044/s1, 1047/s1].
word-links-list-hint = Check for link type fields with Insert+F5.
word-fields-list-none = no fields
