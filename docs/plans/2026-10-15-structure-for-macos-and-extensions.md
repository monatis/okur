# Plan: Structure for macOS and Extensions (2026-10-15)

**Branch**: `structure`, from `main` at `5cc13e1`.
**Why now**: old-pc is offline, so new JAWS evidence cannot be recorded. Okur's core keeps
growing in three ways that a macOS port and an extension SDK
([`docs/references/extension-sdk-vision.md`](../references/extension-sdk-vision.md)) would
both have to undo:
- announcements built as strings;
- commands dispatched by string comparison;
- Windows class names in general code.

Restructuring now is cheaper than later.
**Still pending for old-pc**: the work items in
[`2026-10-15-jaws-feature-loop-round-2.md`](2026-10-15-jaws-feature-loop-round-2.md).

## Status (2026-10-15, end of the first session)

| Step | State | Commits |
|---|---|---|
| Dead code first | done: the simulated desktop and five unused formatters | `5a87202` |
| 1. Commands and keys | done | `0a94da5` |
| 2. Platform-neutral windows | done | `6185a8e` |
| 3. Application modules | done | `2fc61b5` |
| 4. Announcements as data | the `Announcement` type, native focus speech, the window heading, application modules; `SpeechFormatter` and the buffer's lines wait for a structured page-line model | `8cf7ffd`, `a7ddbf8` |
| 5. Separate fixes | Q bound to the main region; key help no longer calls K a link key; `Insert+F` dead arm and unused formatters gone with the refactors | `88eff63`, `e696275` |

Next for step 4 (2026-10-16: items 1 and 2 done in `a7ddbf8`):
1. ~~The window heading in `engine/focus.rs`: title, "dialog", text and page as lines.~~
2. ~~Application modules return announcements instead of strings.~~
3. `SpeechFormatter`, module by module.
4. The virtual buffer's lines. Design a structured page line (parts with roles and states)
   first: tonight's web features (aria-current, role descriptions, popups) add parts to it.

Spacing around the marks differs between places ("  ‹ dialog › ", " ‹ X › ",
" ‹X› "). Parity normalizes whitespace, but unit tests pin exact strings. Two ways forward:
- reproduce each place's spacing with the adjacent text, as `native_focus` already shares
  one spacing;
- unify the spacing in one commit that changes only whitespace in test expectations,
  checked against how JAWS's own speech history spaces them in the evidence.

