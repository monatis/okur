# Handoff 2026-11-09 (evening): Chrome's menus on Linux, Okur's GTK dialogs, the third review

After [`2026-11-09-linux-text-fields-live-regions-and-wide1.md`](2026-11-09-linux-text-fields-live-regions-and-wide1.md).

**Plan:** [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md); its status section is current as of ebcc22e6, and this handoff adds what came after.

## What was done

**The full run `full2` finished:** all 356 scenarios at fcd2601b, every one "success True" (ccb4c70a).

**Gaps found by setting the Linux replay beside Windows' parity.** At ccb4c70a, 119 steps matched JAWS on Windows but not in the Linux replay. Built from that list:
- **Chrome's site information button is a menu button (a6b5b100).** Chrome on Linux gives it neither `haspopup` nor a collapsed state, only its view's class [obs, 204/s12]. Ten tab-order steps.
- **The lab's fixture path is compared as old-pc's (a6b5b100).** Runs record where their fixtures are, and the address bar's path is compared as the reference machine's.
- **Chrome's own menus (69c99756, then 3dc4cbca).**
  - Chrome on Linux opens its menus in a window of their own and highlights items with AT-SPI's `selected`, focusing none. Okur's virtual cursor kept the arrows and read the page under the open menu.
  - The menu window coming forward now starts menu mode and its popup. Highlights are said as focus with their places, and the window going ends the popup.
  - Live in `gtk2`: "Context Menu | To navigate press Up or Down Arrow. | New tab Ctrl+T | 1 of 21", then Escape: "Leaving menus".
- **Page progress bars (157e2b01, then 3dc4cbca).** Their values, and their value changes as the engine's `ProgressChanged`.
- **A table's click listeners (157e2b01).** xa11y-linux reads a table's, a row's and a cell's default action ("clickable").
- **Say System Time and Date on Linux (adc94d7a).** `LinuxClock`: `localtime_r`, plus `nl_langinfo_l` for the locale's formats.
- **A text field read at its caret as its focus is delivered (ac1304bd).** Windows' focus carries the field's text. Live in `gtk2`, 676's Insert+5 gives "navy on yellow", "red on white" and "black on white".
- **Route PC to Virtual on what cannot take the focus gives it to the page (0a81b747).** This is what Windows' Chrome does [obs, 569/s8].
- **Live regions due together are read in the page's order (963222c1).** Chrome signals the status before the log [obs, 206/s3, 961/s3]. The first attempt, the order of first signals (91f803a4), did not hold in `gtk3`.
- **The runner (a6b5b100, 5bf54de3, cc980a5d):**
  - a step's `during` keys;
  - JAWS's TypeKey and TypeString steps, used by 142 scenarios, most of them Okur's dialogs;
  - a step's `quiet_ms` and `settle_max_ms`;
  - every crate's warnings in Okur's log.

