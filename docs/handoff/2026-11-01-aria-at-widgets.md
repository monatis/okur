# Handoff: ARIA-AT's menus, dialogs, grid, spin button and sliders (2026-11-01, second)

Follows [`2026-11-01-aria-at-radios-and-tabs.md`](2026-11-01-aria-at-radios-and-tabs.md) and
covers `7a302ba5` to this one, made under
[`../plans/2026-10-31-aria-at-web-round-2.md`](../plans/2026-10-31-aria-at-web-round-2.md), which
stays the active plan.

Where things stand: 27 of ARIA-AT's test plans are in the corpus, nine more since the last
handoff (tabs-automatic-activation, the three menu buttons, modal-dialog, minimal-data-grid,
quantity-spin-button, horizontal-slider, seek-slider). Every trial of them succeeded. The
assertions now have a measure of their own (`okur-lab aria-at-report`).

## What was built

### What the plans found in Okur, and fixed

- **Tabs, automatic activation** (`b8be782a`): a tab the arrows select is focused before the
  page's change reaches the buffer; the focus's own selection refreshes it. A tab panel taking
  the focus says "Peter Müller TabPanel".
- **Menu buttons and their menus** (`e4e3e992`, `bc4fbc3e`): a quick key says "Actions button
  menu collapsed"; with the virtual cursor off the focus gives "Press space to activate the
  menu, then navigate with arrow keys"; the menu opens with "Actions menu | Action 1 | 1 of 4"
  (a page menu item's focus carries its menu's name); with the virtual cursor off the items are
  said by their focus, "Action 1 | 1 of 4", a key focusing the item said last says it again;
  Insert+Tab names the menu; the button's collapse is not said as the focus returns to it.
- **Modal dialogs** (`0051d797`, `7e56ad29`): an open aria-modal dialog keeps the virtual
  cursor to its content (Ctrl+Home its heading, Ctrl+End its last button, the edges say their
  line again), checked as each page is installed and as the focus moves; the old check matched
  only the dialog node itself and never confined. A focus entering a dialog names it, with its
  description, "Modal" with the virtual cursor off: UI Automation does not say a dialog is
  aria-modal, so the tracer reads IA2's modal state for a focus in a page dialog
  (`xa11y_windows::ia2::chromium_modal_dialog_of`). The invented "Leaving Dialog", Escape's
  exit from the confinement and the unbound EnterModal/ExitModal commands are gone: Escape is
  the page's.
- **ARIA grids** (`37ef1cdb`): "grid with 5 columns and 7 rows", "grid end"; a table named from
  outside it has no caption line; a focus into a grid's cell says the grid first.
- **Spin buttons** (`32799f7a`): an "edit spin box", its line "1 edit spin box", its own hint
  with the virtual cursor off, its group named by a quick key.
- **The PC cursor in page fields** (`32799f7a`): a page's focused field is edited as a native
  one: Up and Down in a spin button say its new value, Insert+Up says its text.
- **Sliders** (`5a5df74f`): "Red left right slider 128" (IA2's horizontal state), F stops on
  them, a quick key names the group, "To increase or decrease use the Arrow keys." with the
  virtual cursor off, and the keys say the new value.

### Harness

- **`okur-lab aria-at-report`** (`41a24943`, docs `fb8734b6`): per plan, how many assertions
  Okur's speech and JAWS's recorded speech convey, judged the same way (a role or state by
  JAWS's words, a name as written, a place, MUST counted apart); `--unmet <plan>` lists what
  Okur leaves unmet. The unmet MUST assertions reviewed by hand were real.
- **The recorder's reset may wait longer** (`reset.settle_max_ms`, `7e56ad29`): the nested
  dialog's setup kept JAWS speaking past 10 s in every trial of 2687-2689; with 30 s they
  recorded (round-aria23b).
- **A queued round inserted in the chain**: only the waiter of the round after it is replaced
  (round-aria23b went in after aria26, aria27 after it).

## Found, and how

- **A page's modal state is IA2's alone**: UI Automation's chain says `is_dialog` but not
  aria-modal; round-aria23 lacked it and was recorded again as round-aria23b with the tracer.
- **Variations in JAWS's grid arrows**: the PC cursor's arrows in minimal-data-grid say a
  different coordinate between trials ("$999,741.00 | Row 3" or "Balance | $999,741.00 |
  column 4 row 3"); left open, not matched to one trial.
- **"clickable"**: JAWS says it after a grid cell whose ancestor has a click handler (Chromium's
  "click ancestor" action); the snapshots hold no default action yet.

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 10529/11362 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 10685/11301 steps match exactly; 308 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `7a302ba5`: 10337/10927. Each plan replayed with `okur-lab parity --only`
over its scenario ids, its line as printed:

```
tabs-automatic-activation (2538-2573): JAWS parity (Ia2): 54/54 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
menu-button-actions (2574-2604): JAWS parity (Ia2): 48/48 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
menu-button-actions-active-descendant (2605-2635): JAWS parity (Ia2): 48/48 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
menu-button-navigation (2636-2665): JAWS parity (Ia2): 46/46 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
modal-dialog (2666-2693): JAWS parity (Ia2): 42/46 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
minimal-data-grid (2694-2717): JAWS parity (Ia2): 22/38 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
quantity-spin-button (2718-2738): JAWS parity (Ia2): 26/30 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
horizontal-slider (2739-2758): JAWS parity (Ia2): 33/33 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
seek-slider (2759-2778): JAWS parity (Ia2): 29/33 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
rating-radio-group (2441-2495): JAWS parity (Ia2): 60/77 steps match exactly; 2 accepted divergences not counted (0 scenarios could not run)
tabs-manual-activation (2496-2537): JAWS parity (Ia2): 60/64 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab aria-at-report`, its total line (assertions, judged, Okur, JAWS, Okur only, JAWS
only, MUST, Okur, JAWS):

```
total                                           3070    2929    2608    2551         64          7    1879  1844  1847
```

`cargo test --workspace --no-fail-fast` (4 test threads): 1572 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 1526 scenarios, 4584 trials, 4567 succeeded.

Open:
- minimal-data-grid: "clickable" (no default action in the snapshots) and the PC cursor's
  arrows (JAWS varies).
- quantity-spin-button: an invalid value's "Has Error | invalid entry | ... min 1 max 8" and
  typing's echo (2732-2735, 2738); the measure's 7 "JAWS only" are these.
- seek-slider: "min 0 max 300" after a value text (no minimum and maximum in the snapshots).
- modal-dialog: the VC Tab's "Type in text." (2677/s1, as accordion 2160/s1) and the nested
  dialog's heading's "Use JawsKey+Alt+R to read descriptive text" (2692).
- rating (19 steps) and tabs (4): round-probe-radio2 decides them.

Not checked live on the dev box.

## old-pc

- Running: round-aria28 (vertical-temperature-slider). Queued after it: round-aria29
  (disclosure-navigation), round-aria30 to aria40 (the first format's eleven), then
  round-probe-radio2. Rounds from round-aria23b on use the staged tracer
  (`okur-lab.exe ed5f3f0e…`) and recorder (`run-scenario.ps1 08b86deb…`).
- Excel's round-1122b waits to be imported (plan 41).

## Next

The plan: process each round as it finishes (vertical sliders will need an "up down slider"
word from their evidence); then the snapshots' default actions and value ranges ("clickable",
"min max"); round-probe-radio2's answer for rating and tabs.
