# Handoff: the settings' long tail, and punctuation by word recorded (2026-10-25)

Follows [`2026-10-24-word-round-2.md`](2026-10-24-word-round-2.md) and covers the 15 commits
after it (`1ebb6115` to `3ccd08e2`) and this one. They were made under
[`docs/plans/2026-10-24-everyday-gaps-round-32.md`](../plans/2026-10-24-everyday-gaps-round-32.md).

The owner's order:
1. the settings redesign to its end;
2. then the rest of Microsoft Word;
3. then punctuation by word and the README's "Not yet" list.

Where each stands:
- **Settings:** the long tail is shorter by six items: three settings in the tree, the search's
  speech, Last Changed Settings' menus and its history, the Startup Wizard, a slider's number,
  and tutor messages in their own voice. What is left is in the next plan, §1.
- **Word:** unchanged since the last handoff; its rest is what Word does not report or old-pc
  cannot record yet.
- **Punctuation by word:** recorded in a RichEdit control (1041) and built for Say Word too.

The next plan is
[`docs/plans/2026-10-25-everyday-gaps-round-33.md`](../plans/2026-10-25-everyday-gaps-round-33.md).

## What was built

### Settings Center and Quick Settings

- **The search says what it found** (`50dde786`). Each key typed in the search box says the first
  setting found and "1 of N Search Results", or "No matches found.", as JAWS's does (walked on
  old-pc: "SearchBox Edit | scr | Screen Echo | 1 of 7 Search Results").
