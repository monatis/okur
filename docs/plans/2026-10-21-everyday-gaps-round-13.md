# Plan: everyday-use gaps, round 13 (2026-10-21)

Follows [`../handoff/2026-10-21-everyday-gaps-round-13.md`](../handoff/2026-10-21-everyday-gaps-round-13.md).
The owner's order stands: the README's "Not yet" list, in its order, and Okur's settings made
as customizable as JAWS's
([`../references/jaws-customization-map.md`](../references/jaws-customization-map.md)). Each item is
found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents become accepted
divergences. old-pc always has a round queued behind the running one. Okur's own words never
use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with
`okur-lab parity`, then a dated handoff and the next plan, with AGENTS.md pointing at them. A
stretch is a few hours or about thirty commits, not more.

## 0. What old-pc records meanwhile

| Round | What | Then |
|---|---|---|
| 153 (`waiter153`) | 716: Flexible Web hides the advertisement, the page read, Undo, the page read again | import; it tells what hiding does and how Undo reads |
| 154 (`waiter154`) | 717-719: 408 with `IndicateCaps=0`, `SayAllIndicateCaps=1`, `LinePauses=1` | import; build the three settings |

For each round:
- Fetch it with `lab-fetch.sh`.
- Check that every trial passed, that `page_focus.in_document` and `jaws_in_page` hold, and
  that the capture script's and the scenario's hashes match the repository's.
- Grep for links, the lab account's folders and the owner's name.
- Commit the corpus alone, with the baseline made from the committed code. Then commit the
  code with its own baseline.

## 1. Flexible Web (the README's next item)

From 705, 708, 716 and the guide (Surf's Up, chapter 16):
- **A property-sheet wizard in `okur-ui`**, as JAWS's own:
  - a `PropertySheetW` wizard (`PSH_WIZARD`), its pages made from `okur-ui`'s in-memory
    dialog templates;
  - `ToUi` messages to show the wizard, change its page, and enable Back, Next and Finish;
  - parity stands JAWS's traced wizard in for Okur's, as the dialog mirror does for the
    PlaceMarker List.
- **The wizard's pages**, as 705 and 708 read them:
  - What would you like to do? (five radio buttons, those not yet possible disabled, and
    Enable Flexible Web);
  - Choose an Action (Hide an element, Start reading at an element);
  - Choose an element (the element at the cursor and the ones around it);
  - Choose a customization ("Hide X 1 from the top", "... from the bottom", "Hide all Xs").
- **Reading a wizard's page as JAWS does**: "What would you like to do?" where the page's
  name was (705/s3); the question, then "Choose an Action dialog", then the question again
  (705/s9); "Choose an element dialog" before the list (705/s15); Escape silent (705/s21).
  This is Okur's native reading of any property-sheet wizard, not only its own.
- **Customizations**: hide an element, or start reading at one. A customization is kept for
  the page until it is left, and Undo and Retry are offered on the first page, as 716 shows.
  Rules saved for a page, a site or all sites come after; they go in a store of their own,
  as PlaceMarkers do.

## 2. Settings

- **From round 154**: Caps Indicate (`IndicateCaps`), capitals in Say All
  (`SayAllIndicateCaps`) and line pauses (`LinePauses`, Say All Reads' line with and without
  pauses). Each is one table row, a catalog label and the behavior.
- **The table's next steps** (the map's §7.6, step 3):
  - Settings Center's group tree, its search box and each setting's description;
  - Quick Settings with lifetimes (saved, until Okur exits, until the focus moves);
  - the voice profile's rate and punctuation in the JAWS import.
- **The map's next settings**, one recording per setting:
  - web verbosity levels;
  - the verbosity output types dialog;
  - numbers and word splitting;
  - pitch and voice contexts.

## 3. Harness

- **Say All progress in replay**: a page-load reading or Insert+Down that JAWS's next key cut
  short leaves JAWS's cursor where its reading got to (507, 538, 539, 712/s2). The replay
  synthesizer could report progress as far as JAWS's own transcript of the step goes, so
  that Okur's cursor ends where JAWS's did.
- **`okur-lab-keys` installed on old-pc** once the owner has imported the development
  certificate: 517 (the Speech History's Escape) and 627 (the clipboard keys) wait for it.

## 4. The rest of plans 11 and 12

These items still wait:
- PlaceMarkers' duplicate checks and the Display PlaceMarkers For filter;
- Line without Pauses (several lines in one utterance), if round 154 shows it;
- the Okur cursor moving the mouse by line and reading outside text fields;
- 689/s1 and 676/s3;
- the Word List and skim reading;
- the rest of the verbosity levels;
- Run Okur Manager (Insert+F2) with its list;
- notifications;
- starting at sign-in;
- Office.

## 5. Owner

- Import the development certificate on old-pc (`sr-harness\signing\` in the shared Dropbox).
- Code signing for releases (Certum's open source certificate, when the owner chooses).
