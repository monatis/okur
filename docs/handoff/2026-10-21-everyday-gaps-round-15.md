# Handoff: everyday-use gaps, round 15 (2026-10-21)

Follows [`2026-10-21-everyday-gaps-round-14.md`](2026-10-21-everyday-gaps-round-14.md) and covers
the commits after it (`d943614` to `16a73a2`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-14.md`](../plans/2026-10-21-everyday-gaps-round-14.md).
The next plan is
[`docs/plans/2026-10-21-everyday-gaps-round-15.md`](../plans/2026-10-21-everyday-gaps-round-15.md).

## The owner's answers and requests

None this stretch.

## Decided without the owner (reversible)

- **Rules are kept as JAWS's are** (`9d187f0`): a library of named rules, each on or off for an
  area (everywhere, a site, a page). The smallest area with a setting wins, as the rules
  dialog's text says. A rule saved To the current page is on for the page and off above it
  (748/s12, s15). A file page's site is "localhost", as JAWS names it (748/s9). Okur keeps the
  rules in its settings folder (`flexible-web.toml`).
- **What a rule hides, Okur hides as the page loads.** JAWS 2025 saves the rule and shows it in
  its rules dialog, but hides nothing on loading, as after a temporary customization (743).
  These steps are an accepted divergence.
- **The question before unsaved customizations is read.** When a page loads while the one
  before has temporary customizations, JAWS asks whether to save them. It says only "Flexible
  Web dialog | Yes Button", without the question (742/s30, 746/s11), though it reads its other
  questions (702/s27). Okur says the page's title, then the question. This is an accepted
  divergence.
- **Okur's wizard gives the focus back to the page.** After Escape or Finish on the save page
  that the question opened, JAWS leaves the focus in its own hidden main window. F5 and R then
  reach nothing (742/s39-s46, 746/s21-s23). The 742 steps where replay differs are an accepted
  divergence. In 746 both are silent in replay, since the focus events are JAWS's.
- **Recordings that did not record what their names say were not imported:**
  - 741: the rule's name never reached JAWS's wizard.
  - 744, 745 and 747: they need a rule an earlier scenario saved, and their files cannot name
    it now without changing their hashes.
  748 and 751 record the same dialog, each naming the scenario it continues.

## What was done

### Features (each checked with `okur-lab parity` against old-pc's evidence)

- **Flexible Web's save page** (`fcb7f11`), from 740, 742 and 743. Save Temporary
  Customizations has:
  - Rule Name;
  - the customizations as a list with check boxes;
  - "How would you like this rule to apply?" (Globally, To the current site, the first
    choice, To the current page);
  - Finish, unavailable until the rule has a name.
- **The question as a page loads with unsaved customizations** (`fcb7f11`): Yes opens the save
  page alone, without Back, and No loses them.
- **Rules applied whenever a page they are on for loads** (`fcb7f11`, `9d187f0`).
- **View or Change Where Rules are Applied** (`9d187f0`, `e6a2b96`), from 748 and 751:
  - the first page's Next becomes Finish when View or change where rules apply is chosen
    (743/s37);
  - a tree of the areas around the page, the site closed at first;
  - the rules under Rule and Set for, checked where on;
  - Space turns a rule on or off for the area chosen;
  - Delete asks "You are about to delete rule ..." (the dialog named Okur, where JAWS's says
    JAWS), and Yes deletes it everywhere;
  - Close, and the dialog's text as a read-only note.
- **Native speech found along the way:**
  - a list view's check state after its last column (`90aadde`);
  - an unlabelled list view is not named after its dialog, though a list box is (511);
  - "To edit the selected item press f2." for list views that rename in place;
  - a read-only edit: "read only edit", its line, "Use your reading keys to read the text.";
  - after a level change, a tree item says its parent: "level 1 | Primary Navigation | 1 of 1 |
    Banner". This fixed JAWS's Document Regions in 241, 256, 274, 577 and 587, but not in a
    WinForms tree (303/s8);
  - "Closed" is said as a word outside WinForms trees;
  - the rules tree says "N Active".

### Harness

- **Text into JAWS's dialogs goes through `TypeString`** (`d943614`), as keys go through
  `TypeKey`. Text typed by the lab never reached JAWS's elevated wizard (741). With Okur
  reading, `TypeString` types the text itself. Replay takes it as the user's text in Okur's
  window (`e5a4532`).
- **A scenario can continue the one recorded before it** (`f225d6f`): `"continues"` names it.
  Replay runs that one first on the same engine and keeps the Flexible Web rules Okur saved
  there. 748 continues 743, and 751 continues 750. On old-pc the scenario continued has to run
  just before, in the same batch: round 170 ran 743 again only for its rule.
- **The scripts on old-pc were redeployed** (`lab-deploy.sh`, binaries left as installed), so
  `queue-batch.ps1` installs `okur-ui.exe` and `run-scenario.ps1` knows `TypeString`.

## Figures

Each is the named command's output at `16a73a2`:

- `okur-lab parity`: "JAWS parity (Ia2): 5889/6111 steps match exactly; 158 accepted
  divergences not counted (0 scenarios could not run)". At the previous handoff: 5718/5927 with
  143.
- `okur-lab parity --spoken`: "JAWS spoken parity: 5743/6073 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 409 scenarios, 1267 trials, 1267 successful.
- `cargo test --workspace`: 1307 passed, 0 failed, 199 ignored.

## Learned

- **JAWS's Flexible Web stores rules in `FlexibleWeb.db`**, a SQLite database in the user
  settings folder. It was only searched for the test rule's name, to check that old-pc was left
  clean; its layout was not read, and Okur keeps its own file.
- **Saving a rule from the question leaves the focus in JAWS's window** (746/s21-s23), as Escape
  there does (742).
- **The rules dialog:**
  - F2 opens an edit on the rule's name; Enter keeps the new name (751/s18-s20).
  - Space speaks the row, "checked", then the row again with its Set for (751/s9).
  - Right Arrow on a closed area says only "open" (748/s10, 751/s11).
  - Moving into the site after the dialog opens once said the level, its position and parent
    (744/s9, not imported), and once the item alone (748/s9).
- **JAWS does not apply a Start reading at rule either.** After F5, reading starts at the top
  (750/s22).
- **old-pc has no rule left:** 749/s3, and the database holds no "Okur" string.

## Recorded meanwhile, and queued

Round 173 (`waiter173`, three trials each) is recording 752 and 753: the JAWS cursor in Notepad
and on a page. 516 found that JAWS's cursor reads "blank" in the PowerShell dialog fixture, so
these test whether JAWS's screen model sees Notepad and Chrome at all, before the Okur cursor's
reading is designed.

## Not done, and why

- **Renaming a rule (F2), Right Arrow's "open", Space's speech in the rules list**: recorded in
  751, not built yet.
- **Starting the reading at an element on load**: JAWS does not do it (750), and the guide says
  it should; to build as an accepted divergence, as hiding is.
- **748's remaining differences**: the dialog's text on opening, the note's line (the trace has
  no line bounds for a Win32 edit), and "dialog" said plainly on returning from the question.
- **Unchanged from plan 14**: the settings dialog's labels, capitals and line pauses in Say All,
  live regions and indentation, Settings Center, Say All progress in replay.
