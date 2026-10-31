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

Every comparison with JAWS applies the table: `okur-lab parity` on all three layers (speech history,
spoken text, the UIA buffer), the parity baseline (whole steps are "accepted divergence") and
`okur-lab live-parity`. Their totals read, for example, "1143/1247 steps match exactly; 8 accepted
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
  `230-web-element-lists-items/s1`, `576-web-element-lists-items-intermediate/s1`,
  `586-web-element-lists-items-advanced/s1`, `233-web-list-combo-boxes/s1`, `234-web-list-edits/s1`,
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
  `506-web-jaws-find/s1` (its Find what field said before the dialog, in some trials); and
  the PlaceMarker List, `699-web-placemarker-list/s12`, `701-web-placemarker-add-dialog/s4`,
  `701-web-placemarker-add-dialog/s13`, `703-web-placemarker-none/s2` and
  `703-web-placemarker-none/s4`, `704-web-placemarker-two/s3`, `704-web-placemarker-two/s34`,
  `704-web-placemarker-two/s38` and `704-web-placemarker-two/s44` (its list, its version or
  its main window said before the dialog, in some trials); and the Word List,
  `560-web-word-list/s1`, `560-web-word-list/s4`, `900-web-word-list-advanced/s4`,
  `764-web-word-list-a-word-followed/s1`,
  `765-web-word-list-dialog-and-case/s1` and `766-web-word-list-past-the-last/s1` (its list,
  then the browser's title, or its version, said before the dialog; one trial of 560/s4 has
  none of it); and Skim Reading, `760-web-skim-reading-dialog/s1` and
  `763-web-skim-reading-rule-types/s1`, `769-web-skim-read-by-sentence/s1`,
  `771-web-skim-reading-rule-saved/s1`, `776-web-skim-reading-text-rules-controls/s1`,
  `777-web-skim-reading-speak-unit-and-attributes/s1`, `799-web-skim-read-by-bold/s1`,
  `800-web-skim-reading-back-to-first-lines/s1`, `803-web-skim-read-by-italic/s1`,
  `804-web-skim-read-by-underline/s1`, `805-web-skim-read-by-text-color/s1`,
  `809-web-skim-read-by-strikeout/s1`, `810-web-skim-read-by-double-strikeout/s1`,
  `811-web-skim-read-by-superscript/s1`, `812-web-skim-read-by-subscript/s1`,
  `813-web-skim-read-by-shadow/s1`, `815-web-skim-read-by-superscript-positions/s1` and
  `816-web-skim-read-by-subscript-positions/s1` (its rule type, or its rules' history, said before the
  dialog); and its Overwrite Clipboard question, `826-desktop-notepad-clipboard-edges/s8` (its
  main window, "JAWS Professional", and the window under it said first).
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

- Steps: `230-web-element-lists-items/s18`, and the same at Intermediate and Advanced,
  `576-web-element-lists-items-intermediate/s18`, `586-web-element-lists-items-advanced/s18`.
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

### A ribbon's places a key behind

- Steps: `523-desktop-wordpad-ribbon/s2`, `523-desktop-wordpad-ribbon/s3`,
  `970-desktop-wordpad-ribbon-lower/s2`, `970-desktop-wordpad-ribbon-lower/s3`,
  `970-desktop-wordpad-ribbon-lower/s5`, `972-desktop-wordpad-ribbon-f10/s2`.
- Difference: JAWS says the place of the tab or command the focus left: "View Tab selected 1
  of 2" after Right from Home, "Home 2 of 2" after Left, "Copy 1 of 3" after Right from Paste.
  Okur says the place of the one it reached, "2 of 2", "1 of 2", "3 of 3".
- Why: in every trial of rounds 270, 276 and 277; the ribbon's MSAA objects put View second
  of the two tabs shown and Copy third of its group's three.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-23.

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
  `539-web-page-load-summary-tables-navigation/s1` ("There are 1 ..."); above Beginner, its
  short message "N Smart Glance Highlights": `847-web-page-load-and-numpad-plus-intermediate/s1`,
  `848-web-page-load-and-numpad-plus-advanced/s1` ("2 ..."),
  `855-web-page-load-summary-news-article-intermediate/s1`,
  `856-web-page-load-summary-news-article-advanced/s1` ("4 ..."),
  `857-web-page-load-summary-tables-navigation-intermediate/s1`,
  `858-web-page-load-summary-tables-navigation-advanced/s1` ("1 ...").
