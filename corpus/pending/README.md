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
| 281 | 219 (WhatsApp Web's patterns) with Auto Forms Mode on, JAWS's default (backlog: the dev-box rules 1 to 3) | `web-chat.html` |
| 407 | 405 in the virtual cursor: words, numbers, dates and money by line and by word | `reading-settings.html` (here) |
| 408 | 406 in the virtual cursor: Say Word and Say Line spelled, Say All, on the capitals line | `reading-settings.html` (here) |

Scenarios from 400 record JAWS's reading settings (plan
`docs/plans/2026-10-18-reading-settings.md`). Their fixtures, `reading-settings.html` (here)
and its desktop twin `desktop-reading.ps1` (recorded with 400), hold the 18 lines of
`docs/references/jaws-reading-settings-research.md` §12, the same text in both. A scenario
that needs a setting other than old-pc's names it in `jaws_settings`
(`skills/jaws-reference/scripts/jaws-settings.ps1`), and `run-batch.ps1` sets it.

Every scenario written here before (208, 210–219, 315–323) has been recorded; git history keeps
the notes written with them.
