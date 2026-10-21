# Handoff: everyday-use gaps, round 18 (2026-10-22)

Follows [`2026-10-21-everyday-gaps-round-17.md`](2026-10-21-everyday-gaps-round-17.md) and covers
the commits after it (`8651a71` to `9b181c9`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-17.md`](../plans/2026-10-21-everyday-gaps-round-17.md).
The next plan is
[`docs/plans/2026-10-22-everyday-gaps-round-18.md`](../plans/2026-10-22-everyday-gaps-round-18.md).

## The owner's answers and requests

None this stretch.

## Decided without the owner (reversible)

- **Notification rules are Okur's own file**, `notification-rules.toml` in the settings folder,
  as Flexible Web's are. JAWS keeps its own (`Notification.rules.json`); Okur never reads it.
- **Shorten and Play Sound are offered but not built**: Shorten says the whole notification,
  Play Sound says nothing. Braille actions are kept, for when Okur has braille.
- **Clear History forgets the last notification too**, so Shift+N says "No notification"
  after it. JAWS was not recorded doing either.
- **Okur's summary viewer reads its lines.** JAWS's Results Viewer gave the recordings no
  speech for Say Line, the arrows or Escape (an accepted divergence, below); Okur's viewer
  answers them as the Virtual Viewer does.
- **A window UI Automation calls a dialog is a dialog box**, whatever draws it (`944ddcb`).
  Okur announced only Win32 dialogs as dialogs; JAWS's WPF windows are announced as JAWS does
  ("Name Your Rule dialog | Please enter a name for the rule.").

## What was done

### Features (each checked with `okur-lab parity` against old-pc's evidence)

- **Skim reading's summary** (`8651a71`, `f56969b`, `334af33`):
  - Create Summary shows what skim reading would read from the cursor in the viewer "Skim
    Reading Results", each piece numbered, the page's title first;
  - Tab and Shift+Tab go from piece to piece as JAWS's links ("1. City Library Expands Evening
    Hours - Riverside Gazette Link | To activate press Enter."), Enter moves there;
  - a skim started from the dialog ends in its summary, as JAWS's does (769, 771);
  - Close keeps no choice, as JAWS's help says; Start Skim Reading keeps them.
- **Skim Reading's rules** (`03a46b7`, `ec8814a`, `be29855`), from 776, 777 and 780:
  - the rules' controls follow the rule type, in JAWS's Tab order;
  - a text rule reads the lines, sentences or paragraphs that hold its pattern, whatever its
    capitals; 780's summary of JAWS's rule "library" is exactly Okur's eight lines.
- **The Notification History** (`ed3c449`), Insert+Space, N, from 774: the list "Recent
  Notifications", text, sender and time, the latest first; Enter shows the details; Clear
  History; Enable Rules.
- **Notification rules** (`5aee2fb`, `a60a063`, `f6f068a`, `16375dc`, `381e576`), from 775, 783,
  784, 786 and 787:
  - Create Rule... with JAWS's controls, then Name Your Rule;
  - the first rule a toast matches mutes it or keeps it out of the history while Enable Rules
    is checked;
  - Manage Rules: Modify Rule, Rename..., Disable, Delete with Confirm Delete;
  - kept in the settings folder.
- **WPF reading** (`9e1a220`, `944ddcb`):
  - a list item named after its data type ("NotificationViewer.Services.NotificationData") is
    read by its texts;
  - UIA dialogs are dialogs, an unnamed field takes the text before it as its name, and the
    dialog window's own focus is not announced before its control's.

### Harness

Nothing new in the tools. The recordings that change JAWS's saved state (skim reading's rule
type and rules, notification rules) ran one trial each and were cleaned up after, by a
recording that sets them back; 787 does it within itself, so it ran three trials.

### Accepted divergences

- **Closing Skim Reading makes JAWS's summary** (763/s23, s24, 776/s19, s20, 777/s28, s29),
  reworded as 781 and 782 showed it: after changes and right after opening, by Escape or Close.
- **JAWS's summary viewer says nothing to Say Line and Escape** (769/s4, s6, s7, 771/s4).
- **JAWS's speech before its own dialogs** now also covers 776/s1, 777/s1.

## Figures

Each is the named command's output at `9b181c9`:

- `okur-lab parity`: "JAWS parity (Ia2): 6197/6434 steps match exactly; 168 accepted
  divergences not counted (0 scenarios could not run)". At the previous handoff: 6113/6339 with
  160.
- `okur-lab parity --spoken`: "JAWS spoken parity: 6064/6396 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 433 scenarios, 1329 trials, 1328 successful.
- `cargo test --workspace`: 1330 passed, 0 failed, 199 ignored.

## Learned

- **JAWS's Results Viewer answers Tab** (TypeKey's): each link is said ("1. ... Link | To
  activate press Enter."), wrapping after the last (780). Say Line, the arrows and Escape get
  nothing, ours or TypeKey's.
- **Skim Reading's saved state**: `[Skim Reading History] LastSkimRead=<type>:...:<pattern>`
  in JAWS's user DEFAULT.JCF, written by Start Skim Reading; deleting the last text rule removed
  the section. Start from the dialog with a saved rule opens on Rules History, Delete Rule and
  Delete All Rules available (779).
- **JAWS's summary on closing Skim Reading** came after changes and right after opening (781,
  782); only 760's Escape, after Tab through the dialog, closed it alone.
- **WPF access keys** are in the traces (`access_key`): Manage Rules... M, Create Rule... C,
  Enable Rules E, and in Manage Rules Modify M, Rename N, Disable A, Delete D. Keys by access key
  made the rule recordings independent of where the focus starts.
- **The Notification History remembers its focus** between openings within JAWS's run (785
  opened on Manage Rules...).
- **JAWS's notification rule dialogs** (787): Name Your Rule's prompt was "To navigate use
  Tab." in one trial of three, "Please enter a name for the rule." in the others.

## Recorded meanwhile, and queued

Round 204 (`waiter204`, three trials) is recording 788: the mouse button locks in Notepad
(Insert+Numpad Slash and Star, each twice).

## Not done, and why

- **WPF combo boxes' values and group names** in JAWS's own windows (775, 783, 787): a closed
  WPF combo box gives its choice only through UIA's selection pattern, which the tracer does not
  read, and its group box has no name in UIA. Needs the tracer to read them, then recordings.
- **JAWS's "Contains text" for an unnamed edit**, and its "_Rules" (a WPF label's access-key
  mark read as is): to judge as accidents with more recordings.
- **Skimming by attribute and color rules**, the Word List's summary.
- **Notification rules' Shorten and Play Sound, toasts Focus Assist hides, applications' own
  UIA notifications in the history.**
- **From plan 16 and 17, untouched**: the mouse following the Okur cursor, traces with bounds,
  the clipboard keys, the rest of verbosity.
