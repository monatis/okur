# Handoff: the Scan Cursor's mouse, and the WPF fixture's focus in the tracer (2026-11-05)

Follows [`2026-11-04-table-edges-laptop-keys-and-key-echo.md`](2026-11-04-table-edges-laptop-keys-and-key-echo.md)
(its last section, "2026-11-05, the next loop"). Plan:
[`../plans/2026-11-03-review-round-2-before-cross-platform.md`](../plans/2026-11-03-review-round-2-before-cross-platform.md),
§3. Branch `windows-ui-round-2`; `main` moved with every commit.

old-pc came back after the owner restarted it (JAWS and Okur started at sign-in).

## Found first-hand (old-pc, 3 trials each, imported)

- **1152, where JAWS's Scan Cursor is** (`c6b51f4d`): Alt+Delete on each line of the WPF
  window. The Scan Cursor moves the mouse, and the trace has every move: to the middle of the
  element whose line it reads, `((left + right) / 2, (top + bottom) / 2)` cut down, then where an
  absolute pointer move lands. Windows takes such a move's coordinates over the screen as 0 to
  65536 and gives the pointer's back cut down, so it lands a pixel left of and above most middles:
  "System", 455 to 597, middle 526, landed at 525 and said "526" (JAWS counts pixels from 1). The
  text block's middle, 960, is exactly 32768 of 65536 on a screen 1920 wide: it landed there and
  was said "961". The rule fits all 13 traced moves. Right along the field keeps the place ("781,
  534" after two) and names the field: "o ‹Edit›".
- **1153, the route and the word keys** (`38e3281f`): Ctrl+Right from the button's line goes to
  the next element's first word, "Focus", without its role, and the mouse to that element's
  middle; Ctrl+Left from it, to "words" of "Right words". End moves nothing. Routed to the button
  (focused by Shift+Tab), the mouse is on its middle exactly (679, 476: no pixel lost, so the route
  moves it otherwise than the line keys do), said "680, 477"; Up from there reads "Left words",
  the element above the mouse, not the line before the button's ("Right words").
- **The route to the WPF field is not explained:** "594, 507", the field being 594, 515 to 969,
  553 and its caret 595, 516 to 596, 551 (traced now). After Page Up (the title), "969, 553",
  the field's corner.
- **The tracer reads the WPF fixture's window as plain Win32.** Every focus event of its
  controls (MSAA object ids 38, 40, 41) carries the window alone. With the debug tracer on
  old-pc (`sr-harness/wpf-trace-probe-debug.ps1`): UI Automation in the tracer's process gives
  the window as the HWND proxy's element (framework "Win32", no children), and
  `UiaHasServerSideProvider` on it answers false, for a new client object too; `okur-lab focus`
  from another process, or the tracer started after the window is up
  (`wpf-trace-probe-late.ps1`), gets WPF's TextBox and Button. The tracer reads the window as it
  comes to the front, and the PowerShell-hosted window keeps answering that process as Win32 from
  then on. PowerShell ISE, a WPF application opened while the tracer ran
  (`wpf-ise-probe.ps1`), was read through WPF's own provider ("Console", then "Text Editor" with
  their chains), so this is the fixture's, and live Okur reads WPF applications. A change asking
  the window for its focused element (`FindFirst` on `HasKeyboardFocus`) found nothing under the
  proxy and was reverted, not committed. 1150's, 1152's and 1153's focus steps (the route's
  place, Tab, Shift+Tab) replay without the field's or button's focus for this reason.

## Built

| Commit | What |
|---|---|
| `2e004052` | Scan Cursor: the mouse to each element's middle, landing as an absolute move does (`ApplicationActions::mouse_landing`, `actions::normalized_landing`, Windows' over the primary screen); a control's role after its character. Trace format 12: the screen's size in the header (replay's mouse lands on it), the caret's place on caret events |
| `c6b51f4d`, `38e3281f` | corpus: 1152, 1153 |

`okur-lab parity --only 1152`: 13/21 (11 before; its trace predates the screen's size, so its
places are a pixel off). `--only 1153`: 14/20. Every step of 1153 the mouse moved for by line or
word matches. Compared over all three layers with `parity-snapshot.sh` before and after
`2e004052`: no other step changed.

## Figures

At `38e3281f`:

```
JAWS parity (Ia2): 11682/12228 steps match exactly; 382 accepted divergences not counted (0 scenarios could not run)
JAWS spoken parity: 11428/12294 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Uia): 10503/12114 steps match exactly; 382 accepted divergences not counted (16 scenarios could not run)
Buffer invariants: 0 violation(s)
```

(`okur-lab parity`, `parity --spoken`, `parity --api uia`, `invariants`.) `cargo test --workspace
--no-fail-fast` (4 threads), summed: 1661 passed, 0 failed, 201 ignored.

## Next

1. The Scan Cursor's open places: the route to a field ("594, 507"), Up from a routed place
   (the element above the mouse), and the route's exact move against the line keys' landing.
   The fixture's focus is missing from its traces; a scenario option that starts the tracer
   after the fixture's window is up (the tracer records the focus as it is when it starts) would
   give it, then 1150, 1152 and 1153 recorded again.
2. Backlog row 43 (missing): the speech dictionary (Insert+D, Dictionary Manager), language
   switching, audio ducking; row 39: screen echo, key repeat, typing interrupt.
3. Key labels' dialog (Manage Key Labels); Screen Layout's lines (984-993); the review's P3 and
   P10; the Office items when Office is back.
4. The misspelling gap (17 steps), parked with its findings in the handoff before this one.

## Later the same day: the speech dictionary

The next stretch took backlog row 43's dictionary (JAWS 2025 help: Dictionary Manager Basics,
read from the installed `jdiction.chm` on old-pc).

