# Responsiveness review (2026-10-19)

Fixed findings were removed on 2026-10-24 and are in git history; the others keep their numbers.

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

| Finding | Status | Commit | Measured |
|---|---|---|---|
| 2c `node()` allocates | not done: 0.6 µs; recursion could overflow the hook's stack | | |
| 7 UIA client timeouts | not changed: UIA's defaults are bounded (2 s, 20 s) | | |
| 9 speech log writes | not changed: `--speech-log` only, a buffered write of microseconds | | |
| 13 UIA text polling | open: needs live checks (Notepad, Word) | | |
| 16 cross-process calls without timeouts | open: an NVDA-style watchdog needs live checks | | |
| 17 smaller costs | open: `editing::chars`, linearizer allocations | | |

## Findings, most impactful first

2. **The hook pays for applying a pending page.** Measured.
   - Where: `engine.rs` `apply_pending_page` at the start of every key.
   - `node()` allocates a path `Vec` on every call.
   - Proposal: a `node()` that does not allocate.
13. **UIA text fields (Notepad, Word) re-read the whole document per poll.** The
    mechanism was read; the size is suspected.
    - Where: `text_context.rs` `read_uia`, every 15 ms until the field changes.
    - Proposal: poll the selection range cheaply; read in full once, when it changed.
16. **Cross-process calls with no timeout.** Suspected.
    - Where: MSAA and IA2 calls on the WinEvent thread and the page and actions workers.
    - Proposals:
      - an NVDA-style watchdog: `CoEnableCallCancellation`, then `CoCancelCall` after 500 ms;
      - one document lookup per batch of parts;
      - no `accLocation` in `window_bars`.
17. **Smaller costs.**
    - `editing::chars()` collects the whole field as `Vec<char>`, 1-3 times per edit speech.
    - The linearizer allocates a `String` per word and copies each paragraph.

## Checked and fine

- The hook callback itself: microseconds.
- Logging: the debug log goes through a writer thread.
- Speech: on its own thread; OneCore cancels at once.
- Chromium field polling: fine as it is.
- The tracer's event filtering and throttling.
- okur-ui IPC: never waits.
- Browse-mode arrows and quick navigation: under 0.25 ms.
- xa11y's UIA backend: not on the Windows live path.
