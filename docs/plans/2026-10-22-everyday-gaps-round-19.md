# Plan: everyday-use gaps, round 19 (2026-10-22)

Follows [`../handoff/2026-10-22-everyday-gaps-round-19.md`](../handoff/2026-10-22-everyday-gaps-round-19.md).
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

- old-pc stopped answering OpenDesk (2026-10-22, about 04:30). When it answers: deploy the
  build of `823ec70` (`lab-deploy.sh`), then queue a round.
- **Mouse echo live**: 794's keys (the `mouse` steps, echo on, then the units) with Okur reading,
  one trial, `-OkurDebugLog`: Okur should say the line, word and character the mouse rests on.
  The run is never evidence; it shows that the UIA read at the point works on Windows 10's
  Notepad and in Chrome.

## 1. The mouse and the Okur cursor's screen

- **The mouse following the Okur cursor**: the cursor's character on screen through UI
  Automation (a text range's bounding rectangles; a page node's bounds); a click then lands
  where the Okur cursor is, and Alt+Delete says its place.
- **Traces with bounds**: a dialog's controls with their places; a Win32 edit's line bounds.
- **Mouse echo's remaining settings**: location tones (`MouseEchoScreenLocationTones`) and
  echo while speech is muted (`AllowMouseEchoWhenMuted`), from JAWS's help.

## 2. WPF, as JAWS reads it

- **The tracer reads a closed combo box's choice** through UIA's selection pattern, and a
  group box's header where UIA gives it no name; then 775, 783 and 787 recorded again.
- **"Contains text" and "_Rules"**: whether JAWS's reading of an unnamed edit and of a WPF
  label's access-key mark are accidents.

## 3. The README's next items

- The clipboard keys (627, which needs `okur-lab-keys` signed).
- The rest of verbosity.
- Notifications' leftovers: Shorten and Play Sound, toasts Focus Assist hides
  (UserNotificationListener), applications' own UIA notifications in the history (a recording
  with Windows Settings' search).
- Skimming by attribute and color rules; the Word List's summary (its content, read with Tab in
  JAWS's Results Viewer).

## 4. Unchanged

- 748's opening and "dialog" plain on return; Start reading at on load.
- Settings: the dialog's labels, capitals and line pauses in Say All, live regions and
  indentation, Settings Center.
- Say All progress in replay; the table's header row in skim reading (761/s1); 769/s3's
  byline.

## 5. Owner

- old-pc's connection, if it does not come back by itself.
- Import the development certificate on old-pc (`sr-harness\signing\`), so that
  `okur-lab-keys` can be installed (517, 627).
- Code signing for releases, when the owner chooses.
- Whether JAWS's mouse echo speaks for a hand on old-pc's mouse (the harness's moves got
  nothing), if the owner is ever at that machine.
