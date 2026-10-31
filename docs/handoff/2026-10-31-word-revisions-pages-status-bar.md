# Handoff: Word's notes and revisions in the Virtual Viewer, its pages and status bar (2026-10-31)

Follows [`2026-10-29-word-notes-and-bookmarks.md`](2026-10-29-word-notes-and-bookmarks.md)
and covers the commits after it (`8a0d1ce9` to this one), made under
[`docs/plans/2026-10-29-everyday-gaps-round-39.md`](../plans/2026-10-29-everyday-gaps-round-39.md).
The next plan is [`docs/plans/2026-10-31-everyday-gaps-round-40.md`](../plans/2026-10-31-everyday-gaps-round-40.md).

Where things stand:
- **Word:** the Virtual Viewer of its comments, footnotes, endnotes and revisions
  (Windows+Semicolon, Alt+Shift+Apostrophe twice); JAWS's revision keys; deleted text not
  read; pages and sections said as the caret reaches them; Insert+Delete; the status bar; a
  note's reference selected as superscript; Save As's and Insert Hyperlink's open steps. Open:
  the Spelling pane's suggestion said twice (1069/s4), Save As's file name said again
  (1077/s1), Insert+Alt+R.
- **Excel:** old-pc records it now (see old-pc); 521, the first Excel scenario, is in the
  corpus, and Okur does not yet read Excel as JAWS does.
- **The dev box is shared:** cargo runs on 4 of its 16 cores for Claude Code's sessions.

## What was built

### Word's notes in the Virtual Viewer (`8a0d1ce9`, `f16e4b0e`, `68b72e85`)

- **Alt+Shift+Apostrophe twice** opens the comment in the Virtual Viewer: the comment, "by
  Lab", "Document text:", the words it is on, "Press ESCAPE to close this message." [obs,
  1095/s6].
- **Windows+Semicolon** opens "Items to Display in Virtual Viewer" (List1: Comments,
  Footnotes, Endnotes, Revisions; OK, Cancel) [obs, 1094, 1098]. OK says "Please wait:
  Processing Comments" and opens the viewer on the count, "There is only one Comment to
  display.", "There are 2 Comments in this document.", then each note after a link of its
  number; comments and revisions on the link's line, footnotes and endnotes on a line of their
  own with their number, apart from the next [obs, 1096, 1097, 1099, 1101, 1102, 1104, 1105].
  With none: "There are no items to display. | Edit" [obs, 1100, 1103].

### Word's revisions (`d4b3ae4a`, `2dbd87fe`)

- **The viewer's Revisions:** "There are 2 Revisions in this document.", each "1  eight
  inserted text Lab Sun, Jan 04, 26" [obs, 1104].
- **Insert+Shift+R and Ctrl+Insert+R** (JAWS's `listRevisions` and `AnnounceRevision`, from its
  Word key map) ask first whether Word tracks changes: without, "Track Changes is disabled.
  Press the Word command, CONTROL+SHIFT+E, to enable the feature." and "You must enable Track
  Changes in order to use this feature." [obs, 1109]; with, the Revisions list ("eight inserted
  text Lab Sun, Jan 04, 26 | 1 of 2"; OK moves there, "nine Deleted | Edit") [obs, 1107] and
  the revision at the caret, "eight | inserted text revised by Lab on Sun, Jan 04, 26", or "No
  Revision" [obs, 1108]. Ctrl+Insert+R is not the regions list in Word.
- **Deleted text is not read:** Word's text holds a tracked deletion its view does not show.
  A Word context is narrowed to the text as shown, every place in it moved with it: "The
  library opens at eight in the morning.", not "eightnine" [obs, 1106].
- **With tracking on** a word read in a revision says its change, "eight | Inserted", and Say
  Line on one begins with "revision" [obs, 1107/s9, 1108/s7].
- The platform places the caret paragraph's revisions as its comments
  (`word_om::paragraph_revisions`) and reads the document's (`Document.Revisions`).

### Pages, sections, Insert+Delete, superscript (`b072a265`)

- A line key onto another page says it first, "page 2", and the section where it changed,
  "page 3 | Section 2"; Ctrl+End too, before "Bottom of file" [obs, 1112, 1113/s7]. The place
  comes from Word's `Selection.Information` (page, section, line on the page, column).