- **Screen Echo, Smart Navigation Mode and Indicate Headings** are in the tree (`50dde786`), in
  User, Web / HTML / PDFs > Navigation and Web / HTML / PDFs > Headings and Frames, with JAWS's
  names for their values ("Echo Highlighted Text", "Controls and Tables", "Headings with
  Level"). Web's subgroups now follow the help's order. Every setting with values now has its
  names, so `Kind::Values` is gone (`018cc0d1`).
- **Last Changed Settings** (`1b6b74ac`), from two walks on old-pc:
  - an entry is "Typing Echo (User) Characters": the label, its group's path, then the value;
  - Shift+F10 on an entry offers Remove, on the group Remove All, "Unavailable" while it is
    empty; both forget at once, with no Apply;
  - the history is kept across runs, in `last-changed.toml` beside the settings files.
  A menu entry can now be unavailable (`MenuEntry::enabled`, `MF_GRAYED` in okur-ui).
- **Tutor messages in the Tutor and Message voice** (`3ccd08e2`). A tutor message ending an
  utterance ("To change the selection use the Arrow keys.") is spoken in that voice context,
  the rest in the cursor's; the speech log keeps the utterance whole, so replay is unchanged.

### The Startup Wizard (`4f435cfa`, `ea6ce5ea`)

Walked on old-pc, pages 2 to 5 and every radio group's names, each put back as found, then
Escape, which saves nothing. Okur's has JAWS's pages with the settings Okur has:
- Speech Settings: Rate, Punctuation, Typing Echo;
- Run Okur Settings: Prompt to confirm when exiting Okur;
- Common Options: Smart Navigation ("Controls Only", as the wizard names it), Forms Mode ("Semi
  Auto"), Use Keyboard Layout;
- Verbosity Settings: Tutor Messages, Access Key ("Speak all"), Verbosity Level ("Beginner,
  Highest", "Intermediate", "Advanced, Lowest").

A change is heard at once (the session layer), Finish keeps the changes for all applications,
Cancel keeps nothing. It opens from Help, Startup Wizard... (access key Z), and on a user's
first start, when Okur has no settings folder of theirs yet; never with `--settings`, so the
lab's runs never see it.

Probed with JAWS reading okur-ui on old-pc (`okur-ui-tree-probe.ps1 -Wizard`), two fixes:
- a wizard page had no slider, so Rate was never made; it now lays one out under its label;
- a window keeping one title turned its pages unannounced. Each page is now titled "Speech
  Settings - Okur Startup Wizard", and a turn reads "Common Options - Okur Startup Wizard dialog
  | Smart Navigation Off radio button checked | 1 of 4". JAWS's own wizard keeps the title
  "JAWS Startup Wizard" and says "Common Options | Smart Navigation Off ...".

### A slider's number (`4f435cfa`)

okur-ui shows a slider's value in its label, "Rate: 2", as JAWS's Voice Adjustment shows "Rate:
20". JAWS read the wizard's as "Rate: 2 left right slider | 60 percent". The number is in Okur's
own units (-10 to 10); JAWS's SAPI voice shows 0 to 20, a scale per synthesizer.

### Punctuation by word

- **A plain Win32 edit's word** (`f54c4539`) is the whole run around the caret between spaces
  and hyphens, as JAWS reads classic Notepad's.
- **1041** (`b0a7926b`): the same words in a RichEdit control, the family of Windows 11
  Notepad, at Most, None and Some.
- **Say Word in a field** (`817de792`) names every symbol, and a lone symbol is that character
  alone.

## Harness

- **`settings-questions-walk.ps1`** walked Settings Center's search, its Screen Echo, Smart
  Navigation and Indicate Headings values, and Last Changed Settings' menus (through wrapper
  scripts, as `powershell -File` drops empty arguments).
- **`jaws-window-walk.ps1`:**
  - opens the Startup Wizard by Alt+H, Z and sends page keys only when the window in front is
    titled as the wizard: counting Downs once left the menu bar closed and walked JAWS's own
    window (nothing changed there; every key was silent);
  - a `wizardvalues` part walks the wizard's radio groups each way and puts them back.
- **`okur-ui-tree-probe.ps1 -Wizard`:** okur-ui alone showing the wizard's Speech Settings page,
  then turned to Common Options as Okur answers Next.
- **`startup-checks.ps1`** says that a bare `okur run` now makes Okur's settings folder and opens
  the Startup Wizard once.

## Found, and how

- **`opendesk push` hung once** with the script half-way: old-pc kept the older copy, and the
  walk queued behind it never started. Push with `timeout`, compare the hash, then run.
- **JAWS's Settings Center search is a results list**, which Okur's filtered tree is not (an
  open item in the design, §1.1).
- **Old-pc's Notepad settings**: the Last Changed walk moved Notepad's Typing Echo to
  Characters, applied, moved it back to Off and applied, then took Remove All. Notepad's JAWS
  settings may now hold Typing Echo Off for themselves, the value it had from the defaults.

## Proof

`okur-lab parity` at `3ccd08e2`:

```
JAWS spoken parity: 8816/9539 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 8903/9478 steps match exactly; 251 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab parity --only 1041`:

```
JAWS spoken parity: 100/100 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 100/100 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab parity --only 1037`: "JAWS parity (Ia2): 97/100 steps match exactly"; the three left
are Say Word on the first stop (JAWS "\", Okur ":\").

`cargo test --workspace`: 1487 passed, 0 failed, 201 ignored (summed from cargo's result lines).
`okur-lab invariants`: "Buffer invariants: 0 violation(s)". `okur-lab corpus-manifest`: "Corpus
manifest is current." The manifest's statistics: 667 scenarios, 2007 trials, 1991 succeeded.

## old-pc

- Idle. JAWS's window ("JAWS Professional") is in front, JAWS running with its own settings and
  Rate 90; Notepad open.
- Still the owner's to end: Excel's three processes from 521's trials (9104, 10728, 12680), the
  windowless Word (9688) and the PowerShell (4200). The owner's Word document ("Document1") is
  open.
- Okur has no settings folder of `ailabs`'s own (`%APPDATA%\Okur`): scenarios pass `--settings`.
- The okur-ui on old-pc is `ea6ce5ea`'s (pushed for the probe), newer than its okur.exe.
