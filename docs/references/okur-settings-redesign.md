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
- Options: Basics... (B), Voices ▸ (V: Voice Adjustment..., Select a Voice Profile...), Manage
  Application Settings (M), Restore to Factory Settings, Exit (X, Alt+F4). Braille, Remote
  Desktop and Early Adopter Program come with those features.
- Utilities: Settings Center, Convenient OCR ▸ (Okur's OCR commands), and later Keyboard Manager,
  Dictionary Manager, Import/Export Settings ▸.
- Language: Okur Language ▸ (once Okur has another), Voice Profiles ▸ (the default, a profile for
  the application in front, No Speech, Create New) [R-SUI-04].
- Help: Commands Search (Okur has it), Okur Help Topics, Keyboard Commands, What's New, About
  Okur, Startup Wizard (Z).

### 3.3 Basics

The controls of [R-SUI-06] that Okur has, in JAWS's Tab order, focus on Tutor Messages: Tutor
Messages (radio group), Access Keys (radio group), Automatically Start Okur (button, a dialog of
its own), Prompt to Confirm When Exiting Okur (check box; Okur's quit confirmation already
exists), Run Okur from System Tray, Show Virtual Viewer on Screen, Use Keyboard Layout (Desktop,
Laptop), OK, Cancel. Enter is OK and Escape Cancel. The JAWS key and Num Lock are not here: they
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

- The main window: whether it starts minimized; its title and first line; the 2025 order and
  access keys of Options and Help (Remote Desktop, Early Adopter Program, Restore to Factory
  Settings). Probe ready: `jaws-window-walk.ps1` (session scratch; to join the skill).
- Basics' and Voice Adjustment's Tab order and spoken labels in 2025.
- Settings Center's tree in Default and in Notepad: groups and settings; Shift+Space; Enter on a
  setting; F6; the Application box; Quick Settings' whole Tab order (Apply? Cancel?).
- Select a Voice Profile's speech.
These are research walks of JAWS's own windows (JAWS's dialogs refuse injected keys: walks use
`TypeKey` through FSAPI or `okur-lab keys` with the process allowed), then scenarios where a
spoken form is a parity target.

## 6. Milestones

1. **The tree dialog.** `Tree` in the UI model and `okur-ui`; Settings Center and Quick Settings
   generated from the table with JAWS's groups, search, Space cycling, descriptions, Apply/OK/
   Cancel, the Application box. Replaces today's dialog. Tests on the dialog model: key in,
   focus, item names and values out; the speech of §3.1 against a recording.
2. **Lifetimes and Last Changed Settings** in Quick Settings; the until-the-focus-moves layer.
3. **The Okur window and its menus** (Insert+J), Basics with its new settings, the quit
   confirmation from Basics' check box.
4. **Voices**: voice contexts, profiles, Voice Adjustment, Select a Voice Profile, the Language
   menu's profiles.
5. **Startup Wizard**, Manage Application Settings, Restore to Factory Settings.
