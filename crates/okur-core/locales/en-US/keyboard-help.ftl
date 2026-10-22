## Keyboard Help (Insert+1). While it is on, a key is not run: Okur says the key's name, then
## what the key does; pressed twice quickly, a longer description [obs, 509]. The key names are
## JAWS's key notation, the names its users know the keys by. The descriptions are Okur's own
## words: JAWS's are Freedom Scientific's, and are never copied.

## Key names, as JAWS's key map writes them

# A chord: a modifier held, then the rest of the keys, "Insert+UpArrow", "Control+Insert+F".
# Modifiers come in this order: Control, Alt, Shift, Windows, then the Okur key.
key-chord = { $modifier }+{ $key }
# The Control key.
key-control = Control
# The Alt key.
key-alt = Alt
# The Shift key.
key-shift = Shift
# The Windows key.
key-windows = Windows
# The Insert key, the Okur key of the desktop layout.
key-insert = Insert
# The Caps Lock key, the Okur key of the laptop layout.
key-caps-lock = CapsLock
# The Scroll Lock key.
key-scroll-lock = ScrollLock
# The Up Arrow key.
key-up-arrow = UpArrow
# The Down Arrow key.
key-down-arrow = DownArrow
# The Left Arrow key.
key-left-arrow = LeftArrow
# The Right Arrow key.
key-right-arrow = RightArrow
# The Home key.
key-home = Home
# The End key.
key-end = End
# The Page Up key.
key-page-up = PageUp
# The Page Down key.
key-page-down = PageDown
# The Tab key.
key-tab = Tab
# The Enter key.
key-enter = Enter
# The Escape key.
key-escape = Escape
# The Spacebar.
key-space = SpaceBar
# The Backspace key.
key-backspace = BackSpace
# The Delete key.
key-delete = Delete
# A function key, F1 to F24.
key-function = F{ $number }
# A letter or a digit key, as it is printed on the key: "T", "1".
key-character = { $character }
# The key at the US comma place, whatever the keyboard layout prints on it.
key-comma = Comma
# The key at the US period place.
key-period = Period
# The key at the US semicolon place, as JAWS's key map spells it.
key-semicolon = SemiColon
# The key at the US apostrophe place.
key-apostrophe = Apostrophe
# The key at the US minus place.
key-dash = Dash
# The key at the US equals place.
key-equals = Equals
# The key at the US slash place.
key-slash = Slash
# The key at the US backslash place.
key-backslash = Backslash
# The numeric keypad's 5.
key-numpad-five = NumPad5
# The numeric keypad's plus.
key-numpad-plus = NumPadPlus
# The numeric keypad's minus.
key-numpad-minus = NumPadMinus
# The numeric keypad's slash.
key-numpad-slash = NumPadSlash
# The numeric keypad's star.
key-numpad-star = NumPadStar
# A key Okur has no name for and that types no character: its code.
key-unnamed = Key { $code }

## What a key does

# After the name of a key Okur does not use: the key goes to the application.
key-help-passes = Goes to the application.

## Reading where the cursor is

# SayAll (Insert+Down Arrow).
command-say-all = Reads from the cursor to the end.
command-skim-read = Reads from the cursor to the end, each paragraph by its first line.
# SkimReadDialog (Control+Shift+Insert+Down).
command-skim-read-dialog = Chooses how skim reading reads each paragraph, and starts it or makes its summary.
# SayAll pressed twice quickly in Keyboard Help.
command-say-all-long = { command-say-all } The cursor follows the reading, and Control stops it on the line last heard.
# SayLine (Insert+Up Arrow).
command-say-line = Reads the line at the cursor.
# SayLine, the longer description.
command-say-line-long = { command-say-line } Pressed twice quickly, it spells the line.
# SayWord (Insert+NumPad5).
command-say-word = Reads the word at the cursor.
# SayWord, the longer description.
command-say-word-long = { command-say-word } Pressed twice quickly, it spells the word.
# SayCharacter (NumPad5).
command-say-character = Reads the character at the cursor.
# SayCharacter, the longer description.
command-say-character-long = { command-say-character } Pressed twice quickly, it says the character phonetically; three times, its character code.
# SayCharacter pressed twice quickly.
command-say-character-phonetic = Says the character at the cursor phonetically.
# SayCharacter pressed three times quickly.
command-say-character-value = Says the character code of the character at the cursor.
# The character code, the longer description.
command-say-character-value-long = { command-say-character-value } It is the character's Unicode number, read digit by digit.
# SayWord pressed twice quickly.
command-spell-word = Spells the word at the cursor.
# SayLine pressed twice quickly.
command-spell-line = Spells the line at the cursor.

