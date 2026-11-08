# Review of the Linux stack and the cross-platform structure (2026-11-08)

Two read-only reviews, run while the Linux lab ran an 80-scenario rerun (the owner, 2026-11-08:
"run a reviewer agent instead of just waiting"). One reviewed the Linux driver and model
(`okur-linux`, `okur-linux-model`, the recent `xa11y-linux` changes, the lab runner) for latency,
naive implementations, edge cases and quality. The other reviewed the crate structure for a third
platform, building on [`crate-structure-proposal.md`](crate-structure-proposal.md). Each finding
acted on was checked against the code first. Both reviewers worked at 3d84b3a4, and their
file:line references are to that commit.

## Linux stack: done

| Finding | What was wrong | Commit |
|---|---|---|
| 1. Every native focus grabbed the X server | `atspi_parent` and `atspi_ancestor` built a full `ElementData` at each level. For a top-level frame that meant the window manager's facts, read under an X server grab. A page's document had its whole text fetched on each Tab. The handle caches never shrank. | 70ab20b1 (`AtspiObject`: two calls a level) |
| 2. Any application's label re-read the focus | `NameChanged` from a clock or a browser tab re-read the focused control in full | 79eb9a8b (`in_focused_application`) |
| 3a. GTK's twin focus read in full | GTK 4 sends two focus events per move, and both were read before the repeat check | 35ac0be5 (`is_twin`, 100 ms) |
| 4. No timeout on AT-SPI calls | One application that did not answer blocked every event, and quitting with them | de593755 (1 s per call) |
| 5. Page actions inside the hook's callback | GrabFocus/DoAction blocked the frozen key. The watchdog then replayed it and activated the element twice. | 18f5d110 (a worker thread) |
| 7. Every signal's object read before mapping | 5 to 7 calls back into the application for signals no one takes. A page load sends thousands. | 27ee842b (kinds first, `subscribe_desktop_kinds`, `is_heard`) |
| 8. `shown_page`'s frame check never ran | `get_parent` on an event's element (handle cache), so every frame's document that loaded passed as the page | 70ab20b1 |
| 10. Returning to a window was silent | Only another window's activation forgot the focus | 79eb9a8b (`window_deactivated`) |
| 12. A re-read kept the event's old snapshot | A button renamed in place, or a check box unchecked, read as before | 43725113 |
| 13. SSML with Okur's markers | U+001D/U+001E are illegal in XML; SAPI and OneCore strip them | 79eb9a8b |
| 15. Silent failures (part) | The bus disconnecting ended the event thread without a word | d290ae2c |
| 17. Small slips | The hook grabbed keys on its own wake window | 35c1d765 |
| 18. The lab runner | Unknown modifiers dropped, keys without keycodes "passed", and nothing cleaned up on an exception | df89c0c4 |
| 19. The desktop not told a screen reader runs | `org.a11y.Status` was never set, and Qt, Firefox and Chromium bridges depend on it | 45202034 (`A11yStatus`) |
| Structure 4 (part) | Linux never set `FocusEvent::app`, so per-application settings did nothing; "EDIT" was matched in general code | 514aabf5, d4e88df9 |

**Also found during the rerun:** the example `okur_linux` set its options on `Config.settings`, which reaches only the voice. The engine reads the settings layers, so every Linux web run read pages at Medium (3b2aacec).

**Gate:** clippy now checks the Linux crates for `x86_64-unknown-linux-gnu` on the dev box when that target is installed (ce08d0ac). Before, nothing but the lab could check them.

## Linux stack: open, in order

