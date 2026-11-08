# Plan: a structure for three platforms (2026-11-08)

The owner, 2026-11-08, on the structure review in
[`../references/linux-and-structure-review-2026-11-08.md`](../references/linux-and-structure-review-2026-11-08.md):
"yes, go for those recommendations".
- D1: a neutral `okur-model` crate.
- D4: macOS builds on the vendored xa11y-macos, extended in place.
- The ranked proposal is the order of work. It includes:
  - Linux traces in the corpus with a replay baseline (D2);
  - one `okur` program for every OS, with a real Linux binary (D3);
  - the Mac's Okur key settled before the macOS driver (D5).

Rules for every step, as the accepted [`../references/crate-structure-proposal.md`](../references/crate-structure-proposal.md) sets them:
- `skills/jaws-reference/scripts/parity-snapshot.sh` before and after must diff empty.
- The tests, the gate (the Linux target's clippy included) and the layer checks pass.
- No `pub use` alias left behind.
- A step that changes Linux behavior is proved by a lab run, with `okur-lab live-baseline linux` not regressed.

## Done

- **5a. `okur-model`** (ed7c39e5).
  - It holds `SnapshotProvenance` and an xa11y element as the engine's focus event.
  - The re-export from okur-windows-model is gone. AGENTS.md §6 names the crate.
  - Parity identical.
- **4 (part):**
  - `FocusEvent::plain_edit` replaces the engine's "EDIT" match (d4e88df9).
  - Linux sets `FocusEvent::app` (514aabf5).
- **1 (part):** okur-linux-model's `focus_track` holds the event loop's rules and its event-kind decisions, tested on every OS (2f71e086, 3d84b3a4, 79eb9a8b, 27ee842b, 35ac0be5).
- **2. The web pivot** (4d3c8a42, 8cc1ef08, 3c555f36): see the handoff of 2026-11-08 (2).
- **1. A replayable Linux event model:**
  - The driver only reads. okur-linux-model's `Converter` decides what the engine is told from an `AtspiRecord` of what was read (f56f2377).
  - The driver writes each record to a trace when given one (`okur run --trace`). The lab's runner keeps it beside the run, with the keys it pressed. `okur-lab live-replay linux` replays the records against the run's speech and JAWS's evidence (f0b3a66d). Records time their reads (f0746d17).
  - okur-model's `EventModel` is the trait both platforms' converters implement (86df02bc).
  - `--pages-after-focus` tries a page-reader thread on the runs before the driver changes (236db52e).
- **4. Neutral identity:**
  - `PlatformIds::window_id`, the top-level window itself, opaque; the engine's window change compares `window_key()`, the id where the platform names one, else the class (68274780). Linux names each window by its frame's object. Windows names none yet: PARITY IDENTICAL.
  - The structure rule R5 widened to every class field and automation id, compared by `!=`, a match's alternatives or a string test too.
- **3. One `okur` program for every OS** (340f3b66):
  - `run.rs` is neutral. `windows.rs` keeps Windows' start, unchanged in order. `linux.rs` builds okur-linux's platform and stops on a terminate signal.
  - The example `okur_linux` is gone. The lab runs `okur run`.
  - Checked live on old-pc's Windows: it starts, `okur quit` is answered through the control pipe, and it ends.

Steps 1 to 4 are done (above); their designs are in git's history of this file (at
d469f746).


## 6. Control context over AOM nodes

The step from "a focused control and its ancestors" to window, container, place, groups and tree level is written twice: Windows over UIA control types, Linux over AT-SPI role names. `okur_model::control` will hold it, over shallow AOM nodes. This lands with the macOS model:
- Linux adopts it.
- macOS is built on it.
- Windows adopts it piece by piece, where P1 proves each piece equal.

## Meanwhile, on Linux

The code review's open items, in their order: [`../references/linux-and-structure-review-2026-11-08.md`](../references/linux-and-structure-review-2026-11-08.md). First the page read off the event thread, and the focus costs, which the lab's debug lines now measure.
