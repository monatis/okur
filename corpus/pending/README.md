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
| 400 | Punctuation level: Insert+Shift+2 four times, lines 1 to 4 read at each level | `desktop-reading.ps1` (here) |

400 and up record JAWS's reading settings (plan `docs/plans/2026-10-18-reading-settings.md`).
Their fixtures, `reading-settings.html` and its desktop twin `desktop-reading.ps1`, hold the 18
lines of `docs/references/jaws-reading-settings-research.md` §12, the same text in both. A
scenario that needs a setting other than old-pc's names it in `jaws_settings`
(`skills/jaws-reference/scripts/jaws-settings.ps1`), and `run-batch.ps1` sets it.

Every scenario written here before (208, 210–219, 315–323) has been recorded; git history keeps
the notes written with them.
