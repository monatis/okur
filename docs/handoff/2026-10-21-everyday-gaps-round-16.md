# Handoff: everyday-use gaps, round 16 (2026-10-21)

Follows [`2026-10-21-everyday-gaps-round-15.md`](2026-10-21-everyday-gaps-round-15.md) and covers
the commits after it (`428d430` to `6eea9fd`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-15.md`](../plans/2026-10-21-everyday-gaps-round-15.md).
The next plan is
[`docs/plans/2026-10-21-everyday-gaps-round-16.md`](../plans/2026-10-21-everyday-gaps-round-16.md).

## The owner's answers and requests

None this stretch.

## Decided without the owner (reversible)

- **The Okur cursor reads text, not pixels.** Routed to the PC cursor, it reads:
  - a text field's lines;
  - a page's lines as its screen shows them;
  - a dialog's rows.
  JAWS's screen model reads the screen itself, title bars included. Okur's knows the text
  the accessibility APIs give, and the window title for the top. In the dialog fixture, JAWS
  read "blank" too (516).
- **A dialog's rows follow the dialog's order** (`5688dbe`). The traces give a dialog's
  controls without their places, so buttons side by side share a row and the rest take one
  each. That is right for Notepad's Go To dialog (755); bounds in the traces would make it
  right everywhere.
- **Windows' Win+X menu is read its own way** (`548d686`): its hint comes after the first item
  (520/s3), as before. Every other context menu says "To navigate press Up or Down Arrow."
  first, as Notepad's and Chrome's do (756/s15, 220/s1). The Win+X menu is told apart by the
  window menu mode starts from, Explorer's `LauncherTipWnd`.
- **Skim reading uses the virtual cursor's paragraphs** (`6eea9fd`). Each paragraph is read by
  its first line, and a table's row counts as one paragraph. The rules this replaced (by node,
  then by enclosing block) left out less than JAWS does. The paragraphs left out exactly what
  JAWS leaves out, but for the table's header row.

## What was done

### Features (each checked with `okur-lab parity` against old-pc's evidence)

- **Flexible Web's last pieces** (`428d430`):
  - F2 renames a rule in place (751/s18-s20);
  - Right Arrow opening a tree item says only " ‹open› ‹›" outside WinForms trees (748/s10,
    751/s11).
- **The Okur cursor** (`bbc166c`, `5688dbe`, `5562158`, `233fde8`), from 690 and 752-759:
  - Up and Down by line; Right and Left by character, starting before the line's first;
  - Ctrl or Insert with Right and Left by word, words split from punctuation ("main", "()") and
    moving on to the next or previous line;
  - Home and End to the line's ends, quietly;
  - Page Up to the window's top (the title bar; a page's first line; with the Focus window
    restriction, the field's first line);
  - Page Down to the window's bottom, "blank";
  - Say Word and Say Character where it is;
  - on a page, the virtual cursor stays where it was;
  - Insert+R steps through the restrictions: Application, Real, Current, Focus window, Frame,
    Unrestricted. Alt+Delete names the one in use.
- **The mouse keys** (`548d686`), from 756:
  - Numpad Slash and Numpad Star say "left mouse button" and "right mouse button" and click at
    the mouse;
  - Insert+Numpad Plus, Route PC to Okur, brings the PC cursor to the Okur cursor;
  - a right-click menu says its hint first, and an unavailable item says so ("Undo
    Unavailable").
- **Say Word splits words from punctuation** (`548d686`) in a text field too: "println" in
  "println!(" (756/s9).
- **Skim reading** (`6eea9fd`), from 761: Ctrl+Insert+Down reads to the page's end, each
  paragraph by its first line, and ends on the last line's first word.

### Harness

Nothing new. The rounds used `TypeString`, `continues` and three trials each where nothing was
saved.

## Figures

Each is the named command's output at `6eea9fd`:

- `okur-lab parity`: "JAWS parity (Ia2): 6014/6243 steps match exactly; 158 accepted
  divergences not counted (0 scenarios could not run)". At the previous handoff: 5889/6111 with
  158.
- `okur-lab parity --spoken`: "JAWS spoken parity: 5881/6205 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 419 scenarios, 1297 trials, 1297 successful.
- `cargo test --workspace`: 1311 passed, 0 failed, 199 ignored.

## Learned

- **JAWS's cursor reads this lab's windows.** Notepad (752, 754), Chrome's pages (753) and
  Notepad's Go To dialog (755) all read. Only the PowerShell dialog fixture reads "blank" (516).
- **The cursor's empty lines.** JAWS speaks "blank" for an empty line under its cursor but keeps
  nothing of it in its history, as for the virtual cursor's empty lines (754/s13-s15).
- **Routing the JAWS cursor** puts the mouse at the caret, or at the virtual cursor's text (12,
  88 in Notepad; 36, 411 on the page), not at the element's middle. In Notepad Alt+Delete adds
  "133%", source unknown.
- **JAWS's key map names** (Default.JKM, which may be read):
  - `RestrictCurrentCursor` (Insert+R), `RoutePCCursorToJAWS` (Insert+Numpad Plus);
  - `RightMouseButton` (Numpad Star);
  - `LeftMouseButtonLock` and `RightMouseButtonLock` (Insert with Numpad Slash and Star);
  - `DragAndDrop` (Ctrl+Insert+Numpad Slash);
  - `StartSkimRead` (Ctrl+Insert+Down), `SkimReadDialog` (Ctrl+Shift+Insert+Down).
- **The Skim Reading dialog** (760) opens on "Use Rule Type:" with "Read First Line of Every
  Paragraph" (1 of 4). Tab reaches Skim Reading Indication, Start Skim Reading, Create Summary,
  Close and Help; the rules' controls are unavailable for that rule type.
- **Skim reading takes long:** 761 needed a settle time of 150 seconds.

## Recorded meanwhile, and queued

Round 179 (`waiter179`, three trials) is recording 762: Skim Reading's Create Summary in the
Results Viewer.

## Not done, and why

- **The Okur cursor moving the mouse**: Okur does not know where its text is on screen, so a
  click lands at the mouse, not at the Okur cursor's word. Alt+Delete's coordinates differ for
  the same reason (752/s3 and five other steps).
- **The mouse button locks, drag and drop, Ctrl and Shift clicks**: not recorded yet.
- **The Skim Reading dialog in Okur**: its rules, its summary (762, recording).
- **748's remaining steps**: the dialog's text on opening, the note's line, and "dialog" plain
  on return.
- **The table's header row in skim reading** (761/s1): JAWS leaves it out, for a reason not
  found yet.
