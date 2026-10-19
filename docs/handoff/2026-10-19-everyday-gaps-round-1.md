# Handoff: everyday-use gaps, round 1 (2026-10-19)

Follows [`2026-10-19-reading-settings-round-1.md`](2026-10-19-reading-settings-round-1.md).
The work list is [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md)
(the 43 gaps, each row updated as features landed); the next plan is
[`docs/plans/2026-10-20-everyday-gaps-round-2.md`](../plans/2026-10-20-everyday-gaps-round-2.md).

## The owner's decisions (2026-10-19)

- The gaps report is accepted: the everyday-use gaps are closed before macOS or any public
  announcement.
- old-pc is always kept recording: one round queued behind the running one.
- The commit gate checks formatting, as CI does (after a CI failure on a pushed commit).

## Two questions for the owner

1. **The browser profile's name in the corpus.** old-pc's Chrome profile is named after the
   owner, and Chrome adds it to every window title ("Riverside Gazette - Google Chrome - M.
   Yusuf"). It is in 477 committed traces and in 4 evidence files, where JAWS itself speaks the
   title. New web recordings are held back until this is decided (530, 531, 534, 535, 536,
   538, 539, and 537, a desktop one whose first trace caught a browser title). Proposed: the
   fixtures open in a Chrome of their own (`--user-data-dir` under `sr-harness`), signed out,
   whose single profile adds no name, and the web scenarios are recorded again over the coming
   rounds. History is not rewritten.
2. **Office on old-pc** still needs activating; 518, 519 and 521 wait for it.

## What was done

### Features (every one checked by `okur parity` against old-pc's evidence)

- **Native edits** (`710d1da`, `74eea7d`): Ctrl+X, Ctrl+V, Ctrl+Z ("cut selection to
  clipboard", "Pasted from clipboard", "Undo"); Say Selected Text, Say To Cursor and Say From
  Cursor spelled when pressed twice quickly (JAWS's history keeps a spelled selection as it is);
  Insert+Left and Right by word in the desktop layout; the sentence and paragraph keys in
  applications, read to the end of the line they start on; selection as JAWS reads it in a
  plain Win32 edit: words before "selected", "Unselected" and what left the selection, a
  multi-line selection's first line only (RichEdit, as Windows 11 Notepad, keeps every line, as
  the owner reported) [313, 504, 510, 529].
- **Say All** (`e658800`, `27a50a8`): Right and Left skip and rewind a line (no further back than
  where reading began), PageUp and PageDown change the rate without restarting the line, a
  quick key moves and reading goes on [500/s10]; in a text field the caret follows the line
  being spoken [505/s2].
- **The window's bottom line** (`710d1da`, `c0f4d79`): the status bar where the platform reads
  one, else an editor's last line at any line break (WordPad's bare carriage returns made one
  line of the whole document).
- **Insert+E outside a dialog box and Insert+Q** (`995a178`): "Not in a dialog box";
  "Default settings are loaded", the application, and its name spelled [513].
- **A native popup with no menu bar is a "Context Menu"** (`d4e620d`) [520/s3].
- **A list view's columns** (`420d7b2`): Ctrl+Insert+1 to 0 read a column with its header
  ("Name", "budget.xlsx"), past the last "3 out of range", "this listview only contains 2
  columns."; the tracer and the live platform read the headers from the list's Header [514].
- **A NetUI ribbon** (`2f4d20e`): in WordPad's and Paint's ribbon the focus is read from MSAA,
  where UI Automation stops at the ribbon's pane: "View Tab selected" where Okur was silent
  [523/s2-s5]; JAWS's positions there are not read yet.
- **Page tools** (`7a27dd1`, `18e6311`): Ctrl+Insert+F7 the link's address; Insert+Delete and
  Ctrl+Insert+Delete route the cursors ("Route Virtual to PC", "Route PC to Virtual Cursor");
  Insert+F9 "No frames found in page"; Ctrl+Insert+Shift+Enter the prior heading; Numpad Slash
  "left mouse button" and a click on the element at the virtual cursor [536, 534].
- **A dialog box's default button and the hot key** (`a36a1dd`): Insert+E "default button is
  No" (the focused button when a button has the focus); Shift+Numpad5 "Alt+r", in a dialog box
  "No", "alt+N", or "no hot key" [537].
- **JAWS Find** (`17af17b`): Ctrl+F says "Virtual Find" before the dialog, Insert+Ctrl+F does
  not; the Find what field no longer says "1 of 1" (live5) [506].

### Harness

- **Live runs at old-pc's voice rate** (`e658800`): JAWS's profile is at 90, which the voice
  receives as SAPI's top rate; `okur reference-settings` now carries it. live2 showed Say All's
  marks coming back; the cursor had stayed on the first line only because Okur spoke at rate 0.
- **A trial never starts in a Windows menu** (`27a50a8`): a Windows key chord (Win+Insert+X)
  goes to Windows on old-pc and left the Start menu open, which spoiled the two scenarios after
  it; `run-scenario.ps1` closes such a menu before a trial and records it.
- **File dialogs from a fixture of their own** (`f29d9b1`): Notepad's Open and Save As dialogs
  opened in the lab account's home folder whatever their last folder and Notepad's working
  folder were (rounds 27, 31), and the traces listed its folders. 524, 525 and 532 now use
  `desktop-file-dialogs.ps1`, whose dialogs are set to `%TEMP%\sr-dialogs`.
- **A dialog's traced tree keeps no items inside its tree views** (`d73f1ae`): a file dialog's
  Quick access lists the account's recent folders.
- **`text` steps**, **staged scenarios** (`queue-batch.ps1 -StagedScripts`), `-Before` for a
  setup before a batch, and `expects_notifications` for a scenario about toasts (`737cb02`).
- **Browser windows put away before a desktop trial's trace starts** (`e5ff7ac`, `2336e19`): a
  second, since the tracer's first event is where the focus already is.

### Recorded and imported

527 (help keys in Notepad), 529 (selecting in a native edit), 526 (screen-sensitive help on a
form's controls), 514 again with column headers traced, 533 (a toast, the notification keys, a
progress bar). Held back: the web recordings (530, 531, 534, 535, 536, 538, 539) for the
browser profile's name, and 537 (one trace caught a browser title); 524, 525 and 532 wait for
round 38. Okur's code for 534, 536 and 537 was checked against them in scratch corpora: 8/10,
15/16 and 11/12 steps (`okur parity -c <scratch>`, quoted in the commits). 528 is dropped:
Windows key chords cannot be recorded on old-pc (520/s3).

### Accepted divergences added (the owner's rule on JAWS's accidents)

- 504/s22: JAWS's bottom line drops the emoji that ends the editor's last line, as its control
  view does (312).
- 522: JAWS reads none of WordPad's document text on old-pc, in every trial (silent arrows,
  "Nothing selected" with text selected), though it reads the font there.

