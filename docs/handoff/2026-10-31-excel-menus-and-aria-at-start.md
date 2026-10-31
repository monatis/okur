# Handoff: Excel's menus, then ARIA-AT's test plans begun (2026-10-31, third)

Follows [`2026-10-31-excel-round-1.md`](2026-10-31-excel-round-1.md) and covers `77a4a50a` to
this one. Plans: [`../plans/2026-10-31-everyday-gaps-round-41.md`](../plans/2026-10-31-everyday-gaps-round-41.md)
until the owner paused Excel, then
[`../plans/2026-10-31-aria-at-web-round-1.md`](../plans/2026-10-31-aria-at-web-round-1.md), the
active plan, followed by [`../plans/2026-10-31-aria-at-web-round-2.md`](../plans/2026-10-31-aria-at-web-round-2.md).

The owner, during this stretch:
- Excel first, then W3C's ARIA-AT reports and tests as a source of web fixtures and cases, then
  the first release through the Microsoft Store as an MSIX (individual developer account, the
  name "okur" reserved): plans `2026-10-31-aria-at-web-round-1.md` and
  `2026-10-31-microsoft-store-release.md`, backlog rows 44 and 45.
- Then: pause Excel, start ARIA-AT now. Excel continues later.
- old-pc lost power once and was restarted; the dev box too, once. The dev box is shared: cargo
  on 4 of 16 cores, one heavy job at a time.

## What was built

### Excel (`77a4a50a` to `82356971`)

- **Selections** said as JAWS says them (1117, 1120, 1121; the last handoff's open item).
- **Insert+Shift+S** opens the sheet tabs' menu: Excel's `Ply` command bar, `ShowPopup` on a
  thread of its own (Excel holds the call until the menu closes). A popup's start carries its
  window's class and caption; an Office command bar's popup is said by its name, "Context Menu
  | Ply pop-up menu | Insert... | To move through items press up or down arrow." [obs, 1116/s5,
  1122/s5].
- **Office's own menus** (NetUI, Shift+F10 on a cell, in a "Net UI Tool Window"): the unnamed
  pane focused before the first item no longer ends the menus, and the menu is its first item
  and how to move, "Cut | To move through items press up or down arrow." [obs, 1122/s2].
- **Accepted:** Word's letter said twice, then phonetically, onto a new page (1119/s3); Excel's
  cell menu closed by the recorder's copy of the speech history (1122/s2, a harness effect).

### ARIA-AT (`0e78e47a` to `6c28791e`)