- **Ctrl+PageDown and Ctrl+PageUp:** "Next page", "Previous page", then the page and the line
  [obs, 1113/s1-s4].
- **Insert+Delete** (JAWS's `sayLineAndColumn` in Word): "line 2 column 28" [obs, 1112/s7,
  1113/s6, s8]; Route Virtual to PC elsewhere.
- **A note's reference selected** (Shift+Right over it, which the text does not show):
  "superscript | selected"; Down from there, "end superscript" before the next line [obs,
  1092/s8, s10]. The platform reports a reference Word's selection holds.

### Word's status bar (`b5505d2d`)

Insert+PageDown in Word reads its status bar as JAWS does: each button by its name, a view's
button only where it is on, not the zoom's slider or the size box, "Page Number Page 3 of 3 |
Word Count 26 words | Spelling and Grammar Check No Errors | Language English (United States) |
Print Layout | Zoom Out | Zoom In | Zoom 100%" [obs, 1113/s9]. It is found by its windows (the
bottom dock's "Status Bar" command bar), whose UI Automation tree is small.

### Dialogs (`05487d06`, `7de2a395`, `7a3dab47`, `68b72e85`)

- Save As: a Win32 combo box's MSAA focus wins over a stale UI Automation chain [obs,
  1077/s2]; back in Word, "Edit", as from Word's own dialogs [obs, 1077/s4].
- Insert Hyperlink: an Office command bar's button (MSAA only) with "Pressed" and its
  description, "Existing File or Web Page Button Pressed | Existing File or Web Page button"
  [obs, 1076/s5].

### Review findings (`cc50d0f6`, `3a05b2a9`)

7: settings written on a thread of their own, the newest snapshot, exit waiting for it. 9:
okur-ui's exit reports each window it left open closed.

## Harness

- **The recorder's Windows key** (`68b72e85`, `d4b3ae4a`): a Windows key chord whose other key
  JAWS takes (Windows+Semicolon) opened the Start menu, whose search box took the next steps.
  A mask key (VK 0xE8) is now sent as soon as the Windows key goes down: sent after the other
  key, it was refused at times while JAWS's dialog came forward. 1104 then recorded three clean
  trials (round-1104b).
- **Replay answers chained reads** (`2dbd87fe`): the revisions, asked once Word says it tracks
  changes, are answered at once, before the list's window reports anything, as live.
- **The snapshot** saves the document's revisions and Word's status bar; **the traces** carry
  each line's revisions, its place on the pages and a selected reference.
- **Closing a fixture's window** (`8fc9c9e4`, `run-scenario.ps1`): with `WM_SYSCOMMAND SC_CLOSE`, as its
  Close button does; Excel took the old `WM_CLOSE` for quitting, every workbook with it.
  Office's own "Want to save your changes to ...?" (a NetUI dialog) is answered Don't Save,
  fixtures being written anew each run. A window the fixture's process leaves naming no
  document (an empty Excel frame) is closed too; Word's process keeps its other documents'
  windows and is left alone.
- **desktop-excel.ps1** (`e4bbeefc`; until now only on old-pc) opens its workbook through Windows, into
  the Excel already running, and no longer asks Excel to calculate it on opening. Excel still
  recalculates it (the package names no calculation engine) and asks to save on closing.

## Found, and how

- **JAWS's revision keys need Track Changes on** (1107, 1108 first recorded without it):
  recorded again after Ctrl+Shift+E.
- **JAWS changes Word's view to gather revisions:** the trace of 1104/s5 shows the "Normal"
  view, zoom changes and the document's focus twice; JAWS says "Print View" as it comes back,
  and again as the viewer closes. An accepted divergence.
