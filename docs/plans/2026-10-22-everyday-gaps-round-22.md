# Plan: everyday-use gaps, round 22 (2026-10-22)

Follows [`../handoff/2026-10-22-everyday-gaps-round-22.md`](../handoff/2026-10-22-everyday-gaps-round-22.md).
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

- old-pc is idle: queue a round first. Its JAWS voice profile is back at Rate=90; check the
  VPF's `Rate=` lines before recording after anyone slows JAWS (memory: old-pc voice rate).
- Use the `opendesk exec` command line with `timeout`, not the MCP tool, for long commands.

## 1. The README's next items

- **The clipboard keys' edges**: Say Clipboard on an empty clipboard, Append with nothing
  selected, Append in a page (the virtual cursor's selection), No in the Overwrite question,
  Ctrl+X's question; and verbosity's wording of "Appended selection to clipboard". Steps that
  touch the clipboard read no history (`"history": false`).
- **Toasts Focus Assist hides** (533): research first, leaving old-pc's notification settings
  alone.
- **The rest of verbosity**, as recordings show it.
- **The Okur cursor where the screen has no text**, after these.

## 2. UI Automation notifications

- A WinForms window's notifications reach no plain client (`okur-lab probe-notifications`).
  Next: a probe with UI Access (a subcommand of the signed `okur-lab-keys`, or a signed probe
  of its own), installed by the owner under Program Files; then a Chromium page's
  notifications (`ariaNotify`), and Teams if the owner uses it.

## 3. Unchanged

- The rule text field's "edit | Contains text" in JAWS's notification dialogs (775/s7 and
  others): decide whether Okur's own field is multi-line or the difference accepted.
- 748's opening and "dialog" plain on return; Start reading at on load.
- Settings: the dialog's labels, capitals and line pauses in Say All, live regions and
  indentation, Settings Center.

## 4. Owner

- Office on old-pc is not activated (518, 519, 521 wait for it).
- Code signing for releases, when the owner chooses.