- **`okur-lab aria-at-import`** makes one of ARIA-AT's plans into scenarios: the plan's test
  page copied unchanged with its scripts and styles to `corpus/fixtures/aria-at/<plan>/` (with
  `NOTICE.md` for the W3C's licenses), a scenario per JAWS command of each test, the test's
  assertions with it (`aria_at`). Both of ARIA-AT's formats: 1154 JAWS scenarios in 39 plans. A
  setup page a plan's folder lacks is made as ARIA-AT's build makes it.
- **Fixtures in folders:** the corpus keys a fixture by its path under `fixtures/`; the
  manifest lists folders' files; snapshots are named with dots for slashes; `lab-deploy` and
  `queue-batch` take folders.
- **The reset** is Ctrl+End, Shift+B, Enter: JAWS's Ctrl+End lands on "main region end", and
  Shift+B goes back to the page's last button, the "Run Test Setup" button. The recorder stamps
  each reset key's time (`reset_keys`), and replay presses the keys among the events traced
  before step 1 in that order.
- **What the first plans found in Okur, and fixed:**
  - a tri-state check box between checked and not is "partially checked" (`NodeStates::mixed`
    from MSAA's mixed state, UIA's indeterminate toggle, xa11y's `Toggled::Mixed`;
    `NodeStates::check_word`);
  - a check box in a named group has its group named before it, by JAWS's two rules: a focus
    names it entering the group's area, a quick key unless it comes from another of the group's
    controls;
  - Insert+Tab on a page names the control's group and gives the usage hint only with the
    virtual cursor off;
  - a check box whose state a page's script changed takes the state its focus brings;
  - with the virtual cursor off, a page's button says "To activate press spacebar."

### Harness

- **The recorder's text step** types each character by its key on the layout in front: a
  character sent as Unicode input carries itself in the key's scan code, which JAWS read as a
  key ("7" was the numpad's star, its right mouse button). The skill's troubleshooting §8.
- `preview-round.sh` copies pending fixtures in folders and the round's snapshots.
- The tracer's waiting queue drops its oldest events, not the newest, and warns; a UI Automation
  notification from a process behind is logged by its length only (review 2026-10-25, 20).

## Found, and how

- **Outlook on old-pc** opens only its account wizard: recording it needs a mail account there,
  the owner's to decide.
- **Office's NetUI menus close when their window loses the foreground**: the recorder's copy of
  JAWS's speech history did it, 2.5 s into the step (1122/s2's trace).
- **A keeper document marked saved is replaced** by the fixture Word opens, and Word quits with
  the fixture: the keepers stay changed and unsaved (Word's Document1, Excel's Book1, "lab").
- **ARIA-AT's reports** include three of old-pc's own JAWS build (2025.2505.43): accordion,
  disclosure-faq, radiogroup-roving-tabindex ([`../references/aria-at-research.md`](../references/aria-at-research.md)).
- **JAWS's group rules** (checkbox-tri-state): Shift+Tab from a link inside a fieldset to its
  check box does not name the group (2008/s1); Shift+X from the same link does (2005/s1).

## Proof

`okur-lab parity` at this commit:

```
JAWS spoken parity: 9519/10278 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 9651/10217 steps match exactly; 285 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `b2f2c2c5`: 9577/10149. `okur-lab parity --only` for this stretch's Excel and
Word scenarios:

```
1116: JAWS parity (Ia2): 8/8 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1119: JAWS parity (Ia2): 6/6 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1122: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

ARIA-AT's rounds replayed with `preview-round.sh <round> target/debug/okur-lab.exe` at this
commit: round-aria2 (checkbox-tri-state, imported), and round-aria3 (disclosure-faq, 36
scenarios, not imported yet):

```
JAWS parity (Ia2): 43/43 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
JAWS parity (Ia2): 33/51 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

disclosure-faq's differences are its setups that expand a disclosure: its pages' snapshots were
captured at load, before the setup, so the buffer has the button collapsed and its answer
hidden. The capture now runs the setup first (`capture-snapshot.ps1 -Keys`, `queue-batch.ps1
-SnapshotKeys "Ctrl+End,Shift+B,Enter"`); round-aria4b captures disclosure-faq's and
radiogroup-roving-tabindex's pages again so.

`cargo test --workspace --no-fail-fast` (4 test threads): 1569 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0
violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's
statistics: 777 scenarios, 2337 trials, 2320 succeeded.

Not checked live: any of this in Okur's own run on the dev box.

## old-pc

- Restarted after its power loss at 12:13: JAWS's voice profile read `Rate=90`; the master
  volume 1%, not muted (`sr-harness\experiments\volume.ps1`, new). Keepers: Excel's Book1,
  Word's Document1.
- **Waiting to import** (Excel, paused): round-1122b, 1123 typing in cells, 1124 Excel's dialogs,
  1125 Word's context menu, recorded with the text step typing by keys; their scenarios in
  `corpus/pending/`.
- **Recording** ARIA-AT, one round per plan: round-aria3 disclosure-faq (done, to import with
  its pages captured again), round-aria4 radiogroup-roving-tabindex (stopped after 27 of 55
  scenarios, 2067-2093, while the queued rounds were taken back to add `-SnapshotKeys`),
  round-aria4b (its other 28, 2094-2121, JAWS started afresh first, and disclosure-faq's and
  radiogroup's pages captured after their setups), round-aria5 accordion, then round-aria6 to
  aria15: checkbox, toggle-button, switch, switch-button, switch-checkbox, command-button,
  alert, link-css, link-img-alt, link-span-text (ids 2122-2385). round-aria1 (the first reset)
  was stopped and is not imported.
- **Stopping a queue** (`queue-batch` runs its batch in its own process): stopping a waiting
  round's process stops it, but one whose batch already started stops mid-batch too, and a PC
  cursor scenario cut off may leave the virtual cursor off: queue the next round with
  `-FreshJaws`.

## Next

[`../plans/2026-10-31-aria-at-web-round-2.md`](../plans/2026-10-31-aria-at-web-round-2.md).
