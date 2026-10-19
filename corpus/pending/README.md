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
| 278 | Down through every line of the web app patterns page, Up five times (the owner's request: form fields read with the arrows) | `web-app-patterns.html` |
| 279 | Down through the forms page and Up back | `forms-interaction.html` |
| 280 | Down through the sign-up form with Auto Forms Mode on (JAWS's default; old-pc has it off), Up five times | `signup-form.html` |
| 405 | Mixed case, words with digits, numbers, dates and money at JAWS's defaults, by line and by word (research §6) | `desktop-reading.ps1` |
| 406 | Capitals at JAWS's defaults: Say Word and Say Line once and twice (spelled), Say All, a capital typed with Characters echo (research §5) | `desktop-reading.ps1` |

Scenarios from 400 record JAWS's reading settings (plan
`docs/plans/2026-10-18-reading-settings.md`). Their fixtures, `reading-settings.html` (here)
and its desktop twin `desktop-reading.ps1` (recorded with 400), hold the 18 lines of
`docs/references/jaws-reading-settings-research.md` §12, the same text in both. A scenario
that needs a setting other than old-pc's names it in `jaws_settings`
(`skills/jaws-reference/scripts/jaws-settings.ps1`), and `run-batch.ps1` sets it.

Every scenario written here before (208, 210–219, 315–323) has been recorded; git history keeps
the notes written with them.
