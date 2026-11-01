# Handoff: ARIA-AT's combo boxes, landmarks, grid and error messages (2026-11-01, fourth)

Follows [`2026-11-01-aria-at-newer-jaws.md`](2026-11-01-aria-at-newer-jaws.md) and covers `889c2286` to
this one, made under [`../plans/2026-10-31-aria-at-web-round-2.md`](../plans/2026-10-31-aria-at-web-round-2.md),
which stays the active plan.

Where things stand: 36 of ARIA-AT's test plans are in the corpus, five more since the last
handoff (combobox-autocomplete-both-updated, combobox-select-only, complementary, contentinfo,
form). Every trial of them succeeded. The owner's question on the containers a quick key enters
is still open (below).

## What was built

### A bug the combo boxes found: Okur's own combo box simulation (`a4584ea3`)

The virtual cursor carried a simulation of combo boxes: Alt+Down and Alt+Up flipped the buffer's
expanded state and said it, the arrows changed its value in the buffer, and Escape on an
"expanded" combo box was consumed and said "collapsed" without ever reaching the page, so a real
page's list stayed open. It is gone. JAWS binds neither Alt key in its key map (Default.JKM read
for this); the keys are the page's, Escape on an open list is the page's in forms mode too, and
the page's own change is what Okur says.

### Combo boxes and list boxes

- **A list box's options** (`a4584ea3`) are said from their focus, held in the buffer or not (a
  combo box's list is shown by the arrow that moves into it): "States List box | Alabama | 1 of 56
  | To move to an item press the Arrow keys." as the focus comes into the list box, "Alaska | 2 of
  56" in it, the whole on Insert+Tab. The focus event carries `container_is_list_box`.
- **"Open List Box" and "Close List Box"** (`a4584ea3`, `aaae1fc2`): after Alt+Down and Alt+Up, as
  the page opens or closes the list; the state is the key's, since Chrome still reports the list
  expanded as Alt+Up's change arrives (JAWS says "Close List Box | expanded" in every trial).
- **Beyond JAWS** (MUST in ARIA-AT's plans): "expanded" as the focus comes from a combo box into
  its list box (`aaae1fc2`); a combo box's state on a focus, a select and an ARIA one alike,
  "Favorite Fruit Combo box collapsed | Choose a Fruit" (`730e1c22`, 28 steps accepted); an edit
  combo's "Supports AutoComplete" (IA2's autocompletion state) on Insert+Tab with the virtual
  cursor on and on Insert+Up.
- An edit combo's line says its text first, "Alabama edit combo collapsed"; a combo box's hint with
  the virtual cursor off, Insert+Tab included.

### Grid, fields, tabs (beyond JAWS, ARIA-AT's MUST or SHOULD)

- **An ARIA grid's cell says where it is** (`a9199485`): Insert+Tab, "Date | 01-Jan-16 | column 1
  row 2"; the arrows with the virtual cursor off, the column or row that changed every time, where
  JAWS says a stale one in some trials.
- **An invalid field says its error message** (`9caa4977`): the IA2 snapshot follows the field's
  error relation to its words; "Adults edit spin box | Has Error | invalid entry | 9 | Must be
  between 1 and 8", not on Insert+Up.
- **A tab's place on the PC cursor's Insert+Up** (`889c2286`).

### Forms and landmarks (`67f47132`)

- A control named as its fieldset's legend was skipped with the legend: ARIA-AT's form's "Add
  Contact" submit button was missing from the buffer and B found no buttons. Only the legend's
  text is skipped now.
- An edit and a button in a named fieldset name it as a quick key or a focus comes in: "Add
  Contact group | Name Edit | blank".
- A form landmark's line, "Add Contact Form", from the catalog where the landmark's type was said
  raw.

### Harness and research

- round-probe-desc (1129) is queued after round-probe-radio2: what JAWS's Insert+Alt+R does on
  described elements, a key no key map of JAWS 2025 binds though its hint names it ("Use
  JawsKey+Alt+R to read descriptive text", modal-dialog 2692).

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 10883/11786 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11125/11725 steps match exactly; 343 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `75bdcb5a`: 10885/11496 with 316 accepted. Plans touched, with `okur-lab
parity --only` over their scenario ids:

```
combobox-autocomplete-both-updated: JAWS parity (Ia2): 100/100 steps match exactly; 12 accepted divergences not counted (0 scenarios could not run)
combobox-select-only: JAWS parity (Ia2): 54/54 steps match exactly; 6 accepted divergences not counted (0 scenarios could not run)
complementary: JAWS parity (Ia2): 30/30 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
contentinfo: JAWS parity (Ia2): 23/23 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
form: JAWS parity (Ia2): 31/31 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
minimal-data-grid: JAWS parity (Ia2): 20/28 steps match exactly; 10 accepted divergences not counted (0 scenarios could not run)
quantity-spin-button: JAWS parity (Ia2): 28/29 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab aria-at-report`, its total line (assertions, judged, Okur, JAWS, Okur only, JAWS
only, MUST, Okur, JAWS):

```
total                                           4347    4039    3647    3468        184          5    2917  2750  2686
```

`cargo test --workspace --no-fail-fast` (4 test threads): 1575 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 1833 scenarios, 5505 trials, 5488 succeeded.

Open:
- minimal-data-grid's 8 open steps: "clickable" (no default action in its snapshots until
  round-probe-radio2 captures them again); quantity-spin-button 2732/s1: the limits, likewise.
- combo boxes' MUST assertions neither conveys: a field's text said "selected" where the page
  selected it (an inline autocomplete's completion as a letter is typed; a focus onto a field
  whose text is selected), the list box's name and role on each arrow within it. The first needs
  the field's selection (IA2 text selection), a feature of its own.
- From before: rating and tabs (round-probe-radio2), modal-dialog 2692 (descriptive text,
  round-probe-desc).

Not checked live on the dev box.

## Waiting for the owner

Unchanged from the last handoff: should a quick key into a landmark or list name it ("MainRegion"
before H's first heading in main)? ARIA-AT's landmark and breadcrumb plans require it; JAWS says
them on Tab only; tried, it changed 215 steps, 180 of them on everyday pages. Always, never (as
JAWS), or a setting?

## old-pc

- Running: round-aria37 (main). Queued after it: round-aria38 (meter), aria39 (rating-slider),
  aria40 (slider-multithumb), round-probe-radio2, round-probe-desc.
- Excel's round-1122b waits to be imported (plan 41).

## Next

The plan: process each round as it finishes; round-probe-radio2's snapshots (limits, "clickable")
and answers (rating, tabs); round-probe-desc for Insert+Alt+R; the owner's answer on containers;
then the plan's §5 (Excel, then the Store release).
