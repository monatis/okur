## Okur's own dialogs, drawn by okur-ui. In a label, "&" marks the next letter as the
## control's access key (Alt and the letter): keep one in each label, on a letter of the
## label, and no two alike in one dialog.

## Okur Settings (Insert+6, Insert+V)

settings-dialog-title = Okur Settings
## What each setting does, as the settings tree shows it below the tree: `<label>-description`.

settings-synthesizer-description = The speech engine Okur speaks with. Automatic takes the system's OneCore voices, and SAPI 5 where there are none. No speech keeps Okur silent.
settings-voice-description = The voice Okur speaks with, among the synthesizer's. Default is the one the system chose.
settings-rate-description = How fast Okur speaks. Control+Alt+Page Up and Page Down change it while you work.
settings-volume-description = How loud Okur speaks, apart from the system's volume.
settings-punctuation-description = How many punctuation marks and symbols are spoken while reading: none, some, most or all. Spelling and a selection name every one.
settings-indicate-caps-description = Capital letters spoken with a higher pitch as you read by character.
settings-layout-description = The keyboard layout of Okur's keys: Desktop with the numeric keypad, or Laptop, where Caps Lock is the Okur key.
settings-okur-key-description = The key held for Okur's own commands, for this keyboard layout.
settings-typing-echo-description = What is spoken as you type: nothing, each character, each word as it ends, or both. Okur key and 2 goes through them.
settings-say-all-unit-description = Whether Say All reads by line, by sentence or by paragraph, which decides where it pauses.
settings-verbosity-description = How much Okur says of what it reads, from Beginner, which says the most, to Advanced, which says the least. Each level can be customized.
settings-forms-mode-description = Whether forms mode turns on by itself on a page: as the arrows reach a field (Auto), as Tab reaches one (Semi-Auto), or only with Enter (Manual).
settings-live-region-whole-text-description = Whether a live region that grows is read whole, or only what was added to it.
settings-viewer-on-screen-description = Whether the Virtual Viewer is drawn on the screen, for someone looking, or kept to speech.
settings-read-page-on-load-description = Whether a page or document is read from its start when it loads.
settings-confirm-exit-description = Whether Okur asks before it quits with Okur key and F4.
settings-mouse-echo-enabled-description = Whether Okur reads what the mouse rests on.
settings-mouse-echo-instant-description = Whether the mouse's text is read at once, or after the mouse rests a moment.
settings-mouse-echo-unit-description = How much of the mouse's text is read: the character, word, line or paragraph under it.

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
# JAWS's Caps Indicate: a capital read alone or spelled at a raised pitch.
settings-indicate-caps = Caps &Indicate
settings-group-keyboard = Keyboard
settings-layout = Keyboard &Layout:
settings-layout-desktop = Desktop
settings-layout-laptop = Laptop
# The Okur key, Okur's modifier: JAWS's "&JAWS Key", in Okur's name. Its access key is K:
# V&olume has O.
settings-okur-key = Okur &Key:
settings-okur-key-insert = Insert
settings-okur-key-caps-lock = Caps Lock
settings-okur-key-scroll-lock = Scroll Lock
settings-group-reading = Reading
settings-typing-echo = Typing &Echo:
settings-typing-echo-off = Off
settings-typing-echo-characters = Characters
settings-typing-echo-words = Words
settings-typing-echo-both = Characters and Words
# Where the dialog's settings are kept, as JAWS's Settings Center's Application box chooses it: all
# applications (JAWS's words), the application in front (its name), the site of its page.
settings-scope = Settings &for:
settings-scope-all = Default (All Applications)
settings-scope-site = { $site } (web site)
# JAWS's Say All Reads: what Say All reads at a time, and what Left and Right during it move
# by. The other letters of its name are taken.
settings-say-all-unit = Say All Rea&ds:
settings-say-all-unit-line = Line
settings-say-all-unit-sentence = Sentence
settings-say-all-unit-paragraph = Paragraph
# JAWS's user verbosity levels (Settings Center): what is said, and how much.
settings-verbosity = Ver&bosity Level:
settings-verbosity-beginner = Beginner
settings-verbosity-intermediate = Intermediate
settings-verbosity-advanced = Advanced
# Each level's spoken items and message length, as JAWS's Settings Center customizes them
# ("Customize Verbosity Level": a level, then its check boxes).
settings-group-verbosity-levels = Verbosity Levels
settings-verbosity-customize = Customize Verbosity Level:
settings-verbosity-length = Message Length:
settings-verbosity-length-long = Long
settings-verbosity-length-short = Short
settings-output-access-key = Access Key
settings-output-hint = Tutor Messages
settings-output-list-position = Announce Position and Count When Navigating in Lists
settings-output-description = Control Description
settings-output-control-group-name = Control Group Name
settings-output-select = Select
settings-output-okur-message = Okur Messages
settings-output-control-type = Control Type
settings-output-item-state = Item State
settings-output-position = Position Information
# JAWS's Speech Mode: Full Speech or Less Speech, and Less Speech's kind, Speech On Demand or
# Mute (Settings Center help).
settings-speech-mode-full = Full Speech
settings-speech-mode-less = Less Speech
settings-less-speech-on-demand = Speech On Demand
settings-less-speech-muted = Mute
settings-live-region-whole-text = Read the &Whole Text of a Growing Live Region
settings-viewer-on-screen = &Show Virtual Viewer on Screen
# JAWS's "Document and Web Pages Automatically Read When Loaded" (Settings Center,
# Web/HTML/PDFs, Reading). Its D is Say All Reads', its M Forms Mode's.
settings-read-page-on-load = Docume&nt and Web Pages Automatically Read When Loaded
# JAWS's Forms Mode (Settings Center): when forms mode turns on by itself.
settings-forms-mode = For&ms Mode:
settings-forms-mode-auto = Auto
settings-forms-mode-semi-auto = Semi-Auto
settings-forms-mode-manual = Manual
# JAWS's "Prompt to Confirm When Exiting JAWS" (Options > Basics): Insert+F4 asks first.
settings-confirm-exit = Prompt to &Confirm When Exiting Okur
# JAWS's Mouse Echo group (Settings Center): what the mouse points to said as it moves.
settings-group-mouse-echo = Mouse Echo
settings-mouse-echo-enabled = Enable Mouse Ec&ho
settings-mouse-echo-instant = Mouse Echo Ins&tant
settings-mouse-echo-unit = Mouse Echo &Unit:
settings-mouse-echo-unit-character = Character
settings-mouse-echo-unit-word = Word
settings-mouse-echo-unit-line = Line
settings-mouse-echo-unit-paragraph = Paragraph
settings-ok = OK
settings-cancel = Cancel
settings-apply = &Apply

