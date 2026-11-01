# Handoff: Excel's titles; the Okur cursor from screen text; the Store's research and package (2026-11-01, seventh)

Follows [`2026-11-01-excel-editing-dialogs-and-okur-words.md`](2026-11-01-excel-editing-dialogs-and-okur-words.md)
and covers `a78afe39` to this one, made under
[`../plans/2026-11-01-everyday-gaps-round-42.md`](../plans/2026-11-01-everyday-gaps-round-42.md).

Where things stand: Excel's row and column titles are in (1132), the recorder can start each
trial without JAWS's own settings for a file, the Store release is researched and its package
builds on the dev box. The release's next steps wait for the owner (below).

## What was built

### Excel's titles (`942b87b3`, 1132)

- Insert+Ctrl+Alt+C sets the active cell's row as the column titles, "Setting column titles row
  range to 1"; Insert+Ctrl+Alt+R its column as the row titles, "Setting row titles column range
  to A".
- A cell entering another column says that column's title first, "Weekdays | 12 | B 2"; one
  entering another row the row's, "Northside | 10 | B 3"; a title cell no title of its own,
  "Central | A 2". The cell waits for Excel's place (the new module hook `sheet_place`), and is
  said alone where the place cannot be read.
- The Okur key with Alt+Shift and C or R says the titles, "Weekends", "Northside". On old-pc,
  whose JAWS uses the laptop layout, Excel's key map binds them to CapsLock only, so 1132 sends
  CapsLock+Alt+Shift.
- The titles are the engine's, per workbook window, for the session: kept across sessions, as
  JAWS keeps them per file, is open. Excel's defined names (ColumnTitle, RowTitle, Title) are
  open too.

### The recorder (`a78afe39`)

- **`jaws_forget`**: files under JAWS's user settings folder that each trial starts without,
  and the scenario leaves none of, removed with JAWS stopped and started again. JAWS keeps a
  workbook's titles in its personalized settings for the file, which would reach every later
  trial and every later Excel recording of the fixture.

### The Okur cursor from screen text (`12c2bac6`)

- With the window's screen text, which the live reader always has, Route Okur to PC put the
  cursor on Notepad's menu bar line: the field's middle, where the PC cursor is, lies below its
  only line. The route now finds the screen line showing the caret's line and the caret's
  character on it. The same text at the same place, which UI Automation gave twice for classic
  Notepad's line, is one run. round-1062c (not imported) then matches s2-s9.

### The Store release (`cd608d9b`, `3991db06`)

- [`../references/microsoft-store-release-research.md`](../references/microsoft-store-release-research.md),
  with sources: a full-trust MSIX works for a screen reader without UI Access; UI Access in a
  package is documented (the `uiAccess` restricted capability; NVDA's 2019 Store package) but
  reported broken since 2021, so it is experiment E1 on old-pc and Microsoft's approval; the
  sign-in screen and Ease of Access stay outside the Store build. Against the plan: the Store's
  certification starts with a malware scan, so Defender's `Bearfoos.A!ml` must be solved first;
  `0.1.0` is no valid package version; the first submission must be a Private audience.
- `packaging/msix/build-msix.ps1` and `AppxManifest.xml.in`: an unsigned full-trust package of
  the reader and `okur-ui.exe`, a startup task after sign-in, a test identity unless Partner
  Center's is given, placeholder logos, `-UiAccess` for E1's variant. Built on the dev box:
  `target/msix/okur-1.0.0.0-x64.msix`, MakeAppx's "Package creation succeeded." Not installed,
  not signed, not run through the certification kit.

## Found, and how

- **JAWS 2025's Set Totals Row** (Ctrl+Insert+Delete) answers "unknown function call to:
  settotalsrowtocurrent", spelling the name out, in every trial of round-1132: left out of 1132.
  Its readers work without it: Insert+NumPad Enter says "column total=14" from the column's SUM
  formula, Insert+Delete "no totals column defined and I cannot find a sum formula in the current
  row."
- **Plain Alt+Shift chords** may switch old-pc's keyboard layout (en-US holds Turkish Q and US):
  scenarios avoid them for now (Excel's comment and gridline keys are Alt+Shift chords).
- **Round-1133** (Excel's information keys): Insert+Shift+R lists the row's cells, "Row 1 dialog
  | List1 List view | Branch: A1 | 1 of 3"; Insert+F says the cell's font and formatting at
  length ("Cell Font and Formatting: | text color Black | background color White | Calibri |
  Regular | 11 point | ..."), longer than the recorder's 10 s settle, so every trial failed;
  recorded again with 30 s for it (round-1133b). Ctrl+PageDown, "Hours change in April. | A 1".

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 11105/11999 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11359/11933 steps match exactly; 365 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `eef1d0e4`: 11348/11920. `okur-lab parity --only 1132`: "JAWS parity (Ia2):
11/13 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)".

`cargo test --workspace --no-fail-fast` (4 test threads): 1582 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 1911 scenarios, 5739 trials, 5722 succeeded.

Open:
- 1132/s11, s12: the totals readers (they need the column's and row's formulas in Excel's
  snapshot).
- Titles kept across sessions; from Excel's defined names.
- From before: 1125/s3, 1062/s10-s13, 1129/s11, 1130/s11, 2857/s1.

Not checked live on the dev box.

## Waiting for the owner

For the Store release (plan 42 §1):
1. **A test certificate trusted on old-pc** (a machine-wide trust change), to install the
   package there for experiments E1-E7: UI Access in a package, the package without it reading
   Chrome, Word and Excel, the startup task, the package's processes and files, the
   certification kit.
2. **Partner Center's identity values** (Name, Publisher, PublisherDisplayName from the Product
   identity page) and the version to give the first package.
3. **Defender's flag** on Okur's binaries, which the Store's malware scan would meet first.
4. **A privacy policy URL** (Store policy 10.5.1) and **a logo**.
5. Whether the listing may compare Okur with JAWS (the research: "JAWS" cannot be a search term).

From before: the containers a quick key enters; a field's selected text on a focus; Outlook's
account on old-pc.

## old-pc

Recording round-1133b (Excel's information keys, Insert+F given 30 s). Its JAWS settings are as
before every round (Rate=90; no Excel settings files, the recorder removes them).

## Next

Plan 42: import 1133b; Excel's row list (Insert+Shift+R: a list kind, Excel's object model, the
lab's snapshot, then a recording with the new lab binary), Insert+F, the totals readers; the
Store's experiments once the owner trusts a test certificate on old-pc.
