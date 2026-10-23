# Plan: everyday-use gaps, round 23 (2026-10-23)

Follows [`../handoff/2026-10-23-everyday-gaps-round-23.md`](../handoff/2026-10-23-everyday-gaps-round-23.md).
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

- Use the `opendesk exec` command line with `timeout`, not the MCP tool, for long commands.
- Check the queue first: round 261 (859-864, the clipboard keys and the Speech History at
  Intermediate and Advanced verbosity) runs after live run 260.
- Import round 261 and build what its steps show.

## 1. The UI Automation notification probe

- Once the owner has run `sr-harness\signing\install-notify-probe.ps1` (elevated, on old-pc),
  run the probe beside the WinForms window of 820 through ShellExecute (`Start-Process`
  without redirection), with `notify-timer.ps1`, and read its file: whether a client with UI
  Access hears the notifications a plain client does not. Then decide 820's path.

## 2. The performance review's open items

[`../backlog/performance-review-2026-10-23.md`](../backlog/performance-review-2026-10-23.md):
- **T1**, the engine in the keyboard hook: step 1, the hook deciding from a published
  snapshot of the key owner and its chords.
- **P3 polls** and **P4**: a trace of typing in Notepad, WordPad and a WinForms field on
  old-pc (`okur-lab trace --debug-log`), to see what each key raises and what a cheap poll
  can compare.
- **P5**: the quick page capture off the WinEvent thread, onto the page worker.
- **C2** for labels and Flexible Web rules: applied while the page is built off the engine.
- **C10**: S, D and N's stops kept per buffer version.
- Live check of every platform change with a before and after run (`live260` is the
  pattern: `okur-lab live-parity` and `okur-lab latency` on each build's run).

## 3. The README's next items

- **The rest of verbosity**: what rounds 259 and 261 showed, built; what is left listed.
- **Toasts Focus Assist hides** (533): research first, leaving old-pc's notification settings
  alone.
- **The Okur cursor where the screen has no text**, after these.

## 4. Owner

- Office on old-pc is not activated (518, 519, 521 wait for it).
- Code signing for releases, when the owner chooses.
