# Handoff: everyday-use gaps, round 10 (2026-10-21)

Follows [`2026-10-21-everyday-gaps-round-9.md`](2026-10-21-everyday-gaps-round-9.md) and covers
the commits after it (`a977ac7` to `f5ee67e`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-9.md`](../plans/2026-10-21-everyday-gaps-round-9.md).
The next plan is [`docs/plans/2026-10-21-everyday-gaps-round-10.md`](../plans/2026-10-21-everyday-gaps-round-10.md).

## The owner's answers and requests

- **The README's "Not yet" list comes first, in its order**, starting with Say All by sentence
  or paragraph, and visited links. The next plan follows that order: Say All units, visited
  links, PlaceMarkers, custom labels, Flexible Web, then the rest of the list.

## Decided without the owner (reversible)

- **Say All Reads is Okur's own setting** (`say_all_unit`: line, sentence, paragraph), in the
  settings dialog as "Say All Rea&ds" and in the settings file, per application and site like
  the others. Line with Pauses (`LinePauses`) is not built: 679 will show what it changes.
- **V and U follow the browser's visited state** (IA2's traversed), which the page's reading
  already kept. U went to any link before. The messages for a page without one follow Okur's
  other quick keys; 682 then showed JAWS says the same: "There are no visited links on this
  page."
- **The Page Setup preview's pictures** are an accepted divergence: JAWS names them "Bottom:"
  and "Page Setup" in Say Window (544/s13, 674/s13, 675/s13). Okur reads no picture there.
- **A trial with JAWS's window in front is recorded again**, never imported: round 124 lost
  the first trial of 662 and 677-682 that way.

## What was done

### Features (each checked with `okur parity` against old-pc's evidence)

- **Say Window reads Win32 combo boxes as JAWS does** (`a977ac7`): the label, "Combo box", the
  value with the list's shown items, the place; then the list box and the edit combo. A
  scroll bar, owner-drawn text and a pane that only wraps a link are skipped. The tracer reads
  a combo box's place (`CB_GETCURSEL`, `CB_GETCOUNT`). 665/s3, 672/s3 and 673/s3 now differ
  from JAWS only in two points (see "Not done").
- **Say Window in a group box** (`814326e`, `8794493`): a field after its label has the box's
  name once ("Margins (inches) Left: | Edit | 0.75"), where a combo box says it again ("Paper
  Size: | Paper | Combo box"); Advanced leaves that out. An empty field is a plain "Edit".
- **Escape out of a dialog that focuses its Cancel button** (`8794493`): Notepad's Page Setup
  moves the focus to Cancel as Escape closes it; JAWS says only Notepad. 544, 674 and 675 now
  match in full.
- **Say Color in page fields** (`1f64734`, `a179ae4`), from 662 and 676: in forms mode the
  colors at the caret, an empty field's too; with the virtual cursor a field's text colors as
  the page shows them, "default on default" when empty, the focused field's as typed. The
  tracer reads a Chromium field's colors at the caret, at offset 0 when empty.
- **A long selection's last characters** (`1411dae`): when they begin with a line break,
  JAWS's synthesizer says "blank" there; Okur's spoken layer does too.
- **Say All by sentence or by paragraph** (`9451c37`, `e3b10a0`): in a page by the sentences
  Alt+Down reads and the paragraphs Ctrl+Down reads, Right and Left moving by them; in a text
  field by sentence from the caret's. Replay takes `SayAllMode` from the evidence.
- **V and U by visited state; the Links List's Display filters** (`95c4968`).

### Harness

- **A web trial waits for its browser to be in front** (`59f92f9`). After JAWS restarted with
  a scenario's settings, its window kept the front, and Windows refused to bring Chrome
  forward. Focus-Target now insists, minimizing and restoring as a desktop launch does.
- **A probe of the Font dialog's windows** (`probe-font.ps1`, round probe126, in the shared
  Dropbox): it opens the common Font dialog without a key and lists each child window's class,
  id, styles and visibility, combo lists included. It is for the two open Say Window points.

### Imported

Each import had all trials passed, the committed capture script (`113d4cf6`), and was grepped
for links and personal data.

| Commit | Scenarios |
|---|---|
| `9351c32` | 670, 671 (rounds 120); 524, 540, 544 again (round 121) |
| `9fb3be9` | 323, 544, 548, 549, 554, 600, 610, 665 again (round 122) |
| `4ed99d4` | 672-675 (round 123) |

`9351c32`'s message says 524, 540 and 544 kept their speech. Comparing transcripts shows one
step varies: 544/s14's empty mark before Notepad's title moved between trials. In round 122
it varies again, and 323/s10 names another note of the Security page in two trials.

Not imported:
- round 124 (662, 676-682): each scenario's first trial ran with JAWS's window in front
  (676's did not, but its tracer lacks an empty field's colors). Round 127 records them again.

## Figures

Each is the named command's output at `f5ee67e`:

- `okur parity`: "JAWS parity (Ia2): 4844/5017 steps match exactly; 106 accepted divergences
  not counted (0 scenarios could not run)". At the previous handoff: 4805/4978 with 105.
  (`8794493`'s message says "of 5018"; with 673/s2 accepted the total is 5017.)
- `okur parity`, spoken layer: "JAWS spoken parity: 4726/4979 steps match exactly". At the
  previous handoff: 4683/4940.
- `okur invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 340 scenarios, 1074 trials, 1074 successful.
- `cargo test --workspace`: 1258 passed, 0 failed, 199 ignored (at `a179ae4`; `f5ee67e`
  adds a pending scenario only).

## Learned

- **Chrome marks no link visited on a page opened by its address** (visited-links snapshot):
  even the link back to the page itself is unvisited, which fits partitioned visited links. A
  link followed from the page should count; the next plan records that.
- **JAWS's Say All by sentence** (677, trials 2-3): it breaks after a period and a space, so
  "9 p.m." and "e.g." end sentences. It keeps lines without one together ("list end Primary
  navigation region end banner end main region article"), and a link is a piece of its own.
- **JAWS's Say All by paragraph** (678): a block's inline parts go together ("By ‹link›
  Maria Lopez, published March 14, 2026"); a picture is read with the heading after it.
- **JAWS's custom labels** (Surf's Up, Using Custom Labels):
  - Insert+Ctrl+Tab labels the element at the cursor;
  - the label is read in its place and in the element lists;
  - labels are kept per domain;
  - Insert+F2's Custom Label Delete and Delete All remove them.
- **An empty page field in forms mode has colors** for JAWS ("black on white", 676/s8); with
  the virtual cursor it has none.

## Recorded meanwhile, and queued

On old-pc, in this order:
1. round 125: 684-686 (Say All in a text box by unit), 687 (a temporary PlaceMarker);
2. probe126: the Font dialog's windows;
3. round 127 (waiter127, push `4a0a6894`): 662, 676-682 again;
4. round 128: 688 (a custom label, deleted before and after).

## Not done, and why

- **Say Window: the Size list's name and the Font dialog's sample** (665/s3, 672/s3, 673/s3).
  JAWS names only the Size list ("Size: List box") and skips the sample "AaBbYyZz". Their UI
  Automation properties do not tell them apart; probe126 reads their window styles.
- **Say All's units against JAWS's**: the evidence is in round 127 and round 125.
- **Line with Pauses, visited links followed from a page, PlaceMarkers, custom labels,
  Flexible Web**: the next plan.
- **The clipboard keys, the layer's help, notifications, Office**: as before (owner steps).
