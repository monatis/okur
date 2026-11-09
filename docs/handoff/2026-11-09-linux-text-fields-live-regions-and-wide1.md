# Handoff 2026-11-09: Linux text fields, live regions, and 147 more scenarios

After [`2026-11-09-linux-replay-exact-and-the-second-review.md`](2026-11-09-linux-replay-exact-and-the-second-review.md).

**Plan:** [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md); its status section is current (d1d46690, b35c4182).

## What was done

**147 web scenarios run on Linux for the first time** (the job `wide1`, at aac6f125):
- 273 were queued. The job was stopped at 147 because the code had moved on: the full run `full2` (below) runs all of them at fcd2601b.
- **Privacy:** the traces were checked for links and the owner's data before committing. They hold only the fixtures' own addresses and the lab's paths (c229220a, a20e822d).
- **`fetch-runs.sh`** copies the runs before archiving them, skipping a run still being written. A locked trace had made the archive fail (ed055c8c).

**Gaps the replay found, built without the lab** (each one needs a live run to show; `full2` carries most of them):
- **A text field read at its caret (ed055c8c).**
  - The engine asks the platform to read the focused field after an editing key (`read_at_caret`), and Linux answered nothing. Down in a field neither read its line nor left the field at its edge [obs, 280, 283, 284, 285, 595, 208].
  - The actions' worker now reads AT-SPI's Text interface. It polls until the key moved the caret, as Windows' page fields are read, then gives the engine a `TextContext`.
  - The read is written to the trace as a record of its own. The trace is held across each record's write and its send, so the trace's order is the engine's.
  - Since 80b28c82 the read also has the colors at the caret (Say Color), the misspelled words and Chrome's word stops.
- **An option's list and place (ed055c8c):** "Fruits list | with 3 items | Apple | 1 of 3". 286's differences went from 52 steps to 1.
- **The field an arrow focused keeps its text when its focus brings none (ed055c8c, an engine change):** Down in an empty "Full name" leaves it [obs, 280/s6].
- **A toggle button pressed is on (0fc82517).** Chrome gives aria-pressed as AT-SPI's PRESSED, which xa11y-linux did not read. "Chats" was said "Not pressed" [obs, 281/s1, 219/s1].
- **A focus in an application region names it (80b28c82):** "Workbench", and the page takes every key there [obs, 215/s9, s10].
- **Live regions (fcd2601b).**
  - xa11y-linux's event filter now sees each signal's object, still before reading it.
  - The driver hears children and text changes from the known live regions' objects and reads the page again.
  - The converter compares each region's lines between the two reads and says what changed before the page's refresh: "Messages | You: Message 1" [obs, 206].
- **Space on a page's check box says its new state (5ae3f96e).** Chrome on Linux focuses a check box before toggling it. An activated toggle's focus now waits up to 80 ms for its state [obs, 229/s8].
- **The runner (ed055c8c, 0fc82517):**
  - A step's `"repeat"` is pressed again 120 ms after each release, as `run-scenario.ps1` does: Say Character twice [404], Down 14 times [617, 620, 656, 657], phonetic on pause [1141].
  - Caps Lock, the laptop Okur key, is pressed by its keycode while Okur holds its keysym [408, 613].
- **The replay gives the engine what Linux's driver does (ed055c8c):** focus, activate, and caret reads where the run's trace has them. Before, a test's actions answered every ask.

**The code review of 2026-11-09** (one reviewer agent while `full2` ran; each finding checked by hand; b35c4182):
- **1, confirmed:** a live region in a background page would have been read as a new page load over the user's editor. Only the page the focus is in is read again now.
- **3:** the first caret read after a page field's focus now waits for the key.
- **4:** a caret read reads the field whose focus the engine was given, and is dropped if the focus moved on.
- **5:** cheaper polling; misspelled words are read only in the caret's line.
- **6:** trace records are stamped with when the engine got them. The replay orders by that stamp and refuses a trace whose stamps go backwards.
- **Open (the plan):** finding 2, a live change reads the whole page; finding 7, regions added after a read.

**Lab:** `probe-chrome-menu.sh` (d1d46690) lists what Chrome sends as its own menu opens. Okur's run of 220 heard the menu window and no item focused. It runs after `full2`.

## Figures

From the commands named:
- **Linux live, the 230 runs at aac6f125.** `okur-lab live-baseline linux` (a20e822d): "3723/4265 steps match JAWS exactly; 39 accepted divergences not counted (230 runs, 0 could not be compared)".
- **Their replay with the code at 80b28c82.** `okur-lab live-replay linux`: "4140/4304 steps say what the run said; replayed, 3811/4265 match JAWS exactly (230 runs replayed, 0 without a trace, 0 could not be replayed)". The same at b35c4182's code (the same command, run before that commit), and `linux_replay_matches_its_baseline` passed in its gate.
- **Steps the live runs matched that today's code loses in replay.** One, 285/s5: the old live run matched there by saying nothing. A run without caret reads has the engine take the arrow's own caret, at the field's start; a live run's caret read gives the real one.
- **On the first 136 runs (before 80b28c82):** live 1561/1823, replayed 1625/1823 (1567 before the first fixes).
- **Each commit's gate:** `cargo fmt`, clippy, `cargo test --workspace`, and clippy for the Linux target.
  - The workspace tests compare the IA2, UIA and AT-SPI parity outputs with their committed baselines exactly, so Windows parity did not change with the one engine change (ed055c8c).
  - The lab's own check passed at fcd2601b: `check-linux.sh`, "LINUX CHECK PASS".

## Next

1. **When `full2` ends:**
   - Fetch its 356 runs and write both baselines.
   - Check the traces' privacy and commit the runs apart from code.
   - List what is left with `live-replay --jaws`.
2. **A targeted run of the text field, live region and check box scenarios at b35c4182**, for the review's fixes.
3. **Run `probe-chrome-menu.sh`**, then Chrome's own menus on Linux (220).
4. **The review's finding 2:** read a live region alone. Measure first with a fixture of about 2,000 objects and a region ticking once a second.
5. **Then the plan's order:** Okur's dialogs on Linux (no okur-ui there yet: 687, 692, 530), GTK's limits written as accepted divergences, the Mac key decision (D5) before macOS.

**Known measurement limit (both systems):** Say All in the lab finishes at once with the silent synthesizer. Steps after an Insert+Down compare where JAWS's speech was cut off (500, 680, 681, 969: Windows' parity fails them alike).