- **Insert+Alt+R (`RevisionDetectionToggle` in JAWS's key map) reached Word** as Alt+R in both
  states (1110, 1111, research rounds not imported: their traces carry the ribbon's focus,
  which replay cannot follow). Okur takes the chord.
- **An edit script wrote raw control characters** into Rust source where it meant `\u{1d}`
  (a JavaScript template literal turned the escape into the character); fixed in `2dbd87fe`.
  Write such escapes with the Edit tool.
- **A blocked commit runs nothing of its command**, a `mv` of a fixture beside it included: the
  gate then saw the fixture and a stale manifest. Move first, commit in a command of its own.
- **Excel on old-pc** (rounds 521e, 521f): each trial's `EXCEL.EXE <path>` started a new Excel
  process, which shows Office's Activation Wizard inside its window and took the trial's keys.
  Ending the processes left the fixture marked ("The last time you opened
  'sr-excel-fixture.xlsx', it caused a serious error. Do you still want to open it?") and
  Excel's Document Recovery pane open, under which Excel refuses to close anything ("Cannot
  quit Microsoft Excel."). Answered by hand on old-pc: Yes to the question, the pane's Close,
  the trials' blank workbooks closed with their own Close buttons.

## Proof

`okur-lab parity` at `8fc9c9e4`:

```
JAWS spoken parity: 9412/10162 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 9521/10101 steps match exactly; 284 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `b2bd14ef`: 9361/9932, 282 accepted divergences. `okur-lab parity --only`
for this stretch's scenarios, each line as printed:

```
1069: JAWS parity (Ia2): 5/6 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1076: JAWS parity (Ia2): 7/7 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1077: JAWS parity (Ia2): 3/4 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1092: JAWS parity (Ia2): 15/15 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1094: JAWS parity (Ia2): 6/6 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1095: JAWS parity (Ia2): 11/11 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1096: JAWS parity (Ia2): 8/8 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1097: JAWS parity (Ia2): 8/8 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1098: JAWS parity (Ia2): 7/7 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1099: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1100: JAWS parity (Ia2): 3/3 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1101: JAWS parity (Ia2): 10/10 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1102: JAWS parity (Ia2): 11/11 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1103: JAWS parity (Ia2): 8/8 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1104: JAWS parity (Ia2): 13/13 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1105: JAWS parity (Ia2): 12/12 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1106: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1107: JAWS parity (Ia2): 9/9 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1108: JAWS parity (Ia2): 10/10 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1109: JAWS parity (Ia2): 4/4 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1112: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1113: JAWS parity (Ia2): 8/8 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
521: JAWS parity (Ia2): 0/14 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace --no-fail-fast` (4 test threads) at `f06069d9`, the code unchanged
since: 1556 passed, 0 failed, 200 ignored (summed from cargo's 35 result lines). `okur-lab
invariants`: "Buffer invariants: 0 violation(s)". `okur-lab corpus-manifest`: "Corpus manifest
is current." The manifest's statistics: 737 scenarios, 2217 trials, 2200 succeeded.

What differs, and why it is open (plan 40):
- 1069/s4: JAWS says a suggestion twice, "the | 1 of 5 | the", in the one trial of three
  that reached the suggestions list; nothing in the trace says why.
- 1077/s1: JAWS says the file name again after the hint.
- 521: Okur reads Excel's cells the generic way, "B2 selected 12" for JAWS's "12 | B 2".

Not checked live: any of this in Okur's own run on the dev box (it needs the owner's leave).

## old-pc

- Installed: push `8512907d` (the status bar build); `run-scenario.ps1` with the Excel frame
  cleanup pushed after it. JAWS's voice profile reads `Rate=90` in every line.
- **Excel:** round-521e's three Excel processes, started by its trials and holding only the
  fixture or no workbook, would not quit (each Close gave "Cannot quit Microsoft Excel.") and
  were ended by their ids. Excel was then started once, its Activation Wizard closed (its Close
  button), and a keeper workbook made: Book1, "lab" in A1, so that Excel does not replace it.
  The fixture now opens in that instance; keep it running.
- **Word:** the keeper Document2 ("lab") is open. Word's recovered documents, after old-pc's
  power loss, were kept ("Yes, keep"), never removed. Word's STARTUP folder
  (`%APPDATA%\Microsoft\Word\STARTUP`) was found missing on 2026-10-30; nothing of ours
  touches it.
- A Run dialog (Windows+R) is open on old-pc, of unknown origin; left alone.

## Next

Plan 40: the settings' long tail (Voices' Apply question with a lab profile, the Startup
Wizard's rest), Word's open items, Excel from 521, Outlook, the review's open findings.
