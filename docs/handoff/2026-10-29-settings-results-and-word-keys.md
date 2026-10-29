# Handoff: Settings Center's search results; Word's gallery, lists, dialogs and keys (2026-10-29)

Follows [`2026-10-28-word-forms-dialogs-read-mode.md`](2026-10-28-word-forms-dialogs-read-mode.md)
and covers the commits after it (`d9a140cc` to `a4dc7bf6`) and this one, made under
[`docs/plans/2026-10-28-everyday-gaps-round-36.md`](../plans/2026-10-28-everyday-gaps-round-36.md).
The next plan is [`docs/plans/2026-10-29-everyday-gaps-round-37.md`](../plans/2026-10-29-everyday-gaps-round-37.md).

Where things stand:
- **Settings' long tail:** the search's results list and the pane's name are built (plan 36
  §1.1, §1.2). Left: Voices' Apply question and Synthesizer Language, the Startup Wizard's
  rest, the lifetimes' icons.
- **Word:** its Paragraph and Find and Replace dialogs, the Styles gallery, the Font and Size
  boxes' lists, Track Changes and the alignment keys are recorded and match. Recorded and
  matching with nothing to build: Ctrl+B, I and U on a selection (silent), Undo and Redo.
  Left: Copy and Paste (1073, recorded again with the recorder fixed), the font size and
  heading style keys (1074, 1075, recording), the Spelling pane (1069, pending: JAWS's own
  focus order differs by trial); Excel and Outlook.
- **old-pc:** round-1071b (1071 again with the alignment saved, 1074, 1075) was recording.

## What was built

### Settings Center's search results (`0200e873`)

JAWS shows the settings a search finds in a list box in its tree's place [the walk of Settings
Center's search, `docs/references/okur-settings-redesign.md`]. Okur's does the same:
- while the search box has words, "11 Search Results List box" stands where the tree was; Down
  from the search box goes into it;
- each result is its label and value ("checked"/"not checked" for a check box), then its
  group's path, without a place: "Typing Echo Off | User | Use the arrow keys to select an
  item. | Press the space bar to toggle between the choices";
- Space and Shift+Space change a result; Enter shows the tree at it, Escape the tree where it
  was, each after "Search edit box cleared.". The tree is left as it was while searching.

How: okur-ui controls can start hidden in the place of the list before them
(`Control::hidden`), be swapped with another (`ToUi::DialogSwap`), have their label set
(`ToUi::DialogLabel`), and a list box item can be renamed in place (`Value::ItemRenamed`). The
engine adds the path and the hints to the result's focus (`SettingsTree::result_focus`, the
focus's `own_hints`), says nothing for the item Space made anew, and says what a dialog did
before the focus it moves (`Outcome::BeforeFocus`). JAWS runs a subgroup's name into its
group's ("NavigationWeb / HTML / PDFs"); Okur keeps a space.

### The pane's name (`d55e39bf`)

F6 into Settings Center's pane says "User Page" first, as JAWS's "User page". A page is named
when it is not the last one the window's focus was on: Notepad's Print page has an untitled
inner dialog whose controls report no page, and naming on any change from none broke 546/s6
and its two verbosity twins (`okur-lab parity` showed 9183 before the narrower rule, 9186
after).

### Word

- **Styles gallery** (`550360c7`) [obs, 1064]: a gallery's item is an MSAA list item in a NetUI
  list, named by its description without the preview mark; said "Heading 2 | Button | 2 of 17".
  The item its key tip opens on and the gallery's window are not said; while Word's key tips
  lead, Escape steps back through them unsaid; the gallery's popup is not left by the ribbon's
  own focus.
- **Modeless dialogs** (`550360c7`) [obs, 1065/s6, s8]: the tracer records whether the window
  in front leaves its owner available (`top_window_owner_enabled`); back in the document from
  a modeless dialog (Find and Replace, by Ctrl+Tab or Escape) the window's title comes before
  "Edit". After the modal Font dialog, "Edit" alone, as before.
- **Font and Size lists** (`8d850001`) [obs, 1066, 1067]: an edit combo's own list dropping
  down is no menu: Down stays the field's and reads "Times New Roman", "18".
