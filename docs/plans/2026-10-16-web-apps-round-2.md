# Plan: Web Apps, Round 2 (2026-10-16)

Follows [`docs/handoff/2026-10-16-responsiveness-and-web-apps.md`](../handoff/2026-10-16-responsiveness-and-web-apps.md)
and, for Google Docs, [`docs/handoff/2026-10-16-google-docs.md`](../handoff/2026-10-16-google-docs.md).
The old-pc work items of [`2026-10-15-jaws-feature-loop-round-2.md`](2026-10-15-jaws-feature-loop-round-2.md)
still stand; this plan adds tonight's.

## 1. When old-pc is back: record

- Round 2's pending scenarios first (its work item 1: 315–323, 208).
- Then 210–217 (`corpus/pending/README.md` lists them; fixture `web-app-patterns.html`). Each
  checks a behavior built from dev-box observations:
  - 210 aria-current, role descriptions, popup buttons, in reading;
  - 211 the autocomplete (forms mode kept on options, "edit combo");
  - 212 Tab and Shift+Tab from the virtual cursor;
  - 213 word stops at punctuation in a text area;
  - 214 "Files NavigationRegion | list | with 1 items";
  - 215 role="application";
  - 216 Tab through a table (header, position);
  - 217 landmarks named on focus;
  - 218 an ARIA menu bar (`web-menus.html`): JAWS's wording for a page's menus, its position
    ("10 of 10") and tutor direction, and whether Tab reaches the bar.
- Where old-pc's JAWS differs from the dev box, old-pc decides (the dev box's settings are the
  user's). Check especially: the hint after Tab into an edit combo (the dev box said it in one
  run and not in another), and whether old-pc says "banner" where the dev box does not.

## 2. With the user (accounts needed)

- **Google Docs**: started (handoff `2026-10-16-google-docs.md`). Say Font from the toolbar,
  Backspace saying only the deleted character, repeated announcements, and Docs' menus are
  done. Next:
  - ask the user before turning on Docs' braille support (a Docs setting): it puts the text
    in the tree, which Say Line (Insert+Up, silent now), Say All and selection reading need;
    then compare JAWS and Okur with it on;
  - selection (Shift+arrows, Ctrl+A), headings and lists in the document, comments, and
    `okur latency` for Docs' announcements;
  - Docs' Ctrl+Alt shortcuts type characters on AltGr layouts (Turkish: Ctrl+Alt+A is "æ").
    Okur could offer them on its own keys; decide with the user.
- **Google Sheets**: cell navigation and Sheets' announcements.
- **Cursor and Antigravity**: the user signs in, or lends a profile. Their editors are Monaco,
  as in vscode.dev, so tonight's work applies; check the workbench, the agent panels and the
  terminal.
- **Claude desktop and Codex**, only on test conversations the user sets up.

## 3. Without old-pc or the user

- ~~Bug: Chrome's address bar is read as the page.~~ Fixed in `0440801`: the engine says with
  each read whether the focus is in a page, and the tracer decides from the focus chain.
  `okur caret` decides the same way (a document above the UIA focus).
- Monaco's completions (Ctrl+Space in vscode.dev): the textarea points at the suggest widget's
  rows with aria-activedescendant, which `01c65c3` handles; check what JAWS says.
- ~~Other landmarks entered on focus~~: done (BannerRegion, ComplementaryRegion, SearchRegion,
  Region, ContentInfoRegion, heard on the dev box). A control in a table cell on Tab:
  done too (the header when the column changes, then "column 1 row 2", "Column 2" or "Row 4").
- JAWS's E on a read-only text area says the whole text; Okur says "Contains text". Decide
  with old-pc's evidence.
- A list item with aria-current: JAWS said "• Item nine", then "Current Step" on its own.
- Quick navigation onto options of an open list box, and Insert+F3.
- **Keys at a field's edge** (Down on the last line, Right at the end) still wait the settle
  time (250 ms): nothing changes, so nothing is noticed. Speaking at once from the known text
  was considered and not done: Monaco's text area holds only a window of lines (its last line
  is not the document's), a console's Down at the prompt recalls history (the text changes),
  and PowerShell's Right at a line's end accepts a suggestion. A safe version needs the
  application to say the key did nothing (a caret event that does not come), or a shorter
  settle where the field is known to be whole and static.
- **Tab from the virtual cursor skips focusable custom widgets** (an ARIA menu bar item with
  `tabindex=0`; found with `web-menus.html`). Okur focuses the next tab stop it knows (links,
  buttons, fields), and accessibility APIs do not give the Tab order (`tabindex=-1` elements
  are focusable too). Do it JAWS's way: put the browser's sequential-navigation starting point
  at the virtual cursor (a caret or selection there, through IA2), then let Chrome do Tab.
  Recheck 212 and 216 after.
- A page's Say Focus (Insert+Tab): not implemented for page focuses
  (`engine/commands.rs`).
- Structure step 4 (`SpeechFormatter`, the buffer's lines): design a structured page line
  (parts with roles and states) before converting.

## Rules that still hold

- Speech changes come only from evidence or dev-box observation named in the commit. Before
  and after each change, `skills/jaws-reference/scripts/parity-snapshot.sh <dir>` saves the
  three parity outputs (compare with `diff -r`); changes to edit keys also rerun
  `skills/jaws-reference/scripts/measure-edit-keys.sh` on a native and a web field.
- Live checks on the dev box only with the user's permission for that occasion, in isolated
  windows, never stopping JAWS (see `docs/references/verification-loop.md`).
