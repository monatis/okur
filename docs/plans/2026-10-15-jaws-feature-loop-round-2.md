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
     - **317**, Alt held while Tab is pressed three times. Check `sr-cli evidence 317 --trace` for `uia_focus` events. If none arrive on Windows 10, read which UIA events the switcher fires. NVDA's explorer module points at focus on `SwitchItemListControl` items.
     - **316**, Win+M. This settles the "Desktop" heading, the rest of the entry, and the `To move to items…` hint.
     - **315**, a RichEdit field: lines and Say Font with paragraph formatting.
     - **208**, a web textarea, character by character (the fixed tracer).
   - Re-record **306**, **314** and **209** with trace format 7, which closes the steps whose traces predate it.
   - Commit the evidence, then regenerate the manifest and baselines. Evidence and code never share a commit.
2. **Native Say Font with paragraph formatting**, in JAWS's tokens from 315.
3. **Ask the user for a Windows 11 check.** The user runs `sr-cli run --debug-log` and does Alt+Tab with Alt held, Win+M, Insert+T on the desktop, and Ctrl+Shift+End in Notepad. The dev box is the user's machine: never drive it.
4. **Virtualize Window in Notepad:** JAWS includes the menu bar and status bar (314/s10, s11).
5. **Discovery,** from the JAWS keystroke list. Candidates:
   - Say All by line;
   - Insert+Num5, Say Character, pressed twice (phonetic), three times (character value);
   - Insert+Up pressed twice (spell the line);
   - Insert+Tab and Insert+B (read window);
   - Insert+Ctrl+Up and Insert+Ctrl+Down for the speech rate.
6. **Gaps without an explanation yet:**
   - 312's missing `·😀`;
   - 207/s87's location after the title;
   - 303/s6;
   - 305/s13.

## Rules

As in round 1:
- **The live pipeline is the replay pipeline.** Every new event kind goes through `event_trace`, live and in replay.
- **Claims quote tools.** Parity numbers are pasted `sr-cli parity` output, and live claims cite `--debug-log` excerpts or the user's report.