- **Track Changes** (`8d850001`) and **alignment keys** (`01649d9a`) [obs, 1068, 1071]: Word's
  object model is read once Word has acted on Ctrl+Shift+E or Ctrl+E, R, J, L (polled until the
  value changes, or 250 ms): "Track Changes on", "Centered", "aligned right", "justified",
  "Aligned left". The line spacing keys say nothing, as JAWS's.
- **Paragraph dialog** (`d9a140cc`) [obs, 1060]: its spin fields named; by key tips JAWS says
  nothing of the dialog (`abdcb48f`, a divergence).
- **Form fields** (`8f89770a`, `59805f11`) [obs, 1061]: a text field tabbed back to says
  "selected" before its text; JAWS's field said again after Escape in a drop-down's list is a
  divergence.

### The Virtual Viewer's words (`f00c2eb7`) [obs, 1063]

Ctrl+Left and Right in the viewer read a word as a word: a lone symbol as that character
("period."), the others with their symbols named.

## Harness

- **`okur-lab evidence --trace --converted`** (`550360c7`): under each traced event, what
  replay's converter makes of it (role, name, place, ribbon, groups). It showed the gallery's
  items were dropped and its window kept as a nameless Generic.
- **The recorder's snapshot** saves Word's Track Changes state and paragraph alignment after
  each step, for replay.
- **The recorder puts the step's clipboard back** after reading JAWS's history (`7db91a1f`):
  round-1072's 1073 (Ctrl+C, then Ctrl+V) pasted the recorder's sentinel.

## Found, and how

- **JAWS's "Lower Ribbon" on Alt** carries over from an earlier trial that left the lower
  ribbon by key tips (1064 trials 2-3, 1066, 1067); replay starts trials anew. A divergence.
- **Escape in the Font box's list** returns Word straight to its document, the focus reported
  about four seconds later, and JAWS says nothing (1066/s9, 1067/s9). A divergence.
- **Find and Replace opens** with JAWS reading the field's own window, without its label, as
  "Edit" (1065/s1, every trial of round-1065b). A divergence.
- **Word's Spelling pane** (1069): its first focus differs by trial in JAWS itself (Ignore All,
  Change All or the suggestions list); Okur takes the NetUI task pane for the ribbon and says
  "Leaving ribbons" where JAWS says the document's name. Left pending.

## Proof

`okur-lab parity` at `a4dc7bf6`:

```
JAWS spoken parity: 9116/9837 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 9212/9776 steps match exactly; 270 accepted divergences not counted (0 scenarios could not run)
```

At the last handoff, `ee552ab3`: 9111/9675, 263 accepted divergences. `okur-lab parity --only`
for this stretch's scenarios, each line as printed:

```
1060: JAWS parity (Ia2): 11/11 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1061: JAWS parity (Ia2): 11/11 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1063: JAWS parity (Ia2): 14/14 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1064: JAWS parity (Ia2): 11/11 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1065: JAWS parity (Ia2): 8/8 steps match exactly; 1 accepted divergences not counted (0 scenarios could not run)
1066: JAWS parity (Ia2): 10/10 steps match exactly; 2 accepted divergences not counted (0 scenarios could not run)
1067: JAWS parity (Ia2): 10/10 steps match exactly; 2 accepted divergences not counted (0 scenarios could not run)
1068: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1070: JAWS parity (Ia2): 9/9 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
1072: JAWS parity (Ia2): 8/8 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace --no-fail-fast`: 1531 passed, 0 failed, 200 ignored (summed from
cargo's 35 result lines). `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
`okur-lab corpus-manifest`: "Corpus manifest is current." The manifest's statistics: 695
scenarios, 2091 trials, 2074 succeeded.

Not checked live: Settings Center's results list and pane name in okur-ui on screen (the
dev box needs the owner's leave; old-pc does not run Okur's windows in rounds).

## old-pc

- Round-1071b (1071 again, 1074 font size keys, 1075 heading style keys) recording with the
  okur-lab and recorder of push `afe33ad4` (before the clipboard fix).
- Next: push the fixed recorder once 1071b ends, then 1073 again.
- JAWS at Rate=90, volume 1%. Still the owner's: Excel's three processes from 521's trials,
  the windowless Word and the PowerShell; the owner's "Document1" is open.

## Next

Plan 37: import round-1071b and build what it shows; 1073 again; then the settings' long tail
(Voices, the Startup Wizard's rest), Word's Spelling pane, the review's open findings.
