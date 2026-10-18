## What Okur says in its own words, outside roles, states, hints and tables.

# An item's place among its siblings: "2 of 5".
position = { $index } of { $total }

# A progress bar's value, in percent.
progress-percent = { $value }%

# A page's title changed (a single-page application moved to another route).
page-loaded = Page has loaded: { $title }

# An inline frame. JAWS names it before the word: "Comments frame".
frame = frame
frame-named = { $name } frame
frame-end = frame end

# Said when forms mode ends and the virtual cursor reads the page again. JAWS's capitals.
cursor-virtual-pc = virtual PC Cursor

# Said for an empty line, or in place of an empty field's value.
blank = blank
# Said on arrival at a text field of several lines that holds text, in place of the text.
field-contains-text = Contains text

## Say Font (Insert+F)

font-bold = bold
font-italic = italic
font-underline = underline
font-size = { $size } point
font-is = Font is { $font }
font-none = No font
paragraph-formatting = Paragraph formatting { $alignment }
paragraph-left-justified = left justified
paragraph-right-justified = right justified

## Selecting text with Shift and the arrows

selection-selected = selected { $text }
selection-unselected = unselected { $text }
# Say Selected Text with nothing selected.
selection-none = No selection

## A modal dialog in a page

dialog-entered = Dialog
dialog-entered-named = Dialog { $title }
dialog-left = Leaving Dialog

## Find (Control+F)

find-not-found = Text not found: { $text }

## Native controls (outside a page)

# A trackbar's value given in percent, said in words.
value-percent = { $number } percent
# A toggle switch's state.
toggle-on = On
toggle-off = Off
# A native tree item opened: how many items it holds.
tree-items = { $count } items
# Arrows in a native tree reached another level.
tree-level-changed = level { $level }
# A combo box's list opened (Alt+Down).
combo-box-opened = Open List Box
# Keys pressed one after another, as an access key "Alt, I" is said: "Alt followed by I".
key-followed-by = { $keys } followed by { $key }
# The access key of a control whose own text holds the letter: "alt+S".
access-key-alt = alt+{ $letter }
# A menu named after the item it opened from: "File menu".
menu-named = { $name } menu