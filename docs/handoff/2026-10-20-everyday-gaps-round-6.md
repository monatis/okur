# Handoff: everyday-use gaps, round 6 (2026-10-20)

Follows [`2026-10-20-everyday-gaps-round-5.md`](2026-10-20-everyday-gaps-round-5.md) and covers
the commits after it (`95e32e0` to `3cf36df`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-5.md`](../plans/2026-10-21-everyday-gaps-round-5.md).
The next plan is [`docs/plans/2026-10-21-everyday-gaps-round-6.md`](../plans/2026-10-21-everyday-gaps-round-6.md).

## The owner's answers and requests

- **"jaws" only for JAWS itself, in message ids and code too** (`289169b`):
  - message ids: `cursor-okur`, `cursor-okur-active`, `cursor-route-okur-to-pc`,
    `settings-okur-key*`;
  - code: `ActiveCursor::Okur`, `OkurKey`, `OkurCursor`, `RouteOkurToPc`, `OkurFind`,
    `OutputType::OkurMessage`, the `OKUR` chords;
  - names that stand for JAWS keep it: its key map's script names in the parse table, its
    speech history (`jaws-history`), its evidence (`jaws_variants`);
  - the catalog's trademark check now reads message ids too.
- **README** (`7213cc1`, and with this handoff):
  - Okur's features are no longer called JAWS's, and a note says Okur speaks in its own name;
  - the trademark line is AGENTS.md §1.5's;
  - "Not yet" lists only what is missing, checked against the code, and agrees with "What
    works today";
  - the figures are rerun at `3cf36df`.
- **The development certificate** (thumbprint given): the release build is signed and on
  old-pc in `C:\Users\ailabs\sr-harness\okur-signed`, hashes checked. The owner's steps there
  are in the next plan's §6.

## Decided without the owner (reversible)

- **Say Color's names** are X11's `rgb.txt` as JAWS says them, plus the entries JAWS was seen
  to name otherwise. The table is a locale data file (`locales/en-US/colors.txt`) with X.Org's
  permission notice, and a line in `NOTICE`. It is written by
  `skills/jaws-reference/scripts/color-table.js`.
- **After a control's change, the host builds the page again** on pages of 5,000 nodes or
  more. Smaller pages are built at once, in a few milliseconds.
- **`okur-lab-keys`** is a harness program with UI Access, never installed for users.
- **Not changed:**
  - The replay's dialog mirror does not yet carry the text typed into Find what. With it, 506/s8-s10 would depend on F3, which 623 asks about. The patch is kept in the scratchpad (`mirror-typed-text.patch`).
  - Okur Find's field still says "1 of 1". JAWS gives a position for the Run dialog's combo ("C: 1 of 26"), so its rule is not known.
  - WordPad's ribbon tab positions: JAWS said "Home 1 of 2", "View 1 of 2" and "Home 2 of 2" (523).

## What was done

### Features (each checked with `okur parity` against old-pc's evidence)

- **Say Color (Insert+5)** (`af4fb10`, `3cf36df`):
  - a page's text runs keep Chromium's `color` and `background-color` (IA2 text attributes);
  - Insert+5 says them by name, "blue2 on white";
  - on a line Okur says for an element (the title, a region, a list) it says "default on default", as JAWS does.
  - The names are X11's as JAWS says them:
    - CamelCase in words ("Pale Green2");
    - without X11's 2014 web additions;
    - JAWS's own entries, such as "lime green 2" for 0,128,0, "crimson" at 102,0,51 and "Dark Gray" at 153,255,255.
  - A color not in the table takes the nearest entry by the "redmean" distance:
    - it names all 365 observed colors as JAWS does (618's 148 CSS colors, 619's 216-color grid, 565's #123456);
    - fixed weights (2, 4, 3) missed 18 of them and the plain distance 37.
- **Performance review P3:**
  - **Step 1** (`ba281e3`): a page control's change (Space on a check box, a combo box's value) patches its node in place. The live screen reader builds big pages again on the blocking pool, and a generation refuses a build that a later change made stale. `big_page_keys` at 25,032 nodes: a check box toggled took 57.0 ms when built at once, 3.0 ms when left to the host. The patch itself takes about 1 µs.
  - **Step 3** (`6754422`): page parts spliced into a page that changed meanwhile are spliced again off the engine, up to 3 tries. A part not found asks for the whole page without splicing again under the lock.
- **Backlog refreshed** (`a01e1fc`, and with this handoff): rows that lagged the code now match it, and open rows name their queued recordings.

### Harness

- **`okur trace --debug-log`, and `-TraceTimings`** in `queue-batch`, `run-batch` and `run-scenario` (`973c022`): a capture's event reads are logged with their stages timed.
- **`okur latency`** reports each stage's own time (`73c0ebf`), for P6's measure.
- **`okur-lab-keys`** (`32e10d4`):
  - a key sender with UI Access, for JAWS's modal dialogs, which TypeKey closes (559/s2);
  - a scenario step of type `key_ui_access` uses it, and replay reads such a step as a key;
  - it needs the development signature and a copy under Program Files, which is the owner's step.
- **`color-table.js`** writes the color table from X.Org's `rgb.txt` and `COPYING`.

### Research written (pending scenarios)

- 621-622: the hints on Tab.
- 623: F3 after Okur Find.
- 624, 625: Insert+T in a page.
- 626: a field arrowed back onto.
- 627: the clipboard keys in their key map's order. 520's Insert+Win+X reached Windows' own Win+X menu.
- 628: the layer's help and Command Search.
- 509 queued again: Keyboard Help timed out in round 13, before the speech recorder voice.

### Imported

- **618 and 619** (`052646a`, round 83 and snap84):
  - 3 trials each, all passed;
  - capture script `1a29afa8`; scenario hashes are the committed files';
  - no links or personal data.

## Figures

Each is the named command's output at `3cf36df`:

- `okur parity`: "JAWS parity (Ia2): 4224/4422 steps match exactly; 88 accepted divergences
  not counted (0 scenarios could not run)". At the previous handoff it was 3484/3694; 618
  and 619 added 728 steps.
- `okur parity`, spoken layer: "JAWS spoken parity: 4104/4370 steps match exactly".
- `okur invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 285 scenarios, 909 trials, 909 successful.
- `cargo test --workspace`: 1226 passed, 0 failed, 199 ignored.

## Learned

- **JAWS's color names** are not any single published table. They are X11's (old `rgb.txt`, first name for a color wins, CamelCase in words), HTML's where X11 differs (gray 128, maroon, purple, olive, silver, teal), and a few of its own. The nearest is chosen by the redmean distance.
- **Forms mode on Tab, and the hints:**
  - 555 (Verbosity written, so JAWS was restarted) went into forms mode on Tab and gave the hints, with AutoFormsMode=0 in the file.
  - 204, 212 and 213 (nothing written, JAWS long running) did neither.
  - `jaws-settings.ps1 -Apply` keeps the file's AutoFormsMode, so the restart, or a state the long-running JAWS had, is the likelier cause.
  - 621 and 622 decide (next plan §2).
- **Notifications:**
  - In 533 Focus Assist was on and no banner showed, yet JAWS said the toast and listed it.
  - JAWS reads notifications without the banner's window.
  - Okur needs either the banner (UIA, as NVDA does) or Windows' notification listener.
- **JAWS's modal dialogs** (Jump To Line) close on an FSAPI call, so TypeKey cannot type into them.
- **The tracer's timings** could not be had from a lab round before `973c022`.

## Recorded meanwhile, and queued

old-pc's queue at this handoff, in order: round 85 (513 and 516 with the DPI tracer, running),
86 (598, 613), snap87 (the selection fixtures), 88 (620), 89 (621, 622), 90 (623-626), 91
(509, 627, 628), perf92 (540, 541, 323, 524 with `-TraceTimings` and the release tracer
`b6f072e9`). No Defender detection since 14:13 (checked before round 92 was staged).

## Not done, and why

- **The large selection rule:** it waits for 617 (fetched), snap87 and 620.
- **Keyboard Help, Command Search and the layer's help:** they wait for 509 and 628. All three need a catalog of Okur's command names and descriptions in Okur's words.
- **The clipboard keys:** they wait for 627.
- **P6, the dialog cache:** it waits for perf92's measurement.
- **The hints on Tab and forms mode on Tab:** they wait for 621 and 622.
- **Office:** last, as the owner asked.
