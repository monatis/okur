# Plan: everyday-use gaps, round 25 (2026-10-23)

Follows [`../handoff/2026-10-23-everyday-gaps-round-25.md`](../handoff/2026-10-23-everyday-gaps-round-25.md).
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

Rounds 269 to 273 are queued; preview each as it records (`preview-round.sh`), import it, and
build what it shows. Queue the next round before the last one ends.

## 1. What the queued rounds show

- **Reading Interrupt** (269): build `reading_interrupt` from what a bound key does during a
  short page's Say All and during one long line with it off.
- **WordPad's ribbon** (270): from the MSAA objects traced, the selected tab on Alt ("Upper
  Ribbon", the ribbon's containers, "Ribbon tabs"), the tab's true place among the tabs (JAWS's
  lags one key behind: an accident), "Leaving ribbons", the lower ribbon. Office's ribbon is the
  same NetUI window.
- **A selection's caret** (271): which character and word Say Character and Say Word give after
  selections both ways; then read a Win32 edit's caret from its own caret, not `EM_GETSEL`'s
  start. Insert+R's short messages.
- **Controls in the buffer's text** (272): the role and state words of buttons, check boxes,
  radio buttons, combo boxes and groups out of what the character and word keys move through,
  as JAWS's buffer has them; and the page's edges.
- **The console** (273): 962-964 compared, then Windows Terminal's output (its own UI Automation
  events) and output past one screen.

## 2. README's Not yet

- JAWS's Configure Verbosity Levels dialog (customization map row 72): its controls from JAWS's
  help, recorded, then Okur's dialog over `OutputType` and the message lengths.
- Startup (M3): the installed Okur's checks on old-pc (`okur-ui`'s lowered token, an elevated
  Notepad read through it, uninstall), in a gap between rounds.

## 3. Smart Navigation

Record Screen Layout (`[HTML] DocumentPresentationMode=1`) on its own, then Smart Navigation's
units and table arrows over it; build together.

## 4. The performance review

P4 (a typed character's full read) with a typing trace; C15 in a live log; T1 when a measurement
shows the lock wait.

## 5. Owner

- Office on old-pc is not activated (518, 519, 521 wait for it).
- Code signing for releases, when the owner chooses.
