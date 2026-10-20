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
| 598, 613 | Round 79 failed one step of each in every trial for the step's 10 seconds: 598/s13 (Ctrl+A at Intermediate reads the whole page) and 613/s16 (spelling a line); both wait longer now | as written |
| 613 | A Turkish page (`lang="tr"`, ç ğ ı İ ö ş ü) read by character, word and line, spelled, and with Say All, in JAWS's English voice (old-pc's language detection is off) | `turkish-text.html` (here) |
| 617 | Ctrl+Shift+End from ten depths of fifteen plain 80-character paragraphs: the size at which JAWS stops reading a selection and says "selected N characters up to", how it counts N, and how much of the end it says (562 left all three open) | `selection-sizes.html` (here) |
| 618, 619 | Say Color (Insert+5) on the 148 CSS named colors and a 6x6x6 RGB grid, for JAWS's color table (565 showed names of its own: blue2, lime green 2, dark slate gray); their IA2 snapshots give the RGB of each line | `color-names.html`, `color-grid.html` (here) |
| 620 | Ctrl+Shift+End over 103 lines of 9 characters, selecting 989 to 1029 characters: the size from which JAWS says "selected N characters up to" (617 put it between 971 and 1052). 617, recorded in round 82, waits in the scratchpad for its page's snapshot, which is captured with 620's | `selection-threshold.html` (here) |
| 621, 622 | Usage hints on Tab in the sign-up form ("Type in text."): 555's keys with no Verbosity setting (621), and 204's with Verbosity=0 written (622). 552, 553 and 555 (Down twice, then Tab; Verbosity set) have the hints at every level; 204, 573 and 583 (Tab from the top) have none at any level: the way in or the setting | `signup-form.html` |

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
