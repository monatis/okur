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

A divergence is proposed with its evidence and decided by the project's owner. Entries the
parity gap audit proposed say so until the owner has reviewed them.

## The list

### JAWS names its own version before its list dialogs

- Steps: `222-web-links-list/s1`, `223-web-links-list-escape/s1`, `224-web-headings-list/s1`,
  `225-web-form-fields-list/s1`, `226-web-element-lists/s1`, `226-web-element-lists/s3`.
- Difference: JAWS also says "JAWS Version 2025.2505.43".
- Why: the Links List, the Heading List, Select a Form Field and Virtual HTML Features belong to
  JAWS's own process (jfw.exe). As one opens, JAWS reads the version shown on its main window. It
  is a side effect, not something a user needs.
- Decided by the owner, 2026-10-18.

### JAWS's control view drops an emoji

- Steps: `312-desktop-virtualize-control/s3`, `/s4`, `/s9`, `/s14` (the full keys:
  `312-desktop-virtualize-control/s3`, `312-desktop-virtualize-control/s4`,
  `312-desktop-virtualize-control/s9`, `312-desktop-virtualize-control/s14`).
- Difference: Okur also says the grinning face emoji at the end of the line.
- Why: Virtualize Current Control (Insert+Shift+V) on the editor fixture drops the
  non-breaking space and the emoji that end its last line. JAWS reads the emoji when it reads
  the field itself [obs, 313/s4]. Text copied from Okur's viewer keeps it; from JAWS's, it is
  lost.
- Proposed by the parity gap audit, 2026-10-18; for the owner's review.

### JAWS finds no font in a WinForms text box

- Steps: `313-desktop-say-font/s1`, `313-desktop-say-font/s2`, `313-desktop-say-font/s3`,
  `313-desktop-say-font/s4`, `313-desktop-say-font/s5`, `313-desktop-say-font/s6`,
  `313-desktop-say-font/s7`, `313-desktop-say-font/s9`.
- Difference: the whole step.
- Why: Say Font (Insert+F) in a WinForms text box gets "No font" from JAWS, though JAWS names
  Notepad's font [obs, 314/s1]. Okur says "Font is Consolas 11 point". Pressed twice, Okur then
  shows the font in the Virtual Viewer, as JAWS does when it finds one. So the scenario's later
  keys (Down, Ctrl+A, Escape) act in that viewer, not in the field, and those steps differ too.
- Proposed by the parity gap audit, 2026-10-18; for the owner's review.

## Considered and not accepted

- `322-desktop-forms-viewer/s15`, `s16`: JAWS's control view of a list view shows its last row
  with its columns ("photo.jpg 1.2 MB"); Okur's shows one row too, another one and without
  columns. Neither is clearly right, so it stays a difference to fix.
- `305-desktop-edit-review/s13`: JAWS says "selected do" where the keys select "bo". It is
  unproven until a recording captures the clipboard after the step.
