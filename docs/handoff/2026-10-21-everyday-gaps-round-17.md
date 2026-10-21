# Handoff: everyday-use gaps, round 17 (2026-10-21)

Follows [`2026-10-21-everyday-gaps-round-16.md`](2026-10-21-everyday-gaps-round-16.md) and covers
the commits after it (`33b75c6` to `cb92608`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-16.md`](../plans/2026-10-21-everyday-gaps-round-16.md).
The next plan is
[`docs/plans/2026-10-21-everyday-gaps-round-17.md`](../plans/2026-10-21-everyday-gaps-round-17.md).

## The owner's answers and requests

None this stretch.

## Decided without the owner (reversible)

- **The Word List's list is named "List1"**, as JAWS's dialog names it (560/s1). Okur's list
  dialogs take JAWS's labels, label for label, and the dialog mirror finds Okur's controls by
  them; Select a Form Field's list is "List1" too.
- **The common words the Word List leaves out are language data**:
  `crates/okur-core/locales/en-US/word-list-common-words.txt`, beside the color names. They
  are the news article's words that JAWS's list lacks, so JAWS's own list may hold more.
- **Skim Reading's rules controls are shown, unavailable.** Okur reads by the first line or the
  first sentence. With text rules or attribute rules chosen, Ctrl+Insert+Down says "Okur cannot
  skim by text, attribute or color rules yet." Create Summary closes the dialog only, until the
  summary's content is known.
- **Skimming by sentence reads a sentence once.** JAWS reads it again for each paragraph that
  starts in it ("list end Primary navigation region end banner end main region article", five
  times, 769/s3). That is left as a difference in 769/s3, with the byline, where JAWS's
  sentence runs across lines and leaves the link's name out.
- **A toast is said after what is being said**, not interrupting it. 770 cannot tell which
  JAWS does: nothing else was speaking.
- **762, 767 and 768 are not imported.** JAWS's Results Viewer gives the recordings nothing,
  whatever keys are sent.

## What was done

### Features (each checked with `okur-lab parity` against old-pc's evidence)

- **The Word List** (`62c4668`, `1622c29`), from 560 and 764-766:
  - Ctrl+Insert+W lists the page's words, most used first: "m (10)";
  - OK moves to the first line with the word, found within other words ("Primary navigation
    region" for "m");
  - W and Shift+W go to the next and prior line with it, whatever its capitals, wrapping with
    "wrapping to top" and "wrapping to bottom";
  - W before a word is chosen says "No word list has been created." and how to make one.
- **Skim Reading's dialog** (`e35b012`), Ctrl+Shift+Insert+Down, from 760 and 763:
  - JAWS's controls and Tab order: Use Rule Type, Skim Reading Indication, Start Skim Reading,
    Create Summary (the default), Close and Help;
  - Escape gives the page back silently;
  - the rule chosen stays for Ctrl+Insert+Down.
- **Skimming by sentence** (`e35b012`, `6a020ff`), from 769:
  - for each paragraph, the sentence its start falls in;
  - a list item's mark with its sentence ("• Weekday hours: 9 a.m."), a link's item by its mark
    alone ("•");
  - a table's row as one paragraph, its sentence going on through its cells ("Central 9 a.m.").
- **Toasts** (`1b6a460`, `4662125`, `cb92608`), from 533, 770 and 773:
  - a toast is said as it shows, by the name the shell gives it ("New notification from Windows
    PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1");
  - Insert+Space, Shift+N says the last one again; before any, "No notification";
  - Insert+Space, Ctrl+N stops and starts saying them ("Toast notification announcements
    disabled", "... enabled"); a toast while they are off is kept for Shift+N.

### Harness

- **`run-scenario.ps1` closes JAWS's viewers before each trial** (`7bad3b6`): the Results Viewer
  left open by 762 answered Escape in 763 (round 180). Trials note it in
  `lab_state_cleared.jaws_viewers_closed`. 763 was recorded again with it (`818bade`).
- **`okur trace` records the shell's windows as they open** (`51748a0`): UIA WindowOpened for
  ShellExperienceHost and Explorer, with the window's subtree, as `uia_window_opened`. That is
  how 770 saw the toast, and how Okur now reads toasts live and in replay.
- **Research** (`4c7c20d`): [`docs/references/notifications-research.md`](../references/notifications-research.md)
  gathers JAWS's documentation of toasts and the Notification History, how Windows and NVDA
  handle toasts, and UserNotificationListener.
- **Pitfalls written down** in `docs/references/verification-loop.md`: JAWS's Results Viewer
  gives the recordings nothing, and `opendesk exec` can hang past its timeout.

### Accepted divergences

- **Escape in Skim Reading makes JAWS's summary** (763/s23, s24): after Shift+Tab and Up Arrow
  on Skim Reading Indication, JAWS's Escape opens the summary, in every trial of two
  recordings. Okur's Escape closes the dialog.
- **JAWS's speech before its own dialogs** now also covers the Word List (560/s1, s4, 764-766/s1)
  and Skim Reading (760/s1, 763/s1, 769/s1, 771/s1): the list, or the rule type, said before
  the dialog.

## Figures

Each is the named command's output at `cb92608`:

- `okur-lab parity`: "JAWS parity (Ia2): 6113/6339 steps match exactly; 160 accepted
  divergences not counted (0 scenarios could not run)". At the previous handoff: 6014/6243
  with 158.
- `okur-lab parity --spoken`: "JAWS spoken parity: 5980/6301 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 428 scenarios, 1320 trials, 1319 successful (770's
  first trial failed: its Notification History came late).
- `cargo test --workspace`: 1319 passed, 0 failed, 199 ignored.

## Learned

- **JAWS saves Skim Reading's rule type** when Start Skim Reading is pressed:
  `[Skim Reading History] LastSkimRead=2:0::|0|0:0` in its user DEFAULT.JCF after 769, and
  the dialog opened on "Read First Sentence of Every Paragraph" in 771. 771 set it back to
  `1`. A scenario that changes it runs one trial and puts it back.
- **Skim reading started from the dialog ends in the summary**, by either rule type (769, 771);
  Ctrl+Insert+Down from the page does not (761). The summary opens in JAWS's Results Viewer
  ("Skim Reading Results"), its first line "1. City Library Expands Evening Hours - Riverside
  Gazette" a link.
- **The Results Viewer cannot be read by our tools**: neither our keys, nor `okur-lab snapshot`,
  nor OpenDesk's UI tree reach it (OpenDesk does not even list it), likely because it runs at
  a higher integrity level. Its content is still unknown.
- **A toast** on Windows 10 is a window of ShellExperienceHost, AutomationId
  `NormalToastView`, named as JAWS says it, with SenderName, TitleText and MessageText inside;
  the shell's CoreWindow ("New notification") opens around it 13 ms later. In 770 the banner
  showed and JAWS said the toast in two trials of three.
- **The Notification History** (774) is JAWS's own window (NotificationViewer): "Notification
  History | Recent Notifications List box | <text> <application> <time> | 1 of 57"; Tab goes to
  Enable Rules (checked), Create Rule..., Clear History, Close; Enter on an item opens
  "Notification Details dialog" with "Notification Text:" in a read-only edit.
- **W in the virtual cursor** finds the word as JAWS Find does: within other words, whatever
  its capitals, and in what the cursor says of lists and regions.

## Recorded meanwhile, and queued

Round 190 (`waiter190`, three trials) is recording 775: the Notification History's Create Rule
dialog, closed with Escape.

## Not done, and why

- **The summaries** (Skim Reading's Create Summary, its end after Start, the Word List's
  Summary): their content is in JAWS's Results Viewer, which our tools cannot read.
- **Okur's own Notification History**: recorded in 774, to build next. Toasts carry their
  sender and their time for it.
- **Skimming by text rules and attribute rules**: not recorded; JAWS saves rules, so each
  recording runs once and cleans up.
- **769/s3's byline and JAWS's repeated sentences**, and **761/s1's table header row**.
- **From plan 16, untouched**: the mouse following the Okur cursor, traces with bounds, the
  mouse button locks, the clipboard keys, the rest of verbosity.
