# Plan: everyday-use gaps, round 17 (2026-10-21)

Follows [`../handoff/2026-10-21-everyday-gaps-round-17.md`](../handoff/2026-10-21-everyday-gaps-round-17.md).
The owner's order stands: the README's "Not yet" list, in its order, and Okur's settings made
as customizable as JAWS's
([`../references/jaws-customization-map.md`](../references/jaws-customization-map.md)). Each item is
found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents become accepted
divergences. old-pc always has a round queued behind the running one. Okur's own words never
use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with
`okur-lab parity`, then a dated handoff and the next plan, with AGENTS.md pointing at them. A
stretch is a few hours or about thirty commits, not more. A feature that draws a window is also
checked live on old-pc (`run-scenario.ps1 -Reader okur`, then `okur-lab live-parity`).

## 0. What old-pc records meanwhile

| Round | What | Then |
|---|---|---|
| 190 (`waiter190`, 3 trials) | 775: the Notification History's Create Rule dialog | its controls, for Okur's |

## 1. Notifications, finishing them

- **Okur's Notification History** (Insert+Space, N), drawn by okur-ui with JAWS's controls
  (774):
  - the list "Recent Notifications", each row the text, the sender and the time ("New
    notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1
    Windows PowerShell 11:07 PM"), the latest first, the past 24 hours, at most 500;
  - Enable Rules (checked), Create Rule..., Clear History, Close, in that Tab order;
  - Enter on a row: Notification Details, the text in a read-only edit "Notification Text:".
- **Toasts carry their sender and time** from the trace's SenderName and the moment they show.
- **The rules** (775 first): what Create Rule asks; Manage Rules; then a rule's actions (say it
  whole, shorter, nothing, a sound).
- **Applications' own UIA notifications**: whether JAWS keeps them in the history (a recording
  with Windows Settings' search, which raises them).

## 2. Skim reading's summary

- **Read JAWS's help** on old-pc: the Skim Reading topics of `JFW.chm` (public documentation),
  for what the summary holds and what its links do.
- **A way to read JAWS's Results Viewer**: our tools cannot reach it. Find out why (its
  integrity level) and whether a capture run with the same rights is the owner's to allow.
- Then the summary in Okur's own viewer: Create Summary, the end of a skim started from the
  dialog, and the Word List's Summary.
- **Text rules and attribute rules**: recordings of one trial each that delete what they make.

## 3. Unchanged from plan 16

- The mouse following the Okur cursor; traces with a dialog's places and a Win32 edit's line
  bounds; the mouse button locks, drag and drop, Ctrl and Shift clicks.
- The clipboard keys (627, which needs `okur-lab-keys` signed); the rest of verbosity.
- 748's opening and "dialog" plain on return; Start reading at on load.
- Settings: the dialog's labels, capitals and line pauses in Say All, live regions and
  indentation, Settings Center.
- Say All progress in replay; the table's header row in skim reading (761/s1).

## 4. Owner

- Import the development certificate on old-pc (`sr-harness\signing\`), so that
  `okur-lab-keys` can be installed (517, 627).
- Code signing for releases, when the owner chooses.
