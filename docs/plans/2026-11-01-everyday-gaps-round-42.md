# Plan: everyday-use gaps, round 42 (2026-11-01)

Follows [`../handoff/2026-11-01-excel-editing-dialogs-and-okur-words.md`](../handoff/2026-11-01-excel-editing-dialogs-and-okur-words.md).
The owner's order (2026-10-31): Excel, then ARIA-AT's test plans (done, round 2), then the
first release through the Microsoft Store; then Outlook (old-pc's needs an account), the
settings' long tail, Word's open items, the review of 2026-10-25's open findings, punctuation
by word's rest, and the README's "Not yet" list in its order.

Rules as before: each item found in JAWS's documentation, recorded on old-pc, then built;
JAWS's accidents become accepted divergences; a figure is copied from the tool's output after
the tool ran; old-pc always has a round queued; the dev box is shared (cargo on 4 cores, one
heavy job at a time, no swarms of subagents). Steps taken inside Office's own menus are
`"history": false` (troubleshooting §10).

## Status (2026-11-01, evening)

Done: the Store research and the package build (§1.1, §1.2), Excel's titles (§2.1-§2.3, 1132),
the Okur cursor from screen text routed onto the caret's line
([`../handoff/2026-11-01-excel-titles-and-store-package.md`](../handoff/2026-11-01-excel-titles-and-store-package.md)).
Next: round-1133b's import, Excel's row list (Insert+Shift+R) and Insert+F, the totals readers;
§1.3 onwards wait for the owner (a test certificate on old-pc, the identity, Defender's flag).

Later (2026-11-01, night): Excel's Insert+Shift+R, Insert+F and Ctrl+Shift+D (1133, 9 of 9);
Partner Center's identity, Okur's logo and PRIVACY.md in the package
([`../handoff/2026-11-01-excel-lists-format-and-store-identity.md`](../handoff/2026-11-01-excel-lists-format-and-store-identity.md)).
Next: the totals readers, titles from defined names; the Store's experiments wait for the owner.

Then: titles from the workbook's names (1134) and the totals readers (1132, 13 of 13)
([`../handoff/2026-11-01-excel-named-titles-and-totals.md`](../handoff/2026-11-01-excel-named-titles-and-totals.md)).
Next: monitor cells and comments, titles kept across sessions.

Then (2026-11-02): the owner's performance and reliability review before the release
([`../handoff/2026-11-02-performance-review-fixes.md`](../handoff/2026-11-02-performance-review-fixes.md)).
Next: [`2026-11-02-everyday-gaps-round-43.md`](2026-11-02-everyday-gaps-round-43.md).

## 1. The first Store release (dev box)

[`2026-10-31-microsoft-store-release.md`](2026-10-31-microsoft-store-release.md), its steps in
order:
1. Research its six questions into `docs/references/microsoft-store-release-research.md`, with
   sources: UI Access in an MSIX (`rescap`, what Partner Center approves), NVDA's Store build
   as the precedent, starting at sign-in (`windows.startupTask`) and what a package cannot do
   (the sign-in screen), the package's processes and folders, identity and updates, the
   listing.
2. A packaging script (MakeAppx from the Windows SDK, or a maintained tool), a test
   certificate for local installs.
3. The Windows App Certification Kit on it.
4. On old-pc, with the owner's leave to trust the test certificate (a system change): install,
   start, speak, read Chrome, Word and Excel, start at sign-in, quit.
5. The release's text: README, help, the "Not yet" list as known limits.
6. The owner submits in Partner Center.

## 2. Excel's titles and totals (old-pc, beside §1)

From `popapps.chm` (decompiled into `Dropbox\sr-harness\jaws-docs\popapps`) and Excel's key
map; the handoff's live look gave the first words.
1. **The recorder forgets JAWS's settings for the fixture** before each trial: JAWS keeps a
   workbook's titles in its personalized settings for the file, and every later trial (and
   every Excel scenario after) would start with them. A scenario names the settings files to
   remove (under JAWS's user settings folder) and the recorder restarts JAWS when it removed
   any, so JAWS does not keep them loaded.
2. **Chords with Alt+Shift** (Insert+Alt+Shift+C and R, Say Column and Row Title): sent by
   `okur-lab keys` they reached Excel as Alt+Shift+R. Check old-pc's input languages (Alt+Shift
   switching them) and the recorder's order of modifier keys; record them only when they reach
   JAWS.
3. Record: set column titles (Insert+Ctrl+Alt+C) and row titles (Insert+Ctrl+Alt+R), moves
   reading the new column's or row's title, Say Column and Row Title, the totals row and column
   (Ctrl+Insert+Delete, Ctrl+Insert+NumPad Enter) and their readers (Insert+Delete,
   Insert+NumPad Enter), titles from Excel's defined names ("ColumnTitle", "RowTitle", "Title";
   a fixture with them). Then build: titles kept per workbook in Okur's own settings (its own
   format, AGENTS.md §1.4), read from Excel's names through its object model.
4. After them, from the same page: Ctrl+PageDown and Ctrl+PageUp (the next and prior sheet),
   Insert+Shift+R (the row's cells), comments (Alt+Shift+Apostrophe, Ctrl+Shift+Apostrophe),
   Insert+F (the cell's font), monitor cells.

## 3. Open from round 41

- 1125/s3: Word's Paste Options gallery inside its context menu ("table | Paste Options: | Keep
  Text Only Button | Keep Text Only (T) | 1 of 1 | Navigate with left and right, up and down
  arrow keys").
- 1062/s10-s13: the empty area below a field's text, which JAWS's cursor reaches with Down and
  says nothing of; and the Okur cursor from screen text starting on the menu bar line after a
  route (round-1062c, not imported).
- 1129/s11 (a page dialog's start line), 1130/s11, 2857/s1.

## 4. Waiting for the owner

- Containers on quick keys; a field's selected text on a focus (the last ARIA-AT handoffs).
- Outlook: a mail account on old-pc.
- Trusting a test certificate on old-pc for the Store package (§1.4).

## 5. Then

The owner's order above: Outlook, the settings' long tail (Voices' Apply question with a lab
profile, the Startup Wizard's rest), Word's open items (1069/s4, 1119/s3, Insert+Alt+R), the
review of 2026-10-25's open findings, punctuation by word's rest, the README's "Not yet" list.