Still open from step 5:
- Insert+F3 (JAWS's list of virtual HTML features) is bound to nothing.
- `FocusEvent::folder_name` is never set by the platform.

## The rule for every step: nothing the user hears changes

Each step is a refactor. Its commit must show, with the tool output:
1. **Replay is unchanged:** `okur parity --all` (IA2), `okur parity --all --api uia` and
   `okur parity --spoken` print the same text before and after.
   - The reference was saved at `5cc13e1`: 1,718, 1,817 and 85 lines; IA2 818/844, UIA
     719/844, spoken 171/172.
   - Parity normalizes whitespace and keeps the role marks, so this is everything parity can
     see.
2. **Every unit test passes unchanged.** A test may change only where it constructs a type
   that changed shape, never in what it asserts.
3. **The pre-commit gate passes.**

A behavior change found along the way, such as a defect, is a separate commit with its own
test. It is never folded into a refactor.

## What exists today

Inventory of `5cc13e1`, file and line numbers in the handoff at the end of this work.

- **Commands.**
  - Keys become strings in `KeyEvent::to_command_string` (`okur-core/src/input.rs`), mixing
    SendKeys notation (`{DOWN}`, `^%{RIGHT}`) with JAWS script names (`MoveToNextHeading`).
  - `execute_command` (`engine/commands.rs`, about 600 lines) compares strings, in several
    places, per mode.
  - Other key tables: the laptop table (`settings.rs`), the viewer, `EditKey`, the "moving" and
    double-press tables in `commands.rs`, plus four Key↔name tables in okur-platform and
    okur-corpus.
- **Windows names in general code.**
  - `engine/focus.rs` recognizes File Explorer, the desktop, the taskbar and Widgets by
    Win32 class names.
  - `commands.rs` recognizes dialog boxes by `#32770`.
  - `native_focus.rs` checks `SysListView32`.
  - `FocusEvent` carries `window_class`, `window_kind` (a class name), `element_class`,
    `container_class` and `process` straight from Windows.
- **Application modules.**
  - `apps/` holds terminal, Windows search and the task switcher. The engine calls them from
    six places.
  - Desktop, taskbar and File Explorer behavior lives in `engine/focus.rs`,
    `engine/commands.rs` and `engine/state.rs`, with app modes of their own.
- **Announcements.** About 100 lines write the role marks by hand:
  - `speech.rs` (42);
  - `native_focus.rs` (22);
  - `linearize.rs` (16);
  - `cursor.rs` (9);
  - the engine files.
  
  `SpeechOutput::speak` takes a string.
- **macOS.** The keyboard hook and event listener are stubs of about 45 lines each. There is no
  macOS speech, actions or clock.

## Steps, in order

### 1. Commands as values, keys as data

- **A `Command` type** in okur-core, one variant per behavior. Variants are named after JAWS's
  script names where JAWS has one (`SayNextLine`, `MoveToNextHeading`, `VirtualizeWindow`).
  Some take arguments (heading level, find text).
- **`Command::from_name`** accepts every name and alias the engine accepts today, so scenario
  steps (`jaws_script`, `keyboard`) keep working.
- **One key map:** chords to commands, per layout (desktop, laptop), declared as data. It
  replaces `to_command_string` and `layout_command`. The double-press table becomes data too.
- **`execute_command(&str)`** parses and calls `execute(Command)`, which matches on the
  type. Mode-dependent behavior stays where it is. A command means what JAWS's script does:
  `SayNextLine` moves a desktop item on the desktop.
- **Done when:** no command string is compared outside `from_name`, and no key-to-command
  mapping lives outside the key map.

### 2. Platform-neutral windows and applications

- **Semantic `FocusEvent` fields for general code:**
  - the window's kind: normal, dialog box, or desktop;
  - the application's identity: an executable on Windows, a bundle id on macOS.
  
  okur-platform fills them in.
- **Raw identifiers move into one `platform` field:** window class, top-level class, element
  class, container class. Only application modules read it, as JAWS's application scripts
  read window classes.
- **General code stops naming Windows classes:**
  - `#32770` becomes "dialog box";
  - `SysListView32` becomes a list-view kind from the platform;
  - File Explorer, desktop, taskbar and Widgets recognition moves into application modules
    (step 3).
- **Done when:** no Win32 class or process name appears in okur-core outside `apps/`.

### 3. Application modules behind one interface

- **An `AppModule` trait** with the hooks the engine already uses:
  - claiming a focus event;
  - its own announcement for a focus;
  - whether a new window is named;
  - edit feedback;
  - Virtual Viewer text;
  - command handling (from step 1).
- **The engine keeps a registry and asks the module that owns the focus first,** as JAWS
  prefers an application's script to the default one.
- **Migration order:**
  1. terminal, Windows search and the task switcher;
  2. Widgets;
  3. the taskbar, the desktop and File Explorer, with their app modes and state.
- **Done when:** the engine has no `if app == ...` branches. Application behavior lives in
  `apps/`, one module per application, grouped by platform (`apps/windows/`).

### 4. Announcements as data

- **An announcement type made of parts:**
  - text;
  - role ("Button");
  - state ("checked");
  - position ("2 of 5");
  - hint ("To activate press spacebar.");
  - line breaks.
  
  It renders the strings parity compares today.
- **Migration order:** `native_focus.rs`, the engine's window and dialog announcements, then
  `SpeechFormatter`, module by module.
- **The virtual buffer's lines** (`linearize.rs`, `cursor.rs`) carry marks inside their text
  and come last. They may need a decision of their own: a structured line model.
- **Done when:** no role mark is written outside the announcement renderer.

### 5. Separate fixes found during the inventory

Each is a separate commit with a test, after the refactor that exposes it:
- `"Insert+F"` has an unreachable second arm in `execute_command`.
- `Insert+F3` produces a command nothing handles.
- `okur run`'s key help says K moves to the next link; K does nothing.
- `Q` is handled by name but bound to no key.
- `format_frame_end`, `format_landmark`, `format_list`, `format_list_item` and `format_math`
  have no callers.
- `FocusEvent::folder_name` is never set by the platform.

### Later, not in this plan

- **The macOS backend:** hook, event listener, speech, actions. It lands on the structure above.
- **A read-only developer MCP server** over local IPC, off by default. See the vision
  document.

## Clean room

On 2026-10-15 AGENTS.md §1.4 was made explicit about what may be read:
- **May be read:** JAWS's documentation and help, its key map, user settings, and black-box
  behavior.
- **Never read:** script source, compiled scripts, or script documentation.

`skills/jaws-reference/references/script-inventory.md` dates from the initial commit
(2026-10-12). It cited `Default.JSS` line numbers and a message constant from the script
source, and told agents to inspect script sources. All three are removed. The handoff records
this.
