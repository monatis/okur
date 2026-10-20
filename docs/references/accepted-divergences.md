# Accepted divergences: where Okur differs from JAWS on purpose

Okur matches what JAWS 2025 means to do, not its accidents. When JAWS says something as a side
effect of how it is built, or gets a fact wrong, Okur keeps the right answer. Each such difference
is a decision, written here and in the table that the parity tools apply. Nothing else may
differ silently.

## How a divergence is marked

The table is `DIVERGENCES` in `crates/okur-corpus/src/divergences.rs`, written with the
`divergences!` macro:

```rust
divergences! {
    Difference::JawsAlsoSays("Web App Patterns - Google Chrome") => [
        "226-web-element-lists/s1",
    ],
    "why",
    "who decided, when";
}
```

An entry marks only what differs, so the rest of a step is still compared:
- `JawsAlsoSays(phrase)`: JAWS says the phrase and Okur does not. The phrase is taken out of
  JAWS's transcripts before they are compared.
- `OkurAlsoSays(phrase)`: Okur says the phrase and JAWS does not. It is taken out of Okur's
  speech before it is compared.
- `Step`: the whole step differs. It is not counted.

Every comparison with JAWS applies the table: `okur parity` on all three layers (speech history,
spoken text, the UIA buffer), the parity baseline (whole steps are "accepted divergence") and
`okur live-parity`. Their totals read, for example, "1143/1247 steps match exactly; 8 accepted
divergences not counted".

