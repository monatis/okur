# Plan: everyday-use gaps, round 32 (2026-10-24)

Follows [`../handoff/2026-10-24-word-round-2.md`](../handoff/2026-10-24-word-round-2.md). The
owner's order:
1. Okur's settings redesigned as JAWS's: what is left is its long tail (plan 31 §1, unchanged);
2. Microsoft Word: done but for line spacing, Web Layout and Read Mode, then Excel and Outlook;
3. punctuation by word, then the README's "Not yet" list, in its order.

Each item is found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents
become accepted divergences. old-pc always has a round queued behind the running one. Okur's own
words never use JAWS's name (AGENTS.md §1.5). A stretch is a few hours or about thirty commits;
it ends with a handoff and the next plan, and the README follows a finished area. A figure in a
commit or handoff is copied from the tool's own line, never typed.

## 0. old-pc

- Idle. okur-lab parity at `323266cc`: "8790/9378 steps match exactly; 251 accepted
  divergences".
- **Queue next:**
  - **Excel (521):** close the lab's Excel and its Office Activation Wizard (started by the
    fixture, no workbook open), then record 521 again. If the wizard comes back with every
    Excel start, the scenario's setup has to close it first.
  - **1037 in the owner's Notepad:** the same line in `desktop-richedit.ps1`'s family (Windows
    11 Notepad, RichEdit), for punctuation by word (§2).
  - **Word's other views:** Web Layout and Read Mode, by the ribbon's View tab or the status
    bar, with Say Window Title in each.
  - **Which voice context JAWS uses** for the virtual cursor, forms mode and its own messages:
    a test voice profile with a pitch per context, the speech recorder's pitches read, then the
    profile deleted (old-pc's rate stays 90).
  - **Okur's dialogs as Okur reads them**: a scenario family that runs Okur on old-pc and records
    its speech of its own dialogs.

## 1. Excel and Outlook

1. Excel: 521 once recorded. Its reading of cells, coordinates (Insert+C), the column's cells
   (Insert+Shift+C), sheets and the list of worksheets, built from what it records.
2. Outlook: whether old-pc's Outlook opens without an account; if not, the owner's call.

## 2. Punctuation by word, the rest

From the research's "What 1037 recorded":
- desktop edits' word at a Ctrl+Right stop: the whole token around the caret, a hyphen ending a
  word ("https://example.org/a", "b"), Say Word's unit;
- then the owner's RichEdit Notepad, once recorded;
- known-gap tests where JAWS's behavior is known and Okur's is not yet.

## 3. Word, what Word does not report

- **Line spacing** in Say Font: Word's UI Automation has no line spacing. Find another source
  that is not Word's object model (its automation hung on old-pc's unactivated Office), or
  leave it as a documented gap.
- **1038's replay start:** trial 1's trace has the caret moved twice before s1. Find out why in
  the recorder or tracer, and whether replay should take another trial's trace.

## 4. Settings, the long tail (plan 31 §1)

The Startup Wizard with setup's start entries; F6 to the setting's control; tutor hints in the
message voice; Last Changed Settings' Remove and Remove All and its history kept; the Rate
slider's number; Speak Sample; the Language menu's Voice Profiles; a Say All's parts at the
context's pitch; Screen Echo, Smart Navigation and Indicate Headings in the tree.

## 5. Still open from before

Plan 29 §3-§7:
- Convenient OCR's rest;
- Smart Navigation and Screen Layout;
- the replay's speech clock;
- the hardening review's open findings;
- the backlog;
- the owner's items on old-pc.