## Moving, then reading

# SayNextLine (Down Arrow).
command-say-next-line = Moves to the next line and reads it.
# SayPriorLine (Up Arrow).
command-say-prior-line = Moves to the previous line and reads it.
# SayNextWord (Control+Right Arrow).
command-say-next-word = Moves to the next word and reads it.
# SayPriorWord (Control+Left Arrow).
command-say-prior-word = Moves to the previous word and reads it.
# SayNextCharacter (Right Arrow).
command-say-next-character = Moves to the next character and reads it.
# SayPriorCharacter (Left Arrow).
command-say-prior-character = Moves to the previous character and reads it.
# SayNextParagraph (Control+Down Arrow, P in a page).
command-say-next-paragraph = Moves to the next paragraph and reads it.
# SayPriorParagraph (Control+Up Arrow, Shift+P in a page).
command-say-prior-paragraph = Moves to the previous paragraph and reads it.
# SayParagraph (Control+NumPad5).
command-say-paragraph = Reads the paragraph at the cursor.
# SaySentence (Alt+NumPad5).
command-say-sentence = Reads the sentence at the cursor.
# SayNextSentence (Alt+NumPadPlus).
command-say-next-sentence = Moves to the next sentence and reads it.
# SayPriorSentence (Alt+NumPadMinus).
command-say-prior-sentence = Moves to the previous sentence and reads it.
# The laptop layout's CapsLock+Control+O.
command-next-paragraph-or-articles = On a web page, lists its articles; elsewhere, moves to the next paragraph and reads it.
# Home with the virtual cursor.
command-line-start = Moves to the start of the line.
# End with the virtual cursor.
command-line-end = Moves to the end of the line.
# Page Up with the virtual cursor.
command-page-up = Moves up a page.
# Page Down with the virtual cursor.
command-page-down = Moves down a page.
# TopOfFile (Control+Home).
command-top-of-file = Moves to the top of the document.
# BottomOfFile (Control+End).
command-bottom-of-file = Moves to the bottom of the document.

## Quick navigation in a page

