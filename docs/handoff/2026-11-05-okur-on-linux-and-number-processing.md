# Handoff: Okur runs on Linux; JAWS's Number and Date Processing (2026-11-05)

After [`2026-11-05-linux-lab-week-1.md`](2026-11-05-linux-lab-week-1.md). Plans:
[`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md) (Linux, its keyboard
section updated), and the active plan's §3 (JAWS's features from Settings Center).

## The owner (2026-11-05)

- Allowed the firewall prompt on old-pc (COM Surrogate); `localhostForwarding=false` stays.
- A dictionary rule's languages are the machine's installed ones: an accepted divergence
  (`6b9ddfe7`, `docs/references/accepted-divergences.md`).
- "Work in a loop; stop only when something blocks."

## Linux: Okur runs, whole, in the lab

Each part has an example under `crates/okur-linux/examples/` and a probe under
`skills/linux-lab/scripts/`, run by `job.sh` in the headless WSL lab on old-pc.

| Part | Commit | Lab result (quoted) |
|---|---|---|
| Keyboard: `X11KeyboardHook` (`okur-linux/src/keyboard.rs`) | `028c30a2`, `a71d70cc`, `d035c560` | `probe-hook.sh`: `target: a w C Down x b`, `asked: Alpha('A') Alpha('H') Alpha('W') Alpha('C') DownArrow Alpha('X') Alpha('B')`, `HOOK PASS` (H kept, W let go by the watchdog, Super+E and Super+R left to the clients that grabbed them before and after) |
| Events: `AtspiEventSource` over xa11y-linux's new desktop subscription (`subscribe_desktop`) | `3dd2a400` | `probe-focus.sh`: `focus events: GTK 4 7, Chrome 7 (six Tabs each)`, `FOCUS PASS` |
| Speech: `SsipEngine`, `LinuxSynthesizers` (`okur-linux/src/speech.rs`) | `01477325`, `023638a4` | `probe-ssip.sh`: `reached 1`, `reached 2`, `reached 3`, `speaking after the last: false`, `monitor peak 32768`, `SSIP PASS` |
| Okur whole: the runtime's `host::run` with Linux's platform (example `okur_linux`, the silent synthesizer) | `023638a4` | `probe-okur.sh`: `[Event Spoken] Edit \| comboboxentry \| Type in text.`, `[Event Spoken] check box not checked \| To check press Spacebar.`, `[Event Spoken] Edit \| Type in text.`, `[Event Spoken] Button \| To activate press spacebar.`, `OKUR PASS` |

Neutral parts in `okur-linux-model`, tested on every OS: `keys` (a keycode and keysym as Okur's
key, punctuation by its US place as JAWS takes it; the character; the modifiers), `focus` (an
AT-SPI element as the engine's focus event, AT-SPI's roles Okur lacks mapped provisionally), and
`ssip` (the protocol's text, lines and scales). Linux code is checked on the dev box with
`cargo clippy --target x86_64-unknown-linux-gnu`, then built and run in the lab.

What the first live runs found, each fixed:
- **x11rb does not ask for the Generic Event extension's version**, and the X server sends
  XInput 2's events only to a client that has: the hook heard nothing (`a71d70cc`).
- **64 modifier combinations per key and window** made 626,000 grabs over the lab's 42
  top-level windows, and the server delivered no key while the test ran (bisected with the
  example `xi_events`: raw events and grabs both worked alone, `wait_for_event` in a thread too).
  XInput 2's any-modifier grab, one per key and window, works (`d035c560`).
- The event source missed GTK's first Tabs when Tabbed 2 s after it started: GTK takes the
  registration a moment later (the probe waits 4 s).

## JAWS's Number and Date Processing

Keys and values read from JAWS's shared `DEFAULT.JCF` on old-pc (settings files may be read,
AGENTS.md §1.4): `Numbers` 0 to 4, `SingleDigitThreshold`, `SpeakNumbersSepByDashesAsDigits`,
`SayNumericDates` 0 to 2, and `Dictionary` (Use Dictionary, `8e67322b`). Recorded on old-pc,
722's page with each value (1159 to 1165, 3 trials each, imported `a42ed034`):

| Setting | JAWS gives the synthesizer | Okur |
|---|---|---|
| Pairs | "1 23", "12 34", "1 00 00 00", "10 30", "20 02"; decimals and money as written | built (`212581d1`) |
| Full Numbers | "1,234", "2,002" | built |
| Controlled by JAWS | numbers in words: "one hundred twenty three", "first", "one million", "U.S. dollars and two cents" | known gap (an ignored test) |
| Some / Extended date translation | "March 2, 2002"; and "January 3" for "1/3"; dashed dates stay digits | built (`89db0ff9`) |
| Dashes cleared | "1 dash 800 dash 444 dash 4443" | built (`ec1d0f5e`) |
| Single digits at 8 | "54321", "123456" as written | built |

`okur-lab parity --spoken` over the seven, before the import (`preview-round.sh`): 177/203; left
are 1161's numbers in words and JAWS's "x86" kept whole on one Ctrl+Right (as in 407/s8, an old
open difference). After the import, `okur-lab parity`: `JAWS parity (Ia2): 12015/12585 steps
match exactly; 382 accepted divergences not counted (0 scenarios could not run)`.

The dictionary rounds 1156 to 1158 (Case Sensitive, a rule in a page, the Add dialog walked)
were recorded again after the firewall prompt was answered and imported (`48572983`); the
handoff before this one has them.

## Next

1. Linux: the engine's speech for GTK's controls (toggle buttons reach it as Generic without a
   name; xa11y-linux's role mapping), Okur's `okur` program on Linux (today an example), speech
   by default in the live run, and the plan's §2.2: a first web scenario run live by Okur on
   Chrome in the lab, its result written by a tool.
2. JAWS: numbers in words (Controlled by JAWS); the Dictionary Manager's opening phrasing (open
   in every dictionary scenario); the help layer's 628, pending since 2026-10-19 (its
   Shift+Slash is another key on old-pc's Turkish layout); Window List's Delete (row 29).

## Later the same day

- **Numbers in words** (Controlled by JAWS, Okur's "Controlled by Okur") built from 1161
  (`f1ca436b`): `okur-lab parity --spoken --only 1161`: `27/27 steps match exactly`; its "$0.02"
  ("zero zero U.S. dollars", JAWS's zero twice) an accepted divergence. After it, `okur-lab
  parity`: `JAWS parity (Ia2): 12013/12583 steps match exactly; 384 accepted divergences not
  counted (0 scenarios could not run)`.
- **628 again inconclusive** (round-628, not imported): Shift+Slash after Insert+Space (":" on
  old-pc's Turkish layout) gave, in trial 1 only, "heading level 2 General Commands" at the next
  step's Escape; after that every step of every trial was silent, Insert+Up included. A live look
  at old-pc (`okur-lab keys` and `jaws-history`, skill Workflow H) is needed before another round.
- **Chrome's AT-SPI tree of a page** (`probe-atspi-tree.sh`): "document web" (named by the title,
  its text U+FFFC per child), "heading" with `tag=h1` and `level=1`, "paragraph", and "static"
  text leaves holding the text, as Chrome's UI Automation tree has them. The Linux virtual
  buffer (the plan's §2.3) can follow `okur-windows-model::uia_snapshot`'s conversion: an AT-SPI
  snapshot format in `okur-linux-model` (role name, name, attributes, states, text), captured
  live (xa11y-linux extended with attributes and text), its `to_aom` tested on snapshots of the
  HTML fixtures, then `DocumentLoaded` from the event source when the focus enters a page.
- **Okur on Linux reads a page** (`c4360c7b`, `646923d3`): xa11y-linux reads an element's AT-SPI
  subtree (`AtspiTree`), `okur-linux-model::page` keeps it as a snapshot format and converts it
  to the AOM, and the event source sends `DocumentLoaded` when the focus lands on a page.
  `probe-okur-web.sh` (Chrome on doc-reading.html, Okur's silent synthesizer and speech log):
  `page read ms=26`, then the speech log `Document Reading Reference Fixture document`, `Page has
  1heading and no links`, `heading level 1 The Accessibility Engine Specification`, the three
  paragraphs (the page read on load), and `WEB PASS: Okur on Linux read the page's heading and a
  paragraph`. Still to do for §2.2: a page the focus enters below its document (a link), the
  keys' speech checked one by one, and the result written by a tool into a Linux baseline.

## Evening: the layer's help, Command Search, and the Linux live path

- **628 recorded and imported** (`57d10ed5`, `e8362a16`), after the tracer learned to keep JAWS's
  help text, Command Search's list and its viewers' addresses out of traces (`c2df3f56`,
  `e194e93f`, `efe10b59`; `privacy::without_jaws_help_text`). JAWS's Results Viewer and its
  Command Search window are elevated: the scenario's keys there are `key_ui_access` steps, and the
  UI Access allow list holds `CommandsSearchViewer`.
- **The Insert+Space layer's help** (Shift+Slash after Insert+Space, `d12f4548`) and **Command
  Search** (J after Insert+Space, `354045d9`): Okur's commands found by name as the user types,
  the first found said as it changes, Down to the results, Enter runs one. The help's words are
  Okur's (628/s3, `c3112234`), and so are the commands found (628/s7, s8, `2cad8397`).
- **Replay of Okur's own windows** (`61b756a5`): JAWS's Command Search is a page in a web view
  (msedgewebview2.exe); the dialog mirror now reports the focus in it as okur-ui's native window
  would (in okur-ui, on no page), so typed letters reach the field, not a page's quick keys, and
  the title is said. Events inside JAWS's Results Viewer window (its web view reports from
  another process) are the viewer's while Okur's own viewer is open or was as the step began;
  where Okur has none open, the page stays the focus left (763/s23). The layer's help is a
  window of its own, as JAWS's: Escape reads the line, then the window is named (628/s4).
  `parity-snapshot.sh` before and after, diffed: only 628 changed. `okur-lab parity --only 628`:
  `JAWS parity (Ia2): 7/9 steps match exactly; 3 accepted divergences not counted`. Full:
  `JAWS parity (Ia2): 12020/12594 steps match exactly; 385 accepted divergences not counted (0
  scenarios could not run)`.
- **628's two open steps** need Okur's real window, which replay cannot draw: s6 (JAWS says "Search
  for:" twice, its page's label read as the dialog's text and again as the field's name) and s10
  (Down from the field to the results: JAWS's results are page headings with links, Okur's a
  list). Next: `run-scenario.ps1 -Reader okur` of 628 on old-pc, then `okur-lab live-parity`.
- **Linux live runs** (`bc9b91f8`): the first run files had no speech at all. The runner now keeps
  Okur's log and fails a trial in which Okur said nothing (`ab20f8ae`). With the engine's debug
  log, every key went `mode=NativeControl` after the page was read: the Linux focus event never
  said it was in a page, which on Windows its focus chain's document says, so the virtual cursor
  never took the keys. Fixed in the event source (`ab20f8ae`): the focus's page is looked up once,
  read when new, and marks the focus `in_document`. `probe-okur-web.sh`'s earlier pass rested on
  the page read at load alone, which holds the heading and paragraphs it checks: it proved the
  page read, not the keys.
- **check-linux.sh** (`cc992f10`): in the lab, clippy with the gate's lints and the Linux crates'
  tests, which the Windows gate cannot build: `LINUX CHECK PASS`.
- **A deadlock in xa11y-linux's subscription** (`69c7fdea`): in a job of several scenarios the
  first run spoke and the later ones heard no AT-SPI event at all, though `xa11y apps` listed
  Chrome, focused. `probe-second-run.sh` (Okur and Chrome twice in one session): the registry
  held Okur's four event classes both times, and `dbus-monitor` showed Chrome sending the same
  signals both times, a focused state change among them; the second Okur heard no focus. The
  subscription read each event's element with calls on the connection whose signals its thread
  reads: zbus's socket reader awaits room in a full stream (64 places), the replies queued
  behind the signals, and the thread waiting for a reply was the one draining them. The calls
  now go on a connection of their own: `run 2, focus events heard: 1`, `utterances: 8`, as run 1.
- **A reload read again** (`9b96af9d`): on Linux, F5 brings no focus event, only
  `Document:LoadComplete`; xa11y-core gains `EventKind::DocumentLoaded` (Linux mapped; macOS and
  Windows not yet), and the event source reads a page shown at a window's top when it finishes
  loading. The engine takes it as a refresh, or as a new load after F5 or Ctrl+R, as on Windows.
- **Keys by keycode** (`38f57971`): xdotool 3.20160805 pressed and let go Alt before each
  function key, whose press fluxbox's Alt grabs took; the runner now presses keycodes through
  XTest (python-xlib). `probe-fkeys.sh`: FKEYS PASS. 712 live: `3/3 steps match JAWS exactly`.
- **The Linux conformance baseline** (`13e232c0`, plan §2.2): the lab's runs kept in
  `corpus/live/linux/` (Okur's speech, never evidence; `fetch-runs.sh`), `okur-lab live-baseline
  linux [--write]`, a test keeping `corpus/live/linux-baseline.json` current, and a write that
  refuses a regression. `Okur live on linux: 27/28 steps match JAWS exactly; 0 accepted
  divergences not counted (9 runs, 0 could not be compared)`. Open: 07-doc-prior-char/s2 said
  "D" twice, 1.5 s apart, in one run of two.
- **628 live on old-pc** (`run-scenario.ps1 -Reader okur`, then `okur-lab live-parity`): 4/9.
  Okur's Virtual Viewer never takes the focus, so the UI Access guard refused s3 and s4 (meant
  for JAWS's elevated Results Viewer); the help stayed open and took Command Search's keys.
  Fixed (`28277acf`): Okur's dialog over the viewer takes its own keys, and with Okur reading a
  refused `key_ui_access` step goes to the window in front. The rerun (`okur-lab live-parity`):
  s1 to s5, s9, s11 and s12 as JAWS, or accepted. It showed three more things:
  - Escape from the help said the line but not the window JAWS names after it: Okur's viewer
    never had the focus, so no focus event comes back live; only replay's trace named the
    window. The engine now names it itself as such a viewer closes (`fe490407`).
  - Command Search's results are a list in Okur's window ("Results List view | Say All. Insert
    + DownArrow | 1 of 40"), and JAWS reads its web dialog's label twice: both accepted
    divergences (`f270b521`; a divergence's phrase now also matches across the recorder's uneven
    spaces). Replay of 628: `8/8 steps match exactly; 4 accepted divergences not counted`, spoken
    8/8; full `JAWS parity (Ia2): 12021/12591 steps match exactly; 388 accepted divergences not
    counted (0 scenarios could not run)`.
  - The speech recorder's record held "Search for: Edit" alone: the dialog's name, said for the
    window's own focus, was cut off 24 ms later by the field's focus. A focus inside the window
    just named, no key between, is now queued after it (`b9567c2a`). Replay cannot see this (it
    records every utterance, cut off or not); the next live run checks it.
  - `--debug-log` into the Dropbox folder left no file; into `C:\Users\ailabs\sr-harness` it
    did. Keep live debug logs out of Dropbox.
  - Checked live (`628-okur3`): `Okur live: 8/8 steps match JAWS exactly; 4 accepted
    divergences not counted`, the dialog's name heard with its field. The recorder still had
    s4's line cut off by the window named after it; the window now follows the line
    (`d0b524a6`).

## Night: more of the web on Linux

- **JAWS scripts on Linux** (`62e61298`): `KeyEvent::chord` writes a key in the scenarios'
  notation and okur-core's example `script_key` prints the key Okur binds to a JAWS script, so
  `run-scenario-linux.py` plays a `jaws_script` step as the user presses it (MoveToNextHeading
  H, FocusToNextField F, SayLine Insert+Up). A test reads every bound key's chord back.
- **Twenty more runs kept** (`ca06bc68`): `Okur live on linux: 201/379 steps match JAWS exactly;
  0 accepted divergences not counted (28 runs, 0 could not be compared)`; whole matches 07, 103,
  104, 106, 107, 110, 113, 19, 30.
- **The Linux page through the IA2 conversion** (`16673ca9`): links said "Link" for JAWS's "same
  page link", the article line was missing, no table read. Chrome builds its AT-SPI and IA2
  objects from the same node; news-article.html over both pairs element for element. A page's
  AT-SPI tree now goes, as the IA2 objects Chrome gives on Windows, through `ia2_to_aom`, which
  Windows replay checks; xa11y-linux reads a link's and the document's address. A reversible
  decision: okur-linux-model depends on okur-windows-model (both neutral); the conversion moves to
  a shared crate before macOS.
- **Steps end before a character's spelling** (`995f4d27`): some characters came twice, 1.5 s
  apart ("s s"): JAWS's default spells a character phonetically after a 1.5 s pause, and the
  runner's 1.5 s quiet raced it; steps now end after 1.0 s of quiet. A failed fetch is said.
- **Linux, all 28 runs again** (`6e084e4a`): `Okur live on linux: 368/379 steps match JAWS
  exactly; 0 accepted divergences not counted (28 runs, 0 could not be compared)`. Left: table
  cell keys (109, Ctrl+Alt+arrows) and Space on a check box or a tab (20/s2, 29/s4), whose state
  changes the Linux event source does not yet take.

## Excel's monitor cells (plan §3.1)

- **Round 1135 imported** (`b99739d8`): recorded 2026-11-01 by run-scenario.ps1 at `a78afe39`
  (its hash matched), three successful trials, Excel's and JAWS's processes alone in the traces.
- **Set and read** (`b273f0c8`): Insert+Shift+1 to 0, "Setting monitor cell 1 to B2"; Alt+Shift+1
  to 0, "12 | B2", the cell read of Excel's object model (SheetCells::Cell) and answered in replay
  from the step's visible cells. Ten per worksheet (SheetPlace's new `sheet`). The engine's
  in-sheet checks go through one helper: its references to app modules fell from 23 to 19.
- **Move to Monitor Cell** (`41854a11`): Ctrl+Shift+M, "B2: 12 | 1 of 1", OK goes to the cell.
  Replay of s6 still speaks twice (JAWS's dialog reports its list box before the item); a live run
  of 1135 with Okur judges it.
- `okur-lab parity --only 1135`: `JAWS parity (Ia2): 6/7 steps match exactly`. Full:
  `JAWS parity (Ia2): 12027/12598 steps match exactly; 388 accepted divergences not counted (0
  scenarios could not run)`.
- A letter-digit rule tried and reverted: JAWS sends "B2" whole in "12 B2" and "B2: 12", but
  splits "Active Cell: D1" and "Setting monitor cell 1 to B2", and "x86" under number Pairs only;
  no rule on the text alone fits, so 1135/s5's spoken "B 2" stays a difference.
- **Round 1166 written** (`corpus/pending/`): none set, the overwrite question, a range, an empty
  cell, a title, and the list of several.

## 2026-11-06: monitor cells finished, Office's wizard, Linux actions

- **Move to Monitor Cell, live** (`9854db90`): Okur's live run of 1135 on old-pc, step by step
  to `Okur live: 7/7 steps match JAWS exactly`. The list is an unnamed list box, as JAWS's;
  UI Automation's stale focus on Excel's cell no longer names Okur's dialog over the cell.
- **The lab's Office** (`6a86918c`): old-pc's Office is not activated, and its Activation Wizard
  now opens over a fresh Excel or Word some seconds after the window and takes every key of a
  trial. run-scenario.ps1 closes it as it comes (UI Automation, WM_CLOSE). Two AutoRecover
  copies of an unsaved Book1 were moved, not deleted, to
  `C:\Users\ailabs\sr-harness\excel-recovered-backup`. The Linux lab's fluxbox runs with no key
  bindings: its Ctrl+Alt+arrows grabbed 109's table keys before Okur.
- **Round 1166 recorded and built** (`e0f80eb1`, `1267b623`, `5c86e99e`): with none set,
  "monitor cell 2 undefined" and "No monitor cells defined for Hours"; the same cell again, "The
  current cell is already defined as a monitor cell." (a statement, not the help's question); a
  range, "Setting monitor cell 2 to B2:B3", read "12 | 10 | B2:B3"; an empty cell read as its
  address; titles before the value, "Weekdays | 12 | B2"; Ctrl+End's "Last Cell" before the
  column's title. The first recording failed every trial (its Escapes had no dialog to leave);
  1166b was recorded without them. `okur-lab parity --only 1166`: `JAWS parity (Ia2): 21/21
  steps match exactly`; full baseline `matched_steps: 12048` of 12619 (corpus/parity-baseline.json).
- **Linux: Space and Enter on a page** (`d702d819`): a page's nodes are numbered depth first as
  their unique ids, the event source keeps the AT-SPI objects in the same order, and
  `LinuxActions` runs a node's first action (Chrome's "click") or gives it the focus. The
  focused control's checked, selected and expanded changes reach the engine as Windows' state
  changes. `check-linux.sh` in the lab: every test passed, clippy clean.
- **Linux live, all steps** (`67c4f2c9`): the lab's runs of 20, 29 and 109 at `d702d819`;
  `okur-lab live-baseline linux`: `Okur live on linux: 379/379 steps match JAWS exactly; 0
  accepted divergences not counted (28 runs, 0 could not be compared)`.

## Key labels (backlog rows 27 and 42)

- **JAWS's list read** (`c782d338`, research): `key-labels-walk.ps1` read Manage Key Labels
  (Settings Center, Keyboard, General; F6 to its button, Space; Enter in the tree is OK) row by
  row: 103 keys, Tab and F18 to F24 muted, Control, Shift, Alt (Menu) and Windows "Always
  Speak"; Toggle Mute, Change Label (an edit over the row's label, "Key Label Edit | + | Type in
  text."), OK, Cancel. `key-labels-store-probe.ps1` found the store: the application's JCF,
  `[KeyLabels] Space=0|Space` (0 muted, 1 spoken); Letters as `Letters=0|Letters`. The probes left
  `Space=1|Space` and `Letters=1|Letters` in old-pc's NotePad.JCF, both the defaults.
- **Built** (`f0ca32a0`): the table is `locales/en-US/key-labels.txt`, the user's changes the
  setting `key_labels`, read from JAWS's `[KeyLabels]`. The echo says a key's label or nothing
  while it is muted; Tab once spoken is named, Shift+Tab "Shift Tab"; Letters, Numbers and
  Symbols muted silence their characters. Defaults change nothing.
- **Rounds 1167 and 1168 recorded and imported** (`df098c1a`): `okur-lab parity --write-baseline`:
  `JAWS parity (Ia2): 12068/12641 steps match exactly; 388 accepted divergences not counted` (was
  12048/12619; the 22 new steps, 20 matching, nothing else moved). Left in 1167: s8, a select's
  "collapsed" said after Tab in forms mode where JAWS says none (the quick key C says it,
  229/s20); s9, JAWS's "selected Izmir a" as Shift+Tab returns to a field whose text it selects.
- **Okur's Manage Key Labels** (`fc254a00`): Settings Center's Keyboard, General ends with
  "Manage Key Labels...", opened by Space or by its button in the pane (F6, Space); Configure
  Verbosity Levels goes the same way now. The dialog: the 103 keys in three columns, the first
  selected; Toggle Mute says the row ("Tab Tab Speak"); F2 or Change Label edits the label in an
  unnamed edit over the cell (okur-ui's new `edit_column`, `ToUi::DialogEditCell`); OK gives
  Settings Center's draft the changes, which its own OK keeps. The list is "Keys" where JAWS's is
  "List1", and JAWS's property page has no counterpart.
- **Round 1169 recorded and imported** (`5d0c5697`): JAWS's dialog from Insert+6 to Escape, three
  trials (key_ui_access now reaches SettingsCenter; its first recording sent nothing, its second
  left the dialog open by a scenario mistake). `okur-lab parity --write-baseline`: `JAWS parity
  (Ia2): 12085/12683 steps match exactly; 388 accepted divergences not counted` (17 of its 42
  match). Replay reads JAWS's own Settings Center as any program's windows; what it shows of
  Okur's general reading: a list row whose column repeats its name loses it ("Zoom Speak" for
  "Zoom Zoom Speak"), the focus back from an in-place edit is not said (s32), and an edit's focus
  said twice (s35).
