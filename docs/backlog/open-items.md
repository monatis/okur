# Backlog: what is known and not done yet

A living list, so nothing found along the way is forgotten. Each item says where it was seen:
a scenario and step, a handoff or the parity gap audit. When an item is done, it moves to "Done"
at the end with its commit. An item whose JAWS behavior is not recorded yet says so: per the
owner's rule, the documentation comes first, then a recording on old-pc, then the code.

Where the JAWS behavior is known, AGENTS.md rule 6 also asks for an `#[ignore = "known gap: ..."]`
test asserting it. Few exist yet (see "Measurement" below).

The active work is in [`2026-10-18-reading-settings.md`](../plans/2026-10-18-reading-settings.md):
punctuation, typing echo, key echo, character reading, words and numbers, capitals, verbosity,
and per-application and per-site settings. Those are not repeated here.

## Everyday use

[`everyday-use-gaps-2026-10-19.md`](everyday-use-gaps-2026-10-19.md) compares JAWS 2025's
help, manual and tutorials with Okur. It lists 43 gaps ranked for everyday use, 13 places
where Okur contradicts JAWS's documentation, and the order of work. P0:
- the reading keys on native controls read the last web page;
- Say All does not move the cursor;
- selecting and copying in the virtual cursor;
- JAWS Find, and Ctrl+F is blocked meanwhile;
- Insert+Shift+Down in a native edit;
- Home, End, PageUp and PageDown in the virtual cursor;
- Word and Outlook.

Scenarios 500-507 record them (old-pc round 12).

## Dialogs and lists

- **A tree child's parent name.** JAWS adds it after the position: "level 1 Primary Navigation 1
  of 1 Banner" (274/s2, 241/s2, 256/s2). Its WinForms tree doesn't (303/s8). Not explained yet.
- **Lists not built:** Smart Glance Highlights (Insert+Ctrl+Y), Anchors, Frames, List Items and
  PlaceMarkers (Ctrl+Shift+K). Virtual HTML Features says "Okur has no … yet" for them. Each
  needs its recording first. Smart Glance items were recorded (243: "9 p.m.", "most", "Branch"),
  but which text JAWS counts as highlighted is not known.
- **Space on a combo box** (229/s21): Okur names the combo box before its opened list; JAWS says
  the list only.
- **The rest of the helper dialogs plan** ([`2026-10-17-helper-dialogs.md`](../plans/2026-10-17-helper-dialogs.md)):
  JAWS Find, the reading highlight, routing, PlaceMarkers, and JAWS's Settings Center.
- **The rest of the message catalog groundwork** (M0, plan 2026-10-18 dialog round, step 2):
  - role and state marks still written by hand in `SpeechFormatter` and `linearize.rs`;
  - voice contexts and short message forms;
  - okur-ui's Virtual Viewer window title and its "Ready";
  - how JAWS's `ACCESS_KEY=a|b|c` fields map (check by changing the setting on old-pc).

## Reading settings

- **Punctuation per voice context** (reading, typing, messages): JAWS sets it per context in
  Voice Adjustment; Okur has one level.

## Virtual cursor keys not built

The owner found N unbuilt (2026-10-19). The whole key map was then compared with Okur's:
[`jaws-quick-navigation-keys.md`](../references/jaws-quick-navigation-keys.md) lists every
virtual cursor key of JAWS 2025 (`Default.JKM` on old-pc), what Freedom Scientific documents
for it, and Okur's status. Scenarios 289-299 record the gaps on old-pc; `virtual-keys.html`
is their new fixture.

Order of work, most used first:
1. N and Shift+N, Insert+Enter: non-link text (289). Done 2026-10-19.
3. S and D, Ctrl+Insert+arrows: same and different element (290, 558). Done 2026-10-20.
3. S and D, Ctrl+Insert+arrows: same and different element (290).
4. Alt+Insert+Home and End, Ctrl+Alt+Insert+1..6: first and last heading (295). Ctrl+Insert+Home
   and End: first and last field (296).
5. Win+Alt+arrows, Alt+Win+Numpad5 and Period: table rows and columns (297).
6. Z, Dash, Y: divisions, separators, Smart Glance highlights (292).
7. Shift+Period and Shift+Comma, Alt+Win+Home and End: step past an element (294).
8. Insert+N, Insert+Shift+Z, Insert+Z, Insert+Escape: quick keys and the virtual cursor on and
   off (298).
