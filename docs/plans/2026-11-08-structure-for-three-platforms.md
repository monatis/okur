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

## 2. An Okur-owned web node as the pivot (next)

A page today:
- Windows: IA2 objects → `ia2_to_aom` (okur-windows-model/src/ia2_snapshot.rs, about 560 lines with `map_role`).
- Linux: AT-SPI → `to_ia2` (fabricated MSAA/IA2 role numbers and state bits) → `ia2_to_aom`.

`convert` uses the IA2 numbers throughout. It distinguishes:
- STATICTEXT text from other text;
- a DOCUMENT inside a group (an article);
- GROUPING, COLUMNHEADER, ROWHEADER;
- IA2's SECTION, CONTENT_DELETION, CONTENT_INSERTION, MARK and TOGGLE_BUTTON;
- MSAA's CHECKED, MIXED, PRESSED, SELECTED, EXPANDED, COLLAPSED, FOCUSED, FOCUSABLE, UNAVAILABLE, INVISIBLE, PROTECTED, READONLY, TRAVERSED;
- IA2's MODAL and REQUIRED.

Design:
- **`okur_model::web`.** `WebNode` has:
  - `role: WebRole`, one value per distinction `map_role` and `convert` make. Built as the union of the MSAA and IA2 roles they test, named for what the element is: `StaticText`, `Article`, `Grouping`, `ColumnHeader`, `ToggleButton`, `Section`, `Deletion`, …
  - `states: WebStates`, named flags for the bits above;
  - name, value, description, default action, value range, unique id, attributes (Chromium's object attributes: `tag`, `display`, `xml-roles`, `level`, …), group position, text, text runs, relations, children.
- **`web_to_aom(&WebNode) -> AomNode`**: today's `convert` and `map_role`, reading `WebRole` and `WebStates` instead of numbers.
- **`ia2_to_web(&Ia2Node) -> WebNode`** in okur-windows-model: the numbers' meaning, in one place.
- **(a)** `ia2_to_aom = web_to_aom ∘ ia2_to_web`. Verified by P1 identical, over 278 IA2 snapshots.
- **(b)** `atspi_to_web(&AtspiNode) -> WebNode` in okur-linux-model, in place of `to_ia2`.
  - A test that it gives what `ia2_to_web(to_ia2(..))` gave on okur-linux-model's fixtures.
  - Then `to_ia2` goes, along with okur-linux-model's dependencies on okur-windows-model and xa11y-windows.
  - Verified by a lab run of the web scenarios and `parity --api atspi` on the captured snapshots.
- **(c)** The rule R9 in `check-crate-layers.ps1`: a platform's model takes no other platform's model or backend.

## 1. A replayable Linux event model

- **What the driver should do:** only capture. For each event it writes an `AtspiRecord` (serde) in okur-linux-model. A record holds:
  - the event's kind, application, object and window;
  - the target's xa11y snapshot;
  - what the read gave: the focus's subtree and ancestors, the page, a re-read.
- **The conversion:** a pure `Converter` turns records into `AccessibilityEvent`s, as Windows' `EventConverter` turns `TraceEvent`s. `FocusTrack` lives in it.
- **The capture:** the driver asks the model's predicates what to read (`is_heard`, `is_twin`, `in_focused_application`).
- **The lab:** the runner keeps each live run's records beside it (`corpus/live/linux/<id>.trace.jsonl`, fixture pages only).
- **Replay:** `okur-lab live-replay linux` replays them against JAWS's evidence and against the run's own speech, with a baseline.
- **A trait for both:** an `EventModel` trait in okur-model, which Windows' `EventConverter` implements too.
- **Verification:** the model's tests; the lab's live-baseline not regressed; the replay equal to the live run.

## 3. One `okur` program for every OS

- `crates/okur/src/run.rs` becomes platform-neutral: options, settings folder and layers, speech log, fixture, `Config`, `host::run`.
- `windows.rs` keeps Windows' start: single instance, control pipe, session end, screen reader flag, elevation.
- `linux.rs` builds the Platform from okur-linux.
- The Linux lab's scripts run `okur run --engine no-speech --settings …`. Then the example `okur_linux` goes, and with it its second argument parser, which this stretch found twice wrong (the seconds, the settings layer).
- Verified by P2, the Linux target's clippy, and a live start on old-pc (P4).

## 4. Neutral identity (rest)

- An opaque `FocusEvent::window_id` each converter fills, in place of comparing top-level window classes for a window change.
- The structure rule R5 widened to any literal compared with a class field.

## 6. Control context over AOM nodes

The step from "a focused control and its ancestors" to window, container, place, groups and tree level is written twice: Windows over UIA control types, Linux over AT-SPI role names. `okur_model::control` will hold it, over shallow AOM nodes. This lands with the macOS model:
- Linux adopts it.
- macOS is built on it.
- Windows adopts it piece by piece, where P1 proves each piece equal.

## Meanwhile, on Linux

The code review's open items, in their order: [`../references/linux-and-structure-review-2026-11-08.md`](../references/linux-and-structure-review-2026-11-08.md). First the page read off the event thread, and the focus costs, which the lab's debug lines now measure.
