# Handoff: ARIA-AT's alert, the other radio groups and tabs (2026-11-01)

Follows [`2026-10-31-aria-at-first-plans.md`](2026-10-31-aria-at-first-plans.md) and covers
`2912ecb4` to this one, made under
[`../plans/2026-10-31-aria-at-web-round-2.md`](../plans/2026-10-31-aria-at-web-round-2.md), which
stays the active plan.

Where things stand: seventeen of ARIA-AT's test plans are in the corpus now, the thirteen of the
last handoff and alert, radiogroup-aria-activedescendant, rating-radio-group and
tabs-manual-activation, with two probe pages of Okur's own (1126, 1127-1128 queued). Every
trial of the four new plans succeeded. The second format's other eleven plans and the first
format's eleven are queued on old-pc.

## What was built

### What the plans found in Okur, and fixed

- **A page's alert is spoken** (alert, `db2d52aa`): the tracer captured Chromium's alert event
  with its region (`64f0b79c`), but parity's replay filter and the live reader's event list
  both left the event out. A message box's alert stays with its dialog's focus.
- **A focusable radio group is a group** (radiogroup-aria-activedescendant, `47fb39a4`): in UI
  Automation a radio group taking the focus itself looked like a contenteditable (a focusable
  group without `readonly=true`), so its radio buttons went missing from the UIA buffer (30
  invariant violations). An editing host now has no role of its own.
- **A checked ARIA radio button says its place** (`596a0817`): the activedescendant and rating
  groups say "Deep dish radio button checked | 2 of 3", and so does every group of a probe page
  (1126) varying what the roving-tabindex page has. That page's silence about it (17 steps) is
  an accepted divergence now, which also settles 2112/s1's (Enter's place).
- **The element naming an ARIA group from inside it is read** (rating-radio-group,
  `08359600`): "group start Rating | Rating | one star radio button"; only a fieldset's legend
  is skipped.
- **Tabs and tab panels** (tabs-manual-activation, `0de4afa1`):
  - A tab's line carries its selection and place: "Maria Ahlefeldt Tab selected | 1 of 4".
  - F stops on tabs, said as their focus says them.
  - A named tab panel has "tab panel start Peter Müller" and "tab panel end" lines, and a focus
    entering it says "Peter Müller TabPanel".
  - With the virtual cursor off, a selected tab says "selected" after its role too, Insert+Tab
    names the tab list by its name alone, and Insert+Up says the tab without its place.

### Harness

- **First-format plans import with readable ids** (`4cc1c72b`): "SHIFT_E" made
  "s-h-i-f-t--e"; the eleven first-format plans with JAWS commands are in `corpus/pending`
  (ids 2859-3153; menubar-editor has none).
- **A snapshot brings its page forward again** (`893c6e31`): Word's kept document stayed in
  front of a round's first page and its UIA snapshot failed (round-aria17, round-probe-radio).
  `capture-snapshot.ps1` now asks `okur-lab front` up to three times.
- **A queued round is known by its `-OutputDir`** (skill troubleshooting §9, `0e0a2dbc`): a
  requeue that took the first round named in a command line left an old waiter beside the new
  one; caught by listing the waiters and their `-After`, then stopped.

## Found, and how

- **Probes settle what one page cannot**: 1126 (seven groups: focusable or not, label inside
  or out, radio buttons focusable or not, aria-activedescendant, checked in the markup or by a
  script) showed every checked radio button saying its place, which turned the roving page's
  behavior from a rule into a divergence.
- **Still unexplained: where JAWS leaves out a place.** A checked radio button (rating
  2475/s1-2480/s1, 2492/s1, 2493/s1) and a tab whose selection the setup changed (2521, 2522, 2524, 2525)
  are said without their place; the rating cases all leave the focus on a radio button of the
  group, the tab cases changed the item's state after the page loaded. Neither explains every
  case (rating 2451 and the activedescendant page say it after a state change). A second probe
  (1127, 1128: a focus left in a group, and labels of each kind) is queued as
  round-probe-radio2.
- **The empty state after "group"**: the Pizza Crust groups' focus says "Pizza Crust group ‹›",
  rating's "Rating group" has none; the groups labelled by a heading have it. 1127 tests it.

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 10193/10988 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 10337/10927 steps match exactly; 306 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `2912ecb4`: 10129/10696. Each plan replayed with `okur-lab parity --only`
over its scenario ids, its line as printed:

```
alert (2346-2349): JAWS parity (Ia2): 6/6 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
radiogroup-aria-activedescendant (2386-2440): JAWS parity (Ia2): 77/77 steps match exactly; 2 accepted divergences not counted (0 scenarios could not run)
rating-radio-group (2441-2495): JAWS parity (Ia2): 60/79 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
tabs-manual-activation (2496-2537): JAWS parity (Ia2): 60/64 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
radiogroup-roving-tabindex (2067-2121): JAWS parity (Ia2): 60/60 steps match exactly; 19 accepted divergences not counted (0 scenarios could not run)
probe 1126: JAWS parity (Ia2): 21/21 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace --no-fail-fast` (4 test threads): 1570 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 1285 scenarios, 3861 trials, 3844 succeeded.

Open: rating's 19 steps (the empty state after "Rating group", the places above, the PC
cursor's hint said first with none checked, 2445/s1 and 2450/s1, as in the other two groups);
tabs' 4; accordion 2160/s1 (from before). Not checked live on the dev box.

## old-pc

- Running: round-aria19 (tabs-automatic-activation). Queued after it, each waiting up to two
  days for the one before: round-aria20 to aria29 (the menu buttons, modal-dialog,
  minimal-data-grid, quantity-spin-button, the sliders, disclosure-navigation), round-aria30 to
  aria40 (the first format's eleven), then round-probe-radio2.
- Excel's round-1122b waits to be imported (plan 41).

## Next

The plan: process each round as it finishes (preview, fix, import); round-probe-radio2 decides
the empty state after a group and the places; then the measure of assertions.
