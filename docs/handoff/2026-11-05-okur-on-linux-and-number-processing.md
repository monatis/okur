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
  refused `key_ui_access` step goes to the window in front. The rerun is next. `--debug-log`
  wrote no file in that run: to check.