## The question when Settings Center is cancelled with changes not applied, JAWS's words with
## Okur's name: "JAWS Settings Center dialog | You have made changes to NotePad's settings. Do
## you want to save them? | Yes Button", Tab to "No Button", "Cancel Button" [obs, the walks of
## Settings Center's questions].

settings-save-title = Okur Settings Center
settings-save-question = You have made changes to { $place }'s settings. Do you want to save them?
settings-save-yes = &Yes
settings-save-no = &No
settings-save-cancel = Cancel

## The question before Okur exits (Insert+F4, JAWS's ShutDownJAWS), JAWS's words with Okur's name:
## "Quit JAWS dialog | Are you sure you want to quit JAWS?  Press Enter to quit now, or Escape
## to cancel. | OK Button", and Tab to "Cancel Button" [obs, 571/s1, s2].

quit-title = Quit Okur
quit-question = Are you sure you want to quit Okur?  Press Enter to quit now, or Escape to cancel.
quit-ok = OK
quit-cancel = Cancel

## Okur's message box, JAWS's words with Okur's name: "JAWS for Windows Message Box dialog | OCR
## Failed to start. No file currently selected. | OK Button" [obs, 1026/s3].

message-box-title = Okur for Windows Message Box
message-box-ok = OK

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

## Custom Label (Insert+Ctrl+Tab in a page): JAWS 2025's dialog [obs, 688/s9], its text and
## its field named alike.

