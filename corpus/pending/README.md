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
| 509 | Keyboard Help (Insert+1), Insert+F1, Insert+H, Insert+W. Recorded in round 13 and not imported: in Keyboard Help mode JAWS describes the harness's history copy instead of running it, so those steps time out (their synthesizer text is recorded) | `desktop-dialogs.ps1` |
| 510 | Laptop layout: sentences, paragraphs, Say To and From Cursor, selected text, top and bottom line, color, character value, Say All | `desktop-reading.ps1` |
| 511 | The Window List (Insert+F10) and the system tray (Insert+F11) | `desktop-dialogs.ps1` |
| 512 | Voice rate on the fly (Ctrl+Alt+PageUp and PageDown), Insert+Escape | `news-article.html` |
| 513 | Default button, active cursor, color, version, configuration name | `desktop-dialogs.ps1` |
| 514 | A list view's columns (Ctrl+Insert+1 to 3), reading keys on an item | `desktop-forms.ps1` |
| 515 | Ctrl+A with nothing selected, F8, skim reading, element information | `news-article.html` |
| 516 | The JAWS cursor: Numpad Minus and Plus, Alt+Delete, routing | `desktop-dialogs.ps1` |
| 517 | The Insert+Space layer: Speech History, speech off and on (last in its batch) | `news-article.html` |
| 518 | Reading a Word document | `desktop-word.ps1` (here) |
| 519 | Word's headings list, spelling errors, revisions | `desktop-word.ps1` (here) |
| 520 | Say the clipboard (Insert+Win+X) | `desktop-editor.ps1` |

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
