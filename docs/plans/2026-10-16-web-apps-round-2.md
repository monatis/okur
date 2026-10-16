# Plan: Web Apps, Round 2 (2026-10-16)

Follows [`docs/handoff/2026-10-16-responsiveness-and-web-apps.md`](../handoff/2026-10-16-responsiveness-and-web-apps.md).
The old-pc work items of [`2026-10-15-jaws-feature-loop-round-2.md`](2026-10-15-jaws-feature-loop-round-2.md)
still stand; this plan adds tonight's.

## 1. When old-pc is back: record

- Round 2's pending scenarios first (its work item 1: 315–323, 208).
- Then 210–215 (`corpus/pending/README.md` lists them; fixture `web-app-patterns.html`). Each
  checks a behavior built from dev-box observations:
  - 210 aria-current, role descriptions, popup buttons, in reading;
  - 211 the autocomplete (forms mode kept on options, "edit combo");
  - 212 Tab and Shift+Tab from the virtual cursor;
  - 213 word stops at punctuation in a text area;
  - 214 "Files NavigationRegion | list | with 1 items";
  - 215 role="application".
- Where old-pc's JAWS differs from the dev box, old-pc decides (the dev box's settings are the
  user's). Check especially: the hint after Tab into an edit combo (the dev box said it in one
  run and not in another), and whether old-pc says "banner" where the dev box does not.

## 2. With the user (accounts needed)

- **Google Docs and Sheets**, signed in by the user, in a test document. Start with reading
  (Down, Ctrl+Right), Docs' own announcements (its live regions), Sheets' cell navigation, and
  latency (`okur latency`).
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
- Other landmarks entered on focus (banner, complementary, contentinfo, search, named regions):
  hear JAWS say them before adding them to `entered_containers`.
- JAWS's E on a read-only text area says the whole text; Okur says "Contains text". Decide
  with old-pc's evidence.
- A list item with aria-current: JAWS said "• Item nine", then "Current Step" on its own.
- Quick navigation onto options of an open list box, and Insert+F3.
- Structure step 4 (`SpeechFormatter`, the buffer's lines): design a structured page line
  (parts with roles and states) before converting.

## Rules that still hold

- Speech changes come only from evidence or dev-box observation named in the commit. Before
  and after each change, `skills/jaws-reference/scripts/parity-snapshot.sh <dir>` saves the
  three parity outputs (compare with `diff -r`); changes to edit keys also rerun
  `skills/jaws-reference/scripts/measure-edit-keys.sh` on a native and a web field.
- Live checks on the dev box only with the user's permission for that occasion, in isolated
  windows, never stopping JAWS (see `docs/references/verification-loop.md`).