# A quick key: H, Shift+H and the others. $element is one of the element-kind messages below.
command-move-to-next = Moves to the next { $element }.
# The same, the longer description.
command-move-to-next-long = { command-move-to-next } It works where the virtual cursor reads, as on a web page.
# Shift with a quick key.
command-move-to-prior = Moves to the previous { $element }.
# The same, the longer description.
command-move-to-prior-long = { command-move-to-prior } It works where the virtual cursor reads, as on a web page.
# What a quick key moves to, said in "Moves to the next heading.": H.
element-kind-heading = heading
# 1 to 6.
element-kind-heading-level = heading at level { $level }
# A link of any kind (MoveToNextLink, which has no key of its own).
element-kind-link = link
# U.
element-kind-unvisited-link = unvisited link
# V.
element-kind-visited-link = visited link
# T.
element-kind-table = table
# O.
element-kind-article = article
# SemiColon: an element that reacts to the mouse pointer moving over it (onMouseOver).
element-kind-on-mouse-over = element that reacts to the mouse
# Slash: an element with a click handler (onClick).
element-kind-on-click = clickable element
# Apostrophe: an ARIA tab.
element-kind-tab = tab
# K: a place on a page the user marked.
element-kind-placemarker = PlaceMarker
# G.
element-kind-graphic = graphic
# A.
element-kind-radio-button = radio button
# A math equation.
element-kind-math = math equation
# F.
element-kind-form-field = form field
# B.
element-kind-button = button
# E.
element-kind-edit = edit field
# X.
element-kind-check-box = check box
# C.
element-kind-combo-box = combo box
# L.
element-kind-list = list
# I.
element-kind-list-item = list item
# R: a landmark.
element-kind-region = region
# M.
element-kind-frame = frame
# Z: a division with text.
element-kind-division = division
# Dash: a horizontal rule.
element-kind-separator = separator
# Backslash: a link that writes an e-mail.
element-kind-mail-link = mail link
# MoveToMainRegion (Q).
command-move-to-main-region = Moves to the main region of the page.
# MoveToNextNonLinkText (N, Insert+Enter).
command-next-non-link-text = Moves to the next text that is not a link.
# The same, the longer description.
command-next-non-link-text-long = { command-next-non-link-text } It skips blocks of links, such as a site's menus.
# MoveToPriorNonLinkText (Shift+N, Insert+Shift+Enter).
command-prior-non-link-text = Moves to the previous text that is not a link.
# MoveToNextSameElement (S).
command-next-same-element = Moves to the next element of the same type as the one at the cursor.
# MoveToPriorSameElement (Shift+S, Control+Insert+Up Arrow).
command-prior-same-element = Moves to the previous element of the same type as the one at the cursor.
# MoveToNextDifferentElement (D, Control+Insert+Right Arrow).
command-next-different-element = Moves to the next element of a different type from the one at the cursor.
# MoveToPriorDifferentElement (Shift+D, Control+Insert+Left Arrow).
command-prior-different-element = Moves to the previous element of a different type from the one at the cursor.

## Tables, headings and form fields

# Control+Alt+Right Arrow.
command-table-cell-right = Moves one cell right in the table and reads it.
# Control+Alt+Left Arrow.
command-table-cell-left = Moves one cell left in the table and reads it.
# Control+Alt+Down Arrow.
command-table-cell-down = Moves one cell down in the table and reads it.
# Control+Alt+Up Arrow.
command-table-cell-up = Moves one cell up in the table and reads it.
# Control+Alt+Home.
command-table-first-cell = Moves to the table's first cell.
# Control+Alt+End.
command-table-last-cell = Moves to the table's last cell.
# Control+Alt+Shift+Left Arrow.
command-move-to-start-of-row = Moves to the first cell of the row.
# Control+Alt+Shift+Right Arrow.
command-move-to-end-of-row = Moves to the last cell of the row.
# Control+Alt+Shift+Up Arrow.
command-move-to-top-of-column = Moves to the top cell of the column.
# Control+Alt+Shift+Down Arrow.
command-move-to-bottom-of-column = Moves to the bottom cell of the column.
# SayCurrentCell (Control+Alt+NumPad5).
command-say-current-cell = Reads the table cell at the cursor, with its row and column.
# Control+Alt+Shift+H.
command-say-column-header = Reads the header of the cell's column.
# Control+Alt+Shift+R.
command-say-row-header = Reads the header of the cell's row.
# Alt+Insert+Home.
command-first-heading = Moves to the page's first heading.
# Alt+Insert+End.
command-last-heading = Moves to the page's last heading.
# Control+Alt+Insert and a level, 1 to 6.
command-first-heading-at-level = Moves to the page's first heading at level { $level }.
# Control+Alt+Insert+Shift and a level, 1 to 6.
command-last-heading-at-level = Moves to the page's last heading at level { $level }.
# Control+Insert+Home.
command-first-form-field = Moves to the page's first form field.
# Control+Insert+End.
command-last-form-field = Moves to the page's last form field.
# Windows+Alt+NumPad5, Windows+Alt+Comma.
command-say-current-row = Reads the whole table row at the cursor.
# Windows+Alt+Period.
command-say-current-column = Reads the whole table column at the cursor.
# Windows+Alt+Down Arrow.
command-read-next-row = Moves to the next table row and reads it whole.
# Windows+Alt+Up Arrow.
command-read-prior-row = Moves to the previous table row and reads it whole.
# Windows+Alt+Right Arrow.
command-read-next-column = Moves to the next table column and reads it whole.
# Windows+Alt+Left Arrow.
command-read-prior-column = Moves to the previous table column and reads it whole.
# StepToStartOfElement (Shift+Comma, Alt+Windows+Home).
command-step-to-start-of-element = Moves to the start of the element the cursor is in, such as a list or a table.
# StepToEndOfElement (Shift+Period, Alt+Windows+End).
command-step-to-end-of-element = Moves to the end of the element the cursor is in, such as a list or a table.
# MoveToFlowTo (Equals): along the page's own reading order (aria-flowto).
command-flow-to = Moves to the element the page says follows the one at the cursor.
# MoveToFlowFrom (Shift+Equals).
command-flow-from = Moves back to the element the page says comes before the one at the cursor.