custom-label-title = Custom Label
custom-label-prompt = Enter a custom label for this item
custom-label-ok = OK
custom-label-cancel = Cancel
# Insert+Ctrl+Tab where there is nothing to label (a paragraph): no dialog opens (JAWS, live).
custom-label-not-here = A Custom Label cannot be created for this item.

## The PlaceMarker List (Ctrl+Shift+K in a page): JAWS 2025's dialog [obs, 699, 702]. An "&"
## marks a label's access key, as in JAWS's dialog.

placemarker-list-title = PlaceMarker List
# The list's name and its first column's header.
placemarker-list-markers = PlaceMarker
placemarker-list-column-domain = Generalized to Domain
# The second column of a PlaceMarker kept for its page alone, or for its whole site.
placemarker-scope-document = Document
placemarker-scope-domain = Domain
# The temporary PlaceMarker (Ctrl+Windows+K), as the list names it.
placemarker-temporary = Temporary
placemarker-list-add = &Add...
placemarker-list-move-to = &MoveTo
placemarker-list-change = C&hange...
placemarker-list-remove = &Remove
placemarker-list-remove-all = Remo&ve All
placemarker-list-cancel = &Cancel
placemarker-list-display = Display PlaceMarkers For:
placemarker-list-display-url = &URL
placemarker-list-display-domain = &Domain
placemarker-list-display-all = A&ll
# Add, from the list: the text at the cursor is proposed as the name [obs, 702/s5].
placemarker-add-title = Add PlaceMarker
# Change, from the list: the chosen PlaceMarker's name.
placemarker-change-title = Change PlaceMarker
placemarker-add-name = &Name:
placemarker-add-domain = &Define for all pages on current domain
# Found by this text where the page's text moved, not kept at its character.
placemarker-add-anchored = &Anchor to text
placemarker-add-anchor-text = Anchor &Text:
placemarker-add-ok = &OK
placemarker-add-cancel = &Cancel
# Remove All, from the list [obs, 702/s27].
placemarker-remove-all-title = PlaceMarker RemoveAll Warning
placemarker-remove-all-question = Are you sure you want to remove all PlaceMarkers?
placemarker-remove-all-yes = &Yes
placemarker-remove-all-no = &No

## Flexible Web (Insert+Space, then X): its customizations, as JAWS's wizard names them
## [obs, 705, 708, 716, 720; the guide, Surf's Up chapter 16]. $kind is an element's kind as
## the wizard names it ("MainRegion", "heading level 1"), $kinds its plural.

flexible-web-heading = heading level { $level }
flexible-web-frame = frame
flexible-web-hide-from-top = Hide { $kind } { $number } from the top
flexible-web-hide-from-bottom = Hide { $kind } { $number } from the bottom
flexible-web-hide-all = Hide all { $kinds }
flexible-web-start-from-top = Start at { $kind } { $number } from the top
flexible-web-start-from-bottom = Start at { $kind } { $number } from the bottom
flexible-web-start-any = Start at any { $kind }
# What a customization's element has, after where it is: "Hide all heading level 1s with id =
# "events"" [obs, 734/s8].
flexible-web-with-text = with text: "{ $text }"
flexible-web-with-id = with id = "{ $id }"
flexible-web-with-text-and-id = with text: "{ $text }", and with id = "{ $id }"

## Flexible Web's wizard, its pages as JAWS's are [obs, 705, 708, 716, 720, 726]. A radio
## button's "&" marks its access key, as the wizard's own labels do.

