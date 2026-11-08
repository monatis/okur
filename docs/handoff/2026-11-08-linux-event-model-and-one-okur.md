# Handoff 2026-11-08 (3): Linux's event model replayed, one `okur` for every OS, pages off the event thread

After [`2026-11-08-reviews-okur-model-web-pivot.md`](2026-11-08-reviews-okur-model-web-pivot.md).

**Plans:**
- [`../plans/2026-11-08-structure-for-three-platforms.md`](../plans/2026-11-08-structure-for-three-platforms.md) (the owner's "yes, go for those recommendations"). Steps 1 to 4 are now done; step 6 goes with the macOS model.
- The Linux review's open items: [`../references/linux-and-structure-review-2026-11-08.md`](../references/linux-and-structure-review-2026-11-08.md).

## What was done

**The structure plan, steps 1, 3 and 4:**
- **1, a replayable Linux event model.**
  - The driver only reads. For each AT-SPI event it builds an `AtspiRecord` of what it read, and okur-linux-model's `Converter` decides what the engine is told, tested on every OS (f56f2377).
  - `okur run --trace` writes the records as JSON lines. The lab's runner keeps them beside each run, with the keys it pressed (f0b3a66d); records time their reads (f0746d17).
  - `okur-lab live-replay linux` replays each run from its trace, against the run's own speech and JAWS's evidence. `--only` replays chosen runs, with the engine's debug log under RUST_LOG.
  - okur-model's `EventModel` is the one trait both platforms' converters implement (86df02bc).
- **3, one `okur` for every OS (340f3b66).**
  - `run.rs` is neutral; `windows.rs` keeps Windows' start in its old order; `linux.rs` builds okur-linux's platform and stops on a terminate signal.
  - The example `okur_linux` is gone; the lab runs `okur run`.
  - Windows' `okur.exe` was checked live on old-pc (below).
- **4, neutral identity.**
  - `PlatformIds::window_id`: the engine's window change compares the window itself where the platform names it, else its class (68274780). Linux names each window by its frame's object; Windows names none yet: PARITY IDENTICAL.
  - The structure rule R5 now counts every class field and automation id compared with a literal, by any comparison (9e2659b9). Its count stays 0.

**The Linux review:**
- **(6) Pages off the event thread (8649f7c4).** A page reader with its own thread and bus connection takes the latest page asked of it. The engine gets the focus at once and the page when it comes.
  - Pages are read in parallel, 3 to 3.5 times as fast as in order, with the same tree (482847b8, from the probe below).
  - Before building it, the replay with `--pages-after-focus` said what the runs said.
- **(3c)** A focus whose fresh read no longer has FOCUSED is not said (8649f7c4).
- **(3d)** A list's item or a column view's row takes its place from GTK's `posinset` and `setsize`, and its list is read alone (ecb4e04a). A tree's rows keep the old way: there `posinset` numbers the flattened list, and GTK's `level` is no depth (2408656b).
- **(16)** A native focus's ancestors are walked once (e7d611dd).
- **(11)** The read plan is in okur-linux-model (c4cdc699), and a place is found by object path (0d360045).
- **(15)** A key grab that another program holds is logged (4d58202f).

**The Linux runs' settings (ce95271b).** The runner gave Okur its defaults. The recordings often had other settings: in 201 Auto Forms Mode was off, but Okur's default turned forms mode on at the "Full name" edit, and every Down after it said nothing.
- `okur-lab reference-settings --evidence` writes the settings JAWS recorded a scenario with, as parity gives them to the engine. The runner gives Okur that folder.
- `live-replay --recorded-settings` estimates what that gives on runs made before: 773/829 on the runs at f0b3a66d, against 735.

**Lab tooling:**
- `probe-read-speed.sh`.
- `fetch-runs.sh` pulls one archive (915670c5): two pulls a run had taken half an hour.

## Figures

From the commands named:
- **Linux live, the 83 runs at f0b3a66d.** `okur-lab live-baseline linux` (44dde9f3):

      Okur live on linux: 737/829 steps match JAWS exactly; 0 accepted divergences not counted (83 runs, 0 could not be compared)

- **Their replay.** `okur-lab live-replay linux`:

      Okur's live runs on linux replayed: 825/829 steps say what the run said; replayed, 735/829 match JAWS exactly (83 runs replayed, 0 without a trace, 0 could not be replayed)

  The four steps it says otherwise are page reloads (507/s2, 539/s2, 712/s2, s3). Their speech goes on after the load, as Say All does, which the replay's speech does not drive.
- **With the recorded settings,** `okur-lab live-replay linux --recorded-settings`:

      Okur's live runs on linux replayed: 783/829 steps say what the run said; replayed, 773/829 match JAWS exactly (83 runs replayed, 0 without a trace, 0 could not be replayed)

- **A page read, in order and in parallel.** `probe-read-speed.sh` in the lab at ebc4fd11, three rounds a page, the same tree every round:

      color-grid    433 objects: in order 1399-1597 ms, parallel 419-524 ms (4 threads)
      color-names   297 objects: in order 1002-1124 ms, parallel 305-322 ms
      news-article  128 objects: in order  430-722 ms, parallel 151-190 ms
      web-chat       33 objects: in order  113-149 ms, parallel  48-55 ms

- **Windows parity**, around the window id (68274780). `parity-snapshot.sh` before and after: PARITY IDENTICAL.

      JAWS parity (Ia2): 12686/13284 steps match exactly; 388 accepted divergences not counted (0 scenarios could not run)
      JAWS parity (Uia): 11375/13170 steps match exactly; 388 accepted divergences not counted (16 scenarios could not run)

- **Windows' `okur.exe` live on old-pc**, after the split (okur-start-check.ps1, run once):

      running after 6 s: True
      okur quit said: Okur was asked to quit, and is quitting.
      ended after quit: True

  Its speech log: the focus as Okur started, then "Okur exiting."
- **The traces' focus reads.** 81 native reads. Every focus read but 3 carries FOCUSED; the 3 are each desktop fixture's window object at its start, with no role and no states.

- **Linux live on `okur run`, the 83 runs at ebc4fd11 (571cce1b).** These add the walk, read-plan, place and grab changes; pages are still read in order on the event thread. `live-baseline linux`: the same 737/829, the baseline unchanged. `live-replay linux`: 825/829.
- **Their timed reads.** From the traces, with the stretch's trace-stats.js summary:
  - A page: 175 ms at the median, at most 634 ms, 3.9 ms an object.
  - A native focus: 66 ms at the median, at most 473 ms.

## Next

1. **The lab's next run (trace3, at HEAD).** It carries the recorded settings, the page reader with parallel reads, 3c, 3d and the window id. Fetch it, then run `live-baseline linux` (the replay estimate is 773) and `live-replay linux`.
2. **Linux's desktop gaps (`live-baseline` DIFFs):**
   - GTK's drop-down says no "2 of 3": its items are not in the tree until it opens.
   - Its open list starts on the first item, not the chosen one (304/s11 to s13).
   - A tree's levels and places: GTK gives no depth.
   - The window's title at the first focus after the fixture opened (301/s1): Okur has no event for a window's activation.
3. **The review's rest:** (14) CapsLock as the Okur key (X still toggles it); (15) a read with per-node errors marked partial; the fixed depth caps; the plausible items 20 to 24.
4. **The replay's own gap:** Say All after a reload (speech marks).
