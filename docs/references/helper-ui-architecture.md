# Okur's Own Windows: Dialogs, Viewers and the Reading Highlight

Decided 2026-10-17. JAWS's lists, JAWS Find, PlaceMarkers, Quick Settings, Settings Center,
the Results Viewer and the red reading highlight are part of the muscle memory Okur keeps
([`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md) records what
they are). This page decides how Okur builds them on Windows, macOS and Linux.

**The decision in one paragraph.** The data and the logic of every dialog live once, in
okur-core: what a dialog holds, how it filters and sorts, which button is the default, and
what happens when it closes. A dialog is described as data, a list of standard controls with
their exact names. A small renderer on each platform turns the description into a real
native dialog: Win32 on Windows, AppKit on macOS, GTK on Linux. The renderer runs in its own
process, `okur-ui`. Okur then reads its own dialogs the way it reads any application's
dialogs, through the accessibility API. The reading highlight is different: it is an output,
like speech and braille, drawn by the Okur process into a click-through overlay window.

---

## 1. Requirements

From what JAWS does, what Okur must be, and what was measured on 2026-10-17:

1. **Real, focusable windows.** JAWS's dialogs are ordinary Windows dialogs. Sighted helpers
   see them, the mouse works, Alt+Tab finds them, and a stray key lands in the dialog instead
   of the page behind. Typing in JAWS Find needs a real edit field: Turkish and other input
   methods, selection, typing echo.
2. **Read like any other dialog.** JAWS speaks its dialogs with its ordinary dialog reading
   ("Links List dialog | links List view | ... | 1 of 13 | To move to items use the Arrow
   keys."). Okur's dialogs must go through Okur's ordinary reading too, so the work that makes
   Okur read Settings and File Explorer dialogs like JAWS makes its own dialogs right, and
   the two can never drift apart.
3. **Exact names are data.** Titles, list names ("links", "headings", even "List1"), button
   labels, groups, mnemonics and default buttons are what the user hears. They come from the
   dialog's description and match JAWS's.
4. **One logic for every platform.** The lists' content (from the virtual buffer, custom
   labels included), sorting, filters, the first selection, the default button switching from
   Add to MoveTo, the results (move, activate, forms mode), the messages for an empty list,
   and the stores (marks per URL, labels per domain, find history per site, settings layers).
5. **Hermetic tests.** Everything but the drawing runs in `cargo test`, the speech on opening
   included.
6. **Fast.** A list opens within a frame or two of the key, and nothing a dialog does may
   hold up the keyboard hook or speech.
7. **Isolated.** A failing dialog never silences Okur. "A screen reader that stops talking
   leaves its user with nothing" ([extension SDK vision](extension-sdk-vision.md)).
8. **Integrity.** Production Okur needs UIAccess to work in elevated windows. A privileged
   process should keep its surface small, so the large UI surface belongs elsewhere. Measured:
   JAWS's dialogs, owned by its elevated process, show only their titles to a
   normal-integrity UI Automation client, so our harness could not read them. Okur's dialogs
   must stay readable by our tools.
9. **Activation.** A dialog opened by a key the hook consumed must come to the front:
   Windows's foreground rules and macOS's activation policy both stand in the way.
10. **Reading highlight.** A rectangle around the unit just read, above every window, never
    taking the mouse or the focus, with the page scrolled to it.
11. **Pointer routing.** Hit testing, element bounds, pointer moves and clicks.
12. **Document windows.** The Virtual Viewer and Results Viewer are Okur's own text, read with
    Okur's virtual cursor. On screen they need a window that shows the text and the cursor
    without taking the focus: JAWS's never has it, takes every key itself, and ends the
    viewer when the window is focused.
13. **Extensions.** The extension SDK will need dialogs. An extension should describe one and
    get an accessible native dialog, not ship its own toolkit.

## 2. Options considered

| | Option | Verdict |
|---|---|---|
| A | **Speech-only dialogs** (Okur today): the engine speaks a dialog that has no window. | Fails 1, 7's spirit and 12: invisible to helpers, no mouse, no input methods; every dialog behavior (Tab, mnemonics, radio groups, combo boxes, history, trees, context menus) re-made in Okur and different from how Okur reads real dialogs; keys Okur doesn't take go to the page. Kept only as the test host's projection (§4). |
| B | **Shared model, a native renderer per platform** (the proposal behind this page). | **Chosen**, with the renderer in its own process (§3). |
| C | **One custom-drawn toolkit with AccessKit** (egui, iced, Slint). | One codebase, but the controls are imitations: list views with positions, radio groups, mnemonics, default buttons and input methods are partial or missing, the look is foreign on every platform, and Okur would read a toolkit's tree instead of the platform's standard controls. A screen reader's own dialogs must be the most accessible windows on the machine. |
| D | **One native-widget toolkit** (wxWidgets bindings, libui-ng). | Native controls everywhere, as NVDA does with wxPython. But a large C++ dependency on every build, young Rust bindings, and less control over exact names and styles. Worth reconsidering for macOS and Linux if writing those renderers proves slow: the description makes it a renderer swap. |
| E | **A web view** (WebView2, WKWebView, WebKitGTK). | Okur would read its dialogs as web pages (virtual cursor, forms mode), start a browser engine per dialog (hundreds of milliseconds, a large runtime), and sound unlike JAWS's dialogs. |
| F | **Scripted renderers** (PowerShell with WinForms, AppleScript, zenity). | The right instinct (shared logic, native drawing), the wrong renderer: a PowerShell process takes most of a second to start, depends on .NET, has no macOS counterpart, and gives little control. A small Rust renderer gives the same controls at once. |

## 3. The decision

### Three kinds of surface

| Surface | Examples | Who owns the content | How Okur reads it |
|---|---|---|---|
| **Dialog** | element lists, JAWS Find, PlaceMarkers, custom label, Quick Settings, Settings Center, skim reading, Flexible Web | okur-core controllers | through the accessibility API, like any application's dialog |
| **Document window** | Virtual Viewer, Results Viewer (hot key help, skim summaries, PlaceMarker summaries) | okur-core's buffer | with Okur's virtual cursor, from its own buffer; the window shows the text and the cursor, never taking the focus |
| **Overlay** | reading highlight; later the mouse echo and Smart Glance marks | okur-core says what, the platform draws | not read: it is an output, like speech and braille |

### Where things live

- **`okur-ui-model`** (new, platform-neutral, small): the dialog description and the protocol.
  - A description is a tree of standard controls: list (columns, items, selection), radio
    group, check box, combo box (editable, with history), edit (read-only, multi-line), tree
    (with per-item values), button (default, enabled), group, label, tabs, wizard page.
  - Every control has an id, its exact label with its mnemonic, and its enabled state.
  - Layout is given as hints (rows, a column of buttons beside a list), never as pixels; each
    renderer lays out by its platform's conventions.
  - Messages: show, update (new items after a filter, a button enabled), close; and events
    back: activated, selection changed, value changed, text changed, closed with a result.
- **okur-core `ui`**: one controller per dialog. A controller builds the description from the
  engine's state (the buffer, the settings, the stores), handles events, and ends with a
  result the engine applies: move the virtual cursor, activate an element, enter forms mode,
  open the next dialog, or nothing.
  - `UiHost`, the trait the engine talks to: show, update, close, and a channel of events.
  - `MockUiHost` for tests: it keeps the descriptions and projects them to the element tree a
    real window would expose (§4).
- **`okur-ui`** (new binary): the renderer, one module per platform behind `cfg`.
  - **Windows**: a dialog template built in memory from the description, with common controls
    6 (`SysListView32`, buttons, combo box, tree view, edit), per-monitor DPI aware. The
    Windows dialog manager then gives Tab, mnemonics, the default button and Escape, and UI
    Automation's own Win32 proxies expose the dialog as they expose JAWS's: what Okur hears
    is what JAWS users hear from any Win32 dialog.
  - **macOS**: AppKit through `objc2` (`NSPanel`, `NSTableView`, radio and check box buttons,
    `NSComboBox`, `NSOutlineView`, `NSSearchField`).
  - **Linux**: GTK 4, best effort.
  - Okur starts `okur-ui` at launch and restarts it if it exits. It talks over a named pipe
    (Windows) or a Unix socket (macOS, Linux). Opening a dialog is then a message and a
    window, with no process start.
- **okur-platform**: the overlay output and the routing primitives.
  - `VisualTracking`: show rectangles, hide.
    - Windows: one layered, click-through, never-activated, topmost window over the virtual
      screen, as JAWS's.
    - macOS: a borderless transparent window at a high level that ignores the mouse and joins
      every space.
  - The platform bridge gains the element at a point, an element's bounds, the text range's
    rectangles, moving the pointer and clicking.

### Why the renderer runs in its own process

- **Isolation.** A crash or a hang in window code cannot take speech with it.
- **Integrity.** Okur will run with UIAccess; `okur-ui` runs as a normal process. The
  privileged process keeps its surface small, and our harness (`okur-lab snapshot`,
  `okur-lab window-text`) can read Okur's dialogs, as it cannot read JAWS's.
- **No reading of one's own process.** Okur reads its dialogs from outside, like any other
  application's. That avoids the special cases of a process reading itself: WinEvent hooks
  that skip their own process, and macOS's accessibility calls answered on the main thread.
- **The same description serves extensions.** An extension asks for a dialog by describing
  it, and gets the same native, accessible window.

The cost is a message over a local pipe (well under a millisecond) and a second binary. The
description makes the choice reversible: the same renderer could run on a thread of Okur.

### Overlay in the Okur process

The highlight is output only: no input, no focus, a few rectangles per keystroke. It changes
with every reading key, so it belongs with the other outputs, without a hop. With UIAccess,
its window is above other topmost windows, as JAWS's is.

## 4. How it fits the engine

- **Key routing** ([forms and interaction modes](forms-and-interaction-modes.md), "Who takes a
  key"). An Okur dialog in front is a native window outside a document, so step 6 (the PC
  cursor) applies: keys go to the dialog, and Okur reads it with its native focus reading. A
  document window never comes to the front: while it shows, step 2 applies (the Virtual
  Viewer takes the keys). Layers (Insert+Space)
  become a step of their own.
- **The page behind.** Opening a dialog records the page it came from (window, buffer,
  virtual cursor) and keeps its buffer while the dialog is in front. On close, the page comes
  back to the front and the result is applied; Escape changes nothing.
- **Speech.** Okur never speaks a dialog itself: it opens one and reads what the platform
  says is focused. The speech-only list formatters retire.
- **Activation.**
  - Windows: Okur allows `okur-ui` to take the foreground (`AllowSetForegroundWindow`) before
    asking it to show a dialog. That needs Okur to hold foreground rights itself, which
    UIAccess gives. Until then, the thread-input method that `okur-lab front` uses.
  - macOS: a non-activating panel that becomes the key window, so the browser stays the
    active application.
- **Settings.** A typed registry in okur-core: each setting's id, group, label, values,
  default, description and scopes (all applications, one application, a site, a document)
  and lifetime (saved, until Okur exits, until the focus moves). Values are layered;
  search and "last changed" come from it. Quick Settings and Settings Center are
  descriptions generated from the registry. JAWS's `.jcf` remains an import, for the
  reference machine.
- **Stores.** PlaceMarkers per URL, labels per domain, find history per site, Flexible Web
  rules: files of Okur's own format under its settings folder.

## 5. Testing

1. **Controllers**, in `cargo test`: content, filters, sorting, first selection, default
   buttons, results, empty messages.
2. **Projection.** `MockUiHost` turns a description into the platform-neutral elements a
   window would expose (dialog name, list name, item, position, states). The engine's own
   native focus reading speaks them, and the speech is compared with JAWS's recorded speech
   for its dialogs (old-pc scenarios). So dialog parity is measured hermetically, through the
   same reading code as any application.
3. **Conformance**, live and `#[ignore]`d: render a description with the real renderer,
   snapshot it with UI Automation (`okur-lab snapshot`), and compare with the mock projection.
   The two must agree, or the hermetic results mean nothing.
4. **Overlay.** The rectangles are computed and tested in core; the drawing is checked live
   with screenshots.
5. **Reference.** JAWS's dialogs can't be read through UI Automation from our harness. Their
   reference is JAWS's speech history, `ReadBoxInTabOrder`, and screenshots.

## 6. Risks, and the spikes that settle them

- **S1, Windows.** `okur-ui` shows an in-memory Links List over Chrome, opened by Insert+F7
  through Okur's hook. Does it come to the front without UIAccess? Does Okur read it as JAWS
  reads its own, and how long from key to speech?
- **S2, macOS** (needs a Mac). A non-activating panel from a background helper takes the
  keys over Safari, and Okur's accessibility client reads it.
- **S3, overlay.** The cost of one update (target under 2 ms), several monitors, mixed DPI.
- **Long lists.** Pages with thousands of links may need a virtual list view; check what UI
  Automation exposes for one before relying on it.
