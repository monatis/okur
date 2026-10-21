# Plan: everyday-use gaps, round 15 (2026-10-21)

Follows [`../handoff/2026-10-21-everyday-gaps-round-15.md`](../handoff/2026-10-21-everyday-gaps-round-15.md).
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
| 173 (`waiter173`, 3 trials each) | 752, 753: the JAWS cursor in Notepad and on a page | design the Okur cursor's reading from what JAWS reads there |

A scenario that needs a Flexible Web rule names the one it `continues`. That one runs just
before it in the same batch.

## 1. Flexible Web, finishing it

- **From 751:**
  - F2 renames a rule: okur-ui edits the list item's label and reports the new name; the
    mirror reports it from the traced rename;
  - Right Arrow's "open";
  - Space's row, "checked", row.
- **Start reading at on load**, as the guide describes, as an accepted divergence where JAWS
  reads from the top (750/s22).
- **748's opening:** the dialog's text read as JAWS reads it; "dialog" plain on returning from
  the question (as JAWS's wizard says it on a page turn).
- **The harness:** the line bounds of a Win32 edit in the traces (EM_LINEFROMCHAR,
  EM_LINEINDEX), so that replay knows the line JAWS reads in a read-only note (748/s17).
- **A live check of Okur on old-pc:**
  - the wizard's save page;
  - the question as a page loads;
  - the rules dialog, with `TypeString` typing for Okur.

## 2. The Okur cursor outside text fields

- From 752 and 753 (round 173), and JAWS's help on the JAWS cursor (its reading keys, Insert+R,
  Numpad Slash and Star as mouse buttons):
  - read the screen at the mouse through UI Automation;
  - the Okur cursor's arrows by line, word and character;
  - Page Up and Down, Home and End.
- Then the rest of the README's "Not yet" list, in order:
  - the clipboard keys (627, which needs `okur-lab-keys` signed);
  - the Word List and skim reading;
  - notifications;
  - the rest of verbosity.

## 3. Settings, unchanged from plan 14

- The settings dialog's controls for Text Processing, same page links, graphics and Heading
  Indication, once JAWS's labels are found.
- Capitals in Say All and Say All's line pauses (718, 719).
- Live regions on or off, and indentation, recorded again.
- The remaining output types of the shared `Default.JCF`.
- Settings Center and Quick Settings.

## 4. Harness, unchanged

- Say All progress in replay (507, 538, 539, 712/s2, 750/s23).

## 5. Owner

- Import the development certificate on old-pc (`sr-harness\signing\`), so that
  `okur-lab-keys` can be installed (517, 627).
- Code signing for releases, when the owner chooses.