## Numbers

`cargo run -p okur -- parity` (replay of old-pc's evidence through the engine):

```
JAWS spoken parity: 1897/2405 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 1982/2457 steps match exactly; 49 accepted divergences not counted (0 scenarios could not run)
```

`okur live-parity` on live4 (Say All) and live5 (Okur reading on old-pc, 2026-10-19):

```
500-web-say-all-cursor: Okur live: 3/11 steps match JAWS exactly; 0 accepted divergences not counted
505-desktop-say-all-in-an-edit: Okur live: 2/5 steps match JAWS exactly; 0 accepted divergences not counted
506-web-jaws-find: Okur live: 7/13 steps match JAWS exactly; 0 accepted divergences not counted
511-desktop-window-list-and-tray: Okur live: 0/6 steps match JAWS exactly; 0 accepted divergences not counted
513-desktop-dialog-helpers: Okur live: 4/8 steps match JAWS exactly; 0 accepted divergences not counted
514-desktop-list-view-columns: Okur live: 11/12 steps match JAWS exactly; 0 accepted divergences not counted
517-web-speech-layer-keys: Okur live: 8/10 steps match JAWS exactly; 0 accepted divergences not counted
```

Where Ctrl stopped Say All, 500/s2 is a line short of JAWS's earlier trial, s5 between JAWS's
two trials, s8 and s11 behind both; 505/s2 matches. Replay cannot time Say All: its recording
voice reports no progress. 511 is 0/6 because Okur has no Window List or system tray list yet.

## old-pc when this was written

- Round 38 (524, 525, 532 with `desktop-file-dialogs.ps1` and the pruning tracer) recording;
  round 39 (540, 541: Notepad's Find and Font dialogs), round 40 (542, 543: Replace and Go To)
  and live6 (506, 513, 500, 505, 514, 517, 537 with today's Okur) queued behind it.
- Fetch with `skills/jaws-reference/scripts/lab-fetch.sh`, wrapped in `timeout`: one fetch
  stalled for ten minutes and had to be stopped.
- A command the commit hook denies runs not at all: run generators and `git add` before.

## Not done, and why

- The Window List (Insert+F10) and the system tray list (Insert+F11): Okur has neither (511).
- Toasts: the tracer and live Okur watch no shell notification window, so Okur says nothing as
  one arrives (533/s1).
- Screen-sensitive help, hot key help and Windows keys help (526, 527): JAWS's texts are
  Freedom Scientific's and are not copied; Okur needs help texts of its own, and those steps
  become accepted divergences.
- Ctrl+Insert+V (the application's version) needs a platform event: actions do not return
  values. Alt+Delete needs the focus's position on screen, which traces do not hold; it goes
  with the JAWS cursor (516). Say Color needs the screen's text colors.
- WordPad's status bar is not collected (`screen_lines::window_bars` finds only a status bar
  that is the window's own child); Say Window in a message box says nothing (537/s8).
- Say All by sentence or paragraph, and with the Shift keys; the page-load summary waits for
  538 and 539 (plural wording).
