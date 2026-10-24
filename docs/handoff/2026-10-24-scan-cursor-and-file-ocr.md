# Handoff: the Scan Cursor, a hardening review, and OCR of files (2026-10-24)

Follows [`2026-10-24-okur-cursor-screen-text.md`](2026-10-24-okur-cursor-screen-text.md) and covers
its 34 later commits (`85f48830` to `d63882d7`) and this one. They were made under
[`docs/plans/2026-10-24-everyday-gaps-round-27.md`](../plans/2026-10-24-everyday-gaps-round-27.md).

The owner's requests in this stretch, in their order:
1. "move it out of experimental": the screen text is how the Okur cursor works (`85f48830`).
2. No JAWS-style "graphic 545" numbers: decided with the owner (research §10).
3. The README's figures, refreshed (`f5e3a7ad`, and again in this handoff's commit).
4. "continue with this following JAWS cursor thing": the Scan Cursor in modern windows.
5. "if this finishes before 8:00 … OCR of files and PDFs": built, below.
6. A performance and hardening review of the whole codebase: done, its fixes in `3e685359`.

The next plan is [`docs/plans/2026-10-24-everyday-gaps-round-28.md`](../plans/2026-10-24-everyday-gaps-round-28.md).

## What was built

### The Okur scan cursor (plan 27 §1.1)

In a WPF, UWP or WinUI window the Okur cursor reads the window element by element, as JAWS's
Scan Cursor does:
- "Okur scan cursor" and "Route Okur scan to PC", JAWS's words with Okur's name;
- one element a line, with its role and state ("Remember me check box not checked"), an unnamed
  combo box or list said by its role ("combo box", "list view"), the title bar's System menu;
- Ctrl+Home and Ctrl+End to the first and last element, "top of file" and "Bottom of file";
- Up and Down land on the element's first character.

Fixed on the way:
- The route goes to the element with the keyboard focus, which the tracer now saves
  (`3178ef2e`): a trace can lack the field's own focus event.
- A WPF window with the focus inside is still read by elements (`ad9d5ae5`): UI Automation
  makes it a dialog of class "Window", and the converter took that class for the window's.

### The hardening review (`docs/backlog/performance-review-2026-10-24.md`)

A reviewer agent read the whole codebase at `92bebb44`. Fixed in `3e685359`:
- the actions worker has two lanes, so the text read after an edit key never waits behind
  OCR or a screen walk; mouse points, screen reads and recognitions are coalesced;
- panics are contained in the WinEvent callbacks, the actions worker, the page worker and the
  speech thread; a remote `accChildCount` is clamped;
- the hook forgets keys thought held when it is installed again;
- the screen model builds its lines once;
- a recognition is filed under the window it was asked for.

Findings 1, 3, 4, 6, 9a-c and 9e stay open there.

### OCR of files and PDFs (plan 27 §1.4; the screen text plan's S4)

Insert+Space, O, then:
- **F**, or **Enter** in File Explorer: the picture or PDF selected in File Explorer is read,
  page by page, into the Results Viewer:
  - "Document OCR started." once a picture or PDF is found, then "finished." and the text;
  - the viewer is titled "<file> - OCR Results", which Insert+T says as a window's title;
  - Escape returns to File Explorer.
- **Refusals:**
  - Nothing selected, several files, or a file that is no picture or PDF: JAWS's message box
    with its words. Okur's is "Okur for Windows Message Box" with OK, JAWS's title with Okur's
    name.
  - F in a window that shows no files: JAWS's message, said.
- **D:** "OCR Failed to start.", then Okur points to F. Okur recognizes no document open in a
  reader yet, and JAWS's sentences about Acrobat would be false of it.
- **?:** "OCR Layer Help Screen", the keys Okur has.
- **Q with nothing under way:** "You can not cancel a Recognition job that does not exist."

Underneath:
- `okur-platform::file_text`:
  - the selection, through the shell's window list and the view's selection;
  - a picture decoded upright by the imaging component;
  - a PDF's pages drawn by Windows.Data.Pdf at 200 dots per inch (at most 500 pages);
  - each read by Windows.Media.Ocr, cancellable between pages.
- The request runs on the actions worker's bulk lane.
- `okur-lab file-text` reads a file or the selection; the recorder saves it after each step of
  a scenario with `trace_file`, and the replay answers F with it.

File Explorer itself, found on the way:
- an application module's focus (File Explorer's item) is now the engine's native focus too:
  Say Line had said nothing there, and F could not tell it was in File Explorer;
