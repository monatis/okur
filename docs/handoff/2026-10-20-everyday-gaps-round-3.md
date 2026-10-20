# Handoff: everyday-use gaps, round 3 (2026-10-20)

Follows [`2026-10-20-everyday-gaps-round-2.md`](2026-10-20-everyday-gaps-round-2.md). The work
list is still [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md);
the plan is [`docs/plans/2026-10-21-everyday-gaps-round-3.md`](../plans/2026-10-21-everyday-gaps-round-3.md),
whose §0 table lists every round queued on old-pc and what to do with it.

## Decided without the owner (reversible)

- **Select an Article's Enter is a JAWS accident** (`c6e4ae8`): in every trial of round 54
  JAWS went to the page's top, not to the article [253/s3-s5]. An accepted divergence; Okur
  moves to the article.
- **The tracer's address redaction and the news article's fixture**: the fixture writes
  `help@riverside-library.org`, not an example domain, so traces redact it. Rather than record
  every news-article scenario again with a new fixture, replay redacts JAWS's transcript the
  same way where Okur read a redacted address (`b061649`).
- **The JAWS cursor reads nothing yet**: Okur has no model of the screen's text. Its reading
  keys say nothing with the JAWS cursor active in an application, which is also what JAWS
  said in the recorded WinForms dialog [516/s4-s6].

## What was done

### Features (each checked with `okur parity` against old-pc's evidence)

- **Element start and end** (`49e0820`): Shift+Comma, Shift+Period, Alt+Windows+Home and End,
  the innermost list, table, region, block quote, article, named group or dialog [294].
- **F8** (`19a75cd`) selects the element at the virtual cursor [515/s5, s6].
- **Slash and Equals** (`a83298d`): clickable elements ("There are no clickable elements on
  this page.", as JAWS in Chrome) and aria-flowto ("No more flows found") [293].
- **aria-flowto** (`9394fc1`): the IA2 capture reads relations; an element that flows says
  "Has Flows To", Equals and Shift+Equals follow it [292/s9 once the snapshot is captured
  again].
- **Property sheet pages** (`8361961`): Ctrl+Tab says "General Page" before the control
  [323/s11].
- **Say Line on the focused field** (`66a555a`): with its label, without "blank" [507/s7,
  249/s4].
- **Numpad Slash on a radio button** (`69fd9a1`) without its position [534/s9].
- **Ctrl+Insert+V** (`378283d`): the application's product and version from its executable
  [513/s4 once traced again].
- **Win32 dialogs' access keys from MSAA** (`ce363c0`): Insert+Tab ends with "Alt+r" [541/s8,
  543/s2 once traced again].
- **The JAWS cursor and the PC cursor** (`c5c2ba9`): Numpad Minus, Numpad Plus, Insert+Numpad
  Minus (moves the mouse), Alt+Delete; the tracer records the focus's rectangle and the mouse
  pointer [516, 513/s2; the coordinates once traced again].
- **Live fix** (`d30fea1`): after Enter in the Regions list Okur stayed out of the virtual
  cursor (live11, 256/s4): a focus from Okur's closing dialog had no window class.
- **The IA2 capture keeps text colors** (`087bf74`), for Say Color (565 records JAWS's names).

### Harness and replay

- run-batch.ps1 keeps a failed scenario's last 40 lines in `<id>-run.log` (`775279d`).
- JAWS's speech before its own dialog is left out on the spoken layer too (`eb2b7a4`).
- A JAWS dialog's first focus reported from JAWS's main window is replayed (`732b265`); the
  dialog mirror compares items with single spaces (`28fb6a0`) and leaves out JAWS's dialog
  taking the focus as it closes (`08c751a`).
- `okur parity --only <id prefix>` (`42008d3`).

### Recorded and imported

- Round 51 (201-219 in the lab's Chrome, `efa7430`) and round 54 (246-274 with the elevated
  tracer, `7409516`).
- live11 (Okur's own list dialogs live, 222-260 and 274): `okur live-parity` over its runs
  gave 263/294 steps, spoken 254/294, before `eb2b7a4`; 268/294 and 268/294 after it.

## Numbers

`cargo run -p okur -- parity` at `33c84ff`, as round 2's handoff quotes it, and at `c5c2ba9`:

```
JAWS spoken parity: 2286/2603 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 2388/2655 steps match exactly; 60 accepted divergences not counted (0 scenarios could not run)
```

```
JAWS spoken parity: 2361/2600 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 2463/2652 steps match exactly; 63 accepted divergences not counted (0 scenarios could not run)
```

The round-2 figures are the round 2 handoff's; the three steps fewer are 253/s3-s5, now
accepted divergences.

## old-pc when this was written

Round 55 finished (not fetched yet); round 56 runs with `okur.exe` 06ec2c92 (built at
`378283d`: MSAA access keys, application versions); round 57 installs 07766e9e (built at
`c5c2ba9`: also the focus's rectangle and `mouse_move` events, for 513 and 516); then 58,
59 and the snapshot job snap60, as the plan's §0 table says.

## Not done, and why

- Speech On Demand, the clipboard keys, a large selection's "N characters up to", Say Color's
  names, S and D, Jump to Line and the Word List: each waits for its recording (566, 561, 562,
  565, 558, 559, 560 in round 59).
- The verbosity levels: wait for round 58 (548-555).
- 246/s3 and 260/s3 (Enter on a radio button or tab from the lists): JAWS's two variants say
  the group in ways not yet understood. 241/s2's trailing "Banner": not explained by the trace.
- 323/s10: the Security page's text, different in each trial.
