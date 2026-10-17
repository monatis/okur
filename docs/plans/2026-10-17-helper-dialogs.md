# Plan: Okur's Own Dialogs, Viewer and Reading Highlight (2026-10-17)

The user changed direction on 2026-10-17: after the web-app and desktop work, the next layer
of muscle memory is JAWS's helper features. The lists, JAWS Find, PlaceMarkers, Quick
Settings, the Results Viewer, the red reading highlight and the cursor routing keys.

- What JAWS does: [`jaws-dialogs-and-visual-features.md`](../references/jaws-dialogs-and-visual-features.md).
- How Okur builds it, and why: [`helper-ui-architecture.md`](../references/helper-ui-architecture.md).
  The data and logic of every dialog live in okur-core; a dialog is described as standard
  controls with exact names; `okur-ui`, a separate process, renders the description natively
  (Win32 now, AppKit and GTK later); Okur reads its dialogs through the accessibility API like
  any other; the highlight is an output drawn by Okur itself.

The open items of [`2026-10-16-web-apps-round-2.md`](2026-10-16-web-apps-round-2.md) and its
old-pc work stay valid; this plan comes first.

## Rules for every step

- Record JAWS first: each step's dialogs get old-pc scenarios (opening speech, Tab through
  every control, Enter, Escape, the empty case) before Okur's speech is compared with them.
  Dev-box captures in the reference page are research.
- No speech is written for a dialog. Okur opens a window and reads it. If Okur reads its own
  dialog unlike JAWS reads JAWS's, fix Okur's native dialog reading, which fixes every
  application's dialogs too.
- Keep `cargo test` hermetic: controllers and the mock projection are tested there; the real
  renderer has `#[ignore = "live: ..."]` conformance tests.

## Steps

Order the user chose on 2026-10-17: the Virtual Viewer on screen first (step 7, done), then
the Links List and a simple settings dialog for what are command-line flags today (step 9,
reduced to those settings; done the same day: "Okur Settings", Insert+6 and Insert+V, see
[the handoff](../handoff/2026-10-17-okur-ui-viewer-and-settings.md)). "They should be configurable on the UI just like JAWS." Broader
settings follow as Okur's configurability and parity grow.

### 1. Spike S1: one real Links List on Windows

**Done 2026-10-17**: Insert+F7 opens it from Chrome; Okur and JAWS read it alike; Move To
Link, Activate Link and Escape return to the page as JAWS's do. The dialog came to the front
without UIAccess, by the thread-input method. What follows is the original outline.

- Crate `okur-ui-model`: the description (list, radio group, check box, combo box, edit, tree,
  button, group, label) and the protocol (show, update, close; events). Serde, versioned.
- Binary `okur-ui`: a Win32 renderer that builds a dialog template in memory; started by
  Okur, connected by a named pipe.
- okur-core: the `UiHost` trait, `MockUiHost` with its projection, and a Links List
  controller (In Tab Order and Alphabetically; All, Visited, Unvisited; Move To Link,
  Activate Link (default), Cancel; the read-only address field), opened by Insert+F7.