- an item's line is its whole row with its date, from the cells' values ("Not Selected SR OCR
  Sign.png | 1/2/2026 3:04 AM PNG File 11 KB | 3 of 3"); the items view counts as a list view;
- back from Okur's message box, File Explorer says its hint, as from the same application.

## Harness

- `launch.hide_console`: a PowerShell fixture's console hidden, its window the application's
  only one (`3d8297f9`). With it shown, JAWS's Scan Cursor walked into the console, a window of
  the same process.
- `launch.shell_window`: a fixture whose window is File Explorer's, found by class and title
  and closed with WM_CLOSE, never killed: its process is the shell's.
- Set-ShellItemFocus: File Explorer's new window can keep the focus in its home view after it
  shows the folder; the selected file now gets the focus through UI Automation at launch.
- A JAWS Results Viewer window left open is closed by the elevated helper before the next
  trial; the recorder's own Stop-Process cannot reach it and had reported it closed.
- `trace_file` and `okur-lab file-text --append-to`; the replay merges a step's file snapshot
  with its screen snapshot.
- The replay leaves out JAWS's hidden window "JFW Hidden" (class JFW).

## Found, and how

- **A scenario generator lost the backslashes** of `C:\Users\…\SR OCR Files`, so round 292's
  fixture window was never found. Every trial failed, nineteen File Explorer windows were left
  open on old-pc, and the file snapshots read none of them. Window titles with backslashes are
  now built from `String.fromCharCode(92)`.
- **Live probes before recording** (Workflow H, research, never evidence) gave JAWS's words
  for F, and showed that UI Access keys reach its elevated Results Viewer.
- **JAWS's file OCR** (1024-1033): F reads into a Results Viewer with an "Open in Word..."
  link; its engine (OmniPage) joins a scanned page's lines and reports the page's orientation.
  Refusals are message boxes, except outside File Explorer, where JAWS speaks. D works only in
  Adobe Acrobat. Old-pc's Acrobat Reader install is broken (no `AcroRd32.exe`), so D in Acrobat
  was not recorded: installing it is the owner's.

## Proof

Each figure is the named command's output at `d63882d7`.

- `okur-lab parity`: "JAWS parity (Ia2): 8609/9168 steps match exactly; 249 accepted
  divergences not counted (0 scenarios could not run)". It was 8482/9052 with 214 at the last
  handoff.
- `okur-lab parity` (spoken layer): "JAWS spoken parity: 8513/9229 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `cargo test --workspace`: 1429 passed, 0 failed, 200 ignored.
- Per scenario, `okur-lab parity --only <id>`:

| Scenario | What | Output |
|---|---|---|
| 1022 | the Scan Cursor through a WPF window of every common control | 22/22, 11 accepted divergences |
| 1023 | the Scan Cursor to the end of a WPF window | 12/12, 6 accepted |
| 1024 | F on a picture | 6/9, 2 accepted |
| 1025 | F on a two-page scanned PDF | 3/10, 4 accepted |
| 1026 | F with nothing selected | 5/5 (what JAWS says before its box set aside) |
| 1027 | F on a text file | 5/5 (likewise) |
| 1028 | Enter on a picture | 4/4, 2 accepted |
| 1029 | Q with nothing to cancel | 4/4 |
| 1030 | F outside File Explorer | 4/4 (JAWS's third sentence set aside) |
| 1031 | D with no PDF open | 3/3, 1 accepted |
| 1032 | the layer's help | 14/15, 7 accepted |

The steps still differing:
- **The Results Viewer's "Open in Word..." link**, a known gap with its test
  (`the_results_viewer_ends_with_open_in_word`): 1024/s7-s9 and 1025/s6-s12.
- **1032/s22:** back from the help, Okur names the WPF field after its window ("SR WPF Text
  Fixture Edit"), where JAWS gives it no name.

## Divergences decided (`docs/references/accepted-divergences.md`)

- JAWS's Scan Cursor is stuck past a window's last element (1022, 1023).
- JAWS reads a file's text over and over as its Results Viewer opens, and its engine's "Page
  is right side up." (1024/s3, 1025/s3, 1028/s3).
- JAWS's engine joins a scanned page's lines (1025/s4, s5).
- JAWS says its main window before its OCR message box (1026/s3, 1027/s3).
- F outside File Explorer: JAWS's third sentence points to its own context menu item
  (1030/s3).
- JAWS says the selected file twice back from its Results Viewer (1024/s11, 1025/s14,
  1028/s6).
- D: Okur points to F (1031/s3).
- The OCR layer's help lists only Okur's keys (1032/s5-s11).

Retired: 1016 and 1017. Their snapshots were taken by rows, and their end walked into the
fixture's console; 1022 and 1023 hold the same keys.

## old-pc

- Round 298 records 1033 (F with two files selected) again: round 297's snapshots predate
  `d63882d7`, which saves several selected files as such. Import it after the privacy check.
- Old-pc's lab state is clean: the File Explorer windows were closed. JAWS's Results Viewer
  process may linger without a window; JAWS reuses it.