- Difference: JAWS also says "There are N Smart Glance Highlights on this page." ("N Smart
  Glance Highlights" above Beginner).
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

### JAWS drops a link's first letter in Say All by sentence

- Steps: `677-article-say-all-by-sentence/s1`.
- Difference: JAWS says "irections" where Okur says "Directions" (the two links of the news
  article's table).
- Why: by sentence, JAWS loses the first letter of a link that follows the sentence "to 9
  p.m.", in every trial; by line it says the links whole (111).
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### JAWS's Say All by sentence or paragraph in a text box starts elsewhere

- Steps: `684-desktop-say-all-by-sentence/s1`, `684-desktop-say-all-by-sentence/s5`,
  `685-desktop-say-all-by-paragraph/s1`, `685-desktop-say-all-by-paragraph/s5`.
- Difference: the whole step.
- Why: Say All in a WinForms text box, from the line "Say All reads this. It has three
  sentences. The last one ends here.", in every trial:
  - by sentence, JAWS starts with the text after the line before's "P.M." ("aB9 é €"), says
    the line's last two sentences, and stops before the text's last line;
  - by paragraph, it reads the text box's first line ("Hello, world. Is it done? ...").
  By line it reads from the caret's line to the end (686). Okur reads from the caret's
  sentence or paragraph to the end.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### JAWS's cursor goes to the page's top after a custom label

- Steps: `688-web-custom-label/s15`.
- Difference: the whole step.
- Why: once a field is given a custom label, JAWS builds the page again with it and its
  virtual cursor is at the page's top: Say Line says the page's title, in every trial. The
  user labelled the field to go on working there. Okur keeps the cursor on the field, and
  Say Line reads it with its label ("lamp Edit").
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-21.

### JAWS reads the page again after a PlaceMarker is added

- Steps: `702-web-placemarker-saved/s8`, `702-web-placemarker-saved/s9`,
  `702-web-placemarker-saved/s10`.
- Difference: the whole step.
- Why: once a PlaceMarker is added, JAWS builds the page again and reads it from its top, as
  on loading, in every trial. Ctrl+Home then gets "selected" and that reading; K says
  "wrapping to bottom" before the PlaceMarker, since the reading moved the cursor down; and
  Insert+Up says "Not in a Table". Okur keeps the page and its cursor, so Ctrl+Home says
  the page's title and K and Insert+Up say the PlaceMarker's line.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-21.

### JAWS's Say Line is silent after the PlaceMarker List

- Steps: `701-web-placemarker-add-dialog/s15`, `703-web-placemarker-none/s6`,
  `704-web-placemarker-two/s37`.
- Difference: the whole step.
- Why: after the PlaceMarker List closed, the focus back in the page (Escape after K opened
  it on a page without PlaceMarkers; MoveTo), Say Line said nothing, in every trial of these
  scenarios. The virtual cursor is on its line, and Okur reads it.

### JAWS's cursor is at the page's end after a PlaceMarker is added

- Steps: `704-web-placemarker-two/s6`, `704-web-placemarker-two/s7`,
  `704-web-placemarker-two/s8`, `704-web-placemarker-two/s9`, `704-web-placemarker-two/s10`,
  `704-web-placemarker-two/s14`, `704-web-placemarker-two/s15`, `704-web-placemarker-two/s16`,
  `704-web-placemarker-two/s17`, `704-web-placemarker-two/s18`, `704-web-placemarker-two/s20`.
- Difference: the whole step.
- Why: after the first PlaceMarker is added, JAWS reads the page again and its cursor is
  left at the page's end, though Ctrl+Home was pressed: H went "wrapping to bottom" to the
  last heading and up from there, so the second PlaceMarker went after the first heading
  where the keys meant the third, in every trial, and K, Shift+K and 8 find it there. Okur
  moves as the keys say.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-21.

### JAWS names the Page Setup preview's pictures after a label and the title

- Steps: `544-desktop-notepad-page-setup/s13`, and the same at Intermediate and Advanced,
  `674-desktop-notepad-page-setup-intermediate/s13`, `675-desktop-notepad-page-setup-advanced/s13`.
- Difference: JAWS also says "Preview Bottom:" (twice) and "Preview Page Setup"; at Advanced,
  which leaves the box's name out, "Bottom:" twice and "Page Setup".
- Why: Say Window (Insert+B) in Notepad's Page Setup dialog ends, in JAWS, with the Preview
  box's three pictures of the page, read by names they do not have: "Bottom:" twice (the label
  before them in the dialog) and "Page Setup" (the dialog's title). The pictures have no text.
  Okur reads no picture in Say Window, as JAWS reads none in the other dialogs recorded.
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
  `633-web-jaws-find-next-board/s14`, and at Intermediate and Advanced in round 267,
  `926-web-jaws-find-intermediate/s8`, `926-web-jaws-find-intermediate/s9`,
  `926-web-jaws-find-intermediate/s10`, `926-web-jaws-find-intermediate/s11`,
  `927-web-jaws-find-advanced/s8`, `927-web-jaws-find-advanced/s9`,
  `927-web-jaws-find-advanced/s10`, `927-web-jaws-find-advanced/s11`.
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
  `570-web-speech-history-clear-and-copy/s10`, and the same step at Intermediate and Advanced,
  `863-web-speech-history-clear-and-copy-intermediate/s10`,
  `864-web-speech-history-clear-and-copy-advanced/s10`.
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
  `665-desktop-font-dialog-say-window/s2`, and at Advanced in round 123,
  `673-desktop-font-dialog-say-window-advanced/s2`, and in round 267,
  `934-desktop-notepad-font-dialog-intermediate/s2` and `935-desktop-notepad-font-dialog-advanced/s2`.
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
  `516-desktop-jaws-cursor/s8`, and in round 267 at Intermediate and Advanced,
  `930-desktop-dialog-helpers-intermediate/s2`, `930-desktop-dialog-helpers-intermediate/s8`,
  `931-desktop-dialog-helpers-advanced/s2`, `931-desktop-dialog-helpers-advanced/s8`,
  `932-desktop-jaws-cursor-intermediate/s8` and `933-desktop-jaws-cursor-advanced/s8`.
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
  `522-desktop-wordpad-reading/s17`, `522-desktop-wordpad-reading/s18`), and Say Line after
  the ribbon (rounds 276, 277): `523-desktop-wordpad-ribbon/s9`,
  `970-desktop-wordpad-ribbon-lower/s14`, `971-desktop-wordpad-ribbon-alt-twice/s5`,
  `972-desktop-wordpad-ribbon-f10/s5`, `973-desktop-wordpad-ribbon-key-tips/s6`.
- Difference: JAWS says nothing, a list item's bullet alone, "Nothing selected", or "top of
  file" and "Bottom of file" without the line; Okur reads the document's text.
- Why: on old-pc JAWS reads none of WordPad's document text, in every trial: the arrows, the
  word keys, Say Line and Say All are silent, and Say Selected Text finds nothing with text
  selected, yet JAWS reads the font in the same field (522/s15). A screen reader that cannot
  read the document is not a target.
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

### A region Flexible Web hides is still read

- Steps: `716-web-flexible-web-hide-and-undo/s17` to `/s22` (the full keys:
  `716-web-flexible-web-hide-and-undo/s17`, `716-web-flexible-web-hide-and-undo/s18`,
  `716-web-flexible-web-hide-and-undo/s19`, `716-web-flexible-web-hide-and-undo/s20`,
  `716-web-flexible-web-hide-and-undo/s21`, `716-web-flexible-web-hide-and-undo/s22`), and
  `725-web-flexible-web-hidden-region-read/s14`, `725-web-flexible-web-hidden-region-read/s15`,
  `725-web-flexible-web-hidden-region-read/s27`, `725-web-flexible-web-hidden-region-read/s28`,
  `725-web-flexible-web-hidden-region-read/s29`, `725-web-flexible-web-hidden-region-read/s30`,
  and after JAWS's refresh `735-web-flexible-web-hidden-after-refresh/s15`,
  `735-web-flexible-web-hidden-after-refresh/s16`.
- Difference: the whole step. JAWS reads the region it was asked to hide (the banner in 716,
  the advertisement in 725); Okur reads on without it.
- Why: after Finish the customization exists (the wizard's first page offers "Undo Hide
  ComplementaryRegion 1 from the top ..." [obs, 725/s34]), yet R finds the hidden region and
  Down reads its lines, in every trial of rounds 153 and 157 (2026-10-21), and after JAWS's
  refresh, Insert+Escape ("Screen updated"), in round 160. JAWS's guide
  (Surf's Up, chapter 16) says a customization "is immediately applied to the page". After
  Undo both read the region again.
- Decided under the owner's rule on JAWS's accidents, 2026-10-21.

### A saved Flexible Web rule hides nothing when the page loads

- Steps: `743-web-flexible-web-saved-rule-for-page/s24`,
  `743-web-flexible-web-saved-rule-for-page/s28`, `743-web-flexible-web-saved-rule-for-page/s29`,
  `743-web-flexible-web-saved-rule-for-page/s46`, `743-web-flexible-web-saved-rule-for-page/s50`,
  `743-web-flexible-web-saved-rule-for-page/s51`.
- Difference: the whole step. After the advertisement's customization was saved as a rule To
  the current page, JAWS reads the advertisement as the page loads again and R finds it; Okur
  applies the rule as the page loads: its summary counts one region and one link less, and R
  goes from the navigation region to the main region.
- Why: the rule exists (the wizard's first page then offers View or change where rules apply
  [obs, 743/s32]), yet JAWS hides nothing, as it hides nothing after a temporary
  customization (above). JAWS's guide
  (Surf's Up, chapter 16) says rules are applied whenever a page they apply to loads.
- Decided under the owner's rule on JAWS's accidents, 2026-10-21.

### The question before unsaved Flexible Web customizations is not read

- Steps: `742-web-flexible-web-save-page-and-reload-question/s30`,
  `746-web-flexible-web-rule-saved-from-question/s11`.
- Difference: the whole step. When a page loads while the page before has temporary
  customizations, JAWS asks whether to save them and says only "Flexible Web dialog | Yes
  Button | To activate press spacebar."; Okur says the page's title, then the dialog with its
  question, "You have created temporary Flexible Web customizations. If you continue without
  saving them, these customizations will be lost. Would you like to save your temporary
  customizations?", and Yes.
- Why: the question is the dialog's text (the trace has it), and JAWS reads the text of its
  other message boxes ("Are you sure you want to remove all PlaceMarkers?" [obs, 702/s27]). A
  Yes heard without its question cannot be answered.
- Decided under the owner's rule on JAWS's accidents, 2026-10-21.

### The focus stays in JAWS's window after the save page the question opened

- Steps: `742-web-flexible-web-save-page-and-reload-question/s39`,
  `742-web-flexible-web-save-page-and-reload-question/s41`,
  `742-web-flexible-web-save-page-and-reload-question/s42`,
  `742-web-flexible-web-save-page-and-reload-question/s43`,
  `742-web-flexible-web-save-page-and-reload-question/s44`,
  `742-web-flexible-web-save-page-and-reload-question/s45`,
  `742-web-flexible-web-save-page-and-reload-question/s46`.
- Difference: the whole step. After Escape on the save page that the question opened, JAWS
  leaves the focus in its own hidden main window (the trace's foreground is "JAWS
  Professional"): F5 reloads nothing, and Ctrl+Home and R say nothing. Okur's wizard gives the
  focus back to the page, which F5 reloads and reads.
- Why: the user is left in a window that cannot be seen and does not answer, and must find
  the page again with Alt+Tab. JAWS does the same after Finish there (746/s21-s23, where both
  are silent in replay, as the focus events are JAWS's).
- Decided under the owner's rule on JAWS's accidents, 2026-10-21.

### A corrected word is not said at its end (typing echo both)

- Steps: `711-desktop-typing-echo-jcf-3/s7`.
- Difference: the whole step. JAWS says "Enter"; Okur says "c Enter".
- Why: with typing echo set to both characters and words (`TypingEcho=3`), "c", "d",
  Backspace and Enter said only "Enter" in every trial of round 151 (2026-10-21). The same keys
  with words alone said the corrected word, "c", then "Enter" (710/s7). With both, the word an
  uncorrected Enter ends is said ("cd Enter", 403/s27). Only the corrected word goes unsaid,
  and only in this mode.
- Decided under the owner's rule on JAWS's accidents, 2026-10-21.

### Closing Skim Reading makes JAWS's summary

- Steps: `763-web-skim-reading-rule-types/s23`, `763-web-skim-reading-rule-types/s24`,
  `776-web-skim-reading-text-rules-controls/s19`, `776-web-skim-reading-text-rules-controls/s20`,
  `777-web-skim-reading-speak-unit-and-attributes/s28`,
  `777-web-skim-reading-speak-unit-and-attributes/s29`.
- Difference: the whole step. Escape (763, 777) or Close's Alt+C (776) closes JAWS's Skim
  Reading dialog and opens its summary in
  the Results Viewer ("1. City Library Expands Evening Hours - Riverside Gazette Link | To
  activate press Enter."), as Create Summary does (762/s2); Say Line then says nothing in the
  viewer. Okur's Escape and Close close the dialog and give the page back.
- Why: Escape and Close close the dialog "without saving the last changes made" (JAWS 2025
  help, Skim Reading Dialog), not make a summary. The summary came in every trial of four
  recordings (rounds 180, 184, 191 and 192, the last three with JAWS's viewers closed before
  each trial), wherever the focus was, so it is JAWS's own doing, not a window left open. It
  came after the rule type was changed, after Delete Rule (781, round 196) and right after
  the dialog opened (782, round 197; neither imported); only 760's Escape, after Tab through
  the dialog, closed it alone (760/s14).
- Decided under the owner's rule on JAWS's accidents, 2026-10-21.

### JAWS's summary viewer says nothing to Say Line and Escape

- Steps: `769-web-skim-read-by-sentence/s4`, `769-web-skim-read-by-sentence/s6`,
  `769-web-skim-read-by-sentence/s7`, `771-web-skim-reading-rule-saved/s4`.
- Difference: the whole step. Skim reading started from its dialog ends in its summary, in
  JAWS's Results Viewer and in Okur's viewer. There JAWS says nothing to Say Line, the arrows
  or Escape; Okur reads the summary's line, and Escape closes the viewer and reads the page's
  line.
- Why: in every recording with JAWS's Results Viewer in front (762, 767, 768, 769, 771), its
  reading keys and Escape got no speech, by our keys or by JAWS's TypeKey, while Tab got its
  links read (780). A viewer that does not answer Say Line cannot be read line by line; Okur's
  reads its lines as the Virtual Viewer does.
- Decided under the owner's rule on JAWS's accidents, 2026-10-22.

### JAWS's drag and drop finds nothing to drag in Notepad

- Steps: `790-desktop-drag-and-drop-in-notepad/s6`.
- Difference: the whole step. Ctrl+Insert+Numpad Slash's first press, with the mouse on
  Notepad's text, says how to drop (790/s2, matched); after the mouse moved three steps, the
  second press said "The object to be dragged was not found. This may be because it has been
  covered by another window. | The drag and drop action has been canceled." in every trial.
  Okur drags from where the first press was to where the mouse is, and says nothing.
- Why: nothing covered Notepad's window, so the message does not say why the drag failed; a
  drag that does what the first press promised is more use. A recording of a drag JAWS
  completes would show what it says then.
- Decided under the owner's rule on JAWS's accidents, 2026-10-22.

### JAWS's Word List Summary opens nothing

- Steps: `808-web-word-list-summary-by-alt-s/s3`, `808-web-word-list-summary-by-alt-s/s4`,
  `808-web-word-list-summary-by-alt-s/s5`, `808-web-word-list-summary-by-alt-s/s6`,
  `808-web-word-list-summary-by-alt-s/s7`, `808-web-word-list-summary-by-alt-s/s8`.
- Difference: the whole steps. With "library" chosen in the Word List, Summary (Alt+S, or Space
  on it in 807) closed the dialog and said the page, "City Library Expands Evening Hours -
  Riverside Gazette document"; nothing opened, and Tab, Enter and Insert+Up met the page.
  Okur shows "Word List Summary" in its viewer, a numbered link for each sentence holding the
  word, as JAWS's help describes it ("The Summary button opens the virtual viewer with a
  list of links. Each link is a sentence that contains the selected word.").
- Why: the help describes a feature JAWS did not show on the lab's page; Okur gives what it
  describes.
- Decided under the owner's rule on JAWS's accidents, 2026-10-22.

### JAWS passes over raised and lowered text of one character

- Steps: `811-web-skim-read-by-superscript/s15`, `812-web-skim-read-by-subscript/s16`,
  `815-web-skim-read-by-superscript-positions/s15`,
  `816-web-skim-read-by-subscript-positions/s16`, `818-web-number-positions-read-down/s2`,
  `818-web-number-positions-read-down/s7`, `824-web-number-positions-by-word/s2`,
  `824-web-number-positions-by-word/s5`, `824-web-number-positions-by-word/s6`.
- Difference: the lines found, and the marks of a one-digit number after a number, by line
  and by word (824: JAWS's "23", Okur's "2 superscript 3", then "end superscript in"). Skimming by an attribute rule with Superscript checked, JAWS
  found "The 1st and 2nd floors are open." and "A mole holds 6 × 10 superscript 23 end
  superscript particles." (815), but not "Einstein's E = mc2 dates from 1905.", "The sum runs
  to xn at most." or "Footnote marka after a word."; with Subscript, "The scale uses log10
  steps." and "The entry aij sits in row i.", but not H2O, CO2 or "Each term xi is counted
  once.", and said nothing when it found none (812). Chromium marks every one of them
  `text-position:super` or `sub` (the pages' IA2 snapshots); JAWS skips those of one
  character. Okur finds every raised and lowered text. Reading by line, JAWS says a raised
  or lowered number right after a number between marks, "10 superscript 23 end superscript",
  "5 subscript 10 end subscript", but not one of one digit: "Two cubed is 23 in short.",
  "Binary 1012 is five." (818). Okur marks those too: "2 superscript 3 end superscript".
- Why: JAWS's help says the rules find text with the attribute; leaving out a one-character
  superscript or subscript, the commonest kind (x², H₂O, a footnote mark), hides what the user
  asked for, and reading 2³ as 23 says another number, the very thing JAWS's marks prevent
  for 10²³.
- Decided under the owner's rule on JAWS's accidents, 2026-10-22.

### JAWS says Manage Rules' label twice, its underscore and all

- Steps: `787-desktop-notification-rule-made-managed-deleted/s8`,
  `787-desktop-notification-rule-made-managed-deleted/s10`,
  `787-desktop-notification-rule-made-managed-deleted/s15`,
  `787-desktop-notification-rule-made-managed-deleted/s18`,
  `798-desktop-notification-rule-shorten/s18`, `798-desktop-notification-rule-shorten/s20`.
- Difference: the phrase "_Rules" and the "Rules" after it. Opening Manage Notification Rules,
  JAWS says "Rules _Rules", then "Rules List box": the list's label as the WPF window holds it,
  the access key's underscore in it, before the list by its name. Okur says "Rules List box".
- Why: the underscore is markup for the access key, not text, and the name is said once.
- Decided under the owner's rule on JAWS's accidents, 2026-10-22.

### The recording of Windows+Insert+X holds Windows' menu, not JAWS

- Steps: `520-desktop-say-clipboard/s3`.
- Difference: the whole step. The harness pressed Windows+Insert+X by injecting the keys, and
  Windows takes an injected Windows-key chord: its own Win+X menu opened ("Context Menu | Apps
  and Features"), and JAWS read the menu. Sent with UI Access, and with Insert first, it does
  the same (old-pc, 2026-10-22). Pressed by a hand, the key is JAWS's SayClipboard, which
  reads the clipboard's text (825, the script called by name). Okur reads the clipboard there,
  then the menu the replay's trace holds.
- Why: the recording is of the harness's injected chord, not of JAWS's key.
- Decided by the agent, 2026-10-22.

### JAWS reads a file's recognized text over and over as its Results Viewer opens

- Steps:
  - `1024-desktop-ocr-file-picture/s3`
  - `1025-desktop-ocr-file-pdf/s3`
  - `1028-desktop-ocr-all-in-one-file/s3`
- Difference: the whole step. As F or Enter opens its Results Viewer, JAWS reads the text's
  first line three times, then all of it with its "Open in Word..." link, then often all of it
  again, and for the picture says its engine's "Page is right side up." before "finished.", in
  every trial. Okur says "Document OCR started.", "finished." and the text once.
- Why: the repeats are the viewer's window taking the focus while JAWS reads it; OmniPage's
  orientation report has no counterpart in Windows' recognizer. The Word link is a known gap
  (`the_results_viewer_ends_with_open_in_word`), not part of this entry's reason.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-24.

### JAWS's engine joins a scanned page's lines

- Steps:
  - `1025-desktop-ocr-file-pdf/s4`
  - `1025-desktop-ocr-file-pdf/s5`
- Difference: the whole step. JAWS's Results Viewer puts each page of the scanned PDF on one
  line, its two lines joined ("A letter drawn as pictures Here is the first page"), in every
  trial, where it keeps the picture's two lines apart (1024/s6).
- Why: the paragraphs are its engine's (OmniPage's) analysis. Windows' recognizer gives lines,
  and Okur shows them as it reads them.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-24.

### JAWS says its main window before its OCR message box

- Steps:
  - `1026-desktop-ocr-file-none-selected/s3`
  - `1027-desktop-ocr-file-not-a-picture/s3`
  - `1033-desktop-ocr-file-two-selected/s3`
- Difference: `JawsSaysBeforeItsDialog`. Before its message box ("OCR Failed to start. No
  file currently selected."), JAWS says its own main window, "JAWS Professional | JAWS Version
  2025.2505.43" and the folder, or the tutor message of a button, in most trials, and nothing in
  others.
- Why: JAWS's speech passes through its own windows on the way to its box. Okur's message box
  opens without them.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-24.

### F outside File Explorer: no context menu item to point to

- Steps:
  - `1030-desktop-ocr-file-outside-explorer/s3`
- Difference: JAWS also says "Or, when not in Windows Explorer, you may use the context menu
  while focus is on a file and choose Recognize With JAWS."
- Why: the sentence sends the user to an item JAWS's installer adds to File Explorer's context
  menu. Okur adds none to Windows, and says the first two sentences.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-24.

### JAWS says the selected file twice back from its Results Viewer

- Steps:
  - `1024-desktop-ocr-file-picture/s11`
  - `1025-desktop-ocr-file-pdf/s14`
  - `1028-desktop-ocr-all-in-one-file/s6`
- Difference: the whole step. Escape out of the Results Viewer: JAWS says the selected file
  and its place first ("SR OCR Sign.png | 3 of 3"), then the window with the file again, in
  every trial.
- Why: the list's focus reaches JAWS before the window's. Okur says the window with the file
  once.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-24.

### D: Okur points to F, as it reads no PDF open in Acrobat

- Steps:
  - `1031-desktop-ocr-document-not-open/s3`
- Difference: the whole step. D where no PDF is open in Adobe Acrobat: JAWS says "OCR document
  is only available inside of Adobe Acrobat." and how to open one there.
- Why: Okur recognizes no document open in a reader yet, so those sentences would be false of
  it. After "OCR Failed to start." it points to F, which recognizes a PDF selected in File
  Explorer. When Okur reads a PDF open in a reader, it says JAWS's sentences.
- Decided by the agent, 2026-10-24.

### The OCR layer's help lists only Okur's keys

- Steps:
  - `1032-desktop-ocr-layer-help/s5`
  - `1032-desktop-ocr-layer-help/s6`
  - `1032-desktop-ocr-layer-help/s7`
  - `1032-desktop-ocr-layer-help/s8`
  - `1032-desktop-ocr-layer-help/s9`
  - `1032-desktop-ocr-layer-help/s10`
  - `1032-desktop-ocr-layer-help/s11`
- Difference: the whole step. Okur's help leaves out JAWS's lines for A (a scanner or camera),
  D (a document open in Acrobat), R (a file into Word) and Ctrl+B's sound, so the lines after
  them come earlier. The first line and the last (Q) are JAWS's.
- Why: Okur has none of those keys; its help says what it does.
- Decided by the agent, 2026-10-24.

### JAWS's Scan Cursor is stuck past a window's last element

- Steps:
  - `1022-desktop-wpf-scan-cursor-controls-alone/s18`
  - `1022-desktop-wpf-scan-cursor-controls-alone/s19`
  - `1022-desktop-wpf-scan-cursor-controls-alone/s20`
  - `1022-desktop-wpf-scan-cursor-controls-alone/s21`
  - `1022-desktop-wpf-scan-cursor-controls-alone/s24`
  - `1022-desktop-wpf-scan-cursor-controls-alone/s25`
  - `1022-desktop-wpf-scan-cursor-controls-alone/s26`
  - `1022-desktop-wpf-scan-cursor-controls-alone/s27`
  - `1022-desktop-wpf-scan-cursor-controls-alone/s29`
  - `1022-desktop-wpf-scan-cursor-controls-alone/s30`
  - `1022-desktop-wpf-scan-cursor-controls-alone/s32`
  - `1023-desktop-wpf-scan-cursor-end-alone/s10`
  - `1023-desktop-wpf-scan-cursor-end-alone/s11`
  - `1023-desktop-wpf-scan-cursor-end-alone/s12`
  - `1023-desktop-wpf-scan-cursor-end-alone/s13`
  - `1023-desktop-wpf-scan-cursor-end-alone/s14`
  - `1023-desktop-wpf-scan-cursor-end-alone/s15`
- Difference: the whole step. JAWS's Scan Cursor goes past a WPF window's last element onto the
  window itself, in every trial: Down from the last element says it again and the window's
  title ("Focus here Edit | SR WPF Controls Fixture"). There it is stuck: Down, Up, Home and
  End say nothing, and Right says the title's first letter again and again. Ctrl+End lands
  there too, its line the title; only Ctrl+Home brings it back. Okur's Scan Cursor stays on the
  last element, Down saying it again, and Ctrl+End goes to it.
- Why: a cursor that Up cannot move is an accident. Recorded with the fixture's console hidden:
  with it shown (rounds 288-290), JAWS walked on into that console, a window of the same
  process.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-24.

### JAWS says "JAWSCursor" after recognizing a window

- Steps: `1011-desktop-screen-text-ocr-window/s4`,
  `1015-desktop-screen-text-ocr-window-english/s4`.
- Difference: the whole step. After Convenient OCR's W, JAWS's Alt+Delete says "JAWSCursor |
  590, 722 | Unrestricted": its name and the cursor's run together, without "active", in every
  trial. The JAWS cursor's own Alt+Delete is "JAWS Cursor active | 590, 722 | Application window
  restriction" (994/s4). Okur says "Okur Cursor active | 590, 722 | Unrestricted".
- Why: an internal name said in place of the words; the place and the restriction are the same.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-24.

### JAWS says what came before its lists in Word

- Steps:
  - `1036-desktop-word-heading-list-move/s5`
  - `1049-desktop-word-links-list-items/s1`
  - `1050-desktop-word-fields-list-items/s1`
  - `1053-desktop-word-fields-list-controls/s1`
  - `1054-desktop-word-fields-list-move/s1`
  - `519-desktop-word-lists-and-spelling/s2`
  - `519-desktop-word-lists-and-spelling/s5`
  - `1082-desktop-word-comments-list/s1`
  - `1083-desktop-word-footnotes-list/s1`
  - `1087-desktop-word-comments-list-controls/s1`
  - `1088-desktop-word-footnotes-list-controls/s1`
  - `1090-desktop-word-endnotes-list/s1`
  - `1091-desktop-word-bookmarks-list/s1`
  - `1094-desktop-word-notes-viewer/s1`, `1096-desktop-word-comments-in-viewer/s1`,
    `1097-desktop-word-footnotes-in-viewer/s1`, `1098-desktop-word-viewer-items/s1`,
    `1099-desktop-word-endnotes-in-viewer/s1`, `1100-desktop-word-no-comments-in-viewer/s1`
  - `1107-desktop-word-revisions-list/s2`
- Difference: `JawsSaysBeforeItsDialog`. Before its Heading List, Links List, List Fields,
  Spelling Errors, Reviewers' Comments, Footnotes, End Notes, Bookmark List, Revisions and Items
  to Display in Virtual Viewer dialogs in Word, JAWS says what it said or showed before them in
  most trials, in every one before its Links List, List Fields, the notes, bookmarks and
  revisions and Items to Display [obs, 1049/s1; 1050/s1, 1053/s1, 1054/s1; 1082/s1, 1083/s1,
  1087/s1, 1088/s1, 1090/s1, 1091/s1; 1094/s1, 1096-1100/s1; 1107/s2]: its main window's
  version, "JAWS Version 2025.2505.43", the document's name, "Microsoft Word Document", a
  button's or a list's hint, or "Please wait processing 1 items" while it gathers the errors,
  notes or bookmarks ("Please wait processing 2 items | Inserted" for two revisions, the first
  one's change); one trial of 1050 says nothing at all before the next key.
- Why: Okur's lists open without them, as JAWS's do in the other trials.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-24.

### Our keys in JAWS's Spelling Errors list

- Steps: `519-desktop-word-lists-and-spelling/s6`, `519-desktop-word-lists-and-spelling/s7`.
- Difference: the whole step. In JAWS's Spelling Errors list, Say Line and Insert+Shift+R got
  nothing in every trial.
- Why: the recorder's keys do not reach JAWS's own dialogs, which run elevated, as with the
  Results Viewer above. Okur's list reads its selected error, "teh 1 of 1", and Insert+Shift+R
  says there are no regions, as in any window.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-24.

### Word's line spacing as a multiple of lines, in Say Font

- Steps:
  - `1034-desktop-word-font-paragraphs-table-keys/s1`
  - `1034-desktop-word-font-paragraphs-table-keys/s3`
  - `1040-desktop-word-formats/s1`
  - `1040-desktop-word-formats/s3`
  - `1040-desktop-word-formats/s5`
  - `1040-desktop-word-formats/s7`
  - `1052-desktop-word-say-font-line-spacing/s3`
  - `1052-desktop-word-say-font-line-spacing/s11`
- Difference: the whole step. Word keeps a multiple line spacing in points, 12 a line (1.08 lines
  is its default for Normal). JAWS divides by the font's size instead and drops the fraction:
  "Line spacing: 1 lines." for 1.08 and 1.15, "2 lines." for 2.5, "0 lines." for 1.08 at 16
  points [obs, 1052/s3, s11; 1034/s1].
- Why: a wrong number. Okur says the true one, "Line spacing: 1.08 lines.", "2.5 lines.", in
  JAWS's words around it; single, one and a half, double, at least, exactly and whole multiples
  are said as JAWS says them.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-25.

### Space on a Word form's check box just checked

- Steps: `1057-desktop-word-protected-form-change/s3`.
- Difference: the whole step. Space on a check box form field JAWS has just checked: JAWS says
  nothing, in every trial, where the same key on the next check box says its new state
  [obs, 1057/s5].
- Why: nothing said for a change. Okur says the state each time, "not checked".
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-28.

### A Word form field said again

- Steps: `1057-desktop-word-protected-form-change/s9`, `1057-desktop-word-protected-form-change/s11`,
  `1061-desktop-word-protected-form-typed/s11`.
- Difference: the whole step. After Enter or Escape in a drop-down's list JAWS says the field with its
  new choice and its hint, then the field once more, or passes through its own menu bar first
  ("Context Menu | leaving menu bar"); after Ctrl+Home onto a text field it says "Name: Edit |
  top of file" three times over in every trial of round-1055h (one trial of round-1055g said it
  once).
- Why: repeats. Okur says each once: "Leaving menus | Branch: Combo box | Northside | 2 of 3 | To
  change the selection use the Arrow keys.", "Name: Edit".
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-28.

### Word's Paragraph dialog by its key tips

- Steps: `1060-desktop-word-paragraph-dialog/s4`.
- Difference: the whole step. The Paragraph dialog opened by the Home tab's key tips (Alt, H, P,
  G): JAWS says "Leaving menus | Leaving ribbons" and nothing of the dialog or its first field,
  in every trial; the next Tab reads the next field [obs, 1060/s5].
- Why: a dialog opened says nothing of itself. Okur says the dialog's first field, "Alignment:
  Combo box | Left | To change the selection use the Arrow keys."
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### Word's Font dialog opening

- Steps: `1058-desktop-word-font-dialog/s1`.
- Difference: the whole step. Opening Word's Font dialog (Ctrl+D), JAWS's words vary by trial:
  the field's value again after its hint in one, the page tab's role in place of the dialog's
  name in two.
- Why: neither is the dialog read once. Okur says the dialog's name, the field and its hint:
  "Font dialog | Font: | edit combo | Calibri | To set the value use the Arrow keys or type the
  value."
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-28.

### Word's Find and Replace opening

- Steps: `1065-desktop-word-find-replace-dialog/s1`.
- Difference: the whole step. Opening Find and Replace (Ctrl+H), JAWS reads the first field's
  own window, which Word reports last and without its label: "Find and Replace | Edit | To set
  the value use the Arrow keys or type the value." [obs, 1065/s1, round-1065b, every trial].
- Why: the field is read without its name or kind, where JAWS names the dialog's other fields
  ("Replace with: | edit combo" [obs, 1065/s2]). Okur says "Find and Replace dialog | Find
  what: | edit combo | To set the value use the Arrow keys or type the value."
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### "Lower Ribbon" carried over from an earlier trial

- Steps: `1066-desktop-word-font-name-list/s1`, `1067-desktop-word-font-size-list/s1`.
- Difference: the whole step. Alt in Word after an earlier trial left the ribbon from its lower
  ribbon by key tips (the Styles gallery, the Font box): JAWS says "Lower Ribbon | Ribbon |
  Home Tab" though the tabs have the focus, and in one trial of 1067 nothing [obs, 1064/s1
  trials 2-3, 1066/s1, 1067/s1].
- Why: the word is JAWS's memory of an earlier visit, and wrong for where the focus is; replay
  starts each trial anew. Okur says "Upper Ribbon | Ribbon | Home Tab", as JAWS does where no
  trial before went there [obs, 1043/s1, 1045/s1].
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### Escape out of the Font box's list

- Steps: `1066-desktop-word-font-name-list/s9`, `1067-desktop-word-font-size-list/s9`.
- Difference: the whole step. Escape in the Font or Font Size box's dropped list: Word closes
  it and gives the focus back to the document, reported about four seconds later (in one trial
  not at all), and JAWS says nothing, there or on the next two Escapes [obs, 1066/s9-s11,
  1067/s9-s11].
- Why: the user is back in the document without a word. Okur says "Leaving menus | Leaving
  ribbons | Edit" when the document's focus comes, as Escape out of the ribbon says [obs,
  1064/s10].
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### Word's Shift+F3 one change behind

- Steps: `1078-desktop-word-change-case/s3`, `1078-desktop-word-change-case/s4`,
  `1078-desktop-word-change-case/s5`.
- Difference: the whole step. Shift+F3 on Word's selected heading: JAWS says the selection as
  it was before the key, its words run together: "LibraryHours" where Word made it "LIBRARY
  HOURS", then "LIBRARYHOURS", "libraryhours" [obs, 1078/s3-s5; the trace reads Word's text
  after each key].
- Why: the case said is the one the user left, and the words lose their space. Okur says the
  selection as Word leaves it: "LIBRARY HOURS", "library hours", "Library Hours".
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### Word's Spelling pane takes the focus anywhere

- Steps: `1069-desktop-word-spelling-pane/s1`.
- Difference: the whole step. F7 in Word opens its Spelling pane with the focus on a different
  control in each trial [obs, 1069/s1]: JAWS said nothing in one, "Spelling | Add Button" in
  another, "teh | teh | No spelling suggestions" in the third, and the next keys followed from
  there.
- Why: where the focus lands is Word's, not JAWS's; Okur reads the control the pane gave the
  focus to in the trial replayed ("Ignore Button").
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### Back from Save As, the document window's name and a hint

- Steps: `1077-desktop-word-save-as-dialog/s4`.
- Difference: the whole step. Escape from Save As back to Word's document: JAWS says the
  document window's own name, "Microsoft Word Document", and the hint of Save As's combo box,
  "To set the value use the Arrow keys or type the value.", before "Edit" [obs, 1077/s4, every
  trial].
- Why: the window's own name is not the document's, and the hint is of a control no longer
  there. Okur says "Edit", as back from Word's own dialogs.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### "blank" before "Edit" after OK on a Word comment

- Steps: `1087-desktop-word-comments-list-controls/s6`.
- Difference: `JawsAlsoSays("blank")`. OK on a comment in JAWS's list of Word's comments: JAWS
  gives its synthesizer "blank" before "Edit", in every trial, where its speech history holds
  only spaces [obs, 1087/s6]: the empty text at the comment's mark said, where a footnote's is
  "footnote reference: 1 Holidays excepted." [obs, 1088/s6].
- Why: "blank" tells nothing about the comment or where the caret is. Okur says "Edit".
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### Word's view said as JAWS gathers the revisions

- Steps: `1104-desktop-word-revisions-in-viewer/s5`, `1104-desktop-word-revisions-in-viewer/s12`.
- Difference: `JawsAlsoSays("Print View")`. Word's revisions in JAWS's Virtual Viewer
  (Windows+Semicolon, Revisions): JAWS changes Word's view while it gathers them, to Draft
  ("Normal") and back, as the trace's zoom and focus events show, and says the view it comes
  back to as the viewer opens and again as it closes [obs, 1104/s5, s12, every trial].
- Why: a side effect of how JAWS reads them. Okur reads the revisions without changing the
  view.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-30.

### Excel's cell said again before JAWS's lists

- Steps: `1116-desktop-excel-lists/s1`, `521-desktop-excel-cells/s9`,
  `521-desktop-excel-cells/s13`.
- Difference: `JawsSaysBeforeItsDialog`. Before its Cells with Formulas, Col B and Move to
  Worksheet lists in Excel, JAWS says the cell it was on and a list's hint, "22 | B 4 | To
  move to an item press the Arrow keys." [obs, 1116/s1, 521/s9, s13, every trial but one of
  1116, which says nothing before the list's next key].
- Why: what JAWS said before its list comes forward, as before its Word lists. Okur's lists
  open without them.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-31.

### Word's letter said twice, then phonetically, onto a new page

- Steps: `1119-desktop-word-pages-by-character-and-word/s3`.
- Difference: the whole step. Right in Word onto the next page: JAWS says the page, then the
  letter twice, the second time by its phonetic name, "page 2 | B | bravo" on its synthesizer
  [obs, 1119/s3, every trial].
- Why: Word sends two caret events as the window scrolls (1119's trace), and JAWS's second
  reading of the character comes as a second press. Okur says it once: "page 2 | B".
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-31.

### Word's line said twice after PageDown

- Steps: `1113-desktop-word-page-keys/s5`.
- Difference: the whole step. PageDown in Word onto the next page: JAWS says the page, then the
  line twice, "page 2 | heading level 1 Borrowing | heading level 1 Borrowing" [obs, 1113/s5,
  every trial].
- Why: Word's caret comes to the line twice as the window scrolls. Okur says it once.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-30.

### Word's revision said twice after JAWS's list of them

- Steps: `1107-desktop-word-revisions-list/s8`.
- Difference: the whole step. OK on a revision in JAWS's list of Word's revisions: JAWS gives
  its synthesizer "blank", then says the revision, "nine Deleted", "Edit", and the revision
  again [obs, 1107/s8, every trial].
- Why: the caret's place read once as the list closes and once as Word's caret event comes.
  Okur says it once: "nine Deleted | Edit".
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-30.

### Word's comment in the Virtual Viewer with JAWS's own name for it

- Steps: `1095-desktop-word-comment-in-viewer/s5`, `1095-desktop-word-comment-in-viewer/s6`.
- Difference: `JawsAlsoSays("vispero:comment")`. Alt+Shift+Apostrophe on a comment in Word,
  once and twice quickly: JAWS reads a name of its own before the comment, also as the first
  line of the Virtual Viewer it opens [obs, 1095/s5, s6, every trial].
- Why: an identifier, not the comment. Okur says the comment and its author, and its viewer
  begins with the comment.
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-30.

### Word's comment read with JAWS's own name for it

- Steps: `1081-desktop-word-comment-read/s8`.
- Difference: the whole step. Alt+Shift+Apostrophe on a comment in Word: JAWS reads a name of
  its own before the comment, "vispero:comment | Check the summer hours. | by Lab" [obs,
  1081/s8, every trial].
- Why: the first words are an identifier, not the comment. Okur says the comment and its
  author: "Check the summer hours. | by Lab".
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### Word's indent said before the key acts

- Steps: `1080-desktop-word-indent-keys/s3`.
- Difference: the whole step. Word's second Ctrl+M: JAWS says the indent as it was before the
  key, "indent | 1.5 inches", where Word's paragraph now starts 2 inches from the page's edge
  (saved with the step) [obs, 1080/s3, round-1080d, every trial; round-1080c had one trial say
  "2 inches"]. The first Ctrl+M is the same in two trials of three ("1 inch" for 1.5), which
  the third trial's words cover.
- Why: the number said is not the paragraph's. Okur says what the key left: "indent | 2
  inches".
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### Notepad's Page Setup unsaid after a menu visit

- Steps: `1084-desktop-notepad-access-keys-all/s6`, `1085-desktop-notepad-access-keys-menus/s6`,
  `1086-desktop-notepad-access-keys-dialogs/s6`.
- Difference: the whole step. Page Setup opened by Alt+F, U after an earlier visit to the
  menus: JAWS says "Leaving menus | edit | Type in text." and nothing of the dialog, in every
  trial (its synthesizer heard nothing more either), then reads the next Tab's control [obs,
  1084-1086/s6, s7]. Recorded without that visit, JAWS says the dialog and its first field
  [obs, 544/s2].
- Why: a dialog opened without a word. Okur says it and its first field: "Page Setup dialog |
  Paper Size: Combo box | Letter | 8 of 10 | ...".
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

### "Menu bar" before a menu opened again, in some trials

- Steps: `1085-desktop-notepad-access-keys-menus/s5`, `1086-desktop-notepad-access-keys-dialogs/s5`.
- Difference: the whole step. Notepad's File menu opened again by Alt+F after Escape twice
  left it: JAWS says "Menu bar" before "File menu" in some trials and not in others (1084/s5
  has both; 1085 and 1086 happened to have it in every trial), and in one trial it said the
  bar's item and a context menu first [obs, 1084-1086/s5].
- Why: the words depend on the trial. Okur says the menu as JAWS's trials without them do:
  "File menu | New Ctrl+N | To move through items press up or down arrow." [obs, 1084/s5].
- Decided by the agent under the owner's rule on JAWS's accidents, 2026-10-29.

## Considered and not accepted

- `322-desktop-forms-viewer/s15`, `s16`: JAWS's control view of a list view shows its last row
  with its columns ("photo.jpg 1.2 MB"); Okur's shows one row too, another one and without
  columns. Neither is clearly right, so it stays a difference to fix.
- `305-desktop-edit-review/s13`: JAWS says "selected do" where the keys select "bo". It is
  unproven until a recording captures the clipboard after the step.
