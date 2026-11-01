# Handoff: Excel's lists and Insert+F; the Store package's identity, logo and privacy policy (2026-11-01, eighth)

Follows [`2026-11-01-excel-titles-and-store-package.md`](2026-11-01-excel-titles-and-store-package.md)
and covers `e73e95cd` to this one, made under
[`../plans/2026-11-01-everyday-gaps-round-42.md`](../plans/2026-11-01-everyday-gaps-round-42.md).

Where things stand: Excel's information keys of 1133 all match (9 of 9); the Store package
carries Partner Center's identity and Okur's logo, and a privacy policy exists. The release's
experiments still wait for the owner's leave to trust a test certificate on old-pc.

## What was built

### Excel

- **Insert+Shift+R** lists the row's cells, "Row 1 dialog | List1 List view | Branch: A1 | 1 of
  3" (`e73e95cd`).
- **Insert+F** says the cell's font and formatting (`62b62cad`): "Cell Font and Formatting: |
  text color Black | background color White | Calibri | Regular | 11 point | highlighted |
  Horizontal alignment is General. | Vertical alignment is Bottom. | Number format is general. |
  Normal | Cell dimensions are 48 points wide by 14 points high." From Excel's object model,
  read with the sheet's place ("sheet format"); old-pc's keeper gave Calibri, Regular, 11,
  black, no fill, 48 by 14.4 points. Colors come from the color table, capitalized as JAWS says
  them here; alignments other than General and Bottom are named as Excel names them [hyp].
- **Ctrl+Shift+D** lists the window's visible cells with data, row by row, "B1: Weekdays", the
  active cell's entry selected, "2 of 12" (`d4d01593`), from Excel's `VisibleRange`.
- The lab's snapshot saves the row's list, the visible cells and the cell's format beside the
  rest, and replay answers them. 1133 was recorded four more times as each came in; round-1133f
  gave Ctrl+Shift+D 2.5 s to start speaking (JAWS's list opens 1.25 s after the key, past the
  recorder's default onset, and its speech had been lost).
- Accepted: what JAWS says before these lists, its version and window or the cell with a list's
  hint, by the trial (`918d738b`).

### The Store package (`44177ca5`, `918d738b`)

- **Identity** from Partner Center (the owner, 2026-11-01): `Monatis.okur`, publisher `Monatis`
  (`CN=4A33A3E2-13C6-458B-B8DF-B58C4CA3CA30`); package family name
  `Monatis.okur_rfy6yt361pz56`, Store ID 9NV0FM5H54X8. The build's identity now; the test one is
  gone.
- **Okur's mark**, asked for minimal, modern and elegant: a white letter O with two amber arcs
  spreading from its right like sound, on a deep navy rounded square
  (`packaging/msix/okur-logo.svg`; `draw-logo.ps1` draws it at the package's sizes and the
  listing's 300 and 1080 pixels into `target/msix/listing/`). A first version with the O open on
  its right read as a C, and was closed.
- **`PRIVACY.md`**: Okur collects nothing and makes no network connections (none of its programs
  uses a networking crate or call). The listing's privacy policy URL can be
  https://github.com/monatis/okur/blob/main/PRIVACY.md once pushed.
- Built: `target/msix/okur-1.0.0.0-x64.msix`, "Package creation succeeded."; its manifest's
  `Identity` is `Monatis.okur`, `CN=4A33A3E2-13C6-458B-B8DF-B58C4CA3CA30`, 1.0.0.0, x64.

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 11112/12008 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11368/11942 steps match exactly; 365 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `9f24e018`: 11359/11933. `okur-lab parity --only 1133`: "JAWS parity (Ia2):
9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)".

`cargo test --workspace --no-fail-fast` (4 test threads): 1585 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 1912 scenarios, 5742 trials, 5725 succeeded.

Open:
- 1132/s11, s12: the totals readers, which find a SUM formula in the active column or row
  ("column total=14").
- Titles kept across sessions; titles from Excel's defined names (ColumnTitle, RowTitle, Title).
- From before: 1125/s3, 1062/s10-s13, 1129/s11, 1130/s11, 2857/s1.

Not checked live on the dev box.

## Waiting for the owner

For the Store release:
1. **A test certificate trusted on old-pc** (a machine-wide trust change, removed afterwards), to
   install the package there for experiments E1-E7: UI Access in a package, the package without
   it reading Chrome, Word and Excel, the startup task, its processes and files, the
   certification kit. Okur's packaged behavior (the startup task in place of its own start
   options) waits for it.
2. **Defender's flag** on Okur's binaries, which the Store's malware scan meets first.
3. **The privacy policy's URL** (above) and whether the listing may compare Okur with JAWS.

## old-pc

Idle after round-1133f. JAWS's settings as before every round (Rate=90; the recorder removes
Excel's settings files).

## Next

Plan 42: the totals readers; titles from defined names (a fixture of its own); the Store's
experiments once the owner trusts a test certificate on old-pc.
