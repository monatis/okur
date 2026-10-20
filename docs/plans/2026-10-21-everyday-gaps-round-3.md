# Plan: everyday-use gaps, round 3 (2026-10-21)

Follows [`../handoff/2026-10-20-everyday-gaps-round-2.md`](../handoff/2026-10-20-everyday-gaps-round-2.md).
The list stays [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md):
the gaps are closed before macOS. Each item is found in JAWS's documentation, recorded on
old-pc, then built; JAWS's accidents become accepted divergences. old-pc always has a round
queued behind the running one.

## 0. First: what old-pc recorded meanwhile

The rounds queued at the end of round 2, in order (each behind the one before):

| Round | What | Then |
|---|---|---|
| 53 | 548-555: the forms fixture, a message box and the sign-up form at Beginner, Intermediate and Advanced verbosity | failed; recorded again in 58 |
| live11 | Okur reading 222-260 and 274: its own list dialogs, live (`okur live-parity`) | compare with the replay of the same steps |
| 54 | 246-274 again, with the elevated tracer and the lab's Chrome | import; most element-list steps should now replay |
| 55 | 275-299, 19, 20, 29 again | import |
| 56 | the 3xx desktop scenarios whose traces caught a browser title, and 524, 525, 532, 537, 540-546; installs `okur.exe` 06ec2c92, whose tracer adds MSAA keyboard shortcuts and the application's product and version | import; Insert+Tab's access keys (541/s8, 543/s2) and Ctrl+Insert+V (513/s4, in 57) should then replay |
| 57 | 401-408 and the 5xx scenarios with a browser title in their traces | import |
| 58 | 548-555 again (53 failed: Say Window outlasted its settle; now 60 s, and a failed run keeps its output in `<id>-run.log`); installs `okur.exe` 3621e5dd (built at `04ce857`: also spelling errors in field reads) | build the levels (§1) |
| 59 | 237 (missed in 52), 284 again (its spelling errors traced); 556-557 (Shift+Period and Shift+Comma outside lists), 558 (S and D on a plain page), 559 (where Jump to Line goes), 560 (the Word List), 561 (the clipboard keys, the Windows-key scripts by name), 562 (a large selection), 563 (following aria-flowto), 564 (Say Line on form fields), 565 (Say Color on colors.html), 566 (Speech On Demand), 567 (spelling errors in a field of several words) | build each; 562 sets the "N characters up to" rule |
| snap60 | virtual-keys, colors and news-article captured through IA2 again, with relations and text colors (`okur.exe` 36671d07) | import the snapshots: Has Flows To, Equals and Say Color replay |
| live12 | Okur reading live: its list dialogs (222-260, 274) and round 3's features (205, 284, 293, 294, 323, 513, 515, 516, 534), `okur.exe` 87fadfd0 | `okur live-parity` against live11 and the replay |
| 61 | 568 (JAWS's text analysis on Insert+Tab, a new Notepad fixture), 569 (Route PC to Virtual on elements that cannot take the focus) | build each |
| 62 | 516 and 546 again with `okur.exe` 71c6fe60 (built at `d4b5da4`: DPI aware, so positions are real pixels; an edit's up-down buddy; `okur tray`), 570 (the Speech History's Ctrl+H and Shift+H), 571 (Insert+F4's question, last) | import: 516's cursor points and 546/s8's spin box should replay; build 570 and 571; then `okur tray` on old-pc against 511/s4's six icons |

For each: fetch with `lab-fetch.sh` (a stalled fetch is stopped and started again), grep the
evidence and traces for links, the lab account's folders and the owner's name, check the
capture-script hash, compare parity before and after, and commit the corpus alone. A re-record
may change a step's status; say which in the commit.

## 1. Verbosity levels

- JAWS's shipped Default.JCF gives each output type per level ([OutputModes]: Beginner,
  Intermediate, Advanced; 0 off, 1 long, 2 short) and the web verbosity levels
  ([VirtualCursorVerbosity]: High, Medium, Low, per element type). Okur's
  `settings::OutputType` has two types; extend it to the types those recordings exercise
  (control name and type, item state, position, dialog name and text, tutor, access key,
  group name), with the shipped defaults per level, checked against 548-555.
- A key or a Quick Settings item to change the level; JAWS's Startup Wizard and Settings
  Center are where users set it.

## 2. The list dialogs, live and replayed

- live11 says what Okur's own dialogs say live; compare it with the replay of the same steps.
- What still differs after round 54's re-records: the dialog said twice as it opens (JAWS's own
  variant), Virtual HTML Features (230), the Enter variants (246-260).
- Jump to Line (J, Shift+J): JAWS's dialog "Enter Line Number (1 to N):" [299]; an Okur dialog.

## 3. Reading and quick keys

- S and D (same and different element), Ctrl+Insert+Right, Left and Up [290]: what JAWS counts
  as an element's type needs a recording of its own on a simpler page.
- Step past and before an element (Shift+Period, Shift+Comma, Alt+Win+End and Home) [294].
- Say All in replay: steps stopped by a timed key cannot match without a speech clock; a model
  of JAWS's speaking pace (words per second at old-pc's rate) would let replay stop where JAWS
  stopped (500, 538, 539).

## 4. Windows and dialogs

- Say Window and group boxes: JAWS says a group box as "Direction group" and each control in
  it with the group's name first, radio buttons with their position ("Direction Up radio
  button not checked 1 of 2"; 540/s4, 544/s13). The traced dialog tree is flat, and order
  alone cannot tell membership (Page Setup's Header and Footer follow its Margins group without
  being in it): the tracer should record each control's group box from the controls'
  rectangles, as `group_box_of` does for the focus, and stage that okur.exe for the next round.
- Say Window in a message box and Page Setup: JAWS names an unlabelled icon or picture after
  the dialog or the label before it (537/s8, 544/s13); find the rule (MSAA's names of those
  statics, which the tracer could record).
- WordPad's edit after its ribbon: "edit | Contains text | Type in text." with no name
  (523/s6), and its ribbon positions (523/s1-s4).
- Toasts (533): still unread by the tracer and by Okur; JAWS likely uses Windows' notification
  listener.

## 5. Owner questions

- Office on old-pc still needs activating; 518, 519 and 521 wait for it.
