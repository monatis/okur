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
- Not yet: Configure Web Verbosity Levels' own choices per level (a dialog of 27 check items; JAWS keeps a change as `[VirtualCursorVerbosity] ApplicationRegion=0|1|0`, one value per level),
  per site levels, and the structures Okur's buffer does not mark yet (figure, feed, log, note,
  toolbar, frame, editable section, DescribedBy).

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
2. Then the plan's §3.2 in its order: Settings Center's other groups beside JAWS's.
3. Linux: more of the web corpus in the lab (379/379 now), GTK's controls.
