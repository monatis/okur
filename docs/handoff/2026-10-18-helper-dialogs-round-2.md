# Handoff: JAWS's list dialogs, Space, speech off the hook, accepted divergences (2026-10-18)

Follows [`2026-10-18-old-pc-round-3.md`](2026-10-18-old-pc-round-3.md). Plan:
[`2026-10-18-helper-dialogs-round-2.md`](../plans/2026-10-18-helper-dialogs-round-2.md). Next
plan: [`2026-10-18-reading-settings.md`](../plans/2026-10-18-reading-settings.md).

## What was done

### JAWS's list dialogs, as real dialogs (okur-ui)

Every one is recorded on old-pc first, built on one shared list dialog
(`crates/okur-core/src/ui/lists.rs`), and compared live with `okur live-parity`.

- **Lists:**
  - the Links List (Insert+F7);
  - the Heading List (Insert+F6), with its level filters;
  - Select a Form Field (Insert+F5), with all 10 fields where Okur had found 8;
  - Virtual HTML Features (Insert+F3), all 22 lists by JAWS's names;
  - 13 element lists on Insert+Ctrl and a letter (`crates/okur-core/src/element_lists.rs`):
    radio buttons, buttons, combo boxes, edits, check boxes, graphics, lists, articles,
    paragraphs, block quotes, tables, divisions and tabs;
  - Document Regions (Insert+Ctrl+R), on a native tree view new in okur-ui.
- **What each list holds:** a snapshot test checks each element list's first items and count
  against JAWS's on the same pages (`element_lists_hold_what_jaws_lists`,
  `document_regions_are_jaws_tree`).
- **What OK does, from recordings 246-260:**
  - form controls and tabs take the focus, and an edit is then in forms mode;
  - the other lists move the virtual cursor to the element;
  - a table is entered as T enters it, with the cursor now on its first cell.
- **Empty lists:** an empty list says JAWS's own sentence (261-273), for example "no Buttons
  were found".

### Space with the virtual cursor (the owner's report)

- **Sources:** JAWS's key map gives Space its own script, `VirtualSpacebar`. Freedom Scientific
  documents that it toggles check boxes, selects radio buttons and activates buttons.
- **Recording (229):** a page that logs what reaches each control. Every control gets the focus
  and a click, never the Space key. An edit takes forms mode, a combo box opens, and on text the
  page gets nothing.
- **Okur now does the same.** Documented in
  [`forms-and-interaction-modes.md`](../references/forms-and-interaction-modes.md) §3.

### Bugs found by measuring live

- **Speech blocked the keyboard hook.** Every key stopped speech inside the hook, and SAPI's
  purge took 750 ms on old-pc. The key then went on to the application: a Space meant for an edit
  toggled the check box that had the focus. Speech now runs on its own thread
  (`crates/okur-platform/src/tts/thread.rs`), and the same live run logged no slow keys.
- **A select box in forms mode was silent** on Down and Up; it now says the new choice. Found by
  re-recording 205.
- **Select a Form Field didn't move the focus:** its focus request went out while Okur's dialog
  was still in front. Page actions now wait for the page to be in front.
- **A dialog opened from another lost the focus** to Chrome when the first one closed.

### Harness and corpus

- **Accepted divergences:** [`accepted-divergences.md`](../references/accepted-divergences.md)
  and the `divergences!` table. Every parity check applies them.
  - **The owner's ruling:** JAWS's version and other speech from its own windows before its
    dialogs.
  - **JAWS's typo:** the stray comma after "Radio Buttons List".
  - **For the owner's review**, from the audit: 312's emoji and 313's Say Font.
- `run-scenario.ps1 -OkurDebugLog`, and `okur live-parity` compares an unsettled run.
- **New recordings:** 227-230, 229 (Space), 231-274 (element lists and Document Regions).
  205 was re-recorded, replacing a trial that held old-pc's file names.
- **Documents:**
  - [`parity-gap-audit.md`](../references/parity-gap-audit.md), a subagent's audit;
  - [`jaws-reading-settings-research.md`](../references/jaws-reading-settings-research.md), a
    subagent's research for the next round.
- `main` holds all of it, through `038d93c` plus this handoff.

## Results (pasted tool output)

`okur parity` (IA2, spoken) and `okur parity --api uia` on `main`:

```
JAWS spoken parity: 529/950 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 1184/1512 steps match exactly; 8 accepted divergences not counted (0 scenarios could not run)
JAWS parity (Uia): 1068/1512 steps match exactly; 8 accepted divergences not counted (0 scenarios could not run)
```

`okur live-parity`, Okur run live on old-pc with the deployed build (run files in
`C:\Users\ailabs\sr-harness\live\`, copies in `target/live/`):

```
222-web-links-list              Okur live: 11/11 steps match JAWS exactly
224-web-headings-list           Okur live: 11/11 steps match JAWS exactly
225-web-form-fields-list        Okur live: 10/11 steps match JAWS exactly
227-web-form-fields-list-items  Okur live: 11/11 steps match JAWS exactly
228-web-form-fields-list-edit   Okur live: 11/12 steps match JAWS exactly
226-web-element-lists           Okur live: 5/5 steps match JAWS exactly
230-web-element-lists-items     Okur live: 26/26 steps match JAWS exactly
229-web-space-activates         Okur live: 19/22 steps match JAWS exactly
232-web-list-buttons            Okur live: 8/8 steps match JAWS exactly
237-web-list-lists              Okur live: 8/8 steps match JAWS exactly
247-web-list-buttons-enter      Okur live: 5/5 steps match JAWS exactly
249-web-list-edits-enter        Okur live: 4/5 steps match JAWS exactly
252-web-list-lists-enter        Okur live: 5/5 steps match JAWS exactly
257-web-list-tables-enter       Okur live: 5/5 steps match JAWS exactly
262-web-list-buttons-none       Okur live: 2/2 steps match JAWS exactly
271-web-list-tables-none        Okur live: 2/2 steps match JAWS exactly
274-web-list-regions-items      Okur live: 7/8 steps match JAWS exactly
241-web-list-regions            Okur live: 7/8 steps match JAWS exactly
256-web-list-regions-enter      Okur live: 4/5 steps match JAWS exactly
```

Each line also says "0 accepted divergences not counted". Where an opening step counts as a
match, the accepted divergence for JAWS's speech before its dialog was applied (a phrase, not a
whole step).

## What is left

- **The field's wording on focus:** after a list focuses an edit, JAWS repeats the field where
  Okur says "Type in text." (225/s10, 228/s9, 249/s3).
- **A tree child's parent name:** JAWS adds it ("level 1 Primary Navigation 1 of 1 Banner";
  274/s2, 241/s2, 256/s2).
- **Space** (229): Okur names the combo box before its opened list, and JAWS's Escape there moves
  to Chrome's menu button (a JAWS quirk, not copied).
- **Lists not built:** Smart Glance Highlights, Anchors, Frames, List Items, PlaceMarkers.
- **205/s6:** JAWS varied on the radio button's position after Space.
- **For the owner** (decided 2026-10-19; see the next plan's "Owner decisions"):
  - review the two proposed divergences;
  - decide whether `.sbl` symbol files and `.jsi` personalized-settings files may be read (they
    are needed for the next round).
- Everything left, from this round and earlier ones, is listed in
  [`docs/backlog/open-items.md`](../backlog/open-items.md).
