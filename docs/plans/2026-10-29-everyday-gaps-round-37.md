# Plan: everyday-use gaps, round 37 (2026-10-29)

Follows [`../handoff/2026-10-29-settings-results-and-word-keys.md`](../handoff/2026-10-29-settings-results-and-word-keys.md).
The owner's order, unchanged from plan 36:
1. Okur's settings redesigned as JAWS's: what is left of its long tail (§1);
2. Microsoft Word: what is left, then Excel and Outlook (§2);
3. the review's open findings (§3), punctuation by word (§4), then the README's "Not yet"
   list, in its order.

Each item is found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents
become accepted divergences. old-pc always has a round queued behind the running one. Okur's own
words never use JAWS's name (AGENTS.md §1.5). A stretch is a few hours or about thirty commits;
it ends with a handoff and the next plan, and the README follows a finished area. A figure in a
commit or handoff is copied from the tool's own line after the tool ran, never typed first.

Before every round: the voice profile's `Rate=` lines on old-pc (or the `jaws_settings` rates of
a fetched evidence file) must read 90; `fix-rate.ps1` otherwise. old-pc's master volume stays at
1%, not muted (the owner, 2026-10-28).

## 0. old-pc

- Round-1071b (1071 again with the alignment saved; 1074, Word's font size keys; 1075, its
  heading style keys): fetch, preview, build what it shows, import.
- Then push the recorder that puts the clipboard back (`7db91a1f`) and record 1073 (Copy and
  Paste in Word) again.
- Queue next, one round behind the other: Word's comments and footnotes as JAWS reads them
  (its documentation first); Word's page and section keys on a longer fixture; Settings
  Center's F6 on a result and Quick Settings' search (not walked yet).
- **Excel (521): the owner's** (its three processes on old-pc).

## 1. Settings, the long tail

1. **Voices:** Apply's question whether to make a saved profile the default (record first);
   Synthesizer Language.
2. **The Startup Wizard's rest:** Run Okur from System Tray; Basics' "Automatically Start
   Okur..." button. Tutor Messages' "Announce Custom Messages Only" and Access Key's "Speak
   Menus Only" and "Speak Dialogs Only" are built from the documentation (2026-10-29), and how
   JAWS's file writes them is learned; record their behavior (a scenario's `jaws_settings`
   with `ACCESS_KEY=2|2|2` over a menu and a dialog).
3. **Smaller:** the lifetimes' icons beside Quick Settings' items.
4. **A live look** at Settings Center's results list and pane name on screen, with the owner's
   leave on the dev box.

## 2. Word, then Excel and Outlook

- Done: the Styles gallery, Find and Replace (modeless), Paragraph, the Font and Size lists,
  Track Changes, the alignment keys; Ctrl+B, I, U and Undo, Redo match as recorded.
- **The Spelling pane** (1069, pending): Word's task pane is taken for the ribbon (every NetUI
  focus is `in_ribbon`); JAWS says the document's name on Escape back. Its first focus differs
  by trial in JAWS itself: record with a steadier start (Tab to a known control first), then
  build.
- **Comments, footnotes, page and section keys:** comments built (1081: marked in a line,
  Alt+Shift+Apostrophe). Pending: JAWS's comments and footnotes lists (1082 Ctrl+Shift+Apostrophe,
  1083 Insert+Shift+F, desktop-word-notes.ps1): their first step is steady ("Reviewers' Comments
  dialog | List1 List view | Check the summer hours. Lab Fri, Jan 02, 26 | 1 of 1"), but after
  JAWS's own list dialog every step's speech is "Speech history cleared" (the recorder's clear
  spoken, the keys silent): fix the recorder first. Page and section keys need a longer fixture.
- **Insert Hyperlink** (1076, pending): JAWS reads the Link to bar's internal name ("MSO Generic
  Control Container") and "Existing File or Web Page Button Pressed"; Okur has no Pressed state
  and drops the MsoCommandBar focus. The dialog's silent opening and closing are like 1060/s4.
- **Save As** (1077) and **indent** (1080): recorded again in round-1080d, to import.
- **Excel** once 521 records; **Outlook**: whether old-pc's opens without an account.

## 3. The review of 2026-10-25, what is open

[`../backlog/performance-review-2026-10-25.md`](../backlog/performance-review-2026-10-25.md):
6 (a slim control kind in okur-ui), 7 (the settings write off the engine's thread), 8
(okur-ui's item index per tree), 9 (the host's restart of okur-ui told to the engine), 10 (a
live check from the installed lowered build), 14 (live), 18, 19, 20.

## 4. Punctuation by word, the rest

The Okur cursor's words (1062, recorded, pending: JAWS splits "C:\Users\Ada\Downloads" at each
backslash where Okur says the whole path); Say Word on classic Notepad's `:\` stop; a live check
in Windows 11 Notepad with the owner's leave.

## 5. Still open from before

Plan 29 §3-§7: Convenient OCR's rest; Smart Navigation and Screen Layout; the replay's speech
clock; the earlier reviews' open findings; the backlog; the owner's items on old-pc.
