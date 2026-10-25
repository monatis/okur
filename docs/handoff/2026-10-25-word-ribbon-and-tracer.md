# Handoff: Word's ribbon and views, a tracer that keeps up with Office, and more settings (2026-10-25)

Follows [`2026-10-25-settings-long-tail.md`](2026-10-25-settings-long-tail.md) and covers the 19
commits after it (`1981d8e9` to `a3573ae9`) and this one. They were made under
[`docs/plans/2026-10-25-everyday-gaps-round-33.md`](../plans/2026-10-25-everyday-gaps-round-33.md).

The owner's order:
1. the settings redesign to its end;
2. then the rest of Microsoft Word;
3. then punctuation by word and the README's "Not yet" list.

Where each stands:
- **Settings:** five more items of the long tail are built (below). Left: the search's results
  as a list box, the pane's name said, Synthesizer Language, Apply's question for a saved
  profile, Run Okur from System Tray, Tutor Messages' and Access Key's other choices, and the
  start options only setup or the secure mode can set.
- **Word:** Web Layout and Read Mode are named, its ribbon is read as JAWS reads it, its Links
  List is recorded. Left: line spacing (not in UI Automation), Read Mode's warning, the
  Links List's hint (it needs a list of Word's fields), Excel and Outlook.
- **Punctuation by word:** unchanged since the last handoff.

The next plan is
[`docs/plans/2026-10-25-everyday-gaps-round-34.md`](../plans/2026-10-25-everyday-gaps-round-34.md).

## What was built

### Settings

- **Language, Voice Profiles** in Okur's window (`166c3be3`), walked on old-pc:
  - Default, with the profiles and the one in use checked;
  - For <application> (the one in front as the window opens), with Use Default first;
  - No Speech (the No Speech synthesizer for the session, until a profile is chosen);
  - Create New... (Voice Adjustment).
  JAWS reading okur-ui: "Voice Profiles sub menu", "Default sub menu", "Automatic - (factory)
  checked". Main window menu items can be checked now.
- **A Say All's parts at the voice context's pitch** (`eedcea21`).
- **Settings Center's configuration display pane** (`0b4375f1`), from a walk of JAWS's (F6,
  "User page | Typing Echo Off radio button checked | 1 of 4"):
  - the selected setting's group beside the tree;
  - radio buttons for up to four values, a combo box for more, check boxes, sliders;
  - F6 to the setting's control, F6 back to the tree;
  - a value set there is the tree's at once.
  okur-ui shows it as a child dialog under the dialog's id, without taking the focus. JAWS
  reading okur-ui: "Typing Echo Characters radio button checked | 2 of 4", "Rate: 0 left
  right slider | 50 percent". JAWS's own pane says its name first; okur-ui's is not
  announced.
- **This user's start after sign-in** (`ae3a2527`), the startup proposal's M4 resolver:
  - `--sign-in` reads `HKCU\SOFTWARE\Okur\StartAfterSignIn` (always, never, all-users; none
    is Always) and the machine's `StartAfterSignInAllUsers`, and ends silently where they say
    not to start;
  - the Startup Wizard's Run Okur Settings offers "Start Okur after logon for this user:" with
    JAWS's three values.
  Always's Ease of Access start waits for E6.
- **Speak Sample** in Voice Adjustment (`8caf0608`). Walked on old-pc, JAWS says "This is a
  sample of the currently selected voice using the current voice settings."; Okur says it in
  the voice the dialog shows.

### Word

- **Web Layout and Read Mode** named by Say Window Title (`facdcfbd`): "Web Layout View",
  "reading layout View" [obs, 1042/s5, s9].
- **The ribbon** (`0aa40796`, `c27a8ed2`, `3ab3dcd9`), recorded in 1043 and 1042. A Word
  module (`apps/windows/word_ribbon.rs`) says:
  - Alt: "Upper Ribbon | Ribbon | Home Tab", the ribbon by its own name;
  - the arrows: a tab alone, "Insert Tab";
  - Down: "menu | Pages group | Cover Page sub menu";
  - a command alone, "Blank Page Button", with no place and no hint;
  - a key tip's tab: nothing (the engine tells modules when the last key was a letter);
  - Escape or a command back to the document: "Leaving menus | Leaving ribbons | Edit".
  WordPad's ribbon is read as before (970-973). A NetUI drop-down grid button (Word's Cover
  Page) is now read as a split button; the platform had not read it at all.

## Harness

- **The tracer keeps up with Office** (`d4b87786`). A trial of Word's ribbon read 423 NetUI
  state changes in 35 s, which put the tracer 15 s behind the keys. The ribbon's tree was read
  after the next key, and the trace stopped before the lower ribbon and the return to the
  document were read.
  - Now a NetUI state change is read only for the object last focused: the same trial skipped
    424 and recorded every focus.
  - A trace asked to stop reads what still waits, until a second passes without a message (at
    most a minute). Live, Okur stops at once.
  The same code runs live, so Okur itself no longer falls behind in Office's ribbon.
- **1038 recorded again** with it (`a280fe2f`). Its wrong replay start was this lag:
  `okur-lab parity --only 1038` gives "JAWS parity (Ia2): 13/13 steps match exactly", where
  the earlier evidence gave 10/13.
- **`jaws-window-walk.ps1`:** language (Voice Profiles and its submenus), samplecheck and
  sample (Speak Sample, the Tab stop heard before it is pressed).
- **`okur-ui-tree-probe.ps1`:** `-Pane` (the configuration display pane, F6 there and back)
  and the Language menu in `-Window`.

## Found, and how

- **NetUI focus comes in pairs:** each move gives the object's focus, then one with a small
  object id, which the tracer skips as a repeat. In 1043's trial 1 the s4 focus is missing
  from the trace (trials 2 and 3 have it), so that step does not match in replay.
- **Word names its views on the status bar's buttons** ("Print Layout", "Web Layout", "Read
  Mode"), whose states change as the view does; the traces carry them.
- **Read Mode:** on entering, JAWS says "reading layout View | WARNING! JAWS does not support
  this view. Press JAWS Key+F1 ... To exit Reading Layout view, press ESC." Whether Okur
  reads Read Mode decides what Okur says there: a live check first.
- **The Links List in Word without links:** "no links | Check for link type fields with
  Insert+F5." The hint names JAWS's list of Word's fields, which Okur does not have yet.

## Proof

`okur-lab parity` at `a3573ae9`:

```
JAWS spoken parity: 8841/9564 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 8928/9503 steps match exactly; 251 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab parity --only` 1042, 1043, 1044, each line as printed:

```
JAWS parity (Ia2): 10/11 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
JAWS parity (Ia2): 10/11 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
JAWS parity (Ia2): 2/3 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

What does not match:
- 1042/s8 is Read Mode's warning;
- 1043/s4 is the focus trial 1's trace lacks;
- 1044/s1 is the Links List's hint.

`cargo test --workspace`: 1494 passed, 0 failed, 201 ignored (summed from cargo's result lines).
`okur-lab invariants`: "Buffer invariants: 0 violation(s)". `okur-lab corpus-manifest`: "Corpus
manifest is current." The manifest's statistics: 670 scenarios, 2016 trials, 2000 succeeded.

## old-pc

- Idle. JAWS running with its own settings and Rate 90; Notepad open; Word's fixture closed.
- The okur.exe, okur-lab.exe and okur-ui.exe there are this stretch's (`d4b87786`'s tracer,
  okur-ui with the pane).
- Still the owner's to end: Excel's three processes from 521's trials (9104, 10728, 12680),
  the windowless Word (9688) and the PowerShell (4200); the owner's Word document
  ("Document1") is open.
