# Implementation Plan: Milestone 13 – Windows Desktop & Shell Surface Navigation Parity

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 13  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-table-headers-coordinates-and-mathml.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-table-headers-coordinates-and-mathml.md)  
**Deferred Backlog**: [`docs/backlog/individual-windows-applications-navigation.md`](file:///c:/Users/Gigabyte/screen-reader/docs/backlog/individual-windows-applications-navigation.md) (File Explorer, Notepad, dialogs deferred)  
**Parity Target**: Freedom Scientific JAWS 2025 Differential Behavior  

---

## 1. Overview & Objective

When the screen reader is active and the user is on the Windows desktop or navigating Windows shell surfaces (e.g., via `Win+M`, `Win+D`, `Tab`, or arrow keys):
1. **Web Quick-Nav Suppression**:
   - Single alphanumeric keys (`T`, `H`, `K`, `C`, `E`, `1`–`6`, etc.) must **never** trigger web virtual buffer search or emit `"There are no tables on this page."`
2. **Desktop First-Letter Item Navigation (Initial Matching)**:
   - Typing character keys jumps between desktop items starting with that character (e.g. pressing `T` moves between items starting with T, such as `"This PC"` and `"Task Manager"`).
3. **Desktop Arrow Navigation**:
   - Up, Down, Left, and Right arrow keys move across desktop grid items, announcing name, index, and total count:
     `"{Name} {Index} of {Total}"` (matching JAWS 2025).
4. **Shell Surface Cycling (Tab / Shift+Tab)**:
   - Navigating via `Tab` and `Shift+Tab` cycles between core Windows shell surfaces:
     - Desktop Folder View (`SysListView32`)
     - Start Button (`Button`)
     - Search Button / Box (`Button` / `Edit`)
     - Taskbar / Task View (`ToolBar` / `List`)
     - System Tray / Notification Area chevron & clock
   - Each transition announces the element name, role, and states.

---

## 2. Autonomous Investigation Plan (`old-pc` Ground Truth)

Before authoring fixtures and scenarios:
1. Drive live JAWS 2025 on `old-pc` across the desktop surface:
   - Send `Win+M` and capture what JAWS announces upon landing on the desktop.
   - Record exact JAWS speech when pressing initial letter keys (`T`, `R`, etc.).
   - Record exact JAWS speech when pressing arrow keys across desktop items.
   - Record exact JAWS speech when pressing `H` (a key with no matching item) to observe silent pass-through vs audio cue.
   - Record exact JAWS speech when pressing `Tab` through Start, Search, Taskbar, System Tray, and `Shift+Tab` back to Desktop.
2. Dump the actual `xa11y` / UIA accessibility tree of the Windows desktop and shell bar from `old-pc` to extract precise roles, control types, class names, and attributes.

---

## 3. Technical Architecture & Component Changes

### 3.1 Platform & Event Model (`crates/sr-platform`)
- **Control Classification**:
  - Distinguish desktop list view (`SysListView32` in `Progman`/`WorkerW`) and shell bar (`Shell_TrayWnd`) from web documents.
- **Enhanced FocusEvent**:
  - Include `item_index: Option<usize>`, `item_count: Option<usize>`, `container_role: Option<Role>`, `container_name: Option<String>`.

### 3.2 Core Engine Subsystem (`crates/sr-core`)
- **`AppMode` State Machine**:
  - `VirtualBuffer`: Web documents (quick-nav active).
  - `NativeDesktop`: Desktop list view (quick-nav suppressed; first-letter item matching active).
  - `NativeShell`: Taskbar, Start button, System Tray.
- **First-Letter Item Navigation**:
  - Maintain active container item list and index.
  - Alphanumeric key press searches for next item whose name starts with that character (case-insensitive wrap-around).
- **Speech Formatting**:
  - `format_desktop_item(name, index, total)` matching JAWS transcript: `"{Name} {index} of {total}"`.

---

## 4. Differential Corpus Fixtures & Scenarios (61–65)

1. **`xa11y` Fixtures**:
   - `corpus/fixtures/windows-desktop.aom.json`: Serialized `xa11y` tree representing desktop items and shell bar components captured from `old-pc`.
2. **Scenarios**:
   - Scenario 61: Desktop initial matching (`T` jumping to item, suppressing table quick-nav).
   - Scenario 62: Desktop initial wrap-around (`T` jumping to next matching item).
   - Scenario 63: Desktop arrow navigation (Up/Down/Left/Right speaking item name and index).
   - Scenario 64: Non-matching initial key suppression (`H` on desktop does not trigger web heading search).
   - Scenario 65: Shell surface Tab/Shift+Tab cycling (Desktop $\rightarrow$ Start Button $\rightarrow$ Search $\rightarrow$ Taskbar $\rightarrow$ System Tray $\rightarrow$ Desktop).
