# Okur's settings, redesigned as JAWS's

Date: 2026-10-24. The owner's request (2026-10-24): settings and customization are now the biggest
gap; JAWS has a window with a menu bar (Insert+J), Basics, Voices with profiles, Settings Center
and Quick Settings as a search box over a tree, and Okur has one flat dialog. This is the design;
it is built in the milestones of §6, each found in JAWS's documentation, recorded on old-pc, then
built (the owner's rule).

Sources: [`jaws-settings-ui-research.md`](jaws-settings-ui-research.md) (JAWS's help, tagged
[R-SUI-nn]), [`jaws-customization-map.md`](jaws-customization-map.md) (which settings exist, their
scopes, and the settings table of §7 there), and Okur's own black-box walk of Quick Settings in
Notepad on old-pc (2026-10-24, research, not evidence; §3.1). Okur names itself where JAWS does
(AGENTS.md §1.5): "Run Okur from System Tray", "Okur Settings Center".

## 1. What a JAWS user expects

| Surface | Keys | JAWS 2025 |
|---|---|---|
| The main window | Insert+J; Alt lands on Options | menus Options, Utilities, Language, Help (O, U, L, H) [R-SUI-01], [R-SUI-35] |
| Options | Alt+O | Basics..., Voices ▸, Braille, Manage Application Settings, Restore to Factory Settings, Remote Desktop, Early Adopter Program, Exit [R-SUI-02] |
| Voices ▸ | Alt+O, V | Voice Adjustment..., Select a Voice Profile..., Add/Remove Voices... [R-SUI-02] |
| Utilities | Alt+U | ... Settings Center, Keyboard Manager, Dictionary Manager, ..., Import/Export Settings ▸, Convenient OCR ▸, ... [R-SUI-03] |
| Basics | Alt+O, B | Tutor Messages, Access Keys, Automatically Start, Prompt to Confirm When Exiting, Automatic Notification of Updates, Run from System Tray, Show Virtual Viewer on Screen, Use Keyboard Layout, Windows System Parameters, OK, Cancel [R-SUI-06] |
| Voice Adjustment | Alt+O, V, Enter | Profile Name, Synthesizer Language, Speak Sample, Adjust (All Context, PC Cursor, JAWS Cursor, Keyboard, Tutor and Message), Person, Rate, Punctuation, Pitch, Volume, Spell Rate Change, Upper Case Pitch Change; OK, Save As, Apply, Cancel [R-SUI-07] |
| Select a Voice Profile | Insert+Ctrl+S | Default and "For <application>" combo boxes [R-SUI-08] |
| Settings Center | Insert+6 | search box, tree of groups and settings, description, Apply, OK, Cancel, Application combo box; Ctrl+Shift+D for Default [R-SUI-23], [R-SUI-37] |
| Quick Settings | Insert+V | the same for the application in front, its own groups first; lifetimes on Shift+F10; Last Changed Settings [R-SUI-18], [R-SUI-37] |
| Startup Wizard | Help, Startup Wizard | seven pages of the most common settings [R-SUI-12] |

### 1.1 Observed on old-pc (research walk, 2026-10-24; `jaws-window-walk.ps1`)

JAWS 2025.2505.43's window, walked with navigation keys from Notepad; speech as JAWS's history
kept it (research, not evidence):
- **Alt**: "Menu bar | Options | To navigate press Left or Right Arrow."; Right: "Utilities",
  "Language", "Help"; Escape twice: "leaving menu bar". Down opens a menu: "Options menu |
  Basics... | To move through items press up or down arrow."; a submenu item is "Voices sub
  menu"; an item with a key says it after a tab, "Notification History INSERT+SPACE, N".
- **Options**: Basics..., Voices ▸ (Voice Adjustment..., Select a Voice Profile..., Add/Remove
  Voices...), Braille..., Early Adopter Program..., Manage Application Settings..., Restore to
  Factory Settings..., Remote Desktop..., Exit (Alt+F4). Menus wrap.
- **Utilities**: JAWS Tandem ▸, Voice Assistant ▸, Notification History (INSERT+SPACE, N), Screen
  Shade (INSERT+SPACE, F11), Synthesizer and Braille Manager, Sound Cards ▸, Settings Center,
  Keyboard Manager, Dictionary Manager, Frame Viewer, Script Manager, Navigation Quick Key Manager
  ▸, Re-Initialize Authorization, Import/Export Settings ▸, Explore Utilities Folder, Convenient
  OCR (INSERT+SPACE,O layer) ▸, Picture Smart (INSERT+SPACE,P layer) ▸, Braille and Text Viewer
  ▸.
- **Language**: JAWS Language ▸ (Unavailable: no other language installed), Voice Profiles ▸.
- **Help**: Commands Search (INSERT+SPACE, J), Message Center (35), Training, JAWS Help Topics
  (F1), Keyboard Commands, What's New, Web Resources ▸, Technical Support, About JAWS..., Startup
  Wizard..., Support Tool..., License ▸, Check for Updates...
- **Basics** (Enter on Basics...): "Leaving menus | Basic Settings dialog | Tutor Messages
  Announce menu and control help radio button checked | 2 of 3 | To change the selection press Up
  or Down Arrow."; then, by Tab: "Access Key Off radio button checked | 1 of 4", "Automatically
  Start JAWS... Button", "Prompt to confirm when exiting JAWS check box checked", "Automatic
  notification of updates check box not checked", "Seamless Updates for Scripts | (Requires Admin
  Privileges) check box checked", "Run JAWS from System Tray check box not checked", "Show Virtual
  Viewer on screen check box checked", "Use Keyboard Layout: Combo box | Laptop | 2 of 3",
  "Windows System Parameters... Button", "OK Button", "Cancel Button", and around again. Escape
  closed the dialog and JAWS's window with it: Notepad came back.
- **Voice Adjustment** (Options, Voices, Voice Adjustment...): "Leaving menus | Voice Adjustment
  dialog | Profile Name: Combo box | SAPI 5X 64 | 5 of 5 | To change the selection use the Arrow
  keys."; then, by Tab: "Delete Profile Button", "Enable Additional Voice Profiles for installed
  Vocalizer Expressive 2 voices check box not checked", "Synthesizer Language: Combo box | Match
  JAWS UI Language (Primary) | 1 of 2", "Speak Sample Button", "Voice Adjust: Combo box | All
  Contexts | 1 of 5", "Voice Person: Combo box | SR Speech Recorder - United States | 2 of 3",
  "Voice | Rate: 20 | left right slider | 100 percent", "Voice Punctuation: Combo box | Most | 3
  of 4", "Voice | Volume: 100 | left right slider | 100 percent", "Voice Spell Rate Change
  Percent: Edit | -2", "Voice Upper Case Pitch Change Percent: Edit | 20", "Save As... Button",
  "Cancel Button". No Pitch with this SAPI voice, and no OK or Apply on a factory profile
  unchanged (the help: OK is unavailable after changing one; Apply appears once something
  changed).
- **Voice Adjustment's contexts** (`jaws-window-walk.ps1 -Parts contexts`): Adjust's five, by
  Down: "All Contexts | 1 of 5", "PC Cursor Voice | 2 of 5", "JAWS Cursor Voice | 3 of 5",
  "Keyboard Voice | 4 of 5", "Tutor and Message Voice | 5 of 5". All Contexts shows Person, Rate,
  Punctuation and Volume; a context shows Person, Rate, Punctuation and "Voice | Pitch: 10 | left
  right slider | 50 percent", and no Volume. On old-pc every context has All Contexts' values.
- **Select a Voice Profile** (Insert+Ctrl+S over Notepad): "JAWS Version 2025.2505.43 | Select a
  Voice Profile dialog | Default: Combo box | SAPI 5X 64 | 5 of 5"; by Tab: "For notepad: Combo
  box | Use Default | 1 of 6", "OK Button", "Cancel Button". JAWS's version before the dialog is
  its accident (as before its other dialogs: accepted divergence `JawsSaysBeforeItsDialog`).
- **Voice Adjustment's profiles** (`-Parts profiles,saveas`): Profile Name by Up: "SAPI 5X -
  (factory) | 4 of 5", "Persian | 3 of 5", "Microsoft Mobile - (factory) | 2 of 5", "Eloquence |
  1 of 5", nothing more at the top. The names are in alphabetical order; a factory profile is named
  "<synthesizer> - (factory)", a saved one by its name (old-pc's own is "SAPI 5X 64"). Save As...
  (after Upper Case Pitch Change Percent): "Save Voice Profile dialog | Voice Profile Name: edit
  combo | SAPI 5X 64 | 5 of 5 | To set the value use the Arrow keys or type the value."; by Tab,
  "OK Button", "Cancel Button"; Escape goes back to "Save As... Button".
- **Manage Application Settings...** (Options, fifth): "Manage Application Settings dialog |
  Application Settings List box | Chrome checked | 1 of 5 | To move to an item press the Arrow
  keys."; by Tab: "Enable All Button", "Disable All Button", "Remove Selected Button", "OK
  Button", "Cancel Button". Escape closes it.
- **Restore to Factory Settings...** (Options, sixth): "Restore to Factory Settings dialog | This
  option will delete all your current settings and restart JAWS. Are you sure you want to
  continue? | Yes Button", Tab "No Button". Escape does nothing there: it was answered No
  (troubleshooting §7 of the skill). Never walk it again.
- **Settings Center's search** (typed into its search box; `settings-questions-walk.ps1` with
  `-Center` keys):
  - Each key typed says the search box, its text, the first result and the count: "SearchBox
    Edit | scr | Screen Echo | 1 of 7 Search Results".
  - Down goes into a list of results, not into a filtered tree: "6 Search Results List box |
    Screen Echo Echo Highlighted Text | User | Use the arrow keys to select an item. | Press the
    space bar to toggle between the choices". Each result is the setting with its value and
    then its group's path ("Smart Navigation Mode Off | NavigationWeb / HTML / PDFs").
  - Escape in the results: "Search edit box cleared.", and the tree again at its first item.
  - The names: "Screen Echo", in User, value "Echo Highlighted Text"; "Smart Navigation Mode",
    in Web / HTML / PDFs > Navigation, value "Off"; "Indicate Headings", in Web / HTML / PDFs >
    Headings and Frames, value "Headings with Level". Space on a result moves its value on and
    says it with its place: "Play Sound | 4 of 5", "Speak Heading with Headings Voice | 5 of 5",
    "No Headings | 1 of 5". The second, "Headings Only", is from the help, not walked. Okur
    has the first three; the sound and the headings voice are not built.
  - Web / HTML / PDFs' subgroups, in the help's order: Reading, Navigation, Images / Graphics,
    Links, Headings and Frames, Tables, Miscellaneous.
  - Okur's search filters the tree to the results in their groups instead. JAWS's results
    list is the shape to follow (an open item).
- **The Startup Wizard** (JAWS's window, Alt+H, Z; `jaws-window-walk.ps1 -Parts
  wizard,wizardpages` and `wizard,wizardvalues`; each radio group moved and put back as
  found, and Escape, which saves nothing):
  - "JAWS Startup Wizard dialog | Help Us Improve Freedom Scientific Products ... | Participate
    check box not checked". The window keeps that title; each page is said by its own title
    before its first control, and the buttons are "< Back", "Next >" or "Finish", "Cancel".
  - Speech Settings: "Rate: 20 | left right slider | 100 percent", "Punctuation: Combo box |
    Most | 3 of 4", "Typing Echo Off radio button checked | 1 of 4". No Voice Assistant on
    old-pc.
  - Run JAWS Settings, after a paragraph on the start options: "Start JAWS at the logon screen
    check box checked", "Start JAWS after logon for all users check box checked", "Start JAWS
    after logon for this user: Combo box | Always | 1 of 3", "Run JAWS from System Tray check
    box not checked", "Prompt to confirm when exiting JAWS check box checked".
  - Common Options: Smart Navigation "Off", "Controls Only", "Tables", "Controls and Tables";
    Forms Mode "Auto", "Semi Auto", "Manual"; "Use Keyboard Layout: Combo box | Laptop | 2 of
    3"; "Use Virtual Ribbon Menus check box"; "Initial state of NumLock key Turn off at
    startup radio button checked | 1 of 3".
  - Verbosity Settings: Tutor Messages "Announce Menu and Control Help" (2 of 3), "Announce
    Custom Messages Only" (3 of 3); Access Key "Off", "Speak all", "Speak Menus Only", "Speak
    Dialogs Only"; Verbosity Level "Beginner, Highest", "Intermediate", "Advanced, Lowest".
  - Then Braille Translation Settings and Braille Settings, the last with "Finish Button".
  - Okur's, since the commit after `1b6b74ac`: "Okur Startup Wizard", with Speech Settings,
    Run Okur Settings (Prompt to confirm when exiting Okur), Common Options and Verbosity
    Settings, each with Okur's settings of it under the wizard's names; a change heard at once
    (the session layer), Finish keeping them for all applications, Cancel nothing. From Help,
    Startup Wizard... (after About Okur..., access key Z), and on the first start for a user
    who has no settings folder yet (never with `--settings`).
- **Last Changed Settings' menus** (Settings Center and Quick Settings over Notepad;
  `settings-questions-walk.ps1`, two walks; Notepad's Typing Echo moved on, applied, moved
  back and applied, so it ended as it began):
  - Empty, End on it: "level 0 | Last Changed Settings | closed | 28 of 28"; Shift+F10:
    "Context Menu | To navigate press Up or Down Arrow. | Remove All Unavailable", one item, in
    both dialogs.
  - With Typing Echo applied (Alt+A): "Last Changed Settings | open | 1 item", then the entry
    "level 1 | Typing Echo (User) Characters | 1 of 1 | Last Changed Settings": the label, the
    group's path in parentheses, then the value. Its Shift+F10: "Context Menu | ... | Remove",
    one item. Space on the entry moves its value as in its own group ("Words | 3 of 4").
  - The entry applied again, the focus went to the group; its Shift+F10 then offered "Remove
    All", and Enter: "Leaving menus | Tree view | Last Changed Settings | open | 1 item", the
    entry gone (Down said nothing). Escape then closed Settings Center at once: Remove All is
    done without Apply or a question.
  - Okur has the same since the commit after `018cc0d1`: the entry's words, Remove and Remove
    All in both dialogs (Shift+F10 on a setting in Settings Center opens nothing, as JAWS's has
    no lifetimes), and the history kept in `last-changed.toml` beside the settings.
- **Settings Center** (Insert+6 over Notepad; `settings-center-walk.ps1`): "NotePad - JAWS
  Settings Center dialog | SearchBox Edit | Type in text."; by Tab: " | Tree view | | User |
  closed | 1 of 28 | To move through or expand items use the Arrow keys.", " read only edit |
  <the group's description> | Use your reading keys to read the text.", "OK Button", "Cancel
  Button", "Application: Combo box | NotePad | 74 of 182 | To change the selection use the Arrow
  keys.", back to the search box. No Apply until something changed. A group is said with its
  state between JAWS's state marks, "Web / HTML / PDFs | ‹closed›". Its 28 groups, in order:
  User, Web / HTML / PDFs, Forms Mode, Text Processing, Speech Verbosity, Speech Options, Speech
  And Sounds Schemes, Say All, Graphics and Symbols, Braille, Braille and Text Viewers, Caret And
  Cursor, Synthesizer, Punctuation, Voice Aliases, Window Classes, Keyboard, Custom Highlight,
  Research It, Text Analyzer, Convenient OCR and Picture Smart, Gestures, Mouse Echo, Visual
  Tracking, Face in View, Math, Miscellaneous, Last Changed Settings. F6 on Last Changed Settings
  says "There is nothing visible at the configuration display pane at the moment." The groups'
  settings: the walk with each group opened (to come).
- **Settings Center's questions** (Insert+6 over Notepad; `settings-questions-walk.ps1`):
  - Space on "Typing Echo Off" says the new value and its place, "Characters | 2 of 4", then
    "Words | 3 of 4"; Shift+Space goes on the same way: "Characters and Words | 4 of 4".
  - Once something changed, Tab from the tree goes: the description, "Apply Button", "OK
    Button", "Cancel Button", the Application box, the search box, the tree.
  - F6 from the tree goes to the setting's control in the pane: "Typing Echo Characters and
    Words radio button checked | 4 of 4 | To change the selection press Up or Down Arrow."
    F6 again returns to the tree item.
  - Escape with a change not applied asks: "JAWS Settings Center dialog | You have made changes
    to NotePad's settings. Do you want to save them? | Yes Button". Tab gives "No Button".
    Escape on the question goes back to Settings Center, at the tree item.
  - Okur has all but F6 since `11497973` and the commit after it. okur-ui's question, probed on
    old-pc with JAWS reading it, says the same with Okur's name: "Okur Settings Center dialog |
    You have made changes to Notepad's settings. Do you want to save them? | Yes Button", then
    "No Button"; Escape goes back to the dialog (`okur-ui-tree-probe.ps1 -Question`).
  - The second walk: the question's buttons are Yes, No, Cancel (Tab after Cancel returns to
    Yes), and Escape is its Cancel. Enter on a setting with nothing changed and no search is
    OK: Settings Center closes. Okur's question has the three buttons, and Enter is OK.
  - Quick Settings over Notepad (Settings Center closed): "QuickSettings - notepad dialog |
    SearchBox Edit"; by Tab: the tree with its groups open ("Touch Cursor Options | open | 4
    items"), the description, "Expand Tree View check box checked | To clear checkmark press
    Spacebar.", "OK Button", and back to the search box. It has no Apply and no Cancel; Escape
    closes it. Shift+F10 on a setting: "Context Menu | To navigate press Up or Down Arrow. |
    Restore setting when JAWS exits", "Restore setting when focus changes", "Save the setting
    checked". Okur's Quick Settings has this Tab order, the check box (clearing it closes every
    group), and these words with Okur's name.
  - Open: JAWS reads its description edit's first line on Tab. okur-ui's note says only "read
    only edit | Use your reading keys to read the text.", and Up or Down read its lines. Keeping
    the dialog manager from selecting the note's text did not change this (probe, 2026-10-24).

## 2. Okur today

- One dialog for Insert+V and Insert+6 (`crates/okur-core/src/ui/settings.rs`): its controls are
  the settings table's (`crates/okur-core/src/settings/registry.rs`, four groups: Speech,
  Keyboard, Reading, Mouse Echo), a "Settings for" box (all applications, the application in
  front, its site), Apply, OK, Cancel.
- No main window, no menus, no Basics, no voice profiles or voice contexts, no tree, no search,
  no lifetimes, no Last Changed Settings.
- Okur's windows are drawn by `okur-ui` from the UI model (`crates/okur-ui-model`), whose
  control kinds have no tree and no menu bar.

## 3. The design

### 3.1 Settings Center and Quick Settings: one generated dialog

Both are made from the settings table; Quick Settings shows the table's `quick` settings for the
application in front, Settings Center every setting, for the place its Application box chooses.

- **Parts, in Tab order**: search edit (focus on open, Ctrl+E from anywhere), tree, the selected
  setting's description (a read-only field Tab reaches), Apply (only once something changed,
  Alt+A), OK, Cancel, then in Settings Center the Application combo box (Default (All
  Applications) first; Ctrl+Shift+D chooses it) [R-SUI-37], [R-SUI-23]. Quick Settings has
  "Expand Tree View" and OK as the walk found.
- **The tree**: groups at level 0, settings at level 1, as JAWS's. Up and Down move, Right and
  Left open and close a group, Home and End, first letters among the visible items. Each setting's
  item is named with its label and value, as the walk heard them: "Typing Echo None",
  "Automatic Activation not checked", "Speak Single Digits Threshold 5 or more digits". Space
  cycles a choice's values (a check box's two), Shift+Space backwards (to record), and a button
  item opens its own dialog (Configure Verbosity Levels).
- **Speech**, as the walk recorded JAWS's (Quick Settings in Notepad):
  - a group: "level 0 | General Options | open | 11 items";
  - a group's first setting, or a setting reached across groups: "level 1 | Typing Echo None |
    1 of 4 | Editing Options";
  - the next settings: "Caps Indicate On Character", the label and value alone;
  - the last group: "level 0 | Last Changed Settings | History | 9 of 9".
  Okur's tree says the same through its own window and the engine's focus reading.
- **Values a tree cannot cycle** (a number, a text): the tree item opens an edit for it with Enter
  or F6, and F6 or Escape returns to the item [R-SUI-23].
- **Search**: typing filters the tree to the settings whose label, description, keywords or value
  labels match, in their groups; Down enters the results; Enter on a result shows the whole tree
  at it; Escape on the results gives the whole tree back at the item before [R-SUI-18],
  [R-SUI-38].
- **Lifetimes (Quick Settings)**: Shift+F10 or the Applications key on a setting or a group opens
  a menu: until Okur exits, until the focus moves, saved (the default; checked) [R-SUI-18]. The
  until-the-focus-moves layer joins the settings layers (customization map §7.3).
- **Last Changed Settings**: the last group, the 25 latest changes for the place, newest first,
  each named with its path ("Typing Echo (Editing Options)"), changeable there; Shift+F10: Remove,
  and on the group Remove All [R-SUI-37], [R-SUI-38].
- **Groups**: JAWS's Settings Center groups, named as JAWS names them, holding Okur's settings as
  the customization map places them (§4 there). Quick Settings in Notepad had, in order: Touch
  Cursor Options, General Options (User Verbosity, Virtual Ribbon Menu, Progress Bars, Top and
  Bottom Edge Alert, Graphics Labeled, Include Graphic Alignment Description, Screen Echo, Focus
  Loss Announcement, Screen Shade, Interrupt Speech When a Dialog Box Is Created, Use Inferred
  Access Keys), Convenient OCR (Primary and Secondary Recognition Language), Reading Options (Text
  Analyzer, Annotation Indication, Smart Word Reading, Language Detect Change, Custom Labels,
  Stopwords Exception Dictionary), Say All Options (Scheme, Say All Reads By, Capitalization),
  Editing Options (Typing Echo, Caps Indicate, Punctuation, Indentation), Spelling Options (Text
  Spell, AlphaNumeric Text), Numbers Options (Speak Single Digits Threshold, Speak Single Digits
  if Number Contains Dashes), Last Changed Settings. A group with none of Okur's settings is not
  shown; a setting Okur does not have is not shown.

### 3.2 The Okur window and its menus (Insert+J)

- Insert+J brings Okur's main window to the front; its title is "Okur" and its first line Okur's
  version, as JAWS's [R-SUI-34], [R-SUI-36]. With "Run Okur from System Tray", Insert+J opens the
  same menus as a context menu [R-SUI-06].
- Menus Options, Utilities, Language, Help, access keys O, U, L, H; Alt alone lands on Options
  and Right walks them [R-SUI-35]. **Only what Okur has is listed**, in JAWS's order and with
  JAWS's access keys: a JAWS user finds each item where the hand expects it, and no item leads
  nowhere.
- Options, in 2025's order (§1.1): Basics..., Voices ▸ (Voice Adjustment..., Select a Voice
  Profile...), Manage Application Settings..., Restore to Factory Settings..., Exit (Alt+F4).
  Braille..., Early Adopter Program... and Remote Desktop... come with those features, in their
  places.
- Utilities, in 2025's order: Notification History (Insert+Space, N; Okur has it), Settings
  Center, Convenient OCR ▸ (Okur's OCR commands, with its layer key), and later Screen Shade,
  Keyboard Manager, Dictionary Manager, Import/Export Settings ▸.
- Language: Okur Language ▸ (once Okur has another), Voice Profiles ▸ (the default, a profile for
  the application in front, No Speech, Create New) [R-SUI-04].
- Help, in 2025's order: Commands Search (Insert+Space, J; Okur has it), Okur Help Topics (F1),
  Keyboard Commands, What's New, About Okur..., Startup Wizard..., and Check for Updates... once
  Okur updates itself.
- Items say their keys after the name, as JAWS's: "Notification History INSERT+SPACE, N".

### 3.3 Basics

"Basic Settings": of the controls JAWS's has (§1.1), those Okur has, in its Tab order, focus on
Tutor Messages: Tutor Messages (radio buttons), Access Key (radio buttons), Automatically Start
Okur... (button, a dialog of its own), Prompt to confirm when exiting Okur (check box; Okur's
quit confirmation already exists), Run Okur from System Tray, Show Virtual Viewer on screen, Use
Keyboard Layout: (combo box: Desktop, Laptop), OK, Cancel; their labels as JAWS writes them, with
Okur's name. Enter is OK and Escape Cancel. The JAWS key and Num Lock are not here: they
are Settings Center's (Keyboard, General) [R-SUI-06], [R-SUI-14].

### 3.4 Voices

- **Voice contexts** become settings: `voice.contexts.<context>` (PC cursor, Okur cursor,
  keyboard, tutor and messages) inheriting from All Context, each with the voice, rate,
  punctuation, pitch and volume (customization map §7.4). Typing echo then speaks in the keyboard
  voice and messages in theirs, as JAWS does.
- **Voice profiles**: a profile is a synthesizer, its language and the contexts' settings, saved
  by name; factory profiles per synthesizer ("(factory)", changed only through Save As). A
  profile is chosen globally and per application.
- **Voice Adjustment** is generated like the settings dialog but laid out as JAWS's (§1 row), the
  Adjust combo box choosing the context whose controls follow; Spell Rate Change and Upper Case
  Pitch Change only with All Context; Apply asks whether to make the profile the default
  [R-SUI-07], [R-SUI-08].
- **Select a Voice Profile** (Insert+Ctrl+S): Default and "For <application>" combo boxes,
  "Use Default" first; first letter and Enter choose and close [R-SUI-08].
- **Add/Remove Voices** is JAWS's installer for Vocalizer voices; Okur's voices are the system's
  (SAPI 5, OneCore, later macOS's), so the item opens Windows' own speech settings or is left out.

### 3.5 The UI model

New control kinds in `crates/okur-ui-model`, drawn natively by `okur-ui` (a Win32 tree view and
menu bar on Windows, NSOutlineView and NSMenu on macOS):
- `Tree { items: Vec<TreeItem> }`: id, level, label, expanded, value text, check state;
- a window menu bar: `Menu { label, access_key, items }` with submenus and separators;
- a read-only multi-line field in the Tab order, for the description.
The engine reads Okur's windows through the platform's accessibility as any window, so their
speech is what the tree view and menus give, worded as JAWS's (§3.1's speech is the test).

## 4. Settings Okur has to add for these surfaces

Tutor Messages, Access Keys, Prompt to Confirm When Exiting, Run from System Tray, Show Virtual
Viewer on Screen, Automatically Start, the voice contexts and profiles, the lifetimes and the
until-the-focus-moves layer, Last Changed Settings' history. The rest of the customization map's
top ten (§8 there) then fill the tree, one table row each.

## 5. To record on old-pc first (research §8.8)

- Done (§1.1): the menus' 2025 order, Basics' Tab order and labels. Still: whether the main
  window starts minimized, its title and first line, and the menus' access keys (the walk heard
  none: old-pc's Access Key setting is Off).
- Done (§1.1): Voice Adjustment's Tab order and labels. Still: what changes after a change (OK,
  Apply), and Pitch with a voice that has it.
- Done (§1.1): Shift+Space, the Tab order once changed, F6, Escape's question and its buttons,
  Enter on a setting, Quick Settings' Tab order and its lifetime words. Still: Settings Center's
  tree in Default, each group opened; what Quick Settings' Escape does after a change.
- Done (§1.1): Voice Adjustment's contexts and Select a Voice Profile's Tab order.
These are research walks of JAWS's own windows (JAWS's dialogs refuse injected keys: walks use
`TypeKey` through FSAPI or `okur-lab keys` with the process allowed), then scenarios where a
spoken form is a parity target.

## 6. Milestones

1. **The tree dialog.** `Tree` in the UI model and `okur-ui`; Settings Center and Quick Settings
   generated from the table with JAWS's groups, search, Space cycling, descriptions, Apply/OK/
   Cancel, the Application box. Replaces today's dialog. Tests on the dialog model: key in,
   focus, item names and values out; the speech of §3.1 against a recording.
   **Built 2026-10-24** (`ui/settings_tree.rs`; Insert+6 and Insert+V since `ef67221c`): JAWS's
   groups and subgroups and Quick Settings' own, check boxes, Space and Shift+Space, the
   description, Apply once changed, the Application box, Ctrl+E, Ctrl+Shift+D, Down from the
   search box, Enter and Escape on a search's results, "Unavailable" for a setting kept for
   all applications only, settings no dialog showed under JAWS's tree names, and Configure
   Verbosity Levels opening the verbosity levels' dialog. okur-ui's tree, keys and menu probed
   on old-pc (`okur-ui-tree-probe.ps1`). Still to do: a scenario comparing Okur's speech of its
   tree with JAWS's (Okur run on old-pc reads its own window); F6 to the setting's control;
   the search's results as JAWS's list box. Screen Echo, Smart Navigation Mode and Indicate
   Headings are in the tree since `50dde786`, so every setting with values has its names.
2. **Lifetimes and Last Changed Settings** in Quick Settings; the until-the-focus-moves layer.
   **Built 2026-10-24**: the lifetime menu (Shift+F10, Applications key) on a setting or a
   group, the session's and the until-the-focus-moves layers; Last Changed Settings in both
   dialogs; JAWS's words for the three lifetimes; Remove and Remove All in Last Changed
   Settings, and its history kept across restarts (`last-changed.toml`). Still to do: the
   lifetimes' icons beside items.
3. **The Okur window and its menus** (Insert+J), Basics with its new settings, the quit
   confirmation from Basics' check box.
   **Built 2026-10-24** (`4194cb88`): Insert+J shows the window, "Okur" and its version, with
   Options (Basics..., Exit Alt+F4), Utilities (Notification History INSERT+SPACE, N; Settings
   Center), Help (About Okur...); Alt+F4 asks to exit. Probed on old-pc with okur-ui alone:
   JAWS reads it as its own ("Menu bar | Options | To navigate press Left or Right Arrow.",
   "Options menu | Basics... | To move through items press up or down arrow.", "leaving menu
   bar"). Basics: Tutor Messages, Access Key, Prompt to confirm when exiting Okur, Show Virtual
   Viewer on screen, Use Keyboard Layout, OK, Cancel. Still to do: Automatically Start Okur...
   and Run Okur from System Tray (with setup's start entries), Tutor Messages' "Announce Custom
   Messages Only" and Access Key's "Speak Menus Only" and "Speak Dialogs Only", the window when
   Okur starts (minimized or not, to record), the Language menu with voice profiles (milestone
   4), Commands Search in Help once Okur has it.
4. **Voices**: voice contexts, profiles, Voice Adjustment, Select a Voice Profile, the Language
   menu's profiles.
   **Voice contexts built 2026-10-24**:
   - `voice.contexts` holds the PC Cursor, Okur Cursor, Keyboard and Tutor and Message voices
     (person, rate, punctuation), each part it leaves out All Contexts';
   - the speech thread speaks typing echo in the keyboard's voice, Okur's own messages
     (`say_message`: "Okur Cursor", "Faster", "left mouse button", "Not in a dialog box"...) in
     the message voice, and all else in the cursor's, the Okur cursor's while it is active;
   - Voice Adjustment's Adjust box offers the five as JAWS's, a context showing its own values
     and no Volume, and a change in All Contexts is every context's.
   **Select a Voice Profile built 2026-10-24**: Insert+Ctrl+S (Default.JKM's
   SelectAVoiceProfile) and Options, Voices, Select a Voice Profile...: Default, "For <application>"
   with Use Default first, OK, Cancel. A profile is a synthesizer's own ("Automatic -
   (factory)", "OneCore - (factory)", "SAPI 5 - (factory)") or one saved by name
   (`voice_profiles`, for all applications), all in alphabetical order; choosing one copies its
   voice into the place's settings with its name (`voice.profile`); Use Default leaves the
   application nothing of its own.
   **A context's Pitch** (Voice Adjustment shows it in Volume's place for a context): the
   synthesizers say everything at the context's pitch, a capital's raise on top; a Say All's
   parts are not pitched yet.
   **Voice Adjustment's profiles built 2026-10-24**: Profile Name first (choosing one shows its
   voice), Delete Profile for a saved one, Save As... asking "Voice Profile Name:" in Save Voice
   Profile, and OK once something changed: not for a synthesizer's own profile changed, which is
   kept only by Save As. Probed on old-pc with okur-ui alone (`okur-ui-tree-probe.ps1 -Voice`):
   "Profile Name: Combo box | Automatic - (factory) | 1 of 3", "Voice Adjust: Combo box | All
   Contexts | 1 of 5", "Voice Person: Combo box", Delete Profile and OK skipped by Tab while
   unavailable, and the profiles replaced after Save As ("Fast | 2 of 4").
   A slider's label now shows its number, "Rate: 0", as JAWS's "Rate: 20" (in Okur's own
   units, -10 to 10; JAWS's SAPI voice shows 0 to 20, its scale per synthesizer).
   Still to do: tutor hints in the message voice (they are part of the focus speech today);
   Speak Sample and Synthesizer
   Language; Apply's question whether to make a saved profile the default; the Language menu's
   Voice Profiles.
5. **Startup Wizard**, Manage Application Settings, Restore to Factory Settings.
   **Manage Application Settings and Restore to Factory Settings built 2026-10-24**, both in
   Options:
   - Manage Application Settings lists the applications with settings of their own, checked
     while they are used (`disabled_applications`, for all applications, makes resolving skip
     an application's layer), with Enable All, Disable All, Remove Selected (forgets its
     settings), OK, Cancel;
   - Restore to Factory Settings asks first, in JAWS's words but for "and restart JAWS", since
     Okur's defaults apply at once; Yes forgets every saved setting and Last Changed Settings.
   **The Startup Wizard built 2026-10-25** (§1.1's walk). Still to do: its start options
   with setup's start entries (Run Okur Settings), Run Okur from System Tray, Tutor Messages'
   and Access Key's other choices; whether JAWS reads okur-ui's check list as JAWS's "List
   box" (a probe).
