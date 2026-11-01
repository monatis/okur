# Handoff: ARIA-AT's plans all in; probes of radio groups, descriptions, dialogs and selections (2026-11-01, fifth)

Follows [`2026-11-01-aria-at-combo-boxes-and-landmarks.md`](2026-11-01-aria-at-combo-boxes-and-landmarks.md)
and covers `4d39774c` to this one, made under
[`../plans/2026-10-31-aria-at-web-round-2.md`](../plans/2026-10-31-aria-at-web-round-2.md).

Where things stand: every ARIA-AT test plan with JAWS commands is in the corpus (40; menubar-editor
has none), the last four since the last handoff (main, meter, rating-slider, slider-multithumb).
Five probes of our own answered the questions the plans left (1127-1131). Two ARIA-AT steps are
open. The plan's §5 comes next: Excel's round-1122b, then the Store release.

## What was built

### From ARIA-AT's last plans

- **A meter** (`e9ee9f86`): "Central Processing Unit (CPU) Usage meter 36.1665" on its line and
  focus, which JAWS skips (ARIA-AT MUST); a page's progress bar or meter changing without the
  focus says its new value, "86.0274", as JAWS does: the converter had dropped value changes of
  anything but the focus.
- **A named separator** is read by its name, "End of Example separator" (`12f6bcaa`).

### From the probes

- **Descriptions on the virtual cursor's lines** (`6b929320`, 1129): "heading level 2 Shipping
  options Orders over fifty dollars ship free.", "Delete account Button | This cannot be undone.",
  a link's after its text. Insert+Alt+R, which JAWS's hint "Use JawsKey+Alt+R to read descriptive
  text" names, said nothing in every trial, and no key map of JAWS 2025 binds it: the hint is an
  accepted divergence.
- **A dialog holding the focus keeps the virtual cursor** (`765a82d5`, 1130), aria-modal or not:
  Ctrl+Home its heading, Ctrl+End its last button, H wrapping inside it. A dialog without a
  description says its text before the focused control as the focus enters it, "Settings dialog |
  Choose your options. | Save Button", its controls' labels and descriptions left out (IA2's
  labelFor and descriptionFor). A page's first focus inside a dialog names the dialog, not the
  page (`6b929320`).
- **A clickable grid cell** says "clickable" after its text in the virtual cursor (`7674e1ad`),
  from Chromium's "click ancestor" default action in the grid's snapshots captured again; not a
  cell holding a link, not with the virtual cursor off: minimal-data-grid's last open steps.
- **Say Selected Text reads a page field's selection** (`d9d0916f`, 1131): "Selection is |
  Alabama" where it said "Nothing selected".
- **Radio groups** (1127, 1128): no property of the pages tells where JAWS says a group's silent
  empty state (a DOM id, a focusable group and a label's place were each tried); the rating
  group's steps and the probe's are accepted, as are rating's and tabs' places within their
  groups, which JAWS leaves out there and says elsewhere (`95b84a3c`).

### Accepted divergences

The slider's and spin button's limits, now in their snapshots, as a newer JAWS says them (22
steps); the sliders' hints on a virtual cursor Tab; a heading broken by an abbreviation (JAWS
reads the abbr's title although its Expand Abbreviations setting is off, Default.JCF); the
meter's lines.

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 11066/11951 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11321/11890 steps match exactly; 362 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `fbcbe5d7`: 11125/11725 with 343 accepted. The probes, with `okur-lab
parity --only`:

```
1127: JAWS parity (Ia2): 20/20 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1128: JAWS parity (Ia2): 15/15 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1129: JAWS parity (Ia2): 15/16 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1130: JAWS parity (Ia2): 12/13 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1131: JAWS parity (Ia2): 8/8 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab aria-at-report`, its total line (assertions, judged, Okur, JAWS, Okur only, JAWS
only, MUST, Okur, JAWS):

```
total                                           4604    4262    3866    3641        230          5    3110  2934  2858
```

`cargo test --workspace --no-fail-fast` (4 test threads): 1575 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 1906 scenarios, 5724 trials, 5707 succeeded.

Open:
- 1129/s11: Okur reads a dialog's start line ("Address Added dialog") where JAWS reads none, in
  the virtual cursor anywhere in the corpus. The dialog's lines also bound its confinement
  (`active_modal_range`), so removing them changes the buffer's container bookkeeping: its own
  slice.
- 1130/s11: JAWS's Shift+Tab out of a focused dialog said the dialog's button again; unexplained.
- 2857/s1 (disclosure-navigation): JAWS's "Moved to line 18".

Not checked live on the dev box.

## Waiting for the owner

Two everyday choices ARIA-AT's MUST assertions raise, each changing what Okur says on every page:

1. **The containers a quick key enters** (from the last handoffs): "MainRegion" before H's first
   heading in main, "Breadcrumb NavigationRegion | list with 4 items" before U's link. About 110
   of the 176 MUST assertions Okur leaves unmet are these. Always, never (as JAWS), or a setting?
2. **A field's selected text on a focus**: Chromium selects a field's whole text as Tab comes into
   it, and ARIA-AT's combo box plans require "selected" said then (22 MUST assertions). JAWS says
   nothing (1131). Saying it would add "selected" to every Tab into a filled field. Always,
   never, or a setting?

## old-pc

Idle: every round queued today is recorded and imported (round-aria30 to aria40, the five
probes). Excel's round-1122b and round-1062b wait to be imported (plan 41).

## Next

The plan's §5: Excel's round-1122b (typing in cells, its dialogs, Word's context menu), then the
first release through the Microsoft Store
([`../plans/2026-10-31-microsoft-store-release.md`](../plans/2026-10-31-microsoft-store-release.md));
the owner's two answers when they come; the dialog lines' slice.
