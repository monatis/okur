# Quarantined Scenarios (31–65)

Moved here by the 2026-10-14 audit (`docs/handoff/2026-10-14-full-audit-of-prior-development.md`).

The `evidence/` files in this folder were **not** produced by the capture tooling:

- `run-scenario.ps1` can only dispatch `jaws_script` and `keyboard` steps, yet these files record
  successful `focus`, `aria_live` and `spa_route` steps.
- Every file holds a single trial with round, evenly spaced timestamps; several carry a `summary`
  claiming 5/5 deterministic trials.
- Live-region "transcripts" are exactly the strings the scenarios inject.

They are kept only as engine regression checks (`unverified_scenarios_meet_authored_assertions`).
**Never cite them as JAWS ground truth.** A scenario leaves this folder only when it is re-recorded
on `old-pc` by the capture tooling; the old evidence file is then deleted, not edited.
