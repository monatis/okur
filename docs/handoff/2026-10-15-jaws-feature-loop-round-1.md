# Handoff: JAWS Feature Loop, Round 1 (2026-10-14 to 2026-10-15)

**Branch**: `jaws-feature-loop`.
**Plan**: `docs/plans/2026-10-14-full-ui-accessibility.md` (its "Next" section: the discovery
loop). Next plan: `docs/plans/2026-10-15-jaws-feature-loop-round-2.md`.
**Preceded by**: `2026-10-14-full-ui-accessibility-round-1.md`.

## Numbers (tool output, pasted verbatim)

```
$ sr-cli parity | tail -2
JAWS spoken parity: 171/172 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 818/844 steps match exactly (0 scenarios could not run)
$ sr-cli parity --api uia | tail -1
JAWS parity (Uia): 719/844 steps match exactly (0 scenarios could not run)
$ sr-cli invariants | tail -1
Buffer invariants: 0 violation(s)
$ sr-cli corpus-manifest
Corpus manifest is current.
$ cargo test --workspace  (summed test-result lines)
tests: 834 passed, 0 failed, 198 ignored
```

At the start of the round: 561/569, and no spoken layer.

The "spoken" line compares what our synthesizer is given with what JAWS sent its
synthesizer. It now includes each fragment's pitch.

Steps still differing (`sr-cli parity`):

| Step | Why it differs |
|---|---|
| 01/s1, 111/s1 | Say All capture boundaries (as before) |
| 104/s1, 12/s1 | Level keys from the top of a page also speak the h1 (as before) |
| 204/s1 | JAWS's region context on the first Tab varies between trials (as before) |
| 207/s87, 209/s1 | Insert+T in Chrome: JAWS speaks the window caption with the profile name. Fixed in 720f1c9, but those traces predate trace format 7 (no caption recorded), so re-record. 207/s87 also has JAWS's "main region level 2 heading is Character review" after the title (unexplained). |
| 209/s7 | One trial of JAWS ends the font viewer with "message" |
| 303/s6, 305/s13 | As before. In 305/s13 the field held `bo d` with 0–3 selected; JAWS said "do" in all five trials. |
| 306/s7 | Alt+Tab. The trace shows only `ForegroundStaging` events while JAWS said "Task Switching". The UIA focus handler (4cfd748, 387ce99) is meant to close this. Re-record 306 and record 317. |
| 312/s3, s4, s9, s14 | JAWS's virtualized control text lacks the line's final `·😀` (unexplained) |
| 313/s7 | Ctrl+A in a WinForms edit: JAWS read only the first line; we read the whole selection, on purpose (user report: JAWS reads all of it in Notepad) |
| 314/s1–s8 | Classic Notepad Say Font: these traces are format 6, from before the tracer read fonts, so replay says "No font". The viewer steps follow from s2. Re-record. JAWS said "Consolas 22 point". old-pc is at 125% scaling and the fixture sets no font, so check old-pc's Notepad font. |
| 314/s10, s11 | Virtualize Window in Notepad: JAWS includes the menu bar and status bar |

The spoken-layer difference is 314/s11, the same Virtualize Window gap.

## What was done

Each item started from a user report or from the loop's discovery.

### Speech
- **OneCore voices** (604a560), the default on Windows: `--engine auto` tries OneCore, then SAPI 5, then no speech.
- **Live stalls removed** (ea318f9):
  - utterances are split into chunks that synthesize while the previous one plays;
  - an interrupted synthesis is cancelled, never waited for;
  - appended and punctuation silence are set to Min, as NVDA sets them;
  - the audio stream runs between chunks.
- **Character names** (0485438) as JAWS's SAPI profile speaks them: space, punctuation, symbols, and emoji by Unicode name. Code review needs these.
- **Capitals read alone are pitched up** (e2a0be4). JAWS sent them at SAPI pitch +4; OneCore gets `AudioPitch` 1.4 and SAPI 5 gets `<pitch middle>`.
  - The spoken layer compares the pitch. A mutation check with pitch 0 drops it from 171 to 167.
  - Not yet heard live: OneCore's 1.4 is a guess at SAPI's +4.