**Okur's own dialogs on Linux.** The GTK 4 renderer (5d0cd15c) builds, lints and passes its tests in the lab ("LINUX CHECK PASS" in `gtk2`, `gtk3` and `full3`). The element lists ran live for the first time in `gtk2`. Fixes since:
- **The dialog came 5.4 s after Insert+F7, its focus 7.7 s.** GTK was drawing with OpenGL in software. With the cairo renderer (21546c2c), `gtk3`'s 222 shows the dialog's window 95 ms after the key and its first focus read about 300 ms after it (Okur's log, lines "a window put in a frame" and "native focus read").
- **A GTK list item cost 124 to 179 ms to read.** With the parallel reader (de428ec5) it costs 64 to 88 ms (the same log lines). Up to three reads come per arrow.
- **Lists:**
  - GTK's list box moved Tab from row to row, so the renderer's lists are now column views and list views with one Tab stop (8c7ba409).
  - The window is a dialog to the accessibility bus. The Linux mapping says a dialog window with its role.
  - Tables are drawn (963222c1): the PlaceMarker list's focus went to its radio buttons [687/s11].
- **Escape from Okur's dialog announced the page as a new load and read it through (33190197, an engine change).**
  - Linux reads the page again as the focus comes back to it, and that read reaches the engine before the focus does.
  - A page the platform read again is now marked (`READ_AGAIN_ATTR`), and the engine takes it as the held page's refresh wherever the focus was.
  - Windows never marks a page, and its parity is unchanged. The first try changed 742's reload after Okur's wizard: dropping the focus condition for every page let the F5 reload pass silently. It was reverted.
- **Chrome's omnibox popup (`chrome://omnibox-popup.top-chrome/`) is no page load (963222c1).** It was announced as one [220/s8].

**The third code review** (one reviewer agent while `gtk2` ran; each finding checked against the code; 3dc4cbca). Findings 1 to 8 were built:
- **1, 3:** each arrow in Chrome's menu read the whole menu on the event thread. The menu is now read once as it opens, and highlights are never read.
- **2:** the menu stack ended only on its windows' deactivations. A window with a title, or a focus, now ends it. Submenus are handled in either order. Tests.
- **4:** progress values come as Chromium's f32 widened (86.02739715576172) and are said as the float they were. The bus is checked.
- **5:** a failed toolkit lookup was kept for the application's life.
- **6:** a read made as a field took the focus could be taken as a key's answer.
- **7:** the clock now calls `tzset`, handles `%E`/`%O`, and copes with CJK seconds.
- **8:** value changes are now heard only from page progress bars.
- **Kept:** 9, one call per table part's default action on a page read.

## Figures

From the commands named:
- **Linux live, 371 runs.** `okur-lab live-baseline linux` (fe7d44b2): "5831/6219 steps match JAWS exactly; 60 accepted divergences not counted (371 runs, 0 could not be compared)". Before: 5645/6004 over 356 runs (ccb4c70a); 3921/4265 at the last handoff.
- **Their replay with the code at 963222c1.** `okur-lab live-replay linux --write`: "replayed, 5834/6219 match JAWS exactly".
- **Windows parity unchanged.** `jaws_parity_matches_baseline` and `atspi_parity_matches_baseline` passed in each code commit's gate.
- **Regressions let go, each with its reason in its commit:**
  - 288/s2: a variant shift.
  - 960/s3, 206/s3, 961/s3: region order, fixed since.
  - 530/s11: JAWS's recording broke after its elevated dialog.
  - 220/s8: the omnibox popup, fixed since.
  - 687/691/692 s12: the page read again, fixed since.
- **280's run in `gtk1` hit a three-second stall in Chrome's accessibility** ("Text.CaretOffset not read: I/O error: timed out"). Its earlier run was kept, and `gtk3` ran it again with "success True".

## Lab state

`full3` runs the 512 scenarios outside ARIA-AT (371 run before, 141 never) at 33190197, about 10 hours. Its okur-ui has the list and column views but not the tables (963222c1), so the PlaceMarker scenarios still show the radio buttons.

## Next

1. **When `full3` ends:**
   - fetch its runs, check their privacy, and write both baselines;
   - list the remaining gaps with `live-replay --jaws`, and against Windows' parity as in this stretch;
   - run the table and region-order scenarios again (687, 691, 692, 206, 961).
2. **Okur's dialogs on Linux:**
   - say JAWS's "List view" and "dialog" as measured live;
   - keys pressed before the dialog's focus go to the page;
   - the quiet return after the Skim Reading dialog (530/s11 says the page and its window);
   - the renderer's trees, check lists, menus, wizard, main window and viewer.
3. **The replay keeps no okur-ui messages.** A dialog's close, and with it the quiet return, cannot be replayed; okur-ui's messages belong in the trace.
4. **Chrome's Alt+F says nothing until the first Down,** because Chrome on Linux highlights no item as the menu opens.
5. **Open from before:** a live region added after its page's read; GTK's drop-down count and tree depth; ARIA-AT's scenarios on Linux (about 1,150), a run of their own.
