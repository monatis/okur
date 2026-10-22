# Plan: everyday-use gaps, round 20 (2026-10-22)

Follows [`../handoff/2026-10-22-everyday-gaps-round-20.md`](../handoff/2026-10-22-everyday-gaps-round-20.md).
The owner's order stands: the README's "Not yet" list, in its order, and Okur's settings made
as customizable as JAWS's
([`../references/jaws-customization-map.md`](../references/jaws-customization-map.md)). Each item is
found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents become accepted
divergences. old-pc always has a round queued behind the running one. Okur's own words never
use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with
`okur-lab parity`, then a dated handoff and the next plan, with AGENTS.md pointing at them. A
stretch is a few hours or about thirty commits, not more. A feature that draws a window or
reads the screen live is also checked live on old-pc (`run-scenario.ps1 -Reader okur`, then
`okur-lab live-parity`).

## 0. old-pc

- Round 222 (804, skimming by Underline, which should find the page's links) and round 223 (800,
  the first-line rule set back) are queued: import them, and build what 804 shows.
- Live: 794's keys again with Okur reading, on the build with the word and character found in
  the line (the last live run, 214, was before it); deploy okur.exe through Dropbox (a direct
  push of it timed out).
- If Dropbox lags after a restart, push a round's few files with `opendesk push` (round 218).

## 1. The README's next items

- **Play Sound's Select Sound...**: Windows' Open dialog, as JAWS's opens it (802): Okur's
  window process shows the system's file dialog and hands the path chosen back.
- **Skim reading's colors**: Text Color and Background Color, by JAWS's color names (Say Color
  already names them); the attributes Okur cannot read yet (strikeout, superscript and the
  rest) through IA2 text attributes.
- **The Word List's summary**: its content, read with Tab in JAWS's Results Viewer.
- The clipboard keys (627, needs `okur-lab-keys` signed), the rest of verbosity, toasts Focus
  Assist hides, applications' own UIA notifications in the history.

## 2. Okur's own dialogs, as JAWS's are read

From 787 and 798, replayed through Okur's own dialogs since round 19:

- After Disable in Manage Rules, JAWS's list takes the focus back and reads the renamed row
  ("... Disabled | 1 of 1", 787/s16): Okur's window needs a message to move its focus.
- A dialog over a dialog is named with its owner ("Manage Notification Rules | Confirm Delete
  dialog", 787/s17), and closing one says the dialog under it again (787/s19).
- An empty list says "List box" and its hint (787/s18).
- JAWS's "Contains text" for the rule's unnamed text field (801's SayLine shows what it holds)
  and its "_Rules" (a WPF label's access-key mark read as text): judge as accidents.

## 3. Unchanged

- 748's opening and "dialog" plain on return; Start reading at on load.
- Settings: the dialog's labels, capitals and line pauses in Say All, live regions and
  indentation, Settings Center.
- Say All progress in replay; the table's header row in skim reading (761/s1); 769/s3's
  byline.

## 4. Owner

- Import the development certificate on old-pc (`sr-harness\signing\`), so that
  `okur-lab-keys` can be installed (517, 627).
- Code signing for releases, when the owner chooses.
- Whether JAWS's mouse echo speaks for a hand on old-pc's mouse (the harness's moves got
  nothing), if the owner is ever at that machine.
