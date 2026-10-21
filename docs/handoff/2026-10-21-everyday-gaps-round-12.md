# Handoff: everyday-use gaps, round 12 (2026-10-21)

Follows [`2026-10-21-everyday-gaps-round-11.md`](2026-10-21-everyday-gaps-round-11.md) and covers
the commits after it (`197232e` to `0596b74`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-11.md`](../plans/2026-10-21-everyday-gaps-round-11.md)
(the README's "Not yet" list in its order, the owner's decision). The next plan is
[`docs/plans/2026-10-21-everyday-gaps-round-12.md`](../plans/2026-10-21-everyday-gaps-round-12.md).

## The owner's answers and requests

- **JAWS's customizability.** The owner pointed out that almost everything in JAWS can be
  turned on, off or configured, and asked for research on which of those settings Okur
  matches, so that Okur's settings can be extended or redesigned. A research agent wrote
  [`docs/references/jaws-customization-map.md`](../references/jaws-customization-map.md)
  (`0596b74`). It maps 134 JAWS settings and facilities from Freedom Scientific's public
  documentation, each with its source: 9 built in Okur, 41 partial, 84 missing. The partial
  ones are mostly JAWS's default with no way to change it. It proposes one declaration per
  setting, from which the dialog, its search, the command line, the files' reference and the
  JAWS import used by replay are generated. The next plan takes this up (§3).

## Decided without the owner (reversible)

- **After a custom label, Okur's cursor stays on the element** (`eff21d4`). JAWS builds the
  page again and its cursor goes to the top (688/s15): an accepted divergence.
- **A PlaceMarker returns to its character; one anchored to text finds that text** (`32007da`),
  as JAWS's Add dialog offers ("Anchor to text", 701). The first build kept every marker's
  line to find it again; that was Okur's invention and is gone.
- **K without PlaceMarkers opens the PlaceMarker List once it was opened since the page
  loaded** (`32007da`). On a fresh load K says "There are no PlaceMarkers on this page." (703/s1,
  221); after the list was opened, K opens it (703/s4, 699/s12, 702/s30), in every trial.
- **Escape in Add, OK, Yes to Remove All and Remove all close the PlaceMarker List** (`36ad58f`,
  `d58d8f7`), the focus going back to the page quietly (701/s11, 702/s6, s28).
- **Accepted as JAWS's accidents** (each in
  [`accepted-divergences.md`](../references/accepted-divergences.md)): JAWS reading the page
  again after a PlaceMarker is added (702/s8-s10); its silent Say Line after the PlaceMarker
  List that K opened (701/s15, 703/s6); its version, main window or list said before the
  PlaceMarker List (699/s12, 701/s4, s13, 703/s2, s4).
- **704 is not imported** (round 141). Remove closed the list in its first trial, so Remove
  All never reached the second PlaceMarker, and trials 2 and 3 started with it: their speech
  disagrees with the first. Version 2 (`884c3c7`) removes one, opens the list again and
  removes the rest. A lab cleanup run, never imported, removes what round 141 left.

## What was done

### Features (each checked with `okur parity` against old-pc's evidence)

- **Custom labels** (`197232e`, `eff21d4`): Insert+Ctrl+Tab opens Custom Label for a form
  field, button, link or graphic; OK gives the element the label typed, kept per site in
  `labels.toml`. Its dialog reads as JAWS's: a static text naming the focused field is the
  dialog's prompt when the field takes no access key from it (688/s9, also 559/s9, s14). On
  text: "A Custom Label cannot be created for this item." 688 matches in all its own steps;
  its Run JAWS Manager steps (Insert+F2, used to delete JAWS's labels) are not Okur's.
- **Escape from Select a Form Field reads the field the cursor is on** (`eff21d4`), the page's
  name only at its top (688/s19, 227/s11).
- **The cursors' short messages at Intermediate and Advanced** (`bb35c07`): 695-698.
- **Saved PlaceMarkers** (`b17eaad`, `d58d8f7`, `32007da`, `36ad58f`), from JAWS's guide
  (chapter 15) and 699-703:
  - kept per page address in `placemarkers.toml`; one "defined for all pages on current
    domain" is offered on its site's other pages;
  - Ctrl+Shift+K opens Okur's PlaceMarker List: a two-column list ("PlaceMarker", "Generalized
    to Domain"), nothing chosen as it opens; Add..., MoveTo, Change..., Remove, Remove All,
    Cancel; Display PlaceMarkers For URL, Domain, All. MoveTo, Change and Remove become
    available, and MoveTo the default, once a PlaceMarker is chosen;
  - Add proposes the line at the cursor as the name; Name, "Define for all pages on current
    domain", "Anchor to text" and its "Anchor Text:" field, OK, Cancel;
  - Remove All asks "Are you sure you want to remove all PlaceMarkers?";
  - K and Shift+K go over saved and temporary markers; 7, 8, 9, 0 say PlaceMarker 1 to 4,
    Shift with them moves there; "No place marker #2".
  699, 701, 702 and 703 match in every step not accepted as JAWS's accident.
- **Dialogs over dialogs** (`d58d8f7`): the focus names the dialogs the active one opened over
  ("PlaceMarker List dialog | Add PlaceMarker dialog | Name: Edit"); okur-ui's dialog opened
  from another of Okur's is owned by it, so live UI Automation shows the same chain.
- **An empty list view** is read "PlaceMarker List view | 0 items | To move to items use the
  Arrow keys."; Say Line there says nothing (699/s6, s13).
- **okur-ui**: a Table control (column headers, rows, none chosen), and DialogEnable and
  DialogDefault messages.

### Harness

- **`lab-fetch.sh` hashes files from standard input** (`922814f`): with a Windows path for the
  local folder, sha256sum marked its line with a backslash and every fetch "failed".
- **A web trial starts with UI Automation's focus in the page, and checks JAWS reads it**
  (`be2b382`, `a5604bc`, `c603bcf`). Rounds 138, 139 and 142 each had a scenario whose every
  key reached no page: JAWS said only the window's title, and the trials counted as
  successes. The setup now asks `okur focus` (new: the focused element as UI Automation
  reports it, and whether it is in a document), presses F6 when it is not, then presses
  Insert+Up: said with the browser's name ("... - Google Chrome"), the lab Chrome is started
  again once, and a trial still outside the page fails. The first check, by the window class
  with the keyboard focus, failed every trial of round 140: Chrome keeps that focus on its
  frame window.
- **Speech On Demand can be recorded** (`8663608`, `c603bcf`): with less speech on, JAWS does
  not speak its copy confirmation, which every step of round 138's 700 waited for; such steps
  read what the recorder heard once the audio is silent.
- **A step's `onset_ms`** (`3ecb2c0`), for speech JAWS starts late (its reading of the page
  after a PlaceMarker is added).
- **A snapshot of a page still loading is captured again** (`128a62c`).
- **Replay**: the dialog mirror keeps a stack of dialogs, tables, the default Okur sets and
  access keys (Alt+V for "Remo&ve All"); typed text reaches an edit as well as an edit combo;
  a focus on JAWS's own main window (its version text) is JAWS's alone.

### Imported

Each import had all trials passed, the committed capture script, the page's focus checked
where the harness had it, and was grepped for links and personal data.

| Commit | Scenarios |
|---|---|
| `0e6d976` | 695-698 (round 134) |
| `31d2b19` | 691, 692 (round 135) |
| `854080a` | 688 (round 133), with the custom-label snapshots (round 137) |
| `7f608e9` | 699 (round 137), 702 (round 139), with the placemarkers IA2 snapshot |
| `05e5a64` | 701, 703 (round 141), with the placemarkers UIA snapshot (round 140) |

Not imported:
- round 138's 700 and round 139's 701: JAWS was outside the page for every key;
- round 140's 701 and 703: the first page-focus check failed every trial;
- round 141's 704: its trials disagree (above);
- round 142's 700 (good, but its steps after Shift+S hold the copy confirmation in the
  spoken layer) and 705 (JAWS outside the page): both are recorded again in round 144.

## Figures

Each is the named command's output at `0596b74`:

- `okur parity`: "JAWS parity (Ia2): 5106/5302 steps match exactly; 116 accepted divergences
  not counted (0 scenarios could not run)". At the previous handoff: 4942/5132 with 110.
- `okur parity`, spoken layer: "JAWS spoken parity: 4973/5264 steps match exactly".
- `okur invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 364 scenarios, 1146 trials, 1146 successful.
- `cargo test --workspace`: 1281 passed, 0 failed, 199 ignored.

## Learned

- **JAWS's PlaceMarker List** (699, 701-704): its list is named "PlaceMarker"; a row reads
  "Not Selected Water in a closed bottle is allowed; meals are not. | Document | 1 of 1", and
  chosen with Down Arrow, "… | Document | 2 of 2". With one row not chosen, Tab goes Add...,
  Remove All, Cancel, the radio button; with one chosen, Add..., MoveTo, Change..., Remove,
  Remove All, Cancel. Remove All asks in "PlaceMarker RemoveAll Warning" with Yes. Adding at
  a place that has one gives "PlaceMarker Add Error: The placemarker has already been defined
  at this location."; a name in use gives "A PlaceMarker with the same name already exists on
  this page. Press enter now to replace … or press escape to choose a new name." (704,
  round 141; not yet built).
