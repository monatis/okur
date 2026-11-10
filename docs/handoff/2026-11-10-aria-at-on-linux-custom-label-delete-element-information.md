# Handoff 2026-11-10 (day): ARIA-AT on Linux, switches, Custom Label Delete, the Window List's Delete, element information, a text area's lines

After [`2026-11-10-offline-linux-replay-fixes-and-run-okur-manager.md`](2026-11-10-offline-linux-replay-fixes-and-run-okur-manager.md).

**Plan:** [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md), with the
everyday gaps' items 29 and 41 ([`../backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md)).

## Lab state

- old-pc came back at about 07:10 (old-pc time). `night2` had ended (exit 0).
- **`night3`** (at 6cf0846f): `LINUX CHECK PASS`, the first lab build of the GTK code of
  af369c04 to 6dcca2f6. `main` moved to 6cf0846f, the commit checked; the commits after it
  (56b18fcb's AT-SPI line read, f9ca67db) are not built in the lab yet. Then the wrapped text
  area's AT-SPI snapshot, the two probes, and 122 runs: the dialog and wizard scenarios, 1207,
  1211 and ARIA-AT's 94 switch scenarios. It was running at this handoff.
- **Round 1215** waits for `night3` to end (`run-batch.ps1` shuts WSL down): 1208 again with the
  new tracer, its snapshot in the same round, 1213 and 1214, with
  `close-explorer-windows.ps1` before it. Its files are in `C:\Users\ailabs\sr-harness\round-1215-in`.
- The JAWS voice profile read Rate=20 (saved 2026-11-07 13:18, after the last round);
  `fix-rate.ps1` put it back at 90 before round 1207.
- The Excel keeper window was closed for round 1207 (the Window List's S could reach it); an
  Excel round starts it again.

## What was done

**night2's ARIA-AT runs on Linux** (5ad8f458): 1154 runs and traces, checked for anything but
fixture content. `okur-lab live-baseline linux --write`: "8203/9558 steps match JAWS exactly;
242 accepted divergences not counted (1666 runs, 0 could not be compared)"; `okur-lab
live-replay linux --write`: "9442/9800 steps say what the run said; replayed, 8442/9558 match
JAWS exactly". Of the Linux-only gaps (steps Windows' parity matches, the Linux replay does
not; 831 in all from the two baselines), 330 are ARIA-AT's: switches, sliders and the menu
button first.

**An ARIA switch on Linux** (3df88e1b): Chrome's AT-SPI gives `role="switch"` as a toggle
button with `xml-roles` "switch", its on state PRESSED; Okur said "Toggle button Not pressed"
for JAWS's "Switch Off". It is a check box with the switch's role now. And a page's control
checked or expanded while the focus is elsewhere (ARIA-AT's setup script turning a switch on)
is read again and spliced in by its id, as a live region is (okur-linux, built in night3).
`okur-lab live-replay linux`: "replayed, 8502/9558 match JAWS exactly"; `--api atspi`:
"12413/13284".

**Round 1207 and the rounds after it** (rounds 1207, 1209b, 1210b, 1212; each 3 trials,
capture script 60f87ebf):
- **Custom Label Delete** (1207, 1211; ca10b3a6): "Delete Custom Label? dialog | lamp | OK
  Button", OK and Cancel; with no label or on text, "There is no Custom Label to delete for
  this item." Built from Run Okur Manager, with Delete All's question as the same dialog.
  Run Okur Manager's return is always quiet now, and a focus in one of Okur's own dialogs is
  never taken in quietly (`in_own_dialog`): JAWS says no page between its list and the question.
- **The Window List's Delete** (1209): the window closes and the list stays, the item now in
  its place said ("Windows PowerShell Minimized | 4 of 5"; JAWS says it twice, an accepted
  divergence). Round 1209b's trace held the reference machine's desktop files (the focus came
  back to an Explorer window on the Desktop folder): not committed, recorded again in round
  1212 with Explorer's windows closed and a lab Notepad behind. The tracer now leaves out the
  files beside an Explorer focus (`privacy::without_files_beside`), and
  `close-explorer-windows.ps1` closes them before a batch.
- **Element information** (1210; f9ca67db): Insert+Shift+F1 and Insert+Ctrl+Shift+F1, JAWS's
  DisplayBasicElementInfo and DisplayAdvancedElementInfo: the element at the virtual cursor and
  each element above it to the document, their IA2 object attributes with the address and the
  ARIA role, their IA2 role in hexadecimal, in a message of the Virtual Viewer. Read live
  through IA2 one object at a time (`chromium_object_chain`), answered in replay from the
  page's IA2 snapshot. 1210: 21/21 steps.
- **A wrapped text area** (1208, not imported): JAWS reads its lines as shown, "The reading
  room opens at", then "nine on weekdays.". 56b18fcb gives `TextContext` the field's own line
  (IA2's line boundary for Chromium, UIA's line unit elsewhere, AT-SPI's on Linux), which
  `line_at` takes. 1208's trace had no line and a caret read too early ("caret: 0" after Down,
  where `okur-lab caret` live on old-pc read 26), so the tracer reads a Chromium caret event
  again once it settles; 1208 is recorded again in round 1215.
- Imported: 1207 (64363f3c), 1209 to 1211 (a2c135b9; its message names 1208 by mistake,
  corrected in 6cf0846f).

**Smaller:**
- 45 line continuations in 13 accepted divergences' reasons, collapsed by bash heredocs in
  earlier sessions into runs of spaces, restored (6cf0846f).
- Debug lines: a focus taken in quietly; a focus in okur-ui in none of its open dialogs; the
  replay mirror's choice of Okur's dialog.

## Figures

From the commands named, after the commits given:
- `okur-lab parity` after f9ca67db: "12779/13373 steps match exactly; 392 accepted divergences
  not counted".
- `okur-lab parity --api atspi` after a2c135b9: "12483/13373 steps match exactly; 392 accepted
  divergences not counted".
- `okur-lab live-replay linux` after f9ca67db: "9371/9800 steps say what the run said;
  replayed, 8503/9558 match JAWS exactly (1666 runs replayed, 0 without a trace, 0 could not be
  replayed)".

## Found, not fixed

- **night3's probes:** the lab's Chrome marks misspellings ("teh" `invalid: spelling` 3 s after
  typing; the en-US dictionary is in the profile), but not the word still being typed ("fxo"
  unmarked at 15 s). GTK 4 marks a no-selection column view's rows SELECTABLE and never
  SELECTED, so Okur would say "Not Selected" on each; the probe's Tabs lagged one list behind,
  so it is to be run again before a fix.
- **1209/s4:** File Explorer's item count on the desktop, "1 of 22" in the trace where JAWS says
  "1 of 33" (round 1209b, not committed): UIA's count of the items it holds, or another count.
- **Element information on Linux:** AT-SPI gives the same attributes and no MSAA role; not built.
- **1208's snapshot in round 1210b** was taken in another Chrome session than its trials, and
  the replay's focus landed on the Save button: snapshots and trials in one round.

## The afternoon (after 99daf8de)

**night3's runs** (76ae8de5): 122 runs at 6cf0846f. `okur-lab live-baseline linux --write
--allow-regression`: "8444/9623 steps match JAWS exactly; 245 accepted divergences not counted
(1668 runs, 0 could not be compared)". Let go, each with its cause in the commit:
- 256/s1, s2: the GTK first-focus race. okur-ui's log says GTK had the focus on the tree's first
  row as the dialog came ("dialog 1 shown: active true, focus Some(GtkTreeExpander)"), and
  AT-SPI told it only after the next key (in 241, the same dialog, within 100 ms).
- 304/s6, s7: Right and Left on the desktop replica's radios went nowhere since c45c97d5 stacked
  them; the replica's group now takes every arrow round itself, as a Win32 group does.
- 742/s24: Down on okur-ui's drop-down's last choice moved the focus to Back.

**Linux** (7a4e26e8): a page's slider says its value (Chrome's `valuetext`, "Red left right
slider 128"); a control read again with no name, after one that had a name, says nothing
(742/s9's stray hint); okur-ui's GTK drop-down takes Up, Down, Left, Right, Home and End as a
Win32 combo box does, stopping at its ends.

**The third review's fixes** (9c3ff12c, one reviewer over 3df88e1b to f9ca67db): a field's line
from the platform only where it holds the caret, with no break inside it; IA2's line read on
S_OK alone; Word's line not read twice; the Explorer privacy rule over a focused list's items;
Custom Label Delete by the label's key; Linux state reads limited to 64 objects, and changes
made while the focus is away noted; element information's replay stops at the first document.
Kept with reasons: `in_own_dialog`'s reach, late element-information answers, the tracer's
40 ms wait.

**night4** (at 9c3ff12c): `LINUX CHECK PASS` (the drop-down arrows, the AT-SPI line read and
the state limits built at last); `main` moved to 9c3ff12c. Its runs (256, 304, 742, the
dialog scenarios, ARIA-AT's sliders, menu buttons and ratings: 257 in all) were running at this
update. The GTK selection probe, each list alone: a no-selection list's rows are SELECTABLE and
never SELECTED, so "Not Selected" would be said on each; the list's own states are the next
read to tell it apart.

**Round 1215** (b9a0dfcc, 0012b223): 1208 recorded again with the line in its trace, and 1213.
Element information's message has a blank line after each element's block too. 1214 (Advanced
on a field) stays pending: it lists the field's HTML attributes (id, name, type) and JAWS's own
"fsFormField=true", which Okur does not read, and one trial recorded no s1.

Figures:
- `okur-lab parity` after 0012b223: "12817/13414 steps match exactly; 392 accepted
  divergences not counted".
- `okur-lab parity --api atspi` after b9a0dfcc: "12548/13414 steps match exactly".
- `okur-lab live-replay linux` after 9c3ff12c: "9555/9868 steps say what the run said;
  replayed, 8701/9623 match JAWS exactly".

## The evening (after 1debf565)

- **Linux, from the Linux-only gaps** (831 in the morning, 610 after c4f2420b, ARIA-AT's 330 to
  176): a page menu's items say their place and the menu its name as it opens (c4f2420b); the
  focused control's value change is heard and told (1c9f0987), in its page and as its value
  text (e9dd01fc); element information on Linux (96dbcd5e); AT-SPI snapshots of ARIA-AT's
  pages read after their "Run Test Setup" (77e78985); Okur's own dialog's first focus looked
  for where GTK tells none (0a37c9bb): the engine asks once 250 ms pass after okur-ui said the
  dialog shown, and the driver finds okur-ui's window by title and its focus.
- **The fourth review** (one reviewer over 7a4e26e8 to 1c9f0987), built in e9dd01fc: the
  slider's change in its page (it said nothing live), its value text, a "\r\n" line not taken
  for a break inside it, no places for a menu bar's menus, a value change with a name emptied
  still said.
- **Ctrl+Home or Ctrl+End that moves nothing says nothing** [obs, 1208/s2] (e9dd01fc), a
  selection taken away counting as a move (660/s11).
- Not built yet in the lab: 96dbcd5e, 77e78985, 1c9f0987's and e9dd01fc's driver parts,
  0a37c9bb's. night5 starts with `check-linux` and stops if it fails.

Figures:
- `okur-lab parity` after e9dd01fc: "12818/13414 steps match exactly; 392 accepted
  divergences not counted".
- `okur-lab parity --api atspi` after e9dd01fc: "12549/13414 steps match exactly".
- `okur-lab live-replay linux` after 0a37c9bb: "9519/9868 steps say what the run said;
  replayed, 8729/9623 match JAWS exactly".

## Paused (the owner, 2026-11-10, evening)

The owner paused Okur's development to work on other repositories. The state it was left in:
- **Code:** every commit is on `windows-ui-round-2`, and `main` was fast-forwarded to it
  after the lab's `check-linux` passed at a072d0e8 (`LINUX CHECK PASS`: every Linux-only
  part of the day, element information, the slider's value change, the setup capture and the
  dialogs' first focus, built, linted and tested there). Nothing is pushed: the owner pushes.
- **The lab (old-pc):** no job runs; the WSL distribution "okur" is stopped. `night4` was
  stopped at 165 of its 257 runs (at 9c3ff12c); its runs are in `D:\okur-linux\runs`, not
  imported (fetch them with `fetch-runs.sh` and their ids from `night4-ids.txt` before the
  next run overwrites them). `night5`'s lists are pushed (`night5-ids.txt`,
  `aria-fixtures.txt`) and never ran. The JAWS voice profile is at Rate=90. The Excel keeper
  window is closed. Pending 1214 is in `corpus/pending`.
- **To resume:** read this handoff, then start with "Next" below; the first job is night4's
  import, then night5.

## Next

1. night4's runs: import them (privacy check, baselines apart from code); 256, 304 and 742
   should come back.
2. night5 (lists pushed: `D:\okur-linux\night5-ids.txt`, 180 runs: the ARIA-AT scenarios
   with Linux-only gaps, 256, 304, 742, the replica's 548, 549, 554, 1208, 1210, 1213;
   `aria-fixtures.txt`, 224 pages): `check-linux`, then the AT-SPI snapshots of ARIA-AT's
   pages with their setup, then the runs. Then `parity --api atspi` with the new snapshots.
3. 1208's End onto a line break ("blank" in Chrome's text area, empty in Word's and Notepad's
   history): one recording more before a rule.
4. Element information's Advanced: the HTML attributes JAWS lists (1214, recorded again).
5. The ARIA combo box's own button taken as the prior form field on Linux (2918/s1).
6. The no-selection GTK list's "Not Selected": read the list's own states.
