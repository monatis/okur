# Plan: everyday-use gaps, round 40 (2026-10-31)

Follows [`../handoff/2026-10-31-word-revisions-pages-status-bar.md`](../handoff/2026-10-31-word-revisions-pages-status-bar.md).
The owner's order, unchanged from plan 39:
1. Okur's settings redesigned as JAWS's: what is left of its long tail (§1);
2. Microsoft Word: what is left, then Excel and Outlook (§2);
3. the review's open findings (§3), punctuation by word (§4), then the README's "Not yet"
   list, in its order.

Each item is found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents
become accepted divergences. old-pc always has a round queued behind the running one. Okur's own
words never use JAWS's name (AGENTS.md §1.5). A stretch is a few hours or about thirty commits;
it ends with a handoff and the next plan, and the README follows a finished area. A figure in a
commit or handoff is copied from the tool's own line after the tool ran, never typed first. A
commit's paths are staged in a command of their own and checked with `git status` first: a
blocked commit runs none of its command, a `mv` beside it included. A new fixture waits outside
the tree during a code commit (the manifest would be stale) and goes in with its evidence.

**The dev box is shared** (the owner, 2026-10-30 and 31): one heavy job at a time, cargo on at
most 4 of its 16 cores (`CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4`, set for Claude Code's
sessions in the untracked `.claude/settings.local.json`), no swarms of subagents.

Before every round: the voice profile's `Rate=` lines on old-pc must read 90; old-pc's master
volume stays at 1%, not muted (the owner, 2026-10-28). Keys inside JAWS's own list dialogs are
`key_ui_access`; keys in its Virtual Viewer are plain. A Windows key chord gets the recorder's
mask key at once (`run-scenario.ps1`).

## 0. old-pc

- Idle after round-1112c (a check of the recorder's new Close with Word). 521, Excel's cells,
  is recorded and in the corpus (round-521h).
- **Excel on old-pc runs one instance** with a keeper workbook (Book1, "lab" in A1) and its
  Activation Wizard closed; the Excel fixture opens into it. Keep it running: a new Excel
  process shows the wizard, and ending one marks its open file and leaves a Document Recovery
  pane under which Excel refuses to close anything.
- Queue next: Excel's keys from JAWS's documentation (`popapps.chm`, Excel's JAWS-specific
  keystrokes); Word's page moves by character and word; 1069 again.

## 1. Settings, the long tail

1. **Voices:** Apply's question whether to make a saved profile the default. It needs a saved,
   non-factory profile changed and applied on old-pc: create a lab profile with Save As, change
   its pitch, Apply, read the question, answer No, Cancel; then delete the profile and check
   the VPF's `Rate=` lines read 90. Synthesizer Language.
2. **The Startup Wizard's rest:** Run Okur from System Tray; Basics' "Automatically Start
   Okur..." button.
3. **Smaller:** the lifetimes' icons beside Quick Settings' items.
4. **A live look** at Settings Center's results list, the pane name, the Access Key choices
   and the menu letters, and Word's notes lists, with the owner's leave on the dev box.

## 2. Word, then Excel and Outlook

- Done in round 39: the Virtual Viewer of notes and revisions, the revisions list and Announce
  Revision, deletions not read, pages and sections, Insert+Delete, the status bar, superscript
  at a note's reference, Save As's and Insert Hyperlink's open steps.
- **Word, open:**
  - the Spelling pane's suggestion said twice (1069/s4): one trial of three reached the list;
    record 1069 again with a steadier path there before deciding;
  - a page reached by Right, Left and the word keys (only the line keys say it now) [hyp];
  - one revision in the viewer ("There is only one Revision to display." [hyp]), another
    change than inserted or deleted text (formatting), and Insert+Shift+R's "no revisions";
  - Insert+Alt+R: JAWS let it through to Word in both states (round-1107, round-1106b's 1110
    and 1111, not imported); Okur takes the chord. Decide with a scenario that shows what
    Word's Alt+R does after it.
- **Excel** from 521 (0 of 14 steps): a cell said as its value and coordinates ("12 | B 2"),
  Say Line as the row, Insert+C, Insert+Shift+C's "Col B" list, Ctrl+PageDown's other sheet,
  Ctrl+Shift+S's "Move to Worksheet"; an Excel module as Word's. Then its keys from JAWS's
  documentation. **Outlook**: whether old-pc's opens without an account.

## 3. The review of 2026-10-25, what is open

[`../backlog/performance-review-2026-10-25.md`](../backlog/performance-review-2026-10-25.md):
6 (a slim control kind in okur-ui), 10 (a live check from the installed lowered build), 14
(live), 18, 19 (the NetUI policy in the platform), 20 (the rest). 7 and 9 are fixed.

## 4. Punctuation by word, the rest

The Okur cursor's words (1062, recorded, pending: JAWS splits "C:\Users\Ada\Downloads" at each
backslash where Okur says the whole path); Say Word on classic Notepad's `:\` stop; a live check
in Windows 11 Notepad with the owner's leave.

## 5. Still open from before

Plan 29 §3-§7: Convenient OCR's rest; Smart Navigation and Screen Layout; the replay's speech
clock; the earlier reviews' open findings; the backlog; the owner's items on old-pc.
