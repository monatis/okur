# Handoff: Word's keys and comments, task panes, JAWS's Access Key choices (2026-10-29, second)

Follows [`2026-10-29-settings-results-and-word-keys.md`](2026-10-29-settings-results-and-word-keys.md)
and covers the commits after it (`791e55e8` to `ac55f505`) and this one, made under
[`docs/plans/2026-10-29-everyday-gaps-round-37.md`](../plans/2026-10-29-everyday-gaps-round-37.md).
The next plan is [`docs/plans/2026-10-29-everyday-gaps-round-38.md`](../plans/2026-10-29-everyday-gaps-round-38.md).

Where things stand:
- **Word:** its formatting keys whose result JAWS says are built and recorded (Track Changes,
  alignment, font size, indent, change case), comments in a line and Alt+Shift+Apostrophe, the
  Spelling pane's controls as a task pane's. Waiting: JAWS's comments and footnotes lists
  (the recorder loses JAWS's history after JAWS's own list dialog), Insert Hyperlink (a Pressed
  state), Save As (a tracer artifact), the Spelling pane's recording (JAWS's own focus differs
  by trial); Excel and Outlook.
- **Settings' long tail:** Tutor Messages' and Access Key's other choices are built, JAWS's file
  encoding learned, and their behavior recorded (1084-1086). Left: Voices, Run Okur from System
  Tray, Basics' Automatically Start button, the lifetimes' icons.
- **Review of 2026-10-25:** findings 8 (okur-ui's tree index) and 19 (the dialogs' shared
  choices) fixed; 20's wizard pages kept by design.

## What was built

### Word's keys, one path (`791e55e8`, `d25390ee`, `900006b4`)

A key whose result JAWS says is read from Word's object model once Word has acted on it
(polled until it changes, or 250 ms), as `WordKey` and `WordKeyState`, one request and one
event; replay answers from the recorder's snapshot after each step.
- **Font size** (Ctrl+Shift+Period, Comma): "16 point" [obs, 1074]. On old-pc's Turkish layout
  the keys reach Word as another key, so the size stays; JAWS says it all the same.
- **Indent** (Ctrl+M, Ctrl+Shift+M): JAWS measures from the page's edge, margin and indent:
  "indent | 1.5 inches" for 0.5 inch on a 1-inch margin [obs, 1080, round-1080c beside Word's
  values]. JAWS often says the value from before the key; 1080/s3 is a divergence.
- **Change case** (Shift+F3): an edit key, `EditKey::ChangeCase`, says the selection as Word
  leaves it. JAWS says it one change behind, its words run together (1078/s3-s5, divergences).
- Ctrl+B, I, U, the line spacing keys, the heading style keys, Undo and Redo, Copy and Paste
  and the List Bullet style match as recorded (1070-1075, 1079): JAWS says nothing or what Okur
  already said.

### Word's comments (`be8688cc`) [obs, 1081]

Word's object model gives the comments on the caret's paragraph (asked only for a document
that has any, once a second), part of the line's structure. A line marks a comment's start and
breaks at its end: "The library opens at has comment nine in the morning .".
Alt+Shift+Apostrophe says the comment at the caret and its author, or "No comment mark at
cursor."; elsewhere the key is the application's. JAWS reads "vispero:comment" first, a
divergence.

### Word's task panes (`97afebcc`) [obs, 1069]

A NetUI focus inside a dock other than the ribbon's (`MsoDockTop`) is a task pane's: not the
ribbon's, said without the dock and pane, its list items placed among their siblings; Escape
back says the document's name and "Edit". NetUI focus events carry their window's class.

### JAWS's Access Key and Tutor choices (`243f3a46`, `481c9d3b`, `b89578b2`)

- Basics and the Startup Wizard offer Tutor Messages' "Announce Custom Messages Only" and
  Access Key's "Speak Menus Only" and "Speak Dialogs Only" (`OutputMode::CustomOnly`,
  `MenusOnly`, `DialogsOnly`); the dialogs share their choices (`Verbosity::chosen`, `choose`).
- `access-key-modes-walk.ps1` chose each in JAWS's Startup Wizard and read DEFAULT.JCF:
  `ACCESS_KEY` 0, 1, 2, 3 per level, `TUTOR` 2, left out at its default; old-pc's file was put
  back as it was ("file as before: True"). Okur's import reads them so.