9. Backslash, Slash, Equals: mail links, clickable elements, flow to (293).
10. J and W: Jump to Line, Word from List (299).
11. Not recorded yet:
    - V (visited links), which needs IA2's traversed state;
    - PlaceMarkers (7-0, K, Ctrl+Shift+K, Ctrl+Win+K), in the helper dialogs plan;
    - the element information (Insert+Shift+F1), F8, Ctrl+Insert+F7;
    - the clickable and mouse-over lists, routing (Insert+Delete), Insert+F9, skim reading;
    - PageUp and PageDown during Say All.

## Forms and browse mode

- **Forms with Up and Down in browse mode** (the owner's request, 2026-10-18).
  - Recorded: 201, 278 and 279 with Auto Forms Mode off; 280 with it on.
  - Done: with Auto Forms Mode on, an arrow onto an edit field focuses it and turns forms mode
    on; Up or Down at the caret's edge leaves the field for the next line.
  - Recorded: 282-285 (Up into fields, fields with text, Tab then arrows, a text area's lines).
  - **For the owner to decide** (285/s19-s24): leaving a text area upward at its top line, JAWS
    goes to "main region" and then the page's title, past the 16 paragraphs above the field,
    in every trial. It looks like a JAWS accident; Okur reads the line above the field.
    Proposed as an accepted divergence.
  - Open: real pages with forms that need no sign-in. GitHub's new issue form needs one: never
    sign in or submit.
- **204/s1:** "blank" after a required field on the first Tab; JAWS says it in some trials only.
  Re-record with the speech recorder voice.
- **205/s6:** after Space on a radio button, JAWS repeated its position ("2 of 3") in the old
  recording and in 229/s10, but not in the five 2026-10-18 trials. JAWS varies.

## Web applications

- **A focus the buffer has no line for** (219; audit §2.2): in WhatsApp-like message lists Okur
  leaves the virtual cursor on the previous focus and says the page title again. Also two
  dev-box rules that old-pc contradicts: how a plain focusable element and a page's popup menu
  items are said. Record 219 with Auto Forms Mode on.
- **A page menu opened by the page's own shortcut** (218/s10): the owner decides the hint.
- **Chrome's own controls** (220; audit §2.4): "New tab page", "Close Document Window", the
  page named after leaving Chrome's menu, and Down in the address bar.
- **Tab from the virtual cursor** skips focusable custom widgets (handoff 2026-10-17 Docs and
  Sheets).
- **Docs:** JAWS's "Type in text." after "Leaving menus". **Sheets:** a module for its font
  (handoff 2026-10-17).
- **WhatsApp:** a live check of the partial page reads on the dev box, with the owner's
  permission (`okur latency` before and after a page change).
- The open items of [`2026-10-17-web-apps-round-3.md`](../plans/2026-10-17-web-apps-round-3.md)
  step 2 and [`2026-10-16-web-apps-round-2.md`](../plans/2026-10-16-web-apps-round-2.md).

## Native desktop

- **Dialogs** (323, 322; audit §2.3):
  - the page's name and text on Ctrl+Tab ("Security Page | For special permissions or advanced
    settings, click Advanced." 323/s10): the tracer has the page's name but not its own text.
    Which statics JAWS counts as a page's text needs experiments on old-pc: the tab control
    spans the page, and labels such as "Group or user names:" are not read;
  - Say Window (323/s2): done but for JAWS's "Opens with: Opens with:", which the tree does
    not explain;
  - a list item with its columns and position after the viewer closes (322/s18);
  - the viewer shows a path clipped as on screen ("sr-property-fixtur", 323/s7): a side effect
    of JAWS's screen model, a candidate for accepted-divergences.md.
- **A list view's control view** (322/s15, s16): JAWS shows its last row with its columns, Okur
  another row without them.
- **Notepad's window view:** its menu bar and status bar lines (314/s10, s11) are built and
  wait for 314's re-recording; JAWS's "ESCAPE" on the first Down in its font message (314/s3).
- **Small differences:** 315/s16.
- **File Explorer** (dev box, handoff 2026-10-17):
  - the Details view button's help text;
  - Ctrl+A reading every selected item with its columns;
  - "Working on it..." on Alt+Up;
  - "not checked" on pinned Quick Access items;
  - "Closed" with a capital.
- **Windows Settings** (handoff 2026-10-17): dialogs of elevated processes ("Rename this PC")
  need an elevated Okur; the search box's doubled "No results".
- **Calculator:** JAWS's digits, expressions and operator names (handoff 2026-10-17). Not built.
- **Not covered yet:** Save As and Open dialogs, Task Manager, Word and WordPad, a desktop mail
  client, consoles with output arriving while a command runs.

## A page's first read (UIA, before the IA2 read)

From the audit §4:
- preformatted text keeps its lines (207);
- `haspopup=menu` gives a menu button (210);
- fonts and message groups (209, 219).

Block quotes cannot be seen through UIA. Measure first how long users stay on the UIA buffer
(`okur latency`).

## Responsiveness

- **The 2026-10-19 review** ([`responsiveness-review-2026-10-19.md`](responsiveness-review-2026-10-19.md)):
  17 findings, measured where possible. First:
  - a partial page read still rebuilds the whole page under the engine lock, once per part;
  - the first key after a re-capture drops the old buffer and scans the page in the hook;
  - Tab in browse mode is O(tab stops × lines);
  - every page focus scans the whole buffer;
  - live runs use a debug build, which multiplies all of these by 3 to 10.

## Measurement and harness

- **The signup fixture's `help@riverside-library.org`** is not at a reserved example domain, so
  the tracer would blank it out of a trace (`crates/okur-platform/src/privacy.rs`). It has never
  appeared in one; move it to `example.org` the next time `signup-form.html` is re-recorded.
- **Known-gap tests** (AGENTS.md rule 6): `crates/okur-corpus/src/lib.rs` replays the step and
  asserts JAWS's speech (`parity::run_one`): Space on a select box (229/s21), level keys into a
  section (104/s1, 12/s1); and
  `crates/okur-core/src/lib.rs`'s `heading_level_keys_say_the_section_heading_entered`, which
  replaced the scenario 12 assertion JAWS contradicted. Add one for each gap whose behavior is
  recorded; `cargo test -- --ignored` runs them.
- **Dev-box rules** (audit §5): rules 4 (Say Window), 5 (Say Focus's access key) and 7 (the
  desktop's list) now follow old-pc; 6 (Notepad's window view) waits for 314's re-recording.
  Rules 1 to 3 (a plain focusable element, a page's popup menu items, the page after Chrome's
  menu) came from the owner's real WhatsApp; 219's fixture only imitates it, so they wait for
  a recording that settles them (WhatsApp-like markup, or the owner's live check).
- **Live runs:** the run files behind `okur live-parity` numbers are kept on old-pc and in
  `target/live/`, not in the repository. Decide where to keep them so a number can be
  checked again.
- **Re-recordings for evidence questions** (audit §8 item 13):
  - 01 in format 2, or retire it;
  - 204 with the speech recorder voice;
  - 305 with a clipboard capture after s13 (JAWS said "selected do" where the keys select "bo");
  - 111's final "end";
  - 207's Insert+T in a region under a heading.
- **The window title's profile name** (209/s1): needs the window's accessible name in the
  tracer.

## Done (from earlier backlogs and the audit)

- JAWS's own dialogs measured live, with `okur live-parity` and `run-scenario.ps1 -Reader okur`
  (audit §8 item 6): the list dialogs of the 2026-10-18 round.
- Accepted divergences written down and kept out of parity counts (audit §8 item 12):
  [`accepted-divergences.md`](../references/accepted-divergences.md).
- Select a Form Field's 8 of 10 fields (audit §2.5): all 10 since `5703dad`.
- The spoken capture cut at the first 500 ms pause (audit §3.3): a step's records now end at
  JAWS's copy confirmation (`fb2b266`); the 17 affected scenarios re-recorded (`2fc0771`).
- A field a list focuses, said as JAWS says it: its line in forms mode, no "Type in text."
  (`91f413d`; live on old-pc 2026-10-19: 225 11/11, 228 12/12, 249 5/5).
- Space with the virtual cursor (the owner's report, 2026-10-18): `410654f`, scenario 229.
- A multi-line native field read as a document (316/s3, 400/s1, s5; audit §2.1): `6c4b81d`,
  passing since 316 and 400 were re-recorded with the window style, together with a quick
  Alt+Tab's switcher focus and the apostrophe passed through below its level (2026-10-19).
- Down and Up in a select box in forms mode: `062666c`.
- See also the status notes in
  [`complex-web-apps-google-docs-sheets.md`](complex-web-apps-google-docs-sheets.md) and
  [`individual-windows-applications-navigation.md`](individual-windows-applications-navigation.md).