flexible-web-title = Flexible Web
flexible-web-start-question = What would you like to do?
flexible-web-create = Create a new customization
# Retry and Undo name the customization made last; with none, two spaces and three dots.
flexible-web-retry = &Retry { $customization }
flexible-web-retry-none = &Retry  . . .
flexible-web-undo = &Undo { $customization }
flexible-web-undo-none = &Undo  . . .
flexible-web-save = &Save temporary customizations as a rule
flexible-web-view = &View or change where rules apply
flexible-web-enable = Enable Flexible Web
flexible-web-action-title = Choose an Action
flexible-web-action-question = How would you like to customize the page?
flexible-web-hide = &Hide an element
flexible-web-start-reading = &Start reading at an element
flexible-web-element-title = Choose an element
flexible-web-element-question = Which of the following elements would you like to customize?
flexible-web-customization-title = Choose a customization
flexible-web-customization-question = Select a customization
# Insert+Space, X outside a page.
flexible-web-not-in-page = You must be in the web page area to activate Flexible Web.
flexible-web-no-elements = No customizable elements were found at the current virtual PC cursor location. Please change the cursor's location and try again.
# Saving the temporary customizations as a rule: the last page, its name, its customizations
# and where it applies [obs, 740, 742/s16-s28].
flexible-web-save-title = Save Temporary Customizations
flexible-web-rule-name = Rule Name:
flexible-web-scope-question = How would you like this rule to apply?
flexible-web-scope-everywhere = Globally
flexible-web-scope-site = To the current site
flexible-web-scope-page = To the current page
# View or Change Where Rules are Applied, from View or change where rules apply: its text,
# its tree of areas, its rules list with Set for, and the question before a rule is deleted
# [obs, 748].
flexible-web-rules-title = View or Change Where Rules are Applied
flexible-web-rules-text = Use the tree view below to select the area of the web where you want to make changes. The tree is organized into three areas: global, specific sites, and specific pages. Use the list of rules to choose which rules apply to the area selected in the tree view. Any changes to the settings of the smaller areas override the settings for the larger areas. Use the context menu in the list to view, rename or delete rules.
flexible-web-area-global = Global
flexible-web-area-site = Current Site: { $site }
flexible-web-area-page = Current Page: { $page }
# How many rules are on in an area, after its place in the tree [obs, 748/s8, s13].
flexible-web-area-active = { $count } Active
flexible-web-rules-column-rule = Rule
flexible-web-rules-column-set-for = Set for
flexible-web-set-for-global = Global
flexible-web-set-for-site = Site
flexible-web-set-for-page = Page
flexible-web-rules-close = Close
flexible-web-delete-title = Okur
flexible-web-delete-question = You are about to delete rule "{ $rule }". If you delete a rule, it will no longer be available for any sites or pages. To disable the rule for the location selected in the tree view, press the spacebar to uncheck the rule. When a rule is disabled, it is still available to run for other sites or pages. Continue?
flexible-web-delete-yes = &Yes
flexible-web-delete-no = &No
# Another page loading while the one before has temporary customizations [obs, 742/s30].
flexible-web-unsaved-question = You have created temporary Flexible Web customizations. If you continue without saving them, these customizations will be lost. Would you like to save your temporary customizations?
flexible-web-unsaved-yes = &Yes
flexible-web-unsaved-no = &No

## Skim Reading (Control+Shift+Insert+Down), as JAWS's [obs, 760, 763].

skim-dialog = Skim Reading
skim-rule = Use Rule Type:
skim-rule-first-line = Read First Line of Every Paragraph
skim-rule-first-sentence = Read First Sentence of Every Paragraph
skim-rule-text = Text Rules Apply for Reading
skim-rule-attributes = Attributes and Color Rules Apply for Reading
skim-rules-history = &Rules History:
# The rules history's entry before any rule is made.
skim-rules-new = <New>
skim-pattern = Text Search &Pattern:
skim-regex = Use regular e&xpression
skim-unit = Speak &Unit:
skim-unit-lines = Lines Containing Text Rule
skim-unit-sentences = Sentences Containing Text Rule
skim-unit-paragraphs = Paragraphs Containing Text Rule
# The group around the colors and the attributes [obs, 777/s16].
skim-color-rules = Color or Attribute Rules
skim-delete = &Delete Rule
skim-delete-all = Delete A&ll Rules
skim-text-color = Text Color:
skim-background = Background Color:
# A color rule's color before one is chosen: any.
skim-any-color = *
skim-bold = Bold
skim-italic = Italic
skim-underline = Underline
skim-strikeout = Strikeout
skim-double-strikeout = Double Strikeout
skim-superscript = Superscript
skim-subscript = Subscript
skim-shadow = Shadow
skim-outline = Outline
skim-indication = Skim Reading &Indication
skim-start = &Start Skim Reading
skim-summary = Create Summar&y
skim-close = &Close
skim-help = &Help
# Said after skim reading by an attribute rule [obs, 799/s10].
skim-matches-found = { $count } matches found
# The viewer Create Summary opens [obs, 763/s23].
skim-results-title = Skim Reading Results

