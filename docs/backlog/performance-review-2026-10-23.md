# Performance, latency and architecture review (2026-10-23)

Fixed findings were removed on 2026-10-24 and are in git history; the others keep their numbers.

The owner asked for a review hunting inefficiencies, sources of lag and stalls, whole reads
where a partial one would do, naive implementations, memory waste and smelly architecture,
and for the findings to be fixed. Three reviewers read the code at `a0628bf4`, read-only, one
each for threading and latency (T), the core engine (C) and the platform layer (P). It builds
on [`performance-review-2026-10-20.md`](performance-review-2026-10-20.md) and
[`responsiveness-review-2026-10-19.md`](responsiveness-review-2026-10-19.md).

Line numbers are as of `a0628bf4`. Every change to the engine kept the three parity runs byte-identical (`skills/jaws-reference/scripts/parity-snapshot.sh` before and after, `diff -r`). Each finding below names what the reviewer saw and what was
done. A change to what the live reader reads does not show in parity (replay feeds recorded
traces), so such changes get tests where a test can hold them and a live run on old-pc.

## Status

| Finding | Status | Commit |
|---|---|---|
| C19, C24 | not changed: the copy is of offsets already gathered; the mute happens only on a quiet return from Okur's dialogs | |
| P17, T11, T18 | not changed: in-process calls of microseconds; files written on the user's own changes; the speech log is the harness's | |
| P4: a full read per typed character | open: dropping the read live turns the event into a value change the engine would say; needs a trace of typing to decide | |
| P5 (capture) | kept: the quick capture runs on the WinEvent thread so that a new page reaches the engine before its first focus, which is then said in the page; on the page worker the focus would come first | |
| T13, T14: OneCore's synthesis and audio polled every 2-4 ms; Say All's progress every 20 ms | not changed: each poll costs microseconds and delays speech by at most its interval; an event-driven player is a rewrite for no audible gain | |
| P9, P11, P20 | not changed: a plain field has one attribute run; UIA's process id comes from the window, not the application; the process name is a kernel query | |
| C7, C10 | not changed: a focus's copy is microseconds beside its reads; S, D and N take 3.5-4.3 ms on 25,032 nodes (measured below) | |
| C15: focus between the page and the address bar rebuilt the page twice | not found in the engine: `big_page_keys` (5,000 rows, release) times the focus to the address bar at 20 µs, back to the page's link at 10 µs and the next key at 9 µs; the platform captures a page again only when its address changes (`load_page_if_new`), which leaving for the address bar does not; to confirm in a live log (F6 and back on a big page) | |

The rest are listed below in the order they are taken.

## Threading and latency (T)

1. **The engine runs on the keyboard hook thread**, under a mutex shared with the event loop
   (`run.rs:629-671`, `hook/windows.rs:370-374`). The hook waits up to 150 ms for the lock,
   then the engine's work has no limit; the reinstall after a slow key, `KEYS_WAITING` and the
   `retire` thread work around it. The fix is architectural: the hook decides consume or pass
   from a routing snapshot the engine publishes, and one engine thread owns the engine and
   serves keys before events. Planned, not started.
4. The lock handoff polls (`take_for_key`, `give_way_to_keys`). Goes with T1.
5. Idle wakeups: the 150 ms idle tick, the WinEvent pump's 50 ms wait and `GetCursorPos` poll.
7. Rebuilds after page-control changes are not coalesced, and `Arc::make_mut` deep-copies the
   page under the lock while a build holds it.
11. Settings and rule files written under the engine lock.
13. OneCore polls its synthesis status and audio padding.
14. Say All progress polled every 20 ms.
17. Logging without `--debug-log` writes to the console synchronously.
18. The speech log written synchronously on the key path.

## Core engine (C)

2. `prepared()` clones and rebuilds pages per refresh (`labeled`, `customized_page`,
   `keep_original`), undoing the off-engine build.
