# Responsiveness review (2026-10-19)

A review of the whole codebase for stalls and slow paths, asked for by the owner. It was done by
a review agent that changed nothing and ran nothing live. Its numbers come from a scratch
benchmark that drove the real `ScreenReaderEngine` and `VirtualBuffer` with a synthetic,
WhatsApp-like page: 30 wrapper levels, an IA2 uniqueID and three attributes on every node.
- At 5,000 rows the page has 25,032 nodes and 15,003 lines; at 1,000 rows, 5,032 nodes.
- Times are wall-clock on the dev box, release unless marked. Noise between runs reached 3x, so
  times are ranges.

Line numbers are as of `aa9c87a` and may move. Each finding says whether it was measured or read.
Every change must keep `okur parity` byte-identical: run
`skills/jaws-reference/scripts/parity-snapshot.sh` before and after, then `diff -r`.

Already done before this review, and not to be redone:
- `4a29343` nothing slow under the engine lock;
- `eaa13f2` a linear buffer build;
- `7e21f63` and `f743f0f` UIA sibling reads;
- `fda92b3` captures once live regions settle;
- `b9a1910` a quick UIA read, then IA2;
- `bc9e1f2` and `37d986e` edit-key polling;
- `cbcd429` partial page reads;
- `97dd907` the speech thread.

## Status (2026-10-19)

Measured with the review's benchmark, release, on the 25,032-node page. After every commit,
`parity-snapshot.sh` gave output identical to before.

| Finding | Status | Commit | Measured |
|---|---|---|---|
| Debug-build multiplier | done: optimized dev profile | `8739b9a` | the whole three-layer parity run takes 5 s |
| 1a parts one by one | done: one event, one rebuild | `18bf4b7` | 8 parts: 550 ms → 87 ms |
| 1b page copies | done: owned builds | `1c49c52` | 1 part: 136-532 ms → 70 ms |
| 1c rebuild under the lock | done: built outside, installed if the page is the same | `4478d14` | engine held 0.012 ms (was 87 ms) |
| 2 pending page in the hook | done: indexes, old buffers freed on a thread | `1c49c52` | 37-116 ms → 0.9 ms |
| 2c `node()` allocates | not done: 0.6 µs; recursion could overflow the hook's stack | | |
| 3 Tab O(stops × lines) | done: binary searches, nearest stop | `8739b9a` | 35-48 ms → 0.0 ms |
| 4 page focus scans | done: object and DOM id indexes | `1c49c52` | 27 ms → 0.4 ms |
| 5 settle drops requests | done: the strongest is served | `bceeb4d` | test |
| 6 superseded speech, two purges | done: batches, 3 ms stop grace | `bceeb4d` | test |
| 7 `cmd /C ver` per capture | done: once per process | `bceeb4d` | |
| 7 UIA client timeouts | not changed: UIA's defaults are bounded (2 s, 20 s) | | |
| 8 hook lock fairness | done: keys go first | `e13a2ee` | test: under 100 ms while events stream |
| 9 voice list, clipboard in the hook | done | `e13a2ee` | |
| 9 speech log writes | not changed: `--speech-log` only, a buffered write of microseconds | | |
| 10 same-page builds on the event loop | done: built on a thread, numbered | `9e8b9df` | test |
| 11 rebuild before speaking | done: speak first | `bceeb4d` | |
| 12 list clone on Enter | done: borrowed | `8739b9a` | 50.6 ms → 0.1 ms |
| 13 UIA text polling | open: needs live checks (Notepad, Word) | | |
| 14 voice re-applied | done: only what changed | `bceeb4d` | |
| 15 Say All in one utterance | done: parts of whole lines | this commit | test |
| 16 cross-process calls without timeouts | open: an NVDA-style watchdog needs live checks | | |
| 17 smaller costs | open: `editing::chars`, linearizer allocations, the actions worker's queue, okur-ui's list | | |

## Multiplier: live runs use a debug build

- `live-daemon-guide.md` runs `cargo run --bin okur -- run`, and `Cargo.toml` has no
  `[profile.dev]`, so live runs are at opt-level 0.
