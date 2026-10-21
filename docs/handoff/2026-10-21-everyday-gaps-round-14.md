# Handoff: everyday-use gaps, round 14 (2026-10-21)

Follows [`2026-10-21-everyday-gaps-round-13.md`](2026-10-21-everyday-gaps-round-13.md) and covers
the commits after it (`5d0d647` to `b001875`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-13.md`](../plans/2026-10-21-everyday-gaps-round-13.md).
The next plan is
[`docs/plans/2026-10-21-everyday-gaps-round-14.md`](../plans/2026-10-21-everyday-gaps-round-14.md).

## The owner's answers and requests

None this stretch. The owner's standing order (the README's "Not yet" list, and settings as
customizable as JAWS's) drove it.

## Decided without the owner (reversible)

- **What Flexible Web hides, Okur hides** (`91e0859`). JAWS 2025 with Chrome does not apply
  the customization: after Finish, and even after its refresh (Insert+Escape), R finds the
  hidden region and Down reads it (725, 735). JAWS's guide says a customization "is immediately
  applied to the page". These steps are an accepted divergence (716/s17-s22, 725/s14, s15,
  s27-s30, 735/s15, s16).
- **Settings without JAWS's labels stay out of the dialog.** Text Processing's three settings,
  same page links, graphics and Heading Indication are in the settings table, the files and
  `okur run --set`. They get their dialog controls once JAWS's names for them are found.
  Heading Indication's values are a new table kind with no labels (`Kind::Values`).
- **A page's reading choices are marked on the page** (`fead04e`, `51b9d2f`, `ce55f54`): the
  linearizer reads the marks, so every buffer built from the page keeps them. A page refreshed
  after a change now keeps the reading choices and Flexible Web's customizations as well as
  the custom labels; before, only the labels were kept.
- **A wizard keeps its earlier pages, hidden** (`2ebd960`). Removing the page before a turn
  left the sheet sending Next to the removed page.

## What was done

### Features (each checked with `okur-lab parity` against old-pc's evidence)

- **Flexible Web** (`d12ff00`, `f7d543d`, `91e0859`, `9a38cee`), from JAWS's guide (Surf's Up,
  chapter 16) and 705, 708, 716, 720, 725, 726 and 732-735:
  - Insert+Space, then X opens Okur's wizard. It is a Windows property-sheet wizard drawn by
    `okur-ui`, with JAWS's pages and words: What would you like to do?, Choose an Action,
    Choose an element, Choose a customization.
  - The elements offered are those at the cursor, innermost first: regions, headings, lists
    and frames. Each customization finds its element by kind, text, id and place, in JAWS's
    words ("Hide all heading level 1s with id = "events"", "Start at any heading level 1").
  - Finish hides the element until another page loads; Undo takes it back. Both close the
    wizard quietly.
  - Outside a page, X says "You must be in the web page area to activate Flexible Web."
  - Replay stands JAWS's traced wizard in for Okur's.
- **Wizards read as JAWS reads them** (`6787e1a`). This holds for any property-sheet wizard,
  not only Okur's:
  - its page is not named, and its text is the dialog's;
  - Next and Back say the page's text, then its name and "dialog", then the control;
  - a menu mode that showed nothing ends silently.
- **Settings**, each from one recording per setting:
  - Forms Mode, Auto, Semi-Auto or Manual (`07416cd`, 714, 715);
  - Document and Web Pages Automatically Read When Loaded (`07416cd`, 712, 713);
  - typing echo's four values, read from JAWS's files (`e7dd991`, 709-711);
  - Caps Indicate (`918938b`, 717);
  - Text Processing: mixed case, numbers as digits and dollars (`6bf5473`, 721-723);
  - Identify Same Page Links (`fead04e`, 724);
  - graphics shown or not (`51b9d2f`, 727);
  - Heading Indication (`ce55f54`, 730, 731);
  - the Control Type, Item State and Position output types, and Tutor (`15dde64`, 736-739).
    So that verbosity can leave their parts out, a page's check box, button and combo box
    focus is built from parts.

### Harness

- **`queue-batch.ps1` installs a staged `okur-ui.exe`** (`e53594d`). It reaches old-pc with
  the next deploy.
- **Two live checks of Okur on old-pc** (`run-scenario.ps1 -Reader okur`, compared with
  `okur-lab live-parity`). They are not evidence.
  - The first found the two wizard bugs fixed in `2ebd960`.
  - The second, at `2ebd960` (run file `live164` in the lab's batches), says: "Okur live:
    23/23 steps match JAWS exactly" for 705, 33/33 for 708, 30/30 with 6 accepted
    divergences for 716, and 41/41 for 720.
- **A diagnostic that drives `okur-ui` alone** (`experiments/diag-wizard.ps1` in the shared
  Dropbox): it sends okur-ui the engine's messages and lists the sheet's windows after each.

## Figures

Each is the named command's output at `b001875`:

- `okur-lab parity`: "JAWS parity (Ia2): 5718/5927 steps match exactly; 143 accepted
  divergences not counted (0 scenarios could not run)". At the previous handoff: 5288/5500
  with 129.
- `okur-lab parity`, spoken layer: "JAWS spoken parity: 5575/5889 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 402 scenarios, 1260 trials, 1260 successful.
- `cargo test --workspace`: 1303 passed, 0 failed, 199 ignored.

## Learned

- **JAWS's settings keys and values**, each recorded:
  - `SayAllOnDocumentLoad`;
  - `AutoFormsMode` 2, which is Semi-Auto;
  - `TypingEcho` 1 to 3;
  - `IndicateCaps`;
  - `MixedCase`, `Numbers` and `SayDollars`;
  - `IdentifySamePageLinks`;
  - `IncludeGraphics` 0;
  - `HeadingIndication` 0 and 1;
  - the output types `TUTOR`, `CONTROL_TYPE`, `ITEM_STATE` and `POSITION` at `0|0|0`.
  The shared `Default.JCF` lists about thirty output types, with modes 0, 1 and 2 per level.
- **`SayAllIndicateCaps=1`** raises capitalized words in Say All, and some lowercase runs as
  well (718). `LinePauses=1` sends Say All one line per utterance (719).
- **728 is unclear:** `AnnounceLiveRegionUpdates=0` still reads live updates, in another
  order. **729:** `Indentation=1` says "Left Margin" before the first line read.
- **Flexible Web's Save Temporary Customizations page** (740): Rule Name, the customizations as
  a list of check boxes, "How would you like this rule to apply?" (To the current site, 2 of
  3), Back and Cancel, Finish unavailable while the name is empty.

## Recorded meanwhile, and queued

Round 165 (`waiter165`, one trial) is recording 741: a rule saved, the page reloaded, the rule
deleted through JAWS's own rules page, and the page reloaded again. Check after it that no
rule is left on old-pc.

## Not done, and why

- **Flexible Web's rules**: saving, applying on load ("Start reading at" needs them), the rules
  page and the sound when a page with rules loads. They wait for 741.
- **Capitals in Say All and Say All's line pauses**: they need per-word pitch, and line
  grouping, in the live synthesizers' Say All path.
- **Live regions on or off, indentation**: their recordings do not yet say what JAWS does.
- **The settings dialog's new controls**: they wait for JAWS's labels.
- **Settings Center, Quick Settings, the Say All progress in replay**: unchanged from plan 13.
