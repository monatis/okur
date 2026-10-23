# Handoff: everyday-use gaps, round 25 (2026-10-23)

Follows [`2026-10-23-everyday-gaps-round-24.md`](2026-10-23-everyday-gaps-round-24.md) and covers
the commits after it (`57cbb2af` to this handoff's), under the plan
[`docs/plans/2026-10-23-everyday-gaps-round-24.md`](../plans/2026-10-23-everyday-gaps-round-24.md).
The next plan is
[`docs/plans/2026-10-23-everyday-gaps-round-25.md`](../plans/2026-10-23-everyday-gaps-round-25.md).

## Decided without the owner (reversible)

- **Toasts Focus Assist hides are not a gap** (`8dd3b5f3`): 533 re-read shows JAWS neither says
  nor keeps them. The earlier note that every trial's history listed its own toast was wrong:
  trials 1 and 2 showed an earlier run's toast (26 items), trial 3, whose banner showed, its
  own (27). The README's Not yet line went.
- **820 stays a known gap** (`e1891e82`): experiment 11 registered on the button's own element
  (made from its IAccessible) besides the root and the window; every registration answered Ok,
  none heard the notification; both elements' providers are UI Automation's MSAA proxy. A .NET
  Framework WinForms control's notification is heard in its own process only.
- **Smart Navigation's units wait** for more recordings: with it on, Down said nothing once
  (955/s12), as if the lines were the screen's (JAWS's Screen Layout, which the research says
  Smart Navigation switches to). The toggle and the setting are built; the units, the table's
  arrows and the layout are not.
- **954 and 957 are kept as recorded**: their K presses are JAWS's PlaceMarker key, not a link
  key, so their arrows ran from the page's top; the evidence pins the scenario file's hash, so
  the names stay.
- **The console is recorded again** (962-964, round 273): its traces were made before the
  tracer read a console's screen.

## What was done

### Settings

- **Insert+S and Insert+X** (`57cbb2af`): `screen_echo` (`[Options] ScreenEcho`) and
  `smart_navigation` (`[HTML] SmartNavigation`, a bit pattern its `DEFAULT.JCF` comments: 1 a
  control, 2 a table cell, as units of the character and word keys), with JAWS's toggles: "All",
  "None", "Highlighted"; "Smart Navigation: Controls and Tables", "Smart Navigation: Off" (913).
- **Screen Echo in a classic console** (`ec8d05f7`): JAWS reads a console's new output after
  Enter, the command's output and the next prompt, at highlighted and all, and nothing at none
  (962-964); Okur said nothing. The tracer notes `EVENT_CONSOLE_UPDATE_*` and reads the
  console's visible rows once its writing pauses for 150 ms; the engine says the rows after the
  longest stretch of the old rows' end (`screen_echo::new_rows`). Live regions in Chrome are read
  alike at none, all and the default (960, 961), so Screen Echo leaves them.

### Verbosity (round 267, imported: 918-953)

- Short above Beginner: Insert+E's "Not in a dialog", Insert+Q's "Default settings." and the
  application's name with a comma, the Okur cursor's restriction "Application", Notepad's
  "Leaving menus" for "leaving menu bar", and Insert+Tab's text analysis without "column"
  ("Stray punctuation { at 11", "initial cap t at 16"); "Virtual Find" left out before Okur Find
  (`4b9185bc`, `e61a8966`).
- At Advanced, an edit combo's item said without its place as the arrows move it (935/s3, s4).
- Round 267's preview went from 501 to 507 of 538 steps; the rest are the Beginner bases' own
  differences (510, 546, 547, 774), JAWS's accidents already accepted (F3 after its Find, the
  PC cursor's place at 125%, the Font dialog's font; `ae3f32eb`), and 920's trials that disagree
  with each other. README: verbosity's Not yet line is now JAWS's Configure Verbosity Levels
  dialog (`047c1f5f`).

### The virtual cursor

- Left and Ctrl+Left at the page's top say the first character and word again, as JAWS does
  (957, `91bf9d75`).
- Found, recorded next (round 272): Okur's buffer holds a button's, a check box's and a radio
  button's role and state words as text ("Count Button"), so Ctrl+Right lands on "Button" where
  JAWS goes to the next line's word (958/s20).

### The performance review

- **P18** (`7db15baf`): the live reader hooks no location changes; each caret move cost an MSAA
  object read for nothing (the caret is read after a key by the actions worker).
- **C15** (`17801c14`): the focus to the address bar and back takes microseconds in the engine
  (20, 10 and 9 µs on 5,000 rows); no rebuild found; to confirm in a live log.
- **P16**: focus events are already read once when they name the current focus again (the
  repeat check), so stale ones cost one read.

### Harness

- The tracer records a NetUI focus's MSAA objects: the ribbon's children when Alt focuses it,
  a tab's siblings (`cf67454f`), for round 270 (WordPad's ribbon).
- `okur-lab list-notifications` (research): whether Windows lets a program read its
  notification list ("allowed" on the dev box and old-pc), printing only the toasts holding a
  given text.

## Figures

Each is the named command's output at `3c438e56`:

- `okur-lab parity`: "JAWS parity (Ia2): 8145/8473 steps match exactly; 203 accepted divergences
  not counted (0 scenarios could not run)" (7546/7815 at handoff 24, before 44 scenarios were
  added).
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `cargo test --workspace`: 1385 passed, 0 failed, 199 ignored.

## old-pc

Queued, each after the one before: round 269 (965-969, Reading Interrupt), round 270 (970-973,
WordPad's ribbon; it installs the tracer at `cf67454f` first), round 271 (974-976, Say Character
and Say Word with a selection; Insert+R at Intermediate), round 272 (977-978, the character and
word keys over controls and at a page's edges), round 273 (962-964 again; it installs the
tracer at `ec8d05f7` first).
