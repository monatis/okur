# Handoff: ARIA-AT beyond JAWS, a newer JAWS's words, banner and breadcrumb (2026-11-01, third)

Follows [`2026-11-01-aria-at-widgets.md`](2026-11-01-aria-at-widgets.md) and covers `c06efc02` to
this one, made under [`../plans/2026-10-31-aria-at-web-round-2.md`](../plans/2026-10-31-aria-at-web-round-2.md),
which stays the active plan.

Where things stand: 31 of ARIA-AT's test plans are in the corpus, four more since the last
handoff (vertical-temperature-slider, disclosure-navigation, banner, breadcrumb). Every trial of
them succeeded. The owner's rule for the web (AGENTS.md §1.2: Okur conveys what ARIA-AT's plans
require where JAWS does not) has its first round of additions, and a second source joined the
recorded evidence: what a newer JAWS (2025.2508.120) said in ARIA-AT's published reports.

## What was built

### Beyond JAWS, under the owner's rule

- **The MUST items of plan §4b** (`c5a547a2`, `ef77e584`, `ed19e9d3`, `93644670`): an accordion
  header's heading and level with the PC cursor's Tab; a spin button's name, role and invalid
  state on the PC cursor's Insert+Up; an invalid field's "invalid entry" and "Has Error"; a
  named region a form-field quick key enters; a disclosure closed as the focus returns; a
  focused region naming itself.
- **A menu button's state on a focus** (`6bf4e8f5`): "Actions button menu collapsed" on Tab,
  Insert+Tab, a return from its menu and the PC cursor's Insert+Up. JAWS says it on quick keys
  only.

### What a newer JAWS says on purpose, Okur says (`8fb17659`)

From ARIA-AT's reports of JAWS 2025.2508.120 (old-pc runs 2025.2505):
- a slider's and a spin button's limits after a number value, "Red left right slider | 128 min 0
  max 255", "Adults edit spin box | 1 | min 1 max 8"; none after a text value; a spin button's
  not on Insert+Up. The IA2 snapshot now carries a control's limits (IAccessibleValue,
  `c06efc02`); the corpus's slider and spin button pages get them when round-probe-radio2
  captures them again, so parity cannot show the limits yet (unit test
  `range_limits_follow_a_number`);
- the hints of a menu button, slider, spin button and a dialog's first field with the virtual
  cursor on too;
- "Modal" before a modal dialog with the virtual cursor on;
- a page menu's item and its place on Insert+Tab and Insert+Up with the virtual cursor on,
  "Action 1 | 1 of 4" (a menu bar's item keeps its line);
- an ARIA grid's name said once on Tab.

Not followed: Insert+Up on a menu button without its state, and radio buttons' hints with the
virtual cursor on (the reports' outputs disagree with each other, and old-pc's forms pages have
none).

### What banner and breadcrumb found (`a65d10c5`)

A control's aria-keyshortcuts after its name in the virtual cursor ("Skip To Content Alt+0
button menu"), not with it off; a button's description on a quick key and a focus; an image's
aria-roledescription in place of its role (Chrome's "Unlabeled graphic").

### Harness

- **A newer JAWS beside the recorded one** (`d77bb248`):
  `skills/jaws-reference/scripts/aria-at-reports.sh <json>` saves ARIA-AT's published outputs
  (the report app's GraphQL API, no sign-in); `okur-lab aria-at-report --newer-jaws <json>`
  lines up each imported scenario's newest JAWS output beside the recorded JAWS and Okur. On
  2026-11-01 the two JAWS differed on 142 of 792 commands; most were the testers' settings or
  timing. Research, never evidence.
- **The judge** (`f279f8ec`, `d77bb248`, `3be40e8b`): counts and a cell's row and column; "not
  convey numeric value" met when the number is said only within its text; JAWS's one-word
  "BannerRegion" for the landmark roles; "position screen reader cursor at heading 'X'" as the
  cursor's line; "Number of list items, 4, is conveyed".
- **One container line** (`d6869c17`): the line a focus says for a landmark, list, dialog or tab
  panel it enters is `SpeechFormatter::container_entered`, ready for other moves.

## Found, and how

- **A newer JAWS changed on purpose**: the reports showed JAWS 2025.2508 passing every slider
  SHOULD that our recorded 2505 missed ("min 0 max 255"); the report page's own text gave the
  words, its JavaScript bundle the GraphQL query.
- **menu-button-navigation's "number of items, '4'"** contradicts its own page, which has six
  items ("1 of 6"): the plan's error, not a gap.
- **A gate quirk**: the reference-data gate judges the working tree, so an uncommitted crates/
  change blocks a corpus import's commit even when unstaged. Set the file aside (a scratch copy),
  commit, put it back.

## Waiting for the owner

**The containers a quick key enters.** ARIA-AT's landmark and breadcrumb plans (drafts) require
a quick key into a landmark or list to say it: "BannerRegion" before B's button, "Breadcrumb
NavigationRegion | list with 4 items" before U's link. JAWS says them on Tab, not on quick keys.
Tried on 2026-11-01 (using `container_entered`): 215 steps changed, 180 of them outside ARIA-AT,
on every page's H, U and L ("MainRegion | City Library Expands Evening Hours heading level 1").
It is the everyday sound of the reader, so it is not in: always, never (as JAWS), or a setting?
The same plans' arrows into a landmark (JAWS's "banner" line of its own, where ARIA-AT wants the
first link with it) are left as JAWS has them, which keeps its line model.

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 10683/11557 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 10885/11496 steps match exactly; 316 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `268fd90e`: 10685/11301 with 308 accepted. The new plans, with `okur-lab
parity --only` over their scenario ids:

```
banner (2859-2887): JAWS parity (Ia2): 35/35 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
breadcrumb (2888-2912): JAWS parity (Ia2): 44/44 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
the sliders and the spin button (2718-2798): JAWS parity (Ia2): 127/129 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab aria-at-report`, its total line (assertions, judged, Okur, JAWS, Okur only, JAWS
only, MUST, Okur, JAWS):

```
total                                           3557    3363    3086    2971        122          7    2269  2219  2198
```

`cargo test --workspace --no-fail-fast` (4 test threads): 1574 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 1660 scenarios, 4986 trials, 4969 succeeded.

Open:
- The MUST assertions Okur leaves unmet (50 by the judge): the containers above
  (banner, breadcrumb); Shift+Tab into a radio group with none checked lands on its first radio
  button (the page's tab stop, in Chrome); a spin button's name on its own arrow line (kept as
  JAWS reads it); an accordion panel's required field two arrows in.
- The slider and spin button limits in parity, once round-probe-radio2's snapshots are in.
- From before: minimal-data-grid's "clickable" and PC arrows; modal-dialog's 2677 hint and
  2692 "descriptive text"; rating and tabs (round-probe-radio2).

Not checked live on the dev box.

## old-pc

- Running: round-aria32 (combobox-autocomplete-both-updated, 71 commands). Queued after it:
  round-aria33 to aria40 (combobox-select-only, complementary, contentinfo, form, main, meter,
  rating-slider, slider-multithumb), then round-probe-radio2 (1127, 1128 and the slider, spin
  button and grid pages' snapshots again, with their limits and default actions).
- Excel's round-1122b waits to be imported (plan 41). old-pc's JAWS is 2025.2505.43; a newer
  build (2025.2508 or later) would make the reports' differences recordable there, which is the
  owner's call.

## Next

The plan: process each round as it finishes; the owner's answer on the containers; then plan
§4b's MAY items (the tab list as a tab list, a tab's "not selected").
