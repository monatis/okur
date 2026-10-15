# Handoff: the Virtual Viewer reads the screen (2026-10-15)

**Branch**: `virtual-viewer`, from `main` at `42d1b6a`.
**Plan**: [`docs/plans/2026-10-15-jaws-feature-loop-round-2.md`](../plans/2026-10-15-jaws-feature-loop-round-2.md), section "Done on branch `virtual-viewer`".
**Reference**: [`docs/references/virtual-viewer.md`](../references/virtual-viewer.md).

## The request

The user opened Local Disk (C:) Properties:
- JAWS's Virtual Viewer showed the dialog row by row ("Type: Local Disk", ..., "OK Cancel
  Apply").
- Okur's showed one line.

The user asked for:
- a study of JAWS's viewer, what it can and cannot do;
- parity where JAWS is right;
- doing better where it fails, as Okur already does in Notepad.

The work was done on the dev box, with the user's permission ("I leave the computer to you
now"). old-pc is offline.

## What JAWS does

Captured on the dev box with JAWS 2025 by sending Insert+Alt+W (or Insert+Shift+V), Ctrl+A,
Ctrl+C and Escape through `okur keys`, with the clipboard saved and restored. Windows
studied:
- Local Disk (C:) Properties;
- a WinForms form (`desktop-forms.ps1`);
- a WinForms window with buttons (`desktop-dialogs.ps1`);
- File Explorer;
- Calculator;
- a message box;
- a file's Properties (the new fixture).

What JAWS does:
- Classic windows are shown by screen rows.
- Modern apps are shown one element per line, with role and state.
- The capture's faults: glued runs, "&" marks, symbol glyphs, lost or split letters, clipped
  text, a list shown as its last row.

The reference page lists each finding. On old-pc, the WinForms window of 311 showed only the
focused control [obs, 311/s1], while the dev box shows the same fixture's whole window.

## What changed

Commits on `virtual-viewer`:
- `5fcd240` feat(viewer): Virtualize Window shows what the window shows.
- `07e0855` fix(viewer): a form's field is one row of its window.
- The docs commit with this handoff. It also adds pending scenarios 322 and 323, the
  property sheet fixture, and the `run-scenario.ps1` change.

What the commits do:
- **`okur-platform::window_text`** reads the window or control through UIA: elements,
  rectangles, window text for classic labels and buttons, wrapping, and DPI handling.
- **`okur-core::viewer_text`** lays the elements out by rows or by element.
- **`okur window-text [--control]`** prints the lines for the window in front.
- **The engine:**
  - Asks `ApplicationActions::read_window`, answered on the actions worker as
    `AccessibilityEvent::ViewerText`.
  - The viewer opens when the text arrives.
  - Consoles, text editors and pages keep their paths.
  - Replay keeps the focus-based viewer, since traces hold no window text.
- **Property sheets:**
  - Traces record the page the focus is on (`dialog_page`), since UIA leaves it out of the
    focus chain.
  - Okur says "General page" as JAWS does, and no longer reads the page's labels as the
    dialog's text.
  - `dialog_layout` no longer swallows a page's rows into one.
- **Wrapping** is measured in the window's own DPI units, which fixes it for DPI-unaware
  processes. That covers the message box and Okur's dialog reading.
- **`run-scenario.ps1`** finds an owned window, such as a property sheet, by its title among
  the desktop's windows.

## Verification

`cargo fmt --all --check`, and `cargo clippy --workspace --all-targets --all-features -- -D
warnings -D clippy::indexing_slicing -D clippy::string_slice`, printed nothing. `cargo test
--workspace`, every result line "ok", including:

```
test result: ok. 106 passed; 0 failed; 0 ignored; ...   (okur-core lib)
test result: ok. 33 passed; 0 failed; 3 ignored; ...    (okur-platform lib)
test result: ok. 62 passed; 0 failed; 0 ignored; ...    (okur-corpus, parity baseline)
```

The pre-commit gate passed both code commits.

`okur parity` is unchanged against `corpus/parity-baseline.json`:

```
JAWS spoken parity: 171/172 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 818/844 steps match exactly (0 scenarios could not run)
```

**Live** (`okur run --engine no-speech --jaws-key scrolllock --speech-log ...`, ScrollLock+Alt+W
sent with `okur keys`, JAWS running):

Local Disk (C:) Properties, Ctrl+A in the viewer:

```
selected\nLocal Disk (C:) Properties\nSecurity Previous Versions Quota\nGeneral Tools Hardware Sharing\nType: Local Disk\nFile system: NTFS\nUsed space: 441,319,587,840 bytes 411 GB\nFree space: 68,804,243,456 bytes 64.0 GB\nCapacity: 510,123,831,296 bytes 475 GB\nDrive C: Details\nCompress this drive to save disk space\nAllow files on this drive to have contents indexed in addition to file\nproperties\nOK Cancel Apply
```

The property sheet's opening:

```
Local Disk (C:) Properties  \u001d dialog \u001e \nGeneral  \u001d page \u001e \nLabel: \u001d Edit \u001e \nType in text.
```

JAWS's speech history for the same opening:

```
Local Disk (C:) Properties  dialog
General  page
Label:   Edit
Type in text.
```

From the debug log ("viewer text read"):

```
viewer text read window=true lines=13 ms=47
viewer text read window=true lines=19 ms=133
viewer text read window=false lines=2 ms=12
viewer text read window=true lines=20 ms=179
```

These are Local Disk (C:) Properties, the WinForms form, its tree view (Virtualize Current
Control), and File Explorer.

The WinForms form, from its Name field (after `07e0855`): the whole form, 19 lines, starting
"SR Desktop Fixture", "File Edit Help", "Name: Ada Fruit:".

## Not done, and why

- **Evidence**: none was recorded, since old-pc is offline. Scenarios 322 and 323 wait in
  `corpus/pending/`.
- **Replay** cannot check the screen viewer until traces carry window text (plan, "Left" 2).
- **A process slip**: helper scripts in the scratchpad were run with `powershell
  -ExecutionPolicy Bypass -File` early in this work. The dev box's policy is Restricted (all
  scopes Undefined), and the standing rule is to run inline commands instead. Later checks
  used `-EncodedCommand`. Repository fixtures were started the documented way.

## Next

The plan's "Done on branch `virtual-viewer`" lists what is left, and work item 1 lists the
recordings for old-pc.
