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
| 275 | Heading level keys across sections: into new and same sections, back, and 1 (audit §2.6; the 2017 rule says JAWS stops at a higher heading) | `headings-sections.html` (here) |
| 276 | A select box opened with Space: its list, Down, Up, Enter, Space, Escape (229/s21 had no position and no combo box) | `signup-form.html` |
| 401 | Say Character once, twice, three times, with Numpad 5 and CapsLock+Comma, on J, €, é, 9, B, a | `desktop-reading.ps1` |
| 402 | Phonetic mode after Numpad 5 twice, and which keys end it | `desktop-reading.ps1` |

Scenarios from 400 record JAWS's reading settings (plan
`docs/plans/2026-10-18-reading-settings.md`). Their fixtures, `reading-settings.html` (here)
and its desktop twin `desktop-reading.ps1` (recorded with 400), hold the 18 lines of
`docs/references/jaws-reading-settings-research.md` §12, the same text in both. A scenario
that needs a setting other than old-pc's names it in `jaws_settings`
(`skills/jaws-reference/scripts/jaws-settings.ps1`), and `run-batch.ps1` sets it.

Every scenario written here before (208, 210–219, 315–323) has been recorded; git history keeps
the notes written with them.
