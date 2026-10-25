# Handoff: Word's ribbon commands, Office's own UIA element, and the review of 2026-10-25

Follows [`2026-10-25-word-ribbon-and-tracer.md`](2026-10-25-word-ribbon-and-tracer.md) and covers
the 17 commits after it (`bd78520f` to `371962d3`) and this one. They were made under
[`docs/plans/2026-10-25-everyday-gaps-round-34.md`](../plans/2026-10-25-everyday-gaps-round-34.md).

The owner's order:
1. the settings redesign to its end;
2. then the rest of Microsoft Word;
3. then punctuation by word and the README's "Not yet" list.

The owner asked, this stretch:
- to take a statement out of the README (`9a103571`);
- whether line spacing had been tried through IAccessible2 instead of UI Automation;
- for a review of the whole codebase for performance, code quality, memory and hardening.

Where each stands:
- **Word:** its lower ribbon is read as JAWS reads it for the Home tab's first two groups and
  the View tab's views and check boxes. Empty lists and the field list say Word's words.
  Left: line spacing, Read Mode's warning, the rest of the ribbon's kinds (galleries, the
  Font group's colors, recording now in round-1048), Word's fields and links listed, Excel
  and Outlook.
- **The review:** 20 findings. Fixed or partly fixed: 1-12, 15-17 and 19 (11 fixed, 5 in
  part). Open: 13, 14, 18, 20. Recorded in
  [`../backlog/performance-review-2026-10-25.md`](../backlog/performance-review-2026-10-25.md)
  with each finding's commit.
- **Settings:** unchanged but for the review's fixes.

The next plan is
[`docs/plans/2026-10-25-everyday-gaps-round-35.md`](../plans/2026-10-25-everyday-gaps-round-35.md).

## Line spacing and IA2

It had not been tried through IAccessible2 before; only UI Automation's text attributes were
probed. `okur-lab ia2-probe` (`e9975f57`) asks the focused window and the window in front for
IA2. Run on old-pc in Word 2016 through `word-probe.ps1 -Ia2`, at every probe step:
- the document window (`_WwG`) and Word's window (`OpusApp`) answer MSAA ("Microsoft Word
  Document", role 10);
- QueryService gives no IAccessible2 (`"ia2":false`), so there is no IAccessibleText either.

IA2 is no source for line spacing in Word 2016. What is left is Word's object model, which hung
on the lab's unactivated Office before. Line spacing stays a documented gap (README, "Not yet").

## Word's ribbon

Recorded in 1045 (the Home tab's Clipboard and Font groups by Tab), 1046 (the View tab), 1047
(Insert+F5), and 1043 again.

**What JAWS says, and where it comes from.** `okur-lab msaa-focus` (`34d527fe`) follows MSAA's
focus and asks UIA for the same control two ways.
- MSAA, and the UIA element `ElementFromIAccessible` gives, have no help text and no toggle
  state.
- Office's own UIA element under the ribbon's window has both: help text "Increase Font Size
  (Alt+Ctrl+!)", Format Painter's toggle state 0.
- Found at the MSAA object's center with ElementFromPoint (9-12 ms on old-pc), it was the same
  element at every step; searching the tree for the keyboard focus took 6-52 ms.
- JAWS takes role and name from MSAA (Paste is a "split button" there, a plain button in
  UIA), the ScreenTip title and the toggle state from UIA.

**The tracer** (`be91ec5d`, `b2179e64`), trace format 8:
- A NetUI control's focus records Office's own UIA element: its rectangle, help text and
  toggle state (`NetuiControl`).
- Another MSAA object's focus is no repeat of the last one when its object resolves. UIA
  names only the ribbon's window, so Grow Font's focus, right after the ribbon's own, had been
  dropped. UIA's own focus events, passed on as WinEvents with small ids and no MSAA object,
  are still repeats.
- The converter:
  - takes drop-downs that open a list or grid (MSAA 0x38, 0x3A) as menu buttons;
  - puts a control under a ribbon group (`NetUIChunk`, Word's Font box) on the ribbon's
    menus;
  - skips the ribbon's own client focus with no tab focused, and a NetUI focus naming no MSAA
    object;
  - takes the same ribbon command named again right after as the same focus.

**The module** (`apps/windows/word_ribbon.rs`, `be91ec5d`, `8f1baaed`), as JAWS reads it:
- Down from the tabs: the group's name alone, then the command, "Clipboard | Paste split
  button";
- an unavailable command, "Cut Button Unavailable", with no place;
- a toggle button or check box with its state, "Format Painter Button not checked", "Show |
  Ruler check box not checked";
- the ScreenTip title after the command, unless one has the other in it: "Grow Font Button |
  Increase Font Size (Alt+Ctrl+!)", but "Cut Button" (tip "Cut (Ctrl+X)");
- an edit combo with no group, "Font: edit combo | Calibri", and no group at Grow Font after
  it;
- the first sub menu makes the ribbon a menu: "menu | | Font group | Change Case sub menu".

**Word's document module** (`apps/windows/word.rs`, `WordDocument`, `8f1baaed`) words empty
lists through a new hook, `AppModule::empty_list`:
- the Links List: "no links | Check for link type fields with Insert+F5." [obs, 1044/s1];
- Select a Form Field: "no fields" [obs, 1047/s1].
Okur does not read Word's fields yet, so both are said of every document.

## The review's fixes

From [`../backlog/performance-review-2026-10-25.md`](../backlog/performance-review-2026-10-25.md),
each with a test that fails without its fix, where one could be written hermetically:
- **1** (`611c43a4`): a list chosen in Virtual HTML Features is no longer destroyed with it. A
  closing dialog passes the dialogs it owns to its own owner. Not checked live.
- **2** (`c86b6804`): a value change is the field's text only when it concerns the focus,
  live and in the converter. Parity's baseline file was byte-identical before and after.
- **3** (`7748d63d`): a voice context back to the default voice sets it again instead of
  starting a new synthesizer.
- **4** (`7748d63d`): a wrong value in a settings layer is left out alone.
- **5** (`611c43a4`): a panicked speech batch is counted out.
- **7** (`2c12e6ce`): a save writes only the settings files whose values changed.
- **8** in part (`42c68661`): a tree's own openings are no longer told to Okur as the user's.
- **9** in part, **10**, **11**, **12**, **15**, **16**, **17**, **19** in part (`611c43a4`,
  `c86b6804`).
The okur-ui fixes (1, 6, 8, 10, 12, 17) were not checked live: a live check with the owner's
leave, or on old-pc beside JAWS, is in the next plan.

## Found, and how

- **old-pc's JAWS profile was at Rate=20 again**, saved 2026-10-24 08:48 old-pc time during an
  owner session. Rounds 1038 (again), 1042, 1044 and 1045's first recording were made at 20.
  Their evidence's provenance shows it (`jaws_settings`). `fix-rate.ps1` restored 90 before
  round-1045b; rounds 1045b and 1046 are at 90.
- **1045's first recording** (Rate 20, the old tracer) lacked Grow Font's focus: the tracer's
  repeat rule. It was recorded again as round-1045b after the fix.

## Proof

`okur-lab parity` at `371962d3`:

```
JAWS spoken parity: 8885/9603 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 8972/9542 steps match exactly; 251 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `a3573ae9`: 8928/9503.

`okur-lab parity --only` 1043 and 1044, as printed:

```
JAWS parity (Ia2): 11/11 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
JAWS parity (Ia2): 3/3 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`preview-round.sh` of round-1045b gave "JAWS parity (Ia2): 16/16 steps match exactly", and of
round-1046 (1043, 1046, 1047) "34/34". `cargo test --workspace`: 1506 passed, 0 failed, 201
ignored (summed from cargo's result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 673 scenarios, 2025 trials, 2009 succeeded.

## old-pc

- Recording round-1048 (Word's Home tab across the Font group into the Paragraph group, 24
  Tabs) as this is written: `C:\Users\ailabs\sr-harness\evidence-out\round-1048`.
- JAWS at Rate=90 (`fix-rate.ps1`, 2026-10-25 06:08 old-pc time). The okur, okur-lab and
  okur-ui there were built at `7748d63d` (the tracer of `b2179e64`, guarded reads).
- Still the owner's to end: Excel's three processes from 521's trials (9104, 10728, 12680),
  the windowless Word (9688) and the PowerShell (4200). The owner's Word document
  ("Document1") is open.
