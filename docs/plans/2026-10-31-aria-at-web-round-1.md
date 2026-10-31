# Plan: ARIA-AT's test plans for the web, round 1 (scheduled after Excel)

Asked for by the owner on 2026-10-31: after the Excel work, use W3C's ARIA-AT reports
(<https://aria-at.w3.org/reports>) and their tests as a source for evidence-driven work on the
web, so that Okur reads the web's widgets as well as any screen reader. It starts when plan 41's
Excel items are done, and comes before the first release
([`2026-10-31-microsoft-store-release.md`](2026-10-31-microsoft-store-release.md)). Backlog row
44 of [`../backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md).

## What ARIA-AT is (looked at 2026-10-31)

The ARIA and Assistive Technologies project of the W3C tests how screen readers convey the
widgets of the ARIA Authoring Practices Guide (APG). Its repository,
<https://github.com/w3c/aria-at>, has:
- `tests/apg/`: 40 test plans, one per pattern: accordion, alert, the landmarks (banner,
  complementary, contentinfo, form, main), breadcrumb, checkbox and the tri-state checkbox, two
  comboboxes, command button, two disclosures, three links, four menu buttons, menubar, meter,
  data grid, modal dialog, spin button, two radio groups and two ratings, five sliders, three
  switches, two tabs, toggle button. `tests/aria/` and `tests/html/` hold 3 more.
- In each plan, `reference/<date>/`: the test page, an APG example; `data/tests.csv`: each test
  (`testId`, title, the setup script that brings the page to the test's state, instructions,
  assertions); `data/jaws-commands.csv`: the JAWS keys for each test and the cursor they are
  pressed in (`virtualCursor`, `pcCursor`); `data/assertions.csv`: what must be conveyed
  ("Role 'checkbox' is conveyed", priority 1 must, 2 should); `data/js/`: the setup scripts.
  NVDA's and VoiceOver's commands sit beside JAWS's, which macOS will use later.
- `tests/support.json`: the screen readers and their modes in ARIA-AT's words ("virtual cursor
  active", "PC cursor active").
- The license: "All documents in this Repository are licensed by contributors under the W3C
  Document License" (`LICENSE.md`).

The reports show, per test plan, screen reader and browser, how many assertions were met, from
testers' runs of JAWS, NVDA and VoiceOver.

## How it fits the harness

ARIA-AT gives what our corpus most lacks for the web: fixtures of real widgets with their
behavior scripted, and a list of keys to try on each. It does not replace our evidence:
- **Evidence is still ours.** Its testers' JAWS output is a lead, never evidence (AGENTS.md
  §3): every test is recorded again on old-pc with our capture tooling, JAWS 2025 and Chrome.
- **Two measures.** Exact parity against our recordings stays the measure. ARIA-AT's
  assertions add a second, coarser one: for each test, whether Okur's speech conveys the role,
  name and state asserted. A JAWS miss in ARIA-AT's reports that our recording repeats becomes
  an accepted divergence only if it is JAWS's accident (plan 30's rule); where JAWS fails an
  assertion and the user loses information, Okur may convey it and the divergence is written
  down.
- **Clean room is untouched.** ARIA-AT is the W3C's, not Vispero's.

## Status (2026-10-31, afternoon)

Started at the owner's word, Excel paused. Done: the research
([`../references/aria-at-research.md`](../references/aria-at-research.md)), the importer
(`okur-lab aria-at-import`, all 28 second-format plans import, 859 scenarios), fixtures in
folders, the recorder stamping each reset key's time and replay pressing the keys among the
setup events by those times. Found on old-pc: Ctrl+End lands on JAWS's "main region end", so
the reset is Ctrl+End, Shift+B, Enter (round-aria1, with Ctrl+End and Enter only, ran no setup
and was stopped after 5 scenarios). Queued: checkbox-tri-state, disclosure-faq,
radiogroup-roving-tabindex, accordion (ids 2000-2169), one round each.

## Steps

1. **Research** (a notes file in `docs/references/aria-at-research.md`): the wiki's test format
   and setup scripts; which JAWS and Chrome versions the reports used and their verdicts per
   plan; the APG examples' own license (the W3C Software and Document License of
   `w3c/aria-practices`); whether the license lets the reference pages be committed under
   `corpus/fixtures/` with their notice, or whether the importer fetches them at a pinned
   commit instead.
2. **An importer** (`okur-lab aria-at import <plan>`): reads a plan's CSVs at a pinned commit
   of `w3c/aria-at` and writes pending scenarios: the page and its setup script as the fixture,
   each JAWS command as steps in the named cursor, the assertions kept beside them.
3. **Record** on old-pc, a few plans per round, several trials each: checkbox, tri-state
   checkbox, radio groups, disclosure, tabs and the landmarks first (closest to what Okur
   reads), then the menus, combobox, sliders, spin button, grid and dialog.
4. **Build** what differs, plan by plan, by which mode owns the key (forms mode, the virtual
   cursor, focus), never by the fixture's names.
5. **Report**: `okur-lab parity --only` per plan, and the assertions met, per plan, beside
   ARIA-AT's JAWS figures.
