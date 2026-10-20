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
| 548, 549, 554 | The forms fixture at Intermediate, Advanced and Beginner verbosity (JAWS's `Verbosity` in DEFAULT.JCF's Options, set for the recording) | `desktop-forms.ps1` |
| 550, 551 | A message box at Intermediate and Advanced verbosity | `desktop-message-box.ps1` |
| 552, 553, 555 | The sign-up form at Intermediate, Advanced and Beginner verbosity | `signup-form.html` |
| 556 | Shift+Period and Shift+Comma across a news article: regions, the article, a nested list (twice at its end), a table cell, a block quote, the footer | `news-article.html` |
| 557 | The same keys where no element is around the text | `doc-reading.html` |
| 558 | S and D, with Shift, and Ctrl+Insert+Right, Left and Up from paragraphs, list items, a list, links and headings: what JAWS counts as the same element | `same-element.html` (new) |
| 559 | Jump to Line (J) with a line and Enter, then where the cursor went; Shift+J; a line past the last | `news-article.html` |
| 560 | The Word List (Ctrl+Insert+W), and W and Shift+W with a word chosen and without | `news-article.html` |

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
