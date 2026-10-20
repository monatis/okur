# Handoff: everyday-use gaps, round 2 (2026-10-20)

Follows [`2026-10-19-everyday-gaps-round-1.md`](2026-10-19-everyday-gaps-round-1.md). The work
list is still [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md),
its rows updated as features landed; the next plan is
[`docs/plans/2026-10-21-everyday-gaps-round-3.md`](../plans/2026-10-21-everyday-gaps-round-3.md).

## Decided without the owner (the agent's lab, reversible)

- **The browser profile's name in the corpus** (round 1's first question): the web fixtures now
  open in a Chrome of the lab's own, signed out, under `sr-harness\chrome-profile`
  (`lab-chrome.ps1`); browsers of any other profile are closed before a fixture starts. Its
  titles end in "- Google Chrome". The held recordings (530, 531, 534-539) were recorded again
  and imported, and every web scenario whose traces held the name is being recorded again
  (rounds 49-57). History is not rewritten.
- **The tracer runs elevated** on old-pc, as JAWS does, so it sees JAWS's own dialogs whole
  (list items, buttons by role): replay of the element lists went from comparing Okur with
  half a dialog to comparing it with the dialog.

## What was done

### Features (each checked by `okur parity` against old-pc's evidence)

- **Data grids** (`1a85115`): a WinForms DataGridView's cell with "selected" and its text;
  Ctrl+Home and Ctrl+End add "top of file" and "Bottom of file" and the cell again [545].
  JAWS's "Column 65535, Row 65535" is an accepted divergence.
- **Help keys** (`8ba12c3`, `f2f0d32`): Insert+F1, Insert+H, Insert+W open help in the Virtual
  Viewer as JAWS shows its own, with the hot keys link and "Press ESCAPE to close this
  message."; the words are Okur's own (help.ftl), Insert+H lists Okur's keys as links Enter
  uses; in a page the help is about the element under the virtual cursor (the page's summary,
  a link's address, a heading's level). Escape reads the focus as JAWS does [526, 527, 531].
- **Menus** (`d46fcfb`, `e540761`, `30dc742`): Alt and a letter says the menu, not the bar;
  "leaving menu bar" for a window's own bar; a menu item that opens a dialog says "Leaving
  menus | edit | Type in text." then the dialog [541, 544, 547].
- **Notepad's edit** (`e540761`): "edit | Type in text." with no name, no line [540-547].
- **The table layer** (`2b9dec3`, `f69b4d2`): Insert+Space, T and its keys; rows and columns
  read whole with Windows+Alt; Ctrl+Alt+Numpad5 [297, 535].
- **Quick keys** (`3bb2922`, `311c4bf`): first and last heading, heading of a level and form
  field; separators, divisions, mail links ("Send Mail Link") [292, 293, 295, 296].
- **Clicks and regions** (`82c03b5`): Numpad Slash's focus names the regions entered [534].
- **Text areas** (`590d61d`): read line by line in the virtual cursor, " Edit | line" [508].

### Harness

- **Lab Chrome** (`ee52370`) and **elevated tracer** (`d8f920d`), above.
- **Reloads in replay** (`2f6ffe3`): Ctrl+R and F5 deliver the page again, as Okur's page
  worker does live, so the page-load summary replays (507, 538, 539).
- **Okur's own dialogs in replay** (`8c1e8b7`): replay gives the engine a window host, and
  JAWS's dialog in the trace stands for Okur's window (`dialog_mirror.rs`); events of JAWS's
  hidden main window are left out. The old headless emulation of three list dialogs is gone.
- **Converter** (`30dc742`, `8c1e8b7`): a dialog around the focus is the window; a menu bar's
  focus as the menus end gives the focus back to the window; a dialog's text that names one of
  its controls is that control's label.

### Recorded and imported

- 544, 545 (round 46); 537 and the default-button re-records 532, 533, 540, 542, 543 (round
  47); 530, 531, 534, 535, 536, 538, 539 in the lab's Chrome (round 48); 547 and 207, 209,
  220, 317, 544 again (round 49); 101-114 again (round 50).

### Accepted divergences added

- "Column 65535, Row 65535" after a data grid's cell (JAWS's place for a cell it cannot
  place).
- The help keys' texts (Freedom Scientific's writing; Okur's are its own).
- "There are N Smart Glance Highlights on this page." after the page-load summary (Okur has no
  Smart Glance).

## Numbers

At round 1's end (`09f5cc4`) `corpus/parity-baseline.json` held `"matched_steps": 2038` of
`"total_steps": 2547` (`git show 09f5cc4:corpus/parity-baseline.json`). `cargo run -p okur --
parity` at `33c84ff`:

```
JAWS spoken parity: 2286/2603 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 2388/2655 steps match exactly; 60 accepted divergences not counted (0 scenarios could not run)
```

The largest steps: the elevated tracer with Okur's own dialogs replayed through JAWS's
(2181 -> 2228 on the old traces, then 2236 -> 2388 with round 52's re-records of 221-245), the
table layer (+17), the help keys and Escape after them (+7), the quick keys (+18). Every
change was checked to leave no step worse, but for 220/s7 (its re-record), 228/s11-s12 (old
trace, fixed by round 52) and the five steps the mirror moved before round 52 came in.

## old-pc when this was written

- Round 53 (548-555, the verbosity levels) **failed**: every scenario exited 1 within seconds
  of the JAWS restart its settings need (555, with no restart, in 4 s), and no evidence was
  written. The cause is not known yet: run 548 by hand once live11 is done (one trial, its
  output shown), fix, then queue 548-555 again.
- live11 (Okur reading 222-260 and 274, live) was running; then rounds 54 (246-274 again, with
  an okur.exe whose tracer records dialogs' group boxes), 55 (275-299, 19, 20, 29), 56 (the 3xx
  desktop scenarios, and 524, 525, 532, 537, 540-546 for the group boxes) and 57 (401-408,
  5xx) are queued in that order.
- 237 did not run in round 52; add it to a later round.

## Not done, and why

- Toasts (533): neither the tracer nor Okur sees them on old-pc.
- S and D, step past an element, Jump to Line, words from a list (290, 294, 299).
- Say All stopped by a timed key (500, 538, 539): replay has no speech clock.
- Office on old-pc is still not activated (518, 519, 521).