## A page's controls

# Tab.
command-next-tab-stop = Moves to the next control.
# Shift+Tab.
command-prior-tab-stop = Moves to the previous control.
# The Spacebar with the virtual cursor.
command-activate = Clicks the control at the cursor.
# The same, the longer description.
command-activate-long = { command-activate } A check box is checked or cleared, a button pressed, and a field gets the focus to type in.
# Enter with the virtual cursor.
command-enter = Uses the link or the control at the cursor.
# The same, the longer description.
command-enter-long = { command-enter } On an edit field or a combo box, it turns forms mode on, so that keys type into the field.
# Escape with the virtual cursor.
command-escape = Closes an open list, leaves forms mode, or leaves a dialog.
# Alt+Down Arrow.
command-expand-combo-box = Opens the combo box's list.
# Alt+Up Arrow.
command-collapse-combo-box = Closes the combo box's list.
# Entering a math equation.
command-enter-math-viewer = Opens the math equation at the cursor in the Math Viewer.
# Entering a page's modal dialog.
command-enter-modal = Keeps the virtual cursor inside the page's dialog.
# Leaving a page's modal dialog.
command-exit-modal = Lets the virtual cursor leave the page's dialog.

## Selecting

# Shift+Right Arrow.
command-select-next-character = Selects the next character.
# Shift+Left Arrow.
command-select-prior-character = Selects the previous character.
# Shift+Down Arrow.
command-select-next-line = Selects the next line.
# Shift+Up Arrow.
command-select-prior-line = Selects the previous line.
# Control+Shift+Right Arrow.
command-select-next-word = Selects the next word.
# Control+Shift+Left Arrow.
command-select-prior-word = Selects the previous word.
# Shift+End.
command-select-to-line-end = Selects from the cursor to the end of the line.
# Shift+Home.
command-select-to-line-start = Selects from the cursor to the start of the line.
# Control+Shift+End.
command-select-to-bottom = Selects from the cursor to the bottom of the document.
# Control+Shift+Home.
command-select-to-top = Selects from the cursor to the top of the document.
# Shift+Page Down.
command-select-next-page = Selects a page down from the cursor.
# Shift+Page Up.
command-select-prior-page = Selects a page up from the cursor.
# Control+A.
command-select-all = Selects all the text.
# SelectHTMLElement (F8).
command-select-html-element = Selects the element at the cursor.
# The same, the longer description.
command-select-html-element-long = { command-select-html-element } On a web page, the whole element is selected, such as a paragraph or a list.
# Control+C with text selected by the virtual cursor.
command-copy-selection = Copies the selected text to the clipboard.
# SayClipboard (Windows+Insert+X).
command-say-clipboard = Reads the text on the clipboard.
# ViewClipboardText (Insert+Space, then C).
command-view-clipboard-text = Shows the text on the clipboard in a viewer.
# AppendSelectedTextToClipboard (Insert+Windows+C).
command-append-selection-to-clipboard = Adds the selected text to the text on the clipboard.
# SaySelectedText (Insert+Shift+Down Arrow).
command-say-selected-text = Reads the selected text.
# The same, the longer description.
command-say-selected-text-long = { command-say-selected-text } Pressed twice quickly, it spells it.
# SaySelectedText pressed twice quickly.
command-spell-selected-text = Spells the selected text.

## Okur's modes and the voice