## Settings Center and Quick Settings: a search box over a tree of groups and settings, as JAWS's
## (old-pc's walk, docs/references/okur-settings-redesign.md §1.1).

# "NotePad - JAWS Settings Center", with Okur's name.
settings-center-title = { $place } - Okur Settings Center
# "QuickSettings - Chrome", as JAWS writes its own.
quick-settings-title = QuickSettings - { $place }
quick-settings-expand = Expand Tree View
settings-tree-search = SearchBox
# What each key typed in the search box says: the first setting found, then this; or "No matches
# found." [obs, the walk of Settings Center's search].
settings-search-results = 1 of { $count } Search Results
settings-search-none = No matches found.
settings-tree-application = Application:
settings-tree-group-description = A group of settings: Right Arrow opens it, and Space on a setting in it changes its value.
# JAWS's group names, in its order; Quick Settings' own after them.
settings-tree-user = User
settings-tree-web = Web / HTML / PDFs
settings-tree-reading = Reading
settings-tree-text-processing = Text Processing
settings-tree-general = General
settings-tree-forms-mode = Forms Mode
settings-tree-speech-verbosity = Speech Verbosity
settings-tree-say-all = Say All
settings-tree-synthesizer = Synthesizer
settings-tree-punctuation = Punctuation
settings-tree-keyboard = Keyboard
settings-tree-mouse-echo = Mouse Echo
settings-tree-miscellaneous = Miscellaneous
# Speech Verbosity's item that opens the verbosity levels' own dialog, as JAWS's.
settings-tree-configure-verbosity = Configure Verbosity Levels
settings-tree-configure-verbosity-description = Space opens a dialog to choose, for each verbosity level, what is spoken and whether messages are long or short.
settings-tree-general-options = General Options
settings-tree-say-all-options = Say All Options
settings-tree-editing-options = Editing Options
# Settings named in the tree as JAWS's tree names them, where its dialogs' labels differ.
settings-forms-mode-tree = Select Forms Mode
settings-punctuation-tree = Punctuation Level
settings-verbosity-quick = User Verbosity
settings-say-all-unit-quick = Say All Reads By
settings-tree-links = Links
settings-tree-numbers = Number And Date Processing
settings-tree-speech-options = Speech Options
# Settings Settings Center names that the settings dialog does not show, by JAWS's names there.
settings-tree-mixed-case = Mixed case processing
settings-tree-mixed-case-description = Whether a word written in mixed case is spoken in its parts, "get Element By Id", or whole.
settings-tree-say-dollars = Speak Dollars
settings-tree-say-dollars-description = Whether amounts of money are left to the synthesizer as written, rather than read by Okur.
settings-tree-speech-mode = Speech Mode
# Screen Echo, in User, and Smart Navigation Mode, in Web / HTML / PDFs > Navigation, with their
# values as Settings Center names them [obs, the walk of Settings Center's search].
settings-tree-screen-echo = Screen Echo
settings-tree-screen-echo-description = How much of the text that changes on screen Okur reads by itself: none, the highlighted text, or all of it. Okur key and S changes it while you work.
settings-screen-echo-none = Echo No Text
settings-screen-echo-highlighted = Echo Highlighted Text
settings-screen-echo-all = Echo All Text
settings-tree-navigation = Navigation
settings-tree-smart-navigation = Smart Navigation Mode
settings-tree-smart-navigation-description = Whether the virtual cursor's arrow keys move through form controls and tables as their own units. Okur key and X turns it on and off while you work.
settings-smart-navigation-off = Off
settings-smart-navigation-controls = Controls
settings-smart-navigation-tables = Tables
settings-smart-navigation-controls-and-tables = Controls and Tables
# Indicate Headings, in Web / HTML / PDFs > Headings and Frames [obs, the walk of Settings
# Center's search; "Headings Only" from its help].
settings-tree-headings-and-frames = Headings and Frames
settings-tree-indicate-headings = Indicate Headings
settings-tree-indicate-headings-description = How a heading is said when you move to it: its text alone, with the word heading, or with the word heading and its level.
settings-indicate-headings-none = No Headings
settings-indicate-headings-only = Headings Only
settings-indicate-headings-level = Headings with Level
settings-tree-speech-mode-description = Full Speech, or less speech, as Okur starts. Okur key and Space, then S, changes it while you work.
settings-tree-less-speech = Less Speech Options
settings-tree-less-speech-description = What less speech does: speak only when asked (on demand), or say nothing.
settings-tree-same-page-links = Identify "Same Page" Links
settings-tree-same-page-links-description = Whether a link to another place on the same page is said as a same page link.
settings-tree-indentation = Say Indented Characters
settings-tree-indentation-description = Whether a line's indentation is said as it is read.
settings-tree-mouse-echo-interrupt = Mouse Echo Interrupt
settings-tree-mouse-echo-interrupt-description = Whether the mouse moving onto text stops what is being said.
settings-tree-mouse-echo-description = Speak description of item
settings-tree-mouse-echo-description-description = Whether the mouse's item is read with its description.
settings-tree-mouse-echo-control = Speak control type and state of item
settings-tree-mouse-echo-control-description = Whether the mouse's item is read with its kind and state, "button" or "checked".
# A setting kept only for all applications, in an application's settings [obs, the walk of
# Settings Center: "Insert Key ModeUnavailable | Only available when modifying the Default
# setting. Press Control Shift D to switch to Default."].
settings-tree-unavailable = Unavailable
settings-tree-default-only = Only available when modifying the Default setting. Press Control Shift D to switch to Default.
# The tree's last group: the settings changed lately for the place [obs, the walks of Quick
# Settings and Settings Center].
settings-tree-last-changed = Last Changed Settings
settings-tree-last-changed-description = The 25 settings changed last for this place, newest first. Each can be changed here as in its own group.
# Quick Settings' lifetimes, Shift+F10 on a setting or a group, JAWS's words with Okur's name:
# "Context Menu | Restore setting when JAWS exits", "Restore setting when focus changes", "Save
# the setting checked" [obs, the second walk of Settings Center's questions].
settings-lifetime-until-exit = Restore setting when Okur exits
settings-lifetime-until-focus-moves = Restore setting when focus changes
settings-lifetime-saved = Save the setting
# Shift+F10 in Last Changed Settings: an entry's menu, then the group's [obs, the walk of Last
# Changed Settings].
settings-last-changed-remove = Remove
settings-last-changed-remove-all = Remove All

