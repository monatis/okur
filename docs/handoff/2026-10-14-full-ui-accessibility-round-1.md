# Handoff: Full UI Accessibility, Round 1 (2026-10-14)

**Branch**: `full-ui-accessibility` (from `master` @ `67c0787`).
**Plan**: `docs/plans/2026-10-14-full-ui-accessibility.md`.
**Preceded by**: `2026-10-14-ia2-paragraphs-native-focus-actions.md`.

## Numbers (tool output, pasted verbatim)

```
$ sr-cli parity | tail -1
JAWS parity (Ia2): 561/569 steps match exactly (0 scenarios could not run)
$ sr-cli parity --api uia | tail -1
JAWS parity (Uia): 487/569 steps match exactly (0 scenarios could not run)
$ sr-cli invariants | tail -1
Buffer invariants: 0 violation(s)
$ cargo test --workspace  (summed test-result lines)
tests: 805 passed, 0 failed, 198 ignored
```

At the start of the round: 497/504.

The eight steps still differing (`sr-cli parity`):

| Step | Why it differs |
|---|---|
| 01/s1, 111/s1 | Say All capture boundaries |
| 104/s1, 12/s1 | Level keys from the top of a page also speak the h1 |
| 204/s1 | The first Tab into a page: JAWS's region context varies between trials |
| 303/s6 | Moving up to a visited tree item drops its position |
| 305/s13 | Shift+Home over `bo ` speaks `selected do` (unexplained) |
| 306/s7 | The Alt+Tab task switcher's announcement varies between trials |

## What was done

### New evidence (all recorded on old-pc, traced)
| Scenario | What it covers |
|---|---|
| 304 | Native state changes |
| 305 | Editing a native text field |
| 306 | Dialogs and windows (new fixture `desktop-dialogs.ps1`) |
| 205 | Web state changes |
| 206 | Live regions (new fixture `live-updates.html`) |
| 303 | Re-recorded with trace format 5 |
| 19, 20, 29 | Converted from untraced legacy scripts to key chords, and re-recorded |

### Harness
- **Trace formats 4 to 6:**
  - context (focus chain, subtree, Win32 list position) on state, value and selection events;
  - the focused field's `TextContext` (text, caret, selection);
  - for a live region, the region and the page after the change.
- **`event_trace::EventConverter`** is used by both the live listener and parity replay. One event can produce several engine events.
- **Parity replays with `RecordingActions`,** so Enter and Space act as they do live and the speech comes from the recorded events.
- **`text_context::read_focused`** reads IA2 in Chromium, UIA TextPattern elsewhere, and window messages for subclassed Win32 edits (WinForms).

### Engine
- **Changes of the focused control** (`native_focus::change_speech`, `page_change`):
  - check boxes, radios, combo box values and drop-downs, tree expand and collapse;
  - web check boxes and radios, combo boxes, tab selection, with the buffer rebuilt from the new state.
- **Editing** (`sr-core::editing`):
  - JAWS's rules for characters, words, lines, Home and End, selection, Backspace and Delete, and Say Line;
  - typing echo on its own channel, because JAWS does not record echo in its history.
- **Windows and dialogs:**
  - the active window is the innermost window, identified by title and class;
  - `#32770` dialogs read their text;
  - Insert+T speaks the window title.
- **The JAWS key:** Insert and every Insert combination are consumed in every mode.
- **Web focus:**
  - the page title on the first focus into the page;
  - regions and the hint when Enter turns forms mode on;
  - disclosure buttons, tabs and group entry, judged from the previous focus.
- **Live regions:** the added text after the region name, then a silent page refresh, held until the change's events are handled.

### Verified live on old-pc (`sr-cli run --engine no-speech --debug-log`)
- WinForms: check box Space (`checked` / `not checked`), combo box Down (`Brazil`), Alt+Down (`Open List Box`), Enter, tree expand, message box (`SR Dialog Fixture dialog`).
- Earlier in the round: editing (Home, Right, Ctrl+Right, End) in Notepad, Chrome inputs and both contenteditable shapes.

## Lab notes
- **Explorer windows** (`Desktop\code`, `chapter-03`) appeared on old-pc and ruined first trials. They were closed; the capture's foreground check caught each one.
- **Arrow keys arrive twice in our hook while JAWS is active,** because JAWS re-injects navigation keys. Live tests of edits on old-pc therefore toggle JAWS's virtual cursor off first.

## Next
1. **Round 2 of the plan:**
   - CapsLock as a JAWS key (a setting in JAWS);
   - Shift+arrow unselect phrasing;
   - Backspace on a selection;
   - the remaining gaps in the table above.
2. **The discovery loop the user asked for:**
   1. Pick a JAWS feature from its help, manual or settings.
   2. Record it.
   3. Implement it.
   4. Prove parity.
   5. Repeat.
