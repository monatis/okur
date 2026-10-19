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
   title. New web recordings are held back until this is decided (530, 531, 534, and 535 and
   536 when they arrive). Proposed: the fixtures open in a Chrome of their own
   (`--user-data-dir` under `sr-harness`), signed out, whose single profile adds no name, and
   the web scenarios are recorded again over the coming rounds. History is not rewritten.
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

### Harness

- **Live runs at old-pc's voice rate** (`e658800`): JAWS's profile is at 90, which the voice
  receives as SAPI's top rate; `okur reference-settings` now carries it. live2 showed Say All's
  marks coming back; the cursor had stayed on the first line only because Okur spoke at rate 0.
- **A trial never starts in a Windows menu** (`27a50a8`): a Windows key chord (Win+Insert+X)
  goes to Windows on old-pc and left the Start menu open, which spoiled the two scenarios after
  it; `run-scenario.ps1` closes such a menu before a trial and records it.
- **File dialogs in a folder of their own** (`e5ff7ac`, `7e55bd5`): Notepad's Open and Save As
  dialogs opened in the lab account's home folder, whose contents then went into the traces.
  They opened there whatever their last folder and Notepad's working folder were (rounds 27
  and 31), so the dialog scenarios now use a fixture of their own, `desktop-file-dialogs.ps1`,
  whose Open and Save As dialogs are set to `%TEMP%\sr-dialogs`. `queue-batch.ps1 -Before`
  runs a setup script before a batch.
- **`text` steps**: characters typed as Unicode input, whatever the keyboard layout (old-pc's is
  Turkish Q); replay reads only their events.
- **Staged scenarios**: `queue-batch.ps1 -StagedScripts` installs scenarios as it does scripts,
  so a scenario the batch before still runs changes only after it.
- **Browser windows put away before a desktop trial's trace starts** (`e5ff7ac`).

### Recorded and imported

527 (help keys in Notepad), 529 (selecting in a native edit), 526 (screen-sensitive help on a
form's controls, recorded again without a browser window in its traces). Held back: 524, 525
and 532 (the home folder in their traces), 530, 531 and 534 (the browser profile's name), 533
(Notification History, JAWS's own dialog, refused the harness's Escape; now typed by JAWS).
528 is dropped: Windows key chords cannot be recorded on old-pc (520/s3).

### Accepted divergences added (the owner's rule on JAWS's accidents)

- 504/s22: JAWS's bottom line drops the emoji that ends the editor's last line, as its control
  view does (312).
- 522: JAWS reads none of WordPad's document text on old-pc, in every trial (silent arrows,
  "Nothing selected" with text selected), though it reads the font there.

## Numbers

`cargo run -p okur -- parity` (replay of old-pc's evidence through the engine):

```
JAWS spoken parity: 1885/2396 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 1970/2448 steps match exactly; 49 accepted divergences not counted (0 scenarios could not run)
```

`okur live-parity` on live4 (Okur reading on old-pc, 2026-10-19 19:18):

```
500-web-say-all-cursor: Okur live: 3/11 steps match JAWS exactly; 0 accepted divergences not counted
505-desktop-say-all-in-an-edit: Okur live: 2/5 steps match JAWS exactly; 0 accepted divergences not counted
```

Where Ctrl stopped Say All, 500/s2 is a line short of JAWS's earlier trial, s5 between JAWS's
two trials, s8 and s11 behind both; 505/s2 matches. Replay cannot time Say All: its recording
voice reports no progress.

## old-pc when this was written

- Round 30 (536, routing and the link's address) recording; round 31 (524, 525, 532 with the
  dialog fixture's working folder, 533 with JAWS typing Escape; its glob also records 522, 523,
  534 and 535 again) queued behind it; round 35 records 524, 525 and 532 with `desktop-file-dialogs.ps1`.
- Fetch with `skills/jaws-reference/scripts/lab-fetch.sh`, wrapped in `timeout`: one fetch
  stalled for ten minutes and had to be stopped.

## Not done, and why

- Screen-sensitive help, hot key help and Windows keys help (526, 527): JAWS's texts are
  Freedom Scientific's and are not copied; Okur needs help texts of its own, and those steps
  become accepted divergences.
- Ctrl+Insert+V (the application's version) needs a platform event: actions do not return
  values. Alt+Delete needs the focus's position on screen, which traces do not hold; it goes
  with the JAWS cursor (516). Say Color needs the screen's text colors.
- WordPad's status bar is not collected (`screen_lines::window_bars` finds only a status bar
  that is the window's own child).
- Say All by sentence or paragraph, and with the Shift keys.
