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
| 99 (running) | 636-641: 624, 625 and 632 at Intermediate and Advanced verbosity | import; build what differs by verbosity |
| 100 | 642: Keyboard Help's second press at growing intervals (Insert+Up, then Insert+Up again 200, 400, 600 and 800 ms later, as a step's `during` keys) | §2: how long JAWS waits |
| 101 | 629-631 at Intermediate and Advanced (the hints on Tab are verbosity's) | §3 |
| 102 | 541 and 524 again with `-TraceTimings`, after P6 step 2 | §4 |

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

Build each in the linearizer and the cursor, then check 562/s13, s17, s20 and 515/s1 (the large
selection's count) and 635. Run the whole corpus after each: Say Line, word and character
navigation, and selections all read these spaces.

## 2. Keyboard Help

- **The wait for a second press** (634/s3). JAWS says nothing for the first press of a key that
  has a twice-pressed meaning; it says the key and the long description once the second comes,
  and the short one only when none comes. Okur describes both presses.
  - This needs a timer: the engine has none. Design one first (the host calls the engine after
    a delay it asks for), then use it here.
  - 642 tells how long JAWS waits.
- The rest of Keyboard Help is built and its words are Okur's (accepted divergence).

## 3. Verbosity

- Import round 99 (Insert+T and the field return at Intermediate and Advanced) and round 101
  (Tab into forms mode). Build what differs; the hints ("Type in text.") are the likeliest.
- 610/s6, a toolbar at Advanced: as before.

## 4. Performance

- **P6 step 2.** In the Font dialog each arrow in the font list costs 0.82 to 1.0 s (round 97):
  the arrow changes the dialog's values, and the whole control tree (244 nodes) is read again.
  Two ways:
  - read again only the controls that changed (the event's control and its combo box), keeping
    the rest of the read;
  - or read the tree only when something needs it (Say Window, a label, a list's name), not on
    every focus.
  Measure with round 102.
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
