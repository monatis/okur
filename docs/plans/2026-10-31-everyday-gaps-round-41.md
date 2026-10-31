# Plan: everyday-use gaps, round 41 (2026-10-31)

Follows [`../handoff/2026-10-31-excel-round-1.md`](../handoff/2026-10-31-excel-round-1.md).
The owner's order, unchanged from plan 40:
1. Okur's settings redesigned as JAWS's: what is left of its long tail (§1);
2. Microsoft Word: what is left, then Excel and Outlook (§2);
3. the review's open findings (§3), punctuation by word (§4), then the README's "Not yet"
   list, in its order.

Each item is found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents
become accepted divergences. old-pc always has a round queued behind the running one. Okur's own
words never use JAWS's name (AGENTS.md §1.5). A stretch is a few hours or about thirty commits;
it ends with a handoff and the next plan, and the README follows a finished area. A figure in a
commit or handoff is copied from the tool's own line after the tool ran, never typed first, nor
written in the command that runs the tool. A commit's paths are staged in a command of their
own and checked with `git status` first: a blocked commit runs none of its command. A new
fixture waits outside the tree during a code commit and goes in with its evidence.

**The dev box is shared** (the owner, 2026-10-30 and 31): one heavy job at a time, cargo on at
most 4 of its 16 cores (`CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=4` in the untracked
`.claude/settings.local.json`), no swarms of subagents.

Before every round: the voice profile's `Rate=` lines on old-pc must read 90; old-pc's master
volume stays at 1%, not muted. Keys inside JAWS's own list dialogs are `key_ui_access`; keys in
its Virtual Viewer and in an application's own menus are plain. A scenario whose replay needs
Word's or Excel's object model carries `"trace_headings": true`. Excel runs one instance with
its keeper workbook; keep it running.

## 0. old-pc

- Idle after round-1118b. Queue next: Excel's selections (§2), several samples of each.

## 1. Settings, the long tail

1. **Voices:** Apply's question whether to make a saved profile the default, with a lab
   profile (Save As, change, Apply, read, No, Cancel; delete the profile; `Rate=` lines read
   90 after). Synthesizer Language.
2. **The Startup Wizard's rest:** Run Okur from System Tray; Basics' "Automatically Start
   Okur..." button.
3. **Smaller:** the lifetimes' icons beside Quick Settings' items.
4. **A live look** with the owner's leave on the dev box.

## 2. Word, Excel, Outlook

- **Excel, open:**
  - selections: done (1117, 1120, 1121);
  - the sheet tabs' menu (1116/s5): Insert+Shift+S opens it (Excel's `CommandBars("Ply")`),
    and an Office popup is named as JAWS names it, "Ply pop-up menu";
  - Insert+C on an empty cell (the grid's focus has no cell: the sheet place gives it);
  - typing into a cell, Delete, and Excel's own dialogs (Format Cells, Go To).
- **Word, open:** 1069/s4 (record again); 1119/s3 (the phonetic name after the page);
  Insert+Alt+R; one revision and other changes in the viewer.
- **Outlook:** old-pc's does not open without an account: it starts "Welcome to Microsoft
  Outlook 2016", the account wizard (tried 2026-10-31, cancelled). Recording Outlook needs a
  mail account set up there, which is the owner's to decide.

## 3. The review of 2026-10-25, what is open

[`../backlog/performance-review-2026-10-25.md`](../backlog/performance-review-2026-10-25.md):
6, 10, 14, 18, 19, 20.

## 4. Punctuation by word, the rest

The Okur cursor's words (1062, recorded, pending); Say Word on classic Notepad's `:\` stop.

## 5. Still open from before

Plan 29 §3-§7.