## Okur's main window (Insert+J) and its menus: JAWS's window and its items, those Okur has
## (old-pc's walk, docs/references/okur-settings-redesign.md §1.1). Access keys from JAWS's book.

okur-window-title = Okur
okur-window-version = Okur version { $version }
okur-menu-options = &Options
okur-menu-basics = &Basics...
okur-menu-exit = E&xit
okur-menu-utilities = &Utilities
okur-menu-notification-history = Notification History
okur-menu-settings-center = Settings Center
okur-menu-help = &Help
okur-menu-about = &About Okur...
okur-menu-startup-wizard = Startup Wi&zard...

## Basics, JAWS's "Basic Settings" [obs, the walk of Basics].

basics-title = Basic Settings
basics-tutor = Tutor Messages
basics-tutor-off = Turn Off Menu and Control Help
basics-tutor-on = Announce Menu and Control Help
basics-access-key = Access Key
basics-access-key-off = Off
basics-access-key-all = Speak All
basics-confirm-exit = Prompt to confirm when exiting Okur
basics-viewer = Show Virtual Viewer on screen
basics-layout = Use Keyboard Layout:
# The Startup Wizard (Help, Startup Wizard..., and Okur's first start): its title, its pages'
# titles and their settings, as JAWS's [obs, the walk of the Startup Wizard].
startup-wizard-title = Okur Startup Wizard
startup-wizard-speech = Speech Settings
startup-wizard-rate = Rate:
startup-wizard-punctuation = Punctuation:
startup-wizard-typing-echo = Typing Echo
startup-wizard-run = Run Okur Settings
startup-wizard-common = Common Options
startup-wizard-smart-navigation = Smart Navigation
startup-wizard-navigation-off = Off
startup-wizard-navigation-controls = Controls Only
startup-wizard-navigation-tables = Tables
startup-wizard-navigation-both = Controls and Tables
startup-wizard-forms-mode = Forms Mode
startup-wizard-forms-auto = Auto
startup-wizard-forms-semi-auto = Semi Auto
startup-wizard-forms-manual = Manual
startup-wizard-access-key-all = Speak all
startup-wizard-verbosity = Verbosity Settings
startup-wizard-verbosity-level = Verbosity Level
startup-wizard-beginner = Beginner, Highest
startup-wizard-intermediate = Intermediate
startup-wizard-advanced = Advanced, Lowest
# The keys the menus show after their items, as JAWS's show theirs.
okur-menu-exit-key = Alt+F4
okur-menu-notification-history-key = INSERT+SPACE, N
okur-menu-voices = &Voices
okur-menu-voice-adjustment = Voice Adjustment...
okur-menu-voice-profile = Select a Voice Profile...
okur-menu-applications = Manage Application Settings...
okur-menu-restore = Restore to Factory Settings...

