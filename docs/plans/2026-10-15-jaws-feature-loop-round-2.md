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
     - **315**, a RichEdit field: lines and Say Font with paragraph formatting.
     - **208**, a web textarea, character by character (the fixed tracer).
   - Re-record **306**, **314** and **209** with trace format 7, which closes the steps whose traces predate it.
   - Commit the evidence, then regenerate the manifest and baselines. Evidence and code never share a commit.
   - Where Windows 10 JAWS differs from what the dev box showed, the old-pc evidence decides, unless the behavior is specific to Windows 11 (`apps::task_switcher`, Notepad's RichEdit).
2. **Make the dev-box tools a harness tool.** The scratchpad C# key sender (foreground-checked) and the FSAPI speech history reader should become a script in `skills/jaws-reference/`, with the user's clipboard saved and restored. It is for checks the user allows on their own machine.
3. **Virtualize Window in Notepad:** JAWS includes the menu bar and status bar (314/s10, s11).
4. **Discovery,** from the JAWS keystroke list. Candidates:
   - Insert+Tab in pages;
   - Insert+B (read window);
   - Insert+Num5, Say Character, pressed twice (phonetic), three times (character value);
   - Insert+Up pressed twice (spell the line);
   - Say All by line;
   - Insert+Ctrl+Up and Insert+Ctrl+Down for the speech rate.
5. **Gaps without an explanation yet:**
   - 312's missing `·😀`;
   - 207/s87's location after the title;
   - 303/s6;
   - 305/s13;
   - the first task switcher opening after start missing item 1;
   - the Claude app announcing twice on arrival.
