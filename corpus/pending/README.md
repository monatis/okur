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
| 509 | Keyboard Help (Insert+1), Insert+F1, Insert+H, Insert+W. Recorded in round 13 and not imported: in Keyboard Help mode JAWS describes the harness's history copy instead of running it, so those steps time out (their synthesizer text is recorded) | `desktop-dialogs.ps1` |
| 518 | Reading a Word document | `desktop-word.ps1` (here) |
| 519 | Word's headings list, spelling errors, revisions | `desktop-word.ps1` (here) |
| 561 | The clipboard keys: SayClipboard, View Clipboard Text, AppendSelectedTextToClipboard (the Windows-key scripts called by name). Round 59 failed: Escape did not close the "Clipboard Text" window View Clipboard Text opens, which then stayed open across trials, and a second View Clipboard Text asked "Overwrite Clipboard Text Viewer" (Yes, No). Needs another way to close that window before it is recorded again | `desktop-editor.ps1` |
| 593-612 | Ten more recorded scenarios at Intermediate (593-602) and Advanced (603-612) verbosity, generated from 216, 228, 283, 289, 302, 501, 504, 524, 526 and 540 with their steps unchanged | as their sources |
| 613 | A Turkish page (`lang="tr"`, ç ğ ı İ ö ş ü) read by character, word and line, spelled, and with Say All, in JAWS's English voice (old-pc's language detection is off) | `turkish-text.html` (here) |
| 614 | Insert+Tab with JAWS's text analysis on a second Notepad fixture: how far brackets pair, which symbols, numbers, spaces and cases it names (568 left open whether parentheses are paired across lines or not checked) | `desktop-proofing-2.ps1` (here) |
| 615, 616 | A table's edges at Beginner and Intermediate: Ctrl+Alt+Left on the first column ("edge of table" in Okur, never recorded), the first row and column by row and column, and past the last cell | `news-article.html` |

Office on old-pc is not activated (2026-10-19): Word and Excel open Microsoft's Office Activation
Wizard and say "Product Activation Failed", which held the fixtures' automation in round 17.
518, 519 and 521 wait for the owner to activate Office there.

A Windows key chord (Win+Insert+X, Insert+Win+C) goes to Windows on old-pc: Win+X opens
Windows' own menu (520/s3), and the Windows key released alone opens the Start menu, which
spoiled 529 and 530 in round 25. `run-scenario.ps1` now closes such a menu before a trial.

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
