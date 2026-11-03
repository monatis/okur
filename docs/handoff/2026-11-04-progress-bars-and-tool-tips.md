# Handoff: JAWS's progress bar announcements, and tool tips (2026-11-04)

Follows [`2026-11-04-step-4-done.md`](2026-11-04-step-4-done.md).
Plan: [`../plans/2026-11-03-review-round-2-before-cross-platform.md`](../plans/2026-11-03-review-round-2-before-cross-platform.md),
its §3 (JAWS's features from Settings Center and the help), begun with the everyday-gaps row 33
(progress bars, tool tips). Branch `windows-ui-round-2`; `main` was at `1971ca70`.

## Found first-hand

- **JAWS's defaults** (old-pc's shared `Default.JCF`, `[Options]`): `ProgressBarUpdateInterval=5000`,
  its comment saying 0 announces no update and any other value is the time that must pass
  before the next one; `ProcessToolTipEvent=1`. old-pc's own settings change neither.
- **Settings Center** (research runs on old-pc, not evidence): the setting is "Progress Bar
  Updates Announcement" in its User group, set to "5 Second Intervals", one of seven: Off,
  Announce All, 5 Second Intervals, 10 Second Intervals, 30 Second Intervals, 1 Minute
  Intervals, 5 Minute Intervals. Nothing was saved (the user `DEFAULT.JCF` hash unchanged).
- **The second voice in Okur's live runs is JAWS**: its own recordings carry the same numbered
  bookmark between every word.

## Recorded and imported

| Scenario | What JAWS does | Commit |
|---|---|---|
| 793, a page's progress element and ARIA progressbar moving on one schedule from a button | "10" at once, "40" at 8 s, "100" at 14 s, in all three trials: changes within 5 s of the last one said are never said, nor later; two bars moving together said once; the virtual cursor reads a bar as "100 Progress bar", its label on the line before | `99bd86ea` |
| 1136, a WinForms progress bar (`msctls_progress32`) on the same schedule, the focus on its button | nothing at all over 16 to 17.5 s; Say Window reads "Copying files progress bar 40%" | `99bd86ea` |
| 1137, WinForms tool tips shown below a button as it takes the focus | the buttons only, never their tool tips (WinForms' own window class, not `tooltips_class32`) | `e7647546` |

The first recordings of 1136 settled after 0.8 s: a step whose speech comes late needs its
`onset_ms` (now a pitfall in [`verification-loop.md`](../references/verification-loop.md) §8,
with the Settings Center route).

## Built

`29a4c730`: a page's progress bar changes are `AccessibilityEvent::ProgressChanged`, the bar and
the time the change came (the trace's, so replay keeps its spacing). The engine says the first
at once and none within `progress_bar_interval_ms` (5000 by default, 0 for none; the setting is
new, in the settings file and `--set`) of the last one said, one interval for all bars; the
page's bar takes its new value, and the virtual cursor reads "100 Progress bar". Test:
`a_progress_bars_changes_are_said_at_most_every_interval`.

## Figures

`okur-lab parity` at `e7647546`:

```
JAWS parity (Ia2): 11472/12058 steps match exactly; 370 accepted divergences not counted (0 scenarios could not run)
```

(793/s2 and s4 turned to matches with `29a4c730`, no other step changed; 1136 matches 2 of 3,
1137 2 of 2.) `okur-lab invariants`: `Buffer invariants: 0 violation(s)`. `cargo test
--workspace --no-fail-fast` (4 threads), summed: 1630 passed, 0 failed, 201 ignored.

## Next

1. Done after this handoff's figures (`6292eb7a`): the setting in Settings Center's tree and
   search as Progress Bar Updates Announcement, its seven choices by JAWS's names; Announce All
   writes `ProgressBarUpdateInterval=1` (saved in Settings Center on old-pc, its own file put
   back after, hash checked).
2. Say Window in a window that is not a dialog box: Okur says nothing where JAWS walks the
   whole window, every control by name, lowercase type and state ("SR Desktop Fixture | window
   | menu bar | File | menu item | ... | status bar | Ready") [548/s14, 554/s14, 1136/s3]. It
   needs a window walk on demand in the driver, the walk recorded in traces for replay, and
   JAWS's format.
3. A comctl32 tool tip (`tooltips_class32` itself), shown by a fixture without the mouse, for
   JAWS's tool tip events; then the tracer records tool tips shown, and Okur says them as JAWS
   does.
4. The plan's other §3 leads (Say All's capitals and blank lines, key labels and key echo, the
   quick key delay), and the review's small open items (P3, P6, P10, P13); Word and Excel wait
   for Office on old-pc.
