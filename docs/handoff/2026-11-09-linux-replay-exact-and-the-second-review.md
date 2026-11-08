# Handoff 2026-11-09: Linux's replay made exact, the second review, and the gaps it found

After [`2026-11-08-linux-event-model-and-one-okur.md`](2026-11-08-linux-event-model-and-one-okur.md).

**Plans:**
- [`../plans/2026-11-08-structure-for-three-platforms.md`](../plans/2026-11-08-structure-for-three-platforms.md): steps 1 to 4 done; step 6 goes with the macOS model.
- The Linux review: [`../references/linux-and-structure-review-2026-11-08.md`](../references/linux-and-structure-review-2026-11-08.md). Its second review is in the same file.

## What was done

**The second review of the stretch's own code** (one reviewer agent while the lab ran; each finding checked by hand):
- **The page reader's two faults.** A page read after its focus had left went to the engine as a new load over the editor; its letters became quick keys. A page read after a focus in it lost the focus's place.
  - Fixed by okur-linux-model's `Delivery` (7c246f2c). A page asked for is a slot before its focus, the records after it wait for it, and a focus that leaves the page cancels it. The event thread keeps reading the bus meanwhile.
  - My own `--pages-after-focus` check could not show these: it reordered within one record only.
- **The rest:**
  - A failed read is said from the snapshot (7c246f2c).
  - A dead reader falls back to the event thread (7c246f2c).
  - A read no longer wanted is stopped (1d9dcf46).
  - The idle loop wakes every 100 ms, not every 20 (1d9dcf46).
  - A poisoned lock still numbers a page, and a hang-up stops `okur run` (7c246f2c).

**CapsLock as the Okur key on X11** (the first review's 14; 9a6b6816, ebaf1373):
- While Okur runs, Caps Lock's key loses its keysym and its modifier, and Okur toggles Caps Lock through XKB when the engine passes a press on.
- Clearing the modifier alone was not enough: XKB's Caps_Lock action locks Lock by name.

**The Linux replay made exact:**
- Say All's marks are given back as live (260f9356): 829/829 steps say what the runs said.
- It is now ratcheted: `corpus/live/linux-replay-baseline.json` and the corpus test `linux_replay_matches_its_baseline` (82e7ca6f). A change that costs a Linux step fails without the lab.
- `live-replay --jaws` lists the replayed steps that differ from JAWS; `--only` with RUST_LOG gives the engine's reasons.

**The Linux runs' settings** (ce95271b, the stretch before): each run gets the settings JAWS recorded its scenario with.

**Gaps found and fixed through the replay, without the lab:**
- **Ctrl+Home to the page's top is no move since the load (d93ecdcc, an engine change, PARITY IDENTICAL).** A live reset's Ctrl+Home had turned forms mode on at the next Tab [obs, 204/s1].
- **A page's focus carries its window's title (c2b9f7eb).** Say Title said "... - Google Chrome" for JAWS on Windows only [obs, 625].
- **A select's open list is said as a list box (c0a567f0)** where Chrome on Linux shows it as a menu [obs, 276/s2]. This needs the read the next run carries.
- **A select's value changed by the arrows is said (a700d3ce)** [obs, 205/s9, s10].
- **A page's focus names its node by the id its page read gave it (7bd5975a).** A link's focus was said without its role [obs, 204/s9, 293/s1].

**Found through AT-SPI parity:**
- **A link is found in its page by its name, whatever its address (019e4a6b, an engine change, PARITY IDENTICAL).** A focus that gives the address in another form, or none, found no link.
- **A page is read again when the focus lands on an object it does not hold (12447157).** A modal dialog's content, hidden until it opened, was in no page Okur had read [obs, 2681]. The next run shows it live.
- **What AT-SPI parity's remaining gap mostly is:** Windows' recorded focus events replayed on Linux snapshots captured at load, so the page state a setup made (a tab activated, a link followed and visited) is missing. Linux's live runs and their replay measure Linux.

**Lab tooling:**
- `probe-capslock.sh`.
- The runner keeps Okur's debug log beside each run on old-pc (c42c66e8).

## Figures

From the commands named:
- **Linux live, the 83 runs at 9a6b6816** (the recorded settings and the page order; fcea7533). `okur-lab live-baseline linux`: 767/829 match JAWS (737 before), no step lost.
- **Their replay with the code since.** `okur-lab live-replay linux`: 792/829 (43c2ad73).
- **Their page reads, parallel and once a run:** 80 ms at the median, at most 192 ms, 1.7 ms an object (3.9 before).
- **CapsLock:** `probe-capslock.sh` at 0e7ad2a0, "CAPSLOCK PASS".
- **Windows parity**, around the Ctrl+Home change and the link lookup: `parity-snapshot.sh`, PARITY IDENTICAL (IA2 12686/13284, UIA 11375/13170).
- **Linux's pages replayed with Windows' recorded focus events.** `okur-lab parity --api atspi`: 12328/13284 (12204 before; 44c05a68).
- **A platform difference, kept:** Insert+F says Linux's font, "Liberation Serif", where JAWS on Windows says "Times New Roman" [obs, 209].

- **Linux live, the 83 runs at 0e7ad2a0** (all the fixes above but the last three; 0dfd1bc3). `okur-lab live-baseline linux`:

      Okur live on linux: 791/829 steps match JAWS exactly; 0 accepted divergences not counted (83 runs, 0 could not be compared)

- **Their replay with the code since** (unique ids, the select's expansion, ca11938e). `okur-lab live-replay linux`:

      Okur's live runs on linux replayed: 823/829 steps say what the run said; replayed, 796/829 match JAWS exactly (83 runs replayed, 0 without a trace, 0 could not be replayed)

- **The day's road, live:** 708 (the stretch before) → 737 → 767 → 791 of 829.

## Next

1. **The lab's run at 12447157 (trace6, running).** It carries the unique ids, the link lookup and the re-read of a changed page. Fetch it, write both baselines (the replay predicts at least 796), and read its Okur logs for 204's missing focus events.
2. **What only the lab shows:**
   - 204/s10 to s13: after Tab onto a link, Chrome sent no focus event for the button and the toolbar that followed. The runner now keeps Okur's log for this.
   - 276/s2: Space on a select says the select before its list, where JAWS says the list alone.
3. **The lab's silent synthesizer reads a Say All instantly**, where JAWS's recordings were made at real speech. The Say Line after an interrupted reading lands at the page's end (01, 507, 539, 969).
   - A paced silent synthesizer, for the lab only, would make these comparable.
   - A braille-only user may run Okur without speech, so plain no-speech must stay unpaced.
4. **Okur on both platforms:**
   - JAWS's Y, the next mark [obs, 292/s12 to s15].
   - The field hint after Ctrl+Home then Tab [obs, 564/s18].
5. **GTK's limits, platform differences to write down as accepted divergences:**
   - A tree's depth and places.
   - The drop-down's "2 of 3".
6. **The second review's open items:** a parallel read cut at its limit is unmarked (6); grab replies on the keyboard thread (9).