- Debug runs 3x to 10x slower on every path below:

  | Path | Release | Debug |
  |---|---|---|
  | page focus by uniqueID | 27 ms | 210 ms |
  | `with_node_changed` | 116 ms | 752 ms |
  | Tab from the virtual cursor | 35-48 ms | 216-230 ms |
  | `pronounce::spoken` over 430 KB | 24 ms | 565 ms |

- Proposal: `[profile.dev] opt-level = 1` and `[profile.dev.package."*"] opt-level = 3`, or
  live use from `--release`.

## Findings, most impactful first

1. **A partial page read still rebuilds the whole page under the engine lock, once per part.**
   Measured.
   - Where: `page.rs` `page_part_loaded` → `buffer.rs` `with_subtree_replaced`, which clones
     the page; `from_aom` clones it again and re-linearizes it. The platform sends up to 8
     `PagePartLoaded`s, one per part.
   - Cost: one part takes 20-43 ms at 5-10k nodes and 136-532 ms at 25k. Eight parts at 25k take
     1.8-2.7 s (4.8 s in debug), while the hook waits at most 150 ms before passing the key on.
   - Proposals:
     - batch the parts into one splice and one rebuild;
     - build from an owned root (`from_aom_owned`), with no clone;
     - build outside the lock, with a generation check;
     - later, splice incrementally, guarded by a test that it equals a full rebuild.
2. **The hook pays for applying a pending page.** Measured.
   - Where: `engine.rs` `apply_pending_page` at the start of every key.
   - Cost at 25k nodes: 37-116 ms for the first key after a re-capture.
     - Dropping the old buffer: 18-36 ms.
     - `same_object_line`: 12 ms.
     - `nodes().find(uniqueID)`: 3.5 ms.
     - `node()` allocates a path `Vec` on every call.
   - Proposals:
     - drop old buffers off the lock (a dropper thread);
     - build uniqueID → node and node → first line indexes during linearization;
     - a `node()` that does not allocate.
3. **Tab in browse mode is O(tab stops × lines) inside the hook.** Measured.
   - Where: `tab_from_virtual_cursor` calls the linear `line_at_offset` for each tab stop.
   - Cost: 35-48 ms with 5,000 stops and 15k lines (216-230 ms in debug).
   - Proposals:
     - binary search in `line_at_offset`, `next_line_from`, `prior_line_from`,
       `word_at_offset`, `paragraph_at_offset`, the `next_*` and `prior_*` functions and
       `node_at_offset` (26 µs → 0.03 µs at 15k lines);
     - search the tab stops around the cursor only.
4. **Every page focus, change and live region scans the whole buffer.** Measured.
   - The scans:
     - `page_node_for` sorts all ids and calls `node()` for each;
     - `region_old_lines` does the same by DOM id;
     - `entered_containers`, `region_markers` and `in_widget_item` walk the whole tree
       through `ancestors_of`;
     - `shows` and `start_of` scan every span.
   - Cost at 25k nodes: `page_node_for` 21 ms; a page focus event 27 ms (210 ms in debug).
   - Proposals:
     - uniqueID and DOM id indexes, keeping the first match in document order;
     - `buffer.ancestors(id)` in place of `ancestors_of` (0.86 ms → 0.009 ms).
5. **The page worker's settle loop swallows Focus, Changes and Refine requests.** Read.
   - Where: `events/windows.rs` `settle` drops any request while a Refresh settles (up to 3 s).
   - Effect: a focus the buffer lacks waits; a lost Refine leaves the page without IA2
     uniqueIDs, so later splices fail and each change is a whole-page read.
   - Proposal: settle absorbs only Refresh; the strongest request wins.
6. **The speech thread runs superseded requests one by one, and with SAPI each key purges
   twice.** The mechanism was read; the size is suspected.
   - Where: `tts/thread.rs` runs every queued request; each key calls `stop()`, then an
     interrupting speak; both purge (750 ms each, measured on old-pc in `97dd907`).
   - Effect: when a key repeats, speech lags behind the keys.
   - Proposal: drain the queue and skip what a later interrupting request supersedes; keep
     Configure in order.