# Insert+Space.
command-layer-start = Waits for one more key, which runs a layered command.
# The same, the longer description.
command-layer-start-long = { command-layer-start } After it, H opens the Speech History and S turns speech off and on.
# NavigationModeToggle (Insert+N).
command-navigation-mode-toggle = Turns the quick keys on, off, or on during Say All only.
# Insert+Shift+Z, Control+Insert+N.
command-quick-keys-suspend-resume = Suspends the quick keys, or resumes them.
# The same, the longer description.
command-quick-keys-suspend-resume-long = { command-quick-keys-suspend-resume } While they are suspended, letter keys go to the page.
# VirtualPCCursorToggle (Insert+Z).
command-virtual-cursor-toggle = Turns the virtual cursor off or on.
# The same, the longer description.
command-virtual-cursor-toggle-long = { command-virtual-cursor-toggle } While it is off, the arrow keys on a web page go to the page.
# RefreshScreen (Insert+Escape).
command-refresh-screen = Updates what Okur knows of the screen.
# Control+Alt+Page Up.
command-rate-faster = Makes the voice faster, until Okur exits.
# Control+Alt+Page Down.
command-rate-slower = Makes the voice slower, until Okur exits.

## Lines of the window and of the cursor

# SayTopLineOfWindow (Insert+End).
command-say-top-line-of-window = Reads the window's top line, which is usually its title.
# SayBottomLineOfWindow (Insert+Page Down).
command-say-bottom-line-of-window = Reads the window's bottom line, which is often its status bar.
# SayToCursor (Insert+Home).
command-say-to-cursor = Reads the line from its start to the cursor.
# The same, the longer description.
command-say-to-cursor-long = { command-say-to-cursor } Pressed twice quickly, it spells that part of the line.
# SayFromCursor (Insert+Page Up).
command-say-from-cursor = Reads the line from the cursor to its end.
# The same, the longer description.
command-say-from-cursor-long = { command-say-from-cursor } Pressed twice quickly, it spells that part of the line.
# SayToCursor pressed twice quickly.
command-spell-to-cursor = Spells the line from its start to the cursor.
# SayFromCursor pressed twice quickly.
command-spell-from-cursor = Spells the line from the cursor to its end.

## Windows, dialogs and the application

