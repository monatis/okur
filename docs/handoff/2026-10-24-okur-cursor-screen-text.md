# Handoff: the Okur cursor's screen text, experimental (2026-10-24)

Follows [`2026-10-23-everyday-gaps-round-26.md`](2026-10-23-everyday-gaps-round-26.md) and covers
the commits after it (`82d401e1` to this handoff's). They were made under the plan
[`docs/plans/2026-10-23-okur-cursor-screen-text.md`](../plans/2026-10-23-okur-cursor-screen-text.md),
which the owner approved on 2026-10-23 with one condition: "keep it experimental until it's
proven", as a feature of Okur's own. The next plan is
[`docs/plans/2026-10-24-everyday-gaps-round-27.md`](../plans/2026-10-24-everyday-gaps-round-27.md).

**A note on the owner's framing.** The owner wrote that JAWS does not support this. JAWS does
recognize the screen in place: W, S and C of its Convenient OCR, recorded in 996-998. What JAWS
does not have is the other half:
- a cursor that reads a window's text where its screen capture catches nothing (WPF labels,
  GDI+ text; 994, 995);
- a click on a recognized word that lands where the word is, with either engine (1000).

Both are what the experiment adds.

## What was built

All of it is behind Okur Settings' Experimental group: "Okur Cursor Reads the Screen's Text
and Recognizes Pictures (OCR)", `experimental.screen_text`, off by default. The replay of
ordinary scenarios runs with it off, so it leaves their figures as they were.

- **The line builder** (`screen_model`, `82d401e1`), shared with the Virtual Viewer's rows:
  - runs of text, each with its rectangle and source, make rows;
  - each piece keeps its place, so the mouse follows the Okur cursor to the character;
  - recognized words join where no accessible text lies.
- **The window's text from UI Automation** (`window_runs`):
  - controls by rows;
  - a field's and a document's visible lines with their places;
  - a field whose lines lie outside it (a WPF text box at 125%) at its own place (`fce8d727`).
- **The Okur cursor over the model.** Pages and dialog boxes keep their recorded lines, unless
  words were recognized there. A route made before the window's text arrived is routed again
  by it (`7e4cae4c`, from 1010/s3).
- **Insert+Space, O** (`82d401e1`, `e0457d3a`):
  - **W, S, C and Enter** recognize the window, the screen or the focused control with Windows'
    own OCR (Windows.Media.Ocr over xa11y's GDI capture). Okur says "OCR started using
    Microsoft OCR.", then "finished." and "Okur Cursor"; the cursor is unrestricted, the
    recognized words stand among the window's lines, and Numpad Slash clicks them.
  - **Q** says "OCR cancelled." (1006).
  - **B** is Background OCR: "Background OCR started", the window recognized again every two
    seconds unsaid, "Background OCR is enabled" in Alt+Delete, and "Background OCR cancelled"
    for Q (1001, 1004).
  - **Removal:** the recognized words go when the PC cursor returns or another window comes to
    the front.
  - **Off,** Insert+Space, O says how to turn the feature on.
- **The recognition language** (`7e4cae4c`): `experimental.ocr_language`, Okur's own language
  (en-US) by default, as JAWS's Microsoft OCR Language is English (1033). Windows' recognizer
  for the user's languages is the fallback.
- **A bug found on the way** (`6e3d477f`): Alt+Delete opened a WinForms window's system menu,
  because the hook took Delete after the application saw Alt's press. Any key the engine takes
  with Alt or Windows held now taps an unassigned key (vkE8) to the application first.

## Harness

- `okur-lab screen-text [--recognize] [--pieces]` (`b1ffeef7`): the model's lines for the
  window in front.
- **Screen snapshots** (`b5c2bce5`, `77efe7f8`):
  - A scenario with `"trace_screen": true` has run-scenario.ps1 save the window's text after
    each step beside the trace (`<trace>.screen.jsonl`), and with `"trace_ocr": true` the
    words Windows' OCR reads too.
  - Only the fixture's window is read; another window gets an empty snapshot.
  - Replay answers the engine's screen reads and recognitions from them.
  - `"okur_settings"` sets Okur's own settings for the replay, as `"jaws_settings"` sets
    JAWS's for the recording.
- **run-scenario.ps1 stops the fixture process it started** (`fce8d727`). A click that renamed
  the fixture had left three windows open after round 283.
- **okur-lab keys knows the keypad keys** by the names scenarios use (NumpadMinus,
  NumpadSlash). The replay parses a bare Shift.

## Proof

- **Hermetic tests** (`cargo test -- --list`): `screen_model` (4), `engine::screen_text` (6),
  `ocr` (3), the replay's snapshot answers (1).
- **The live run on old-pc** (research §8; `screen-probe.ps1`, research, never evidence):
  - the lab Okur with the feature on recognized the fixture;
  - it read its seven lines;
  - Numpad Slash on the recognized "Start" clicked the button ("SR Drawn Text Fixture -
    started"), where JAWS's click with OmniPage missed (1000).
- **Against JAWS**, each figure the output of `okur-lab parity --only <id>` at `7b86751a`:
  - 1010 (the window by UI Automation): "13/15 steps match exactly". The two others are the
    routed mouse's pixel, and JAWS's driver reading painted text where Okur reads the label.
  - 1011 (recognized, Microsoft's engine): "13/15". The two others are JAWS's "JAWSCursor"
    after OCR, which looks like an internal name, and "Plus" for "plus": 1011's snapshots were
    recognized in Turkish, old-pc's first profile language, and 1015 records them again in
    English.
  - 1013 (WPF): "4/17". JAWS uses its Scan Cursor there, one element per line; the experiment
    reads rows. Plan 27 takes JAWS's way.
  - 1014 (Background OCR read for twelve seconds): "36/36", but only because the replay does
    not wait between steps, so Okur's two-second refresh never fires. JAWS's own line stayed
    the same for over twenty seconds.
- **Not counted as proof:** 1012 (a click) is recorded again in round 287. Its first recording
  captured old-pc's taskbar with the console a click brought forward, and was not imported.

## Found

- **JAWS's Scan Cursor in WPF** (1002, 1003): "JAWS scan cursor", "Route JAWS scan to PC". Each
  element is a line, and Right and Ctrl+Right move inside it.
- **Graphics** (1007-1009): with `[OSM] IncludeGraphics=2`, unlabeled pictures are "graphic"
  and a checksum ("graphic 545"). The number changes when the picture does: a hover look gave
  "graphic 105". At the default nothing is said of them.
- **Background OCR** (1001, 1004, 1014): nothing unasked, and the line under the cursor
  unchanged for twenty seconds.
- Research and findings:
  [`docs/references/okur-cursor-screen-text-research.md`](../references/okur-cursor-screen-text-research.md)
  §7, §8.

## Figures

Each is the named command's output at `7b86751a`:

- `okur-lab parity`: "JAWS parity (Ia2): 8482/9052 steps match exactly; 214 accepted
  divergences not counted (0 scenarios could not run)". It was 8358/8816 at handoff 26, before
  rounds 282-286 added 236 steps.
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `cargo test --workspace`: 1409 passed, 0 failed, 199 ignored.

## old-pc

Round 287 is recording 1012 and 1015 with the fixture-only, English snapshots of `77efe7f8`.
