# Handoff: Okur's settings redesigned as JAWS's, milestones 1 to 3 (2026-10-24)

Follows [`2026-10-24-word.md`](2026-10-24-word.md) and covers its 22 later commits (`d5189fbe` to
`09608e47`) and this one. They were made under
[`docs/plans/2026-10-24-everyday-gaps-round-29.md`](../plans/2026-10-24-everyday-gaps-round-29.md)
§1.

The owner's requests in this stretch, in their order:
1. The settings and customization redesign, as JAWS's: Quick Settings and Settings Center as a
   tree with search, the window with a menu bar (Options: Basics..., Voices ▸; Utilities:
   Settings Center). Milestones 1 to 3 are built, below; 4 (Voices) and 5 come next.
2. README updated for Word: done (`94eeaf47`), and remembered for every area finished.
3. Word's remaining items scheduled right after the settings work (`a6711161`): Say All's
   announcements, Say Font's line spacing, Word's other views.
4. Punctuation by word: researched by an agent and written in the backlog (`42faadda`).

The next plan is [`docs/plans/2026-10-24-everyday-gaps-round-30.md`](../plans/2026-10-24-everyday-gaps-round-30.md).

## What was built

The design and its record of what JAWS 2025 does:
[`../references/okur-settings-redesign.md`](../references/okur-settings-redesign.md). Its §1.1
holds the old-pc walks: JAWS's menus in their 2025 order, Basics', Voice Adjustment's and Settings
Center's Tab order and speech, and Settings Center's 28 groups with every group opened.

### Settings Center (Insert+6) and Quick Settings (Insert+V): one generated tree dialog

`crates/okur-core/src/ui/settings_tree.rs`, generated from the settings table:
- **Parts:** a search box ("SearchBox Edit"), a tree, the selected item's description, OK,
  Cancel, Apply (available once something changed), and in Settings Center the Application
  box. The title is "Notepad - Okur Settings Center" or "QuickSettings - Notepad".
- **Groups:** JAWS's Settings Center groups and subgroups: User, Web / HTML / PDFs > Reading
  and Links, Forms Mode, Text Processing > General and Number And Date Processing, Speech
  Verbosity, Speech Options, Say All, Synthesizer, Punctuation, Keyboard > General, Mouse Echo,
  Miscellaneous. Quick Settings has its own, as in Notepad: General Options, Say All Options,
  Editing Options.
- **Items:** each setting is named with its value ("Typing Echo Characters"); on/off settings
  are check boxes. JAWS's tree names are used where they differ ("Select Forms Mode",
  "Punctuation Level", Quick Settings' "User Verbosity").
- **More settings:** ten that no dialog showed now appear under JAWS's names: Mixed case
  processing, Speak Dollars, Speech Mode, Identify "Same Page" Links, Say Indented Characters,
  the mouse echo details and others.
- **Descriptions:** every setting has one, in Okur's words (`<label>-description` in the
  catalog).
- **Keys:**
  - Space and Shift+Space change a value.
  - Ctrl+E goes to the search box, and Down from it into the tree.
  - Enter on a result shows the whole tree at that setting, its groups open; Escape on the
    results goes back to where the search began. Elsewhere Enter is OK and Escape Cancel.
  - Ctrl+Shift+D switches to the Default settings.
- **Default-only settings:** in an application's settings, a setting kept only for all
  applications is "Unavailable", with JAWS's reason.
- **Configure Verbosity Levels:** this Speech Verbosity item opens the old dialog, which keeps
  the per-level customization.
- **Lifetimes (milestone 2):** Quick Settings' Shift+F10 or Applications key opens a native
  menu of lifetimes: until Okur exits (the session's layer), until the focus moves (a new layer,
  cleared when another application takes the focus; Okur's own windows don't count), or saved.
- **Last Changed Settings (milestone 2):** the last group, the place's 25 latest changes, newest
  first, each named with its place ("Typing Echo Characters (User)").

`okur-ui` gained:
- tree check boxes, with the tree view's own toggle refused;
- Space and Shift+Space reported, and open/closed groups reported;
- one tree item changed in place, or the whole tree replaced;
- edits and combo boxes in list dialogs;
- a dialog's own keys, optionally only from one control;
- native context menus.

### Okur's window (Insert+J) and Basics (milestone 3)

- **The window:** "Okur", its version, and a native menu bar of JAWS's menus with the items Okur
  has: Options (Basics..., Exit Alt+F4), Utilities (Notification History INSERT+SPACE, N;
  Settings Center), Help (About Okur...). Alt+F4 asks to exit.
- **Basics:** JAWS's "Basic Settings" with Tutor Messages, Access Key, Prompt to confirm when
  exiting Okur, Show Virtual Viewer on screen, Use Keyboard Layout, OK and Cancel, for all
  applications.

## Harness

- `jaws-window-walk.ps1` and `settings-center-walk.ps1` walk JAWS's own windows with navigation
  keys only. `jaws-ui-walk.ps1` opens groups whose state is between JAWS's state marks.
- `okur-ui-tree-probe.ps1` feeds okur-ui a dialog, or the main window (`-Window`), or a menu
  (`-Menu`), over its standard input. It presses keys and writes, after each one, what okur-ui
  reported and what JAWS said of it (`-Outline` adds the UI Automation tree). It verified every
  piece above on old-pc:
  - "SearchBox Edit | Type in text.";
  - "Tree view | Speech Options Closed | 1 of 2";
  - "level 1 | Typing Echo Characters | 1 of 2";
  - "Caps Indicate not checked";
  - "Context Menu | Restore When Okur Exits";
  - "Menu bar | Options | To navigate press Left or Right Arrow.".

## Found, and how

- **The list layout dropped edits and combo boxes**, so Settings Center had no search box. Found
  by the probe's UI Automation outline (`0951d489`).
- **JAWS's tree says a group's state between state marks** (U+001D, U+001E), so the walk's
  automatic opening never fired. Found in the walk's bytes.
- **PowerShell names are case-blind**: the probe's `$outline` was its own `-Outline` switch.
- **A word that goes silent below Most** (the owner's report): the research
  ([`../references/jaws-punctuation-by-unit-research.md`](../references/jaws-punctuation-by-unit-research.md))
  traces it to the virtual cursor's, Say Word's, the Okur cursor's and the viewer's words, which
  are spoken at the punctuation level. Ctrl+Left and Right in a field already name every symbol.

## Proof

`okur-lab parity` at `09608e47` (the settings work does not touch what the corpus measures):

```
JAWS spoken parity: 8578/9300 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 8675/9239 steps match exactly; 251 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace`: 1456 passed, 0 failed, 201 ignored. Each piece of the dialogs has
tests on its model (`settings_tree.rs`, `basics.rs`, the engine's `settings_center_*`,
`insert_j_*`, `quick_settings_changes_last_as_their_lifetimes`), and the probe's outputs are
quoted above. There is no recorded scenario of Okur's own speech in these dialogs yet: that needs
Okur run on old-pc reading its own windows (next plan).

## Backlog (`docs/backlog/`)

- Added at the owner's request: setup's desktop icon, and JAWS's feedback sounds, with
  royalty-free UI sounds once their licenses are checked.
- Implemented entries removed from every backlog file and review, each removal checked against
  the commits and the parity baseline.
- Added: Virtual HTML Features' PlaceMarkers item, and punctuation by word.

## old-pc

- Idle after the walks and probes. The probes ran okur-ui alone, with JAWS reading it.
- Still the owner's to end: the windowless Word (pid 9688) and the PowerShell (4200) from the
  first Word fixture.
