# Handoff: key labels, Manage Key Labels, Linux page actions (2026-11-06)

After [`2026-11-05-okur-on-linux-and-number-processing.md`](2026-11-05-okur-on-linux-and-number-processing.md).
Plans: the active plan's §3 ([`../plans/2026-11-03-review-round-2-before-cross-platform.md`](../plans/2026-11-03-review-round-2-before-cross-platform.md),
JAWS's features from Settings Center and the help) and
[`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md).

## 2026-11-06: monitor cells finished, Office's wizard, Linux actions

- **Move to Monitor Cell, live** (`9854db90`): Okur's live run of 1135 on old-pc, step by step
  to `Okur live: 7/7 steps match JAWS exactly`. The list is an unnamed list box, as JAWS's;
  UI Automation's stale focus on Excel's cell no longer names Okur's dialog over the cell.
- **The lab's Office** (`6a86918c`): old-pc's Office is not activated, and its Activation Wizard
  now opens over a fresh Excel or Word some seconds after the window and takes every key of a
  trial. run-scenario.ps1 closes it as it comes (UI Automation, WM_CLOSE). Two AutoRecover
  copies of an unsaved Book1 were moved, not deleted, to
  `C:\Users\ailabs\sr-harness\excel-recovered-backup`. The Linux lab's fluxbox runs with no key
  bindings: its Ctrl+Alt+arrows grabbed 109's table keys before Okur.
- **Round 1166 recorded and built** (`e0f80eb1`, `1267b623`, `5c86e99e`): with none set,
  "monitor cell 2 undefined" and "No monitor cells defined for Hours"; the same cell again, "The
  current cell is already defined as a monitor cell." (a statement, not the help's question); a
  range, "Setting monitor cell 2 to B2:B3", read "12 | 10 | B2:B3"; an empty cell read as its
  address; titles before the value, "Weekdays | 12 | B2"; Ctrl+End's "Last Cell" before the
  column's title. The first recording failed every trial (its Escapes had no dialog to leave);
  1166b was recorded without them. `okur-lab parity --only 1166`: `JAWS parity (Ia2): 21/21
  steps match exactly`; full baseline `matched_steps: 12048` of 12619 (corpus/parity-baseline.json).
- **Linux: Space and Enter on a page** (`d702d819`): a page's nodes are numbered depth first as
  their unique ids, the event source keeps the AT-SPI objects in the same order, and
  `LinuxActions` runs a node's first action (Chrome's "click") or gives it the focus. The
  focused control's checked, selected and expanded changes reach the engine as Windows' state
  changes. `check-linux.sh` in the lab: every test passed, clippy clean.
- **Linux live, all steps** (`67c4f2c9`): the lab's runs of 20, 29 and 109 at `d702d819`;
  `okur-lab live-baseline linux`: `Okur live on linux: 379/379 steps match JAWS exactly; 0
  accepted divergences not counted (28 runs, 0 could not be compared)`.

## Key labels (backlog rows 27 and 42)

- **JAWS's list read** (`c782d338`, research): `key-labels-walk.ps1` read Manage Key Labels
  (Settings Center, Keyboard, General; F6 to its button, Space; Enter in the tree is OK) row by
  row: 103 keys, Tab and F18 to F24 muted, Control, Shift, Alt (Menu) and Windows "Always
  Speak"; Toggle Mute, Change Label (an edit over the row's label, "Key Label Edit | + | Type in
  text."), OK, Cancel. `key-labels-store-probe.ps1` found the store: the application's JCF,
  `[KeyLabels] Space=0|Space` (0 muted, 1 spoken); Letters as `Letters=0|Letters`. The probes left
  `Space=1|Space` and `Letters=1|Letters` in old-pc's NotePad.JCF, both the defaults.
- **Built** (`f0ca32a0`): the table is `locales/en-US/key-labels.txt`, the user's changes the
  setting `key_labels`, read from JAWS's `[KeyLabels]`. The echo says a key's label or nothing
  while it is muted; Tab once spoken is named, Shift+Tab "Shift Tab"; Letters, Numbers and
  Symbols muted silence their characters. Defaults change nothing.
- **Rounds 1167 and 1168 recorded and imported** (`df098c1a`): `okur-lab parity --write-baseline`:
  `JAWS parity (Ia2): 12068/12641 steps match exactly; 388 accepted divergences not counted` (was
  12048/12619; the 22 new steps, 20 matching, nothing else moved). Left in 1167: s8, a select's
  "collapsed" said after Tab in forms mode where JAWS says none (the quick key C says it,
  229/s20); s9, JAWS's "selected Izmir a" as Shift+Tab returns to a field whose text it selects.
- **Okur's Manage Key Labels** (`fc254a00`): Settings Center's Keyboard, General ends with
  "Manage Key Labels...", opened by Space or by its button in the pane (F6, Space); Configure
  Verbosity Levels goes the same way now. The dialog: the 103 keys in three columns, the first
  selected; Toggle Mute says the row ("Tab Tab Speak"); F2 or Change Label edits the label in an
  unnamed edit over the cell (okur-ui's new `edit_column`, `ToUi::DialogEditCell`); OK gives
  Settings Center's draft the changes, which its own OK keeps. The list is "Keys" where JAWS's is
  "List1", and JAWS's property page has no counterpart.
- **Round 1169 recorded and imported** (`5d0c5697`): JAWS's dialog from Insert+6 to Escape, three
  trials (key_ui_access now reaches SettingsCenter; its first recording sent nothing, its second
  left the dialog open by a scenario mistake). `okur-lab parity --write-baseline`: `JAWS parity
  (Ia2): 12085/12683 steps match exactly; 388 accepted divergences not counted` (17 of its 42
  match). Replay reads JAWS's own Settings Center as any program's windows; what it shows of
  Okur's general reading: a list row whose column repeats its name loses it ("Zoom Speak" for
  "Zoom Zoom Speak"), the focus back from an in-place edit is not said (s32), and an edit's focus
  said twice (s35).
- **Live on old-pc** (`d9723bef`, `44444b93`): Okur's tree differs from JAWS's (Keyboard, General
  holds 3 items, the tree 13 groups to JAWS's 28), so `run-scenario.ps1 -Reader okur` of 1169
  itself went astray at s7 (`okur-lab live-parity`: `Okur live: 0/42 steps match JAWS exactly`).
  A research scenario on Okur's own path to the dialog, then 1169's s19 to s40, found and fixed:
  a list row's column equal to its name dropped (all list views; replay `Ia2 12085 -> 12090`,
  only 1169 moved), pane pages without buttons (F6 said nothing), F2's arm in the wrong
  notification block, the cell's edit named after the dialog, the F2 hint. The last run's steps
  12 to 33 against 1169's s19 to s40: 19 of 22 identical (normalized history); the other 3 differ
  only by the list's name, "Keys" against JAWS's "List1" (an accepted divergence now). Left:
  Okur's dialog opens naming Settings Center's first; Settings Center's own live speech ("0
  Search Results" at its opening, Right's "open" without the item's name and count, "Closed" as
  text) and its tree's size against JAWS's.

## Open from this stretch

- Okur's Settings Center opens with "SearchBox | 0 Search Results" before "SearchBox Edit", where
  JAWS's says the edit alone (1169/s1). "SearchBox" is the edit's label read as the dialog's
  prompt (a label that gives no access key, the rule of 688/s9); JAWS's search box has no label
  window at all. "0 Search Results" is the hidden results list's label: UI Automation keeps it in
  the control view, not offscreen, while the dialog's own read of visible windows leaves it out.
  Marking a sibling offscreen by its window's visibility (its runtime id `[42, hwnd]`) changed
  nothing live, so it was reverted; a probe of Okur's open Settings Center (window styles, UIA's
  view of that static) comes first. `1a3d96d1` logs the dialog's read.
- Settings Center's other live differences: Right says "open" without the item's name and its
  count ("Keyboard open 2 items" in JAWS's, 1169/s4), "Closed" as text where JAWS marks a state,
  and the tree's size against JAWS's (13 groups to 28; Keyboard, General 3 items to 10).
- Manage Key Labels opens naming Settings Center's dialog first; JAWS's names its own alone.
- 1167/s8, s9: a select's "collapsed" after Tab in forms mode; JAWS's "selected Izmir a" as
  Shift+Tab returns to a field.
- Chords beside Shift+Tab are not named by the key echo (none recorded): Ctrl, Alt chords.

## Later: Settings Center's opening, and the Web Verbosity Level

- **"0 Search Results" gone** (`04b6852e`): `okur-dialog-probe.ps1` (research: Okur's dialog as
  Windows holds it) showed UI Automation's "0 Search Results" to be the visible, empty label above
  the description, named after the hidden results label before it. okur-ui makes no empty labels
  now. Live: "Notepad - Okur Settings Center dialog | SearchBox | SearchBox Edit | Type in text."
  "SearchBox" stays (the prompt rule); a hidden label would still name the edit and leave the
  control view, but marking one touches every `Control` literal (47) or `Edit` (24): later.
- **Right's "open"** is JAWS's own rule for a Win32 tree (" ‹open› ‹›", 748/s10); JAWS's Settings
  Center tree is WinForms, read "Keyboard open 2 items" (303/s7's rule). Not a reading bug.
- **Web Verbosity Level researched** (`2db7afa3`): `web-verbosity-walk.ps1` read the three levels'
  "Announce Start and End For:" trees (`jaws-reading-settings-research.md` §14, 27 items each).
  Over Chrome old-pc reads at **High** (`Chrome.JCF [VirtualCursorVerbosity]
  VirtualCursorVerbosityLevel=2`): the web corpus was recorded at High, where JAWS's default is
  Medium; Okur announces High's set today, for everyone.

## Web Verbosity Level, built

- **Recorded** (`e959ca0f`): 1170 and 1171, 101's article read with Down at Chrome.JCF levels 1
  and 0: 1 is Medium (no banner, complementary or content information), 0 is Low (only the table;
  list items keep their bullets, without "list of" and "list end").
- **Built** (`09a31f8a`): `web_verbosity` (High, Medium, Low), Medium by default as JAWS's, per
  application from its JCF; a reading mark on the page; landmarks, block quotes, articles, group
  boxes and lists the level leaves out are blocks without start and end lines. Replay merges the
  recording's Chrome.JCF over DEFAULT.JCF, and the reference machine is High. **Users now hear
  Medium on the web** (JAWS's default): no banner, complementary, content information, form,
  region or search starts and ends, which Okur said before, fitted to old-pc's High.
- `okur-lab parity --write-baseline`: `JAWS parity (Ia2): 12238/12831 steps match exactly; 388
  accepted divergences not counted` (12090/12683 before, unchanged by the code; 1170 and 1171's
  148 steps all match).
- **Live on old-pc**: `run-scenario.ps1 -Reader okur` of 1170 (Okur's settings from DEFAULT.JCF and
  Chrome.JCF beside it, with the scenario's level 1), `okur-lab live-parity`: `Okur live: 74/74
  steps match JAWS exactly`; `Okur live, spoken: 74/74 steps send the synthesizer what JAWS sent
  its own`.
- **The full tree and its store** (`bbd5da12`, `07fa6ef6`): 17 items with Regions holding 11 (the
  first reading had hidden the lines that began with a level: Clickable Elements, Application,
  Tab Panel). JAWS keeps a change in the application's JCF, `[VirtualCursorVerbosity]
  ApplicationRegion=0|1|0`, High|Medium|Low (its shipped DEFAULT.JCF names every key; its defaults
  are the walk's).
- **Each level's choices** (`45eab1dd`): `web_structures` holds the changed structures, read from
  the files; the page's mark names the structures left out.
- **Okur's Configure Web Verbosity Levels** (`acaae423`, `ae70d6cd`): under Web Verbosity Level,
  High..., Medium..., Low..., each "<Level> Virtual Cursor Verbosity Preferences" with "Announce
  Start and End For:" (17 items, Regions with 11), the description, OK, Cancel. Live on old-pc it
  reads "Block Quotation checked | 1 of 17" and Space "Banner | checked | 3 of 11", as JAWS's.
  The live runs fixed for every dialog: tree check boxes said, a read-only text following its
  item (okur-ui never set a Note: Settings Center's description had stayed the first item's), an
  empty label a space (left out, it let the next field borrow a hidden label's name).
- Not yet: per site levels, and structures Okur's buffer does not mark yet (figure, feed, log,
  note, toolbar, frame, editable section, DescribedBy, clickable elements).

## Round 1172: the rest of the structures

- **Recorded** (`358257ff`): `web-structures.html` (a figure, a feed of two named articles, a log,
  a note, a toolbar, a contenteditable, a fieldset, a named form, a region, a search region) read
  down at High, Medium and Low (1172 to 1174), three trials each, with its UIA and IA2 snapshots.
  JAWS says "Updates feed", "First update article" and "First update article end", "note" and
  "note end", "Formatting toolbar with 2 controls" and "toolbar end", and an editing host as
  "Notes field | Editable Section Start | Editable text | Editable Section End"; at Low a fieldset
  loses its legend with its start and end ("Express check box not checked").
- **Built** (`98f8d397`): those structures in the linearizer, each only where the level announces
  it; both snapshot mappings mark the editing host (`editing_host`, IA2's section role, UIA's
  writable focusable group). The first cut marked every IA2 edit not a text field, spin buttons
  included: 6 regressions in 2718 to 2727, fixed by IA2's section role.
- `okur-lab parity --write-baseline`: `JAWS parity (Ia2): 12388/12981 steps match exactly; 388
  accepted divergences not counted` (12238/12831 before; the round's 150 steps all match).
  `okur-lab invariants`: `Buffer invariants: 0 violation(s)` (the gate first caught "Editable
  text" never spoken in the UIA snapshot: the UIA mapping lacked the mark).
- README figures rerun at `358257ff`; `cargo test --workspace`: 1,727 passed, 0 failed, 201
  ignored.
- The figure and the log need nothing: at every level JAWS says no figure start or end, and reads
  the log as a group ("group start Messages"), as Okur does (1172 to 1174 all match). Still not
  marked: frames, DescribedBy, clickable elements.
- **Per site levels** work already: every setting goes through the site layer (`sites/<host>.toml`,
  Settings Center's "Settings for" a site), and the page is reread when the level in force
  changes. `a_sites_web_verbosity_level_reads_its_pages` pins it.

## The Store's certification: no support ticket possible

Submission 2 (1.1.0.0, public) has been "Update in certification" since 2026-11-03; the Action
Center has no items and says "Your contact email address: not provided" (an account setting, left
for the owner). A support ticket could not be opened (2026-11-06): the account is a personal
Microsoft account, Partner Center's ticket routes go to `login.microsoftonline.com/organizations`
(work accounts) or to Engage Center, and Engage Center answers the account 403 ("Unable to build
navigation using spaceId from sfb", its gateway `gateway.engagehub.azure.com/sp/space`) on old-pc
and on the dev box alike. At the owner's request the question went to Microsoft Q&A instead
(2026-11-06, tag "Microsoft Partner Center | Other"):
<https://learn.microsoft.com/en-us/answers/questions/6063178>. If nothing moves by the end of
2026-11-11, the subagent's plan stands: cancel and submit 1.2.0.0.

## Okur's dialogs: Settings Center's opening and the way back

- **Settings Center opens on its search box alone** (`70000015`): JAWS's own says "NotePad - JAWS
  Settings Center dialog | SearchBox Edit | Type in text." [1169/s1]; Okur's said "SearchBox"
  twice, its shown label read as the dialog's prompt (a label with no access key, JAWS's rule,
  688/s9). `ControlKind::SearchBox`: an edit named by a label that is not shown (the hidden
  static still names it, live). Live on old-pc, `run-scenario.ps1 -Reader okur` of 1169's s1 and
  s42, `okur-lab live-parity`: s1 "Notepad - Okur Settings Center dialog | SearchBox Edit | Type
  in text.", differing only in JAWS's "NotePad" (its settings file's name).
- **The window after Okur's settings**: Escape out of Settings Center said nothing live, where
  JAWS says "sr-notepad-fixture.txt - Notepad | edit | Type in text." [1169/s42; 1154/s33,
  1155/s28, 1156/s43]. JAWS's Settings Center and Dictionary Manager are programs of their own
  (SettingsCenter.exe, JDiction.exe): leaving them is a switch of programs. Its lists and viewers
  are jfw.exe's and return to the focus alone (1036/s3, 1133/s3), which the gate's ratchet
  caught when the first fix applied to every dialog (29 Word and Excel steps). Now
  `returning_from_program` for Settings Center, Quick Settings and the Dictionary Manager:
  `okur-lab live-parity`: `s42 MATCH` ("Okur live: 1/2 steps match JAWS exactly").
- `lab-push.ps1` sends `okur-ui.exe` too: old-pc's had been copied by hand.
- **1169's replay (22/42) is JAWS's Settings Center script**, not a reading rule: its tree
  ("FeatureTree", reached through MSAA alone: 197 flat items whose value is their level) is read
  with values from each item's selected child option ("JAWS Key for Laptop Layout Caps Lock") and
  an "Unavailable Only available when..." text that no accessibility API holds. Not built: Okur's
  users do not use JAWS's Settings Center, and no other recorded application has such a tree.

## Rounds 1175 and 1178: frames, clickable text, descriptions

- **Recorded** (`0d571ff5`): `web-structures-2.html` (a titled iframe, a clickable division, a
  button and a paragraph with aria-describedby) at High, Medium and Low. JAWS: High "Weather frame
  frame | Frame text Weather frame | Weather frame frame end"; Medium "Frame text Weather frame";
  Low "Frame text". Clickable text plain at every level. The described paragraph at High adds
  "Use JawsKey+Alt+R to read descriptive text", the accepted divergence for a key that does
  nothing (`7e639a32`).
- **Built** (`b75ea863`): an iframe is Okur's frame role in both mappings (IA2's iframe tag,
  UIA's document holding its page's document), which nothing produced before: the buffer's frames
  and Insert+F9 had none to find. Its ends at High. The frame's name after its text at High and
  Medium is a known gap (`a_frames_name_follows_its_text_at_high_and_medium`).
- `okur-lab parity --write-baseline`: `JAWS parity (Ia2): 12476/13071 steps match exactly; 388
  accepted divergences not counted` (12388/12981 before the round).
- **Round 1178 recording** (`web-structures-3.html`, in `corpus/pending/`): a titled frame of two
  lines, a frame without a title and a division with a title attribute, at the three levels, to
  tell where the name goes and what Low turns off.
- A round's page waits in `corpus/pending/` with its scenarios (`2ac0d6c7`,
  verification-loop.md): in `corpus/fixtures/` it made every commit before the import fail.

## Round 1178: a frame's name is its title; Excel's titles kept (1181)

- **The title after the first line** (`460864a4`): round 1178 (a titled frame of two lines, an
  untitled frame, a division with a title attribute) shows the trailing name is the element's
  title: "First frame line News frame" then "Second frame line", "Titled text Tooltip title",
  at High and Medium; at Low not; an untitled frame "frame" and "frame end" at High. The IA2
  mapping marks the title (a description from the tooltip, a frame's name from it), a fifth
  reading mark turns titles off at Low. The invariant that no word glues two elements counts an
  element's title as its own words (`f0c1efeb`).
- **1179 recorded again**: its first recording's trial 1 trace lost the page's focus to the
  batch's console (a foreground event back to Chrome with no focus event): an open item in
  `docs/backlog/open-items.md`, "A window back in front with no focus event".
- **Excel's titles kept** (`4a31073e`): per workbook name (`spreadsheet::workbook`, the window
  title before the application's name), in `excel-titles.toml` (a `TitleStore` written by the
  settings folder's background store). Round 1181 (titles set, Excel closed with Alt+F4 and N,
  the workbook opened again through Windows+R): JAWS reads "Weekdays | 12 | B 2" and
  CapsLock+Alt+Shift+C says "Weekdays" [1181/s10-s12]; replay 9/12, the rest the desktop's
  "Not Selected", the Run dialog's tail and Excel's "Please wait | ... | Hours" on opening.
- Imported (`1fac9b4a`): `okur-lab parity`: `JAWS parity (Ia2): 12547/13143 steps match
  exactly; 388 accepted divergences not counted` (12478/13071 before). README figures rerun at
  `4d6d7222`.
- **Queued on old-pc**: round 1182 (Excel's window as the focus comes back to it, on each sheet)
  and round 1183 (Excel's notes: Alt+Shift+Apostrophe, Ctrl+Shift+Apostrophe, a new fixture
  `desktop-excel-comments.ps1` in `corpus/pending/`).

## Excel's notes (round 1183)

- **Recorded** (`5c139fb0`): a new fixture, `desktop-excel-comments.ps1` (the workbook with a note
  on B2). JAWS moves onto the cell without a word of it ("12 | B 2"), Alt+Shift+Apostrophe says
  "Includes Saturday mornings." or "No note or comment visible for this cell.", and
  Ctrl+Shift+Apostrophe opens "Cells with Notes | List1 List view | B2: Includes Saturday
  mornings. | 1 of 1".
- **Built** (`e586b63e`): two of Excel's lists, `SheetListKind::Notes` and `Note`, read through
  Excel's object model (the sheet's Comments within the window's visible range) and traced by
  okur-lab at each step; 1183 was recorded a third time with that okur-lab (its first run met an
  Excel that no longer opened files, left by round 1182, restarted on old-pc with its Activation
  Wizard and Document Recovery closed; its second traced no notes).
- **Replay** (`9e365b5e`): JAWS's main window came back to the front between its dialog's first
  focus and its list's selection, and the converter dropped both; a JAWS dialog's foreground now
  stands for its first focus where the step ends with none traced in Okur's dialog, the item's
  place given from Okur's list. The words JAWS says before its list are the accepted divergence
  of its other Excel lists (`deda80ca`). `okur-lab parity`: `JAWS parity (Ia2): 12555/13151 steps
  match exactly` (1183: 8/8).
- Round 1182 (Excel's window as the focus returns) is not imported: Escape from the Run dialog
  went to the desktop. The opening's "Please wait | ... | Hours" is an open item.

## Excel's information keys (round 1184)

- **Recorded** (`0c11b30e`, the third recording; the first had Excel's "already open" question
  take trial 1, the second traced no region): JAWS's Alt+Shift+V, "visible range is A1 through
  X30 | 12 of the 720 visible cells contain data."; Alt+Shift+G, "Gridlines are on"; Alt+Shift+H,
  "no hyperlink associated with this cell."; Ctrl+Shift+V, "This cell has no input message.";
  Insert+Ctrl+F2, "No formula in current cell."; Ctrl+NumPad5, "10"; Ctrl+Shift+Space, "select |
  A 1 | Branch | C 4 | 14"; Shift+Backspace, "Selection collapsed to active cell | 12 | B 2";
  Alt+Shift+End, "14 | C 4 | Last cell in current data region". Alt+1, 2, 4 and Ctrl+2 said
  nothing.
- **Built** (`b0509c44`, `6305fbc4`): one read of Excel's object model, `SheetCells::Details`
  (the visible range and `CountA` over it, `DisplayGridlines`, the first hyperlink, the
  validation's input message, the formula, `CurrentRegion`), traced by okur-lab in each step's
  complete place. Alt+Shift+Home and End ask for the region, select its edge, keep the old cell's
  focus on the way quiet and say the cell moved to with the edge's words. The keys sit before the
  reading keys (Ctrl+NumPad5 is Say Paragraph elsewhere). `okur-lab parity`: `JAWS parity (Ia2):
  12575/13173 steps match exactly` (1184: 20/22).
- Left: Ctrl+Insert+V's "Microsoft Excel Retail Version" (Okur says the file's product, "Microsoft
  Office 2016 Version"), Alt+Shift+B's border ("Black Thin Grid Line surrounding Border of active
  cell.", wants a fixture with borders), a hyperlink's, an input message's and a formula's own
  text (not recorded).
- Old-pc's Excel: kept running by `sr-excel-comments.xlsx` (a fresh Excel shows Office's
  Activation Wizard; the fixture's own workbook as the keeper made Excel ask to reopen it).

## Excel's entry keys and Say Current Focus (rounds 1185, 1186)

- **Recorded** (`7cc5adb1`): AutoSum (Alt+Equals), "Type in the range to Sum using colon to
  separate or | Press enter to accept default range"; Ctrl+Semicolon, "Inserted date";
  Ctrl+Shift+Semicolon, "Inserted time"; Alt+PageDown, "Moved right one screen | blank | Z 5 |
  Active Cell: Z5", Alt+PageUp likewise left. Insert+Tab, "Hours | Branch | A 1"; twice quickly
  on a cell with text, the Virtual Viewer's "Description of cell: | Cell Font and Formatting: |
  Calibri | Regular | 11 point | cell font text color Black | cell font background color White |
  vertical alignment | horizontal alignment | format general | Normal style | Press ESCAPE to
  close this message." (Excel's own alignments left empty), on an empty cell "blank" alone;
  Escape from the viewer says the cell again. PageDown and PageUp say the cell alone; Ctrl+A says
  the region selected as Ctrl+Shift+End does.
- **Built** (`0d29ecc5`): `spreadsheet::entry_key_said` (the key goes on to Excel), the screen
  move's words before the cell (`excel::key_words`, also for an empty cell's place), the
  `SheetReading::Focus` and `Appearance` readings, `Command::SayCurrentFocusInDetail` (Say
  Current Focus pressed twice; elsewhere the focus as once), the application's `focus_again`
  after the viewer closes, Ctrl+A a selecting key. `okur-lab parity`: `JAWS parity (Ia2):
  12611/13209 steps match exactly` (1185: 17/17, 1186: 19/19).

## Excel's worksheet lists, borders, totals and hyperlinks (rounds 1187 to 1194)

- **1187** (`10441d1f`, recorded in round 1189's run; the first lost Excel's focus in a trial):
  Ctrl+Shift+8 silent and Ctrl+Home then "First cell" alone (the cell already active, no
  selection said since; after Ctrl+Shift+End's said selection it is "First cell | Branch | A 1",
  1186/s17); Ctrl+Shift+B "Collecting cells at PageBreaks. | no horizontal or vertical page
  breaks."; Ctrl+Shift+O "Please wait, collecting objects. | Worksheet contains no objects.";
  Insert+Ctrl+C "The chart is not active. You must first press Insert+ALT+c to select it.";
  Insert+Alt+C "There are no charts on this worksheet."; Insert+F7 "Please wait, collecting
  hyperlinks. | No hyperlinks on worksheet."; Insert+Delete without a SUM in the row "no totals
  column defined and I cannot find a sum formula in the current row."; Equals "type in the
  formula and press enter". Built in `7922552b`: four new `SheetListKind`s read from Excel's
  object model (page breaks, shapes, charts, hyperlinks), traced by okur-lab.
- **1188** (`a1e22736`, recorded in round 1192's run with okur-lab tracing the border edges and
  the fill pattern; Insert+F needs `settle_max_ms` 20000, its 12 lines outlast 10 s): Alt+Shift+B,
  "Black Thin Grid Line surrounding Border of active cell.", "Red Thick Continuous Bottom Edge,
  Black Thin Grid Line top, left and right borders of active cell.", "green Medium Continuous
  Right Edge, Black Thin Continuous Left Edge, Black Thick Double Bottom Edge, Black Thin Dot Top
  Edge"; the appearance viewer's fill "Yellow solid" after the font's white background; Insert+F's
  dimensions rounded (76 for 75.75). Built in `743e944d` (`spreadsheet::border_said`, Office's
  standard colors named before the color table: #00B050 is JAWS's "green").
- **1189** (not imported: no trial settles): Ctrl+Insert+Enter and Ctrl+Insert+Delete run a JAWS
  script that fails ("unknown function call to: settotalscolumntocurrent | Setting totals column
  to") yet sets the totals; the readers then read that cell. Built in `956e8c31` as an accepted
  divergence: "Setting totals column to B", "Setting totals row to 2". Round 1189 also read
  Excel's version: EXCEL.EXE's ProductName "Microsoft Office 2016", Application.Name "Microsoft
  Excel", ProductCode {90160000-0011-0000-1000-0000000FF1CE}; where JAWS's "Retail" comes from
  is still open (Ctrl+Insert+V, 1184/s8).
- **1191** (new fixture `corpus/pending/desktop-excel-links.ps1`, hyperlinks and manual page
  breaks): Alt+Shift+H "https://example.com/"; Down onto a link cell "Docs | has hyperlink | A 2 |
  To activate press Enter.", Up "Home page | has hyperlink | A 1" (built in `15958f73`, the hint
  not twice in a row, a hypothesis 1195 should test); Insert+F7 "Select Hyperlink dialog | List
  box | Home page | 1 of 2". JAWS's Select Hyperlink dialog then took none of the recorder's keys
  (no focus events, only the recorder's own "Speech history cleared"), so 1191 now ends at
  Insert+F7, and 1194 records the page breaks' list alone. In replay Okur names the list box
  after the dialog ("Select Hyperlink List box"), still to find.
- **1193** (cell PlaceMarkers, Ctrl+Windows+K and the rest) and **1194** recorded or running
  when old-pc went offline (2026-11-07, "target_offline"); fetch both when it is back.
- `okur-lab parity`: `JAWS parity (Ia2): 12651/13248 steps match exactly; 388 accepted
  divergences not counted`.

## Also

- README: the demo video's badge (`fd540b71`, the owner's YouTube video); "Trying it" built
  `okur-lab` where it runs `okur` (`06bbd332`, since `a8f1b0a6`).

## Next

1. ~~**Web Verbosity Level, built**~~ (done above; the plan as written): a setting with High, Medium
   (the default) and Low, per application, read from the application's JCF; replay merges the
   recording's application JCF (Chrome.JCF, in its provenance) over DEFAULT.JCF as JAWS does, and
   the reference machine is High, so parity keeps measuring what was recorded. The level reaches
   the linearizer as a reading mark (`ReadingMarks`, as Identify Same Page Links does); a
   structure off has no start and end lines, its spans kept for R, Shift+R, O and the Regions list
   (cursor.rs reads landmarks from spans). Record first: one page with every landmark, an
   article, a figure, a block quote, a list and a table read with Down at Medium and at Low
   (`jaws_settings` Chrome.JCF VirtualCursorVerbosityLevel 1 and 0, to confirm those values).
2. ~~Round 1172's structures~~, per site levels, ~~frames' ends, DescribedBy, clickable
   elements, titles~~ (done above).
3. Excel: ~~notes~~ (done above); the rest of JAWS's Excel keys (the README's "Not yet"), and
   the window as it opens (an open item).
4. Then the plan's §3.2 in its order: Settings Center's other groups beside JAWS's.
5. Linux: more of the web corpus in the lab (379/379 now), GTK's controls.
