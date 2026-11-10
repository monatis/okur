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

## Next

1. When `night3` ends: import its runs (privacy check, baselines, apart from code); then round
   1215 (`lab-deploy.sh` first: the tracer of 56b18fcb).
2. 1208 with its new trace and both snapshots; 1213 and 1214's layout and Advanced.
3. `night4`: `check-linux` at HEAD (56b18fcb's AT-SPI line read, f9ca67db), the switch
   scenarios' Linux-only gaps again, sliders and the menu button next.
4. Element information on Linux; the GtkNoSelection probe again.
