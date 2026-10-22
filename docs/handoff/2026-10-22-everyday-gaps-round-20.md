# Handoff: everyday-use gaps, round 20 (2026-10-22)

Follows [`2026-10-22-everyday-gaps-round-19.md`](2026-10-22-everyday-gaps-round-19.md) and covers
the commits after it (`ccdb517` to `c94c504`, and this handoff's), under the plan
[`docs/plans/2026-10-22-everyday-gaps-round-19.md`](../plans/2026-10-22-everyday-gaps-round-19.md).
The next plan is
[`docs/plans/2026-10-22-everyday-gaps-round-20.md`](../plans/2026-10-22-everyday-gaps-round-20.md).

## The owner's answers and requests

- The owner restarted the machines mid-stretch for another task, then asked to resume. After
  the restart the dev box's Dropbox lagged: round files went to old-pc by `opendesk push`.

## Decided without the owner (reversible)

- **Skim reading's attribute rules find text with any checked attribute**, as JAWS did (803),
  not with all of them as its help says.
- **A notification rule's Play Sound plays the file and says nothing**; Select Sound... opens
  nothing yet (JAWS's opens Windows' Open dialog, 802).
- **Only the focused control's changes are said** (`dccfd47`): a change event of another
  control, even in a dialog opening, is not read.

## What was done

### Features (each checked with `okur-lab parity` against old-pc's evidence)

- **The mouse follows the Okur cursor** (`ccdb517`, `f8f90ec`): in a text field onto the
  character it is at (a Win32 edit's EM_POSFROMCHAR), on a page onto its line's element (IA2
  accLocation, or UIA's BoundingRectangle); Insert+Numpad Minus puts the mouse on the caret
  or the virtual cursor's element (`dc98551`). Okur's own mouse moves are not echoed.
- **Mouse echo, checked live on old-pc** (live run 214, 794's keys with Okur reading): lines
  were echoed as the mouse moved down Notepad's text, the same line not twice. The word and the
  character are now found in the line at the point's column (`e2cec5a`).
- **Notification rules: Shorten** (`a928e35`, from 798): the "Shortened Text:" field once
  Shorten is chosen, its text said in the notification's place ("Build done"), `\1`-style
  captures for pattern rules. **Play Sound** (`5ac5d4a`, from 801): "Path to Sound File:",
  "Select Sound...", "Play Sound"; a sound output in the core (`SoundOutput`) with a Windows
  driver (`PlaySoundW`).
- **Notification History's access keys** (`c8044a9`): Recent Notifications N, Enable Rules E,
  Create Rule... C, Manage Rules... M, Clear History L, from the traces. Manage Rules gives its
  list the focus back after Disable (`d12f4be`, a new `DialogFocus` message to Okur's window).
- **Skim reading by attribute rules** (`90892c5`, `c94c504`, from 799 and 803): the lines
  holding bold, italic or underlined text, found by each character's formatting or, for text
  joined inline into a line, among the line's element's siblings; then "N matches found" and
  the last line's first word. First-line skimming leaves out a table's row of column headers.
- **WPF dialogs as JAWS reads them**: the tracer reads a closed combo box's choice and an
  unnamed group box's header (`7b51eee`); a field in a group box gets the group's line, its
  header and the texts before the field ("Speech or Sound Action Path to Sound File:",
  `5ac5d4a`).

### Harness

- The replay's dialog mirror follows combo box choices, check boxes turned, unnamed fields'
  changes and the new focus message, so 798's rule is made through Okur's own dialogs and its
  notification is said shortened in replay.
- 775 and 787 recorded again with the new tracer (round 213).

### Accepted divergences

- JAWS's speech before its own dialogs now also covers 799/s1, 800/s1 and 803/s1.

## Figures

Each is the named command's output at `c94c504`:

- `okur-lab parity`: "JAWS parity (Ia2): 6347/6587 steps match exactly; 169 accepted
  divergences not counted (0 scenarios could not run)". At the previous handoff: 6267/6504.
- `okur-lab parity --spoken`: "JAWS spoken parity: 6210/6549 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 445 scenarios, 1359 trials, 1353 successful.
- `cargo test --workspace`: 1347 passed, 0 failed, 199 ignored.

## Learned

- **JAWS's notification rule dialogs** (798, 801, 802): Down Arrow on Speech Action says
  "Shorten", then "Play sound"; Shorten shows "Shortened Text:", Play sound "Path to Sound
  File:", "Select Sound..." and "Play Sound"; Select Sound... opens Windows' Open dialog in
  JAWS's sounds folder. The rule's text field holds the notification's text (SayLine reads it),
  though its focus says only "edit | Contains text".
- **JAWS's TypeKey takes "DownArrow", not "Down"**: round 214's `TypeKey("Down")` chose
  nothing.
- **Skim Reading keeps no attribute check box between openings** (803 found Bold unchecked
  after 799), though its Rules History lists "bold */*" and "bold italic */*".
- **An attribute rule's skim opens no summary** and ends "N matches found" and the last line's
  first word.
- **Okur's live echo on old-pc** read Notepad's lines through UI Automation in about 12 ms a
  read; Notepad's own word unit at a line's indentation gave "{\r\n    ".

## Recorded meanwhile, and queued

Rounds 213 to 221 are imported (but 793 to 796, kept out as before). Round 222 (804, skimming
by Underline) and round 223 (800, the first-line rule set back) are queued.

## Not done, and why

- **The replay of 787/s16** (after Disable, JAWS reads the renamed row) still gives nothing;
  the focus inside Okur's mirrored dialog is not spoken there. Found in the plan.
- **A dialog over a dialog named with its owner, the dialog under it said again on return, an
  empty list's "List box"** (787/s17-s19).
- **Select Sound...'s file dialog, skim reading's colors, the Word List's summary.**
- **"Contains text"**: what JAWS reads there is not the field's text; its source is unknown.