- **JAWS reads the page again after a PlaceMarker is added**, from its top, a moment after OK.
- **Speech On Demand** (700, round 142): keys outside the key map's [Speech On Demand] list
  are not spoken (Down, H, Ctrl+Right, Tab), but JAWS still writes what they would say into
  its speech history ("banner", "list end"); Insert+Up, Insert+T and Insert+5 speak. Shift+S
  in the speech layer says "Full Speech" or "Speech On Demand". So Okur can build it as a
  filter in front of the synthesizer, its speech history unchanged.
- **Flexible Web** is Insert+Space then X (`DoFlexibleWebDialog`); outside a page JAWS says
  "You must be in the web page area to activate Flexible Web."

## Recorded meanwhile, and queued

On old-pc, in this order:
1. round 143 (waiter143, push `351e2cc6`): the lab cleanup of the reading room page's
   PlaceMarkers (never imported), then 704 version 2;
2. round 144 (waiter144, push `f0429ec5`): 700 and 705 again, with the in-page check.

## Not done, and why

- **Speech On Demand in Okur**: its recording is clean in the history layer; the build waits for
  round 144's spoken layer, and the plan puts it right after PlaceMarkers.
- **Flexible Web**: round 142's recording had JAWS outside the page; 705 is recorded again.
- **The settings registry and the map's additions**: the owner's new request; the next plan.
- **PlaceMarkers' duplicate checks, Display For filters**: after 704 version 2.
- **Line without Pauses, the Okur cursor moving the mouse, the Word List, the rest of the
  verbosity levels, the owner's items**: unchanged from plan 11.
