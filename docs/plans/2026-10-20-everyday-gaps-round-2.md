# Plan: everyday-use gaps, round 2 (2026-10-20)

Follows [`../handoff/2026-10-19-everyday-gaps-round-1.md`](../handoff/2026-10-19-everyday-gaps-round-1.md).
The list stays [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md):
the owner's decision is to close those gaps before macOS. Each item is found in JAWS's
documentation, recorded on old-pc, then built; JAWS's accidents become accepted divergences.
old-pc always has a round queued behind the running one.

## 0. First

- Fetch and check rounds 38 to 40 and live6 (524, 525, 532 with the file dialog fixture; 540 to
  543, Notepad's Find, Font, Replace and Go To dialogs). Before importing anything, grep its
  evidence and traces for links, the lab account's folders and the owner's name.
- The owner's answers to the handoff's two questions decide what happens to the recordings held
  back (530, 531, 534 to 539) and to Office (518, 519, 521). If the fixtures get a Chrome of their
  own, `run-scenario.ps1` starts it with `--user-data-dir`, and the web scenarios are recorded
  again, a few rounds at a time.

## 1. Windows and dialogs

- The Window List (Insert+F10) and the system tray list (Insert+F11), as Okur dialogs: the
  running applications' windows, Enter switches to one (511). Okur has neither.
- A dialog box whose focus is not on a button: its default button (DM_GETDEFID on Win32
  dialogs), from 532; Say Window in a message box (537/s8).
- The file dialogs (524, 525) once round 38 is in.

## 2. Notifications and progress

- Toasts: the tracer and the live platform watch the shell's notification window, so that a
  toast is said as it arrives ("New notification from Windows PowerShell, SR Fixture, Build
  finished in 12 seconds. 1 of 1"); Repeat Last Notification (Insert+Space, Shift+N) and a
  Notification History of Okur's own (533). Then 533 is recorded again with the new tracer.

## 3. Reading

- Say All: the unit (line, sentence, paragraph) and the Shift keys; JAWS joins short lines into
  one utterance with bookmarks, which may close the gap 500/s8 and s11 show.
- The page-load summary and Say All on load (507, 538, 539: "Page has 1Region, 1heading and
  1link", then reading from the top).
- The window's bottom line in WordPad: its status bar, found below the window's own children.
- A NetUI ribbon's positions ("View" 1 of 2, "Home" 2 of 2 in JAWS, reversed from the tabs'
  order) and "Leaving ribbons" (523).
- Skim reading (515, 530) and the table layer (535) once their recordings are in.

## 4. Help

- Insert+F1, Insert+H and Insert+W with Okur's own texts (JAWS's are Freedom Scientific's): the
  wording is Okur's, the keys and where the text is shown follow JAWS (526, 527).

## 5. The JAWS cursor and positions

- 516 (the JAWS cursor) and 513's Alt+Delete need the focus's and the caret's positions on the
  screen, which the platform does not trace yet; Numpad Slash outside a page clicks at the mouse
  pointer, which waits for the same.

## 6. Application information

- Ctrl+Insert+V: the application's product name and version, through a platform event (the
  foreground process's version resource).
- Verbosity levels: find on old-pc which JCF key holds the level (change it in Quick Settings and
  compare the files), then record Intermediate and Advanced.
