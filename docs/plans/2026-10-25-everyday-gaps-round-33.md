# Plan: everyday-use gaps, round 33 (2026-10-25)

Follows [`../handoff/2026-10-25-settings-long-tail.md`](../handoff/2026-10-25-settings-long-tail.md).
The owner's order:
1. Okur's settings redesigned as JAWS's: what is left of its long tail (§1);
2. Microsoft Word: done but for what Word does not report, then Excel and Outlook (§2);
3. punctuation by word (§3), then the README's "Not yet" list, in its order.

Each item is found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents
become accepted divergences. old-pc always has a round queued behind the running one. Okur's own
words never use JAWS's name (AGENTS.md §1.5). A stretch is a few hours or about thirty commits;
it ends with a handoff and the next plan, and the README follows a finished area. A figure in a
commit or handoff is copied from the tool's own line, never typed.

## 0. old-pc

- Idle. okur-lab parity at `3ccd08e2`: "JAWS parity (Ia2): 8903/9478 steps match exactly; 251
  accepted divergences not counted".
- **Queue next:**
  - **Settings Center's F6 pane:** F6 on a setting of each kind (a choice, a check box, a number
    such as Mouse Echo Delay, a combo box) and Tab in the pane, then F6 back; nothing changed,
    Escape out (`settings-questions-walk.ps1`).
  - **Settings Center's search results:** Down into the list, Up and Down in it, Space on a
    result, Enter, and Escape from the list (the list box's own words).
  - **Word's other views:** Web Layout and Read Mode, by the ribbon's View tab or the status
    bar, with Say Window Title in each.
  - **Which voice context JAWS uses** for the virtual cursor, forms mode and its own messages:
    a test voice profile with a pitch per context, the speech recorder's pitches read, then the
    profile deleted (old-pc's rate stays 90).
  - **Okur's dialogs as Okur reads them:** a scenario family that runs Okur on old-pc and records
    its speech of its own dialogs (now the Startup Wizard too).
  - **Excel (521): the owner's.** Its three processes and Office's activation.

## 1. Settings, the long tail

From [`../references/okur-settings-redesign.md`](../references/okur-settings-redesign.md) §6:
1. **F6 to the setting's control:** JAWS's configuration display pane, needed for a number
   ("Press F6 to switch between the tree view and this pane. This is necessary when typing in
   an edit or edit spin box", the help); okur-ui shows the selected setting's group of
   controls, F6 there and back. Recorded (the design's §1.1, `166c3be3`).
2. **The search's results as JAWS's list box** ("6 Search Results List box | Screen Echo Echo
   Highlighted Text | User"), in place of the filtered tree. Recorded (the design's §1.1);
   okur-ui needs to show one control in another's place first.
3. **Voices:** Speak Sample and Synthesizer Language in Voice Adjustment; Apply's question
   whether to make a saved profile the default. Done: the Language menu's Voice Profiles
   (`166c3be3`), a Say All's parts at the context's pitch (`eedcea21`).
4. **The Startup Wizard's rest:** this user's start after sign-in is done (the startup
   proposal's M4 resolver); left are Run Okur from System Tray, Tutor Messages' "Announce
   Custom Messages Only" and Access Key's "Speak Menus Only" and "Speak Dialogs Only" (in
   Basics too), each with its behavior, and Basics' "Automatically Start Okur..." button.
5. **Smaller:** the lifetimes' icons beside Quick Settings' items; whether JAWS reads okur-ui's
   check list as JAWS's "List box" (a probe).

## 2. Word, then Excel and Outlook

- **Line spacing** in Say Font: Word's UI Automation has no line spacing. Find another source
  that is not Word's object model, or leave it as a documented gap.
- **Web Layout and Read Mode:** their names, once recorded (§0).
- **Insert+F7** in Word, unrecorded.
- **1038's replay start:** the recorder should take a text read after the reset (or the tracer
  a caret event), then 1038 is recorded again.
- **Excel** once 521 records; **Outlook**: whether old-pc's opens without an account.

## 3. Punctuation by word, the rest

- The Okur cursor's words (`cursors.rs`) and the Virtual Viewer's: record first.
- Say Word on classic Notepad's `:\` stop (JAWS "\", Okur ":\").
- A live check in Windows 11 Notepad on the dev box, with the owner's leave.

## 4. Still open from before

Plan 29 §3-§7:
- Convenient OCR's rest;
- Smart Navigation and Screen Layout;
- the replay's speech clock;
- the hardening review's open findings;
- the backlog;
- the owner's items on old-pc.
