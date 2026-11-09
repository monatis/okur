# Handoff 2026-11-09 (night): okur-ui in the replay, the fourth review, and full5's first third

After [`2026-11-09-linux-menus-dialogs-and-the-third-review.md`](2026-11-09-linux-menus-dialogs-and-the-third-review.md).

**Plan:** [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md); its status section and known gaps are current (c5c59a7b, 06e28175).

## What was done

**The replay of Okur's own dialogs.**
- **The trace:** okur-ui's messages (a dialog's choices, its buttons, its close) are records of the Linux trace (fa7403d8). Okur's spawner on Linux wraps okur-ui's output and writes each message line as the UI host reads it.
- **The replay:** it gives its engine a UI host, as parity's replay does (f5c9f29d), so Okur's dialogs open in the replay and the records act on them. Before, a dialog scenario lost its choices and its close in the replay.
- **Proof:** full5's 228 replays to 12 of 12 steps as JAWS's.

**The fourth code review** (one reviewer agent; each finding checked against the code; 240fb118, 79a1208e, b3f3639f):
- **Insert+E in a Linux dialog.** The dialog box was one to the engine without a default button, so Insert+E said nothing in it. A dialog box is now read whole once for its window: its buttons in order, its default by GTK's IS_DEFAULT state, and a message box's text (79a1208e). Tests.
- **okur-ui's Enter.**
  - Enter in an edit now activates the default button.
  - Enter on a list's item clicks the default button where it is available.
  - DialogRows fills a table.
  - Closed dialogs are no longer kept alive by their handlers.
  - okur-ui is woken for each message instead of polling every 10 ms.
- **The rest:**
  - live regions due together share one due time, then are read in the page's order;
  - Chrome's Reading Mode WebUI is no page load;
  - the refresh rule is one engine method, `is_refresh`.

**Linux, more:**
- **A page changing outside its live regions is captured again once its changes settle (90d63662).** The first change of a read page's children is heard, and none more until the page is read again.
- **From full5's first runs (f5c9f29d):**
  - An arrowed field's caret is placed at its start in Chrome too. Chrome kept it at the end, and the field read at its focus kept Up in the field a key longer (283).
  - Up and Down in a combo box whose list is open read no line (211/s9).
  - Windows' parity is unchanged.
- **okur-ui (db45c092, 153453ef):**
  - controls in the spec's order, so Tab follows it (the list, its buttons, then its radios [obs, 222/s2]);
  - a list's first focus once the dialog is laid out (its row's object was gone as Okur read it);
  - a list's first-letter search, which GTK's lists lack ("h" is "Headings List" [obs, 226/s2]).
- **Chrome's app menu (c2a83a88).** Its submenu items carry AT-SPI's HAS_POPUP state. The menu opens with no item highlighted on Linux, read in full5's 220; a probe of AT-SPI's SelectChild is ready (cc4e5285).
- **The runner:**
  - `text` and `key_ui_access` steps (2a4f0992), which full5 skipped in 1151 and 1157;
  - unbuffered output (836cb49c): full5's log stood 25 minutes behind its runs.
- **`job.sh stop` (ac832aec).** okur-ui failed to build in full4 (E0373, 63bc9099), was stopped, and was run again.

## Figures

From the commands named:
- **Linux live, 417 runs.** `okur-lab live-baseline linux` (fd07695b): "6101/6511 steps match JAWS exactly; 64 accepted divergences not counted (417 runs, 0 could not be compared)". The evening's handoff had 5831/6219 over 371.
- **Their replay with the code at f5c9f29d.** `okur-lab live-replay linux`: "6528/6575 steps say what the run said; replayed, 6109/6511 match JAWS exactly", unchanged at c2a83a88.
- **Windows parity unchanged.** `jaws_parity_matches_baseline` passed in each code commit's gate.
- **Okur's GTK dialog latency (gtk3's 222, from its log).** The dialog's window comes 95 ms after Insert+F7, its first focus is read about 300 ms after it, and a list item reads in 64–88 ms.
- **The lab's check.** "LINUX CHECK PASS" at 63bc9099 and 240fb118 (full4, full5), with okur-ui's tables. The okur-ui changes after 240fb118 are checked in the next job.

## Lab state

`full5` runs the 512 scenarios outside ARIA-AT at 240fb118, 244 done at 15:14, about 66 an hour. Its first 163 runs are committed (fd07695b).

## Next

1. **When `full5` ends:**
   - fetch its runs, check their privacy, and write both baselines;
   - compare the replay with Windows' parity for Linux's own gaps.
2. **Then `gtk4`:**
   - the check (okur-ui since 240fb118 is unbuilt so far);
   - `probe-chrome-menu-select.sh`;
   - the 42 scenarios this stretch touched (the list in the scratchpad's `gtk4-ids.txt`: the dialogs, 211, 220, 228, 283, 284, the region order, 1151, 1157).
3. **Open (the plan):**
   - 228's live Escape going to Chrome (a debug line now names an application region if that is it);
   - Chrome's tabs (220/s9, s12);
   - Alt+F on Chrome, once the probe answers;
   - keys before Okur's dialog takes the focus;
   - the GTK list's three reads per arrow;
   - ARIA-AT's scenarios on Linux, a run of their own.