1. **(6) Pages are read whole, synchronously, on the event thread.** At about 7 sequential calls per object, a 5,000-object page is about 15 s (estimated), and the page's own focus is said only after it. Plan: a page-reader thread, cancelled when the focus leaves; the focus sent first; pipelined or Cache-interface reads; pages cached by document; a truncation flag.
2. **(3c–e)** Drop a focus whose fresh node no longer has FOCUSED (fast Tabs). Get "n of m" from posinset/setsize or GetIndexInParent instead of reading the parent whole. Pipeline the per-node calls. First add `ms` and a call count to "native focus read" for real numbers.
3. **(11) Make the model's read plan explicit.** The read plan (which ancestors whole, `CONTROL_LIMIT`) moves into okur-linux-model. A tree cut at 200 objects is marked. Peers are matched by object path, not by name and states.
4. **(9)** Page actions find objects by index into a list a later read replaces; they need a generation stamp or (bus, path) on each node.
5. **(14)** CapsLock as the Okur key is never grabbed, so X toggles Caps Lock.
6. **(15, rest)** Warn when grabs fail (a leftover Okur holds the keys). Mark a read with per-node errors as partial.
7. **(16)** One walk of references up to the frame, shared by `document_of` and `native_focus`. The 40-level page depth and the 12-level window depth are fixed caps.
8. **Plausible, need the lab:**
   - (20) Windows without AT-SPI (xterm, Java) are invisible: watch `_NET_ACTIVE_WINDOW`.
   - (21) Focus events from inactive windows.
   - (22) Lists that move only their active descendant (GTK 3, Qt).
   - (23) Window managers that nest the client deeper than two levels.
   - (24) Key repeat, XKB groups and AltGr, keys during a consumed key's hold, keys while a menu holds the keyboard.

## Cross-platform structure: the proposal

**Where the accepted plan stands:**
- Steps 0–4 and 5a are done. 5b, 6 and parts of 7 remain.
- The ports (KeyboardHook, EventSource, SynthesizerFactory, UiSpawner) need no change for macOS. The gaps are around them.

Ranked:

1. **A replayable Linux event model.** Windows captures a serializable `TraceEvent`, and one pure `EventConverter` serves live runs and replay. Linux's decisions ran inside the driver's thread, untestable off Linux.
   - This stretch began it: `focus_track` holds the focus rules and the event-kind decisions, tested on every OS (2f71e086, 3d84b3a4).
   - What remains:
     - A serde record of what the driver read for each event, and a pure converter over it.
     - Traces kept beside each live run.
     - `okur-lab live-replay linux`, so Linux runs can be debugged and ratcheted on any OS.
   - A macOS driver should start in this shape.
2. **An Okur-owned `WebNode` instead of IA2 as the web pivot.**
   - A Linux page is converted to fabricated MSAA/IA2 objects, then through Windows' `ia2_to_aom`. That is why okur-linux-model depends on okur-windows-model and xa11y-windows.
   - Most of `ia2_to_aom` reads Chromium's object attributes, and only the role numbers and state bits are IA2's.
   - A neutral `WebNode` and `web_to_aom` would take `ia2_to_web`, `atspi_to_web` and later `ax_to_web`.
   - Step (a), `ia2_to_aom = web_to_aom ∘ ia2_to_web`, is checkable with parity-snapshot (identical).
3. **One `okur` program for every OS.**
   - Linux runs from an example with its own argument parser, which misread `--no-speech 3600` and silenced a rerun. It also dropped `--web-verbosity` (above).
   - `run.rs` would become platform-neutral, with a `windows.rs` and a `linux.rs` (later a shared `unix.rs` and `macos.rs`) for each OS's start.
4. **Neutral identity in okur-core.** An opaque `window_id` in place of the top-level window class; facts in place of class literals (`plain_edit` done); `app` set on every platform (done for Linux); the structure rule R5 widened.
5. **A shared neutral crate** for `SnapshotProvenance` (now re-exported by okur-linux-model), the xa11y `ElementData`→`FocusEvent` mapping, `WebNode` and the event-model trait. Plus a rule that one platform's model never depends on another's.
6. **Control context over AOM nodes** (window, container, place, groups, level), written once for every platform's native focus. This goes with the macOS model, not before.

### Decisions for the owner

- **D1:** a new neutral crate (`okur-model`), or a module inside okur-core?
  - Recommendation: the crate. okur-core should not take xa11y-core and serde-shaped platform data.
- **D2:** should Linux live-run traces go into the corpus with a replay baseline (fixture pages only)?
- **D3:** should Linux get a real `okur` binary (Tier 2, no installer), replacing the example?
- **D4:** should macOS build on the vendored xa11y-macos, extended in place with raw tree reads, as Linux's `atspi_tree` and `atspi_parent` extended xa11y-linux?
  - Recommendation: yes.
- **D5:** settle the open questions T5 and T10 before the macOS driver: the Okur key on Mac keyboards (no Insert), and Command in `Modifiers`.