# SayDefaultButton (Insert+E).
command-say-default-button = Says the dialog box's default button.
# The same, the longer description.
command-say-default-button-long = { command-say-default-button } It is the button that Enter presses.
# SayConfiguration (Insert+Q).
command-say-configuration = Says which settings are loaded and which application is in use.
# SayAppVersion (Control+Insert+V).
command-say-app-version = Says the name and the version of the application in use.
# The Okur cursor (NumPadMinus), JAWS's JAWS cursor.
command-okur-cursor = Makes the Okur cursor, which is the mouse pointer, the active cursor.
# The PC cursor (NumPadPlus).
command-pc-cursor = Makes the PC cursor, which follows the focus, the active cursor.
# Insert+NumPadMinus, JAWS's Route JAWS to PC.
command-route-okur-to-pc = Moves the mouse pointer to the focus.
# The same, the longer description.
command-route-okur-to-pc-long = { command-route-okur-to-pc } The Okur cursor is then where the PC cursor is.
# SayActiveCursor (Alt+Delete).
command-say-active-cursor = Says which cursor is active and where it is on the screen.
# Shift+NumPad5.
command-say-hot-key = Says the shortcut key of the control that has the focus.
# Control+Insert and a digit: a list view item's column, 1 to 10.
command-say-list-column = Reads column { $column } of the list item that has the focus, with its header.
# IndicateLinkSourceURL (Control+Insert+F7).
command-say-link-address = Says the address of the link at the cursor.
# RouteVirtualToPc (Insert+Delete).
command-route-virtual-to-pc = Moves the virtual cursor to the control that has the focus.
# RoutePCToVirtual (Control+Insert+Delete).
command-route-pc-to-virtual = Moves the focus to the element at the virtual cursor.
# SelectAFrame (Insert+F9).
command-select-a-frame = Lists the page's frames, to move to one.
# NumPadSlash.
command-left-mouse-click = Clicks the left mouse button on the element at the cursor.
command-right-mouse-click = Clicks the right mouse button where the mouse is.
# ControlLeftMouseClick, ShiftLeftMouseClick, AltLeftMouseClick (Numpad Slash with Control, Shift or Alt).
command-modified-mouse-click = Clicks the left mouse button where the mouse is, with the key held.
# DragAndDrop (Control+Insert+Numpad Slash).
command-drag-and-drop = Drags with the mouse: pressed where the drag starts, then where it drops.
# MouseUp, MouseDown, MouseLeft, MouseRight (Alt+Shift with the arrows).
command-move-mouse = Moves the mouse a little that way.
# LeftMouseButtonLock (Insert+Numpad Slash).
command-left-mouse-lock = Holds the left mouse button down where the mouse is, or lets it go.
# RightMouseButtonLock (Insert+Numpad Star).
command-right-mouse-lock = Holds the right mouse button down where the mouse is, or lets it go.
command-restrict-okur-cursor = Restricts the Okur cursor to the next area: the application's window, the real window, the current window, the focused control's, the frame, or none.
command-route-pc-to-okur = Moves the PC cursor, or the virtual cursor, to the Okur cursor.
# WindowList (Insert+F10).
command-window-list = Lists the open windows, to switch to one.
# ListTaskTrayIcons (Insert+F11).
command-system-tray-list = Lists the icons of the notification area, to click one.
# JumpToLine (J in a page).
command-jump-to-line = Asks for a line number and moves to that line.
command-define-temp-placemarker = Marks the cursor's place in the page for this session, to come back to with K.
# SelectAPlaceMarker (Control+Shift+K in a page).
command-select-a-placemarker = Lists the page's PlaceMarkers, to move to one, or to add, rename or remove them.
# WordList (Control+Insert+W in a page).
command-word-list = Lists the page's words, most used first, to move to one.
# MoveToNextWordFromList (W in a page).
command-next-word-from-list = Moves to the next line with the word chosen in the Word List.
# MoveToPriorWordFromList (Shift+W in a page).
command-prior-word-from-list = Moves to the prior line with the word chosen in the Word List.
# SpeakPlaceMarkerN (7, 8, 9, 0 in a page).
command-speak-placemarker = Says the text at the page's PlaceMarker { $number } without moving there.
# MoveToPlaceMarkerN (Shift+7, 8, 9, 0 in a page).
command-move-to-placemarker = Moves to the page's PlaceMarker { $number }.
command-assign-custom-label = Gives the element at the cursor a name of your own, read in place of its own on its site's pages.
# JumpReturnFromLine (Shift+J in a page).
command-jump-return-from-line = Moves back to where the last jump to a line was made.

## Help

# ScreenSensitiveHelp (Insert+F1).
command-screen-sensitive-help = Tells what the control that has the focus is for, and its keys.
# The same, the longer description.
command-screen-sensitive-help-long = { command-screen-sensitive-help } The help opens in the Virtual Viewer, and Escape closes it.
# HotKeyHelp (Insert+H).
command-hot-key-help = Lists Okur's keys for the window or the page in front.
# The same, the longer description.
command-hot-key-help-long = { command-hot-key-help } Enter on a key in the list uses it.
# WindowKeysHelp (Insert+W).
command-windows-keys-help = Lists the keys Windows gives every application.

## Finding

# Control+F in a page.
command-virtual-find = Finds text on the page.
# The same, the longer description.
command-virtual-find-long = { command-virtual-find } F3 finds the next match, and Shift+F3 the previous one.
# Insert+Control+F: Okur Find, JAWS's JAWS Find.
command-okur-find = Opens Okur Find, to find text in the window.
# The same, the longer description.
command-okur-find-long = { command-okur-find } F3 finds the next match, and Shift+F3 the previous one.
# F3.
command-find-next = Finds the next match of the text last searched for.
# Shift+F3.
command-find-prior = Finds the previous match of the text last searched for.
# Finding given text: $text is what is looked for.
command-find = Looks for { $text } after the cursor and moves there.

