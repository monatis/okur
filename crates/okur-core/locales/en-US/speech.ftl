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
# The same, JAWS's short message, above Beginner verbosity [obs, 552/s10, 553/s10].
cursor-virtual-pc-short = virtual PC
# The cursors: Numpad Minus, Numpad Plus, Insert+Numpad Minus and Alt+Delete [obs, 516,
# 513/s2]. The position is the screen's x and y in pixels, "1045, 544". JAWS's words, in
# Okur's name where JAWS names itself ("JAWS Cursor").
cursor-okur = Okur Cursor
cursor-pc = PC Cursor
cursor-route-okur-to-pc = Route Okur to PC
cursor-okur-active = Okur Cursor active
cursor-pc-active = PC Cursor active
cursor-position = { $x }, { $y }
cursor-restriction-application = Application window restriction

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

## Say Color (Insert+5)

# The text's color and its background's, by their names in colors.txt: "blue2 on white"
# [obs, 565, 618]. Where the cursor is on no text, both are "default" [obs, 565/s1, 530/s2].
color-on = { $foreground } on { $background }
color-default = default

## Punctuation levels (Set Punctuation Level, Insert+Shift+2)

punctuation-none = None
punctuation-some = Some
punctuation-most = Most
punctuation-all = All

## Typing echo (Insert+2)

typing-echo-none = None
typing-echo-characters = Characters
typing-echo-words = Words
typing-echo-both = Both characters and words
# The keys typing echo names.
typing-echo-space = Space
typing-echo-enter = Enter

## Say Character pressed three times quickly

# The character's value, digit by digit: "Character 7 4" for J, "Character 8 3 6 4" for the
# euro sign.
character-value = Character { $digits }

## Selecting text with Shift and the arrows

# A character or word taken out of a selection with the virtual cursor.
selection-removed = unselected
# Lines taken out of a selection (Ctrl+A) [obs, 501/s13].
selection-removed-lines = Unselected
# Say Selected Text with nothing selected.
selection-none = Nothing selected
# Say Selected Text, before the selection [obs, 501/s8, 504/s2].
selection-is = Selection is
# A selection too long to read, said by its size and its end: "selected | 1032 characters up
# to | d of all of it here." The count leaves out the end said after it [obs, 617/s24, 515/s1].
selection-summary = { $count } characters up to

## A modal dialog in a page

dialog-entered = Dialog
dialog-entered-named = Dialog { $title }
dialog-left = Leaving Dialog

## A property sheet in a dialog box

# Ctrl+Tab moved the focus onto another page of the sheet [obs, 323/s10, s11].
property-page-changed = { $page } Page

## Find (Control+F)

find-not-found = Text not found: { $text }

## Native controls (outside a page)

# A trackbar's value given in percent, said in words.
value-percent = { $number } percent
# A toggle switch's state.
toggle-on = On
toggle-off = Off
# A native tree item opened: how many items it holds.
tree-items =
    { $count ->
        [one] { $count } item
       *[other] { $count } items
    }
# Arrows in a native tree reached another level.
tree-level-changed = level { $level }
# With short messages (Intermediate and Advanced verbosity), the level alone [obs, 579/s8].
tree-level-changed-short = { $level }
# A combo box's list opened (Alt+Down); with short messages nothing [obs, 580/s11].
combo-box-opened = Open List Box
# Keys pressed one after another, as an access key "Alt, I" is said: "Alt followed by I".
key-followed-by = { $keys } followed by { $key }
# The access key of a control whose own text holds the letter: "alt+S".
access-key-alt = alt+{ $letter }
# A menu named after the item it opened from: "File menu".
menu-named = { $name } menu

## JAWS's text analysis: what Say Current Focus (Insert+Tab) adds after a text field, one per
## inconsistency of the caret's line, with its column from 1 [obs, 568, 571/s4].
text-analysis-space-run = Space run at column { $column }
text-analysis-missing-initial-cap = Missing initial cap { $symbol } at column { $column }
text-analysis-inverted-cap = Inverted cap { $symbol } at column { $column }
text-analysis-unopened = Unopened { $symbol } at column { $column }
text-analysis-stray-punctuation = Stray punctuation { $symbol } at column { $column }
text-analysis-unspaced-punctuation = Unspaced punctuation { $symbol } at column { $column }
text-analysis-unclosed = Unclosed { $symbol } at column { $column }
