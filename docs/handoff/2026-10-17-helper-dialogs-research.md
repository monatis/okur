# Handoff: JAWS's Helper Dialogs, Researched; How Okur Will Build Its Own (2026-10-17)

After [`2026-10-17-docs-sheets-settings.md`](2026-10-17-docs-sheets-settings.md). The user
turned the work toward JAWS's helper features (the lists, JAWS Find, routing the virtual
cursor to the mouse, the reading highlight, the settings dialogs): "these are also what
contributes to muscle memory and mental model". They asked for a careful reading of Freedom
Scientific's *Surf's Up* guide, live tests with JAWS, screenshots, a specification, and a
decision on how to build such features across platforms: "business logic and data
representation, then delegating the UI rendering to the individual platforms ... or is there a
better option?"

## What was done

1. **Read every chapter of *Surf's Up***, from the introduction to MathML, with its quick keys
   and reading command tables, saved as text in the scratchpad. Summarized in my
   own words, never copied.
2. **Captured JAWS's dialogs live on the dev box** (research, not evidence), with the user's
   standing permission for this task, in a test Chrome on the guide's sample pages: Links
   List, Heading List, Select a Form Field, Frames List, Select a Graphic, Select a List,
   Virtual HTML Features, PlaceMarker List, JAWS Find, Select a Scheme, Run JAWS Manager,
   Quick Settings, Settings Center, Skim Reading, Flexible Web, the Virtual Viewer window, the
   empty-list messages, the reading highlight after line, word, character and heading moves,
   and the two routing commands between the mouse and the virtual cursor. For each dialog:
   a picture, JAWS's speech on opening, and `ReadBoxInTabOrder`'s reading.
3. **Wrote the specification**,
   [`references/jaws-dialogs-and-visual-features.md`](../references/jaws-dialogs-and-visual-features.md),
   with pictures in [`references/jaws-dialogs/`](../references/jaws-dialogs/). Pictures that
   showed the sample pages' text at length were left out; the two highlight pictures are cut
   to one heading.
4. **Decided the architecture**,
   [`references/helper-ui-architecture.md`](../references/helper-ui-architecture.md): dialogs
   described as data in okur-core and rendered natively by a separate `okur-ui` process
   (Win32, then AppKit, GTK); Okur reads them through the accessibility API like any other
   application's; document windows for the viewers; the highlight as an output overlay drawn
   by Okur. Six options compared (speech-only, native per platform, AccessKit toolkits,
   native-widget toolkits, web views, scripted renderers such as PowerShell).
5. **Wrote the plan**, [`plans/2026-10-17-helper-dialogs.md`](../plans/2026-10-17-helper-dialogs.md),
   now the active plan in AGENTS.md: spike S1 (a real Links List on Windows) first.
6. **Fixed `okur keys`** after it left a modifier stuck down (below).

## Findings that shaped the decision

- JAWS's dialogs are ordinary Win32 dialogs (`#32770`, standard controls, mnemonics, default
  buttons), read by JAWS's ordinary dialog reading. So Okur's own should be real dialogs read
  by Okur's ordinary reading.
- The list dialogs belong to the elevated `jfw.exe` and are topmost. From normal integrity,
  `okur window-text` on the open Links List returned only "Links List": UI Automation sees
  their titles only. Our tools could not inspect Okur's dialogs either, if they lived in a
  UIAccess Okur. Hence the separate, normal-integrity `okur-ui`.
- The Virtual Viewer window is a separate process, `VIRTVIEW.EXE`, at normal integrity.
- The highlight is one full-screen window of `jfw.exe`, class `OverlayWindowClass_<GUID>`,
  extended style `0x0A0800A8`: layered, transparent to the mouse, topmost, a tool window,
  never activated, composited. It stays above JAWS's own dialogs, still showing the page's
  last rectangle.
- Insert+NumPad Minus said "Route JAWS to PC" and moved the pointer onto the heading under the
  virtual cursor; Insert+NumPad Plus said "Route PC to JAWS" and brought the virtual cursor to
  the element under the pointer (Insert+Up then read that heading). The pointer was put back
  where it was.

## Incident: a modifier left down on the user's machine

- **What happened.** At 07:58, Ctrl+Shift+Insert+Down (sent with `okur keys`) opened JAWS's
  Skim Reading dialog between the chord's key events. JAWS runs elevated, so Windows dropped
  the key-ups that followed, and left Shift was down for every application until 08:07.
  Meanwhile:
  - the next Escape became Shift+Escape and opened Chrome's Task Manager in the test Chrome
    (closed with `WM_CLOSE`);
  - Insert+F5 became Shift+F5 and reloaded the page;
  - Ctrl+Right selected words, and JAWS said "selected".
  
  It was found by `GetAsyncKeyState` and released with a key-up. At 08:21 the same happened
  with Ctrl and Shift after Ctrl+Shift+K (the PlaceMarker List), released a minute later.
  If the user typed between 07:58 and 08:07, their keys were shifted.
- **The fix, in `crates/okur-platform/src/harness.rs`.**
  - A chord is sent in one `SendInput` batch, so all its key-ups are queued before its command
    can open a window.
  - Keys still down are released before the first step, once an allowed window is in front,
    and named on stderr.
  - After the last step, a key still down is an error. When a higher-integrity window is in
    front, Windows hides the key state from us (`GetAsyncKeyState` read "up" while Ctrl and
    Shift were down), so the command says so instead of checking.
- **Checked live** on the dev box after the fix:
  - Insert+T reached JAWS as one batch ("Title is | Jim's Cafe at Innisport - Google
    Chrome").
  - Ctrl+Shift+K opened the PlaceMarker List, and `okur keys` printed: `note: "PlaceMarker
    List" runs at a higher integrity level, so Windows hides the key state; the next `okur
    keys` with a normal window in front releases any key left down`.
  - After closing the dialog, no key was down, by `GetAsyncKeyState` over both Shifts, Ctrls,
    Alts, Windows keys, Insert and CapsLock.
- Also recorded in [`verification-loop.md`](../references/verification-loop.md) §8 and the
  skill's Workflow H.

Also seen: from 08:04 to 08:06 JAWS's process reported `Responding = False` while it kept
speaking and answering FSAPI. It recovered by itself and was never restarted.

## State of the tree

- Commits: `85c680e` (the `okur keys` fix, through the commit gate: format, clippy, the
  full test suite, reference separation), then the documents and pictures.
- Before it: `cargo clippy --workspace --all-targets --all-features -- -D warnings
  -D clippy::indexing_slicing -D clippy::string_slice` printed nothing;
  `cargo test -p okur-platform harness`: `test result: ok. 1 passed; 0 failed`.
- No engine behavior changed, so parity is unchanged.

## Next

Plan step 1, spike S1: `okur-ui-model`, `okur-ui` with a Win32 Links List, `MockUiHost`, and
Insert+F7 opening it from Chrome. Before Okur's speech is compared, record JAWS's lists on
old-pc (the plan lists the scenarios).