- **Found first-hand (research, then 1154, 3 trials):** with a default rule "total" as "sum",
  JAWS's speech history keeps "let total = price" and Say Word "total", while its synthesizer is
  given "let sum equals price" and "sum"; a character read alone stays itself. Insert+D in Notepad
  asks "New Dictionary | File Name: notepad.JDF"; Escape opens the default dictionary on Add...;
  Add... proposes the word at the cursor; Delete asks "Are you sure you want to delete the
  dictionary entry total=sum?". Inside its own Dictionary Manager JAWS uses no dictionary: "total
  equals sum", and "spacebar" where its shipped dictionary elsewhere says "space Bar".
- **Lab pitfalls:** the lab keyboard is Turkish Q, so a scan-coded I typed into JAWS's dialog
  gives "ı" ("prıce=cost" matched nothing): the scenario's words have no i. JAWS's Dictionary
  Manager runs elevated (JDiction.exe): run-scenario's UI Access keys now may reach it (every key
  was refused in the first run). The tracer kept the tree's other items, JAWS's own dictionary;
  it now leaves them out (`privacy::without_shipped_dictionary`), and that first recording was not
  imported. Each trial saves a user `DEFAULT.JDF` (Ctrl+S); old-pc had none, and it was removed
  after the batch.

| Commit | What |
|---|---|
| `962e4b13` | The speech dictionary: rules applied where the history's text becomes the synthesizer's, whole words, a root by `*`, Case Sensitive; Use Dictionary (Text Processing) |
| `096cdff1` | Tracer: JAWS's own dictionary stays out of the traces |
| `c9fca233`, `4249d26b` | corpus: JDiction.exe reviewed; 1154 |
| `b54c1775` | The Dictionary Manager (Insert+D) in Okur's dialogs; no dictionary while it is open; an application's rules before the default ones; replay: Alt+F4 closes Okur's dialog, and a focus without its program's name keeps the application in front |

`okur-lab parity --spoken --only 1154`: 28/34 (history 28/34). Over all scenarios
(`parity-snapshot.sh` before and after `b54c1775`) only 1154's steps changed. Left in 1154:
JAWS's own dialog phrasing (s4 "File Name:" said once where Okur says it twice; s5, s25 the
Notepad focus JAWS shows for a moment as Escape opens the manager; s6, s26 the window's title
JAWS says on the first Tab into the tree).

At `b54c1775`:

```
JAWS parity (Ia2): 11710/12262 steps match exactly; 382 accepted divergences not counted (0 scenarios could not run)
JAWS spoken parity: 11456/12328 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Uia): 10531/12148 steps match exactly; 382 accepted divergences not counted (16 scenarios could not run)
Buffer invariants: 0 violation(s)
```

(`okur-lab parity`, `parity --spoken`, `parity --api uia`, `invariants`.)

### Next, after this

1. The dictionary's rest: an application's own dictionary recorded (OK on New Dictionary), the
   Change dialog, a root word and Case Sensitive in JAWS's speech, a rule in a page; the Use
   Dictionary key in JAWS's files (change it in Settings Center on old-pc and compare the files).
2. 1154's dialog phrasing (above), shared with other JAWS dialogs.
3. Then the Next list above.

### Then: an application's dictionary, a root, and the Linux decision

- **1155** (`b559f208`): Enter on New Dictionary makes Notepad's own dictionary
  ("notepad.jdf - JAWS Dictionary Manager"); a root "tot*" as "sum" (typed by JAWS's TypeString)
  is spoken "let sumal equals price": **a root replaces its own letters and keeps the rest of the
  word**. Insert+D then opens the application's dictionary at once; Change... opens "Change
  Dictionary Definition | Actual Word Edit" (no colon). old-pc holds no user dictionary afterwards.
- `d71f9cb9`: a root replaces its own letters; the manager's tree has a rule selected from the
  start, so Change... works (it did nothing, and the Escape after it closed the manager).
  `okur-lab parity --spoken --only 1155`: 25/29 (17 before); only 1155 changed over all scenarios.
- The left differences in 1154 and 1155 are JAWS's own dialog phrasing (above). A rule that a
  colon-ended text is a label, not a prompt, fixed "File Name:" said twice but broke JAWS's Jump
  To Line ("Enter Line Number (1 to 73):" is said twice by JAWS); reverted.
- **Linux first** (the owner): the research [`../references/linux-lab-proposal.md`](../references/linux-lab-proposal.md)
  and the plan [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md), in a
  headless WSL distribution on the dev box; the owner frees ≈20 GB there first.
