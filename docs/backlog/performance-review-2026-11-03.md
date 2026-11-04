# Second review before cross-platform: performance and structure, 2026-11-03

The owner, 2026-11-03: "review performance and project structure, identify performance gaps or
naive implementations, anything that can be implemented better, and a second pass for
refactoring before considering to go cross-platform. identify any crate structure that can be
refactored better to support cross-platform functionality."

**Who reviewed what:** two read-only reviewer agents at `ceb00ead`, at the same time (neither
built anything): performance, latency and naive implementations; structure, layering and
cross-platform readiness. Both started from what the earlier reviews left open
([2026-11-02](performance-review-2026-11-02.md) and before) and from the accepted
[crate structure](../references/crate-structure-proposal.md) (steps 0 to 3 done).

**Method:** the reviewers read code; the performance reviewer also counted events in the
corpus's traces (5,916 trials, 306,691 events). Their times are estimates from figures already in
the repository, not measurements. Each finding marked fixed was checked against the code before
it was fixed, by hand; each fix has a test that fails without it where replay can see it, and
`okur-lab parity` was run after it. The live path was measured on old-pc before (below) and is
measured again after the fixes.

Statuses: **fixed** with the commit, or **open**.

## Performance findings, by user impact

| # | Finding | Status |
|---|---|---|
| P1 | Word's Say All asks for every paragraph of the document before it says a word: from the document's start, not the caret, up to 20,000 paragraphs, about 8-9 UIA calls each (estimated 13 ms: 1.5 s for 10 pages), on the lane every key's read waits on, with no cancellation. Headings, links, fields, notes and Excel's lists the same. | Lane fixed `8ad52a3e` (the whole-document and whole-sheet reads on the bulk lane); from the caret's paragraph on `5f3d00c0`. Open: in pieces as Say All advances; headings by `FindAttribute` or Word's own list; a cancellation check per paragraph |
| P2 | Live, each window come to the front read the whole focus (`GetFocusedElement`, 42 properties, the ancestor chain, MSAA, the process), and the converter keeps only the window's handle. 5,654 such events in the corpus; 73% followed within 200 ms by a focus, which waited behind the read. | fixed `8ad52a3e` (live: handle, class and title only; a trace keeps the whole read) |
| P3 | A Win32 list view's own focus walks the list to the focused index, two uncached calls per item, up to 500. | Calls halved `8ad52a3e` (each step brings its child's type). Open: O(1) through MSAA (`AccessibleObjectFromWindow` and `ElementFromIAccessible` at the index), checked live against the walk; skip when a newer focus of the window waits |
| P4 | Excel's row and column readers, totals and lists read cell by cell, each `Cells` plus `Text` with `GetIDsOfNames` per call: about 2,000 calls per reader key, up to 60-80k for a list; `SheetPlace` not coalesced under key repeat. | Lists to the bulk lane `8ad52a3e`. Open: one `Range.Value2` array per row or column, `SpecialCells` for the lists, cached DISPIDs, coalesced `SheetPlace` |
| P5 | Skim reading by a text rule compiled its regular expression for every line and formatted every line of the page from the first, inside the keyboard hook. | fixed `8ad52a3e` |
| P6 | Every focus in a Store app read its core window's texts (Insert+T's page title), three uncached calls per child: 4-13 ms of a 35-60 ms Settings focus, once 568 ms as the window opened (live baseline below). | Calls cut to one per child `8ad52a3e`; read on demand for Insert+T `72267a06` (live: Insert+T asks the platform; a trace keeps them with the focus) |
| P7 | The reading highlight slept a fixed 80 ms after each virtual cursor move on the long-read lane, which mouse echo, the Virtual Viewer and the Okur cursor share. | fixed `8ad52a3e` (in view: framed at once; scrolled: asked for every 10 ms, at most 80). Open: clamp its frame to the monitor for an element many screens tall |
| P8 | Word's whole read after every key asks four `Selection.Information` values that only rare commands use; in a table, every key numbers the document's tables (`FindAll` of the whole document); the window handle asked six times. | open |
| P9 | Word's Say All waiting for its paragraphs was not withdrawn by a key: it started later from a caret the user had moved. | fixed `8ad52a3e` (with a test that presses a key between the ask and the answer) |
| P10 | The focus's window and rectangle were asked for uncached after its properties were read; the list items' column headers are found again on every item's focus, two calls per child looked at. | Window and rectangle fixed `8ad52a3e`; the header search with its children's types cached `3a26d63d`. Open: column headers kept per list while its window stays in front |
| P11 | Notification rules compiled their patterns for every notification (Settings' search box sends one per letter). | fixed `8ad52a3e` |
| P12 | A live region's added lines are found by a search that can turn quadratic for a long log of repeated lines. | fixed `665c02d2` (the prefix function: linear) |
| P13 | Say All resumed after a page refresh scans all lines for each place it holds. | open (low impact) |
| P14 | Settings Center's search resolves each setting's name by a linear scan of about 1,400 message ids, two or three times per setting per letter. | fixed `59e7d068` (a map made once) |
| P15 | Office's NetUI focus: its MSAA read took about 450 ms per focus in Excel's message bar (live baseline, 1115) | open (new; seen in the baseline below) |

Earlier findings still open, with new evidence: page controls' siblings read on every focus
(about 12 per page button, 10.6 per Excel cell, unused): fixed in replay's terms (`9af1de45`,
and cells in the next commit; `focus_needs_siblings`, checked over every traced focus, 6,894 of
18,864 with siblings skip them), the live time saved to be measured on old-pc; two
`OpenProcess` per focus (one since the next commit: its executable read once for its name and
its product); the debug log formatting each event in full, which inflates the runs
that measure these fixes (an event is logged by its kind since the next commit)