7. **Every live page capture spawns `cmd /C ver`, and its UIA client has no timeouts.**
   Measured.
   - Where: `uia_snapshot.rs` and `ia2_snapshot.rs` `os_version`, on the WinEvent thread.
   - Cost: 21-28 ms per capture, 224 ms cold.
   - Also: a new `CUIAutomation8` per capture with UIA's default timeouts, and a `FindFirst`
     over all of the browser's descendants for the document.
   - Proposals:
     - cache `os_version`;
     - one UIA client per thread with bounded timeouts;
     - take the document from the focus chain.
8. **The hook polls the engine with `try_lock` and 1 ms sleeps.** Suspected.
   - Where: `main.rs`. The event loop re-locks within microseconds, so during event streams the
     hook can wait out its 150 ms and pass the key on.
   - Proposal: `parking_lot`'s `try_lock_for` (fair hand-off), or a key-waiting flag the event
     loop yields to.
9. **Blocking calls that can run on the hook thread.** Read.
   - `open_settings` → `voices()` locks the TTS manager, which waits behind a SAPI purge, then
     enumerates voice tokens. Fix: a voice list cached on the speech thread.
   - The Virtual Viewer's copy sets the clipboard on the hook thread; a hung clipboard owner
     blocks it. Fix: move it to the actions worker.
   - `SpeechLog::record` writes the file on the calling thread with `--speech-log`. Fix: a
     writer thread.
10. **The event loop builds same-page refreshes inline.** Measured.
    - Where: `main.rs` `DocumentLoaded` runs `from_aom(root)`, a clone plus a build, before the
      events queued behind it: 69-275 ms at 25k nodes.
    - Proposal: build from the owned root, on a builder thread, as an internal "page built"
      event.
11. **Space on a check box rebuilds the whole buffer before speaking.** Measured.
    - Where: `page_change` and the focus path's stale-node rebuild through `with_node_changed`:
      116 ms (752 ms in debug).
    - Proposals:
      - speak first;
      - update the one line in place, guarded by a test that it equals a full rebuild.
12. **Whole-subtree clones on keys.** Measured.
    - Where: `node_under_cursor` returns `.cloned()`. Enter on a list of 5,000 items clones it:
      25-30 ms.
    - Proposal: return `&AomNode`.
13. **UIA text fields (Notepad, Word) re-read the whole document five times per poll.** The
    mechanism was read; the size is suspected.
    - Where: `text_context.rs` `read_uia`, every 15 ms until the field changes.
    - Proposal: poll the selection range cheaply; read in full once, when it changed.
14. **Any voice-settings change re-applies the voice** (SAPI token enumeration, `SetVoice`), even
    a punctuation level change. Read. Proposal: apply only what changed.
15. **Say All is one utterance of the whole rest of the page.** Measured: `spoken` takes
    24-565 ms before the first word. Proposal: pronounce and speak it in chunks on the speech
    thread.
16. **Cross-process calls with no timeout.** Suspected.
    - Where: MSAA and IA2 calls on the WinEvent thread and the page and actions workers.
    - Proposals:
      - an NVDA-style watchdog: `CoEnableCallCancellation`, then `CoCancelCall` after 500 ms;
      - one document lookup per batch of parts;
      - no `accLocation` in `window_bars`.
17. **Smaller costs.**
    - `editing::chars()` collects the whole field as `Vec<char>`, 1-3 times per edit speech.
    - The linearizer allocates a `String` per word and copies each paragraph.
    - The actions worker's long reads delay the edit-key text read queued behind them.
    - okur-ui inserts list items one by one with redraw on.

## Checked and fine

- The hook callback itself: microseconds.
- Logging: the debug log goes through a writer thread.
- Speech: on its own thread; OneCore cancels at once.
- Chromium field polling: fine as it is.
- The tracer's event filtering and throttling.
- okur-ui IPC: never waits.
- Browse-mode arrows and quick navigation: under 0.25 ms.
- xa11y's UIA backend: not on the Windows live path.