### Commands
- **Say System Time and Date** (23c7809): Insert+F12, and pressed twice quickly.
- **A configurable JAWS key** (2a2bbe8): Insert, plus CapsLock in the laptop layout.
- **The Virtual Viewer** (a1e6649): Insert+Alt+W, Virtualize Window and Virtualize Current Control.
- **Say Font** (2378999, 664843d), in pages and in native fields.
  - Native fields read the font through UIA text attributes or WM_GETFONT.
  - Pressed twice, it shows the result in the viewer.

### Native applications
- **Notepad's lines** (0485438): RichEdit ends lines with a bare `\r`.
- **Whole selections are spoken** (9c14f76). Live, only the actions worker reads after an edit key, and it waits until two reads agree, so a growing selection is read whole.
- **The desktop** (2511279, 7f4bba6):
  - Insert+T speaks the virtual desktop's name ("Desktop 1"), read from the registry;
  - the entry is headed "Desktop", which comes from the user's report, with no evidence yet;
  - a list item in another window ends desktop mode.
- **Alt+Tab with Alt held** (4cfd748, 387ce99):
  - a system-wide UIA focus-changed handler wakes the tracer only while the foreground window is the task switcher or `ForegroundStaging`;
  - the tracer records a `uia_focus` event.
- **Window titles** (720f1c9): the top-level caption is preferred over the UIA name, as JAWS does.

### Live pipeline (5fb20ee)
- The event thread drops events the engine never uses, and drops value and state changes from background processes, before any cross-process read.
- Pages are refreshed by a coalescing worker.
- UIA calls time out after 1 s.
- The keyboard hook waits at most 150 ms for the engine.

### Harness
- **The SR Speech Recorder SAPI voice** (`crates/sr-sapi-recorder`, ae86d8e) records what JAWS sends its synthesizer, as `observation.spoken` and `spoken_records`.
- **`run-scenario.ps1`** gains `repeat` (double presses), `capture_clipboard`, and `key_down` / `key_up` (holding Alt).
- **Provenance** records the culture, the UTC offset and JAWS's settings.
- **`sr-cli evidence --trace`** (2f39ee5) lists each step's traced events under its transcript.
- **Trace format 7** records the top-level caption and the virtual desktop.

### New evidence
Scenarios 207, 209, 307, 308, 309, 311, 312, 313 and 314 (see their `corpus:` commits).

## Not finished

- **old-pc has been offline since about 01:30** (`rendezvous error: target_offline`). These are written but not recorded, and sit untracked in the working tree:
  - `corpus/scenarios/208-web-textarea-character-review.json` (re-record with the fixed tracer);
  - `315-desktop-richedit-lines-and-font.json`, with fixture `corpus/fixtures/desktop-richedit.ps1`;
  - `316-desktop-show-desktop.json`;
  - `317-desktop-alt-tab-hold.json`.

  A scenario without evidence counts as "could not run", so they are committed only with their evidence.
- **Native Say Font's paragraph formatting.** The user reports JAWS in Notepad: "font is: consolas 11 point, paragraph formatting: left justified". The exact tokens come from 315.
- **Not yet verified live, needing the user's Windows 11 or old-pc:**
  - Alt+Tab with Alt held;
  - Win+M ("Desktop");
  - Insert+T on the desktop.

## Lab notes (old-pc)
- **JAWS uses the "SAPI 5X 64" voice profile with the SR Speech Recorder voice, not the original Eloquence profile.** Settings: punctuation Most, rate 11, Upper Case Pitch Change 20.
  - `DEFAULT.JCF [Voice Profiles] ActiveVoiceProfileName=SAPI 5X 64`.
  - To revert, use Insert+Ctrl+S on old-pc.
- **The recorder's COM class and voice token are registered under HKLM** (`install-speech-recorder.ps1`; `-Uninstall` removes them). This used old-pc's silent elevation (`ConsentPromptBehaviorAdmin=0`), through `C:\Users\ailabs\sr-harness\recorder\elev.ps1`.
- **Before recording:**
  - close Notepad and Chrome;
  - minimize the JAWS window if it is in front, since it took the foreground in trial 1 of 207 and 208.
