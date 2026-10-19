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

## Forms and browse mode

- **Forms with Up and Down in browse mode** (the owner's request, 2026-10-18; not started).
  Okur may say form fields differently from JAWS when arrowing through them. Record JAWS on the
  form fixtures (`forms-interaction.html`, `signup-form.html`, `web-app-patterns.html`), every
  field, group and form boundary. Then real pages with forms that need no sign-in. GitHub's new
  issue form needs one: never sign in or submit. Capture snapshots, check privacy, then compare.
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

- **A multi-line field read whole on focus** (316/s3; audit §2.1): JAWS reads the caret's line
  and says "Contains text". Daily use, hours of work; the evidence exists. In the same kind of
  field (a WinForms text box) JAWS's Say Line gives the line without the field's name, and
  Ctrl+Home says "top of file" first (400/s1, s5), as in a document. Okur knows a native field
  has several lines only from its role; the tracer would need the window's multi-line style.
- **Dialogs** (323, 322; audit §2.3):
  - the page's name and text on Ctrl+Tab;
  - Say Window's format (323/s2);
  - the access key on Insert+Tab (323/s1);
  - a list item with its columns and position after the viewer closes (322/s18).
- **A list view's control view** (322/s15, s16): JAWS shows its last row with its columns, Okur
  another row without them.
- **Notepad's window view:** its menu bar and status bar lines (314/s10, s11), and JAWS's
  "ESCAPE" on the first Down in its font message (314/s3).
- **Small differences:** the desktop list's role marks (316/s1, 319/s9); 315/s16.
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

## Measurement and harness

- **The signup fixture's `help@riverside-library.org`** is not at a reserved example domain, so
  the tracer would blank it out of a trace (`crates/okur-platform/src/privacy.rs`). It has never
  appeared in one; move it to `example.org` the next time `signup-form.html` is re-recorded.
- **Known-gap tests:** none exist yet (AGENTS.md rule 6). Write one per group above where JAWS's
  behavior is recorded.
- **A test asserting speech JAWS contradicts:** `crates/okur-core/src/lib.rs`, scenario 12, the
  section heading on level keys (104/s1, 12/s1; audit §2.6).
- **Dev-box rules:** seven rules in the code come from the owner's JAWS on the dev box, and
  old-pc's evidence contradicts them (audit §5). Record the settings and Windows version each
  was seen with, and settle each conflict with a recording.
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
- Down and Up in a select box in forms mode: `062666c`.
- See also the status notes in
  [`complex-web-apps-google-docs-sheets.md`](complex-web-apps-google-docs-sheets.md) and
  [`individual-windows-applications-navigation.md`](individual-windows-applications-navigation.md).
