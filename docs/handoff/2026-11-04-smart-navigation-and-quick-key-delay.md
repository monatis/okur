# Handoff: Smart Navigation, P6 and Navigation Quick Key Delay (2026-11-04)

Follows [`2026-11-04-after-the-outage.md`](2026-11-04-after-the-outage.md).
Plan: [`../plans/2026-11-03-review-round-2-before-cross-platform.md`](../plans/2026-11-03-review-round-2-before-cross-platform.md),
§2.1 (P6) and §3. Branch `windows-ui-round-2`; `main` moved with every commit.

old-pc's Windows Security Alert (the firewall asking about the lab's `opendesk.exe` since the
power outage) is still open. It is the owner's to answer. Until it is, recordings that start a
desktop fixture can catch it coming to the front (1142 did again, trials 2 and 3), and their
traces then hold it and the lab account's path: not importable.

## Found first-hand

- **Smart Navigation in a table** (956, recorded before): with its tables on, the arrows move by
  cell and say what Ctrl+Alt+arrows say ("Revenue | $18,000 | Column 2 of 4", "Q1 | Row 2 of
  4"); past the last column or row the cursor goes to "table end"; Left and Ctrl+Left there go
  back to the last cell with its place ("$8,800 | Column 4, Row 4"); Ctrl+Left in a cell stays on
  its word; T says the first cell's place again after it. Right on the page's last unit says it
  again. The other edges (the first row or column, a row that is not the last, Ctrl+Right at a
  cell's end) are not recorded: `corpus/pending/1143` asks.
- **Smart Navigation's lines** (984, 986, 987, 991, 992): with it on, the page has Screen
  Layout's lines (blank lines between some blocks, a fieldset's radio buttons on one line, a
  table row on one line). They need where each element is on the screen, which the snapshots do
  not keep: not built.
- **977's Ctrl+Right** stops a second time on a fieldset's legend, which Down never shows as a
  line of its own (990/s6, s7): JAWS's own buffer. An accepted divergence (s13 to s24).
- **The spoken layer's "Q 1"** (62 steps): the speech recorder voice's own word splitting shows
  through ("W3C" arrives as "W3 C" on Tab, "W 3 C" on U, "A1" only as a call of its own), so
  this layer cannot settle where JAWS splits letters from digits. Left as it is.
- **P6 live** (A/B, three runs each, `performance-review-2026-11-03.md`): a Windows Settings
  focus read 36.0 ms before, 30.7 ms after, at the median; its window stage 6 ms to 0. Insert+T
  live: `app window texts read texts=2`, then `Title is | System`.
- **Navigation Quick Key Delay** (research on old-pc, JAWS's files put back and checked): in
  Settings Center under Forms Mode > Auto Forms Mode Settings, unavailable unless Semi-Auto or
  Auto; Space steps .5, 1, 1.5, 2, 3, 4, 5 Seconds, then Never ("8 of 8"); saved as `[FormsMode]
  AutoFormsModeThreshold` in milliseconds (1500), -1 for Never. Escape out of Settings Center
  with a change asks "Do you want to save them?". Enter on a search result jumps to it in the
  tree, it does not save; Alt+A applies.
- **1144** (the delay at one second, Auto Forms Mode): E onto an edit says it, then again a
  second later as forms mode turns on, and X is typed in it; an X 300 ms after E is a quick
  key; a combo box the same, Down then picks its next value; Escape says "virtual PC Cursor"; a
  check box takes no forms mode.
- **The tracer reads what is under the mouse at a trial's start**: with the pointer over the
  Security Alert, 1144's first recording held its path field. The pointer was moved off it (a
  move, no click) and 1144 recorded again; the first recording was not imported.

## Built

| Commit | What |
|---|---|
| `034c8c16` | Smart Navigation's tables: the arrows by cell, the word keys in the cell, "table end", the last unit again |
| `72267a06` | P6: a Store app's window texts read when Insert+T asks (live), with each focus only in a trace |
| `29559918` | 977's legend stops an accepted divergence |
| `8ddcd6b3` | Harness: no key press or character while a system prompt (rundll32, a credential prompt) is in front; the trial fails with a warning |
| `9c619f12` | Review: P6 measured live |
| `bfada6d3` | The setting `quick_key_delay_ms`, as Settings Center has it |
| `f8eaac7e` | corpus: 1144 |
| `ab467bdd` | Forms mode after the delay; replay asks the engine's timers before each recorded event |
| `aa3e1dd1` | Harness: a live Okur run reads old-pc's `DEFAULT.JCF` and the scenario's `jaws_settings` (`okur-lab reference-settings --jcf --scenario`) |

1144 live with Okur (`run-scenario.ps1 -Reader okur`, after `aa3e1dd1`): all 12 steps said as
JAWS said them, s5's "City Edit | xIzmir" included. Replay gives 11/12: the text typed in the
field reaches the buffer live through the page's re-read, of which the recording keeps nothing.

## Figures

At `aa3e1dd1`:

```
JAWS parity (Ia2): 11550/12074 steps match exactly; 383 accepted divergences not counted (0 scenarios could not run)
JAWS spoken parity: 11296/12140 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Uia): 10331/11968 steps match exactly; 383 accepted divergences not counted (15 scenarios could not run)
Buffer invariants: 0 violation(s)
```

(`okur-lab parity`, `parity --spoken`, `parity --api uia`, `invariants`.) `cargo test --workspace
--no-fail-fast` (4 threads), summed: 1654 passed, 0 failed, 201 ignored. Every change was
compared over all three layers with `parity-snapshot.sh` before and after: no step got worse.

## Next

1. When the owner has answered old-pc's Security Alert: record 1142 again and import it; record
   1143 (Smart Navigation's other table edges) and build what it shows.
2. JAWS's Scan Cursor in a WPF window (1002, 1003, 1013: one element a line, the title bar and
   its System menu first): 1013 to record again with `"trace_window": true`, then build it.
3. Screen Layout's lines (986, 987, 991, 992, 984): the snapshots would need each element's
   place on the screen.
4. Key labels (Manage Key Labels, read from JAWS's dialog with the Settings Center route
   above); the rest of the review (P3, P10, and the Office items when Office is back).
