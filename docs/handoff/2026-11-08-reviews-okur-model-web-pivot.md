# Handoff 2026-11-08 (2): two reviews, okur-model, the web pivot, Linux pages replayed

After [`2026-11-08-linux-desktop-controls.md`](2026-11-08-linux-desktop-controls.md).

**The owner, 2026-11-08:**
- "Run a reviewer agent instead of just waiting for" the lab.
- Then, on the structure review's recommendations: "yes, go for those recommendations".

**Plans:**
- [`../plans/2026-11-08-structure-for-three-platforms.md`](../plans/2026-11-08-structure-for-three-platforms.md), new.
- [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md).

**The reviews and their status:** [`../references/linux-and-structure-review-2026-11-08.md`](../references/linux-and-structure-review-2026-11-08.md).

## What was done

**The Linux stack's review: 16 findings fixed.** Each finding was checked against the code first. The worst:
- **An X server grab on every focus outside a page (70ab20b1).** Walking up the AT-SPI tree built full element data, and for a frame that means the window manager's facts read under a grab. Walks now use light references.
- **No timeout on any AT-SPI call (de593755).** Calls are now bounded at a second.
- **Every signal's object read before mapping (27ee842b).** A page load sends thousands of signals. Kinds are mapped first, and the object read only for one Okur takes.
- **Page actions inside the keyboard hook's callback (18f5d110).** A slow application meant a double activation. The actions now run on a worker.
- **The desktop never told a screen reader runs (45202034).** `org.a11y.Status` is now set, as Orca does.
- **Lab and runner fixes:** the runner fails keys it did not press and cleans up (df89c0c4). The sync now fails loudly (6d3d525a, earlier).

**The live runs' settings.** The example `okur_linux` set its options where the engine never reads them: every Linux web run read pages at Medium (3b2aacec). The 80-scenario rerun at High: 9e2f9d0c.

**Gate.** Clippy now checks the Linux crates for the Linux target on the dev box (ce08d0ac). Linux code no longer waits for the lab to be compiled.

**The structure.** Steps 5, 4 (part) and 2 of the plan:
- **`okur-model` (ed7c39e5).** A new neutral crate:
  - It holds snapshot provenance and the xa11y element mapping.
  - Both platform models take it.
  - AGENTS.md §6 names it.
- **A plain edit is the platform's fact (d4e88df9).** Linux names each focus's application (514aabf5).
- **The web pivot:**
  - `okur_model::web`: `WebNode` and `web_to_aom`. Windows' IA2 conversion moved there with names for numbers, and `ia2_to_aom = web_to_aom ∘ ia2_to_web` (4d3c8a42).
  - okur-linux-model's `atspi_to_web` replaces the fabricated IA2 objects (8cc1ef08). okur-linux-model no longer depends on okur-windows-model or xa11y-windows.
  - The rule R9 (3c555f36): a platform's model takes no other platform's.

**Linux pages, replayed on every OS (the Linux plan's §2.3):**
- Parity loads AT-SPI snapshots (85872e4a).
- The lab captures them (ad3f8606).
- All 270 fixtures are in: 46 outside ARIA-AT (faf451b4) and 224 ARIA-AT (55832596). Every scenario now replays on the pages as Linux gives them.

## Figures

From the commands named:
- **Windows parity, refactors only.** `parity-snapshot.sh` before (at 3d84b3a4) and after each refactor (d4e88df9, ed7c39e5, 4d3c8a42): "PARITY IDENTICAL".

      JAWS parity (Ia2): 12686/13284 steps match exactly; 388 accepted divergences not counted (0 scenarios could not run)
      JAWS parity (Uia): 11375/13170 steps match exactly; 388 accepted divergences not counted (16 scenarios could not run)

- **Linux pages, replayed from their AT-SPI snapshots.** `okur-lab parity --api atspi`, before and after 8cc1ef08 (identical output, on the 46 fixtures then captured):

      JAWS parity (Atspi): 10232/11661 steps match exactly; 312 accepted divergences not counted (1154 scenarios could not run)

  With all 270 snapshots (55832596), the same scenarios as IA2's 12686/13284. The gap of 1281 steps measures how Linux reads pages:

      JAWS parity (Atspi): 11405/13284 steps match exactly; 388 accepted divergences not counted (0 scenarios could not run)

- **Linux live.** `okur-lab live-baseline linux` at 9e2f9d0c:

      Okur live on linux: 708/829 steps match JAWS exactly; 0 accepted divergences not counted (83 runs, 0 could not be compared)

- **Measured in the lab:**
  - A native focus read takes 40 to 180 ms for 12 to 47 objects.
  - A page reads at about 3 ms per object (color-grid: 433 objects in 1280 ms).
  - GTK's frames carry ACTIVE, so a focus in a window behind is passed over (41adac79).

## Next

1. **The AT-SPI gap.** The 1281 steps where Linux's pages read otherwise than IA2's, by kind (`parity --api atspi` beside `--api ia2`). Then an AT-SPI parity baseline that may only improve.
2. **The lab's rerun `web5`** (the 80 web and 3 desktop scenarios at 3c555f36, the first live proof of the review's fixes and the web pivot):
   - Fetch the runs, run `live-baseline linux` (no regression), and commit the runs and baseline.
3. **The structure plan's next steps:**
   - 1, a replayable Linux event model (records, a pure converter, traces kept with live runs).
   - 3, one `okur` program for every OS (needs a live start on old-pc).
   - Then 4's rest.
4. **The Linux review's open items, in their order:**
   - The page read off the event thread.
   - "n of m" from posinset/setsize, instead of reading the parent whole.
