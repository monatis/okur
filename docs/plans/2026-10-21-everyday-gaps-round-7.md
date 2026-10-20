# Plan: everyday-use gaps, round 7 (2026-10-21)

Follows [`../handoff/2026-10-20-everyday-gaps-round-7.md`](../handoff/2026-10-20-everyday-gaps-round-7.md).
The list stays [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md):
the gaps are closed before macOS. Each item is found in JAWS's documentation, recorded on
old-pc, then built; JAWS's accidents become accepted divergences. old-pc always has a round
queued behind the running one. Okur's own words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with `okur parity`,
then a dated handoff and the next plan, with AGENTS.md pointing at them. A stretch is a few
hours or about thirty commits, not more.

## 0. What old-pc records meanwhile

| Round | What | Then |
|---|---|---|
| 99 (done, `f93051f`) | 636-641: 624, 625 and 632 at Intermediate and Advanced | Insert+T's short messages built (`2bc4130`) |
| 100 (done, `cc97db7`) | 642: Keyboard Help's second press 200 to 1200 ms later | §2 |
| 101 (done, not imported) | 643: selection counts to the top from six places; its third trial failed | §1: every count to the top agrees with Okur's |
| 102 (running) | 644-649: 629-631 at Intermediate and Advanced | §3 |
| 103 | 650-653: Ctrl+A on four pages with different endings; 643 again | §1: which pages get JAWS's one more character |
| 104 | 654: Say Line pressed twice at 250 to 450 ms | §2: JAWS's double-press window |
| perf105 | 541, 524, 540 and 323 with `-TraceTimings` and push `a0a31ed9` (P6 step 2); 613 with each read's start logged | §4; 613's hang |

For each: fetch with `lab-fetch.sh`, check every trial passed and the capture script's hash
against a committed version, check each scenario's hash against the committed file, grep the
evidence and traces for links, the lab account's folders and the owner's name, compare parity
before and after, and commit the corpus alone, with the baseline from the committed code.
Before queueing more, check `Get-MpThreatDetection` on old-pc.

## 1. JAWS's text around inline links (635)

What 635 showed, and what Okur does instead:

- JAWS keeps a text run's space next to a link that breaks the line ("Gazette. " before Contact
  us, "New " before study rooms, " open in May." after it, "By " before Maria Lopez). Okur trims
  them.
- A list item's bullet is outside its link: Left from the link's first letter gives a space,
  then "•". Okur's link span starts at the bullet, so Shift+Tab puts the cursor there.
- End goes past a line's last letter, silently; Left then says that letter. Okur's End stops on
  the last letter.
- The page's text ends with a line break after its last line. Okur's ends without one.

Built (`e6c5670`): the spaces, the bullet and End; 635 matches every step. Left: the page's
end. Counting a final line break in every selection to the end fixed 562 and 515 but broke 617
and 620, whose pages end with a paragraph; 643's counts to the top from six places all agree
with Okur's, so the one character is after the news article's last line, "content information
end". Round 103 (Ctrl+A on pages ending with a region, a list, paragraphs and web
applications) says which endings have it; then build that rule.

## 2. Keyboard Help and twice quickly

- **JAWS does not wait for a second press** (642). It says the key at once and its description
  right after, the key's speaking time later (about 120 ms after "Tab", 420 after "Insert plus
  Up Arrow"); a second press flushes the first's description from its queue before the speech
  recorder hears it. Okur's speech is the same, but its spoken record keeps both: accept that
  as a divergence of the recorder, not of what a user hears.
- **The double-press window.** 200 ms apart is twice for JAWS, 400 ms is not (642/s3); Okur's
  is 500 ms (Windows' double-click time). Round 104 (654) narrows it with Say Line; then set
  `DOUBLE_PRESS_MS` from it, for every twice-quickly key.
- The rest of Keyboard Help is built and its words are Okur's (accepted divergence).

## 3. Verbosity

- Round 99 is built: at Intermediate and Advanced Insert+T says "article | 3 | What changes"
  (`2bc4130`). The field return matches but for JAWS's misspelling mark: the traces have no
  spelling marks for "abc", which Chrome sets after the tracer read the field (632/s3, s8;
  640, 641). Record it again with a pause after typing.
- Import round 102 (Tab into forms mode at Intermediate and Advanced); the hints ("Type in
  text.") are the likeliest difference.
- 610/s6, a toolbar at Advanced: as before.

## 4. Performance

- **P6 step 2.** In the Font dialog each arrow in the font list cost 0.82 to 1.0 s (round 97):
  the arrow changes the dialog's values, and the whole control tree (244 nodes) is read again,
  every property of each of the 187 fonts through UI Automation's MSAA proxy. Built
  (`a7d94d5`): a list's, tree's or grid's items are read a level at a time with the twelve
  properties their uses need. Measure with perf105; if the arrows are still slow, read again
  only the controls that changed, or read the tree only when something needs it.
- The rest as before: P5 steps 4 and 5, P7, P11-P15, P2 steps 2-3, P4 steps 2 and 4-6, P9 step 3;
  the live checks never made (P2, P3, P4, P5, P8, the Window List's switch, the secure-screens
  M2 with Insert+F4).

## 5. Carried over

- **The spoken "blank"** for an empty first line in a large selection's last 20 characters
  (620/s10-s35): a speech-only utterance.
- **The clipboard keys** (627 was inconclusive): Say Clipboard, View Clipboard Text, Append to
  Clipboard. Windows+Insert+X reached Windows' own Win+X menu from the harness's keys; record
  again with `okur-lab-keys` once the owner has installed it.
- **The layer's help and Command Search** (628 was inconclusive on the Turkish layout).
- **613's tracer hang:** read round 92's log (`perf92/613-web-turkish-text-run.log`), fix, and
  record 613 again.
- **Notifications:** the banner's UI Automation window or Windows' notification listener; one
  recording with Focus Assist off, if the owner agrees.
- **Say Color** outside pages, and the Turkish color table.

## 6. Owner

- **The development certificate on old-pc:** trust the `.cer` (`dev-signing.ps1 -TrustOnLab`,
  elevated), then `okur-setup.exe install --from C:\Users\ailabs\sr-harness\okur-signed
  --no-start --no-start-after-sign-in`; the agent then checks M3. Then copy `okur-lab-keys.exe`
  to `C:\Program Files\Okur Lab`.
- **Focus Assist off on old-pc**, for one notification recording, if the owner agrees.
- **The Defender exclusion on old-pc**, as before.
- **Office on old-pc**, last; 518, 519 and 521 wait for it.
