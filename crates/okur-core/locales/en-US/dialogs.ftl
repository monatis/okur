## Okur's own dialogs, drawn by okur-ui. In a label, "&" marks the next letter as the
## control's access key (Alt and the letter): keep one in each label, on a letter of the
## label, and no two alike in one dialog.

## Okur Settings (Insert+6, Insert+V)

settings-dialog-title = Okur Settings
settings-group-speech = Speech
settings-synthesizer = S&ynthesizer:
# Okur picks OneCore, else SAPI 5, else none.
settings-synthesizer-automatic = Automatic
settings-synthesizer-onecore = OneCore
settings-synthesizer-sapi5 = SAPI 5
settings-synthesizer-none = No speech
settings-voice = &Voice:
# The voice list's first entry: the synthesizer's own default voice.
settings-voice-default = Default
settings-rate = &Rate:
settings-volume = V&olume:
# JAWS's Voice Adjustment's Punctuation; the levels are punctuation-none to punctuation-all.
settings-punctuation = &Punctuation:
settings-group-keyboard = Keyboard
settings-layout = Keyboard &Layout:
settings-layout-desktop = Desktop
settings-layout-laptop = Laptop
# The key that acts as the JAWS key (Okur's modifier).
settings-jaws-key = &JAWS Key:
settings-jaws-key-insert = Insert
settings-jaws-key-caps-lock = Caps Lock
settings-jaws-key-scroll-lock = Scroll Lock
settings-group-reading = Reading
settings-typing-echo = Typing &Echo:
settings-typing-echo-off = Off
settings-typing-echo-characters = Characters
# JAWS's user verbosity levels (Settings Center): what is said, and how much.
settings-verbosity = Ver&bosity Level:
settings-verbosity-beginner = Beginner
settings-verbosity-intermediate = Intermediate
settings-verbosity-advanced = Advanced
settings-live-region-whole-text = Read the &Whole Text of a Growing Live Region
settings-viewer-on-screen = &Show Virtual Viewer on Screen
# JAWS's "Prompt to Confirm When Exiting JAWS" (Options > Basics): Insert+F4 asks first.
settings-confirm-exit = Prompt to &Confirm When Exiting Okur
settings-ok = OK
settings-cancel = Cancel
settings-apply = &Apply

## The question before Okur exits (Insert+F4, JAWS's ShutDownJAWS), JAWS's words with Okur's name:
## "Quit JAWS dialog | Are you sure you want to quit JAWS?  Press Enter to quit now, or Escape
## to cancel. | OK Button", and Tab to "Cancel Button" [obs, 571/s1, s2].

quit-title = Quit Okur
quit-question = Are you sure you want to quit Okur?  Press Enter to quit now, or Escape to cancel.
quit-ok = OK
quit-cancel = Cancel

## The Links List (Insert+F7), as JAWS's. Its title and its list's name are list-dialog-links
## and list-dialog-links-list.

links-list-move = &Move To Link
links-list-activate = &Activate Link
links-list-cancel = &Cancel
links-list-sort = Sort Links
links-list-sort-tab-order = In &Tab Order
links-list-sort-alphabetical = Al&phabetically
links-list-display = Display
links-list-display-all = A&ll Links
links-list-display-visited = &Visited Links Only
links-list-display-unvisited = &Unvisited Links

## The Heading List (Insert+F6), as JAWS's. Its title and its list's name are
## list-dialog-headings and list-dialog-headings-list.

heading-list-move = &Move To Heading
heading-list-cancel = &Cancel
heading-list-sort = Sort Headings
heading-list-sort-tab-order = In &Tab Order
heading-list-sort-alphabetical = Al&phabetically
heading-list-display = Display
# Alt+L chooses it, as in JAWS.
heading-list-display-all = A&ll Headings
# One option per heading level, 1 to 6; Alt and the level's digit choose it, as in JAWS.
heading-list-display-level = Level &{ $level } Headings

## Select a Form Field (Insert+F5), as JAWS's. Its title and its list's name are
## list-dialog-form-fields and list-dialog-form-fields-list.

# Enter chooses it, as in JAWS: JAWS's labels have no access key.
form-field-list-ok = OK
form-field-list-cancel = Cancel

## Virtual HTML Features (Insert+F3), as JAWS's. Its title and its list's name are
## list-dialog-features and list-dialog-features-list.

# Enter chooses it, as in JAWS: JAWS's labels have no access key.
features-ok = OK
features-cancel = Cancel

## JAWS Find (Ctrl+F in a page, Insert+Ctrl+F): JAWS 2025's labels and access keys. Its title
## is find-dialog.

find-what = Find &what:
find-context-history = Conte&xt Specific History
find-maintain-history = &Maintain History of Recent Finds
find-direction = Direction
find-forward = F&orward
find-reverse = Re&verse
find-ignore-case = &Ignore Case
find-find = Find
find-delete = Delete
find-clear-history = Clear Find History
find-cancel = Cancel

# Ctrl+F in a page, said before JAWS Find opens [obs, 506/s1].
virtual-find = Virtual Find

## The Window List (Insert+F10): JAWS 2025 help, Window List Dialog Box [obs, 511/s1]

window-list-title = Window List
window-list-running = &Running Applications
window-list-on-top = &Only Include On Top Windows
window-list-switch = &Switch To
window-list-cancel = Cancel
# A window's state, after its title in the list.
window-minimized = Minimized
window-maximized = Maximized
window-restored = Restored

## The system tray list (Insert+F11): JAWS 2025 help, Select a System Tray Icon Dialog Box.
## Its list has no label; JAWS names it by the dialog [obs, 511/s4].

tray-title = Select a System Tray Icon
tray-right-click = Right Single Click
tray-left-click = Left Single Click
tray-left-double-click = Left Double Click
tray-cancel = Cancel

## Jump To Line (J in a page): JAWS 2025's dialog [obs, 559/s1]; JAWS's help names the
## line count and the current line it shows.

jump-title = Jump To Line
# Its field's label: the page's number of lines.
jump-line-number = Enter Line Number (1 to { $count }):
jump-ok = OK
jump-cancel = Cancel
# Shift+J before any jump [obs, 559/s7].
jump-none = No previous jump
