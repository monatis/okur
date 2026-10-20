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
| 562 | Selections of growing size, each copied: where JAWS says "selected N characters up to" instead of the text, and how it counts. Round 59 failed because 561's "Overwrite Clipboard Text Viewer" question was still open; round 70 because JAWS read the selection from the h3 to the end for longer than a step's 10 seconds. Its Ctrl+Shift+End steps now wait up to 90 seconds | `news-article.html` |
| 568 | JAWS's text analysis: Insert+Tab and Insert+Up on Notepad lines holding one inconsistency each | `desktop-proofing.ps1` (new) |
| 569 | Route PC to Virtual on a heading, a paragraph, a list, a list item, a graphic and a table cell | `news-article.html` |
| 570 | The Speech History's other layer keys (`Default.JKM`): Insert+Space, Ctrl+H copies it to the clipboard, Shift+H clears it, then H shows what was said since. Round 70 failed only at Ctrl+H, whose own "Copy speech history to clipboard" the harness took for a leaked copy; the step now says it copies the history (`copies_speech_history`, run-scenario.ps1) | `news-article.html` |
| 571 | Insert+F4 (ShutDownJAWS) asks before quitting; Tab in its question, Escape answers no, then Insert+Tab in Notepad. Last in its batch | `desktop-notepad.ps1` |
| 572-591 | Ten recorded scenarios again at JAWS's Intermediate (572-581) and Advanced (582-591) verbosity, `Verbosity` in DEFAULT.JCF's Options, generated from 101, 204, 205, 222, 230, 241, 297, 303, 304 and 306 with their steps unchanged: what each level changes across reading, forms, lists, regions, tables, trees and dialogs | as their sources |

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
