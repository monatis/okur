# Handoff: Desktop Navigation Repetition Fix & File Explorer Parity

- **Date**: 2026-10-13
- **Milestone**: 13 (Native Desktop & Windows Application Navigation)
- **Status**: Complete & Verified (Differential Tests Passing, Quality Gate 100%)

---

## 1. Summary of Changes

This session resolved two user-reported UX discrepancies between our screen reader and Freedom Scientific JAWS 2025 on native Windows 11:

### 1. Desktop Entry Announcement Repetition (Issue 1)
- **Problem**: When moving between desktop icons with arrow keys or pressing initial letters, the engine repeatedly announced the full entry speech:
  ```text
  List box
  FolderView list view
  <item name>
  <index> of <total items>
  To move to items use the arrow keys.
  To edit the selected item press f2.
  ```
- **Root Cause**: In live runtime mode without a static corpus fixture, `engine.desktop_state` started as `None`. In `handle_focus_event`, `is_first_entry` checked `self.desktop_state.as_ref().is_none_or(|ds| !ds.entered)`. Because `self.desktop_state` remained `None`, `is_none_or` evaluated to `true` on every single focus event.
- **Solution**:
  - In `crates/sr-core/src/engine.rs`: Ensured `self.desktop_state` is lazily initialized on the first desktop event (`entered = true`).
  - `is_first_entry` is evaluated strictly as `prev_mode != AppMode::NativeDesktop || !ds.entered`.
  - On container entry, `format_desktop_entry` is spoken once.
  - On subsequent arrow and initial-letter movements within the desktop grid, only `format_desktop_item` (`<name>\n<index> of <total>`) is spoken.
  - When focus leaves Desktop (to Taskbar, File Explorer, or another window), `ds.entered` is reset to `false`.

### 2. File Explorer Recognition and Real Index/Total Count (Issue 2)
- **Problem**: Pressing Enter on Desktop to open File Explorer (e.g. "Dropbox") caused items inside File Explorer to be misidentified as desktop `FolderView` items and announced with hardcoded `"1 of 1"`.
- **Root Cause**:
  1. `engine.rs` previously checked `focus.role == Role::ListItem && self.app_mode == AppMode::NativeDesktop`, causing any list item focused after Desktop to be treated as Desktop.
  2. `sr-platform/src/events/windows.rs` only queried Win32 `LVM_GETITEMCOUNT` for `SysListView32`. In modern Windows 11 File Explorer, the list is a DirectUI/XAML `ItemsView` (`UIItemsView`) where Win32 `SysListView32` messages fail.
- **Solution**:
  - Added `AppMode::NativeExplorer` and `NativeExplorerState`.
  - Added `UIA_PositionInSetPropertyId` (`30152`), `UIA_SizeOfSetPropertyId` (`30153`), and `UIA_SelectionItemIsSelectedPropertyId` (`30079`) querying in `FocusChangedHandler::process_element`.
  - Top-level window handle resolution via `GetAncestor(hwnd, GA_ROOT)` or `GetForegroundWindow()`, extracting window title (e.g. `"Dropbox - File Explorer"`) and folder name (e.g. `"Dropbox"`).
  - Implemented `SpeechFormatter::format_explorer_entry` and `format_explorer_item`, matching JAWS 2025 verbatim:
    - On folder entry:
      ```text
      Dropbox - File Explorer
      Dropbox
      Items view multi select box
      Not selected
      .cache
      1 of 143
      ```
    - On in-list navigation (e.g. pressing 'a' or Down Arrow):
      ```text
      ailabs
      3 of 143
      ```

---

## 2. Verification & Quality Gates

- **Unit & Integration Tests**:
  - `test_engine_desktop_entry_vs_subsequent_item_navigation` verified.
  - `test_engine_file_explorer_entry_and_item_navigation` verified.
  - All 54 tests pass across the entire workspace (`sr-core`, `sr-platform`, `sr-corpus`).
- **Clippy**:
  - `cargo clippy --all-targets --all-features -- -D warnings` -> **0 errors, 0 warnings**.
- **Formatting**:
  - `cargo fmt --check` -> **Pass**.

---

## 3. Files Modified
- [`crates/sr-core/src/event.rs`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-core/src/event.rs): Added `window_title` and `folder_name` metadata fields and builder methods.
- [`crates/sr-core/src/speech.rs`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-core/src/speech.rs): Added `format_explorer_entry` and `format_explorer_item`.
- [`crates/sr-core/src/engine.rs`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-core/src/engine.rs): Added `AppMode::NativeExplorer`, `NativeExplorerState`, fixed desktop entry tracking, and added comprehensive unit tests.
- [`crates/sr-platform/src/events/windows.rs`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-platform/src/events/windows.rs): Added UIA `PositionInSet`, `SizeOfSet`, `IsSelected`, and top-level window resolution.
