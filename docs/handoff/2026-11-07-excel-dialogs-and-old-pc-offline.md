# Handoff 2026-11-07: Excel's JAWS dialogs, small catalog and list fixes, old-pc offline

After [`2026-11-06-key-labels-and-linux-actions.md`](2026-11-06-key-labels-and-linux-actions.md),
whose last sections ("Excel's entry keys and Say Current Focus", "Excel's worksheet lists,
borders, totals and hyperlinks") hold rounds 1185 to 1194. The plan is still
[`2026-11-03-review-round-2-before-cross-platform.md`](../plans/2026-11-03-review-round-2-before-cross-platform.md),
§3 (JAWS's features from the help), Excel's keys from `popapps.chm` (JAWS Commands for Excel).

## State of old-pc

old-pc went offline twice on 2026-11-07 (OpenDesk: "rendezvous error: target_offline"). The
owner brought it back once; round 1193 then started (03:16) and finished, and round 1194 started
before the second drop. Neither has been fetched:

- `C:\Users\ailabs\sr-harness\evidence-out\round-1193` (cell PlaceMarkers:
  `corpus/pending/1193-desktop-excel-cell-placemarkers.json`);
- `round-1194` (1191 cut at Insert+F7, and a page-break scenario since deleted: its keys in
  JAWS's dialog were plain keys, which JAWS's elevated dialogs refuse). Do not import 1194.

Queued next, not started: round 1195 (`Dropbox/sr-harness/round-1195-in`, its start script,
`start-1195.ps1`, beside it, with the hashes it checks: run it on old-pc through `opendesk exec`
as an encoded command): 1191 again with JAWS's dialogs walked
by `jaws_function` `TypeKey("Down")`, `TypeKey("Escape")`, `TypeKey("Enter")` (as 511/s5 does);
1195, the hyperlink hint's sequence (Down, Down, Up, Up, Down over A1 link, A2 link, A3 plain);
1196, the custom summary (Ctrl+Insert+Tab, Add with `TypeKey("Alt+A")` and `TypeString`, OK,
Ctrl+Insert+Shift+Tab).

## Built (each `okur-lab parity` from the commit's tree)

- **1188 imported** (`a1e22736`): Alt+Shift+B and the appearance viewer, 22/22.
  `JAWS parity (Ia2): 12651/13248 steps match exactly; 388 accepted divergences not counted`.
- **Totals set by hand** (`956e8c31`): Ctrl+Insert+Enter and Ctrl+Insert+Delete; JAWS's own
  script fails there, an accepted divergence ("Excel's totals setters finish their sentence").
- **A cell's hyperlink** (`15958f73`): "Docs | has hyperlink | A 2 | To activate press Enter.";
  the hint not twice in a row is a hypothesis 1195 tests (the other: Down only).
- **Replay of Okur's list boxes** (`b5192874`): JAWS names an unlabelled list box by its dialog
  in Select a System Tray Icon (511/s4) and not in Select Hyperlink (1191/s4), the same controls
  traced in both; the mirror now names it as Okur's own dialog does. Parity unchanged.
- **Virtual HTML Features' PlaceMarkers** opens the page's PlaceMarker List (`edf75b78`), a
  backlog item; the background store's own-files test waits as long as a loaded machine needs
  (it failed once under the gate).
- **The Virtual Viewer window's "Ready"** comes from the catalog (`adb41aa6`), one of M0's
  leftovers.

## Learned

- **JAWS's own dialogs refuse the recorder's keys** (they run elevated): Insert+F7's Select
  Hyperlink took none, so every later step heard only the recorder's own "Speech history
  cleared" and traced no focus. `run-scenario.ps1`'s header says so (`jaws_function`
  `TypeKey`/`TypeString`, or `key_ui_access`); a scenario that opens one of JAWS's dialogs
  walks it that way.
- **A long reading outlasts the 10 s settle**: Insert+F's twelve lines need
  `"settle_max_ms": 20000` on the step, or no trial succeeds.
- Excel's Office: `EXCEL.EXE` ProductName "Microsoft Office 2016", `Application.Name`
  "Microsoft Excel", ProductCode `{90160000-0011-0000-1000-0000000FF1CE}`; where JAWS's
  "Retail" in Ctrl+Insert+V comes from is still unknown (1184/s8).

## Next

1. When old-pc answers: fetch 1193, import it if its trials succeed and build cell PlaceMarkers;
   start round 1195; import 1191, 1195 and 1196 and build what they show.
2. Excel's remaining keys: Shift+F2 (a note's editing; it changes the workbook, so the fixture's
   close must not ask to save), Ctrl+Shift+O and Insert+Alt+C with a shape and a chart (a fixture
   with a drawing part), the opening announcement (open item), Ctrl+Insert+V's "Retail".
3. Then the README's Excel line and figures (`readme-follows-features`), and the plan's §3.2.
