# Plan: everyday-use gaps, round 5 (2026-10-21)

Follows [`../handoff/2026-10-20-everyday-gaps-round-5.md`](../handoff/2026-10-20-everyday-gaps-round-5.md).
The list stays [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md):
the gaps are closed before macOS. Each item is found in JAWS's documentation, recorded on
old-pc, then built; JAWS's accidents become accepted divergences. old-pc always has a round
queued behind the running one. Okur's own words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with `okur parity`,
then a dated handoff and the next plan, with AGENTS.md pointing at them. A stretch is a few
hours or about thirty commits, not more.

## 0. First: what old-pc recorded meanwhile

| Round | What | Then |
|---|---|---|
| 83 | 618 and 619: Say Color on the 148 CSS named colors and a 6x6x6 RGB grid | wait for snap84, then build Say Color's table from JAWS's names (§1) |
| snap84 | IA2 and UIA snapshots of `color-names.html` and `color-grid.html` | import with 618 and 619; their fixtures move from pending to `corpus/fixtures` |
| 85 | 513 and 516 again, with the tracer of `87314da` (`okur.exe` f0c4bb20, installed as the round starts) | 516/s8 and 513/s8 (the PC cursor at 125%) should then match; if not, the probe of the previous plan's §3 |
| 86 | 598 (Ctrl+A at Intermediate, now waiting 90 s) and 613 (the Turkish page, its spelled line now waiting 60 s) | import; build what differs |
| snap87 | snapshots of `selection-sizes.html` and `selection-threshold.html` | import 617 (in the scratchpad's `r82/`) with its snapshot |
| 88 | 620: selections of 989 to 1029 characters | build the large-selection rule (§1) |

For each: fetch with `lab-fetch.sh`, check every trial passed and the capture script is
`1a29afa8`, check each scenario's hash against the committed file, grep the evidence and
traces for links, the lab account's folders and the owner's name, compare parity before and
after, and commit the corpus alone, with the baseline from the committed code. Before queueing
more, check `Get-MpThreatDetection` on old-pc. If queued batches seem stalled, list the
harness processes (a script like the scratchpad's `ps-list.ps1`): a queue process that died
leaves every batch after it waiting.

## 1. Recorded, to build

- **A large selection (562, 617, 620).** JAWS reads a selection whole up to a size. From
  there it says "selected N characters up to" and the selection's last 20 characters, where
  N is the selection's length in JAWS's buffer text (a line break counts one) less those 20:
  1052 − 20 = 1032 (617/s24), 1133 − 20 = 1113, 1230 − 20 = 1210. 617 read 971 characters
  whole and summarized 1052; 620 narrows the cut-off. Okur's buffer has about twice JAWS's
  line breaks around inline links (562), so N on a real page needs the count of JAWS's lines.
  Build the rule on 617 and 620's plain lines first, then compare 562.
- **Say Color.** Build the table from 618 and 619's names (JAWS's own: "blue2", "lime green
  2"), each with its RGB from the snapshots, and the nearest name for other colors from 619's
  grid. 565 checks it.
- **598 and 613** after round 86.

## 2. Verbosity, beyond what is built

- The hints on Tab ("Type in text.", "To change the selection ..." in page forms): 552, 553 and
  555 (Down twice, then Tab; Verbosity written at each level) have them at every level, 204,
  573 and 583 (Tab from the top; no Verbosity, then Intermediate and Advanced written) have
  none at any level. The recorded settings differ only in the Verbosity key, so the way in is
  the likelier cause. 621 (555's keys, no Verbosity) and 622 (204's keys, Verbosity=0) decide.
  Also 555/s10: Escape says "virtual PC Cursor" there, nothing in Okur.
- 552-555 (the forms fixture at each level, 7 steps each), 591/s6, 586/s1 and 587/s2 (the
  same as at Beginner).
- 610/s6: Tab onto a toolbar's button at Advanced drops "Tool bar"; containers in general are
  kept (20 steps regressed). Record a toolbar, a list box and a group at Advanced side by side.

## 3. Research recordings to write

- The Word List (560): Ctrl+Insert+W on a page of known words, its items and order, and W and
  Shift+W.
- Insert+T's "article | level 3 | heading is …" (566/s7): Insert+T outside and inside articles
  and at other depths.
- 567/s12: Down back onto a field just left by Up is silent in JAWS, while 283/s13 reads one;
  both orders on one form.
- 559 and 299, Jump to Line: TypeKey's digits sent the focus back to the page (559/s2). Type
  into JAWS's elevated dialog another way (JAWS's `TypeString`, or the edit's text set through
  FSAPI), and record 559 again.

## 4. Performance and live checks

As the previous plan's §4: the review's open items (P3, P5 steps 2, 4, 5, P6, P7, P11-P15, P2
steps 2-3, P4 steps 2 and 4-6, P9 step 3), and the live checks never made (P2, P4, P5, P8,
the Window List's switch, the secure-screens M2 with Insert+F4).

## 5. Owner

- **The Defender exclusion on old-pc** (the owner's yes): `Add-MpPreference -ExclusionPath
  'C:\Users\ailabs\sr-harness'` from an elevated PowerShell there.
- **The development certificate**, steps of the previous plan's §5:
  1. the agent built the release programs (done);
  2. the owner runs `dev-signing.ps1 -NewCertificate` on the dev box and gives the
     thumbprint;
  3. the agent signs, verifies and pushes;
  4. the owner trusts the `.cer` on old-pc and runs the install, elevated;
  5. the agent checks;
  6. the owner signs out and in, uninstalls and undoes.
- **The owner's own settings file:** `jaws_key` is now `okur_key` (the trademark pass).
- **Office on old-pc**, last; 518, 519 and 521 wait for it.
