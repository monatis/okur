# Performance, responsiveness and memory review (2026-10-20)

A review of the whole codebase for stalls, freezes, late speech and memory, asked for by the
owner. It builds on [`responsiveness-review-2026-10-19.md`](responsiveness-review-2026-10-19.md)
(read at `aa9c87a`/`c883524`) and does not repeat what that review got done. The reviewer
changed nothing in the repository but this file and ran nothing live.

- Read at `9f7c218`; the tree moved while the review ran. **Line numbers are as of `dd48218`**,
  the HEAD when this was written, and may move.
- **Measured** numbers come from a scratch benchmark crate outside the repository (see
  [Method](#method)), built against an export of `dd48218` (`git archive`), with its own target
  directory. Times are medians of 3-5 runs on the dev box, release unless marked "dev" (the
  repository's dev profile, which live runs use). Another engineer was building on the same
  machine, so runs varied up to 2x; ranges are given.
- The synthetic page is the previous review's: a WhatsApp-like page, 30 wrapper divisions, an
  IA2 uniqueID and three attributes on every node. At 5,000 rows it has **25,032 nodes and
  20,001 lines**; at 1,000 rows, 5,032 nodes.
- **Measured from traces** means counted in the corpus's 570 recorded traces (20,283 events,
  `corpus/traces/`): the tracer that records them is the live event reader (`record_at`), so
  they show what live reads per event, though not how long it took.
- **Read** means reasoned from the code.

Every change must keep `okur parity` byte-identical: run
`skills/jaws-reference/scripts/parity-snapshot.sh <dir>` before and after, then `diff -r`.
Replay feeds recorded traces to the converter and the engine; it does not run the live reader.
So a change to what `record_at` reads does not show in parity: such changes are made for live
runs only (`state.live`), keep `okur trace` complete, and get the replay-equivalence tests
described under P5, plus live checks on old-pc.

## Status

Measured with `crates/okur-core/examples/big_page_keys.rs` (this review's synthetic page,
25,032 nodes, release, median of 3 fresh engines, one press each); `parity-snapshot.sh`
before and after each change gave identical output.

| Finding | Status | Commit | Measured |
|---|---|---|---|
| P1 steps 1-2: span index, `block_text` gathered as far as the item keeps | done; step 3 (offsets alone for Z) not needed yet | `e3a1f22` | Z 559 → 9.7 ms, Shift+Z 547 → 10.4 ms, Ctrl+Insert+Z 579 → 12.1 ms, P 468 → 8.8 ms, X 450 → 6.6 ms |
| P5 step 1: caret and text selection events not read live | done; live check on old-pc (305, 313, 400, 403, 504, 522) to come | this commit | test: all 3,191 in the traces give nothing without their text |

## Summary: the five worst

1. **P1. The Z quick key and JAWS's element lists are quadratic, in the keyboard hook.**
   Measured: on the 25,032-node page, Z takes 0.7-1.4 s, Shift+Z 0.8-1.1 s, Ctrl+Insert+Z
   0.7-1.1 s, Ctrl+Insert+P and Ctrl+Insert+X 0.5-0.9 s, all inside `handle_key_event`. Windows
   silently removes a low-level keyboard hook that runs past `LowLevelHooksTimeout`: after one
   Z on a big page, Okur may hear no key until it restarts. A one-pass index, prototyped,
   gives the same offsets for all 25,032 nodes in 3.3 ms instead of 533 ms.
2. **P4. One hung application, or one chat log, can stop all focus speech.** Read. Every
   cross-process read for an event runs synchronously on the WinEvent thread; MSAA and IA2 calls
   have no timeout; a Chromium live region is read whole through IA2 on that thread for every
   change (about 12 s per change for a 5,000-object log, by the code's own IA2 rate); a page
   whose quick UIA read fails is read through IA2 on that thread too.
3. **P3. Rebuilds under the engine lock land right on the hook's 150 ms budget.** Measured:
   a page focus whose check box state changed rebuilds the page before speaking, 126-163 ms; a
   page check box's change event rebuilds after speaking, 104-145 ms. A key pressed meanwhile
   waits 150 ms and then goes to the browser unread.
4. **P5 and P6. The WinEvent thread reads what it then drops.** Measured from traces: 73% of
   state changes and 57% of selection changes are read in full (focus chain, context, text)
   and then converted to nothing; every caret and text-selection event (16% of all events) is
   read and dropped live; a typed character in a native field moves its text, whole or in
   part, across processes about 8 times on that thread; every Tab in a dialog box reads the
   dialog's whole control tree (up to 244 UIA nodes in the Font dialog) and its layout again.
5. **P8, P9, P10. Smaller costs that still block.** Measured: page parts are spliced and
   built inline in the event loop (135-175 ms); a page focus missing from the buffer's indexes
   scans every node under the lock (11-17 ms); the Links List and Select a Form Field scan
   every span per item in the hook (24-84 ms).

## Findings, most severe first

### P1. Element lists and the division quick keys are quadratic, in the keyboard hook

- **Where.**
  - `crates/okur-core/src/element_lists.rs:131` `ElementList::items`: for every element of the
    list, `buffer.start_of_subtree(node)` (`buffer.rs:495`) puts the node's whole subtree in a
    `HashSet` and scans all spans; `block_text` (`element_lists.rs:289`) collects the whole
    subtree's text before `items` keeps its first 254 characters.
  - `crates/okur-core/src/cursor.rs:1487` `move_division` (Z, Shift+Z, added in `311c4bf`)
    builds the whole Divisions list, texts included, on every press, to use only its offsets.
  - `crates/okur-core/src/engine.rs:1219` `open_list` → `elements.items` (`engine.rs:1246`):
    Ctrl+Insert+letter in the hook; the same from Virtual HTML Features (Insert+F3), under the
    engine lock on the event loop.
- **What happens.** On the keyboard hook's thread, holding the engine, once per key press.
  Nested divisions make it worse: each of the 30 wrappers collects the whole page's text.
- **Measured** (`keys` sweep, `handle_key_event` timed per chord, cursor mid-page):

  | Chord | 25,032 nodes, release | 25,032 nodes, dev | 5,032 nodes, dev |
  |---|---|---|---|
  | Z | 0.69-0.95 s | 1.42 s | 52 ms |
  | Shift+Z | 0.82-1.09 s | 1.13 s | 49 ms |
  | Ctrl+Insert+Z (Select a Division) | 0.68-1.10 s | 0.88 s | 50 ms |
  | Ctrl+Insert+P (Select a Paragraph) | 0.53-0.84 s | 0.80 s | 27 ms |
  | Ctrl+Insert+X (Select a Check Box) | 0.53-0.87 s | 0.70 s | 26 ms |

  Release ranges span runs at `9f7c218` and `dd48218`. `start_of_subtree` alone for the 5,031
  divisions: 531-533 ms. The rest is `block_text`.
- **Proposed change.**
  1. A subtree-start index: for every page node, the start of the first span (in push order)
     of a node in its subtree. One post-order pass: each node id's first span index from one
     walk over `spans`, then each node's minimum over itself and its children. Keep it in
     `VirtualBuffer` as a lazily built `OnceLock<HashMap<usize, usize>>` (or a `Vec` by slot),
     reset by `node_mut` and `for_each_node_mut`. `start_of_subtree` and `start_of` read it.
     Prototyped in the scratch crate: 3.1-3.3 ms for the whole page, and its result equals
     `start_of_subtree` for all 25,032 nodes (0 differ).
  2. `block_text` stops gathering once its whitespace-normalized text holds `ITEM_LENGTH`
     characters. Appending can only change characters after that point, so the first 254 are
     the same.
  3. Z and Shift+Z need offsets only: an `ElementList::offsets(buffer)` that keeps the same
     filter (an element is left out when its subtree has no span, or its text and name are
     empty) with an early-exit "has text" check, computed in the same pass as (1) and cached
     per buffer per list kind.
- **Expected gain.** O(n) per list: the 1 s cases fall to a few milliseconds; Z repeated on
  the same page to microseconds.
- **Risks.** Parity: the outputs must be identical; ids that a page repeats behave as the
  `HashSet` of ids did (the index is by id too). Push order, not offset order, decides "first
  span": keep that. Platform: core only, helps macOS as is.
- **Tests.** Keep the old `start_of_subtree` and `block_text` as `#[cfg(test)]` oracles and
  assert equality for every node of every `corpus/snapshots/*.json` page and of a generated
  deep page; the same for `items` and `offsets` of every `ElementList` kind. Add the hook sweep
  (below, [Method](#method)) as an example under `crates/okur-core/examples/`, run by hand.

### P2. Nothing bounds the hook's work, and nothing notices when Windows removes the hook

- **Where.** `crates/okur/src/main.rs:959` (the hook callback), `:657` `take_for_key`;
  `crates/okur-platform/src/hook/windows.rs:134` (the hook thread), `:234`
  `low_level_keyboard_proc`.
- **What happens.** The callback waits up to 150 ms for the engine, then runs
  `handle_key_event` with no limit. Windows passes a key on, and on Windows 7 and later
  silently removes the hook, once a callback runs past `LowLevelHooksTimeout` (not set on the
  dev box, so Windows' default applies; it is capped at 1 s from Windows 10 1709). Okur has no
  way to tell and keeps running deaf. The hook thread runs at normal priority, so a busy machine
  (a Rust build) delays every key.
- **Read.**
- **Proposed change.**
  1. After a callback whose wait plus engine time passed a threshold (say 250 ms), post a
     thread message to the hook thread; its message loop then unhooks and installs the hook
     again (`UnhookWindowsHookEx`, `SetWindowsHookExW`), and logs it. A re-installed hook is
     called first, which is what Okur wants anyway.
  2. Raise the hook thread to `THREAD_PRIORITY_HIGHEST` at start; it only does short work once
     P1 is fixed. Beware priority inversion: the hook waits on the engine mutex held by the
     event loop, a tokio worker at normal priority. Run the event loop on a dedicated OS thread
     at `ABOVE_NORMAL` (it is one synchronous consumer already), or keep both at normal.
  3. A rule for the engine, written into `docs/references/`: nothing in a key's path may be
     O(page). Work that is (list dialogs, Say All texts, searches) is handed to the event loop
     as a job and spoken when done; replay runs the job at once, so parity does not change.
- **Expected gain.** A slow key costs one leaked key, not a deaf screen reader.
- **Risks.** Re-installing while Windows still has the old hook: unhook first. Priority: test
  with a CPU-bound build running.
- **Tests.** The threshold decision as a pure function with unit tests; the re-install itself
  as a live test (`#[ignore = "live: ..."]`).

### P3. Page rebuilds under the engine lock, at the hook's budget

- **Where.**
  - `crates/okur-core/src/engine/focus.rs:493-517`: a page focus whose node is stale (a check
    box's state, a combo box's value, expansion or name) calls `with_node_changed`
    (`buffer.rs:275`: a copy of the whole page and a full linearization) **before** the focus is
    spoken. The previous review's finding 11 fixed only `page_change`.
  - `crates/okur-core/src/engine/page.rs:247` `page_change`: speaks first, then the same
    rebuild, still under the lock.
  - `page.rs:512` `page_parts_loaded`: when `install_page_parts` finds the page changed
    meanwhile, the splice and build run under the lock.
  - `buffer.rs:546` `node_mut` (Space on a check box in the virtual cursor, in the hook):
    `Arc::make_mut` copies the whole page if the event loop holds the page's `Arc` for a splice
    at that moment (P8), and the splice's install then fails over to the rebuild above.
- **What happens.** On the event loop holding the engine, for Space on a page control, a
  click, a select's or autocomplete's change. The hook waits at most 150 ms for the engine,
  then passes the key to the browser: Space then Down on a big page scrolls the page.
- **Measured** (25,032 nodes): page focus onto a check box whose state changed, 126-163 ms
  (release and dev), 98-118 ms at `9f7c218`; a change event on a page check box, 104-145 ms;
  `with_node_changed` alone, 114-155 ms; at 5,032 nodes, 15-17 ms.
- **Proposed change.**
  1. Speak from a patched node, rebuild elsewhere. Under the lock: clone the one node (a check
     box has no children; a combo box, its list), apply the change to the clone, and compute the
     speech from it (`page_change` builds its speech from the event already; `focus_node_from`
     reads the buffer's node, so give it a variant that takes the patched node). Then hand the
     host a rebuild job: the page's `Arc`, the change, and a generation number. Live, the host
     copies, patches and builds on the blocking pool and gives the result back as a page
     refreshed (`handle_page_refreshed`, or a variant that checks the base with `Arc::ptr_eq`
     as `install_page_parts` does). Replay's host runs the job at once, so the buffer is rebuilt
     before the next event, as now. Never patch the shared page in place under the lock:
     `node_mut` would copy all of it while a job holds the `Arc`.
  2. Later, an incremental line update: relinearize the changed node's lines and shift the
     offsets after it in every index. Only with a property test that it equals a full rebuild.
  3. The failed-install path of `page_parts_loaded` becomes a job too, never a build under the
     lock.
- **Expected gain.** The engine is held for under 1 ms instead of 100-160 ms; no key leaks.
- **Risks.** Until the job lands, the buffer's node and line keep the old state: Say Line
  right after Space could read it. Apply the job's result at the next key
  (`apply_pending_page` does), and, if a key arrives first, let the key path wait briefly for
  an in-flight job of this kind or read the node. Parity: the speech must come out the same
  from the patched node as from the rebuilt buffer; `parity-snapshot.sh` decides. Platform:
  the job type and the generation check live in core; the hosts (`okur run`, replay) execute.
- **Tests.** Hermetic engine tests with a mock host that runs jobs on demand: the speech before
  the job equals today's speech after the rebuild; the buffer after the job equals
  `with_node_changed`'s. Existing check box and combo box tests keep passing.

### P4. One hung application, or one chat log, can stop all focus speech

- **Where.** `crates/okur-platform/src/event_trace.rs:2955` `record_at` and everything it
  calls, on the `okur-winevents` thread; `:2071` `chromium_live_region` (`:3192`);
  `crates/okur-platform/src/events/windows.rs:427` `load_page_if_new` (`:449`, `:458`);
  `crates/okur-platform/src/uia_snapshot.rs:1110` `capture_foreground_document`.
- **What happens.**
  - Each WinEvent is read in full before the next is looked at: `AccessibleObjectFromEvent` and
    five MSAA calls per event, IA2 calls, UIA calls, `SendMessageTimeout`s. COM calls into
    another process have no timeout of their own, so a hung application's event blocks the
    thread until it answers, and every focus, menu and change event behind it waits. This is
    the previous review's finding 16, still open.
  - The thread's own UIA client has 1 s timeouts (`automation()`, `uia_snapshot.rs:603`), but
    `capture_foreground_document` creates a new `CUIAutomation8` with UIA's defaults (2 s and
    20 s) for each page, and finds the document with `FindFirst(TreeScope_Descendants)` from the
    window (`uia_snapshot.rs:1119`, `:1138`). That runs on the WinEvent thread for every new
    page (`load_page_if_new`).
  - If that quick read fails, `load_page_if_new` reads the page through IA2 on the WinEvent
    thread: by the code's own figures (GitHub's repository page, 1,182 objects: 160 ms through
    UIA, 3 s through IA2) that blocks focus speech for seconds.
  - A Chromium live region change (`live_region_change`) reads the region's whole subtree through
    IA2 (`capture_chromium_object`) on that thread, once per event. A chat log marked as a live
    region holds every message; at the same rate (about 2.5 ms an object), a 5,000-object log
    takes about 12 s per change, and a streamed reply raises several changes a second.
- **Read** (the live-region time is an estimate from the code's IA2 rate, not measured).
- **Proposed change**, small steps first:
  1. `load_page_if_new`: when the quick read fails, send `PageRequest::Focus` to the page worker
     instead of reading through IA2 on the WinEvent thread.
  2. One UIA client per thread, made by `automation()`, passed to the capture functions. Find
     the document from the page's render window (`Chrome_RenderWidgetHostHWND`, found as
     `chromium_document` finds it for IA2) with `ElementFromHandle` and a search of its element
     and children only, or keep the document element the event's focus chain passed through,
     instead of `FindFirst(TreeScope_Descendants)` over the whole browser window. Both were
     proposed in the previous review's finding 7 and are not done.
  3. Before reading an event, `IsHungAppWindow(GetAncestor(hwnd, GA_ROOT))`: a window that has
     not pumped messages for 5 s is skipped (logged; its events dropped until it answers again).
  4. Live regions: the WinEvent thread notes the region's object in a dirty set and wakes the
     page worker, which reads each dirty region once (its latest state) and sends the
     `LiveRegion` event; a read is bounded (node count and time) and a region beyond the bound is
     read as its last children. Trace mode keeps reading the region with its event.
  5. A watchdog, as NVDA's: the WinEvent, page and actions threads call
     `CoEnableCallCancellation`; a watchdog thread that sees one of them in a read for more than
     500 ms calls `CoCancelCall` on it, which fails the call (`RPC_E_CALL_CANCELED`) and lets the
     thread go on. Log the application.
  6. Measure the quick UIA read of a big page on the WinEvent thread (it scales with the page:
     on the 160 ms per 1,182 objects figure, about 3 s for 25,000). If it is long, move it to the
     page worker too, with the engine told that a page is loading so that it holds the page's
     focus speech until the page arrives.
- **Expected gain.** No more minutes-long silences behind a hung application or a busy chat
  log; new pages no longer block other windows' focus.
- **Risks.** Dropped events of a hung application: JAWS says nothing for one either. Coalesced
  live regions: what a growing region adds is the same text, read in fewer, larger pieces
  (`LiveRegionAppend::Appended` already gathers pieces); `WholeText` repeats less. Parity: live
  only (replay reads recorded regions). Platform: Windows code; the "page loading" notion, if
  added, is a core event.
- **Tests.** The dirty-set and bound logic as pure functions with unit tests; the hung-window
  and cancellation paths as live tests, checked on old-pc with a test application that sleeps
  in its window procedure.

### P5. The WinEvent thread reads in full for events it then drops

- **Where.** `event_trace.rs:2955` `record_at`: `:3049` (the focused element), `:3098`
  `with_properties`, `:3104` `focus_chain_from`, `:3130` siblings, `:3141`
  `focus_subtree_of`, `:3210` the text context, `:3231` the MSAA object; the converter's
  `convert_one`, `event_trace.rs:1182` onward.
- **What happens**, per event on the WinEvent thread:
  - **Changes of objects that are not the focus.** For every `state_change`, `value_change` and
    `selection` in the foreground process, the reader asks UIA for the focused element, its 42
    properties, its ancestors one cross-process call each, a Win32 list position, the focused
    element's subtree (asking UIA for the focus a second time), the focused field's whole text
    (outside Chromium, a third time) and the event's MSAA object. The converter then keeps the
    change only when it is about the focused object (`same_object || same_name`).
  - **Caret and text-selection events.** Live, they are in `LIVE_EVENTS` (`:2114`), their text
    is not read, and the converter turns them into nothing; but their MSAA object is read
    (`AccessibleObjectFromEvent` and five calls). For a field's `text_selection_change` the
    object's value is the field's whole text.
  - **Typing in a native field.** Each typed character raises a `value_change`, read as above:
    the field's text crosses processes in the 42-property cache (its Value), again in
    `focus_subtree_of`, up to five times in `read_uia` (`text_context.rs:297`: the document
    range, and four prefix ranges for the caret, selection and word), and in the MSAA object's
    value; the text-selection event that comes with the key reads the MSAA value once more:
    about 8 whole or partial copies on this thread, plus the actions worker's own reads (P11).
  - **Page focus siblings.** `needs_siblings` is true for most page controls (no reported
    position), so every Tab onto a button, edit, link, check box or combo box fetches all its
    siblings, which the engine does not use for page controls (it speaks the buffer's node).
- **Measured from traces** (570 traces, 20,283 events; scratch `traces`, `caret`, `siblings`):

  | Event | Recorded | Converted to nothing after a full read | Chain nodes read for them |
  |---|---|---|---|
  | `state_change` | 3,421 | 2,503 (73%) | 10,796 |
  | `selection` | 440 | 251 (57%) | 1,657 |
  | `value_change` | 1,092 | 275 (25%) | 879 |

  - `caret` 1,992 and `text_selection_change` 1,199 (16% of all events): live, all converted to
    nothing. In WinForms edits the MSAA value read with a text-selection event averages 776 B,
    the field's whole text in the fixture.
  - Scenario 403 (typing echo): 33 `value_change` events, all 33 with the field's text read.
  - Page focus siblings read: buttons 12.4 on average, check boxes 17.6, combo boxes 22.7,
    edits 17.7, links 8.1; at most 32 in the corpus's fixtures.
- **Proposed change** (live mode only; `okur trace` keeps reading everything):
  1. **Caret and text selection:** take them out of `LIVE_EVENTS`, or return before any read.
  2. **A converter pre-filter for changes:** `EventConverter::wants_change(&self, hwnd,
     id_object, id_child, object_name) -> bool`, the same test as the top of `convert_one`'s
     change branch. `record_at` reads the MSAA object first (it needs only that), asks the
     converter (on the same thread: share it between the sink and the reader through
     `Rc<RefCell<…>>`), and skips the UIA reads for a change it will drop. Start with
     `state_change` and `selection`.
  3. **`value_change` of the focused text field while an edit key's read is pending:** the
     actions worker reads the field after every edit key and typed character. Drop the event
     live when the actions worker has a text read in flight or finished one within 300 ms (an
     atomic timestamp in `okur-platform`), so that no second, racing read answers the key. Note
     that a `value_change` without its text must not be taken for a select's value
     (`is_choice_without_text`), so drop it rather than send it without text.
  4. **One focused element per event:** pass the element `record_at` already has to
     `focus_subtree_of` and to the text read (`read_uia` takes an element), instead of asking UIA
     for the focus again. The code's own comment explains why a second query is dangerous (7 s in
     Windows 11 Notepad).
  5. **Siblings for page controls:** read them only where the converter uses them (menu items,
     list items and options, and native controls); a page button, edit, link, check box or
     combo box gets none.
- **Expected gain.** Per the traces, about seven in ten state and selection changes cost a few
  calls instead of dozens; a typed character in a native field moves its text across processes on
  this thread once (the MSAA value) instead of about 8 times; the WinEvent thread is free
  sooner for the next focus.
- **Risks.** Parity: none for replay; live behavior must stay the same. A `value_change` that an
  application raises on another object than the field (a container) would no longer feed the
  field's text; the actions worker's read covers edit keys. Platform: Windows code; the
  converter method is platform-neutral.
- **Tests.** Replay-equivalence tests over all corpus traces, hermetic: replay each trace through
  the converter; for every event the pre-filter would skip, assert that converting it gives no
  engine event and changes no converter state. For caret and text-selection events: with
  `text_context` removed, `convert` gives nothing. For the sibling rule: with `focus_siblings`
  emptied for page controls, the converter gives the same focus events. Live: old-pc scenarios
  305, 313, 400, 403, 504, 522 through `okur live-parity`.

### P6. Every focus in a dialog box reads the whole dialog again

- **Where.** `event_trace.rs:3162-3189` (the `#32770` branch): `screen_lines::dialog_layout`
  (`screen_lines.rs:513`), `window_subtree` (`uia_snapshot.rs:1015`) with `assign_group_boxes`
  (`:1037`), `page_of`, `group_box_of` (`screen_lines.rs:448`), `default_button_of` (`:484`);
  and for every focus, `window_bars` (`:250`, `event_trace.rs:3263`) and, since `6a0a7b0`,
  `is_spin_buddy` (`event_trace.rs:2477`).
- **What happens.** On every focus event in a dialog box (every Tab, every arrow that moves the
  focus), on the WinEvent thread:
  - the dialog's UIA control tree to depth 6, up to 300 nodes, one cross-process call per node
    with 42 properties, through UIA's MSAA proxy for Win32 dialogs;
  - an uncached `CurrentBoundingRectangle` per top-level control;
  - `EnumChildWindows` twice, with `WM_GETTEXT` and `WM_GETFONT` per control and GDI text
    measuring for every static;
  - `DM_GETDEFID`, the menu bar, the status bar's parts through MSAA, and the up-down controls.

  Most of it serves Say Window (Insert+B), the Virtual Viewer and Insert+E, which the user
  asks for rarely; the dialog rarely changes between two Tabs.
- **Measured from traces** (sizes, not times): the Font dialog's focus events (541) carry a
  244-node dialog tree, 14 siblings and 7 dialog lines; a recorded focus event averages 7.9 KB.
  The time per Tab is not measured: the debug log's "event read" line has it as the `window`
  stage; measure it on old-pc with 541, 323 and 524.
- **Proposed change.**
  1. A per-dialog cache on the WinEvent thread, keyed by the dialog's window (and its property
     page): the UIA tree, the layout rows and text, the group boxes' rectangles and names, the
     default button, the menu and status bars. Invalidate it on any WinEvent for a window inside
     that dialog that can change it: show, hide, reorder, create, destroy (`0x8000`-`0x8004`),
     name, value and state changes; drop it when another window comes to the front. Extend
     `on_page_change`'s hook (today Chromium pages only) to mark a cached dialog dirty, reading
     nothing. `record_at` fills the trace event from the cache, so the converter and the engine
     see the same data as now.
  2. Later: read on demand. Say Window, the Virtual Viewer, Insert+E and Say Focus's folder ask
     the platform through `ApplicationActions` (core), answered by the actions worker live and,
     in replay, from the recorded focus event's tree, so parity stays byte-identical.
  3. Check each new per-focus read (as `is_spin_buddy`) against the same rule: read for the
     controls that need it, cache per window, or read on demand.
- **Expected gain.** A Tab in a dialog reads the focus and its context only: likely tens to
  hundreds of milliseconds less per Tab on Win32 dialogs; to be confirmed with the `window`
  stage before and after.
- **Risks.** A missed invalidation leaves Say Window stale: keep the cache short-lived (the
  dialog's foreground session) and invalidate generously. Parity: replay unchanged in step 1;
  step 2 changes the engine's contract and needs the replay answer to match. Platform: the
  cache is Windows code; the on-demand trait method is core.
- **Tests.** The invalidation rules as a pure function over event kinds and windows, unit
  tested; live checks of 323, 524, 537, 540-546 with `okur live-parity`.

### P7. Event storms are read one by one, with no coalescing

- **Where.** `event_trace.rs:2220` `on_event` → `record_at`; the pump at `:3498`.
- **What happens.** Out-of-context WinEvents are delivered inside `PeekMessageW`; each is read in
  full right away. Holding an arrow key in a slow list, a dialog filling its lists, a page
  raising state changes by the hundred: every event is read even when a later one in the queue
  supersedes it. Filters exist for known storms (scroll bars, the taskbar's widget, a file
  dialog's navigation pane), one case at a time.
- **Read.**
- **Proposed change.** A limiter as NVDA's `OrderedWinEventLimiter`: `on_event` only records
  the event's key (event, window, object, child, time) in a thread-local list; after the
  `PeekMessageW` drain, the list is deduplicated (an identical key keeps its last place), focus
  events keep only the last one per drain, at most about ten events per window thread are kept,
  and then each survivor is read. Apply it in trace mode too, so that the live screen reader
  still consumes the stream the harness records, or document the difference. Also log each
  event's queueing delay (`now - t_ms`) in the "event read" debug line, and have `okur latency`
  report it: today the log has the read time but not the wait before it.
- **Expected gain.** Bursts cost one read per distinct object instead of one per event.
- **Risks.** Converter rules that depend on seeing an intermediate event (a focus repeated by
  several windows, a selection right after its focus): check them against the traces. New
  traces will hold fewer events. Platform: Windows code.
- **Tests.** The limiter as a pure function over event-key sequences, unit tested; a replay
  test that runs each corpus trace through the limiter (by recorded time, as if drained in
  50 ms windows) and the converter, listing the steps whose engine events change, reviewed
  before enabling; then live checks.

### P8. Page parts are spliced and built inline in the event loop

- **Where.** `crates/okur/src/main.rs:1100-1121` (`PagePartsLoaded`):
  `VirtualBuffer::spliced(&base, parts)` (`buffer.rs:304`, a copy of the whole page) and
  `from_aom_owned`, inside the event loop's task.
- **What happens.** Outside the engine lock (the previous review's 1c), but the event loop
  handles nothing else meanwhile: focus, caret and edit speech wait. Parts come after page
  changes, several times a minute in a busy web application.
- **Measured:** 135-175 ms on the 25,032-node page with 8 parts; 16-17 ms at 5,032.
- **Proposed change.** Do as refreshes do (`main.rs:1071`): run the splice and build on the
  blocking pool, number it, and send it back through `built_rx`; install it with
  `install_page_parts` (which checks the base with `Arc::ptr_eq`). When the base has changed by
  then, ask for a whole-page refresh instead of rebuilding under the lock (P3, step 3).
- **Expected gain.** The event loop is never held for a page build.
- **Risks.** Parts built after a newer refresh must be dropped: the base check does that.
  Holding the base `Arc` for the whole build widens the window in which Space on a check box in
  the virtual cursor (`node_mut`, `Arc::make_mut`) copies the whole page in the hook: move those
  toggles to P3's patch-and-job path first, or together. Parity: replay goes through
  `page_parts_loaded` directly; unchanged.
- **Tests.** A hermetic test of the install rules with a stale base (exists in part); the
  previous review's ordering test for refreshes, extended to parts.

### P9. Page lookups that still scan the whole page under the lock

- **Where.**
  - `crates/okur-core/src/engine/page.rs:357` `page_node_for`: after the uniqueID and DOM id
    indexes, a scan of every node by role and name, with all ids collected and sorted
    (`:379`). It runs for every page focus the indexes miss: while the page is the quick UIA
    read (its nodes have no IA2 uniqueIDs), and for content the buffer lacks. During a quiet
    return from Okur's dialogs it runs twice per focus (`engine.rs:1185`).
  - `page.rs:831` `region_old_lines`: the same sorted scan by DOM id, once per live region.
  - `page.rs:268`, `:325`, `:960` (`entered_containers`, `region_markers`, `in_widget_item`):
    `ancestors_of` walks the whole tree; the previous review proposed `buffer.ancestors(id)`.
  - `page.rs:591` `apply_pending_page`: a linear `node_with_dom_id` in the hook's first key.
- **Measured:** a page focus not in the indexes, 11-17 ms on the 25,032-node page, 1.5 ms at
  5,032; found by uniqueID, 0.002-0.004 ms.
- **Proposed change.**
  1. A role-and-name index in `VirtualBuffer`, built lazily: (role class, ASCII-lowercased name
     or text) → smallest id among readable nodes. `eq_ignore_ascii_case` compares ASCII letters
     only, so ASCII lowercasing gives the same matches. `same_role`'s equivalences are looked up
     as their alternatives, taking the smallest id.
  2. `region_old_lines` uses `node_with_dom_id_indexed`, which already gives the smallest
     readable id with that DOM id, the scan's answer.
  3. `buffer.ancestors(id)` where the id is not repeated; where the page repeats ids, keep the
     current semantics (first node in document order: slots are numbered in document order, so
     an index by slot gives it).
- **Expected gain.** 11-17 ms to microseconds per page focus on big pages.
- **Risks.** Parity: identical results by construction; the tests prove it.
- **Tests.** For every node of every corpus snapshot, and random names, the index answers as
  the scan; same for DOM ids and ancestors.

### P10. List dialogs and offset lookups scan every span

- **Where.** `buffer.rs:1005-1116` (`find_form_control_node`, `find_button_node`,
  `find_link_node`, …): each scans all spans for one with this start; `get_links_list`
  (`:1215`), `get_headings_list`, `get_form_fields_list` (`:1270`) call one per item. Spans are
  not sorted by start: containers and table cells are pushed after their content.
- **What happens.** In the hook: Insert+F7, Insert+F5, Insert+F6 (one lookup per heading), the
  quick keys that resolve a node by offset, Backslash (mail links), Alt+Insert+Down.
- **Measured** (25,032 nodes, 5,000 links and 5,000 check boxes): Insert+F7 25-62 ms, Insert+F5
  24-84 ms, Alt+Insert+Down 17-78 ms, Backslash 5-10 ms. The page has no headings; a page with
  thousands (a long article, an issue thread) pays the same per heading.
- **Proposed change.** A `spans_by_start: Vec<u32>` of span indexes, stably sorted by start
  offset, built at the end of linearization (or lazily). A lookup binary-searches the first
  index with this start and walks the spans with that start in push order; the first whose role
  fits is the answer, the same one the linear scan finds.
- **Expected gain.** These keys to well under 1 ms.
- **Risks.** Equal by construction (stable sort keeps push order among equal starts).
- **Tests.** For every span start of every corpus snapshot and every `find_*` kind, the indexed
  lookup equals the scan.

### P11. Text reads after keys are heavier than they need be

- **Where.** `crates/okur-platform/src/actions.rs:419` `read_other_after_key`, `:386`
  `read_chromium_after_key`, the worker at `:140`; `text_context.rs:297` `read_uia`, `:275`
  `offset_of`, `:179` `from_ia2`; `crates/xa11y/xa11y-windows/src/ia2.rs:875`
  `ChromiumTextField::misspelled`.
- **What happens.**
  - Native fields (Notepad, WordPad, WinForms): every poll (every 15 ms, up to 250 ms, 500 ms
    with a selection) reads the whole text and four prefixes; the previous review's finding
    13, still open. With a 100,000-character document the caret's prefix alone is the whole
    text when the caret is near the end.
  - Chromium fields: each read (once per key, after the cheap position poll) scans the field's
    attribute runs for spelling errors, one cross-process call per run, up to 256; `from_ia2`
    converts each run's offsets by walking the text from its start (runs × length).
  - One serial worker: a long read (Virtual Viewer text, page elements, a page action waiting
    up to 500 ms for a page) delays the edit-key read behind it; the previous review's
    finding 17, still open.
  - The worker's UIA client is made without timeouts (`actions.rs:147`).
- **Read.**
- **Proposed change.**
  1. Poll cheaply, read once: for Win32 edits, poll `EM_GETSEL` and `WM_GETTEXTLENGTH` (two
     messages with a timeout) until they change, then read; for other UIA fields, keep the
     selection range from the first read and poll with `CompareEndpoints` against it. Compute the
     caret offset with one prefix, the selection's end from the selection's own text, the word's
     ends from the caret and the word range's text.
  2. Spelling errors: only the caret's line is ever used (`editing::misspelled_mark` is the
     only reader of `TextContext::misspelled`), so scan the runs of that line only: the range
     `editing::line_range` gives (between line breaks), in UTF-16 units of the text already
     read. Convert offsets in one pass.
  3. Two lanes in the worker: text reads after keys on their own thread, so that viewer and page
     reads never delay them; page actions on the other.
  4. Create the worker's client with `automation()` (1 s timeouts).
- **Expected gain.** A key in a long native document costs a few messages per poll instead of
  several copies of the document; a Chromium field with many spelling errors no longer costs up
  to 256 calls per key.
- **Risks.** Offsets computed differently must equal the current ones (line breaks, surrogate
  pairs); spelling marks on the caret's line are what the engine reads today, so speech stays
  the same, but new traces record fewer ranges. Live only; parity replays recorded contexts.
- **Tests.** Offset arithmetic as pure functions over texts with `\r\n`, surrogates and emoji,
  unit tested against the current results; live checks of 305, 313, 400, 403, 504, 505, 522,
  284 on old-pc.

### P12. Page captures read what is never used

- **Where.** `ia2.rs:513` `read_node`, `:393` `read_text`, `:455` `read_relations` (xa11y);
  used live by `capture_foreground_page`, `read_changed_parts` and live regions;
  `crates/okur-platform/src/ia2_snapshot.rs` `ia2_to_aom`.
- **What happens.** For every object with text, the IA2 capture reads its hypertext
  (`get_text`), every rendered line (one call per line) and every attribute run (one call per
  run, up to 512). `ia2_to_aom` uses none of the hypertext or lines, and only the first run.
  `read_relations` resolves the targets of every relation (about five calls per target), and only
  `flowsTo` is used. On the page worker, so it delays the full page (the "refine" after the
  quick read) and partial reads rather than blocking keys.
- **Measured** (estimate from the 18 corpus IA2 snapshots, which record what the capture read):
  715 objects, 693 with text, 985 lines, 595 runs; of about 15,000 calls, about 2,400 (16%) read
  data the mapping ignores. Text-heavy real pages (long wrapped paragraphs, many styled runs since
  colors are kept, `087bf74`) and labelled forms (relations) give a larger share.
- **Proposed change.** A capture detail in xa11y: `Full` for `okur snapshot --api ia2` (the
  harness keeps everything), `Live` for Okur: no hypertext, no lines, the first run only, and
  relation targets only for `flowsTo` (read the type first).
- **Expected gain.** Roughly a sixth or more off every live IA2 read.
- **Risks.** None for the AOM if the mapping ignores the left-out data, which the test checks.
- **Tests.** For every corpus IA2 snapshot: `ia2_to_aom(snapshot)` equals `ia2_to_aom` of the
  snapshot stripped of what `Live` leaves out.

### P13. UIA event handlers make cross-process calls on UIA's threads

- **Where.** `event_trace.rs:2845`, `:2853` (live regions: `CurrentProcessId`,
  `CurrentFrameworkId`), `:2568` (selection: the foreground process's name per event), `:2621`
  (notifications); registrations at `:3447`, `:3463`, `:3485` on the desktop with
  `TreeScope_Subtree` and no cache request.
- **What happens.** Every selection, live region change and notification anywhere on the
  desktop reaches Okur; the handler then asks the element for properties across processes, or
  opens the foreground process to read its name, before filtering. A slow call there holds the
  UIA thread delivering the event, and the events UIA delivers after it wait.
- **Read.**
- **Proposed change.** Pass a cache request with `ProcessId` and `FrameworkId` to the
  registrations and read `Cached*`; keep the foreground window's process name in a small cache
  refreshed only when `GetForegroundWindow` changes; cache process names by id. Consider
  registering selection and live region handlers on the foreground window only, re-registering on
  a foreground change (NVDA's selective registration); notifications stay desktop-wide (the
  shell's toasts).
- **Expected gain.** Fewer cross-process calls on UIA's threads; less system-wide event traffic.
- **Risks.** Selective registration could miss events of windows owned by another (popups):
  check the task switcher, Windows search and Settings on old-pc.
- **Tests.** Live (`#[ignore = "live: ..."]`) and old-pc scenarios 316, 533, Windows search.

### P14. Memory: a big page is 44 MB in almost a million allocations

- **Where.** `crates/okur-core/src/aom.rs` (`AomNode::attributes: HashMap<String, String>` per
  node), `buffer.rs:182` (`VirtualBuffer`: a `String` per word, line text and speech per line,
  `HashMap`/`HashSet` indexes keyed by `usize` and by cloned `String`s).
- **Measured** (counting allocator): the 25,032-node page is 20.0 MB in 335,299 allocations
  (800 B and 13 allocations per node), and its buffer adds 23.5 MB in 625,367 allocations
  (120,001 words). Building takes 72-127 ms, freeing 108-153 ms (on the retire thread). While a
  refresh is built, up to three such pages are alive (the buffer, the pending page, the new
  build). At 5,032 nodes: 4.0 MB and 5.1 MB.
- **Proposed change**, by gain per effort:
  1. `BufferWord` without its own `String`: its text is `buffer.text[start..end]`.
  2. Dense ids: `slot_of` as a `Vec<u32>` by id, `readable` as a bit set (capture ids are
     sequential; spliced parts continue them).
  3. A compact attribute map: a small sorted `Vec<(Box<str>, Box<str>)>` behind a type with
     `get`, `insert` and iteration, so call sites stay as they are.
- **Expected gain.** Roughly half the allocations; builds, copies and frees faster in proportion.
- **Risks.** (3) touches every attribute access; mechanical but wide. Parity: no output change.
- **Tests.** Existing tests; a memory example with a counting allocator, run by hand.

### P15. Smaller costs

Each measured or read as marked; none is a stall alone.

- **`FocusEvent` copies.** The converter keeps a copy of each focus (`event_trace.rs:1272`);
  the engine copies it in `keys_as_output` (`focus.rs:111`) and into `last_native_focus`
  (`focus.rs:788`); a change copies the last focus's `window_items` (`event_trace.rs:1329`).
  Measured: 21-35 µs per copy with 300 window items or a 107k-character text. Put
  `window_items`, `dialog_lines` and the text behind `Arc` if they grow.
- **`window_items`** (`event_trace.rs:249`): copies each child with its whole subtree, then
  clears the children. Read. Build the leaf without the subtree.
- **okur-ui list fills** (`crates/okur-ui/src/dialog.rs:244`, `:264`, `:357`): one insert per
  item with redraw on, one expand per tree item; the Links List's filter refills on every
  letter. Read (the previous review's finding 17, open). `WM_SETREDRAW` off around fills,
  `LVM_SETITEMCOUNT`/`LB_INITSTORAGE`, `TVIS_EXPANDED` at insert; an owner-data list view past
  about 1,000 items.
- **Mouse polling** (`event_trace.rs:2756`, `:3536`): the WinEvent thread wakes every 50 ms
  forever to look at the pointer, and each move is an engine event. Measured in the engine:
  negligible. Coalesce moves (keep the latest) and poll only while a feature needs the pointer.
- **Settings saved under the engine lock** (`engine.rs:1054`): a file write and rename on the
  event loop when the settings dialog applies. Read. Hand a copy to a writer thread.
- **`ApplicationActions::clipboard_text()`** (`actions.rs:510`) is synchronous and documented
  for the clipboard keys; reading the clipboard can wait on its owner (delayed rendering). Read.
  Unused by the engine today; make it a request answered by an event before a key uses it.
- **Debug log** (`events/windows.rs:418`): with `--debug-log`, every non-focus event is logged
  with `?other`, a `TextContext` with the field's whole text, a change with its whole focus
  event. Read. Log sizes and kinds, as the actions worker already does for texts.
- **`properties_request`** (`uia_snapshot.rs:622`): a new cache request with 42 properties for
  each call, several per event. Read. Keep one per thread.
- **Process names** (`event_trace.rs:3287`, `:1792`): `OpenProcess` and
  `QueryFullProcessImageNameW` for every event read. Read. Cache by process id.
- **`node()` allocates a path** (`buffer.rs:533`, the previous review's 2c): measured
  0.6-0.7 µs per call at 25,032 nodes. Walk the slots without a `Vec` if it ever shows.

## The previous review's open items

| Item | Status at `dd48218` |
|---|---|
| 2c `node()` allocates | open; measured small (P15) |
| 4 page focus scans | uniqueID and DOM id indexes done; the name scan, `ancestors_of` and `region_old_lines` remain (P9) |
| 1c rebuild under the lock | parts: done; but inline in the event loop (P8) |
| 7 UIA clients | `os_version` done; the event thread's client has 1 s timeouts; captures and the actions worker still make their own with defaults; `FindFirst` remains (P4, P11) |
| 9 speech log writes | unchanged, as decided |
| 11 rebuild before speaking | `page_change` speaks first; the focus path still rebuilds first (P3) |
| 13 UIA text polling | open (P11) |
| 16 cross-process timeouts | open (P4) |
| 17 smaller costs | `editing::chars()` measured small (fine); linearizer allocations (P14); actions worker queue (P11); okur-ui fills (P15) |

Nothing the previous review measured has regressed: the first key after a refresh is
0.25-0.36 ms (was 0.9 ms), Tab from the virtual cursor 0.001 ms, a page focus by uniqueID
0.002-0.004 ms. The new hook stalls come from code the previous review did not time (element
lists, `086a015`) and from code added since (Z and Shift+Z, `311c4bf`).

## Suggested order of implementation

Small and provably equivalent first, then structural:

1. P1 (subtree-start index, capped `block_text`, offsets for Z), with its oracle tests.
2. P5 step 1 (no reads for caret and text-selection events live).
3. P10 (spans by start) and P9 (role-and-name index, indexed DOM ids).
4. P2 (re-install the hook after a slow key; priorities after measuring under load).
5. P4 steps 1-3 (IA2 fallback to the page worker, one UIA client per thread, hung windows).
6. P5 steps 2, 4, 5 with the replay-equivalence tests; then step 3 with live checks.
7. P8 (parts on the blocking pool).
8. P3 (patch, speak, rebuild as a job), then its incremental update later.
9. P12 (IA2 capture detail) with the snapshot test.
10. P11 (cheap polls, spelling on the caret's line, two lanes), with live checks.
11. P6 (dialog cache, then on-demand reads), measuring the `window` stage first.
12. P7 (limiter, queue delay in the log) and P4 steps 4-6 (live regions, watchdog).
13. P13, P14, P15.

Each step: `parity-snapshot.sh` before and after, `diff -r` empty; the gate's fmt, clippy
(`-D warnings -D clippy::indexing_slicing -D clippy::string_slice`) and `cargo test`.

## Checked and found fine

- Browse-mode keys on the 25,032-node page: Down, Up, Left, H, X, Tab, Insert+Down (Say All
  start): 0.001-0.07 ms (release and dev). The 814-chord sweep found only the chords in P1 and
  P10 above 10 ms; two single-run outliers (Ctrl+F 238 ms, Up 165 ms) gave 0.01-0.03 ms in five
  reruns each.
- The first key after a refresh (`apply_pending_page`): 0.25-0.36 ms.
- A page focus found by uniqueID: 0.002-0.004 ms. Mouse moves in the engine: about 0.01 µs
  each. `on_idle`: nothing measurable.
- The converter: all 20,283 recorded events in 86-245 ms (4-12 µs each, at most 0.45-4.8 ms).
- Parity replay: 0.9-1.0 s (IA2 and UIA layers) and 1.4 s (spoken), release; parsing all 570
  traces about 1 s warm.
- The message catalog: 0.6-2.6 ms on first use, built lazily; 0.4-0.6 µs per formatted message.
  (A first run of a freshly built binary measured 22 ms: disk, not the catalog.)
- Edit speech in a 107k-character field: 0.08-0.36 ms; `editing::chars()` is not worth changing.
- Freeing buffers on the retire thread while keys are handled: key p99 0.03-0.12 ms, at most
  2-3 ms while six 25,032-node buffers are freed.
- Speech: one thread; superseded requests skipped; the voice re-applied only when it changed;
  voices cached; Say All handed over three lines ahead.
- Locks: no ordering cycle among the engine mutex, the hook's statics, the speech manager (the
  engine only `try_lock`s it), the page requests, the retire queue and the UIA queues. Keys go
  before events (`KEYS_WAITING`).
- Logging: a writer thread; `tracing` field expressions are not evaluated when the level is off.
- okur-ui: messages written from a thread of their own; the engine never waits on it.
- Bounded state: speech history 50, JAWS Find history 20 per site, UIA queues 8, changed page
  parts 8; live-region maps cleared with each page; menus reset on menu end; settings looked up
  again only when the application or site changes.
- `os_version` once per process; the voice list cached; the clipboard written on the actions
  worker; refreshes of the same page built on the blocking pool and numbered.

## Method

The scratch crate (not in the repository) depends on `okur-core` and `okur-platform` by path,
from `git archive HEAD` of `dd48218` exported to a scratch folder, built with its own
`CARGO_TARGET_DIR`, in release and in a dev profile matching the repository's (its crates at
opt-level 1, dependencies at 3). Its binaries, to rebuild if needed:

- `core`: buffer builds and drops, element lists, `with_node_changed`, splices, engine focus and
  change events, mouse moves, the first key after a refresh, list keys, catalog first use.
- `keys`: every chord of 74 keys with 11 modifier sets (814) through `handle_key_event` on a
  fresh engine with the page loaded and the cursor mid-page; prints those over 2 ms; with chord
  names, five runs each.
- `focus`: page focus onto check boxes whose state the buffer has wrong, with a counting
  allocator for peaks.
- `traces`, `chains`, `siblings`, `caret`: per-event read sizes, converter time and drop counts
  over `corpus/traces`.
- `ia2calls`: calls an IA2 capture makes, from the corpus's IA2 snapshots.
- `memory`: heap bytes and allocations of a page and its buffer (counting global allocator).
- `contention`: key handling while another thread frees or builds buffers.
- `proto`: the one-pass subtree-start index of P1 against `start_of_subtree`.

The synthetic page (5 nodes per row: a division with a link, a paragraph, a check box and a
time text, under 30 wrapper divisions, each node with an IA2 uniqueID, `tag`, `id` and
`display`) is easy to rebuild from that description; adding it and the `keys` sweep to
`crates/okur-core/examples/` would let every change to a key path be timed on a big page.
