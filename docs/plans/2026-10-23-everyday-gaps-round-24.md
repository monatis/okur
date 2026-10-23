# Plan: everyday-use gaps, round 24 (2026-10-23)

Follows [`../handoff/2026-10-23-everyday-gaps-round-24.md`](../handoff/2026-10-23-everyday-gaps-round-24.md).
The owner's order stands: the README's "Not yet" list, in its order, and Okur's settings made
as customizable as JAWS's
([`../references/jaws-customization-map.md`](../references/jaws-customization-map.md)). Each item is
found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents become accepted
divergences. old-pc always has a round queued behind the running one. Okur's own words never
use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with
`okur-lab parity`, then a dated handoff and the next plan, with AGENTS.md pointing at them. A
stretch is a few hours or about thirty commits, not more.

## 0. old-pc

- Round 267 (918-953) and experiment 11 are queued; preview the round as it records
  (`preview-round.sh`), import it, and build what it shows.
- After any owner session on old-pc, check the voice profile's `Rate=` lines (`fix-rate.ps1`).

## 1. Verbosity, to finish

- What round 267 shows (dialogs, the Okur cursor, toasts, the Notification History, selections,
  Find, the form fields list at both levels).
- Then the README's "Not yet" line can go if nothing else differs.

## 2. Settings from round 264

- **Reading Interrupt** (`[Options] ReadingInterrupt`): with it off, a bound key during a Say
  All is answered and the reading goes on (917); build the setting, then record 917 again with
  a shorter page so that a trial settles.
- **Screen Echo** (Insert+S) and **Smart Navigation** (Insert+X): their words are recorded
  (913); what they change is not built (Okur has no screen model; Smart Navigation's layout of
  controls on a line is to research).
- **Typing Interrupt**: design a recording that settles (a desktop field, a long line typed
  over during speech).

## 3. 820 and the notifications

- Experiment 11's result: if the control's own provider hears the notification, register the
  tracer's handler on the focused control's own element too; else 820 stays a known gap.
- Toasts Focus Assist hides: done, not a gap (533 re-read: JAWS neither says nor keeps them).

## 4. Startup (M3, then M4)

- The installed Okur: `okur-ui`'s lowered token, an elevated Notepad read through it (a live run
  with the installed reader), and uninstall.

## 5. The performance review's open items

- P4 (a full read per typed character), P5's capture timing, C2 for labels and Flexible Web,
  T1 when a measurement shows the lock wait.

## 6. Owner

- Office on old-pc is not activated (518, 519, 521 wait for it).
- Code signing for releases, when the owner chooses.
