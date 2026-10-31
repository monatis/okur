# Handoff: Excel read as JAWS reads it, first round (2026-10-31, second)

Follows [`2026-10-31-word-revisions-pages-status-bar.md`](2026-10-31-word-revisions-pages-status-bar.md)
and covers the commits after it (`dc246f12` to this one), made under
[`docs/plans/2026-10-31-everyday-gaps-round-40.md`](../plans/2026-10-31-everyday-gaps-round-40.md).
The next plan is [`docs/plans/2026-10-31-everyday-gaps-round-41.md`](../plans/2026-10-31-everyday-gaps-round-41.md).

Where things stand:
- **Excel:** a cell, an empty cell, the coordinates, the Ctrl moves, Say Line and the row and
  column readers, JAWS's three lists and their returns, F2, Enter: recorded in seven
  scenarios and matching but for selections (1117) and the sheet tabs' menu (1116/s5).
- **Word:** a page reached by a character or word key is said first (1119).
- **The dev box** runs cargo on 4 of its 16 cores (`CARGO_BUILD_JOBS`, `RUST_TEST_THREADS` in
  the untracked `.claude/settings.local.json`).

## What was built

### Excel's cells (`dc246f12`, `0dbc7ddb`, `60938e08`)

An Excel module (`apps/windows/excel.rs`), as JAWS's script for Excel:
- a cell of the grid window (`EXCEL7`, UI Automation's table cell named by its reference) is
  its value, then its coordinates, "12 | B 2" [obs, 521]; Insert+C the coordinates, "B 2";
- Ctrl+End and Ctrl+Home say "Last Cell" and "First cell" first; a Ctrl arrow says "edge of
  current data region" after, or "start of new Data Region" coming from an empty cell [obs,
  1115, 1118/s5]; modules now see the last key (`AppContext::key`);
- an empty cell has no UI Automation element: the grid itself takes the focus, and the module
  asks for the active cell from Excel's object model: "blank | D 1 | Active Cell: D1" [obs,
  1118/s3, s4] (`AppContext::ask_sheet`);
- F2's editor (`EXCEL6`) is entered silently [obs, 1118/s6]; a cell's focus gives the keys to
  Excel, as for any native control.

### Excel's object model (`6fc2f285`, `71ee0cb5`)

`excel_om`, reached from the grid window as Word's (`AccessibleObjectFromWindow` with
`OBJID_NATIVEOM` gives Excel's `Window`), shares `word_om`'s IDispatch helpers. It reads:
- the active cell's reference, row and column within the used range, for the row and column
  readers: Shift+Insert+Up, Home, PageUp ("Northside 10 6", one breath) and Ctrl+Insert with
  the same keys ("Weekdays | 12 | 10 | 22", cell after cell) [obs, 1114]; Say Line reads the
  row cell after cell [obs, 521/s1, 1118/s10];
- JAWS's lists: Insert+Shift+F "Cells with Formulas" ("B4: =SUM(B2:B3)"), Insert+Shift+C "Col
  B" ("Weekdays: B1"), Ctrl+Shift+S "Move to Worksheet" ("Hours"); OK selects the cell or
  activates the sheet [obs, 1116, 521/s9, s13].
okur-lab's snapshot saves the place and the lists; scenarios that need them carry
`"trace_headings": true`, which is what makes the recorder snapshot each step.

### Returning to a cell (`ec21fc71`, `b7757aef`)

- From Excel's menus: the engine keeps "Leaving menus" for the cell's focus and says them
  together, "Leaving menus | 14 | C 4" [obs, 1116/s7]
  (`AppModule::says_focus_after_menus`).
- From Okur's list: OK says the cell gone to (the quiet after a dialog no longer covers it),
  Escape the cell the list was opened from, "22 | B 4" [obs, 521/s10, s14]
  (`AppModule::focus_again`; a list keeps the focus it was opened from).

### Word (`8eaa4166`)

Right, Left and the word keys onto another page say the page first, "page 2 | Borrowing",
"page 1" at a line's end [obs, 1119/s4-s6]; a character after it is still read as one.

## Harness

- **Excel on old-pc** runs one instance with a keeper workbook (Book1, "lab" in A1) and its
  Activation Wizard closed; the fixture opens into it. Recorded rounds: 1114, 1114c, 1116b,
  521i, 1118b, 1119.
- **Recorded again for their snapshots:** 1114 (round-1114c), 521 and 1116 (round-521i), 1118
  (round-1118b), after adding `"trace_headings": true` to them.

## Found, and how

- **An empty Excel cell has no UI Automation element** (1118's trace: the focus is the grid
  window, a "tab list" named after the workbook).
- **A list dialog's closing quiets focus events for 800 ms** (`returning_from_dialog`): OK's
  move to a cell was silent until that quiet was cleared for it.
- **The gate judges the whole command**: an `sed` beside a blocked commit did not run either.
- **Insert+Shift+S** opens Excel's sheet tabs' menu (`MsoCommandBarPopup` "Ply"); JAWS reads it
  "Context Menu | Ply pop-up menu | Insert... | To move through items press up or down arrow."
- **Selections** leave the focus on the active cell; JAWS reads the selection from Excel:
  "select | Weekdays | B 1" for one cell added, "select | A 1 | Branch | B 2 | 12" for a range,
  "selected entire row", "Selected entire column" (1117). One sample each: more needed.
- **A figure typed before its tool ran:** `d56204cc`'s message was written in the command
  that ran parity; the figure it gives is the tool's (9567/10139, 284), checked after.

## Proof

`okur-lab parity` at `b2f2c2c5`:

```
JAWS spoken parity: 9455/10210 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 9577/10149 steps match exactly; 284 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `8fc9c9e4`: 9521/10101. `okur-lab parity --only` for this stretch's
scenarios, each line as printed:

```
521: JAWS parity (Ia2): 14/14 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1114: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1115: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1116: JAWS parity (Ia2): 7/8 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1117: JAWS parity (Ia2): 1/5 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1118: JAWS parity (Ia2): 10/10 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1112: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1113: JAWS parity (Ia2): 8/8 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1119: JAWS parity (Ia2): 6/7 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace --no-fail-fast` (4 test threads) at `b2f2c2c5`: 1560 passed, 0
failed, 200 ignored (summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer
invariants: 0 violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The
manifest's statistics: 743 scenarios, 2235 trials, 2218 succeeded.

What differs, and why it is open (plan 41):
- 1117: selections, one sample each of JAWS's words.
- 1116/s5: the sheet tabs' menu, which Okur neither opens nor names as JAWS does.
- 1119/s3: JAWS says the letter reached on the new page again, by its phonetic name ("page 2
  | B | bravo" on the synthesizer).

Not checked live: any of this in Okur's own run on the dev box (it needs the owner's leave).

## old-pc

- Installed: push `3545169f` (the empty cell build). JAWS's voice profile read `Rate=90` in
  every line earlier today. Excel: the keeper Book1 open, its wizard closed. Word: the keeper
  Document2 open.
- Idle after round-1118b.

## Next

Plan 41: Excel's selections and sheet tabs' menu, Outlook, the settings' long tail, Word's
open items, the review's open findings.
