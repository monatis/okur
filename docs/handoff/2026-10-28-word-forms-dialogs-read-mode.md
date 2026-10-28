# Handoff: Word's fields, forms, dialogs and Read Mode; a faster tracer (2026-10-28)

Follows [`2026-10-25-word-links-and-ribbon-round-2.md`](2026-10-25-word-links-and-ribbon-round-2.md)
and covers the commits after it (`484d7f0d` to `ee552ab3`) and this one, made under
[`docs/plans/2026-10-25-everyday-gaps-round-35.md`](../plans/2026-10-25-everyday-gaps-round-35.md).
The next plan is [`docs/plans/2026-10-28-everyday-gaps-round-36.md`](../plans/2026-10-28-everyday-gaps-round-36.md).

Where things stand:
- **Word:** plan 35's Word items are built but its galleries: List Fields, line spacing, form
  fields as controls, links marked in a line, Read Mode, the Quick Access Toolbar and title
  bar, the Font dialog. Left: a drop-down list's Escape and a typed field left by Tab (round
  1060, queued), Word's other dialogs (Paragraph, round 1060), the galleries; Excel and
  Outlook.
- **The settings' long tail** (the owner's first item) waits: plan 36 §1 starts with the
  search's results list.
- **The review of 2026-10-25:** finding 13 fixed.

## What was built

### Word's object model (OBJID_NATIVEOM)

Windows hands an assistive technology the running Word's `Window` object through the document
window, as NVDA's Word support takes it; nothing is started, so old-pc's unactivated Office
does not stand in the way. Calls are cross-process (`crates/okur-platform/src/word_om.rs`).

- **Say Font's line spacing** (`484d7f0d`): JAWS says it, "Line spacing: Double" [obs, 1040,
  1052]. JAWS's multiples divide by the font's size and drop the fraction ("0 lines." for
  1.08 at 16 points); Okur says the true number, an accepted divergence (`a12a9a9a`).
- **List Fields, Insert+F5** (`3814d83c`): "List Fields dialog | List1 List view | HYPERLINK
  opening hours | 1 of 4" [obs, 1050, 1053, 1054]; OK selects the field.
- **Form fields as controls** (`0794d517`, `60e043f5`, `b857a367`): Word's UI Automation shows
  a text field's result and nothing for a check box or a drop-down. The platform asks the
  object model for the caret's paragraph's fields (when the document has any) and places them
  in the text (`LineStructure::form_fields`). Said as JAWS says them, named by the text
  before them: "Name: Edit", with what it holds; "Member: check box not checked"; "Branch:
  Combo box | Central | 1 of 3" [obs, 1055, 1056, 1057]. A line read says a text field by its
  name while empty ("Name: Name"); Tab moves among them, Space says a check box's new state,
  Alt+Down says "Open List Box" and the list's close says the field with its new choice.
- **Recorder snapshot:** the selection's and the paragraph's form fields per step.

### Word, more

- **The Quick Access Toolbar, title bar and search box** (`a77f4ee5`) [obs, 1051/s4-s13]:
  "menu | Leaving menus | Customize Toolbar sub menu | ...", "Can't Undo Unavailable | Can't
  Undo split button Unavailable | To move to an item, use the RIGHT ARROW, ...", "Ribbon Tabs
  Tab | Share Button | To activate press Enter.", "Tell me what you want to do... sub menu |
  ... Edit | Type in text.", "Ribbon | Collapse the Ribbon Button not checked | ...". A
  module can now say the menus it leaves itself (`FocusResponse::SpeakWhole`).
- **Read Mode** (`3c328b76`): back in the document by a key tip, the module asks for the view
  first (`AppContext::ask_view`, `AppModule::document_view`); Read Mode is said with JAWS's
  warning in Okur's name [obs, 1042/s8, 1059/s3]. Replay shows Okur reads Read Mode as JAWS
  does (the first line wherever Down goes), so the warning holds for Okur too. Insert+F1 there
  says how to leave it and how to keep Word from opening documents in it.
