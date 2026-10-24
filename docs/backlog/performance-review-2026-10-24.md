# Performance and hardening review, 2026-10-24

Fixed findings were removed on 2026-10-24 and are in git history; the open ones keep their
numbers.

A read-only review of the whole codebase at `92bebb44`, made at the owner's request by a
reviewer agent, as the reviews of 2026-10-19
([responsiveness](responsiveness-review-2026-10-19.md) where it exists), 2026-10-20
([`performance-review-2026-10-20.md`](performance-review-2026-10-20.md)) and 2026-10-23 were.
It built the commit in a scratch copy and measured with benchmarks of its own outside the
repository. Nothing ran live.

Statuses are this session's: **fixed** with the commit, **open**, or **kept** with the reason.

## Findings, by user impact

| # | Kind | Finding | Status |
|---|---|---|---|
| 1 | Stall | On a site with a custom label or a Flexible Web rule, every page refresh rebuilds the whole page under the engine, inside the keyboard hook: `apply_pending_page` → `prepared()` → `labeled()`, `with_customizations()` (`engine.rs:619`, `labels.rs:31`, `flexible_web.rs:403`). Measured on 25,032 nodes: the first key after a refresh 0.6-0.9 ms without a label, 89-102 ms with one in release, 128-147 ms in the dev build, near the hook's 150 ms limit. Fix: apply labels and rules off the engine, where the reading marks are applied, before `from_aom_owned`. | open |
| 3 | Stall | A Chromium live region is read whole through IA2 on the WinEvent thread for every change (`chromium_live_region`, `read_node` without a node cap): a long `role=log` chat holds the thread every focus waits on. Earlier P4 step 4. | open |
| 4 | Performance, quality | A web field's read after each key walks all its attribute runs, up to 256 `get_attributes` calls (`ia2.rs:890-958`); past the 256th run the caret's colors and misspellings are lost. Earlier P9, P11 step 2. Fix: the caret's line's runs only. | open |
| 6 | Quality | Page changes are dropped while another event is read: `on_page_change` uses `STATE.try_borrow_mut()` and returns on failure, and `record_at` holds `STATE` through its reads, during whose SendMessage waits Windows delivers WinEvents. | open |
| 9a | CPU | `give_way_to_keys` spins `yield_now` up to 200 ms on a tokio worker while a key holds the engine. Earlier T4. | open |
| 9b | Memory | The okur-ui writer queue is unbounded: a hung okur-ui piles up viewer texts. | open |
| 9c | Performance | Okur cursor keys in a text field clone the field's whole text for each mouse or caret move. | open |

## The earlier reviews' open items

- **Responsiveness 2026-10-19:**
  - 2c is fine.
  - 13 is partly done: each poll still reads the whole document and the caret's prefix.
  - 16 is open: MSAA and IA2 calls have no timeout and there is no `CoCancelCall` watchdog.
  - 17 is open.
- **Performance 2026-10-20:**
  - P2 steps 2-3 are open: no thread priority, and no written "nothing O(page) in a key" rule.
  - P3 step 2 is open: acceptable except on labelled sites.
  - P4 steps 4-6 are open (finding 3).
  - P5 steps 3 and 5 are open.
  - P6 step 2, P7, P11 step 2, P12, P13 and P15 are open.
  - P14 was not re-measured.
- **Performance 2026-10-23:**
  - T1 still holds, except for finding 1. A sweep from the middle of 25,032 nodes had nothing over 6.4 ms when warm.
  - T4 and T5 are open; T11, T13, T14, T17 and T18 are unchanged, as decided.
  - C2's remainder is finding 1. C10 and C17 are fine.
  - P4, P6, P9, P11 and P13 are open. P21 and P22 were not re-checked.

## Checked and found fine

- The menu mask key: SendInput from the hook is legal and cheap, and its key goes through the
  normal paths.
- OCR's inputs: a zero-sized capture is refused, and sizes are bounded by the recognizer's
  largest side.
- The speech history's cuts, and Say All's parts.
- Every `SendMessageTimeoutW` has a 100-500 ms limit.
- The WinEvent re-entrancy fix holds (`in_turn`), and the deferred queue is bounded.
- The WinEvent thread is in the multithreaded apartment, so the `MtaElement` `Send` impl is
  sound.