One rule holds for the whole corpus and is not a divergence: JAWS's name for itself reads as
Okur's. Where JAWS names itself, Okur names itself (AGENTS.md §1.5): "JAWS Cursor" is "Okur
Cursor", "Route JAWS to PC" is "Route Okur to PC", "JAWS Find" is "Okur Find". `comparable` in
`crates/okur-corpus/src/parity.rs` reads both sides of every comparison so (the whole word "JAWS",
in JAWS's capitals, as "Okur"), and the replay of JAWS's own dialogs as Okur's
(`crates/okur-corpus/src/dialog_mirror.rs`) matches their titles the same way. It lists no step,
and the list below does not change for it.

Guards (`cargo test -p okur-corpus`):
- `jaws_parity_matches_baseline` fails on a stale entry: a step that no longer exists, a phrase
  JAWS or Okur no longer says, or a whole step that now matches JAWS.
- `the_written_list_names_every_divergence` fails when this page does not name a step the table
  covers.

A divergence is proposed with its evidence and decided by the project's owner. An entry decided
under one of the owner's rulings says which.

## The list

### JAWS passes through the browser's window on the way to its dialog

- Step: `226-web-element-lists/s1` (also says "Web App Patterns - Google Chrome", the browser
  window's title).
- Why: opening Virtual HTML Features, JAWS's speech passes through a window the user never
  visits. The same keys recorded again (230/s1) have none of it.
- Decided under the owner's ruling on JAWS's version, 2026-10-18.

A step can be in more than one entry; all of its entries apply.

### What JAWS says before naming its own dialog

- Difference: `JawsSaysBeforeItsDialog`. JAWS's speech before the line that names its dialog
  ("Select a Button ‹ dialog ›") is left out; from that line on, the step is compared.
- Why: opening one of its list dialogs, JAWS's speech passes through its own windows before it
  names the dialog. It may say its main window (its version, a list's tutor message), the
  browser's title, or the dialog's list before the dialog itself. Okur's dialog opens without
  them, and some trials of the same keys have none of it either.
- Steps (the step that opens the dialog, s1, of each):
  `222-web-links-list/s1`, `223-web-links-list-escape/s1`, `231-web-list-radio-buttons/s1`,
  `232-web-list-buttons/s1`, `226-web-element-lists/s3`,
  `224-web-headings-list/s1`, `225-web-form-fields-list/s1`, `226-web-element-lists/s1`,
  `227-web-form-fields-list-items/s1`, `228-web-form-fields-list-edit/s1`,
  `230-web-element-lists-items/s1`, `233-web-list-combo-boxes/s1`, `234-web-list-edits/s1`,
  `235-web-list-check-boxes/s1`, `236-web-list-graphics/s1`, `239-web-list-paragraphs/s1`,
  `240-web-list-block-quotes/s1`, `241-web-list-regions/s1`, `242-web-list-tables/s1`,
  `243-web-list-glance-highlights/s1`, `244-web-list-divisions/s1`, `245-web-list-tabs/s1`,
  `247-web-list-buttons-enter/s1`,
  `248-web-list-combo-boxes-enter/s1`, `249-web-list-edits-enter/s1`,
  `250-web-list-check-boxes-enter/s1`, `251-web-list-graphics-enter/s1`,
  `252-web-list-lists-enter/s1`, `254-web-list-paragraphs-enter/s1`,
  `255-web-list-block-quotes-enter/s1`, `257-web-list-tables-enter/s1`,
  `258-web-list-glance-highlights-enter/s1`, `259-web-list-divisions-enter/s1`,
  `260-web-list-tabs-enter/s1`; and Insert+F11's Select a System Tray Icon,
  `511-desktop-window-list-and-tray/s4` (its version, round 57); and JAWS Find,
  `506-web-jaws-find/s1` (its Find what field said before the dialog, in some trials).
- In the recordings of 2026-10-19 (round 9, JAWS freshly started for the batch), JAWS no longer
  said its version or the list's tutor message before the Links List, the Heading List and
  Virtual HTML Features, and said nothing before some dialogs at all: those steps were taken
  off this list and the entry for the version removed.
- Round 52 (2026-10-20) recorded 221-245 again in the lab's own Chrome, and JAWS said them again
  before the Links List (222, 223), Select a Radio Button (231) and Select a Button (232): those
  steps were put back, with `226-web-element-lists/s3`, where Enter opens the Heading List from
  Virtual HTML Features. On the spoken layer, where JAWS's speech has no line breaks and no
  role marks, the step is compared from the last place JAWS says what Okur's speech begins
  with, the dialog's title and "dialog".
- Round 54 (2026-10-20) recorded 246-274 again: JAWS said nothing before Select a Radio Button
  in 246, so that step left the list.
- Decided under the owner's ruling on JAWS's version, 2026-10-18.

### A stray comma in Virtual HTML Features

- Step: `230-web-element-lists-items/s18`.
- Difference: JAWS also says "," after "Radio Buttons List".
- Why: Virtual HTML Features (Insert+F3) shows "Radio Buttons List," where every other list is
  named "... List". It is a typo in JAWS's text; Okur shows "Radio Buttons List".
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### JAWS's text analysis names spaces past a line's end

- Steps: `614-desktop-text-analysis-rules/s23`, `614-desktop-text-analysis-rules/s27`.
- Difference: JAWS also says "Space run at column 12" after "Hello!World" and "Space run at
  column 17" after "Mixed Case wORD.", one column past each line's end.
- Why: neither line has a run of spaces, and JAWS names none past the end of the fourteen
  other lines of the same fixture, which end the same way. Okur names the runs a line has.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-20.

### JAWS says it is unloading before it asks

- Step: `571-desktop-quit-question/s1`.
- Difference: JAWS also says "Unloading JAWS" before its question "Quit JAWS dialog | Are you
  sure you want to quit JAWS?".
- Why: JAWS has not begun to unload: answered no (Escape), it stays (571/s3). Okur asks
  first and says its exit words only as it exits.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-20.

### JAWS's control view drops an emoji

- Steps: `312-desktop-virtualize-control/s3`, `/s4`, `/s9`, `/s14` (the full keys:
  `312-desktop-virtualize-control/s3`, `312-desktop-virtualize-control/s4`,
  `312-desktop-virtualize-control/s9`, `312-desktop-virtualize-control/s14`).
- Difference: Okur also says the grinning face emoji at the end of the line.
- Why: Virtualize Current Control (Insert+Shift+V) on the editor fixture drops the
  non-breaking space and the emoji that end its last line. JAWS reads the emoji when it reads
  the field itself [obs, 313/s4]. Text copied from Okur's viewer keeps it; from JAWS's, it is
  lost.
- Proposed by the parity gap audit; accepted by the owner, 2026-10-19.

### JAWS's bottom line drops an emoji

- Steps: `504-desktop-edit-selection-and-clipboard/s22`, and the same step at Intermediate
  and Advanced verbosity, `599-desktop-edit-selection-and-clipboard-intermediate/s22` and
  `609-desktop-edit-selection-and-clipboard-advanced/s22`.
- Difference: Okur also says the grinning face emoji at the end of the line.
- Why: Say Bottom Line of Window (Insert+PageDown) in the editor fixture drops the
  non-breaking space and the emoji that end the last line, as its control view does (312).
  JAWS reads the emoji when it reads the field itself [obs, 313/s4]; Okur reads the line
  whole.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### A radio button's place after JAWS's mouse click

- Step: `534-web-numpad-slash-check-box/s10`.
- Difference: Okur also says the radio button's place in its group, "2 of 3".
- Why: after Numpad Slash (the left mouse button) checked the radio button, JAWS's Say Line
  said it without its place in the group in all three trials. Say Line on a radio button says
  the place otherwise (246/s4), as Down onto one does (534/s8). Okur says it.
- Decided under the owner's rule on JAWS's accidents, 2026-10-21.

### JAWS counts its Smart Glance highlights after a page's summary

- Steps: `507-web-page-load-and-numpad-plus/s1` ("There are 2 Smart Glance Highlights on
  this page."), `538-web-page-load-summary-news-article/s1` ("There are 4 ..."),
  `539-web-page-load-summary-tables-navigation/s1` ("There are 1 ...").
- Difference: JAWS also says "There are N Smart Glance Highlights on this page.".
- Why: Smart Glance highlights are the content JAWS's own heuristic picks as a page's most
  important; no observation defines them, and Okur does not have them. Okur's page-load
  summary stops at the regions, headings and links.
- Decided by the agent with the gaps report's row 20, 2026-10-20; to be revisited if Okur
  gets such highlights.

### The help keys in Okur's own words

- Steps: `526-desktop-screen-sensitive-help/s1`, `526-desktop-screen-sensitive-help/s5`,
  `526-desktop-screen-sensitive-help/s10`, `526-desktop-screen-sensitive-help/s13`,
  `526-desktop-screen-sensitive-help/s21`, `527-desktop-notepad-help-keys/s1`,
  `527-desktop-notepad-help-keys/s3`, `527-desktop-notepad-help-keys/s5`,
  `531-web-screen-sensitive-help/s2`, `531-web-screen-sensitive-help/s5`,
  `531-web-screen-sensitive-help/s8`; and 526's steps again at Intermediate and Advanced verbosity,
  `601-desktop-screen-sensitive-help-intermediate/s1`, `/s5`, `/s10`, `/s13`, `/s21` (in full:
  `601-desktop-screen-sensitive-help-intermediate/s5`,
  `601-desktop-screen-sensitive-help-intermediate/s10`,
  `601-desktop-screen-sensitive-help-intermediate/s13`,
  `601-desktop-screen-sensitive-help-intermediate/s21`), and
  `611-desktop-screen-sensitive-help-advanced/s1`,
  `611-desktop-screen-sensitive-help-advanced/s5`,
  `611-desktop-screen-sensitive-help-advanced/s10`,
  `611-desktop-screen-sensitive-help-advanced/s13`,
  `611-desktop-screen-sensitive-help-advanced/s21`; and Keyboard Help's descriptions,
  `634-desktop-keyboard-help-spoken/s2`, `634-desktop-keyboard-help-spoken/s3`,
  `634-desktop-keyboard-help-spoken/s4`, `634-desktop-keyboard-help-spoken/s5`,
  `634-desktop-keyboard-help-spoken/s6`, and the second presses of 642,
  `642-desktop-keyboard-help-second-press/s2` to `/s6` (in full:
  `642-desktop-keyboard-help-second-press/s2`, `642-desktop-keyboard-help-second-press/s3`,
  `642-desktop-keyboard-help-second-press/s4`, `642-desktop-keyboard-help-second-press/s5`,
  `642-desktop-keyboard-help-second-press/s6`) (steps of JAWS's speech alone: Keyboard Help
  describes the capture's history scripts instead of running them).
- Difference: the whole step: the help's words.
- Why: JAWS's help texts (screen-sensitive help, Insert+F1; hot key help, Insert+H; Windows
  keys help, Insert+W) are Freedom Scientific's writing. Okur's help keys open the Virtual
  Viewer as JAWS's do, with the "List JAWS Hot Keys" link and "Press ESCAPE to close this
  message.", and Escape reads the focus as JAWS does (526/s6, s11, s14; 527/s2, s4, s6). The
  help is in Okur's own words, and Insert+H lists Okur's own keys, each a link that Enter
  uses. In a page the help is about where the virtual cursor is, as JAWS's is: the page's
  summary, a link's address, a heading's level (531). In Keyboard Help (Insert+1) a key is
  named as JAWS names it ("Insert plus Up Arrow"), then described in Okur's words, a longer
  description when pressed twice quickly; a key Okur leaves to the application says so (634).
  Pressed twice quickly, JAWS's recorder holds the key once and the long description: the
  first press's description, queued behind its key, is flushed by the second press before
  the synthesizer gets it (642/s2); Okur's spoken record keeps every utterance it was asked
  to say.
- Decided under the clean-room rule (AGENTS.md §1.4), 2026-10-19.

### JAWS places a data grid's cell at column and row 65535

- Steps: `545-desktop-data-grid/s2` to `/s8`, `/s10`, `/s11` (the full keys:
  `545-desktop-data-grid/s2`, `545-desktop-data-grid/s3`, `545-desktop-data-grid/s4`,
  `545-desktop-data-grid/s5`, `545-desktop-data-grid/s6`, `545-desktop-data-grid/s7`,
  `545-desktop-data-grid/s8`, `545-desktop-data-grid/s10`, `545-desktop-data-grid/s11`).
- Difference: JAWS also says "Column 65535, Row 65535".
- Why: moving between a WinForms data grid's cells, JAWS adds it after each cell, in every
  trial. 65535 is -1 as an unsigned 16-bit number: a place JAWS failed to find. The cell's
  name already gives its column and row ("Quantity Row 1"), and Okur says nothing more.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### JAWS finds no font in a WinForms text box

- Steps: `313-desktop-say-font/s1`, `313-desktop-say-font/s2`, `313-desktop-say-font/s3`,
  `313-desktop-say-font/s4`, `313-desktop-say-font/s5`, `313-desktop-say-font/s6`,
  `313-desktop-say-font/s7`, `313-desktop-say-font/s9`.
- Difference: the whole step.
- Why: Say Font (Insert+F) in a WinForms text box gets "No font" from JAWS, though JAWS names
  Notepad's font [obs, 314/s1]. Okur says "Font is Consolas 11 point". Pressed twice, Okur then
  shows the font in the Virtual Viewer, as JAWS does when it finds one. So the scenario's later
  keys (Down, Ctrl+A, Escape) act in that viewer, not in the field, and those steps differ too.
- Proposed by the parity gap audit; accepted by the owner, 2026-10-19.

### F3 right after JAWS Find

- Steps: `506-web-jaws-find/s8`, `506-web-jaws-find/s9`, `506-web-jaws-find/s10`,
  `506-web-jaws-find/s11`, `633-web-jaws-find-next-board/s9`,
  `633-web-jaws-find-next-board/s10`, `633-web-jaws-find-next-board/s11`,
  `633-web-jaws-find-next-board/s12`, `633-web-jaws-find-next-board/s13`,
  `633-web-jaws-find-next-board/s14`.
- Difference: the whole step. JAWS says nothing on F3 and Shift+F3, and Say Line then reads
  the match's line or a line above it; Okur says the next or previous match's line.
- Why: right after JAWS Find found "board", F3 and Shift+F3 said nothing in every trial of 506
  and 633, and the virtual cursor landed on no match: it stayed on the match's line, or ended a
  line or two above it on lines without the word (633: "published March 14, 2026", then "Maria
  Lopez,"), and not the same way in both scenarios. From the top of the page F3 found the match
  and said its line (633/s16), as JAWSFindNext, which JAWS's key map binds to F3, is to: find
  the next occurrence of the text last searched for. Okur's F3 and Shift+F3 always go to the
  next or previous match and say its line.
- Decided under the owner's rule on JAWS's accidents, 2026-10-20.

### "Required" for a field that is not required (Auto Forms Mode)

- Steps: `280-web-forms-arrows-auto-forms/s7`, `632-web-field-returned-later/s10`, and 632
  at Intermediate and Advanced, `640-web-field-returned-later-intermediate/s10`,
  `641-web-field-returned-later-advanced/s10`.
- Difference: the whole step. JAWS says "Email address Edit Required | sam@example.com"; Okur
  says it without "Required". 632 arrived at the field the same way, Down from its label, and
  JAWS said "Required" there too, then not on Say Line right after (632/s11).
- Why: with Auto Forms Mode on, Down onto the sign-up form's email field said "Required" in
  all three trials. Nothing makes that field required: it has no `required` attribute, and the
  trace's focus says UIA's IsRequiredForForm and ARIA's required are both false. JAWS's Tab
  onto the same field (204/s2) does not say it. It is a wrong fact.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### Select an Article moves to the page's top

- Steps: `253-web-list-articles-enter/s3`, `253-web-list-articles-enter/s4`,
  `253-web-list-articles-enter/s5`.
- Difference: the whole step. JAWS says "banner" on Enter and on Say Line, then Down reads
  the banner's link; Okur says "article", then the article's heading.
- Why: Enter on the article chosen in Select an Article (Ctrl+Insert+O) took JAWS to the
  page's top in every trial, not to the article. The list is for moving to an article, and
  JAWS's other lists move to what was chosen (252, 254, 255).
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### Down out of a text area jumps to the page's end (Auto Forms Mode)

- Steps: `280-web-forms-arrows-auto-forms/s21`, `280-web-forms-arrows-auto-forms/s22`,
  `280-web-forms-arrows-auto-forms/s23`, `280-web-forms-arrows-auto-forms/s24`,
  `280-web-forms-arrows-auto-forms/s25`, `280-web-forms-arrows-auto-forms/s26`.
- Difference: the whole step. JAWS says "main region end" six times; Okur reads the check
  boxes, the link and the buttons that follow the text area.
- Why: Down out of the empty text area took JAWS to the page's last line, skipping six lines,
  and Up from there read them (s29–s33). JAWS's documentation has the arrow leave a field for
  the page's next line ("Using Forms with JAWS", Surf's Up), which JAWS did for the other
  fields (s6, s9, s12) and Okur does here too.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### Up out of a text area jumps to the page's top (Auto Forms Mode)

- Steps: `282-web-forms-arrows-auto-forms-up/s8` to `282-web-forms-arrows-auto-forms-up/s25`
  (18 steps: `282-web-forms-arrows-auto-forms-up/s8`, `282-web-forms-arrows-auto-forms-up/s9`,
  `282-web-forms-arrows-auto-forms-up/s10`, `282-web-forms-arrows-auto-forms-up/s11`,
  `282-web-forms-arrows-auto-forms-up/s12`, `282-web-forms-arrows-auto-forms-up/s13`,
  `282-web-forms-arrows-auto-forms-up/s14`, `282-web-forms-arrows-auto-forms-up/s15`,
  `282-web-forms-arrows-auto-forms-up/s16`, `282-web-forms-arrows-auto-forms-up/s17`,
  `282-web-forms-arrows-auto-forms-up/s18`, `282-web-forms-arrows-auto-forms-up/s19`,
  `282-web-forms-arrows-auto-forms-up/s20`, `282-web-forms-arrows-auto-forms-up/s21`,
  `282-web-forms-arrows-auto-forms-up/s22`, `282-web-forms-arrows-auto-forms-up/s23`,
  `282-web-forms-arrows-auto-forms-up/s24`, `282-web-forms-arrows-auto-forms-up/s25`).
- Difference: the whole step. JAWS said "main region", then the page's title for every Up
  after it; Okur reads the form's lines above the text area, its fields entered and left as
  in 280, and reaches the page's title at s26.
- Why: the same accident as Down out of the text area (280/s21): Up out of the empty text
  area went to the page's first lines and stayed there, skipping the whole form above it.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### JAWS is left outside the page after the Speech History

- Steps: `517-web-speech-layer-keys/s4`, `517-web-speech-layer-keys/s10`,
  `570-web-speech-history-clear-and-copy/s10`.
- Difference: the whole step. JAWS says nothing (or "blank"); Okur reads the page's line on
  Escape, and the next line on Down. 570 recorded the same Escape again, silent in every
  trial.
- Why: Escape out of the Speech History (Insert+Space, H) left JAWS's virtual cursor outside
  the page: it said "blank" in one trial and nothing in the others, and Down later said
  nothing though speech was on (its synthesizer log holds only the harness's own message).
  After its help viewer the same Escape reads the page's line (531/s3).
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### Up out of a text area holding text jumps to the page's top (Auto Forms Mode)

- Steps: `285-web-text-area-arrows-auto-forms/s19` to `/s24` (the full keys:
  `285-web-text-area-arrows-auto-forms/s19`, `285-web-text-area-arrows-auto-forms/s20`,
  `285-web-text-area-arrows-auto-forms/s21`, `285-web-text-area-arrows-auto-forms/s22`,
  `285-web-text-area-arrows-auto-forms/s23`, `285-web-text-area-arrows-auto-forms/s24`).
- Difference: the whole step. JAWS says "main region", then the page's title for every Up
  after it; Okur reads the field's label, "Text", and the page's lines above it.
- Why: the accident of 282 again, from a text area that holds text: Up from its first line
  went to the page's first lines in every trial of round 55 (2026-10-20), skipping the lines
  between.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### The Open dialog's "File name:" is said before the dialog

- Steps: `525-desktop-open-dialog/s1`, `532-desktop-dialog-default-button-and-hot-key/s1`.
- Difference: what JAWS says before the dialog. JAWS says "File name:", then "Open dialog",
  then the field ("File name: edit combo …"); Okur starts with the dialog.
- Why: the file name field's name arrives before the dialog JAWS has not yet announced, in every
  trial of both scenarios (round 56, 2026-10-20); the Save As dialog opens without it (524/s2).
  The name is said twice, the first time out of place.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### "Unavailable" on returning from a file dialog

- Steps: `524-desktop-save-as-dialog/s11`, `532-desktop-dialog-default-button-and-hot-key/s9`,
  and 524's at Intermediate and Advanced verbosity,
  `600-desktop-save-as-dialog-intermediate/s11` and `610-desktop-save-as-dialog-advanced/s11`.
- Difference: the whole step. JAWS says "Unavailable | Save As... Button | To activate press
  spacebar." without the window's title; Okur says the window's title, then the button.
- Why: Escape out of a common file dialog (Save As, Open) leaves the owner window disabled by
  the modal dialog for a moment, and JAWS reads the focus then, in every trial: the button is
  enabled. Other dialogs return with the window's title (306/s3, 537/s10).
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### The Font dialog's font is said before the dialog

- Steps: `541-desktop-notepad-font-dialog/s2`, and the same opening in 665 (rounds 115 and 119),
  `665-desktop-font-dialog-say-window/s2`.
- Difference: the whole step. JAWS says "Consolas" between the Notepad edit it leaves and
  "Font dialog", then again in the Font field; Okur says the dialog, then the field with its
  font once.
- Why: the dialog fills its font field about 400 ms after the field takes the focus, and JAWS
  reads that text as it arrives, ahead of the dialog it has not yet announced: in 3 of 3 trials
  of round 56 (2026-10-20) and 2 of 3 of the earlier recording. The font is said twice, the
  first time out of place.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### JAWS's PC cursor coordinates at 125%

- Steps: `513-desktop-dialog-helpers/s2`, `513-desktop-dialog-helpers/s8`,
  `516-desktop-jaws-cursor/s8`.
- Difference: JAWS says "PC Cursor active | 1045, 544"; Okur says "PC Cursor active | 836,
  435".
- Why: old-pc's screen is 1920 by 1080 pixels at 120 DPI (125%). The tracer, aware of each
  monitor's DPI (perf92's log: awareness 2), read the focused button's window rectangle as 723,
  416 to 948, 454, whose middle is 836, 435; JAWS gives 1.25 times that, a place that near the
  screen's right edge would lie past it. Okur says the screen's pixels, where its mouse keys
  put the pointer.
- Decided under the owner's rule on JAWS's accidents, 2026-10-20.

### JAWS reads nothing of WordPad's document

- Steps: `522-desktop-wordpad-reading/s1` to `/s14`, `/s16` to `/s18` (the full keys:
  `522-desktop-wordpad-reading/s1`, `522-desktop-wordpad-reading/s2`,
  `522-desktop-wordpad-reading/s3`, `522-desktop-wordpad-reading/s4`,
  `522-desktop-wordpad-reading/s5`, `522-desktop-wordpad-reading/s6`,
  `522-desktop-wordpad-reading/s7` (Say Word, added 2026-10-20 when Okur's laptop layout began
  reading a field's word with Insert+Numpad5 as JAWS's does), `522-desktop-wordpad-reading/s8`,
  `522-desktop-wordpad-reading/s9`,
  `522-desktop-wordpad-reading/s10`, `522-desktop-wordpad-reading/s11`,
  `522-desktop-wordpad-reading/s12`, `522-desktop-wordpad-reading/s13`,
  `522-desktop-wordpad-reading/s14`, `522-desktop-wordpad-reading/s16`,
  `522-desktop-wordpad-reading/s17`, `522-desktop-wordpad-reading/s18`).
- Difference: JAWS says nothing, a list item's bullet alone, "Nothing selected", or "top of
  file" and "Bottom of file" without the line; Okur reads the document's text.
- Why: on old-pc JAWS reads none of WordPad's document text, in every trial: the arrows, the
  word keys, Say Line and Say All are silent, and Say Selected Text finds nothing with text
  selected, yet JAWS reads the font in the same field (522/s15). A screen reader that cannot
  read the document is not a target.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

## Considered and not accepted

- `322-desktop-forms-viewer/s15`, `s16`: JAWS's control view of a list view shows its last row
  with its columns ("photo.jpg 1.2 MB"); Okur's shows one row too, another one and without
  columns. Neither is clearly right, so it stays a difference to fix.
- `305-desktop-edit-review/s13`: JAWS says "selected do" where the keys select "bo". It is
  unproven until a recording captures the clipboard after the step.
