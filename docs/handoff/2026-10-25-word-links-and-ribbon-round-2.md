# Handoff: Word's Links List, more of its ribbon, and more of the review's fixes (2026-10-25)

Follows [`2026-10-25-word-ribbon-commands-and-review.md`](2026-10-25-word-ribbon-commands-and-review.md)
and covers the 15 commits after it (`a5df0263` to `07aec7cf`) and this one, made under
[`docs/plans/2026-10-25-everyday-gaps-round-35.md`](../plans/2026-10-25-everyday-gaps-round-35.md),
which stays the active plan with this handoff's additions.

Where things stand:
- **Word:**
  - The Links List lists the document's hyperlinks.
  - The ribbon's Home tab is recorded end to end (1048, 1051), the View tab's check boxes
    too.
  - Left: Insert+F5's fields (UIA lacks Word's form fields), the Quick Access Toolbar and
    title bar (1051/s4-s13), Read Mode, line spacing, Excel and Outlook.
- **The review of 2026-10-25:** findings 8 (the engine half), 13 (in part) and 19 (in part)
  done this stretch; see [`../backlog/performance-review-2026-10-25.md`](../backlog/performance-review-2026-10-25.md).

## What was built

### Word's Links List (`8abaebb9`, `58e40031`)

Recorded in 1049 (two links) and 1050 (Insert+F5), on a new fixture with two hyperlinks, a text
form field and a check box form field (`desktop-word-fields.ps1`).

- **What JAWS shows:** its Links List in Word is its web one with Word's hyperlinks in it:
  "Links List dialog | links List view | opening hours | 1 of 2". Insert+F5 is "List Fields |
  List1 List view | HYPERLINK opening hours | 1 of 4", then "FORMTEXT", "FORMCHECKBOX".
- **What Word gives:** its UI Automation exposes the hyperlinks as the document range's
  children, each with its address as its value, but not the form fields (probe
  `fields-uia.ps1` in the scratchpad, run on old-pc).
- **Built on the Heading List's route:**
  - core: `DocumentLinks`, `read_document_links`, `activate_document_link`;
  - platform: `text_context::word_document_links`, one pass over the paragraphs for each
    link's;
  - recorder: `okur-lab document-headings` saves the links;
  - replay: answers with them.
  - Move To Link moves the caret to the link's paragraph; Activate Link invokes it.
- **A live bug fixed on the way** (`58e40031`): in Word, Insert+F7 and Insert+F5 built their
  lists from the virtual buffer, which can still hold the last page read. After a page with
  links, Word's Links List showed the page's. A test fails without the fix.
- **Insert+F5 stays "no fields"**, documented in the backlog (Word's form fields are not in
  UIA; "List1" looks like an unlabeled control of JAWS's, an accident).

### Word's ribbon

- **A sub menu is a menu only while the focus is on it** (`a5df0263`, 1048): Text Effects after
  the Font group's buttons is "menu | Font group | ... sub menu" again, and the Paragraph group
  after it is named alone.
- **ScreenTips by the whole name** (`3c4ae3eb`): said unless the tip has the whole name in it,
  its dots too, or the name has it. "Paragraph Settings" for "Paragraph..." and "More" for
  Styles are said; "Font... (Ctrl+D)", "Add a Blank Page", "View Ruler" are not.
- **A button that opens a menu** (`3c4ae3eb`, trace format 9): Styles is a drop-down that UIA
  calls a button, so "sub menu | Styles Button | More", no menu entered. The tracer records
  Office's own element's control type; `FocusEvent::opens_menu`.

### The review's fixes

- **8, the engine half** (`988e9fad`): Settings Center sends only the tree items a change
  changed (1 or 2 per Space, where all 53 were).
- **13, in part** (`bda32db1`): a failed synthesizer call is logged, the first and then every
  hundredth.
- **19, in part** (`d9edb199`): Settings Center's `change` loses its dead back direction
  (Shift+Space goes on as Space does in JAWS).

## Found, and how

- **Keys in JAWS's own lists** must be JAWS's (`jaws_function` `TypeKey("DownArrow")`): round
  1049's first recording sent ours, which the elevated list refused, and every step after the
  list opened was silent. Recorded again as 1049b.
- **A deploy cut off by a tool timeout** left old-pc on the previous okur-lab for round 1049c,
  whose snapshots had no links. Build and deploy now run as a background command of their own,
  with the remote hash compared before recording (1049d).
- **Shift+Tab from Paste** leaves the lower ribbon for the tabs and the Quick Access Toolbar,
  not its last group (1051). JAWS's words there, not built yet:
  - "menu | Leaving menus | Customize Toolbar sub menu | To move through items press up or
    down arrow.";
  - "Leaving menus | Can't Repeat Button Unavailable";
  - "Can't Undo Unavailable | Can't Undo split button Unavailable | To move to an item, use
    the RIGHT ARROW, then use UP and DOWN ARROWS to select one.";
  - "Save Button | To activate press spacebar." (no place);
  - "Ribbon Tabs Tab | Share Button | To activate press Enter.";
  - "Sign in Link | <its tip> | To activate press Enter.";
  - "Tell me what you want to do... sub menu | ... Edit | Type in text.";
  - "Ribbon | Collapse the Ribbon Button not checked | To activate press spacebar.";
  - back into the lower ribbon, "menu | Upper Ribbon | Editing group | Select sub menu".

## Proof

`okur-lab parity` at `07aec7cf`:

```
JAWS spoken parity: 8937/9667 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 9025/9606 steps match exactly; 251 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `371962d3`: 8972/9542. `okur-lab parity --only` 1048, 1049, 1050 and 1051,
each line as printed (1049's s1 counts under `JawsSaysBeforeItsDialog`, `9921c062`; 1050 lacks
Insert+F5's fields; 1051 lacks the Quick Access Toolbar and title bar):

```
JAWS parity (Ia2): 30/30 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
JAWS parity (Ia2): 5/5 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
JAWS parity (Ia2): 4/6 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
JAWS parity (Ia2): 14/23 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace`: 1508 passed, 0 failed, 201 ignored (summed from cargo's result
lines). `okur-lab invariants`: "Buffer invariants: 0 violation(s)". `okur-lab corpus-manifest`:
"Corpus manifest is current." The manifest's statistics: 677 scenarios, 2037 trials, 2021
succeeded.

## old-pc

- Idle. JAWS at Rate=90. The okur, okur-lab and okur-ui there were built at `3c4ae3eb`
  (trace format 9), the hashes compared.
- Still the owner's to end: Excel's three processes from 521's trials, the windowless Word and
  the PowerShell; the owner's "Document1" is open.

## Next

Plan 35 stays active, with:
- Word's Quick Access Toolbar and title bar, from 1051's words above;
- Word's fields (Insert+F5): only the hyperlinks are in UIA, so a documented gap or Word's
  object model;
- the review's open findings.