- Recorded with each choice set through `jaws_settings` (1084-1086): at Speak all and Speak
  Menus Only, a Win32 menu item ends with its letter ("New Ctrl+N | To move through items press
  up or down arrow. | n", the bar's "f"); at Speak all and Speak Dialogs Only, a dialog's control
  with its key ("alt+o"). Okur's Win32 menu items now carry MSAA's shortcut, and the menus say
  the letter last.

### okur-ui (`95518883`)

A tree's items are found by their place from the order `fill_tree` inserted them, not by
walking the tree for each lookup (finding 8, its UI half).

## Harness

- **The recorder cancels what the fixture's window has open before closing it** (`662ee5f8`):
  a failed Save As trial left Word refusing to close, and round-1078's keys went to its
  dialogs (that round was dropped; old-pc cleaned by posting OK and Cancel).
- **The recorder's snapshot** saves the font size and the paragraph's left edge.
- **desktop-word-notes.ps1**, a document with a comment and a footnote (`d24d13ea`); the
  recorder sends Semicolon and Apostrophe by their US places.
- **access-key-modes-walk.ps1**: JAWS's Startup Wizard driven to learn its settings file.

## Found, and how

- **A blocked commit drops the command's `git add` too:** after the gate blocked two commits, the
  retries staged only the file fixed, so `4df9a613` held the manifest alone and `6b6d60c5` the
  written list alone; `d24d13ea` and `54e1ae00` committed what they left out, and the 1078
  divergence's table entry went in with `e62e9513`. Stage every path in its own command, check
  `git status`, then commit.
- **JAWS's own list dialogs** (Ctrl+Shift+Apostrophe, Insert+Shift+F in Word): after them each
  step's speech is "Speech history cleared" (1082, 1083 in round-1080c).
- **Save As's opening** (1077): an MSAA focus in its folder tree came with a UI Automation chain
  already on "Save as type", so replay hears a focus JAWS never had.
- **old-pc's link** dropped twice for minutes (os error 10060); fetches resumed.

## Proof

`okur-lab parity` at `ac55f505`:

```
JAWS spoken parity: 9182/9907 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 9282/9846 steps match exactly; 280 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `a4dc7bf6`: 9212/9776, 270 accepted divergences. `okur-lab parity --only`
for this stretch's scenarios, each line as printed:

```
1071: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1073: JAWS parity (Ia2): 7/7 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1074: JAWS parity (Ia2): 6/6 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1075: JAWS parity (Ia2): 7/7 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1078: JAWS parity (Ia2): 3/3 steps match exactly; 3 accepted divergences not counted (0 scenarios could not run)
1079: JAWS parity (Ia2): 5/5 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1080: JAWS parity (Ia2): 4/4 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1081: JAWS parity (Ia2): 7/7 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1084: JAWS parity (Ia2): 8/8 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1085: JAWS parity (Ia2): 7/7 steps match exactly; 2 accepted divergences not counted (0 scenarios could not run)
1086: JAWS parity (Ia2): 7/7 steps match exactly; 2 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace --no-fail-fast`: 1536 passed, 0 failed, 200 ignored (summed from
cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
`okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's statistics: 706
scenarios, 2124 trials, 2107 succeeded.

Not checked live: the Access Key choices and menu letters in Okur's own run, okur-ui's tree
index, Settings Center's results (the dev box needs the owner's leave).

## old-pc

- Idle after round-1084. JAWS at Rate=90, volume 1%, its Access Key and Tutor choices as found
  (Off, Announce Menu and Control Help).
- Pending scenarios there and here: 1062 (the Okur cursor's words), 1069 (Spelling pane), 1076
  (Insert Hyperlink), 1077 (Save As), 1082, 1083 (comments and footnotes lists).
- Still the owner's: Excel's three processes from 521's trials, the windowless Word and the
  PowerShell; the owner's "Document1" is open.

## Next

Plan 38: the recorder after JAWS's own list dialogs, then Word's lists of comments and
footnotes; the settings' long tail (Voices, Run from System Tray); the review's open findings.
