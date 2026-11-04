# Handoff: after old-pc's outage (2026-11-04)

Follows [`2026-11-04-say-window-and-balloon-tips.md`](2026-11-04-say-window-and-balloon-tips.md).
Plan: [`../plans/2026-11-03-review-round-2-before-cross-platform.md`](../plans/2026-11-03-review-round-2-before-cross-platform.md),
§3. Branch `windows-ui-round-2`; `main` was at `9cba8487`.

old-pc lost power and came back with the Store build of Okur (`Monatis.okur_1.0.0.0`) started at
sign-in beside JAWS, its Startup Wizard open, and a Windows Security Alert asking whether to let
the lab's `opendesk.exe` (the owner's `opendesk serve` after the reboot) through the firewall.

## Found first-hand

- **Okur at sign-in is as designed:** the package's startup task is enabled
  (`packaging/msix/AppxManifest.xml.in`), and a start by it counts as a sign-in start, which
  honours the user's choice (`crates/okur/src/sr.rs`, `start_kind`; on by default, as JAWS
  starts after sign-in for the user by default). An ordinary `Stop-Process` cannot end it: with
  UI Access it runs above an ordinary process; an elevated one can.
- **The phonetic pause in Settings Center** (research, JAWS's user file unchanged, hash checked):
  "When pausing after character navigation, speak character phonetically", in Text Processing,
  General, Phonetic Announcement, beside Spell Phonetically Always; Never, After one half second,
  After one second, After one and a half seconds (the default). JAWS's key names for TypeKey are
  `DownArrow` and `UpArrow` (`Down` and `Up` do nothing).
- **Say Blank Lines During Say All** (1139, imported): no change in a text box.
- **Keys that type nothing** (1142, recorded, import undone): JAWS says nothing for Escape,
  Shift, Ctrl, F2 or Space at old-pc's defaults; Backspace deleting a space says "space", kept in
  its history, where a space the arrows reach is kept out (about 80 recorded steps); Delete on
  the empty last line says "blank", kept out, as a line break is.
- **The live speed-ups measured** (A/B, three rounds, in `performance-review-2026-11-03.md`): a
  grid cell's focus read about 10 ms faster at the median, a page's button about 4; Settings, the
  Run dialog and Explorer's ribbon unchanged within their spread.

## Built

| Commit | What |
|---|---|
| `582eea85` | corpus: 1139 |
| `b46751a8` | Settings Center: the phonetic pause by JAWS's name and four choices, in a Phonetic Announcement subgroup |
| `5d9d6d5d` | Harness: Okur stopped (elevated) before a JAWS recording when it runs; deployed |
| `68b67f47` | Review: the live A/B |
| `ac2f9a8d` | Parity: bidirectional marks left out of the comparison (nine 3155 steps to matches) |
| `5ed0d363` | The settings store's flush says whether it wrote; its test, which failed once under load, waits for it |
| `f6a88fd5` | Edits: a deleted space by its name; the text's end as a line break |
| `ed924c4b` | Corpus: every traced process must be a reviewed one (fails on 1142's first trace, "rundll32.exe") |

1142's first recording traced the Security Alert (its path names the lab account), which took
the focus during the trials. Its import was undone before any commit: evidence is never
edited, so it is to be recorded again once the alert is answered. Answering it changes the
firewall's settings, which is the owner's to do (asked, 2026-11-04).

## Figures

`okur-lab parity` at `ed924c4b`:

```
JAWS parity (Ia2): 11501/12074 steps match exactly; 371 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab parity --spoken`: `JAWS spoken parity: 11251/12140 steps match exactly`. `okur-lab
invariants`: `Buffer invariants: 0 violation(s)`. `cargo test --workspace --no-fail-fast` (4
threads), summed: 1649 passed, 0 failed, 201 ignored.

## Next

1. When the owner has answered old-pc's Security Alert: record 1142 again (`corpus/pending`),
   preview it (8 of 8 expected in both layers with `f6a88fd5`), import it.
2. Smart Navigation (`SmartNavigation=3`, recorded in 954-956, 984, 991-993: about 105 differing
   steps): the virtual cursor's character and word keys step over a control, a label, the page's
   title, a region's and a list's start as one unit, said as its line says it; plain text by
   character; in a table the arrows by cell (customization map row 105).
3. The review's P6 (a Store app's window texts read on demand for Insert+T, about 6 ms of each
   Settings focus), P3, P10, measured with the A/B script (`live-ab.ps1`'s form).
4. Key labels (Manage Key Labels, read from JAWS's dialog), the quick key delay (Never by
   default).
