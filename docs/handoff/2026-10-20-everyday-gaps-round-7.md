# Handoff: everyday-use gaps, round 7 (2026-10-20)

Follows [`2026-10-20-everyday-gaps-round-6.md`](2026-10-20-everyday-gaps-round-6.md) and covers
the commits after it (`2b7933e` to `842e4c5`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-6.md`](../plans/2026-10-21-everyday-gaps-round-6.md).
The next plan is [`docs/plans/2026-10-21-everyday-gaps-round-7.md`](../plans/2026-10-21-everyday-gaps-round-7.md).

## The owner's answers and requests

None new in this stretch. The owner's steps from the last plan (§6) are still open.

## Decided without the owner (reversible)

- **JAWS's accidents, written as accepted divergences** (`docs/references/accepted-divergences.md`):
  - **F3 right after JAWS Find** (506/s8-s11, 633/s9-s14). JAWS says nothing, and leaves the
    cursor on lines without the word. Okur goes to the match.
  - **The PC cursor's coordinates at 125%** (513, 516). JAWS gives 1.25 times the screen's
    pixels. Okur says the screen's own pixels.
  - **"Required" for the sign-up form's email field** now also covers 632/s10, reached the same
    way as 280/s7.
- **Help in Okur's own words.** Keyboard Help's descriptions (634/s2-s6) join the help keys'
  entry: the key's name is JAWS's, the description is Okur's.
- **Okur Find's Find what has no position.** JAWS says none for its own Find what (506/s12,
  633/s1), but says one for the Run dialog's combo ("C: 1 of 26"). The rule applies only to a
  focus in Okur Find.
- **The dialog cache's rule** (P6). A dialog's read is kept until any event from its process
  says it may have changed. Kept events are:
  - the focus;
  - the caret and the text selection;
  - a push button's state change, as the default button follows the focus.
  - A dialog still hidden is never kept.
- **Steps from the speech recorder alone** (`"history": false`). Parity counts them only on the
  spoken layer: "spoken only" in the baseline.

## What was done

### Features (each checked with `okur parity` against old-pc's evidence)

- **Selections as JAWS says them** (`246b4b1`):
  - over 1000 characters, "N characters up to" and the last 20;
  - selected text with every symbol named;
  - a selected character named and pitched;
  - 617 and 620 match every history step.
- **A line selected whole is said as the line is** (`8d8f25b`): a list item that is only a link
  keeps "same page link" after its bullet (562/s2, s8).
- **Insert+T in a page** (`8e10759`): after the title, the innermost region or article, its level
  and "heading is" the last heading before the cursor in it. 624 and 625 match.
- **Forms mode on Tab without Auto Forms Mode** (`3c32b39`):
  - Tab onto a one-line edit turns forms mode on once the virtual cursor has moved since the
    page loaded (629-631);
  - in forms mode, Tab keeps it on onto edits, combo boxes and radio buttons, with the hints and
    the first field's regions;
  - 552, 553, 555 and 621 match 11 of 11 steps; 204 and 622 match 13 of 13.
- **Down back onto the field Up left is silent** (`603454b`, `5430a8a`), straight back (626,
  567) or later (632/s7). The field stays the one left until the page's focus moves elsewhere.
- **Keyboard Help, Insert+1** (`f12d98f`, `e42430d`):
  - a key is named as JAWS names it, then described in Okur's words;
  - pressed twice quickly, a longer description;
  - a key Okur leaves to the application says so;
  - the catalog has 159 commands.
- **Okur Find in replay** (`2be518d`): the text typed into Find what reaches Okur's dialog (633/s2-s6).
  633's Enter, Say Line and F3 from the top match.
- **okur.exe is aware of each monitor's DPI from its start** (`6a187f4`, PerMonitorV2 in its manifest).

### Performance

- **P5 step 2** (`c783795`): a state or selection change of another control than the focus is
  dropped after its MSAA object, before the focus chain, siblings and text are read.
- **P6, the dialog cache** (`8cb10d6`, `6d8aa2f`):
  - A focus in a Win32 dialog reads the dialog's layout and control tree once.
  - Later focuses reuse it until the dialog changes.
  - Measured on old-pc with `okur latency` over the tracer's logs. The `window` stage per
    focus, as count, median, 90th percentile (ms):

    | Dialog | Before (round 92) | After (round 97) |
    |---|---|---|
    | Font (541) | 19, 801.0, 844.0 | 17, 1.0, 843.0 |
    | Save As (524) | 16, 301.0, 328.0 | 16, 0.0, 320.0 |

  - The 90th percentile is the first read of each dialog and the font list's arrows. Each arrow
    there changes the dialog's values, so the dialog is read whole again, 0.82 to 1.0 s per
    arrow (round 97's log). That is the next plan's P6 step 2.

### Harness

- **`"history": false` steps** (`429cf49`). The capture neither clears nor reads JAWS's history
  for them. Keyboard Help describes those FSAPI calls instead of running them (509).
- **The dialog mirror reports typed text** (`2be518d`), from the field's text events.
- **The tracer logs each change counted against a dialog's read** (`6d8aa2f`, "dialog changed").
- **What Okur says before its own dialog stays compared** (`2be518d`): "Virtual Find" before
  JAWS Find, when JAWS says it first too.
- **635**: Left and Right around a page's inline links, which shows the characters JAWS's text
  holds there.

### Imported

Each import had all trials passed and a capture script hash of a committed version. The traces
were grepped for links and personal data.

| Commit | Scenarios |
|---|---|
| `2b7933e` | 513, 516 |
| `7ed04e7` | 598, 617 |
| `22a7d07` | 620 |
| `9d408a8` | 621, 622 |
| `c411653` | 624-626 |
| `e342abb` | 629-631 |
| `98c9661` | 632-634 |
| `842e4c5` | 635 |

Not imported:
- 623 is retired (TypeKey typed old-pc's Turkish dotless "ı"; 633 replaced it).
- 627 and 628 were inconclusive.
- Round 86's 613 trials failed: the tracer stopped reading events and did not exit.

## Figures

Each is the named command's output at `842e4c5`:

- `okur parity`: "JAWS parity (Ia2): 4439/4612 steps match exactly; 102 accepted divergences
  not counted (0 scenarios could not run)". At the previous handoff it was 4224/4422 with 88.
- `okur parity`, spoken layer: "JAWS spoken parity: 4314/4567 steps match exactly".
- `okur invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 300 scenarios, 954 trials, 954 successful.
- `cargo test --workspace` (at `8d8f25b`, no test changed since): 1242 passed, 0 failed, 199
  ignored.

## Learned

- **Forms mode on Tab** depends on how the user came into the form, not on a written setting.
  Tab first after the page loads leaves the virtual cursor (629). Down then Tab turns forms mode
  on (630). Say Line then Tab does not (631).
- **Keyboard Help waits for a second press.** Pressed twice quickly, JAWS said the key's name
  once, then the longer description (634/s3). The first press was never described. Okur
  describes both presses: the next plan's §2.
- **JAWS's text around inline links** (635, 3 of 3 trials alike):
  - It keeps a space before a link that starts a line: before Contact us come "Gazette. " and a
    line break, both silent to Left.
  - A list item's bullet is outside its link: Left from the link's first letter gives a space,
    then "•".
  - End goes past a line's last letter, saying nothing; Left then says the last letter.
  - The page's text ends with a line break after its last line.
  - Okur trims those spaces, puts the bullet inside the link, stops End on the last letter and
    ends without a line break. That is 562's five characters (2030 for 2035): "By ", "New ",
    " open in May.", "Gazette. " and the last line break. The next plan's §1.
- **The Font dialog takes the focus while still hidden.** UI Automation already sees its
  controls, but none is visible to lay out yet (round 96).
- **old-pc types JAWS's TypeKey("i") as the Turkish dotless "ı"**: a scenario must not type an "i".

## Recorded meanwhile, and queued

Round 99 is running on old-pc: 636-641, Insert+T in a page (624, 625) and the field Down
returns to (632) at Intermediate and Advanced verbosity, 3 trials each.

## Not done, and why

- **JAWS's text around inline links:** found by 635 at the end of this stretch; the first item of
  the next plan.
- **Keyboard Help's wait for a second press:** the engine has no timer yet.
- **The spoken "blank"** for an empty first line in a large selection's last 20 characters
  (620): a speech-only utterance, not built.
- **The clipboard keys:** 627 was inconclusive; Windows+Insert+X reached Windows' own Win+X menu.
- **The layer's help and Command Search:** 628 was inconclusive on old-pc's Turkish layout.
- **613's tracer hang:** the log from round 92 is fetched (`perf92/613-web-turkish-text-run.log`
  in the scratchpad) and not yet read.
- **Notifications:** wait for the owner (Focus Assist off for one recording).
- **Office:** last, as the owner asked.