4. Edit keys turn the field's text into `Vec<char>` 2-4 times per key; `TextContext` cloned.
7. `FocusEvent` clones.
10. S, D and N rebuild `element_stops` / `non_link_text_lines` per press.
12. `node()` and `ancestors()` allocate a path per call; SipHash on dense ids.
13. Page text stored 5-6 times.
14. `record_focus_stop` is quadratic in page building.
15. Focus between the page and the address bar rebuilds the page twice.
16. Attributes are strings on hot paths.
17. Recursive walks on the hook thread.
19. `quick_nav` copies all targets, then searches linearly.
20. Dead `*_from` and `find_*_node` helpers; two DOM-id lookups used inconsistently.
22. Field Say All clones the whole `TextContext` per piece.
23. `ScreenReaderEngine` has 135 fields; forms mode held twice.
24. A new `MockSpeechOutput` boxed per focus event to mute speech.
25. Per-line `to_lowercase` in searches and the word list.

## Platform (P)

4. Every typed character raises a full live event read (`value_change`).
5. New pages captured on the WinEvent thread.
6. Most COM clients have no timeout.
8. The whole focus chain read again for every event.
9. Web fields read in full on every key (256 `get_attributes` calls).
11. The UIA handlers make cross-process calls on UIA's threads.
13. The IA2 capture has no node cap.
15. The status bar read through MSAA on every focus.
16. Focus events never coalesced.
17. Cache requests rebuilt per call.
19. A dialog's tree cloned per focus.
20. `OpenProcess` per event for the process name.
21. `event_trace.rs` is 5.9k lines doing five jobs; event kinds are strings.
22. xa11y-windows `uia.rs` and `bridge.rs` unused on Windows.

## Measured after the fixes: keys on a big page

`cargo run -p okur-core --release --example big_page_keys` now also presses 90 everyday page
keys from the middle of the reviews' synthetic page (25,032 nodes), one after another on one
engine, as the hook runs them. None took 5 ms; the slowest (2026-10-23, release): Insert+F5
4.5 ms, Shift+S 4.3, D 4.2, Insert+F7 4.1, Shift+D 4.0, S 3.5, V 3.2, U 3.1, Shift+U 2.3,
Insert+Down 1.5, Ctrl+Insert+Down 1.3; the rest under 0.2 ms. The first key after a refresh,
which applies the page built off the engine, took 0.5 ms (median of 3). With live keys at 1.0 ms p99
(live run 260), the hook's remaining risk is the wait for the engine while the event loop
holds it, bounded at 150 ms. C10 (S, D, N) is left as it is at these times, and T1 below is
no longer the next step: it is kept for when a measurement shows the wait.

## T1, the engine in the keyboard hook: the plan

The one finding that is the design itself. Every key runs `handle_key_event` inside the
low-level hook's callback, under the mutex the event loop also takes; the 150 ms wait for the
lock, the re-install after a slow key, `KEYS_WAITING` and the `retire` thread all work around
it, and every other finding that runs "on the hook" (the clipboard, the drag, Z on a big page)
is a case of it. The change, in steps that each keep parity:

1. **The hook decides from a snapshot.** The engine publishes, after each key and event, what
   decides whether a key is taken: the key owner (virtual cursor, forms mode, a menu, one of
   Okur's dialogs, a layer, Keyboard Help), the bound chords of that owner, and the pass-through
   keys of the application in front. The hook reads it without a lock (an `ArcSwap` or a
   seqlock of a small struct), takes or passes the key, and sends it on.
2. **One engine thread.** The keys taken go, with their time, to a thread that owns the engine
   without a mutex and serves keys before events, marks and page builds (a priority queue of
   two lanes). The event loop's handlers become messages to it.
3. **What a key needs from the screen is asked of workers** (the clipboard, text reads, the
   window's lines), as the text after an edit key already is: the engine says what it has and
   finishes the command when the answer comes.

Step 1 needs every place that decides "taken or passed" in `handle_key_event` to be a function
of the snapshot; `routing.rs` already gathers most of them. The risk is a key the snapshot
calls taken that the engine would have passed, or the reverse, while a mode changes between
two keys; the snapshot is replaced before the key that changed it returns, so a following key
sees the new mode, as now.
