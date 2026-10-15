# Pending scenarios (written, not yet recorded)

These scenarios and fixtures were written while `old-pc` was offline (2026-10-15). They wait
here because a scenario in `corpus/scenarios/` without evidence fails the parity test, and the
corpus manifest and `okur parity` read only `corpus/scenarios/` and `corpus/fixtures/`.

To record one:
1. Move it into `corpus/scenarios/`, and its fixture (if it is here) into `corpus/fixtures/`.
2. Follow `docs/references/verification-loop.md` §4.3–4.5: lab-push, lab-pull, record on
   `old-pc`, bring the evidence home, regenerate the manifest and baselines.
3. Commit the scenario, fixture and evidence together in a `corpus:` commit.

| Scenario | What it records | Fixture |
|---|---|---|
| 208 | A web textarea, character by character, with the fixed tracer | `characters.html` |
| 315 | A RichEdit field: lines, and Say Font with paragraph formatting | `desktop-richedit.ps1` (here) |
| 316 | Win+M (show the desktop) | `desktop-editor.ps1` |
| 317 | Alt held while Tab is pressed three times (task switcher) | `desktop-dialogs.ps1` |
| 318 | Virtualize Window (Insert+Alt+W) in a page | `news-article.html` |
| 319 | The Run dialog: its announcement and Virtualize Window | `desktop-editor.ps1` |
| 320 | Notepad, laptop layout: CapsLock+I, K, O, L, J, M, U, Insert+Up, Ctrl+Right | `desktop-notepad.ps1` |
| 321 | Virtualize Window in a console: opening line, Insert+T ("Virtual Viewer"), first and last lines, Escape | `desktop-console.ps1` (here) |

The plan `docs/plans/2026-10-15-jaws-feature-loop-round-2.md` (work item 1) lists what to
check in each recording.
