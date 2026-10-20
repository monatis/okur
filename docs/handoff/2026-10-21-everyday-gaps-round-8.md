# Handoff: everyday-use gaps, round 8 (2026-10-21)

Follows [`2026-10-20-everyday-gaps-round-7.md`](2026-10-20-everyday-gaps-round-7.md) and covers
the commits after it (`a70858a` to `50d03ec`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-7.md`](../plans/2026-10-21-everyday-gaps-round-7.md).
The next plan is [`docs/plans/2026-10-21-everyday-gaps-round-8.md`](../plans/2026-10-21-everyday-gaps-round-8.md).

## The owner's answers and requests

None new in this stretch. The owner's steps (the development certificate, Focus Assist, the
Defender exclusion, Office) are as the last plan's §6.

## Decided without the owner (reversible)

- **JAWS's double-press window is 350 ms**, not Windows' 500 (`452308f`). It applies to every
  twice-quickly key. Say Line pressed again 300 ms later spelled the line in 3 of 3 trials,
  350 ms later in 1 of 3, 400 ms later in none (654).
- **Keyboard Help does not wait for a second press** (642). JAWS says the key at once and its
  description right after. A second press flushes the first press's description from JAWS's
  queue before the speech recorder hears it. Okur's spoken record keeps every utterance; this
  is written in the help keys' accepted divergence, with Okur's own description words.
- **A radio button's place after JAWS's mouse click** (534/s10). JAWS's Say Line left out
  "2 of 3" right after Numpad Slash checked the button, in all three trials. Say Line gives
  it otherwise (246/s4). Accepted as JAWS's accident.
- **"Required" for the email field** also covers 640/s10 and 641/s10 (632 at Intermediate and
  Advanced).
- **Say Color outside a page says "default on default"** (`d0121f7`), as JAWS on a dialog's
  button (513/s3). In a native edit JAWS reads "black on white" (661); Okur does not read
  native colors yet (next plan, §1).

## What was done

### Features (each checked with `okur parity` against old-pc's evidence)

- **The text beside inline links as JAWS's holds it** (`e6c5670`, 635 matches every step):
  - the space where a link's line break cuts the text stays ("Gazette. ", " open in May.");
  - no space is kept after punctuation the link took ("Maria Lopez," then "published");
  - a list item's link starts after its bullet;
  - End goes past the line's last letter.
- **A selection to a page's end** (`56e678d`) takes the line break JAWS's text has after a
  closing region. 562/s13, s17, s20 and 515/s1 now match, and 650/s1 and 653/s1 end the same
  way. 617, 620 and 651, whose pages end with a paragraph, have no such line break.
- **Twice quickly, spelling and Ctrl+End** (`452308f`):
  - the 350 ms window;
  - the virtual cursor's spelled line ends with its space ("b a n n e r space", 654, 408/s8);
  - Ctrl+End goes to the last line's start (643/s22).
- **Insert+T in short messages** (`2bc4130`): "article | 3 | What changes" at Intermediate
  and Advanced (636-639, every step).
- **The menu bar's first line** (`8ced51e`): "Main menu MenuBar with 3 items" (652/s1).
- **A text area's lines keep their leading spaces and tabs, and a `<pre>` its empty lines**
  (`6a42c0a`, 650/s1).
- **Recorded and matching as built, nothing to change:**
  - 644-649 (Tab into a form at Intermediate and Advanced);
  - 656-659 (large selections and Say Color at Intermediate and Advanced): 96 of 96 steps.

### Performance

- **P6 step 2** (`a7d94d5`): a dialog's list, tree and grid items are read with 12 properties,
  not 42. Measured by `okur latency` over the tracer's logs: the Font dialog's `window` stage
  per focus (count, median, 90th percentile, 99th, in ms):

  | Round | Count | Median | 90th | 99th |
  |---|---|---|---|---|
  | 97 | 17 | 1.0 | 843.0 | 852.0 |
  | perf105b | 17 | 1.0 | 673.0 | 713.0 |

  The font list's 187 items still cost about 3.6 ms each through UI Automation's MSAA proxy;
  the next plan reads only those on screen.

### Harness

- **The tracer logs each event's read as it starts** (`61fc818`, "reading event"). Its first
  use found that 613 never hung: the tracer read on to the trial's end. (Corrected after this
  handoff was written: the cause was not Chrome's offer to translate the page, as `dc85a1c`
  guessed, nor the page load, whose wait is capped at 3 s in a browser. run-scenario read the
  fixture's title "Türkçe Metin" in old-pc's code page, and the window title check failed every
  trial, round 110's too. Fixed in `f78ec7b`, `882a89f` and `24c59e6`; the fixture keeps its
  `notranslate`.)
- **Round 105 broke and was rerun.** Its first waiter pulled an older build. Stopping that
  waiter stopped the batch during 524, so round 105 never ended and rounds 106 and 107 waited
  on it. With no capture running, the right build was installed, round 105 was ended with a
  note (`aborted.txt`), and it was run again as perf105b; nothing from it was imported.
  Lesson: a waiter that pulls a build must be replaced only before it starts pulling.

### Imported

Each import had all trials passed, the committed capture script `bf89d333`, and was grepped
for links and personal data (the fixtures' sample addresses only).

| Commit | Scenarios |
|---|---|
| `cc97db7` | 642 |
| `f93051f` | 636-641 |
| `7dded1b` | 644-649 |
| `c56f4a7` | 643, 650-654 |
| `0fe418a` | 656-659 |
| `d23afbb` | 660 |
| `b6b94b9` | 661-663 |

## Figures

Each is the named command's output at `50d03ec`:

- `okur parity`: "JAWS parity (Ia2): 4712/4888 steps match exactly; 104 accepted divergences
  not counted (0 scenarios could not run)". At the previous handoff: 4439/4612 with 102.
- `okur parity`, spoken layer: "JAWS spoken parity: 4590/4850 steps match exactly".
- `okur invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 327 scenarios, 1035 trials, 1035 successful.
- `cargo test --workspace`: 1247 passed, 0 failed, 199 ignored.

## Learned

- **Ctrl+End** puts JAWS's virtual cursor at the start of the last line, not past its end.
- **JAWS's counts on the web application patterns page** are not one text's lengths: 1271
  characters from the fourth heading to the end, 1100 from the second, 1134 for the whole page
  (663, 653). Counts there cannot be modelled from Okur's text.
- **The editor fixture's WinForms text box does not take Ctrl+Backspace**: it typed a DEL
  character (660/s8). Round 111 records the key in Notepad.
- **Spelling marks**: Chrome marks a word misspelled only once the caret leaves it, so a
  trace read right after typing has no marks (632/s3, s8 and 640, 641).
- **A page field's color** is "default on default" while empty and "black on white" with
  text (662).

## Recorded meanwhile, and queued

old-pc's queue at this handoff: round 110 (613 again, its fixture not translated), then 111
(664, Ctrl+Backspace and Ctrl+Delete in Notepad).

## Not done, and why

- **Native colors for Say Color** (661) and **a page field's colors** (662): the tracer reads
  neither yet; the next plan's §1.
- **650 and 653's other differences**: 650 matches now; 653's counts are not text lengths.
- **The clipboard keys, the layer's help and Command Search, notifications, Office**: as before.
