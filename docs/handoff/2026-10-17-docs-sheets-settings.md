# Handoff: Google Docs with braille support, Google Sheets, Windows Settings, File Explorer, Windows Terminal (2026-10-17, night)

Follows [`2026-10-16-google-docs.md`](2026-10-16-google-docs.md). The user restarted JAWS,
allowed changing Google Docs' settings, chose the copyright holder ("M. Yusuf Sarıgöz"), and
left the dev box to the agent until 07:00: Docs, then Sheets, then Windows Settings screens.

Everything here was observed on the dev box with the user's JAWS configuration: research,
not evidence. old-pc stayed offline. JAWS's process still showed `Responding=False` all
night, but answered speech-history reads and spoke (the user said it works).

Method for every comparison: the same keys under JAWS alone (`web/jaws-session.sh`, JAWS's
speech history), then under Okur (`web/okur-session.sh`, Okur's speech log), from the same
starting state. Settings runs reopen the page first (`web/settings-reset.sh`,
`web/settings-compare.sh`, in the session scratchpad); they press only Tab, the navigation
arrows, Enter on page links and Escape, and change no setting.

## Commits (on `main`, not pushed)

- `b37fb67` chore: the copyright holder is M. Yusuf Sarıgöz (NOTICE, the workspace's authors).
- `8db50ac` feat(web): Google Docs with braille support reads as a document.
  - Docs' "braille support" (Tools > Accessibility > Accessibility settings, turned on for
    the user's account) makes the document a multi-line field whose paragraphs are objects
    of their own; Okur read its IA2 text as "￼￼￼￼￼". A compound field (`is_compound`) is
    read through UIA, with the font; the paragraph's alignment comes from IA2 `text-align`
    (`caret_paragraph_attributes`).
  - Document rules for a page's multi-line field: Ctrl+Home "top of file", Home its
    character, Say Line the line alone; Say Font with paragraph formatting in a page field.
  - A refreshed page finds a focus without a DOM id by role and name.
- `8dbdaaf` feat: Say All in a document reads its lines from the caret.
- `e2d2b50` feat(web): Google Sheets' cells, editor and grid as JAWS says them.
- `0efd27c` feat(web): Say Focus on a page, and a typed-in combo box's line.
- `5eb044e` feat(web): a page's menus as JAWS says them ("menu | Accessibility | 10 of 10 | To
  move through items press left or right arrow.", "Help | 9 of 10", "... sub menu").
- `9787cb5`, `50d1c44`, `2e0f015`, `e4ca425`, `37c10d8`, `bba359c`, `886a631`: Windows
  Settings (below).
- `2fa765d` feat(windows): UIA notifications, and Windows Settings' search suggestions.
- `66c4fa1` feat(windows): an unnamed list is named by the text just before it ("98 apps
  found List box", Home's "Recent and commonly used settings List box"), replacing
  `e4ca425`'s guess; Power's InfoBar is said as a dialog among the groups.
- `9ae8582` feat(windows): a modern app's button says its access key sequence (Calculator:
  "Alt followed by I"); Win32-style keys stay unsaid on buttons.
- `da0db72` docs: why a caret cannot start Chrome's Tab from the virtual cursor (below).
- `37b32ac` fix(windows): the Widgets module silenced every focus named "Widgets…"
  (Settings' Taskbar page skipped its "Widgets" switch); it now keys on the Widgets
  board's processes and the taskbar. A control that is a list's entry (Notifications' app
  switch, inside the app's button) keeps the groups above it.
- `30fec74` feat(harness): numpad and minus keys for `okur keys`.
- `77b23cc` feat(windows): native UIA live regions are read (their texts, then their name):
  Alt+Left back to Display now says "Brightness Change brightness for the built-in display
  | Show more settings". The WinEvent read for a native live region, which took 970 ms as
  Settings opened, is gone.
- `384de93` feat(windows): `okur run` says when it is not elevated (below, Rename this PC).
- `2f27ecf` feat: a typing echo setting (`okur run --typing-echo none`); both old-pc and the
  dev box have JAWS's `TypingEcho=0`, and replay reads it from the evidence.
- `ea1eea8`, `40e12ea`, `374f119`: File Explorer (below).
- `7a0d488` feat(windows): WinUI menus (Explorer's context menu) and leaving them.
- `4a7d6bf` fix(windows): arrows in a tree say a position only with a new level; 303/s6
  now matches (parity numbers below).
- `9103505` fix(windows): a search result selected again says its name alone.
- `247dac7`, `80100b9`: Windows Terminal (below).

## Docs and Sheets: where they stand (dev box)

- Docs with braille support, keys Ctrl+Home, Down, Down, Up, End, Home, Ctrl+Right x4,
  Right, Insert+F, Insert+Up, Left, Shift+Right x2, Left: JAWS and Okur say the same,
  including "Font is bold  Arial 11 point, | Paragraph formatting left justified".
  - Backspace says the deleted character in both; Shift+Down, Shift+End and Ctrl+A select
    with "selected"; Say All reads the lines.
  - Okur's text reads in Docs take 17–41 ms (debug log), so key to speech is 32–77 ms
    (`okur latency`, one run).
- Docs' menus, Alt+Shift+A, Left, Down x3, Right, Down, Left, Escape: the same, but for
  JAWS's "Type in text." after "Leaving menus".
- Sheets, keys across cells, Ctrl+arrow jumps, Shift selections, Tab, Enter, typing, F2,
  Escape and Delete: the same.
  - The cell editor is "D1 edit combo | n | To set the value use the Arrow keys or type
    the value."
  - Back in the grid: "group".
  - Say Line on the grid: "blank".

## Windows Settings: where it stands (dev box)

- Pages compared with Tab:
  - home; Display (`ms-settings:display`); Sound; Bluetooth & devices; Network & internet;
    Personalization; Apps > Installed apps; Date & time; Accessibility; Windows Update;
    About;
  - plus the navigation list's arrows, the Accessibility list's arrows, a subpage (Night
    light) and back, and the search box's suggestions.
  - Later the same night, 12 Tabs each, the same as collapsed text (`web/same.js`): Game Bar,
    Notifications, Multitasking, Colors, Taskbar, Default apps, Storage, Themes, Background,
    Typing, Mouse, Language & region, Location, Text size, Privacy & security. Windows Update
    loaded an "Available updates" section for JAWS's run and not yet for Okur's; the rest of
    it is the same. Opening Display (Okur running first) is the same as JAWS. Win+I too.
  - F6 does nothing in Settings, for either.
- What made them the same:
  - **The groups and lists a control is in** are said on the way in, outermost first, and
    not again while the focus stays in them: "Brightness & color | Night light", "Scale &
    layout | Scale", "Input | Input List box".
    - Built in the platform (`nested_groups`, `FocusEvent::groups`).
    - A text counts as a group; scroll panes are passed through.
    - Inside a list item the groups go on through it. Inside another control there are none
      (the brightness slider, in the expander's button).
    - An unnamed list takes the name of the group it is entered with.
  - **Toggle switches** say "On"/"Off", and no longer pass for repeats of the same-named
    button beside them.
  - **Expander buttons** say "collapsed"/"expanded".
  - **Sliders** say "left right slider", their reported value ("100"), or their range as a
    percentage ("34%").
  - **Closed WinUI combo boxes** say their selected item as the value ("125%
    (Recommended)"), not "1 of 1".
  - **Native buttons:**
    - a button with a heading level (the breadcrumb) is a button;
    - links say "Link | To activate press Enter.";
    - a button says only the position its application reports.
  - **Navigation items** (no container) keep the list's hint to the first.
  - **The search box:**
    - Up and Down move through the suggestions, read through UIA selections (Settings
      module, `PlatformIds::automation_id`);
    - UIA notifications from the application in front are said ("Suggestions available");
    - the same notification again within 2 s is said once.

## File Explorer, Windows search, Windows Terminal (dev box)

Explorer was opened on the repository's `docs` folder (no personal files); Windows search
looked for "settings"; Windows Terminal ran in a new window titled `OkurTestTerminal`, with
PowerShell in the session scratchpad. Only moving keys, selection, and typing at a prompt;
no file or setting changed. Compared as collapsed text with JAWS's speech history.
- **Explorer, Tab through the window** (toolbar, breadcrumb, search, command bar,
  navigation pane, file list, column header, view buttons, tab): the same but for the
  "Details" view button, where JAWS reads its help text ("Displays information about each
  item in the window. (Ctrl+Shift+6)") and says no position; Okur says "1 of 2". Settings'
  toggles carry help text JAWS does not read, so no rule was guessed.
  - UIA split buttons ("This PC split button | 1 of 6", "Name split button"), a reported tab
    position ("docs Tab | selected | 1 of 1"), MSAA states only from the focused control's
    own object (the view button was "not checked").
- **Explorer's list:** entering it, a folder opened with Enter, Alt+Up, first letters, Alt+D
  and Escape, Shift+Down: the same ("backlog - File Explorer | backlog | Items View
  MultiSelect ListBox | Not Selected … | Markdown Kaynak Dosyası 1 KB | 1 of 2 | To move to an
  item press the Arrow keys.", "selected | handoff"). The item's type and size are its
  `System.ItemTypeText` and `System.Size` cells. Left: Ctrl+A (JAWS reads every selected item
  with its columns), and Alt+Up, where JAWS's "Working on it..." split its reading.
- **Explorer's context menu** (Shift+F10): the same item for item ("Cut<tab>Ctrl+X | 1 of 4 |
  To move through items press up or down arrow. | Alt followed by T"), and "Leaving menus"
  on Escape. JAWS also said "Popup" and the first item again.
- **The navigation pane:** arrows say no position within a level (and 303/s6 now matches,
  the same rule). Left: JAWS's "not checked | not checked" on pinned Quick Access items (no
  checked state in UIA or MSAA) and "Closed" with a capital.
- **Windows search** (Start, typing): the same, but for the results' counts (timing) and the
  typing echo (the dev box's JAWS has it off). A result selected again says its name alone.
- **Windows Terminal:** output after Enter and the recalled line come as the terminal's UIA
  notifications. Escape reads the cleared line (the prompt); Left, Right and Backspace read
  as in a field ("c", "b", the deleted "b"). With `--typing-echo none`, the same as JAWS but
  for Home, End and Ctrl+Left (JAWS: " ", "ac"; Okur reads none of them).

## Verification

- `cargo test --workspace` (gate, each commit): passes; clippy with the gate's flags clean.
- `skills/jaws-reference/scripts/parity-snapshot.sh` then `diff -r` against the previous
  reference, after every change: identical to `5cc13e1` until `4a7d6bf`, which made 303/s6 a
  match (and 303/s5 "Pictures", still a match); identical to `4a7d6bf` after it.
  ```
  JAWS parity (Ia2): 819/844 steps match exactly (0 scenarios could not run)
  JAWS parity (Uia): 720/844 steps match exactly (0 scenarios could not run)
  JAWS spoken parity: 171/172 steps match exactly (evidence recorded through the speech recorder voice)
  ```
- New tests cover each rule (Settings pages as focus sequences, the platform's group
  chains, slider values, notifications, Sheets' editor, Docs' document keys and Say All).

## Differences left (known)

- **Settings:**
  - JAWS's "Insert f1 Help." on the brightness slider: JAWS's own timing as the page loads
    (one JAWS run said it in place of the value, another said "100" for the same keys).
  - "1 of 1" on Sound's "All sound devices" (no reported position).
  - On a page reached with Alt+Left, JAWS says the expander's live region before the focus;
    Okur says it after (its event comes 140 ms after the focus).
  - **"Rename this PC"** (About) opens a dialog in `SystemSettingsAdminFlows`
    (`Shell_Dialog`, a XAML island). JAWS reads "Rename your PC | Current PC name:
    DESKTOP-… Edit"; a UIA client without UIAccess (PowerShell's, Okur's) sees only two
    empty panes, and keys sent with `okur keys` did not reach it (it was closed with
    `WM_CLOSE`, nothing renamed). The cause is the integrity level: the dialog's process runs
    at High (0x3000), Okur at Medium (0x2000), and JAWS's `jfw` at High (process tokens).
    An elevated Okur should read it; not tried, the agent's shell is not elevated. `384de93`
    makes `okur run` say so at startup.
  - The search box: JAWS said each "No results for …" twice (a second source not found).
  - Okur echoes typed letters by default (JAWS's default); the user's JAWS has it off:
    run with `--typing-echo none` to match.
- **Docs and Sheets:**
  - JAWS's "Type in text." after "Leaving menus".
  - Sheets' Insert+F says "No font", as JAWS does. The Docs module covers documents only,
    and Sheets' formatting buttons were hidden in its toolbar's overflow at this window
    width, so a Sheets module could not be checked.
- **Tab from the virtual cursor** still skips focusable custom widgets. Tried: a caret at
  the virtual cursor (IA2 `setCaretOffset`), then the browser's Tab. Chrome's Tab never starts
  from a selection (Blink's `FocusController`, `Document::SequentialFocusNavigationStartingPoint`),
  so it went from wherever focus had last been; the code was not kept (plan, §4).
- **Calculator:** Okur says each notification ("Display is 52"). JAWS (dev box) says the
  digit typed ("2"), the expression for an operator ("5 plus"), "Square", "Reciprocal",
  "Percent" for Q, R and %, the bare result after Enter, Backspace and F9 ("8", "-9"), and
  "Clear expression" or "Clear display" for Escape and Delete, then the value if it
  changed. The display's bare value is the name of its child pane ("3"); the expression is
  `CalculatorExpression` ("Expression is 5 + "). Not built.

## Left in the user's account

- Google Docs' braille support is on (Tools > Accessibility > Accessibility settings). The
  user allowed it; it can be turned off there.
- The test document "Untitled document" and the test spreadsheet "Untitled spreadsheet" (a
  five-row table) in the user's Google Drive.
- The clipboard: one Enter meant for "Rename this PC" may have pressed About's "Copy"
  (device info) instead; the clipboard was not read to check.
