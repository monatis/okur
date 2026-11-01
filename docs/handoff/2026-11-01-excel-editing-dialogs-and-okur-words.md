# Handoff: Excel's editing, dialogs and menus; the Okur cursor's words (2026-11-01, sixth)

Follows [`2026-11-01-aria-at-complete-and-probes.md`](2026-11-01-aria-at-complete-and-probes.md)
and covers `bae0633d` to this one, made under
[`../plans/2026-10-31-everyday-gaps-round-41.md`](../plans/2026-10-31-everyday-gaps-round-41.md).

Where things stand: plan 41's Excel items are done and imported (1122 v1.1, 1123, 1124, 1125),
and so is the Okur cursor's words through a path (1062). The next plan
([`../plans/2026-11-01-everyday-gaps-round-42.md`](../plans/2026-11-01-everyday-gaps-round-42.md))
starts the first Store release, the owner's next item, while old-pc records Excel's title
reading.

## What was built

### Excel (`dbff0a01`)

- **An empty cell** comes from Excel's object model, "blank | D 1 | Active Cell: D1", after
  Right, Enter and Down (1123/s2, s5, s9). Round-1122b had no snapshots of it: its lab binary
  was older than the one that saves Excel's active cell beside each step. Recorded again
  (round-1122c), no tracer change was needed.
- **Insert+C on an empty cell** asks Excel for the active cell, "D 1" (1123/s3): the grid's
  focus is now the last native focus, so the row and column readers work there too.
- **Delete** empties the cell and UI Automation's focus moves to the grid with only a value
  change: the converter now reads such a change as the focus moving, "blank | D 1", without
  "Active Cell" (1123/s7). **Ctrl+Z** says "Undo | 7 | D 1" (1123/s8).
- **Go To** (an Office SDM dialog, `bosa_sdm_XL9`): its field is the label MSAA gives it as
  editable text, "Go To dialog | Reference: Edit | A1 | Type in text."; the field window's own
  nameless report between the dialog's two is dropped (1124/s1, s8).
- **Enter in Go To**: Excel reports the cell four times; app modules now learn a focus is a
  `repeat` (the same focus, no key since), and the cell is said once, "6 | C 3" (1124/s3).
- **Escape back to the sheet**: the cell's report comes from Excel's main window (`XLMAIN`),
  now a cell too, "6 | C 3" (1124/s7, s9).
- **Office dialogs' tabs** say "To switch pages, press Control+Tab." (1124/s4-s6).

### Office's context menus (`c10a4061`, `e3432043`)

- **A change of the clipboard closes Office's own menus** (NetUI), shown on old-pc with
  `Set-Clipboard`; reading the clipboard, clearing JAWS's history and stopping speech do not.
  Every step that reads JAWS's speech history changes the clipboard, so a recording's menu
  closed 2.5 s into its first step (1122/s2, 1125/s1). Steps inside such a menu are now
  `"history": false`, compared on the spoken layer
  ([`../../skills/jaws-reference/references/troubleshooting.md`](../../skills/jaws-reference/references/troubleshooting.md)
  §10). 1122 was recorded again that way (round-1122d), and its accepted divergence went.
- **An unavailable item after the first** says so, "Copy Unavailable" (1125/s2).
- **Leaving**: Excel's cell, "Leaving menus | Weekdays | B 1" (1122/s4); Word's document says
  "Edit" alone (1125/s4). Modules learn the menu was Office's own (`office_menu_closed`).

### The Okur cursor's words (`7654b479`)

- **Route Okur to PC makes the Okur cursor active**, on the caret's character: in 1062 JAWS's
  Ctrl+Right after Insert+NumPad Minus moved the mouse, not the caret, in every trial. JAWS's
  help says only that the route moves the cursor.
- **Its words keep a colon with the word before it** and make each other run of marks a word:
  "C:", "\", "Users", "\", "Ada", "\", "Downloads" (1062/s2-s9). A page field keeps
  `web_word_at`'s split ("std", "::"), so the rule is the Okur cursor's alone.

### Accepted divergences (`c718a19e`)

Format Cells' pages said twice (1124/s4-s6): JAWS says the page's name alone before its tab,
and the Number page's list choice "General" before or after it by the trial; opening on that
page it left out the dialog's name.

## Found, and how

- **Excel's title reading** (a live look on old-pc, research, not evidence): Insert+Ctrl+Alt+C
  says "Setting column titles row range to 1", Insert+Ctrl+Alt+R "Setting row titles column
  range to  A"; then moving says the new column's title, "Weekdays | 12 | B 2", or the new
  row's, "Northside | 10 | B 3". JAWS keeps them per workbook (its personalized settings for
  the file) and wrote an Excel settings file beside them: both removed afterwards and JAWS
  restarted, so later recordings of the fixture are as before. Excel's key map: Insert+Delete
  reads the row total and Insert+NumPad Enter the column total (`popapps.chm` names them the
  other way round); Insert+Alt+Shift+C and R say the titles, but the chord sent by
  `okur-lab keys` reached Excel as Alt+Shift+R and opened the ribbon (Alt+Shift may be
  old-pc's input-language switch).
- **`popapps.chm`** decompiled on old-pc into `Dropbox\sr-harness\jaws-docs\popapps` (not in
  the repository): Excel's JAWS keystrokes, title reading, command descriptions.
- **A scenario using the Okur cursor's screen text** must carry `"trace_screen": true`; 1062
  was imported from its first recording without it, since the replay reads Notepad's field
  text there (as 754 does), and the screen-text replay starts on the menu bar line (round-1062c
  showed it): open.

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 11096/11986 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11348/11920 steps match exactly; 365 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `7122f863`: 11321/11890 with 362 accepted. Per scenario, `okur-lab parity
--only`:

```
1062: JAWS parity (Ia2): 10/14 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1122: JAWS parity (Ia2): 7/7 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1123: JAWS parity (Ia2): 10/10 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1124: JAWS parity (Ia2): 6/6 steps match exactly; 3 accepted divergences not counted (0 scenarios could not run)
1125: JAWS parity (Ia2): 2/2 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

With `--spoken`, 1122: "JAWS spoken parity: 9/9 steps match exactly"; 1125: "JAWS spoken
parity: 4/5 steps match exactly".

`cargo test --workspace --no-fail-fast` (4 test threads): 1580 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 1910 scenarios, 5736 trials, 5719 succeeded.

Open:
- 1125/s3: Word's Paste Options gallery inside the context menu, "table | Paste Options: | Keep
  Text Only Button | Keep Text Only (T) | 1 of 1 | Navigate with left and right, up and down
  arrow keys" (spoken layer).
- 1062/s10-s13: Down below Notepad's only line and Ctrl+Right there, where JAWS says nothing:
  Okur has no model of the empty area below a field's text.
- From before: 1129/s11 (a dialog's start line), 1130/s11, 2857/s1; the owner's two choices.

Not checked live on the dev box.

## old-pc

Idle. Its JAWS settings are as before the look at title reading (Rate=90, no Excel settings
files). The keepers (Excel's Book1, Word's Document1) are open.

## Next

[`../plans/2026-11-01-everyday-gaps-round-42.md`](../plans/2026-11-01-everyday-gaps-round-42.md):
the Store release's research and package on the dev box; on old-pc, Excel's titles and totals
once the recorder forgets JAWS's settings for the fixture before each trial.