## Lists of a page's elements

# SelectAHeading (Insert+F6).
command-select-a-heading = Lists the page's headings, to move to one.
# SelectALink (Insert+F7).
command-select-a-link = Lists the page's links, to move to one or follow it.
# SelectAFormField (Insert+F5).
command-select-a-form-field = Lists the page's form fields, to move to one.
# VirtualHTMLFeatures (Insert+F3).
command-virtual-html-features = Offers lists of the page's elements, such as its tables or buttons.
# Control+Insert and a letter. $elements is one of the elements messages below.
command-select-element = Lists the page's { $elements }, to move to one.
# What an element list lists, said in "Lists the page's buttons, to move to one.": A.
elements-radio-buttons = radio buttons
# B.
elements-buttons = buttons
# C.
elements-controls = combo boxes, list boxes and trees
# E.
elements-edits = edit fields
# X.
elements-check-boxes = check boxes
# G.
elements-graphics = graphics
# L.
elements-lists = lists
# O.
elements-articles = articles
# P.
elements-paragraphs = paragraphs
# Q.
elements-block-quotes = block quotes
# T.
elements-tables = tables
# Z.
elements-divisions = divisions
# Apostrophe.
elements-tabs = tabs
# SelectARegion (Control+Insert+R).
command-select-a-region = Lists the page's regions, to move to one.
# A heading chosen by its text: $name is the heading's text.
command-jump-to-heading = Moves to the heading { $name }.
# A link chosen by its text.
command-jump-to-link = Moves to the link { $name }.
# A form field chosen by its name.
command-jump-to-form-field = Moves to the form field { $name }.

## Windows, the focus and the time

# PassKeyThrough (Insert+3).
command-pass-key-through = The next key you press goes to the application instead of Okur.
# SayWindow (Insert+B).
command-say-window = Reads the dialog box in front.
# SayCurrentFocus (Insert+Tab).
command-say-current-focus = Says what has the focus.
# SayWindowTitle (Insert+T).
command-say-window-title = Says the title of the window in front.
# SaySystemTime (Insert+F12).
command-say-system-time = Says the time.
# The same, the longer description.
command-say-system-time-long = { command-say-system-time } Pressed twice quickly, it says the date.
# Control+Insert+F12.
command-say-system-time-with-seconds = Says the time with its seconds.
# The same, the longer description.
command-say-system-time-with-seconds-long = { command-say-system-time-with-seconds } Pressed twice quickly, it says the week of the year.
# SaySystemTime pressed twice quickly.
command-say-system-date = Says the date.
# Control+Insert+F12 pressed twice quickly.
command-say-week-number = Says the week of the year.

## The Virtual Viewer, fonts and colors

# Insert+Alt+W.
command-virtualize-window = Shows the window's text in the Virtual Viewer, to read it with the arrow keys.
# Insert+Shift+V.
command-virtualize-control = Shows the text of the control that has the focus in the Virtual Viewer.
# Insert+F.
command-say-font = Says the font at the cursor.
# The same, the longer description.
command-say-font-long = { command-say-font } Pressed twice quickly, it shows the font in the Virtual Viewer.
# Insert+F pressed twice quickly.
command-say-font-in-viewer = Shows the font at the cursor in the Virtual Viewer.
# SayColor (Insert+5).
command-say-color = Says the colors of the text and of its background at the cursor.

# KeyboardHelp (Insert+1).
command-keyboard-help = Turns Keyboard Help on or off: while it is on, keys say what they do instead of doing it.

## Okur's settings, and Okur itself

# Insert+V and Insert+6: one dialog for both.
command-open-settings = Opens Okur's settings.
# Insert+Shift+2.
command-set-punctuation-level = Changes how much punctuation is read, until Okur exits.
# Insert+2.
command-next-typing-echo = Changes what is said as you type: characters, words, both, or nothing, until Okur exits.
# ShutDownJAWS (Insert+F4).
command-shut-down = Exits Okur.
# The same, the longer description.
command-shut-down-long = { command-shut-down } When the setting to confirm is on, Okur asks first.
