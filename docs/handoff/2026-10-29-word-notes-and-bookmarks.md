# Handoff: Word's notes and bookmarks, JAWS's own list dialogs recorded (2026-10-29, third)

Follows [`2026-10-29-word-keys-comments-access-keys.md`](2026-10-29-word-keys-comments-access-keys.md)
and covers the commits after it (`d6d1a88f` to `b2bd14ef`), made under
[`docs/plans/2026-10-29-everyday-gaps-round-38.md`](../plans/2026-10-29-everyday-gaps-round-38.md).
The next plan is [`docs/plans/2026-10-29-everyday-gaps-round-39.md`](../plans/2026-10-29-everyday-gaps-round-39.md).

Where things stand:
- **Word:** JAWS's lists of its comments, footnotes, endnotes and bookmarks are built and
  recorded, with Alt+Shift+E and the notes' references said in a line. The Spelling pane,
  Insert Hyperlink and Save As are recorded with their dialogs' speech. Open: the
  Virtual Viewer of notes (Windows+Semicolon, Alt+Shift+Apostrophe twice: recording), Insert
  Hyperlink's Pressed button, Save As's Tab and Escape, superscript said at a note's
  reference; Excel and Outlook.
- **The recorder** reaches JAWS's own list dialogs (they run elevated) with `key_ui_access`;
  JAWS's Virtual Viewer takes plain keys.

## What was built

### Word's lists of notes and bookmarks (`d6d1a88f`, `eee7aeb2`)

Each list is read from Word's object model when its key is pressed and shown as JAWS's dialog
is: list "List1", OK (the default) and Cancel [obs, 1087/s2-s5, 1088, 1090, 1091].
- **Ctrl+Shift+Apostrophe**, "Reviewers' Comments": "Check the summer hours. Lab Fri, Jan 02,
  26", the comment, its author and its date in the user's locale's "ddd, MMM dd, yy" [obs,
  1082/s1]. Elsewhere the key is the application's.
- **Insert+Shift+F**, "Footnotes", and **Insert+Shift+E**, "End Notes": "1 Renewals are free."
  [obs, 1083/s1, 1090/s1].
- **Insert+Alt+B**, "Bookmark List", in the document's order (Word's collection is by name):
  "Opening nine in the morning" [obs, 1091/s1].
- **OK** puts the caret on the note's mark and says "Edit", after the note by its mark for a
  footnote or endnote ("end note reference: i Renewals are free.") and after the text for a
  bookmark ("three weeks") [obs, 1087/s6, 1088/s6, 1090/s6, 1091/s9].
- **None:** "There are no comments in the document.", "There are no end notes in the
  document." [obs, 1089, 1093].

### Notes in a line and at the caret (`d6d1a88f`, `eee7aeb2`)

The paragraph's footnote and endnote references are part of the line's structure, asked of
Word only for a document that has any: a line says them where they stand, "It is open every
day footnote 1.", "three weeks end note i." [obs, 1083/s4, 1090/s7]. A note's mark follows its
collection's numbering style (Word's endnotes in lower-case roman). Alt+Shift+E reads the note
whose reference Word's caret is on, "end note reference: i Renewals are free.", or "No footnote
or EndNote reference at cursor." [obs, 1092]; Left onto a reference says it, "end note i".

## Harness

- **Keys into JAWS's own dialogs:** its list dialogs run elevated and refused the recorder's
  keys, so every step after one spoke only "Speech history cleared" (round-1080c). Those keys
  are `key_ui_access` steps now. JAWS's Virtual Viewer is different: plain keys reach it, keys
  with UI Access do not (round-1094).
- **Slow first steps:** JAWS's main window often speaks before its list dialog, past the 10 s
  settle; those steps wait up to 20 s (`settle_max_ms`). Word's own dialogs (Insert Hyperlink,
  Save As, the Spelling pane) take longer than 800 ms to speak: `onset_ms` 5000 on the key that
  opens them, 3000 on Escape. Without it their speech was lost between steps.
- **The snapshot** saves the document's comments, footnotes, endnotes and bookmarks, which
  replay answers the lists with.
- **desktop-word-marks.ps1**, a document with two bookmarks, an endnote and a footnote.

## Found, and how

- **Insert+Shift+F fell through** to the virtual cursor's Shift+F ("There are no form fields on
  this page.") when its check came after the key map's: the key map's Shift chords match with
  Insert held. Word's Okur-key chords are checked first.
- **The preview script runs `target/release` by default:** two previews ran an old build.
  Pass `target/debug/okur-lab.exe` as its second argument.
- **A new fixture makes the manifest stale** for a code commit: the gate blocked one. The
  fixture went aside for that commit and came back with its evidence.
- **Endnotes' marks are not their numbers:** JAWS lists "1 Renewals are free." but says "end
  note reference: i" and "end note i" in a line.

## Proof

`okur-lab parity` at `b2bd14ef`:

```
JAWS spoken parity: 9258/9993 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 9361/9932 steps match exactly; 282 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `ac55f505`: 9282/9846, 280 accepted divergences. `okur-lab parity --only`
for this stretch's scenarios, each line as printed:

```
1069: JAWS parity (Ia2): 5/6 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1076: JAWS parity (Ia2): 5/6 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1077: JAWS parity (Ia2): 2/5 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1082: JAWS parity (Ia2): 4/4 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1083: JAWS parity (Ia2): 5/5 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1087: JAWS parity (Ia2): 8/8 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1088: JAWS parity (Ia2): 8/8 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1089: JAWS parity (Ia2): 4/4 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1090: JAWS parity (Ia2): 8/8 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1091: JAWS parity (Ia2): 11/11 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1092: JAWS parity (Ia2): 13/15 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1093: JAWS parity (Ia2): 6/6 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace --no-fail-fast`: 1542 passed, 0 failed, 200 ignored (summed from
cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
`okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's statistics: 718
scenarios, 2160 trials, 2143 succeeded.

What differs, and why it is open (plan 39):
- 1077/s1: JAWS says the file name again after the hint (2 trials of 3; the third lost its
  speech); s2: replay hears "File name: edit combo ... 1 of 17", the MSAA focus on Save as
  type joined to UI Automation's chain still on File name; s4: back in Word JAWS says
  "Microsoft Word Document | (the combo box's hint) | Edit", Okur the window's title.
- 1076/s5: the Pressed button in the dialog's command bar (MSAA only; UIA stops at the dialog).
- 1069/s4: JAWS says a suggestion twice, "the | 1 of 5 | the".
- 1092/s8, s10: "superscript" and "end superscript" as the selection or caret crosses a
  note's reference.

Not checked live: any of this in Okur's own run on the dev box (it needs the owner's leave).

## old-pc

- Recording round-1095b: 1095 (Alt+Shift+Apostrophe twice, the comment in the Virtual Viewer,
  plain keys), 1096 and 1097 (Windows+Semicolon, Enter on Comments, on Footnotes). 1094
  (Windows+Semicolon's dialog) is recorded and pending: "Items to Display in Virtual Viewer
  dialog | List1 | Comments | 1 of 4", OK and Cancel.
- Installed: push `c0630af8` (the endnotes and bookmarks build). JAWS at Rate=90, volume 1%.
- Still the owner's: Excel's three processes, the windowless Word, the PowerShell; the owner's
  "Document1" is open.

## Next

Plan 39: the Virtual Viewer of Word's notes (1094-1097), then Save As and Insert Hyperlink's
open steps, superscript at a reference; the settings' long tail; the review's open findings.
