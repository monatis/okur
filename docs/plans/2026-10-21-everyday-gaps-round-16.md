# Plan: everyday-use gaps, round 16 (2026-10-21)

Follows [`../handoff/2026-10-21-everyday-gaps-round-16.md`](../handoff/2026-10-21-everyday-gaps-round-16.md).
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
| 179 (`waiter179`, 3 trials) | 762: Skim Reading's Create Summary in the Results Viewer | build the summary |

## 1. Skim reading, finishing it

- **Okur's Skim Reading dialog** (Ctrl+Shift+Insert+Down), drawn by okur-ui with JAWS's
  controls (760). It needs:
  - its four rule types;
  - the text rules: a pattern, a regular expression, the unit spoken;
  - Skim Reading Indication;
  - Start Skim Reading, and Create Summary (default);
  - the summary in the Results Viewer, each line a link back to its place (762).
- **Recordings:** another rule type (first sentence; a text rule), to learn what each reads.

## 2. The Okur cursor's screen

- **The mouse follows the Okur cursor.** The cursor's character goes on screen through UI
  Automation (a text range's bounding rectangles in a text field, a page node's bounds). A click
  then lands where the Okur cursor is, and Alt+Delete says its place.
- **The traces:**
  - record a dialog's controls with their places, so that the cursor reads a dialog's rows
    from them;
  - record a Win32 edit's line bounds (748/s17).
- **The mouse keys not yet recorded:** the button locks (Insert with Numpad Slash and Star),
  drag and drop, Ctrl and Shift clicks.

## 3. The README's next items

- The Word List.
- Notifications: toasts and the notification history.
- The clipboard keys (627, which needs `okur-lab-keys` signed).
- The rest of verbosity.

## 4. Unchanged from plan 15

- 748's opening and "dialog" plain on return; Start reading at on load.
- Settings: the dialog's labels, capitals and line pauses in Say All, live regions and
  indentation, Settings Center.
- Say All progress in replay.

## 5. Owner

- Import the development certificate on old-pc (`sr-harness\signing\`), so that
  `okur-lab-keys` can be installed (517, 627).
- Code signing for releases, when the owner chooses.