Checked and fine (not to redo): the keyboard hook's own work; command lookup; settings resolved
only when the application or site changes; message rendering; `pronounce::render` (on the speech
thread, tens of microseconds); the speech thread's queue; `FOCUS_BEHIND`; the bounded event
queues; Excel's cell move since L2; Word's per-document caches; the page worker; the clipboard;
okur-ui's threads; the converter; the front-application checks.

## Structure findings, for cross-platform first

| # | Finding | Status |
|---|---|---|
| T1 | `okur` built off Windows only by accident: `set_sounds` had no cfg; and nothing built the workspace off Windows to notice. | fixed `153a5c4d` (gated; CI's portable job runs `cargo check --workspace --all-targets`); since `2eebd1a6` the workspace checks for Linux with no warning, and that step no longer hides a failure |
| T2 | `TtsEngineType` repeated the core's `Synthesizer` one to one. | fixed `153a5c4d` |
| T3 | Ratchets undercounted: `engine_app_references` missed calls through an imported module's name (85, not 82); R5 counted application modules, where literals belong, and missed constants and `starts_with`. | fixed `153a5c4d` |
| T4 | Step 4 as written needs changes: no COM hook (the synthesizers start COM themselves; build the first synthesizer on the speech thread through a factory instead); `bridge.rs` and the `xa11y` facade out of the runtime (the facade pulls every platform's backend into its closure); the moved modules' `cfg!` checks (settings and log folders, the UI host's lowered-token start) become ports the program fills; the two macOS stubs that report success while doing nothing are deleted; `okur-setup` and `okur-lab` take the Windows driver as a target dependency once it is gated at its root. | fixed as step 4.1 to 4.7 (`05bc9c0c` to `573bc638`): `okur-runtime` with the ports, speech made on its thread, the settings files, the UI host with its `UiSpawner`, the event loop; the driver gated at its root (`platform_cfg_sites` 186 -> 1); `bridge.rs` gone |
| T5 | The key model is Windows': one `insert` field means Insert held and the Okur key; the Windows key is the Mac's Command; `EditKey::of` and Excel's and Word's keys hard-code Windows' editing chords (Ctrl+arrows, Ctrl+C), so on macOS no field would echo its caret. | open: physical modifiers from the hook, a resolved chord, and a host key table per platform |
| T6 | Word's and Excel's renderers (about 40 of the 85 references) are pure functions of neutral types: the document and spreadsheet capabilities the owner's answer 2 places in the core. | fixed `30725cf6`, `0553d7f8` (`crate::document`, `crate::spreadsheet`; references 85 -> 24); `AppMode`'s shell variants `93709ed9` (now `Shell` and `Application`, each named by its module). The 23 references left are step 5b's: Word's and Excel's commands, keys and pending requests |
| T7 | Six window-class literals in general engine code (a toggle switch, IDCANCEL, Win32's menu window, Office's command bar popup, Word's dialogs). | fixed `c2b69712`: `FocusEvent`'s `switch`, `cancel_button`, `on_menu_window`, `pages_by_control_tab` and the menus' `launcher` and `PopupKind`, from the converter (`okur-windows-model/src/classes.rs`); `platform_literals_in_core` 5 -> 0, now blocking |
| T8 | `FocusEvent` (76 fields) mixes neutral facts, Win32 quirks and facts only application modules read; `PlatformIds` is Windows'. | open: module facts, a dialog context, `PlatformIds` per OS |
| T9 | `ApplicationActions` is one 45-method trait. | open: capability traits a platform grows one at a time |
| T10 | `Synthesizer` is a closed Windows enum; rates on SAPI's scale; locale formats as Windows picture strings. | open |
| T11 | Parity is wired to the Windows model; the portable CI job's crate list is fixed by hand; `engine/tests.rs` is 9,534 lines reaching into private fields. | open |

## Live baseline (old-pc, before the fixes)

Okur reading in JAWS's place, one trial each (research, never evidence), release build of
`ceb00ead`; `okur-lab latency` over each debug log, the focus reads (count, p50, p90, p99, max in
ms):

```
106-article-words: event read: focus                         4      34.1      60.6      60.6      60.6
1113-desktop-word-page-keys: event read: focus              16      72.8     601.1     609.4     609.4
1115-desktop-excel-moves: event read: focus                 15     291.8     472.9     479.7     479.7
201-form-read-down: event read: focus                        1      61.4      61.4      61.4      61.4
3155-desktop-settings-apps-list: event read: focus          18      50.7      64.5    1568.2    1568.2
3158-desktop-settings-display: event read: focus            14      38.1      60.2     116.3     116.3
3159-desktop-run-dialog-tab: event read: focus              10      35.8      92.6      95.9      95.9
3161-desktop-explorer-ribbon: event read: focus             13      96.5     129.1     206.2     206.2
510-desktop-laptop-sentence-paragraph-and-cursor-keys: event read: focus  3  37.5  62.7  62.7  62.7
```

1115 is not comparable with 2026-11-02's (36.4 ms p50): in this run the focus went to Excel's
"Product Activation Failed" message bar's link, and each of those NetUI focuses spent about 450
ms in its MSAA read (P15). 201 and 1113 ended with an error of the harness (201's run printed
nothing; Word's window did not close), after their steps.

## Live A/B: the sibling, cell and process reads (old-pc, 2026-11-04)

Okur reading in JAWS's place (research, never evidence), release builds of `3affafab` (A,
before `9af1de45`, `56d776ca`, `1d16b4f9` and `4c16a1d3`) and of `582eea85` (B), alternating, three
rounds of five scenarios each (`live-ab.ps1` in the session's scratchpad; `okur-lab latency`'s
lines parsed per focus read). Every focus read of all rounds, in ms: the whole read, and its
context and MSAA stages together (B reads the MSAA object inside the context stage, before the
siblings):

```
scenario build  focuses  total p50  p90   context+msaa p50  p90
229-web-space-activates a              39     37.9    70.5         12     28
229-web-space-activates b              26     33.7    64.6          8     26
3158-desktop-settings-display a        43     35.7   113.6         10     28
3158-desktop-settings-display b        44     38.2   109.9          9     29
3159-desktop-run-dialog-tab a          24     39.3    94.7         14     22
3159-desktop-run-dialog-tab b          33     36.2    95.5         11     19
3161-desktop-explorer-ribbon a         51     88.6   184.8         52    111
3161-desktop-explorer-ribbon b         43     94.3   126.8         63     91
545-desktop-data-grid a                39     58.5   105.2         22     28
545-desktop-data-grid b                39     48.1    88.7         14     16
```

Left out: two runs that failed (3159 A round 1, exit 1; 229 B round 2, two focuses read).
What it shows: a grid cell's focus is about 10 ms faster at the median (its siblings, about 22
to 14 ms with the MSAA read), a page's button about 4; Windows Settings and the Run dialog
(siblings still read in a dialog) as before. Explorer's ribbon is slower at the median and faster
at p90, within its run-to-run spread (its A rounds' medians alone vary more than that); not
counted as a change.

## Live A/B: P6, a Store app's window texts read on demand (old-pc, 2026-11-04)

Research, never evidence. Okur in JAWS's place in Windows Settings (3158, Insert+T at s14),
build A `034c8c16` (the window's texts read with every focus) and build B `72267a06` (read when
Insert+T asks), alternating, three runs each, with the debug log. `node p6-compare.js <logs>`
(the A/B script's form: each `event read event="focus"` line, its whole read and its "window"
stage, where the texts were read):

```
build  focuses  total p50   p90   window stage p50  p90
a           44       36.0  102.7            6     9
b           42       30.7   93.2            0     1
```

A Settings focus read about 5 ms faster at the median, the window stage's 6 ms gone. Insert+T
live in B: `app window texts read texts=2`, then `spoken for event msg=Title is | System`. No
key was refused by the recorder's prompt guard (`8ddcd6b3`) while old-pc's Security Alert stayed
open.
