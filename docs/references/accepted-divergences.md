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
    Difference::JawsAlsoSays("JAWS Version 2025.2505.43") => [
        "222-web-links-list/s1",
        // ...
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

Guards (`cargo test -p okur-corpus`):
- `jaws_parity_matches_baseline` fails on a stale entry: a step that no longer exists, a phrase
  JAWS or Okur no longer says, or a whole step that now matches JAWS.
- `the_written_list_names_every_divergence` fails when this page does not name a step the table
  covers.

A divergence is proposed with its evidence and decided by the project's owner. An entry decided
under one of the owner's rulings says which.

## The list

### JAWS names its own version before its list dialogs

- Steps: `222-web-links-list/s1`, `223-web-links-list-escape/s1`, `224-web-headings-list/s1`,
  `225-web-form-fields-list/s1`, `226-web-element-lists/s1`, `226-web-element-lists/s3`.
- Difference: JAWS also says "JAWS Version 2025.2505.43".
- Why: the Links List, the Heading List, Select a Form Field and Virtual HTML Features belong to
  JAWS's own process (jfw.exe). As one opens, JAWS reads the version shown on its main window. It
  is a side effect, not something a user needs.
- Decided by the owner, 2026-10-18.

### JAWS's own windows between its dialogs

- Steps: `226-web-element-lists/s1` (also says "Web App Patterns - Google Chrome", the browser
  window's title) and `226-web-element-lists/s3` (also says "JAWS Professional" and "To move to
  an item press the Arrow keys.").
- Why: opening Virtual HTML Features, and going from it to the list chosen in it, JAWS's speech
  passes through windows the user never visits: the browser's, and JAWS's own main window
  with the tutor message of a list in it. The same keys recorded again (230/s1) have none of it.
  It is the side effect of the version above.
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
  `222-web-links-list/s1`, `223-web-links-list-escape/s1`, `224-web-headings-list/s1`,
  `225-web-form-fields-list/s1`, `226-web-element-lists/s1`, `227-web-form-fields-list-items/s1`,
  `228-web-form-fields-list-edit/s1`, `230-web-element-lists-items/s1`,
  `232-web-list-buttons/s1`, `233-web-list-combo-boxes/s1`, `234-web-list-edits/s1`,
  `235-web-list-check-boxes/s1`, `236-web-list-graphics/s1`, `237-web-list-lists/s1`,
  `239-web-list-paragraphs/s1`, `240-web-list-block-quotes/s1`, `241-web-list-regions/s1`,
  `242-web-list-tables/s1`, `243-web-list-glance-highlights/s1`, `244-web-list-divisions/s1`,
  `245-web-list-tabs/s1`, `246-web-list-radio-buttons-enter/s1`,
  `247-web-list-buttons-enter/s1`, `248-web-list-combo-boxes-enter/s1`,
  `249-web-list-edits-enter/s1`, `250-web-list-check-boxes-enter/s1`,
  `251-web-list-graphics-enter/s1`, `252-web-list-lists-enter/s1`,
  `253-web-list-articles-enter/s1`, `254-web-list-paragraphs-enter/s1`,
  `255-web-list-block-quotes-enter/s1`, `256-web-list-regions-enter/s1`,
  `257-web-list-tables-enter/s1`, `258-web-list-glance-highlights-enter/s1`,
  `259-web-list-divisions-enter/s1`, `260-web-list-tabs-enter/s1`.
- Decided under the owner's ruling on JAWS's version, 2026-10-18.

### A stray comma in Virtual HTML Features

- Step: `230-web-element-lists-items/s18`.
- Difference: JAWS also says "," after "Radio Buttons List".
- Why: Virtual HTML Features (Insert+F3) shows "Radio Buttons List," where every other list is
  named "... List". It is a typo in JAWS's text; Okur shows "Radio Buttons List".
- Decided under the owner's rule on JAWS's accidents, 2026-10-18.

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

### "Required" for a field that is not required (Auto Forms Mode)

- Step: `280-web-forms-arrows-auto-forms/s7`.
- Difference: the whole step. JAWS says "Email address Edit Required | sam@example.com"; Okur
  says it without "Required".
- Why: with Auto Forms Mode on, Down onto the sign-up form's email field said "Required" in
  all three trials. Nothing makes that field required: it has no `required` attribute, and the
  trace's focus says UIA's IsRequiredForForm and ARIA's required are both false. JAWS's Tab
  onto the same field (204/s2) does not say it. It is a wrong fact.
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

## Considered and not accepted

- `322-desktop-forms-viewer/s15`, `s16`: JAWS's control view of a list view shows its last row
  with its columns ("photo.jpg 1.2 MB"); Okur's shows one row too, another one and without
  columns. Neither is clearly right, so it stays a difference to fix.
- `305-desktop-edit-review/s13`: JAWS says "selected do" where the keys select "bo". It is
  unproven until a recording captures the clipboard after the step.
