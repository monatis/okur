# Plan: everyday-use gaps, round 31 (2026-10-24)

Follows [`../handoff/2026-10-24-settings-redesign-round-2.md`](../handoff/2026-10-24-settings-redesign-round-2.md).
The owner's order:
1. Okur's settings redesigned as JAWS's, to its end: what is left is below (§1), a short stretch;
2. then the rest of Microsoft Word (the owner, 2026-10-24: "let's schedule it after you're done
   with settings thing");
3. then punctuation by word and the README's "Not yet" list, in its order.

Each item is found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents
become accepted divergences. old-pc always has a round queued behind the running one. Okur's own
words never use JAWS's name (AGENTS.md §1.5). A stretch is a few hours or about thirty commits;
it ends with a handoff and the next plan, and the README follows a finished area.

## 0. old-pc

- Idle. okur-lab parity at `a470fb09`: "8760/9339 steps match exactly; 251 accepted
  divergences".
- **Queue next:**
  - **The Startup Wizard** (Help, Startup Wizard...): its pages by Tab and Next, then Cancel. Its
    walk was cut short. Confirm the dialog's process before sending keys, and never Finish.
  - **Settings Center in Default**, each group opened, for the value names of Screen Echo, Smart
    Navigation and Indicate Headings (`settings-center-walk.ps1`).
  - **1037 in the owner's Notepad**: the same line in `desktop-richedit.ps1`'s family (Windows 11
    Notepad, RichEdit), for punctuation by word (§3).
  - **Which voice context JAWS uses** for the virtual cursor, forms mode and its own messages
    (extension SDK proposal §6.4): give each context of a test voice profile its own pitch, read
    the pitch of each fragment the speech recorder logs, then delete the test profile. Old-pc's
    rate stays 90.
  - **Okur's dialogs as Okur reads them**: a scenario family that runs Okur on old-pc and records
    its speech of its own dialogs, JAWS's recordings of the same dialogs being the reference.

## 1. Settings, the rest ([`../references/okur-settings-redesign.md`](../references/okur-settings-redesign.md) §6)

In this order, then Word:
1. **The Startup Wizard**, as walked.
2. **F6 to the setting's control** in Settings Center: JAWS's pane ("Typing Echo Characters and
   Words radio button checked | 4 of 4"), F6 back to the tree. This needs okur-ui to show a
   control in place of another.
3. **Tutor hints in the message voice**: the focus speech's hint said apart from the rest.
4. **Last Changed Settings**: Remove and Remove All (to record), and its history kept across
   restarts.
5. **Smaller:**
   - the Rate slider's number;
   - Speak Sample;
   - the Language menu's Voice Profiles;
   - a Say All's parts at the context's pitch;
   - the description note read on Tab;
   - Screen Echo, Smart Navigation and Indicate Headings in the tree.
6. **With setup:** Basics' Automatically Start Okur... and Run Okur from System Tray, setup's
   start entries and desktop icon (backlog, Installer).

## 2. Word, the rest (the owner, 2026-10-24)

From plan 29 §2, unchanged:
- Say All with the structure (518/s20);
- Say Font's line spacing (1034/s1, s3);
- Word's other views;
- tables without a header row;
- bold, italic and underline in Say Font;
- then Excel (521) and Outlook.
The README's Word entry then loses its "Not yet".

## 3. Punctuation by word

1037 is recorded in old-pc's classic Notepad: see the research's "What 1037 recorded". Steps:
1. Record the owner's RichEdit Notepad (§0).
2. Make words as JAWS's: the whole token at a Ctrl+Right stop, a hyphen ending a word, and Say
   Word's unit.
3. Name every symbol in a word read alone, wherever words are read: the virtual cursor, Say Word,
   the Okur cursor, the Virtual Viewer.
4. Add known-gap tests meanwhile for 106/s4, s6, s23 and 213/s11.

## 4. Still open from before

Plan 29 §3-§7:
- Convenient OCR's rest;
- Smart Navigation and Screen Layout;
- the replay's speech clock;
- the hardening review's open findings;
- the backlog;
- the owner's items on old-pc.