- Measure and record in the handoff:
  - whether the dialog comes to the front without UIAccess, and how;
  - Okur's speech on opening against JAWS's ("Links List dialog | links List view |
    Reputation for Quality Service | 1 of 13 | To move to items use the Arrow keys.");
  - key-to-speech time (`okur latency`).
- **Done when:** on the dev box (with permission) and on old-pc, Insert+F7 in Chrome opens a
  real Links List that Okur reads like JAWS's, and Enter, Alt+M and Escape return to the page
  with the right result.

### 2. Engine integration

- A helper session in the engine: the page it came from (window, buffer, virtual cursor),
  kept while the dialog is in front; the result applied on close (move, activate, forms mode
  for a field); Escape changes nothing.
- The page's buffer must survive the focus leaving the browser for Okur's dialog.
- Retire `ActiveElementList` and the speech-only list formatters.
- Update "Who takes a key" in `forms-and-interaction-modes.md`.
- Conformance test: the rendered dialog's UI Automation snapshot equals the mock projection.

### 3. The list family

Heading List (sort, Display All and Level 1 to 6, Alt+1 to Alt+6 and Alt+L), Select a Form
Field (Enter turns forms mode on), Frames List, Select a Graphic (nearest graphic selected),
Select a List, regions, the Insert+Ctrl+letter lists, Insert+F3 Virtual HTML Features. JAWS's
messages when a list is empty ("no form fields were found", "No regions were found on the
page").

### 4. JAWS Find

The dialog (Find what with a history of 20, Context Specific History, Maintain History,
Direction, Ignore Case; Find, Delete, Clear Find History, Cancel), the per-site history store,
Insert+Ctrl+F outside documents; F3 and Shift+F3 exist.

### 5. The reading highlight

- `VisualTracking` in okur-platform, Windows first: one layered, click-through, never
  activated, topmost window.
- Rectangles of the unit just read: the line, the word (also after a character move), the
  element after a quick key, from the text range's bounding rectangles (IA2 or UIA); the page
  scrolled to it.
- A setting to turn it off; check its cost per key with `okur latency` (target under 2 ms).
- Unlike JAWS, hide the page's highlight while an Okur dialog is in front: JAWS leaves a stale
  rectangle over its own dialogs.

### 6. Routing and the mouse

Insert+NumPad Plus (virtual cursor to the pointer, "Route PC to JAWS"), Insert+NumPad Minus
(pointer to the virtual cursor, "Route JAWS to PC"), NumPad Slash and NumPad Star (clicks),
their locks, and later the JAWS cursor (NumPad Minus) reading at the pointer. The platform
bridge gains the element at a point, bounds, pointer moves and clicks.

### 7. The Virtual Viewer on screen

**Done 2026-10-17, first** (the user chose it as the first target): `okur-ui` shows the
viewer as JAWS's does, never taking the focus, with a red frame on what was read; see
[virtual-viewer.md](../references/virtual-viewer.md#on-screen). Links in results (skim
summaries, later) are followed with Enter.

### 8. PlaceMarkers and custom labels

The PlaceMarker List (Add proposes the text at the cursor; the default button switches from
Add to MoveTo; Change, Remove, Remove All; Display for URL, Domain, All), K and Shift+K with
wrapping, Ctrl+Windows+K, the per-URL store. Custom labels (Insert+Ctrl+Tab) per domain, used
everywhere names are, the lists included.

### 9. Settings

The typed settings registry with scopes and lifetimes; Quick Settings (Insert+V) and Settings
Center (Insert+6) as descriptions generated from it: search (Ctrl+E), tree, configuration
pane (F6), description, Apply, OK, Cancel, the Application combo box with Ctrl+Shift+D,
Last Changed Settings, the lifetime context menu.

### 10. Layers

Insert+Space as a key-routing step: the pending layer, the table layer that stays until a key
outside it, ? for a layer's keys.

### 11. Later

Skim reading and its summary, Flexible Web, Select a Scheme, Run JAWS Manager.

### 12. macOS and Linux

Spike S2 on a Mac (a non-activating panel taking keys over Safari, read by Okur's
accessibility client), then the AppKit renderer and the overlay. GTK 4 best effort.

## old-pc scenarios to record

For the first steps, in `corpus/pending/` before they run:
- Insert+F7, Insert+F6, Insert+F5 on a fixture with links, headings and fields: opening
  speech, Insert+B, Tab through every control, Down twice, Enter; and again with Escape.
- Each list on a fixture without its elements: the message JAWS says.
- Insert+F3, then Enter on Headings List.
- Ctrl+F, typing a word, Enter; F3 to wrapping.
- Ctrl+Shift+K on a page without marks; Add, Enter; K from the top.