- **Links in a line** (`0936d6d8`): "See the  link opening hours before you come." [obs,
  1055/s1, s11], from the line's UI Automation hyperlink children (`LineStructure::links`).
- **Word's own dialogs** (`6525e534`), a module for `bosa_sdm_msword` windows [obs, 1058]: a
  field said once though Word reports it twice, as an edit combo ("Font: | edit combo |
  Calibri | To set the value use the Arrow keys or type the value."); a page's tab with
  "Control+Tab"; Escape back says "Edit" alone.

### The review's fixes

- **13** (`00290780`): a synthesizer failing five calls in a row is started anew.

## Harness

- **`okur-lab word-discard --title`** and the recorder: every fixture window is marked saved
  through Word's object model before it is closed, Close is posted once and waited for
  (`483ad517`). Before, a form left changed (a run cut short by a held trace) met Close
  twenty times over Word's save question, and Word piled nineteen "You cannot close Microsoft
  Word" boxes; cleaned by hand on old-pc (OK on each, Don't Save on the fixture's question).
- **`okur-lab word-form-fields`, `word-fields`, `word-line-spacing`**: probes of the object
  model.
- **The tracer** (`26df18d9`, `19593802`, `c035b811`): a caret's text is read once (the
  second read 40 ms later is a page's only), a caret event with a newer one waiting is
  recorded without a read, the form fields are read lean and kept a second (forgotten after a
  key live, and after a value change). One trial of 1055 with the tracer's timings: reading
  took 35.1 s and 16.7 s of the trial, before and after.
- **Replay** (`60e043f5`): a selection change that names no object but carries its text read
  is replayed where no caret event follows it in its step (Word's Tab in a protected form).
- **desktop-word-form.ps1**: a form protected for filling in; Word opens it in Read Mode,
  which the scenarios' reset leaves with Escape.

## Found, and how

- **Word opens a protected document in Read Mode**, where Tab does nothing: round-1055's 1056
  had JAWS say "Name: Edit" at every step. The recorder's snapshot said "view Read Mode".
- **The tracer fell seconds behind the keys** in Word's form: replay heard Home read the caret
  Ctrl+Right had moved to. Measured with `run-scenario.ps1 -TraceTimings` (the tracer's
  `--debug-log`), then fixed where the time went.
- **Word's dialogs remember their last page**: round-1055's 1058 trials opened on different
  pages; the scenario goes to the next page and back, and old-pc's Font dialog was put back on
  its first page by hand (Ctrl+D, Ctrl+Tab, Escape, by `okur-lab keys`).
- **old-pc's volume is 1%** (the owner, 2026-10-28): rounds still report `heard_audio: true`.

## Proof

`okur-lab parity` at `ee552ab3`:

```
JAWS spoken parity: 9019/9736 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 9111/9675 steps match exactly; 263 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `07aec7cf`: 9025/9606, 251 accepted divergences. `okur-lab parity --only`
for this stretch's scenarios, each line as printed:

```
1051: JAWS parity (Ia2): 23/23 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1055: JAWS parity (Ia2): 11/11 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1056: JAWS parity (Ia2): 11/11 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1057: JAWS parity (Ia2): 11/11 steps match exactly; 3 accepted divergences not counted (0 scenarios could not run)
1058: JAWS parity (Ia2): 8/8 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1059: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace`: 1519 passed, 0 failed, 200 ignored (summed from cargo's result
lines). `okur-lab invariants`: "Buffer invariants: 0 violation(s)". `okur-lab corpus-manifest`:
"Corpus manifest is current." The manifest's statistics: 685 scenarios, 2061 trials, 2044
succeeded.

## old-pc

- Round 1060 (1060, Word's Paragraph dialog; 1061, a form field typed into and left by Tab,
  a drop-down's list closed by Escape) queued behind round-1055h.
- JAWS at Rate=90, volume 1%. The okur, okur-lab and okur-ui there were built at `b857a367`.
- Still the owner's to end: Excel's three processes from 521's trials, the windowless Word
  and the PowerShell; the owner's "Document1" is open.

## Next

Plan 36: the settings' long tail first (the search's results list), then Word's rest, the
review's open findings.
