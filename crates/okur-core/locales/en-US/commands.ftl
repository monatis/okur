## What commands say about themselves.

caps-lock-on = Caps lock on
caps-lock-off = Caps lock off
# Pass Key Through (Insert+3): the next key goes to the application as it is.
pass-key-through = Enter key to pass through
clipboard-copied-selection = Copied selection to clipboard
# Ctrl+X with text selected, in JAWS's lower case [obs, 504/s19].
clipboard-cut-selection = cut selection to clipboard
# Ctrl+V [obs, 504/s6].
clipboard-pasted = Pasted from clipboard
# Ctrl+Z [obs, 504/s7].
edit-undo = Undo
# Okur's own settings dialog could not open (there is no window to show it in).
settings-dialog-unavailable = The settings dialog is not available
# Okur's last words as it exits. What JAWS says as it exits is to be recorded (the proposal's
# J2, docs/references/startup-and-secure-screens-proposal.md); until then, Okur's own.
okur-exiting = Okur exiting.
# Said before a window's title (Insert+T), on a line of its own; with short messages the title
# is said alone [obs, 581/s12].
window-title-is = Title is
# Menus closed: the focus is back where it was.
menus-left = Leaving menus
# Said as a window's own menu bar is left from the bar itself (Alt, or Escape on the bar).
menu-bar-left = leaving menu bar
# A list's size, said after its name and role: "Fruits list | with 3 items".
list-items-count = with { $count } items

## The element lists with nothing to list

# The Links List on a page without links.
links-list-none = no links
# The Form Fields list on a page without fields. JAWS's lowercase, without a period.
form-fields-list-none = no form fields were found
# The Regions list on a page without regions. JAWS ends it without a period.
regions-list-none = No regions were found on the page

## JAWS Find (Insert+Control+F)

find-dialog = JAWS Find

## The Virtual Viewer

# The Virtual Viewer's window, and what is said when it has the focus.
viewer-title = Virtual Viewer
viewer-virtualizing-window = Virtualizing current window
viewer-virtualizing-control = Virtualizing current control
viewer-already-virtual = The current location is already in a virtual view
viewer-font-information = Font information:
# A link at the bottom of the Virtual Viewer's messages.
viewer-hot-keys-link = List JAWS Hot Keys
viewer-close-hint = Press ESCAPE to close this message.

## Editing text

# Text just selected with Shift and a move: said after a character, before more text.
selection-added = selected
# Control+Home in a document.
text-top-of-file = top of file
# Control+End in a document. JAWS's capital.
text-bottom-of-file = Bottom of file

## Say System Date (twice: the week)

week-number = Week { $week }

## Quick keys and the virtual cursor turned off and on [obs, 298]

quick-keys-navigation = Navigation Quick Keys
quick-keys-off = Off
quick-keys-say-all-only = Say all only
quick-keys-on = On
quick-keys-suspended = Quick Keys Suspended
quick-keys-resumed = Resuming Quick Keys
virtual-cursor-off = use virtual PC cursor off
virtual-cursor-on = use virtual PC cursor on
# Insert+Escape, Refresh Screen [obs, 298/s13, 512/s7].
screen-updated = Screen updated

## The voice rate changed with Ctrl+Alt+PageUp and PageDown [obs, 512]

rate-faster = Faster
rate-slower = Slower

## The Insert+Space layer [obs, 517]

# The Speech History viewer (Insert+Space, H), as JAWS names it on opening.
speech-history-title = Speech History

## Say Default Button (Insert+E) and the settings in use (Insert+Q) [obs, 513]

# Insert+E with no dialog box in front.
not-in-dialog-box = Not in a dialog box
# Insert+Q when the application has no settings of its own.
default-settings-loaded = Default settings are loaded
# Insert+Q: the application's executable, then spelled.
application-in-use = The application currently being used is { $app }
# Ctrl+Insert+V: the application's product name and version [obs, 513/s4].
app-version = { $product } Version { $version }

## A list view's columns, Ctrl+Insert+1 to 0 [obs, 514/s5-s7]

# A column past the last, then how many there are.
list-column-out-of-range = { $column } out of range
list-view-columns = this listview only contains { $count } columns.

## Routing the cursors and the page's frames [obs, 536]

# Insert+Delete: the virtual cursor goes to the focus.
route-virtual-to-pc = Route Virtual to PC
# Ctrl+Insert+Delete: the focus goes to the virtual cursor.
route-pc-to-virtual = Route PC to Virtual Cursor
# Insert+F9 on a page without frames.
no-frames-in-page = No frames found in page
# Numpad Slash, before the click on the element at the virtual cursor [obs, 534/s2].
left-mouse-button = left mouse button

## Say Default Button in a dialog box, and Say Current Control Hot Key (Shift+Numpad5) [obs, 537]

# Insert+E in a dialog box: its default button, the focused one when a button has the focus.
default-button-is = default button is { $name }
# Shift+Numpad5 on a control without one.
no-hot-key = no hot key
# Shift+Numpad5 in a dialog box, after the control's name: its letter as the label writes it.
hot-key-in-dialog = alt+{ $letter }

## When a page loads: its summary, before Say All reads it [obs, 507/s1, 538/s1, 539/s1]

# A page's regions, headings and links, the number run into the word as JAWS says it.
page-load-regions = { $count ->
    [one] { $count }Region
   *[other] { $count }Regions
}
page-load-headings = { $count ->
    [0] no headings
    [one] { $count }heading
   *[other] { $count }headings
}
page-load-links = { $count ->
    [0] no links
    [one] { $count }link
   *[other] { $count }links
}
page-load-summary-two = Page has { $first } and { $second }
page-load-summary-three = Page has { $first }, { $second } and { $third }
