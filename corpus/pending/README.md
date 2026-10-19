# Pending scenarios (written, not yet recorded)

Scenarios and fixtures wait here until they are recorded, because a scenario in
`corpus/scenarios/` without evidence fails the parity test, and the corpus manifest and
`okur parity` read only `corpus/scenarios/` and `corpus/fixtures/`.

To record one:
1. Move it into `corpus/scenarios/`, and its fixture (if it is here) into `corpus/fixtures/`.
2. Follow `docs/references/verification-loop.md` §4.3–4.5: deploy to `old-pc`, record, bring
   the evidence home, regenerate the manifest and baselines.
3. Commit the scenario, fixture and evidence together in a `corpus:` commit.

| Scenario | What it records | Fixture |
|---|---|---|
| 504 | A native edit: selected text, clipboard keys, Say To and From Cursor, sentence, paragraph | `desktop-editor.ps1` |
| 505 | Say All in a native edit stopped by Ctrl and moved by Right and Left | `desktop-reading.ps1` |
| 508 | A text area's lines with the virtual cursor (Say All reads each with "Edit", 408/s10) | `reading-settings.html` |
| 509 | Keyboard Help (Insert+1), Insert+F1, Insert+H, Insert+W | `desktop-dialogs.ps1` |
| 510 | Laptop layout: sentences, paragraphs, Say To and From Cursor, selected text, top and bottom line, color, character value, Say All | `desktop-reading.ps1` |
| 511 | The Window List (Insert+F10) and the system tray (Insert+F11) | `desktop-dialogs.ps1` |
| 512 | Voice rate on the fly (Ctrl+Alt+PageUp and PageDown), Insert+Escape | `news-article.html` |

Scenarios from 400 record JAWS's reading settings (plan
`docs/plans/2026-10-18-reading-settings.md`). Their fixtures, `reading-settings.html`
and its desktop twin `desktop-reading.ps1` (recorded with 400), hold the 18 lines of
`docs/references/jaws-reading-settings-research.md` §12, the same text in both. A scenario
that needs a setting other than old-pc's names it in `jaws_settings`
(`skills/jaws-reference/scripts/jaws-settings.ps1`), and `run-batch.ps1` sets it.

Scenarios from 500 record the everyday-use gaps of
`docs/backlog/everyday-use-gaps-2026-10-19.md`. A step's `during` presses keys while JAWS
still speaks (Ctrl or Right during Say All).

Every scenario written here before (208, 210–219, 315–323) has been recorded; git history keeps
the notes written with them.
