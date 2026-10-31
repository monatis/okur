# Handoff: ARIA-AT's first thirteen plans recorded and matched (2026-10-31, fourth)

Follows [`2026-10-31-excel-menus-and-aria-at-start.md`](2026-10-31-excel-menus-and-aria-at-start.md)
and covers `a13ca940` to this one, made under
[`../plans/2026-10-31-aria-at-web-round-2.md`](../plans/2026-10-31-aria-at-web-round-2.md), which
stays the active plan.

Where things stand: thirteen of ARIA-AT's test plans are in the corpus, 382 scenarios (ids
2000-2385 but alert's 2346-2349), every trial succeeded; of their 525 steps 521 match JAWS, 3
are accepted and 1 differs (accordion 2160/s1), summed from the rounds' lines below. Alert's 4
scenarios wait to be recorded again.

## What was built

### What ARIA-AT's plans found in Okur, and fixed

Each with the plan that found it; the details and their evidence are in the commits.
- **Check boxes** (checkbox-tri-state, checkbox): a mixed check box is "partially checked"
  (`NodeStates::mixed` from MSAA's mixed state, UIA's indeterminate toggle, xa11y's
  `Toggled::Mixed`); a check box in a named group says its group, by JAWS's two rules: a focus
  names it entering the group's area, a quick key unless it comes from another of its controls;
  the containers a focus enters are said outermost first, the control's group among them; a
  state a page's script changed comes with the focus.
- **Disclosures** (disclosure-faq): with the virtual cursor off a button's hint is "To activate
  press spacebar."
- **Radio groups** (radiogroup-roving-tabindex): a heading that names a group is read; a checked
  ARIA radio button has no place in the virtual cursor, an unchecked one has; a focus into an
  ARIA radio group says an empty state after it; with the virtual cursor off, the radio's hint
  and a same page link as "Link"; Space or Enter checking an ARIA radio says it whole.
- **Accordions** (accordion): a button that is its heading's only content is "Billing Address
  heading level 3 | Button collapsed", the heading only in the virtual cursor.
- **Toggle buttons** (toggle-button): "Mute Toggle button Not pressed" on its line; pressing it,
  "Toggle button Pressed | Mute", released "Not pressed" as a word.
- **Switches** (switch, switch-button, switch-checkbox): "Notifications Switch Off", no hint,
  "On"/"Off" as toggled; a switch is a check box in both API mappings now (UIA had a toggle
  button, so its focus never met its line); Insert+Tab names no switch's group.
- **Insert+Tab** names the control's group and gives the hint only with the virtual cursor off;
  **Insert+Up** with the virtual cursor off says the focused control as its focus does (a
  checked ARIA radio with its place, a button without its heading, a link name first).

### Harness

- **Setup pages captured after their setup** (`capture-snapshot.ps1 -Keys`, `queue-batch.ps1
  -SnapshotKeys "Ctrl+End,Shift+B,Enter"`): a disclosure's expanded state, a radio's check.
- **The tracer reads a page's alert** (`role="alert"`): Chromium tells of it with an alert event
  alone, so the alert element is captured as a live region's (`64f0b79c`); staged on old-pc for
  alert's next round.
- `okur-lab aria-at-import` reads both of ARIA-AT's formats (1154 JAWS scenarios in 39 plans).

## Found, and how

- **A queued round gives up after six hours** of waiting (`-MaxWaitMinutes 360`, from its
  queueing): the fourteen rounds queued at 18:00 were taken back and queued again with 1440
  (skill troubleshooting §9).
- **Stopping a started round's queue process stops its batch** mid-scenario; round-aria4 was
  completed by round-aria4b.
- **JAWS's rules, from the recordings:** quick keys name a group a focus does not
  (2005/s1 against 2008/s1); a switch's group is never named by Insert+Tab (switch-checkbox's
  fieldset settled it, 2318/s1); a checked ARIA radio has no place in the virtual cursor, an
  unchecked one has (2084/s2, 2074/s2).

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 9976/10757 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 10129/10696 steps match exactly; 288 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `a13ca940`: 9651/10217. Each ARIA-AT round replayed with
`okur-lab parity -c <round>-preview`, its line as printed:

```
round-aria2 (checkbox-tri-state): JAWS parity (Ia2): 43/43 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-aria3 (disclosure-faq): JAWS parity (Ia2): 51/51 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-radiogroup (aria4, 4b): JAWS parity (Ia2): 76/76 steps match exactly; 3 accepted divergences not counted (0 scenarios could not run)
round-aria5 (accordion): JAWS parity (Ia2): 61/62 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-aria6 (checkbox): JAWS parity (Ia2): 46/46 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-aria7 (toggle-button): JAWS parity (Ia2): 48/48 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-aria8 (switch): JAWS parity (Ia2): 44/44 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-aria9 (switch-button): JAWS parity (Ia2): 44/44 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-aria10 (switch-checkbox): JAWS parity (Ia2): 42/42 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-aria11 (command-button): JAWS parity (Ia2): 18/18 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-aria13 (link-css): JAWS parity (Ia2): 16/16 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-aria14 (link-img-alt): JAWS parity (Ia2): 16/16 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
round-aria15 (link-span-text): JAWS parity (Ia2): 16/16 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace --no-fail-fast` (4 test threads): 1569 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 1128 scenarios, 3390 trials, 3373 succeeded.

Open: accordion 2160/s1 (Tab into a required field in a panel: JAWS gives no "Type in
text."); alert (round-aria12, traced before the alert capture; recorded again in round-aria12b);
2112/s1's provisional divergence (Enter's place on an ARIA radio) awaits the other radio groups.
Not checked live on the dev box.

## old-pc

- Queued, one round per plan, each waiting up to a day for the one before: round-aria12b
  (alert, with the staged tracer `sr-harness\staged-alert\okur-lab.exe`), then round-aria16 to
  aria29: radiogroup-aria-activedescendant, rating-radio-group, the two tabs, the three menu
  buttons, modal-dialog, minimal-data-grid, quantity-spin-button, the three sliders,
  disclosure-navigation (ids 2386-2858).
- Excel's round-1122b waits to be imported (plan 41).

## Next

The plan: process each round as it finishes (preview, fix, import); then the first format's
eleven plans; then the measure of assertions.
