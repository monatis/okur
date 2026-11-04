# Handoff: Say Window in any window, balloon tips, password edits (2026-11-04)

Follows [`2026-11-04-progress-bars-and-tool-tips.md`](2026-11-04-progress-bars-and-tool-tips.md).
Plan: [`../plans/2026-11-03-review-round-2-before-cross-platform.md`](../plans/2026-11-03-review-round-2-before-cross-platform.md),
its §3 (JAWS's features from Settings Center and the help). Branch `windows-ui-round-2`;
`main` was at `b3fc8d9c`. The owner, twice this stretch: 1.1.0.0 is not published yet; continue.

## Found first-hand

- **Say Window outside a dialog box** (548/s14, 549/s14, 554/s14, 1136/s3, recorded before):
  JAWS walks the whole window in order, an entry for each part of each control: its name, its
  kind's word, its states, its value, the same at every verbosity level. The words are UI
  Automation's own names of its control types ("menu bar", "tab item", "header item"); text
  and list items have none; "collapsed  selected" is one entry, a leaf tree item "leaf node";
  a list with column headers "list | grid with 2 columns and 3 rows"; off-screen and
  unavailable controls, a list item's cells and the title bar are left out.
- **Tool tips** (1138, recorded today): JAWS says a balloon tip shown for the window in front
  by its MSAA name, its title and its text ("Unacceptable Character: You can only type a
  number here.", an edit's own as a letter is typed in a numbers-only field; "Password: Use at
  least 8 characters.", one an application shows), queued after what the focus said. A plain
  `tooltips_class32` tip shown beside a focused button without the mouse is not said. MSAA
  gives a balloon role 31 (help balloon) and "Title: text"; UI Automation names it by its
  text alone (research probe on old-pc, not evidence). A WinForms password field is
  "Password: | password edit | ‹› | Type in text.".
- **Say Blank Lines During Say All** (`[Options] SayBlankLineCount`, 0 in old-pc's shared
  `Default.JCF`; 1139's first recording, not imported): with it at 1, JAWS's Say All over a
  WinForms text box's blank line is the same as at the default (406/s9), in both layers: the
  blank line passes silently. Arrowing onto it says "blank" in the synthesizer layer only.
  Whether the option acts elsewhere (a page, Word) is open.

## Built

| Commit | What |
|---|---|
| `0adec58e` | Harness: `okur-lab window-tree` (with `--append-to`, `--step`, `--window-title`), and a scenario's `"trace_window"` saving the fixture's window after each step beside its trace |
| `13d91c1f` | Say Window outside dialog boxes: `okur_core::window_walk` (`ControlKind`, `WindowControl`, `say`), `ApplicationActions::read_window_controls` answered by `AccessibilityEvent::WindowControls`, the Windows model's `window_walk::window_controls`, the live driver's walk on the bulk lane, replay answering from the saved tree |
| `3ee5b8ab` | corpus: 548, 549, 554, 1136 recorded again with the window's tree |
| `408d2fb4` | Balloon tips: the capture's `tool_tip_shown` (a tool tip window owned by the window in front, with its MSAA client object; live too), `AccessibilityEvent::BalloonTip` from a help balloon's name, said without interrupting |
| `c56173d8` | Password edits ("password edit", an empty state, never the text); the setup's fresh text reads answered in the setup |
| `546bc576` | corpus: 1138 (`desktop-balloon-tips.ps1`) |
| `ff02a419` | Replay: a reset key that moved nothing in a field settled within the setup; corrects `c56173d8`'s "no step changes", which was computed with an older binary |
| `a1e8a82d` | Harness: `parity-snapshot.sh` and `preview-round.sh` build the working tree's okur-lab when none is named; two pitfalls in [`verification-loop.md`](../references/verification-loop.md) §8 |

## Figures

`okur-lab parity` at `a1e8a82d`:

```
JAWS parity (Ia2): 11480/12062 steps match exactly; 370 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab parity --spoken`: `JAWS spoken parity: 11238/12128 steps match exactly`. From
11472/12058 at the last handoff: the four Say Window steps turned to matches, and 1138's four
steps were added, all matching in both layers. `okur-lab invariants`: `Buffer invariants: 0
violation(s)`. `cargo test --workspace --no-fail-fast` (4 threads), summed: 1637 passed, 0
failed, 201 ignored.

## Judged, not changed

- **P13** (Say All resumed after a page refresh scans the lines once per place it held): the
  places are the few pieces Say All has queued, one pass over the lines each; an index of the
  lines would cost a pass and an allocation per line itself. Left as it is.
- **Capitals during Say All** (`SayAllIndicateCaps=1`, 718): JAWS raises each capitalized
  word by +4 inside one utterance, and also some lowercase words at an utterance's start
  ("four", "spaces", "teh", "quick"), which looks like its own pitch state leaking, an accident
  not to copy. Okur's speech carries one pitch per utterance; per-word pitch needs markup
  through the speech thread and both synthesizer drivers. Not built yet; the history layer of
  718 matches.

## Next

1. Import 1139 recorded again (its four Ups as steps of their own: replay presses a repeated
   step's keys before playing its events, and the engine keeps one pending edit key, so a
   repeated arrow in a field read only its first line; every desktop repeat step recorded so
   far is a double press, which that order suits), queued behind 1140 and 1141.
2. Import 1140 and 1141 (JAWS's phonetic spelling after a pause on a character, desktop and
   virtual cursor, each step followed by 3 s of quiet), queued behind it; if JAWS says the
   phonetic word, build it with its setting (Settings Center's "When Pausing After Character
   Navigation, Speak Character Phonetically", 1.5 s by default; the customization map's row
   63).
3. The rest of §3 by everyday use: key labels and key echo (row 27), the quick key delay
   (row 21, Never by default), per-word pitch for capitals during Say All.
4. The review's open items that need old-pc's live runs (P3, P6, P10) and Office (P4, P8, P15).
