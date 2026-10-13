# Native Desktop and Windows Shell Surface Navigation Reference

This reference document outlines the architectural patterns, speech formatting, keyboard interactions, and operational modes implemented in the screen reader to achieve exact behavioral parity with **Freedom Scientific JAWS 2025** on the Windows desktop and shell bar.

---

## 1. Context & Design Rationale

In web browser documents or virtual buffers, alphanumeric keystrokes act as single-key quick navigation commands (e.g., `H` for next heading, `T` for next table, `K` for next link). If a requested element is not present on the page, the screen reader announces an explicit negative boundary notice (e.g., `"There are no tables on this page."`).

On the native Windows desktop (represented by `Progman` / `WorkerW` hosting the `FolderView` `SysListView32` list), the virtual buffer is deactivated. Instead:
1. **Initial Alphanumeric Matching**: Pressing a letter jumps to the next item starting with that letter. Consecutive presses of the same letter cycle through matching items in top-to-bottom, left-to-right grid order with automatic wrap-around.
2. **Web Quick-Nav Suppression**: Unassigned letters or letters that have no match on the desktop (e.g., `H` when no desktop shortcut starts with `H`) do **not** emit web quick-nav error announcements. They pass through silently to the operating system.
3. **Arrow-Based Navigation**: Arrow keys (`Up`, `Down`, `Left`, `Right`) navigate between grid items, announcing only the item name and 1-based index (e.g., `"Task Manager\n3 of 6"`).
4. **Shell Surface Cycling**: Pressing `Tab` cycles focus outside the desktop list into the Windows shell tray:
   - **Start Button**: `"Taskbar \n   Start Button   \nTo open press Enter."`
   - **Search Button**: `"Type here to search    Button   \nTo activate press spacebar."`
   - **Task View**: `"Task View    Button   \nTo activate press spacebar."`
   - **Running Taskbar Buttons**: `"  Task bar    Microsoft Edge    Button   \nTo activate press spacebar."`
   - Pressing `Tab` on the final taskbar item returns to the desktop folder view.
   - Pressing `Shift+Tab` (`+{TAB}`) reverses the cycle.

---

## 2. Modes of Operation (`AppMode`)

The core engine supports the following operating modes (`crates/sr-core/src/engine.rs`):
- `AppMode::VirtualBuffer`: Active in web documents and complex virtualized areas. Single-key quick navigation is enabled.
- `AppMode::NativeDesktop`: Active when focus is in `FolderView` (`SysListView32` within `Progman`/`WorkerW`). Alphanumeric keys trigger initial letter jumps; arrows move grid items; web quick-nav is suppressed.
- `AppMode::NativeShell`: Active when navigating taskbar and shell bar controls. Tab and Shift+Tab traverse buttons; Space/Enter activates them.
- `AppMode::NativeControl`: Direct OS control interaction.

---

## 3. Canonical Speech Tokens

| Event / Keystroke | Target UI Component | Canonical JAWS Speech Announcement |
|---|---|---|
| Focus arrives at Desktop (`Windows+M`) | `FolderView` (Item 1: "This PC") | `  List box    \nFolderView    List view   \nThis PC\n1 of 6\nTo move to items use the Arrow keys.\nTo edit the selected item press f2.` |
| Alphanumeric key (`T`) | Jump to "Task Manager" | `Task Manager\n3 of 6` |
| Consecutive key (`T`) | Jump to "Terminal" | `Terminal\n5 of 6` |
| Consecutive key (`T`) | Wrap-around to "This PC" | `This PC\n1 of 6` |
| Non-matching key (`Q` or `H`) | Desktop list item | *(Silent pass-through; no speech emitted)* |
| Arrow key (`{DOWN}`) | Next list item ("Recycle Bin") | `Recycle Bin\n2 of 6` |
| Arrow key (`{UP}`) | Prior list item | `This PC\n1 of 6` |
| `Tab` from Desktop | Start Button | `Taskbar \n   Start Button   \nTo open press Enter.` |
| `Tab` from Start | Search Button | `Type here to search    Button   \nTo activate press spacebar.` |
| `Tab` from Search | Task View | `Task View    Button   \nTo activate press spacebar.` |
| `Tab` from Task View | Edge on Taskbar | `  Task bar    Microsoft Edge    Button   \nTo activate press spacebar.` |
| `Tab` from Taskbar | Desktop folder view | `  List box    \nFolderView    List view   \nThis PC\n1 of 6\nTo move to items use the Arrow keys.\nTo edit the selected item press f2.` |
| `Shift+Tab` from Desktop | Edge on Taskbar | `  Task bar    Microsoft Edge    Button   \nTo activate press spacebar.` |

---

## 4. Test Fixtures & Differential Verification

- Fixture: `corpus/fixtures/windows-desktop.aom.json` & `corpus/fixtures/windows-desktop.html`.
- Scenarios:
  - `61-desktop-initial-match`: Initial letter jump from "This PC" to "Task Manager" and quick-nav suppression.
  - `62-desktop-initial-wrap`: Multi-jump initial matching with wrap-around across desktop items.
  - `63-desktop-arrow-navigation`: Four-way arrow grid navigation.
  - `64-desktop-nonmatching-key-suppression`: Suppression of `h` and `q` without spurious web error speech.
  - `65-shell-tab-cycling`: Full bidirectional Tab and Shift+Tab traversal between Desktop and Shell bar.
