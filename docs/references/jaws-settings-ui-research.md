# JAWS 2025's settings user interface: how a user moves through it

Date: 2026-10-24. Research only: no code changed, nothing recorded on old-pc, JAWS not touched on
any machine. Sources are JAWS's public help (the `.chm` files JAWS 2025 installs, dated 2025-05-07
on the dev box), Freedom Scientific's web pages, training books and lesson transcripts, and one
third-party tip sheet (labelled). Freedom Scientific's prose is summarized; only short UI labels
and short spoken announcements are given as they are. Every fact carries a source id from the
table at the end.

Companion documents, not repeated here: [`jaws-customization-map.md`](jaws-customization-map.md)
(what settings exist and their scopes, §2), [`jaws-reading-settings-research.md`](jaws-reading-settings-research.md)
(the reading settings' values), [`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md)
(Okur's own black-box recordings of Run JAWS Manager, Quick Settings and Settings Center on old-pc).

Okur names itself where JAWS does (AGENTS.md §1.5): "JAWS window" becomes Okur's main window,
"Prompt to Confirm When Exiting JAWS" becomes "... Exiting Okur", "Start JAWS ..." becomes
"Start Okur ...". The labels below are JAWS's, for reference.

## 0. Question and scope

The decision this settles: **what Okur's settings surfaces must look like and how the keyboard
moves through them**, so that a JAWS user's muscle memory works: the main window and its menus,
the Basics dialog, Voice Adjustment and voice profiles, Settings Center and Quick Settings, and
the Startup Wizard. Out of scope: which settings exist (the map), braille settings beyond their
place in the UI, Keyboard Manager and Dictionary Manager internals.

## 1. Contradictions with the brief or the project's current assumptions (read first)

1. **Basics has no JAWS-key choice and no Num Lock choice.** The 2025 Basics dialog has a
   keyboard-layout combo box (Desktop, Laptop, Kinesis); the layout decides the JAWS key (Insert
   on Desktop, Caps Lock on Laptop). Choosing a different JAWS key for the current layout is a
   Settings Center item (Keyboard, General, "JAWS Key for Current Layout", Default.jcf only). Num
   Lock's initial state is in Settings Center (Keyboard, General) and the Startup Wizard's Common
   Options page, not in Basics. **fact** [R-SUI-06], [R-SUI-14], [R-SUI-24], [R-SUI-12]
2. **No "show JAWS window / show in tray" pair and no Advanced button in Basics.** There is one
   check box, "Run JAWS from System Tray" (cleared by default; takes effect at the next start).
   The only "Advanced" button is in the *Braille* basics dialog, where it opens Settings Center
   on the Braille group. **fact** [R-SUI-06], [R-SUI-11], [R-SUI-12]
3. **Voice Adjustment has no "Save Profile", "Preview" or "Apply to all contexts" control.**
   Its buttons are Delete Profile, Set Language as Primary, Speak Sample, OK, Save As, Apply and
   Cancel; the context is chosen in an "Adjust" combo box whose first item, "All Context", is the
   way to change every context at once. **fact** [R-SUI-07], [R-SUI-36]
4. **The Voices submenu's third item is "Add/Remove Voices" and handles only Vocalizer Expressive
   2 voices.** Other synthesizers (Eloquence, SAPI 5 and so on) are added or removed with the
   Synthesizer and Braille Manager (Utilities menu). **fact** [R-SUI-02], [R-SUI-09], [R-SUI-21]
5. **The 2025 Utilities menu lists no Speech and Sounds Manager, no Explore My Settings and no
   Skim Reading item.** Speech and sound schemes are a Settings Center group (with a Modify
   Schemes item); Explore My Settings is a shortcut inside the folder that "Explore Utilities
   Folder" opens; the Skim Reading Tool is in the Run JAWS Manager list (Insert+F2).
   **fact** [R-SUI-03], [R-SUI-17], [R-SUI-25], [R-SUI-40], [R-SUI-15]
6. **Settings Center from the main window's Utilities menu opens the Default (all applications)
   settings**, not the application in front; Insert+6 and Insert+F2 open it on the application
   that had focus. **fact** [R-SUI-03], [R-SUI-23], [R-SUI-37]
7. **The help's Options Menu Overview is incomplete for 2025.** JAWS 2025's own help and What's
   New tell the user to choose "Early Adopter Program" in the Options menu, an item the overview
   does not list. Its position is unknown. **fact** [R-SUI-22], [R-SUI-43]; position **open
   question**
8. **Enter in the Settings Center tree is described two ways.** The Settings Center help says
   Space or Enter changes a radio button, check box or button item in the tree; Surf's Up uses
   Enter on an ordinary tree item to save and close Settings Center (the OK button), and Enter on
   a search result to jump to it in the full tree. **contradiction**; record on old-pc.
   [R-SUI-23], [R-SUI-38]
9. **Freedom Scientific's own training contains errors to not copy:** Basic Training book 4 gives
   Insert+Alt+S for switching voice profiles (the keystroke list gives Insert+Ctrl+S for that and
   Insert+Alt+S for Select Scheme) and mentions a File menu in the JAWS window (there is none).
   **fact** [R-SUI-36] vs [R-SUI-42], [R-SUI-01]
10. **A third-party tip sheet says Quick Settings has an Application combo box.** Freedom
    Scientific's help says Quick Settings cannot switch to the default file, and Okur's own
    recording of Quick Settings shows no Application combo box. Treat the tip sheet as wrong.
    [R-SUI-48] vs [R-SUI-18], [R-SUI-49]

The customization map (§2.1) agrees with everything here; this document only adds the UI
detail. One nuance for the map's Startup Wizard row: JAWS 2025's wizard also has Enable Voice
Assistant, Use Virtual Ribbon Menu, Run JAWS from System Tray and Prompt to Confirm When Exiting
JAWS, and no longer the audio-ducking check box that JAWS 2020's had [R-SUI-12], [R-SUI-35].

## 2. The main window ("JAWS window")

### 2.1 Window, tray, and Insert+J

- **fact**: JAWS runs either in a normal window or from the system tray; the same options are
  in a shortcut menu when it runs from the tray [R-SUI-01].
- **fact**: Insert+J is "JAWS Window" in the keystroke list [R-SUI-42]. It brings the window to
  the front even when it is minimized or behind another application [R-SUI-36]. Lessons speak
  the window's title on arrival: "JAWS Professional" (2021 lessons [R-SUI-27], [R-SUI-32]);
  "JAWS" in the 2020 book [R-SUI-35].
- **fact**: with "Run JAWS from System Tray" checked, JAWS leaves the Alt+Tab order and the
  taskbar; Insert+J then opens a context menu with Options, Utilities, Languages and Help;
  Right Arrow opens their submenus, Alt closes the menu [R-SUI-06], [R-SUI-12]. A 2020 lesson
  hears "JAWS Context Menu, Options sub menu, O." on Insert+J in tray mode [R-SUI-33].
- **fact**: the window shows the version just below the menu bar (Say Line reads it, e.g.
  "JAWS version 2020 ILM") and the serial number on its bottom line [R-SUI-34], [R-SUI-36].
- **fact**: by default JAWS is in the Alt+Tab order ("ALT+TAB. JAWS. ... Desktop.") [R-SUI-34].
- **open question**: whether the window is shown or minimized when JAWS starts; no source says.
- **fact**: Help topics open with F1 in the window [R-SUI-35].

### 2.2 The menu bar and Alt

- **fact**: four menus, Options, Utilities, Language, Help [R-SUI-01]; access keys O, U, L, H
  [R-SUI-35].
- **fact**: Alt alone lands on Options: "Menu bar, Options, To navigate press LEFT or RIGHT
  ARROW, O." Right Arrow walks Utilities, Language, Help, then the window's System menu, then
  wraps to Options. Down Arrow opens the menu: "Menu, Options menu, Basics, ... B." Menus wrap
  top to bottom. Items that open dialogs end in an ellipsis, spoken "dot dot dot" (JAWS 2020)
  [R-SUI-35], [R-SUI-36].

### 2.3 Options menu (Alt+O)

Order from the JAWS 2025 help [R-SUI-02]; access keys from JAWS 2020's book [R-SUI-35],
[R-SUI-36]; later additions from What's New.

| # | Item | Opens | Access key | Source |
|---|---|---|---|---|
| 1 | Basics... | Basic Settings dialog (§3) | B | [R-SUI-02], [R-SUI-35] |
| 2 | Voices ▸ | submenu: Voice Adjustment..., Select a Voice Profile..., Add/Remove Voices... (§4) | V (Add/Remove Voices: A) | [R-SUI-02], [R-SUI-36] |
| 3 | Braille | Braille Basic Settings: default display combo, Modify Settings, Add Braille Display, Advanced (opens Settings Center's Braille group), Show Braille Load Error, Run JAWS Without Speech | L | [R-SUI-11], [R-SUI-35] |
| 4 | Manage Application Settings | list of applications with changed settings, a check box each (checked = used); Enable All, Disable All, Remove Selected, OK, Cancel | M | [R-SUI-10], [R-SUI-35] |
| 5 | Restore to Factory Settings | a Yes/No confirmation; Yes deletes every custom setting and JAWS restarts as newly installed (since 2022) | ? | [R-SUI-02], [R-SUI-45] |
| 6 | Remote Desktop | remote-access troubleshooting dialog: authorization state, channel status, Repair, Test connection (since 2020) | ? | [R-SUI-02], [R-SUI-22], [R-SUI-46] |
| ? | Early Adopter Program | a dialog of check boxes, one per preview feature, each with Send Feedback; OK, restart needed (2025) | ? | [R-SUI-22], [R-SUI-43] |
| last | Exit | the quit confirmation (§2.7); hot key Alt+F4 shown | X | [R-SUI-02], [R-SUI-31], [R-SUI-35] |

- **fact**: the Voices submenu opens with Voice Adjustment first: Alt+O, V, Enter opens Voice
  Adjustment [R-SUI-08], [R-SUI-26]; Alt+O, V, A opens Add/Remove Voices [R-SUI-36].

### 2.4 Utilities menu (Alt+U)

Order from the JAWS 2025 help [R-SUI-03]. A 2021 lesson confirms the first item and the tail
(arrowing up from the top wraps to Braille and Text Viewer, Picture Smart, Convenient OCR,
Explore Utilities Folder) [R-SUI-32].

| # | Item | What it opens | Source |
|---|---|---|---|
| 1 | Voice Assistant ▸ | voice commands; its Settings item turns it off or changes the wake word | [R-SUI-03], [R-SUI-32] |
| 2 | JAWS Tandem ▸ | Tandem Center ▸ (Allow Access to my Computer...), Tandem Direct ▸, Disconnect, Quick Start Guide | [R-SUI-03], [R-SUI-36] |
| 3 | Notification History | the last 24 hours of notifications | [R-SUI-03] |
| 4 | Synthesizer and Braille Manager | tree of installed drivers; Add, Remove, or Change Devices wizard | [R-SUI-03], [R-SUI-21] |
| 5 | Sound Cards ▸ | "Windows default sound card" then each device, current one checked (access key O) | [R-SUI-03], [R-SUI-21] |
| 6 | Settings Center | Settings Center on Default.jcf | [R-SUI-03] |
| 7 | Keyboard Manager | | [R-SUI-03] |
| 8 | Dictionary Manager | | [R-SUI-03] |
| 9 | Frame Viewer | | [R-SUI-03] |
| 10 | Script Manager | | [R-SUI-03] |
| 11 | Navigation Quick Key Manager | | [R-SUI-03] |
| 12 | Re-Initialize Authorization | | [R-SUI-03] |
| 13 | Import/Export Settings ▸ | Export, Import, Restore, Migrate wizards (`.sbak` file) | [R-SUI-03], [R-SUI-20] |
| 14 | Explore Utilities Folder | a File Explorer folder of shortcuts: Help, Run HJPad, Explore Error Reports, Explore My Settings, Explore Scripts, Explore Shared Settings, Explore the Manuals, license viewers, Synthesizer and Braille Manager, Export/Import/Restore Settings (access key X) | [R-SUI-03], [R-SUI-17], [R-SUI-32], [R-SUI-39] |
| 15 | OCR Options ▸ (spoken "Convenient OCR INSERT+SPACE,O layer sub menu" in 2021) | | [R-SUI-03], [R-SUI-32] |
| 16 | Picture Smart ▸ | | [R-SUI-03], [R-SUI-32] |
| 17 | Braille and Text Viewer ▸ | | [R-SUI-03], [R-SUI-32] |

### 2.5 Language menu (Alt+L)

- **fact**: JAWS Language ▸ (interface language; only when extra languages were installed;
  default OS Language) and Voice Profiles ▸: pick the default profile, a profile for the current
  application, "No Speech" (silent until a profile is chosen or JAWS restarts), and "Create New"
  (opens Voice Adjustment) [R-SUI-04], [R-SUI-36]. A profile change lasts across restarts
  [R-SUI-41].

### 2.6 Help menu (Alt+H)

- **fact** (2025 order [R-SUI-05]): Commands Search, Message Center, Training, JAWS Help Topics,
  Keyboard Commands, What's New, Technical Support, Web Resources ▸, About JAWS for Windows,
  Startup Wizard, FS Support Tool, License ▸, Check for Updates.
- **fact** (2020 speech with access keys [R-SUI-35]): Commands Search "INSERT+SPACE, J", C;
  Training T; JAWS help topics "F1", J; Keyboard commands K; What's new W; End-user license
  agreement E; Web resources submenu R; Technical support S; About JAWS A; Startup Wizard Z.
  Alt+H then Z opens the Startup Wizard (also [R-SUI-13], [R-SUI-27]).

### 2.7 Quitting

- **fact**: Insert+F4 anywhere, Alt+F4 in the window, or Options, Exit (Alt+O, X). With "Prompt
  to Confirm When Exiting JAWS" checked (default) a dialog asks; Enter quits, Esc returns
  [R-SUI-02], [R-SUI-31], [R-SUI-34]. Speech: Insert+F4 says "Unloading JAWS" before the dialog;
  the dialog is spoken as "Quit JAWS dialog", its question, the Enter/Escape hint, then its OK
  button [R-SUI-34]. On start JAWS says "JAWS for Windows" [R-SUI-34].

### 2.8 Run JAWS Manager (Insert+F2) and the other ways in

- **fact**: Insert+F2 lists the managers and utilities; changes made in a manager opened from an
  application apply to that application; Ctrl+Shift+D in several managers switches to the
  default level [R-SUI-15]. Okur's recording lists 21 entries alphabetically, Settings Center
  between Script Manager and Skim Reading Tool [R-SUI-49]. **inference**: S twice reaches
  Settings Center by first-letter navigation.
- **fact**: direct keys: Settings Center Insert+6 (number row), Quick Settings Insert+V, Switch
  Voice Profiles Insert+Ctrl+S, Select Scheme Insert+Alt+S, Keyboard Manager Insert+8, Dictionary
  Manager Insert+D [R-SUI-16], [R-SUI-42].
- **fact**: Freedom Scientific's own summary of the ways to change settings: the main window
  (Insert+J), Quick Settings (Insert+V), Settings Center and the other managers (Insert+F2), and
  the Startup Wizard; in any manager, Insert+T reads the title to tell which application's file
  is open [R-SUI-41].

## 3. The Basics dialog (Options, Basics...)

- **fact**: title and first speech: "Basic settings dialog, Tutor messages" [R-SUI-35].

Controls in the order of the 2025 help [R-SUI-06], with the tab order a 2020 lesson walks
[R-SUI-36] (Keyboard Layout is about seven Tabs from the start [R-SUI-34]):

| Tab stop (2020) | Control | Type | Values; default | Source |
|---|---|---|---|---|
| 1 (focus on open) | Tutor Messages | radio group | Turn Off Menu and Control Help; Announce Menu and Control Help (default, per the wizard); Announce Custom Messages Only | [R-SUI-06], [R-SUI-12] |
| 2 | Access Keys | radio group | Off; Speak All (default); Speak Menus Only; Speak Dialogs Only | [R-SUI-06], [R-SUI-12], [R-SUI-41] |
| 3 | Automatically Start JAWS | button → Start JAWS Options dialog | see below | [R-SUI-06], [R-SUI-30] |
| 4 | Prompt to Confirm When Exiting JAWS | check box | checked | [R-SUI-06] |
| 5 | Automatic Notification of Updates | check box | checked | [R-SUI-06] |
| ? (2024+) | Seamless Updates for Scripts | check box | checked; admin rights to change | [R-SUI-06], [R-SUI-44] |
| 6 | Run JAWS from System Tray | check box | cleared; effective at next start | [R-SUI-06], [R-SUI-12] |
| 7 | Show Virtual Viewer on Screen | check box | checked; restart needed | [R-SUI-06], [R-SUI-36] |
| 8 | Use Keyboard Layout | combo box | Desktop (default), Laptop, Kinesis | [R-SUI-06], [R-SUI-34], [R-SUI-35] |
| 9? | Windows System Parameters | button → check boxes of Windows features turned off while JAWS runs (all checked recommended) | | [R-SUI-06] |
| then | OK, Cancel | buttons | Enter = OK, Esc = Cancel | [R-SUI-36], [R-SUI-41] |

- **Start JAWS Options dialog** [R-SUI-06], [R-SUI-30]: an administrator sees three controls,
  a standard user only the third: "Automatically start JAWS at the Log On screen" (check box,
  checked; all users; admin only), "Automatically start JAWS after log on for all users" (check
  box, cleared; admin only), "Start JAWS after log on for this user" (combo box: Always (default),
  Never, Using the All Users Setting; overrides the all-users box). OK closes it.
- **open question**: the 2025 tab position of Seamless Updates for Scripts and of Windows System
  Parameters (the 2020 lesson skipped what follows the layout), and whether a
  standard user hears it at all (it needs admin rights). **inference**: it follows Automatic
  Notification of Updates, as in the help.

## 4. Voices: Voice Adjustment, profiles, Add/Remove Voices

### 4.1 Voice Adjustment dialog (Options, Voices, Voice Adjustment...)

Controls in help order [R-SUI-07]; the 2020 lesson's Tab walk and spoken labels in brackets
[R-SUI-36]:

| Control | Type | Notes | Source |
|---|---|---|---|
| Profile Name (focus on open) | combo box | factory profiles per installed synthesizer, "(factory)" suffix; Eloquence, Microsoft Mobile and SAPI 5 always present; one profile per installed Vocalizer Expressive voice; arrowing plays each voice | [R-SUI-07], [R-SUI-36], [R-SUI-47] |
| Delete Profile | button | not for factory profiles; deleting a changed factory profile restores it | [R-SUI-07], [R-SUI-08] |
| Enable Additional Voice Profiles for Installed Vocalizer Expressive 2 Voices | check box | checked | [R-SUI-07], [R-SUI-36] |
| Synthesizer Language | combo box | first item "Match JAWS UI Language" (primary); one set of voice settings per language in a profile; dialects share settings | [R-SUI-07], [R-SUI-36] |
| Set Language as Primary | button | unavailable when the primary is selected; warns if the language differs from the interface's | [R-SUI-07] |
| Speak Sample | button | | [R-SUI-07] |
| Adjust ["Voice Adjust: Combo box, All Contexts"] | combo box | All Context, PC Cursor Voice, JAWS Cursor Voice, Keyboard Voice, Tutor and Message Voice | [R-SUI-07], [R-SUI-36] |
| Person | combo box | voices of the synthesizer, all dialects listed | [R-SUI-07] |
| Rate ["Voice Rate: 68, left right slider, 25 percent"] | slider | arrows by 1 %, Page Up/Down by 10 %, Home/End | [R-SUI-07], [R-SUI-35], [R-SUI-36], [R-SUI-41] |
| Punctuation ["Voice Punctuation: Combo box, most"] | combo box | None, Some, Most (default), All | [R-SUI-07], [R-SUI-36] |
| Pitch | slider | not every synthesizer | [R-SUI-07] |
| Volume | slider | Vocalizer, SAPI 5, some hardware; Eloquence uses the system volume | [R-SUI-07] |
| Spell Rate Change Percent | edit | only with All Context; 2020 default −20 | [R-SUI-07], [R-SUI-36] |
| Upper Case Pitch Change Percent | edit | only with All Context; 2020 default 20 | [R-SUI-07], [R-SUI-36] |
| OK, Save As, Apply, Cancel | buttons | OK unavailable after changing a factory profile (use Save As); Apply saves without closing | [R-SUI-07] |

- **fact**: Apply asks whether to make the changes the default voice profile (Yes/No); Save As
  asks for a name, then whether to make it the default [R-SUI-08].
- **fact**: Ctrl+Win+Alt+Page Up/Page Down change the rate and save it to the active profile;
  Alt+Ctrl+Page Up/Page Down change it until the focus leaves the application [R-SUI-41].
- **open question**: the 2025 Tab order of Person, Pitch and Volume (the 2020 lesson skipped
  them) and the label text the 2025 help writes "Upper Case Pitch Case Percent".

### 4.2 Select a Voice Profile (Insert+Ctrl+S, or Options, Voices)

- **fact**: two combo boxes: "Default" (global; focus on open) and "For <application>"; the
  latter's first item "Use Default" follows the Default box. First letter then Enter picks a
  profile and closes the dialog [R-SUI-08], [R-SUI-09], [R-SUI-26], [R-SUI-36].
- **fact**: Language, Voice Profiles ▸ offers the same choice from the menus [R-SUI-04],
  [R-SUI-41].

### 4.3 Add/Remove Voices (Options, Voices, Add/Remove Voices...)

- **fact**: the "Add/Remove Vocalizer Expressive Voices" utility: Select a Language combo box
  (focus on open), Premium Voices list (Space selects several; installed ones marked; Alt+P plays
  a sample), Install Selected Voices (Alt+I; UAC prompt, then Finish), and Additional options:
  Remove a Voice, Remove All Voices, Remove All Compact Voices; Close. Internet required. Each
  installed voice gets its own profile [R-SUI-09], [R-SUI-36]. Voices are shared with ZoomText
  and Fusion [R-SUI-09].

## 5. Settings Center and Quick Settings

Both have the same parts: a search edit box, a tree view, a configuration display pane (the
same settings as ordinary controls, for sighted helpers and the mouse) and a read-only help
text [R-SUI-18], [R-SUI-23].

### 5.1 Opening and titles

- **fact**: Settings Center opens on the application that had focus, title "JAWS Settings
  Center - <application>"; Ctrl+Shift+D switches to "JAWS Settings Center - Default (All
  Applications)", also the first item of the Application combo box [R-SUI-37], [R-SUI-41].
  From the Utilities menu it opens on Default.jcf [R-SUI-03].
- **fact**: Quick Settings (Insert+V) covers the application (and document) in front: its own
  groups first, the general ones after; title "Quick Settings - <program>"; it cannot switch to
  the default file [R-SUI-18], [R-SUI-37], [R-SUI-41].
- **fact**: focus starts in the search edit box in both [R-SUI-18], [R-SUI-23], [R-SUI-37].
  Okur's recording of the speech: "QuickSettings - Chrome dialog | SearchBox Edit | Type in
  text. | CTRL+E" and "Chrome - JAWS Settings Center dialog | SearchBox Edit | Type in text. |
  control+E" [R-SUI-49].

### 5.2 Tab order (Settings Center)

- **fact**: search edit → tree view → read-only help text → Apply (only present once something
  changed) → OK → Cancel → Application combo box → search edit [R-SUI-37]. Shift+Tab twice from
  the tree reaches the Application combo box [R-SUI-23].
- **fact** (Quick Settings): search → tree → help text → OK; Okur's recording adds an "Expand
  Tree View" check box [R-SUI-37], [R-SUI-41], [R-SUI-49]. **open question**: the full Quick
  Settings Tab order and whether it has Apply and Cancel.

### 5.3 Search

- **fact**: Ctrl+E moves to the search box from anywhere in the dialog; typing filters the tree
  to the matches; Down Arrow moves into the filtered results; Enter on a result restores the
  full tree with focus on that item; Esc on the results restores the full tree at the item
  focused before the search [R-SUI-18], [R-SUI-23], [R-SUI-38].
- **fact**: Down Arrow from the search box also enters the tree without a search [R-SUI-39].
- **fact**: search words used in Freedom Scientific's own steps: "typing", "punctuation",
  "verbosity", "screen", "dollars", "web verbosity", "scheme", "braille", "marks", "clock",
  "message" [R-SUI-18], [R-SUI-23], [R-SUI-38], [R-SUI-40], [R-SUI-41], [R-SUI-43].

### 5.4 The tree

- **fact**: Up/Down move; Right/Left expand/collapse a group; first-letter navigation (only
  among visible items); Page Up/Page Down; Home/End (End reaches the last group) [R-SUI-23],
  [R-SUI-37], [R-SUI-38].
- **fact**: a closed group is spoken with its state, e.g. "Last Changed Settings, closed"
  [R-SUI-38]; Surf's Up describes a closed "Web / HTML / PDFs" group [R-SUI-37].
- **fact**: Space cycles an item's values (combo-box-like and radio-like items), toggles a check
  box, or activates a button item; button items open their own dialogs (Configure Web Verbosity
  Levels, Medium; Modify Schemes) [R-SUI-23], [R-SUI-37], [R-SUI-38], [R-SUI-40].
- **fact**: edit and spin items cannot be typed in the tree: F6 moves to the matching control
  in the configuration pane (e.g. Select Speech and Sounds Scheme combo box), F6 again returns
  to the same node. In the pane, Alt+access key jumps to a control (Alt+A = Auto in the Forms
  Mode group) and Tab moves within the pane [R-SUI-23], [R-SUI-40].
- **fact**: Tab from a tree item reaches its help text, read with Say Line or Say All
  [R-SUI-23], [R-SUI-37]; Insert+F1 on an option also explains it [R-SUI-37].
- **fact**: Alt+A activates Apply and keeps the dialog open [R-SUI-38]; Esc closes without
  saving [R-SUI-41].
- **open question** (docs silent): how a tree item and its value are spoken (for example
  whether "Typing Echo" is followed by its value, a control type, a position), whether
  Shift+Space cycles backwards, and whether Space wraps from the last value to the first. Record
  on old-pc.

### 5.5 Quick Settings lifetimes

- **fact**: the Applications key or Shift+F10 on an item opens a menu of three lifetimes:
  restore the setting when JAWS exits, restore it when the focus changes, save it (checked by
  default). On a group the menu has the same three for every item in it. The current lifetime is
  a check mark in the menu and an icon beside the item (JAWS icon, double window, disk). Set the
  lifetime before changing the value. Document settings have no such menu and are always saved
  [R-SUI-18], [R-SUI-37].

### 5.6 Last Changed Settings

- **fact**: the last group of the tree in both dialogs; the last 25 changes for the application
  or document, the oldest dropped first. Each entry is named with its path, e.g. "Medium...
  (Web / HTML / PDFs ,Reading,Configure Web Verbosity Levels)", and can be changed there. Shift+F10
  on an entry offers Remove; on the group, Remove All [R-SUI-37], [R-SUI-38].

## 6. The Startup Wizard

- **fact**: runs after a new installation's activation unless settings were migrated; later from
  Help, Startup Wizard (Alt+H, Z). Finish on the last page saves; Cancel or Esc anywhere saves
  nothing [R-SUI-05], [R-SUI-12], [R-SUI-27].
- **fact**: pages have Back ("Less, back button", Alt+B; absent on page 1), Next ("Next greater
  button", Alt+N), Cancel; the last has Finish instead of Next. Tab cycles each page; Insert+F1
  explains a control [R-SUI-29], [R-SUI-35]. First speech: "JAWS Startup Wizard dialog", the
  page title and question, then "Participate check box" [R-SUI-27], [R-SUI-28].

| Page | Controls (2025 help) | Source |
|---|---|---|
| 1 Help Us Improve Freedom Scientific Products | Participate (check box, Alt+P); Learn More button | [R-SUI-12], [R-SUI-28], [R-SUI-35] |
| 2 Speech Settings | Rate slider; Punctuation combo (Most); Typing Echo radios (Off, Characters (default), Words, Characters and Words); Enable Voice Assistant check box (2020: "Lower the volume of other programs while JAWS is speaking" instead) | [R-SUI-12], [R-SUI-35] |
| 3 Run JAWS Settings | the three Start JAWS options (§3); Run JAWS from System Tray; Prompt to Confirm When Exiting JAWS | [R-SUI-12], [R-SUI-35] |
| 4 Common Options | Smart Navigation (Off default; spoken as a radio group in 2020-21); Forms Mode radios (Auto default, Manual, Semi-auto); Use Virtual Ribbon Menu; Use Keyboard Layout; Initial State of NUM LOCK Key (Turn off at startup default, Turn on at startup, Do not modify) | [R-SUI-12], [R-SUI-28], [R-SUI-35] |
| 5 Verbosity Settings | Tutor Messages; Access Keys; Verbosity Level (Beginner default, Intermediate, Advanced); 2020 also Graphic verbosity | [R-SUI-12], [R-SUI-35] |
| 6 Braille Translation Settings | Language, Output, Input combo boxes; Show Current Word in Computer Braille | [R-SUI-12], [R-SUI-35] |
| 7 Braille Settings | Braille Mode (Structured default); Enable Flash Messages; Enable Word Wrap; Reverse Panning Buttons; status cells placement; dot firmness | [R-SUI-12], [R-SUI-35] |

- **fact**: changes take effect as soon as made inside the wizard (tutor messages stopped at
  once in the 2020 lesson) and are saved by Finish [R-SUI-35].
- **open question**: the rate slider's scale. The help gives a default of 57; lessons heard
  "Rate 68 ... 25 percent" (2020) and "Rate, colon, 100 ... 14%" (2021).

## 7. Domain terms

| JAWS term | Meaning |
|---|---|
| JAWS window / JAWS application window | JAWS's main window with the menu bar (Okur: its main window) |
| Run JAWS Manager | the Insert+F2 list of managers and utilities |
| Settings Center | every setting, for one application or Default (All Applications) |
| Quick Settings | the application or document in front, with lifetimes |
| Persistence level | Quick Settings' lifetime of a change: until JAWS exits, until focus changes, saved |
| Configuration display pane | the controls beside the tree, reached with F6 |
| Last Changed Settings | the tree's last group: the 25 latest changes |
| Voice profile | synthesizer, language(s) and voice settings, chosen globally and per application |
| Voice context ("Adjust") | All Context, PC Cursor, JAWS Cursor, Keyboard, Tutor and Message voices |
| Factory profile | a synthesizer's built-in profile, "(factory)"; changed only through Save As |
| Default.jcf | the all-applications settings file; some settings exist only there |
| Startup Wizard | seven pages of the most common settings |

## 8. Implications for Okur's requirements and tests

Each is testable on Okur's own UI model and, where noted, needs a recording on old-pc first
(owner's rule: document, record, then build).

1. **Main window.** Insert+J (Okur key+J) activates Okur's main window, or opens a context menu
   with the same four submenus when Okur runs from the tray. Menu bar Options, Utilities,
   Language, Help with access keys O, U, L, H; Alt alone lands on Options. Test: the UI model's
   menu tree, order and access keys equal §2.3 to §2.6 with Okur's name substituted.
2. **Options menu** at least: Basics..., Voices ▸ (Voice Adjustment..., Select a Voice
   Profile..., Add/Remove Voices...), Braille, Manage Application Settings, Restore to Factory
   Settings, Exit (X, Alt+F4). Remote Desktop and Early Adopter Program only if Okur gets those
   features. Record the 2025 order and access keys first.
3. **Basics dialog**: the controls and defaults of §3, focus on Tutor Messages, Tab order as
   listed, Enter = OK, Esc = Cancel; Okur names ("Prompt to Confirm When Exiting Okur", "Run
   Okur from System Tray", "Automatically Start Okur"). No JAWS-key or Num Lock control here;
   those go to Settings Center's Keyboard group (default scope only).
4. **Quit confirmation**: Okur key+F4 speaks the unloading message, then a dialog whose Enter
   quits and Esc cancels; skipped when the Basics check box is cleared.
5. **Voice Adjustment**: the Adjust combo box with the five contexts; Spell Rate and Upper Case
   Pitch disabled unless All Context; OK disabled after changing a factory profile; Apply asks
   about making it the default. **Select a Voice Profile** (Okur key+Ctrl+S): Default and
   For <app> combo boxes, "Use Default" first, Enter closes. Tests on the dialog model.
6. **Settings Center and Quick Settings, one generated dialog** (the map's §7 design):
   - opens with focus in the search box; title shows the scope; Ctrl+Shift+D switches to
     Default (Settings Center only);
   - Tab order of §5.2, Apply present only when dirty;
   - Ctrl+E, filtering, Down Arrow into results, Enter on a result restores the full tree at it,
     Esc on results restores the previous item;
   - tree keys of §5.4; Space cycles values or opens a sub-dialog; F6 to the matching pane
     control and back; Alt+A applies;
   - Quick Settings' lifetime menu (Applications, Shift+F10) on items and groups, with a check
     mark on the current lifetime; none on document settings;
   - Last Changed Settings as the last group, capped at 25, entries named with their path,
     Remove and Remove All.
   Tests can be hermetic on the dialog model (key in, focus and value out); the spoken form of
   tree items needs old-pc evidence first (§5.4 open question).
7. **Startup Wizard**: seven pages as in §6, Back absent on page 1, Finish only on the last,
   Cancel or Esc discards everything; reachable from Help (access key Z).
8. **Record on old-pc before building** (open questions): Options and Help menu order and
   access keys in 2025; Basics Tab order including Seamless Updates; Voice Adjustment Tab order;
   how tree items, their values and groups are spoken; Shift+Space; Enter in the tree; Quick
   Settings' full Tab order; whether the main window starts minimized.

## Sources

Accessed 2026-10-24. "Help" sources are JAWS 2025's own help as installed (files dated
2025-05-07) under `C:\ProgramData\Freedom Scientific\JAWS\2025\Locale\enu\Help\`, read by
decompiling the `.chm` files (`hh.exe -decompile`); the URL column gives the `mk:@MSITStore`
address of each topic, which opens it in Windows' help viewer. Freedom Scientific does not
publish these topics as web pages that this research could find. Lesson pages on
support.freedomscientific.com were fetched with curl because the fetch tool got HTTP 403.

| ID | URL | Kind, strength | What it supports |
|---|---|---|---|
| R-SUI-01 | `mk:@MSITStore:JFW.chm::/using_jaws/the_jaws_application_window/the_jaws_application_window.htm` | official help 2025, strong | window or tray; four menus; tray shortcut menu |
| R-SUI-02 | `…JFW.chm::/using_jaws/the_jaws_application_window/options_menu_overview.htm` | official help 2025, strong (incomplete: no Early Adopter Program) | Options items and what each opens |
| R-SUI-03 | `…JFW.chm::/using_jaws/the_jaws_application_window/utilities_menu_overview.htm` | official help 2025, strong | Utilities items in order; Settings Center opens Default.jcf |
| R-SUI-04 | `…JFW.chm::/using_jaws/the_jaws_application_window/language_menu_overview.htm` | official help 2025, strong | JAWS Language, Voice Profiles, No Speech, Create New |
| R-SUI-05 | `…JFW.chm::/using_jaws/the_jaws_application_window/help_menu_overview.htm` | official help 2025, strong | Help items in order; Startup Wizard via Alt+H |
| R-SUI-06 | `…JFW.chm::/using_jaws/the_jaws_application_window/basics_dialog.htm` | official help 2025, strong | every Basics control, defaults, Start JAWS Options dialog, tray behavior |
| R-SUI-07 | `…JFW.chm::/using_jaws/the_jaws_application_window/Voice_Adjustment.htm` | official help 2025, strong | Voice Adjustment controls, contexts, buttons |
| R-SUI-08 | `…JFW.chm::/using_jaws/controlling_speech/application_specific_voice_settings.htm` | official help 2025, strong | creating, deleting, restoring profiles; Apply/Save As prompts; Select a Voice Profile combos |
| R-SUI-09 | `…JFW.chm::/using_jaws/the_jaws_application_window/Add_Remove_Voices.htm`, `…/installing_jaws/Adding_More_Voices.htm` | official help 2025, strong | Add/Remove Vocalizer Expressive Voices utility |
| R-SUI-10 | `…JFW.chm::/using_jaws/the_jaws_application_window/Application_Settings.htm` | official help 2025, strong | Manage Application Settings controls |
| R-SUI-11 | `…JFW.chm::/using_jaws/the_jaws_application_window/braille_basics_dialog.htm` | official help 2025, strong | Braille basics controls, Advanced button |
| R-SUI-12 | `…JFW.chm::/using_jaws/installing_jaws/Startup_Wizard.htm` | official help 2025, strong | wizard pages, controls, defaults; Finish/Cancel |
| R-SUI-13 | `…JFW.chm::/using_jaws/starting_jaws_for_windows.htm` | official help 2025, strong | Alt+H, Z; wizard's Run JAWS Settings page |
| R-SUI-14 | `…JFW.chm::/keystrokes/jaws_key.htm` | official help 2025, strong | JAWS key per layout; changed in Settings Center Keyboard group |
| R-SUI-15 | `…JFW.chm::/using_jaws/overview_of_the_jaws_utilities.htm` | official help 2025, strong | Run JAWS Manager list; application vs default level; Ctrl+Shift+D |
| R-SUI-16 | `…JFW.chm::/keystrokes/jaws_keystrokes/keystrokes_for_accessing_the_jaws_utilities.htm` | official help 2025, strong | Insert+6, Insert+F2, Insert+8, Insert+D… |
| R-SUI-17 | `…JFW.chm::/using_jaws/The_JAWS_Program_Group.htm` | official help 2025, strong | Utilities folder contents (Explore My Settings etc.) |
| R-SUI-18 | `…JFW.chm::/customizing_jaws_for_windows/Quick_Settings/Quick_Settings.htm` | official help 2025, strong | Quick Settings parts, search, lifetimes, no default switch |
| R-SUI-19 | `…JFW.chm::/using_jaws/the_jaws_application_window/jaws_dialogs/adjust_jaws_verbosity_dialog.htm` | official help 2025, strong | Settings Center vs Quick Settings roles (background) |
| R-SUI-20 | `…JFW.chm::/customizing_jaws_for_windows/Settings_Import_and_Export/Import_and_Export_Settings.htm` | official help 2025, strong | Import/Export submenu: Export, Import, Restore, Migrate |
| R-SUI-21 | `…JFW.chm::/customizing_jaws_for_windows/Sound_Cards/Sound_Cards.htm`, `…/Synth_Braille_Manager/Synthesizer_and_Braille_Manager.htm` | official help 2025, strong | Sound Cards submenu (Alt+U, O); synthesizer installation |
| R-SUI-22 | `…JFW.chm::/contacting_freedom_scientific/technical_support/Early_Adopter_Program.htm`, `…/Remote_Desktop.htm` | official help 2025, strong | both are Options menu items; their dialogs |
| R-SUI-23 | `mk:@MSITStore:SettingsCenter.chm::/Overview.htm` | official help 2025, strong | Settings Center parts, keys, search, F6, Tab, Space/Enter |
| R-SUI-24 | `…SettingsCenter.chm::/Keyboard/keyboard_options_dialog.htm` | official help 2025, strong | JAWS Key for Current Layout, Num Lock, Default.jcf only |
| R-SUI-25 | `…SettingsCenter.chm::/User/user_options_dialog.htm`, `…/SpeechAndSoundsSchemes/speech_manager_dialog.htm` | official help 2025, strong | schemes are a Settings Center group |
| R-SUI-26 | `mk:@MSITStore:tutorial.chm::/new_users_start_here/voice_settings.htm` | official help 2025, strong | Alt+O, V, Enter opens Voice Adjustment; Select a Voice Profile combos |
| R-SUI-27 | https://support.freedomscientific.com/teachers/lessons/2.3_StartupWizard-transcript-Launching.htm | official lesson transcript (c. 2021), medium | Insert+J speech; Help menu speech; Z; wizard's first speech |
| R-SUI-28 | https://support.freedomscientific.com/teachers/lessons/2.3_StartupWizard-transcript-Navigating.htm | official lesson transcript (c. 2021), medium | the seven pages as spoken; Alt+N |
| R-SUI-29 | https://support.freedomscientific.com/teachers/lessons/2.3.2_StartupWizard-Navigating.htm | official lesson, medium | wizard control types, Insert+F1, Finish |
| R-SUI-30 | https://support.freedomscientific.com/teachers/lessons/2.2.1_StartJAWS-Video.htm | official lesson (2021), medium | Basics, Automatically Start JAWS button, Start JAWS Options |
| R-SUI-31 | https://support.freedomscientific.com/teachers/lessons/2.2.2_StartJAWS-QuittingJAWS.htm | official lesson, medium | Alt+O, X; Insert+F4; confirmation |
| R-SUI-32 | https://support.freedomscientific.com/teachers/lessons/5.6.1_Transcript-UtilitiesFolder.htm | official lesson transcript (JAWS 2021), medium | Utilities menu first item and tail as spoken; Utilities folder |
| R-SUI-33 | https://support.freedomscientific.com/teachers/lessons/4.2.2_Transcript-JAWSCursorMouseCommands.htm | official lesson transcript (JAWS 2020), medium | tray mode: Insert+J opens "JAWS Context Menu" |
| R-SUI-34 | https://doccenter.freedomscientific.com/doccenter/jaws-basic-training/html/02-GettingStartedWithJAWS-html.zip | official training book 2 (2019, JAWS 2020), medium for 2025 | quit dialog speech; window title; seven Tabs to keyboard layout; Alt+Tab order |
| R-SUI-35 | https://doccenter.freedomscientific.com/doccenter/jaws-basic-training/html/03-MenusDialogBoxesAndTheJAWSStartupWizard-html.zip | official training book 3 (2019, JAWS 2020), medium for 2025 | menu bar and menu speech, access keys, every wizard page as spoken |
| R-SUI-36 | https://doccenter.freedomscientific.com/doccenter/jaws-basic-training/html/04-UserInterfaceAndUtilities-html.zip | official training book 4 (2019, JAWS 2020), medium; contains two errors (§1.9) | Basics Tab walk, Voices submenu, Voice Adjustment Tab walk, Add/Remove Voices, Language and Help menus |
| R-SUI-37 | https://support.freedomscientific.com/SurfsUp/6-TemporaryVersusPermanent.htm | official tutorial (current), strong | Settings Center Tab order, titles, Last Changed Settings, Quick Settings lifetime menus |
| R-SUI-38 | https://support.freedomscientific.com/SurfsUp/7-WebVerbosity.htm | official tutorial (current), strong | Enter on results, Alt+A Apply, End, "closed", Last Changed entry naming, Enter to close |
| R-SUI-39 | https://support.freedomscientific.com/SurfsUp/17-PersonalizedSettings.htm | official tutorial, strong | Down Arrow from search into the tree; P for Personalize Web Settings; Alt+U, X |
| R-SUI-40 | https://support.freedomscientific.com/SurfsUp/18-SpeechSoundsSchemes.htm | official tutorial, strong | F6 to the matching pane control and back; Space on Modify Schemes |
| R-SUI-41 | https://doccenter.freedomscientific.com/doccenter/doccenter/rs11f929e9c511/2015-02-10_teacherstrainers-l3/02_ChangingSettingsInJAWS.htm | official training (2015), medium (old) | ways to change settings; titles with Ctrl+Shift+D; Basics radios; rate keys; profile switching |
| R-SUI-42 | https://support.freedomscientific.com/content/html/jawshq/JAWS-Keystrokes.html | official keystroke list, strong | Insert+J, Insert+V, Insert+6, Insert+Ctrl+S, Insert+F2, Insert+F4, Insert+Alt+S |
| R-SUI-43 | https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2025 | official release notes, strong | Early Adopter Program in the Options menu; Settings Center search steps |
| R-SUI-44 | https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2024 | official release notes, strong | Seamless Updates for Scripts in Basics, admin only |
| R-SUI-45 | https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2022 | official release notes, strong | Restore to factory settings in Options |
| R-SUI-46 | https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2020 | official release notes, strong | Remote Desktop dialog in Options |
| R-SUI-47 | https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2018 | official release notes, strong | Microsoft Mobile profile in Voice Adjustment and Select a Voice Profile (Ctrl+Insert+S) |
| R-SUI-48 | https://atresources.wcbvi.k12.wi.us/accessibility-tips/adjusting-jaws-settings | third-party tip sheet, weak | Space cycles values; claims an Application combo in Quick Settings (contradicted) |
| R-SUI-49 | [`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md) | Okur's black-box recording on old-pc (JAWS 2025), strong for what it shows | opening speech of Quick Settings and Settings Center; Run JAWS Manager list; Expand Tree View check box |
