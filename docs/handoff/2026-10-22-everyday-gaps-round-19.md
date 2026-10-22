# Handoff: everyday-use gaps, round 19 (2026-10-22)

Follows [`2026-10-22-everyday-gaps-round-18.md`](2026-10-22-everyday-gaps-round-18.md) and covers
the commits after it (`4549502` to `823ec70`, and this handoff's), under the plan
[`docs/plans/2026-10-22-everyday-gaps-round-18.md`](../plans/2026-10-22-everyday-gaps-round-18.md).
The next plan is
[`docs/plans/2026-10-22-everyday-gaps-round-19.md`](../plans/2026-10-22-everyday-gaps-round-19.md).

## The owner's answers and requests

None this stretch.

## Decided without the owner (reversible)

- **Mouse echo speaks as JAWS's help describes it, not as recorded.** JAWS's echo said nothing
  to any mouse the harness moved (below), so what Okur says of what the mouse rests on follows
  the Settings Center help: the unit's text, or the control's name where it gives no text by
  unit, then its type and state, then its description, each as the settings choose. The same
  text is not said again while the mouse stays in it.
- **The layer's toggles are for the session**, as Insert+2's typing echo is: Insert+Space, E,
  O and the unit keys change the session's settings, not the saved ones. Toggling mouse echo on
  and off on old-pc left no key in JAWS's user `DEFAULT.JCF`.
- **The mouse echo delay is not in Okur Settings' window**: no letter was left for its access
  key in that dialog. It is set in the settings file and on the command line, with the
  interrupt, control type and description settings.
- **Drag and drop drags** where JAWS's second press found nothing to drag in Notepad (790/s6,
  an accepted divergence, below).

## What was done

### Features (each checked with `okur-lab parity` against old-pc's evidence)

- **The mouse button locks** (`eb3ed3e`), from 788: Insert+Numpad Slash and Star hold the left
  or right button down where the mouse is and let it go, "left button locked", "Left button
  unlocked", with JAWS's capitals.
- **Clicks with Ctrl, Shift and Alt, and the mouse moved by keys** (`522cb47`), from 789:
  Numpad Slash with Ctrl, Shift or Alt ("Control Mouse Click", "alt Mouse Click"), Ctrl+Numpad
  Star for a right click, and Alt+Shift with the arrows ("Mouse Down"), three pixels a step.
- **Drag and drop** (`78f0201`), from 790: Ctrl+Insert+Numpad Slash, its first press saying
  how to drop, in JAWS's words with Okur's name; the second drags with the left button from
  where the first was to where the mouse is.
- **Mouse echo's layer and settings** (`9557de5`, `6e717fd`), from 791, 792 and 797:
  - Insert+Space, E: "Mouse echo layer"; then O "on" and "off", C, W, L and P "character",
    "word", "line", "paragraph"; Escape leaves it silently;
  - choosing a unit turns echo on, as JAWS's did (797);
  - JAWS's Mouse Echo group became Okur's settings, read from JAWS's `DEFAULT.JCF` keys
    (`MouseSpeechEnabled`, `MouseSpeechEchoUnit`, `MouseSpeechInstant`, `MouseSpeechDelay`,
    `MouseMovementStopsSpeech`, `MouseEchoSpeaksControlTypeAndState`,
    `MouseEchoSpeaksHelpAndDescription`) with JAWS's defaults, and a Mouse Echo group in Okur
    Settings (Enable Mouse Echo, Mouse Echo Instant, Mouse Echo Unit).
- **Mouse echo itself** (`823ec70`): what the mouse points to is read through UI Automation
  once it rests for the delay, or at once with Instant: the element under it and its text
  there by unit (`RangeFromPoint`, expanded). Not yet checked live (old-pc went unreachable
  before the deploy; see the plan).

### Harness

- **A `mouse` step** in `run-scenario.ps1` (`d49d7b0`): the mouse moved by "dx,dy" pixels in
  absolute SendInput steps, as a hand moves it; replay takes the step's traced moves.
- **The tracer reads what the mouse rests on** (`823ec70`): a `mouse_point` event, the element
  there with its ancestors and its text by unit, traced once the mouse has stayed 100 ms. Live,
  the engine asks for the same read (`read_at_point`).

### Accepted divergences

- **JAWS's drag and drop finds nothing to drag in Notepad** (790/s6).

## Figures

Each is the named command's output at `823ec70`:

- `okur-lab parity`: "JAWS parity (Ia2): 6267/6504 steps match exactly; 169 accepted
  divergences not counted (0 scenarios could not run)". At the previous handoff: 6197/6434 with
  168.
- `okur-lab parity --spoken`: "JAWS spoken parity: 6133/6466 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 439 scenarios, 1347 trials, 1346 successful.
- `cargo test --workspace`: 1335 passed, 0 failed, 199 ignored.

## Learned

- **JAWS's mouse echo answers no mouse the harness moves.** With echo on, SendInput moves of 6
  to 360 pixels, in Notepad (794, 795) and on a page in Chrome (796), got no echo in any of nine
  trials, after waits up to 2 s; each move only stopped speech (an empty utterance, audio with
  no text). JAWS's Alt+Shift+arrow moves were not echoed either (791). Either JAWS leaves
  injected mouse input out of its echo or its echo needs something the lab lacks; a hand on
  old-pc's mouse would tell. 794 to 796 were not imported: their silence is the harness's.
- **A mouse echo unit turns echo on** (797), and JAWS keeps the unit and the toggle for its
  run: 794's first trial began with echo on, left so by 792's unit keys.
- **The mouse echo layer's help** (Insert+Space, E, ?) opens JAWS's Results Viewer, which gives
  the recordings nothing (793, not imported).
- **JAWS's shipped mouse echo settings** (`[Options]` of its shared `DEFAULT.JCF`): off, a line,
  400 ms, not instant, movement stops speech, control type and state on, description off,
  location tones off.

## Recorded meanwhile, and queued

Rounds 204 to 212 are fetched; nothing is queued. old-pc stopped answering OpenDesk at about
04:30 (connection timed out) while the new build was being deployed; the deploy did not finish.

## Not done, and why

- **Mouse echo live**: a live run of 794's keys with Okur reading (`run-scenario.ps1 -Reader
  okur`), once old-pc answers and the build is deployed.
- **The mouse following the Okur cursor, traces with bounds**: not started.
- **WPF combo boxes' values and group names** (775, 783, 787), **"Contains text" and "_Rules"**:
  unchanged.
- **The clipboard keys** (627, needs the development certificate on old-pc), **the rest of
  verbosity, notifications' Shorten and Play Sound, toasts Focus Assist hides, applications'
  UIA notifications in the history, skim attribute rules, the Word List's summary**: unchanged.
