# Handoff: Excel's titles from names, its totals readers; replay answers reads in their step (2026-11-01, ninth)

Follows [`2026-11-01-excel-lists-format-and-store-identity.md`](2026-11-01-excel-lists-format-and-store-identity.md)
and covers `9d9e09f3` to this one, made under
[`../plans/2026-11-01-everyday-gaps-round-42.md`](../plans/2026-11-01-everyday-gaps-round-42.md).

Where things stand: Excel's titles and totals are in, 1132 and 1134 matching on every step.
The Store release waits for the owner (unchanged from the last handoff).

## What was built

- **Titles from the workbook's names** (`db56104e`, 1134): a workbook whose name "Title" sits on
  A1 has JAWS say the titles with no key pressed; Excel's place now carries the titles its names
  give the sheet ("Title", "ColumnTitle", "RowTitle", maybe with a sheet number or region, as
  JAWS's help describes; a region's bounds are not kept yet). A workbook's titles not known yet
  are learnt with its first move's place, and kept for the session. A new fixture,
  `desktop-excel-titles.ps1`, is the same table with that name.
- **The totals readers** (`23054223`, 1132/s11, s12): Insert+NumPad Enter, "column total=14",
  from the column's SUM formula; Insert+Delete, "no totals column defined and I cannot find a
  sum formula in the current row." The sheet's on-demand read ("sheet details") brings the
  first SUM formula's text of the active column and row with the cell's format.
- **A cell re-reported as Excel's readers read the sheet** is not said again (`cb3265e5`): Okur's
  commands that only ask Excel for a reading count as no key for the application, and a cell
  that waited for its titles is the last focus.
- **Replay** (`db56104e`): reads a step's events make are answered in that step even where the
  recording saved no snapshot, and the setup's reads within the setup; the cell waiting for its
  titles had said the step before's in the older Excel recordings.

## Found, and how

- JAWS 2025 honors a workbook's named "Title" (round-1134), as its help says.
- JAWS's totals readers work without a totals row set, from SUM formulas; its Set Totals Row
  fails (round-1132).

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 11119/12015 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11377/11949 steps match exactly; 365 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `9d9e09f3`: 11368/11942. `okur-lab parity --only`:

```
1132: JAWS parity (Ia2): 13/13 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1134: JAWS parity (Ia2): 7/7 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace --no-fail-fast` (4 test threads): 1587 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 1913 scenarios, 5745 trials, 5728 succeeded.

Open: titles kept across sessions (JAWS keeps them per file); a named region's bounds; monitor
cells and comments; from before, 1125/s3, 1062/s10-s13, 1129/s11, 1130/s11, 2857/s1.

Not checked live on the dev box.

## Waiting for the owner

Unchanged: a test certificate trusted on old-pc for the Store package's experiments; Defender's
flag; the privacy policy's URL and whether the listing may compare Okur with JAWS.

## old-pc

Idle after round-1132c; JAWS's settings as before every round.

## Next

Plan 42: Excel's monitor cells and comments (popapps.chm), titles kept across sessions; the
Store's experiments once the owner trusts a test certificate on old-pc.
