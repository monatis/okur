# Handoff: okur-ui's First Windows, the Virtual Viewer on Screen and a Settings Dialog (2026-10-17)

After [`2026-10-17-helper-dialogs-research.md`](2026-10-17-helper-dialogs-research.md). The
user agreed with the architecture and set the order: inspect JAWS's Virtual Viewer ("it can
be an actual on-screen frame"), make it `okur-ui`'s first target, then the Links List and a
simple settings dialog: "currently all our configurable things are flags to okur
executable, they should be configurable on the UI just like jaws". They left the dev box to
the agent for this.

## What JAWS's Virtual Viewer is on screen

Observed on the dev box (research), recorded in
[`virtual-viewer.md`, "On screen"](../references/virtual-viewer.md#on-screen):
- `VIRTVIEW.EXE` starts with each viewer and exits when it closes. Its window "Virtual Viewer"
  is topmost, three quarters of the screen, a read-only rich edit in Arial 12 point, and a
  "Ready" status bar.
- It never has the focus: the application keeps it, and JAWS takes every key (F5 did not
  reload Chrome's page).
- The caret follows the virtual cursor, and JAWS's red highlight frames the word at it.
- Bringing the window to the front ends the viewer within about 320 ms.
- "Show Virtual Viewer on Screen" is `bVirtViewer=1` in JAWS's default settings.

## What was built

1. **`okur-ui`** (new binary) and **`okur-ui-model`** (new crate: the JSON-lines protocol).
   Okur starts `okur-ui` beside its own executable (`ProcessUiHost` in okur-platform),
   writes to its input, and reads what the user did from its output. If it exits, it is
   started again. It ends with Okur.
2. **The Virtual Viewer on screen** (`a047048`). It is drawn as JAWS's: same title, size,
   position and window styles, a read-only rich edit, a status bar, and no activation. A red
   frame in JAWS's colour, RGB(255,115,92), marks the line, word or character read last.
   Focusing or closing the window ends the viewer. Setting: `ViewerDisplay`.
3. **Dialogs.** A dialog is described as standard controls (group, check box, choice,
   slider, button) with their exact labels. `okur-ui` lays it out and encodes an in-memory
   `DLGTEMPLATEEX` (`template.rs`, tested), so Windows creates a real `#32770` dialog. Its
   extended style, 0x10109, is that of JAWS's own list dialogs. It also:
   - draws its controls in the current style, using the Common Controls 6 activation
     context that Windows keeps in shell32.dll;
   - comes to the front by the method `okur front` uses, and returns the focus to the window
     it came from when it closes;
   - sends each change and button press to Okur.
4. **Okur's settings.** They are one serializable set: voice (synthesizer, voice, rate,
   volume), keyboard layout and JAWS key, typing echo, live regions, and the viewer on
   screen. Insert+6 and Insert+V open "Okur Settings". OK and Apply apply the change at once
   (the synthesizer included, through `SpeechOutput::configure`) and save it. The settings
   file is `%APPDATA%\Okur\settings.json`. `okur run`'s options now override the saved
   settings for one run; `--settings-file` keeps tests away from the user's own file.
5. **Okur reads Win32 dialogs more like JAWS** (all dialogs, not only its own):
   - a dialog's labels are no longer read as its static text;
   - the group box around a control is said before its name (found by geometry,
     `screen_lines::group_box_of`, recorded in traces as `group_box`);
   - access keys are said last: as UIA gives them for a labelled control, and as `alt+` and
     the letter as written for a control with its own text;
   - a trackbar's percentage is said in words.

## Verified

- **Live, dev box, Okur run with `--engine no-speech --jaws-key scrolllock --settings-file`
  (scratchpad).** JAWS and Okur read "Okur Settings" the same, on opening and on all eleven
  Tab stops, from their speech histories. For example, both said: `Speech Rate: left right
  slider | 50 percent | To increase or decrease use the Arrow keys. | Alt+r`.
- **Changes reach the file.** Choosing Typing Echo "Off" and pressing Enter saved
  `"typing_echo": "none"` to the test file. Escape returned the focus to the test Chrome.
- **The viewer, live.** It showed at 192,108, 1152×648, style 0x14CF0000, extended style
  0x108, which is JAWS's to the pixel. Escape hid it. Bringing it to the front hid it, and
  the next ScrollLock+Alt+W virtualized again.
- **Parity unchanged.** `skills/jaws-reference/scripts/parity-snapshot.sh`, before (a
  worktree at `a047048`) and after: `diff -r` printed nothing. IA2 `819/844`, UIA `720/844`.
- **Tests.** `cargo test --workspace`: all passed (okur-core 180, okur-platform 48 with 3
  ignored, okur-ui 2, okur-ui-model 3, the rest unchanged). Strict clippy printed nothing.

## Then the Links List

- **JAWS's, captured** with JAWS's own `TypeKey` through FSAPI (its dialog refuses our keys):
  - tab order: the list, Move To Link (alt+M), Activate Link (alt+A), Cancel (alt+C),
    "Sort Links In Tab Order" (alt+T), "Display All Links" (alt+l);
  - after Move To Link: "same page link Reputation for Quality Service | To activate press
    Enter."; nothing after Escape or Enter.
- **Okur's.** Three new control kinds (list view, radio group, read-only text), a list
  layout as JAWS's (list, buttons beside it, groups below), and `DialogItems` to refill the
  list. okur-core's `LinksListDialog` sorts and filters; the engine moves to or activates the
  link and returns to the page silently. Without `okur-ui` the speech-only list stays.
- **Two general fixes found on the way:**
  - the desktop module claimed any `SysListView32` outside File Explorer; it now needs the
    desktop's window;
  - an unnamed list (UIA gives none) takes its window text as its name, as MSAA, and so JAWS,
    does: "links List view".
- **Live, dev box.**
  - JAWS read Okur's Links List with its own strings.
  - Okur's opening speech: `Links List dialog | links List view | Reputation for Quality
    Service | 1 of 13 | To move to items use the Arrow keys.`
  - Down: `Dishes and Appetizers | 2 of 13`.
  - Move To Link: `same page link Driving Directions | To activate press Enter.`, and nothing
    more; Say Line then read the link.
  - Escape and Enter: silent.
- **Parity** unchanged after this too: `diff -r` against the `a047048` snapshot printed nothing.

## Found, not fixed

- When the Claude desktop window had the focus, Okur's page worker built a buffer from
  another Chrome window's page ("page refined", Surf's Up). It was flagged as a separate task:
  the IA2 capture of the foreground document seems not to be limited to the window in front.

## Next

- The Links List as an `okur-ui` dialog (plan step 1), now that dialogs, the template and
  Okur's reading of them exist. Its list view needs a new control kind (a list with an
  initially selected item, read "links List view | … | 1 of 13"), and radio groups.
- Record on old-pc what this session observed on the dev box: the Virtual Viewer on screen,
  and JAWS reading a Win32 dialog's group boxes, access keys and trackbars.
