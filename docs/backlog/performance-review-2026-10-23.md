# Performance, latency and architecture review (2026-10-23)

The owner asked for a review hunting inefficiencies, sources of lag and stalls, whole reads
where a partial one would do, naive implementations, memory waste and smelly architecture,
and for the findings to be fixed. Three reviewers read the code at `a0628bf4`, read-only, one
each for threading and latency (T), the core engine (C) and the platform layer (P). It builds
on [`performance-review-2026-10-20.md`](performance-review-2026-10-20.md) and
[`responsiveness-review-2026-10-19.md`](responsiveness-review-2026-10-19.md).

Line numbers are as of `a0628bf4`. Each finding below names what the reviewer saw and what was
done. A change to what the live reader reads does not show in parity (replay feeds recorded
traces), so such changes get tests where a test can hold them and a live run on old-pc.

## Status

| Finding | Status | Commit |
|---|---|---|
| P1: a WinEvent delivered inside another's read aborted the reader | done: events wait their turn (`in_turn`) | this batch |
| P2, T2: the clipboard read in the keyboard hook, unbounded by its block's size | done: read on a thread of its own, waited for 150 ms; the scan bounded by `GlobalSize` | this batch |
| P23: `set_clipboard_text` leaked its block when not set | done | this batch |
| T3: a drag slept 120 ms in the keyboard hook | done: on the actions worker | this batch |
| P24: `EM_GETSEL`'s 16-bit words | done: the out-pointers | this batch |
| T8: a panic while the engine was held left the reader deaf | done: the lock recovered, a key's or an event's panic caught and logged, every panic logged | this batch |

The rest are listed below in the order they are taken.

## Threading and latency (T)

1. **The engine runs on the keyboard hook thread**, under a mutex shared with the event loop
   (`run.rs:629-671`, `hook/windows.rs:370-374`). The hook waits up to 150 ms for the lock,
   then the engine's work has no limit; the reinstall after a slow key, `KEYS_WAITING` and the
   `retire` thread work around it. The fix is architectural: the hook decides consume or pass
   from a routing snapshot the engine publishes, and one engine thread owns the engine and
   serves keys before events. Planned, not started.
2. The clipboard read on the hook thread: see P2.
3. The drag's sleeps on the hook: done.
4. The lock handoff polls (`take_for_key`, `give_way_to_keys`). Goes with T1.
5. Idle wakeups: the 150 ms idle tick, the WinEvent pump's 50 ms wait and `GetCursorPos` poll.
6. Every event, mouse moves too, postpones idle work (`timeout(idle, ...)` restarts).
7. Rebuilds after page-control changes are not coalesced, and `Arc::make_mut` deep-copies the
   page under the lock while a build holds it.
8. A poisoned engine mutex: done.
9. Shutdown joins the hook and WinEvent threads, which may be stuck.
10. One actions worker serves everything; a slow read delays injected keys.
11. Settings and rule files written under the engine lock.
12. Stop cannot interrupt a long utterance (`thread.rs:311-318`).
13. OneCore polls its synthesis status and audio padding.
14. Say All progress polled every 20 ms.
15. The okur-ui viewer rescans its text for UTF-16 offsets on every redraw (`win32.rs:121-133`).
16. okur-ui list refills without `WM_SETREDRAW` (`dialog.rs:365-429, 529-550`).
17. Logging without `--debug-log` writes to the console synchronously.
18. The speech log written synchronously on the key path.
19. Two `replace` calls on every spoken event string, logged or not (`run.rs:875-884`).
20. Speech history entries have no size limit (Say Clipboard keeps the whole clipboard).

## Core engine (C)

1. Skim by attribute rule scans every span for every byte of the text (`say_all.rs:224-229`);
   `inline_texts_with` clones its siblings per child.
2. `prepared()` clones and rebuilds pages per refresh (`with_reading_choices`, `labeled`,
   `customized_page`, `keep_original`), undoing the off-engine build.
3. Sentence keys rebuild every sentence on the page per press (`cursor.rs:1630-1676`).
4. Edit keys turn the field's text into `Vec<char>` 2-4 times per key; `TextContext` cloned.
5. `node_mut` drops `PageIndex` even for a state change.
6. Page focus events walk the tree several times (`ancestors_of`, a recursive DFS).
7. `FocusEvent` clones.
8. Live regions copy their text 3-4 times per event.
9. The Z key builds the whole Divisions list, text included, per press.
10. S, D and N rebuild `element_stops` / `non_link_text_lines` per press.
11. `skim_sentences` is quadratic (`say_all.rs:353`).
12. `node()` and `ancestors()` allocate a path per call; SipHash on dense ids.
13. Page text stored 5-6 times.
14. `record_focus_stop` is quadratic in page building.
15. Focus between the page and the address bar rebuilds the page twice.
16. Attributes are strings on hot paths.
17. Recursive walks on the hook thread.
18. `command_for` scans 276 bindings and clones, up to 6 times per key.
19. `quick_nav` copies all targets, then searches linearly.
20. Dead `*_from` and `find_*_node` helpers; two DOM-id lookups used inconsistently.
21. `field_texts` never cleared when a refresh renumbers ids.
22. Field Say All clones the whole `TextContext` per piece.
23. `ScreenReaderEngine` has 135 fields; forms mode held twice.
24. A new `MockSpeechOutput` boxed per focus event to mute speech.
25. Per-line `to_lowercase` in searches and the word list.

## Platform (P)

1. A nested WinEvent aborted the reader: done.
2. The clipboard read: done.
3. A UIA text read transfers the document about five times, polled for up to 500 ms.
4. Every typed character raises a full live event read (`value_change`).
5. New pages captured on the WinEvent thread with a new `CUIAutomation8`, no timeout.
6. Most COM clients have no timeout.
7. `GetFocusedElement` up to three times per change event.
8. The whole focus chain read again for every event.
9. Web fields read in full on every key (256 `get_attributes` calls).
10. The MSAA object read twice for live state and selection changes.
11. The UIA handlers make cross-process calls on UIA's threads.
12. The page's Document found by `FindFirst(Descendants)` from the top window.
13. The IA2 capture has no node cap.
14. Column headers read uncached on every list item's focus.
15. The status bar read through MSAA on every focus.
16. Focus events never coalesced.
17. Cache requests rebuilt per call.
18. `LOCATIONCHANGE` hooked live, unused.
19. A dialog's tree cloned per focus.
20. `OpenProcess` per event for the process name.
21. `event_trace.rs` is 5.9k lines doing five jobs; event kinds are strings.
22. xa11y-windows `uia.rs` and `bridge.rs` unused on Windows.
23. `set_clipboard_text` leak: done.
24. `EM_GETSEL`: done.
