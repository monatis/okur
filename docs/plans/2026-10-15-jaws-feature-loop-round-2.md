# Plan: JAWS Feature Loop, Round 2 (2026-10-15)

**Branch**: `jaws-feature-loop`.
**Preceded by**: handoff `docs/handoff/2026-10-15-jaws-feature-loop-round-1.md`.

## Goal

Keep the loop running:
1. Discover a JAWS feature.
2. Record it on old-pc.
3. Implement it.
4. Prove it with `sr-cli parity` (both layers) and live.

Live usability gaps the user reports come first.

## Work items, in order

1. **Record what round 1 left unrecorded** (old-pc was offline). The scenarios are untracked in the working tree.
   - Deploy a release `sr-cli` built with trace format 7 and the UIA focus handler: `lab-push.ps1 -SrCli`, then `lab-pull.ps1` on old-pc.
   - Record:
     - **317**, Alt held while Tab is pressed three times. Check `sr-cli evidence 317 --trace` for `uia_focus` events. On Windows 10 the switcher (`MultitaskingViewFrame`) sits behind `ForegroundStaging`.
     - **316**, Win+M.
     - **318**, Virtualize in a page.
     - **319**, the Run dialog: its announcement, Virtualize Window, and Insert+Tab could join.
     - **320**, Notepad in the laptop layout: CapsLock+I, K, O, L, J, M, U, Insert+Up, and Ctrl+Right over code punctuation. It settles classic Notepad's word breaks against the UIA word unit used on Windows 11.
     - **315**, a RichEdit field: lines and Say Font with paragraph formatting.
     - **208**, a web textarea, character by character (the fixed tracer).
   - Re-record **306**, **314** and **209** with trace format 7, which closes the steps whose traces predate it.
   - Commit the evidence, then regenerate the manifest and baselines. Evidence and code never share a commit.
   - Where Windows 10 JAWS differs from what the dev box showed, the old-pc evidence decides, unless the behavior is specific to Windows 11 (`apps::task_switcher`, Notepad's RichEdit).
2. ~~Make the dev-box tools a harness tool~~: done (`sr-cli keys`, `sr-cli jaws-history`, b2a0ecf).
3. **Windows 11 XAML menus** (Notepad's menu bar). UIA's MenuItem control type maps to `Generic`, so ours reads bare names ("Edit", "Undo"). JAWS 2025 on the dev box:
   - Alt: "menu | | File | 1 of 3 | To move through items press left or right arrow. | Alt followed by F"
   - Down: "Leaving menus | Text editor edit | Contains text | Context Menu | | New window Ctrl+Shift+N | 2 of 13 | To move through items press up or down arrow. | Alt followed by W"
   - Down: "New Markdown tab | 3 of 13 | Alt followed by M"
   - Right: "Undo Ctrl+Z | 1 of 15 | Alt followed by U"
   - Escape: "Leaving menus"

   Mapping MenuItem also changes web pages' buffers, so check it against parity, and against 302's Win32 menu evidence on old-pc.
4. **Virtualize Window in Notepad:** JAWS includes the menu bar and status bar (314/s10, s11).
5. **Discovery,** from the JAWS keystroke list. Candidates:
   - Insert+Tab in pages;
   - Insert+B in windows other than dialog boxes;
   - Insert+Num5, Say Character, pressed twice (phonetic), three times (character value);
   - Insert+Up pressed twice (spell the line);
   - Say All by line;
   - Insert+Ctrl+Up and Insert+Ctrl+Down for the speech rate.
6. **Gaps without an explanation yet:**
   - 312's missing `·😀`;
   - 207/s87's location after the title;
   - 303/s6;
   - 305/s13;
   - the Claude app announcing twice on arrival (fixed for Chrome by d8235b8; recheck in the Claude app);
   - Windows search on Windows 10 (old-pc): whether its results raise the same ElementSelected events (scenario still to write).