## Voice Adjustment, JAWS's "Voice" group [obs, the walk of Voice Adjustment].

voice-adjustment-title = Voice Adjustment
voice-adjustment-profile = Profile Name:
voice-adjustment-delete = Delete Profile
voice-adjustment-save-as = Save As...
# Save As's question: "Save Voice Profile dialog | Voice Profile Name: edit combo" [obs, the walk
# of Voice Adjustment's profiles].
voice-adjustment-save-title = Save Voice Profile
voice-adjustment-save-name = Voice Profile Name:
voice-adjustment-group = Voice
voice-adjustment-adjust = Adjust:
voice-adjustment-all-contexts = All Contexts
# The voice contexts, JAWS's names with Okur's: "PC Cursor Voice | 2 of 5", "JAWS Cursor Voice",
# "Keyboard Voice", "Tutor and Message Voice" [obs, the walk of Voice Adjustment's contexts].
voice-adjustment-pc-cursor = PC Cursor Voice
voice-adjustment-okur-cursor = Okur Cursor Voice
voice-adjustment-keyboard = Keyboard Voice
voice-adjustment-message = Tutor and Message Voice
voice-adjustment-person = Person:
voice-adjustment-rate = Rate:
voice-adjustment-punctuation = Punctuation:
voice-adjustment-pitch = Pitch:
voice-adjustment-volume = Volume:

## Select a Voice Profile (Insert+Ctrl+S): "Select a Voice Profile dialog | Default: Combo box",
## "For notepad: Combo box | Use Default | 1 of 6" [obs, the walk of Select a Voice Profile].

voice-profile-title = Select a Voice Profile
voice-profile-default = Default:
voice-profile-for = For { $application }:
voice-profile-use-default = Use Default
# A synthesizer's own profile: "SAPI 5X - (factory)" [obs, the walk of Voice Adjustment's
# profiles].
voice-profile-factory = { $synthesizer } - (factory)

## Manage Application Settings: "Manage Application Settings dialog | Application Settings List
## box | Chrome checked | 1 of 5", Enable All, Disable All, Remove Selected, OK, Cancel [obs, the
## walk of JAWS's Options].

applications-title = Manage Application Settings
applications-list = Application Settings
applications-enable-all = Enable All
applications-disable-all = Disable All
applications-remove = Remove Selected

## Restore to Factory Settings' question, JAWS's words but for "and restart JAWS": Okur's
## defaults apply at once [obs, the walk of JAWS's Options].

restore-factory-title = Restore to Factory Settings
restore-factory-question = This option will delete all your current settings. Are you sure you want to continue?
restore-factory-yes = &Yes
restore-factory-no = &No
