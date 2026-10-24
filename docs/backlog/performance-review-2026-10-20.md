# Performance, responsiveness and memory review (2026-10-20)

Fixed findings were removed on 2026-10-24 and are in git history; the open ones keep their numbers.

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

## Summary: the five worst

2. **P4. One hung application, or one chat log, can stop all focus speech.** Read. Every
   cross-process read for an event runs synchronously on the WinEvent thread; MSAA and IA2 calls
   have no timeout; a Chromium live region is read whole through IA2 on that thread for every
   change (about 12 s per change for a 5,000-object log, by the code's own IA2 rate).
4. **P5 and P6. The WinEvent thread reads what it then drops.** Measured from traces: a typed
   character in a native field moves its text, whole or in part, across processes about 8
   times on that thread.

## Findings, most severe first

### P2. Nothing bounds the hook's work, and nothing notices when Windows removes the hook

- **Where.** `crates/okur/src/main.rs:959` (the hook callback), `:657` `take_for_key`;
  `crates/okur-platform/src/hook/windows.rs:134` (the hook thread), `:234`
  `low_level_keyboard_proc`.
- **What happens.** The callback waits up to 150 ms for the engine, then runs
  `handle_key_event` with no limit. Windows passes a key on, and on Windows 7 and later
  silently removes the hook, once a callback runs past `LowLevelHooksTimeout` (not set on the
  dev box, so Windows' default applies; it is capped at 1 s from Windows 10 1709). The hook
  thread runs at normal priority, so a busy machine (a Rust build) delays every key.
- **Read.**
- **Proposed change.**
  2. Raise the hook thread to `THREAD_PRIORITY_HIGHEST` at start; it only does short work once
     P1 is fixed. Beware priority inversion: the hook waits on the engine mutex held by the
     event loop, a tokio worker at normal priority. Run the event loop on a dedicated OS thread
     at `ABOVE_NORMAL` (it is one synchronous consumer already), or keep both at normal.
  3. A rule for the engine, written into `docs/references/`: nothing in a key's path may be
     O(page). Work that is (list dialogs, Say All texts, searches) is handed to the event loop
     as a job and spoken when done; replay runs the job at once, so parity does not change.
- **Risks.** Priority: test with a CPU-bound build running.

### P3. Page rebuilds under the engine lock, at the hook's budget

- **Proposed change.**
  2. Later, an incremental line update: relinearize the changed node's lines and shift the
     offsets after it in every index. Only with a property test that it equals a full rebuild.

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
  - A Chromium live region change (`live_region_change`) reads the region's whole subtree through
    IA2 (`capture_chromium_object`) on that thread, once per event. A chat log marked as a live
    region holds every message; at the same rate (about 2.5 ms an object), a 5,000-object log
    takes about 12 s per change, and a streamed reply raises several changes a second.
- **Read** (the live-region time is an estimate from the code's IA2 rate, not measured).
- **Proposed change**, small steps first:
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
- **Risks.** Coalesced live regions: what a growing region adds is the same text, read in
  fewer, larger pieces (`LiveRegionAppend::Appended` already gathers pieces); `WholeText`
  repeats less. Parity: live only (replay reads recorded regions). Platform: Windows code; the
  "page loading" notion, if added, is a core event.
- **Tests.** The dirty-set and bound logic as pure functions with unit tests; the hung-window
  and cancellation paths as live tests, checked on old-pc with a test application that sleeps
  in its window procedure.

### P5. The WinEvent thread reads in full for events it then drops

- **Where.** `event_trace.rs:2955` `record_at`: `:3049` (the focused element), `:3098`
  `with_properties`, `:3104` `focus_chain_from`, `:3130` siblings, `:3141`
  `focus_subtree_of`, `:3210` the text context, `:3231` the MSAA object; the converter's
  `convert_one`, `event_trace.rs:1182` onward.
- **What happens**, per event on the WinEvent thread:
  - **Changes of objects that are not the focus.** For every `value_change` in the foreground
    process, the reader asks UIA for the focused element, its 42 properties, its ancestors one
    cross-process call each, a Win32 list position, the focused element's subtree, the focused
    field's whole text and the event's MSAA object. The converter then keeps the change only
    when it is about the focused object (`same_object || same_name`).
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

  - Scenario 403 (typing echo): 33 `value_change` events, all 33 with the field's text read.
  - Page focus siblings read: buttons 12.4 on average, check boxes 17.6, combo boxes 22.7,
    edits 17.7, links 8.1; at most 32 in the corpus's fixtures.
- **Proposed change** (live mode only; `okur trace` keeps reading everything):
  3. **`value_change` of the focused text field while an edit key's read is pending:** the
     actions worker reads the field after every edit key and typed character. Drop the event
     live when the actions worker has a text read in flight or finished one within 300 ms (an
     atomic timestamp in `okur-platform`), so that no second, racing read answers the key. Note
     that a `value_change` without its text must not be taken for a select's value
     (`is_choice_without_text`), so drop it rather than send it without text.
  5. **Siblings for page controls:** read them only where the converter uses them (menu items,
     list items and options, and native controls); a page button, edit, link, check box or
     combo box gets none.
- **Expected gain.** A typed character in a native field moves its text across processes on
  this thread once (the MSAA value) instead of about 8 times; the WinEvent thread is free
  sooner for the next focus.
- **Risks.** Parity: none for replay; live behavior must stay the same. A `value_change` that an
  application raises on another object than the field (a container) would no longer feed the
  field's text; the actions worker's read covers edit keys. Platform: Windows code; the
  converter method is platform-neutral.
- **Tests.** Replay-equivalence tests over all corpus traces, hermetic: with `focus_siblings`
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
- **Proposed change.**
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
| 9 speech log writes | unchanged, as decided |
| 13 UIA text polling | open (P11) |
| 16 cross-process timeouts | open (P4) |
| 17 smaller costs | `editing::chars()` measured small (fine); linearizer allocations (P14) |

Nothing the previous review measured has regressed: the first key after a refresh is
0.25-0.36 ms (was 0.9 ms), Tab from the virtual cursor 0.001 ms, a page focus by uniqueID
0.002-0.004 ms.

## Suggested order of implementation

Small and provably equivalent first, then structural:

1. P2 (priorities after measuring under load).
2. P5 step 5 with the replay-equivalence tests; then step 3 with live checks.
3. P3's incremental update.
4. P12 (IA2 capture detail) with the snapshot test.
5. P11 (cheap polls, spelling on the caret's line), with live checks.
6. P6 (on-demand reads).
7. P7 (limiter, queue delay in the log) and P4 steps 4-6 (live regions, watchdog).
8. P13, P14, P15.

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
`display`) is easy to rebuild from that description.
