# Handoff: Live Windows 11 Desktop & Shell Surface Navigation Parity

- **Date**: 2026-10-13
- **Author**: Autonomous Pair Programming Agent
- **Status**: Complete & Verified on Live Windows 11 Machine
- **Issue Addressed**: Live Screen Reader daemon divergence on native Windows 11 desktop (`Win+M` / `Win+D`, pressing `T` announced "no tables", `Tab` cycled terminal window titlebar controls instead of Start, Search, Task View, and Taskbar buttons).

---

## 1. Root-Cause Analysis & Investigation Findings

When the live screen reader daemon (`cargo run --bin sr-cli -- run --engine sapi5`) was launched without an explicit `--fixture` argument, the following chain of issues occurred:

1. **Static Fallback Virtual Buffer Leak**:
   - `bridge.get_active_document(None)` was falling back to the first window (the Windows Terminal window from which the daemon was launched).
   - This converted the terminal window controls (Minimize button, Maximize button, Close button, scrollbar large/small increase/decrease buttons) into an active `VirtualBuffer`.
   - The daemon remained locked in `AppMode::VirtualBuffer`.

2. **Keystroke Trapping & Virtual Quick-Nav Leaks**:
   - Because `app_mode` was `VirtualBuffer`, pressing `T` intercepted the key as `QuickNav::Table`. Since no HTML table existed in the terminal buffer, it announced: `"There are no tables on this page."` and consumed the keystroke (`KeyAction::Consume`). Windows OS never received `T` to navigate icons.
   - Similarly, pressing `Tab` executed `NextFormField` across the terminal buffer, reciting: `"minimize button, maximize button, vertical large increase, vertical large decrease, vertical small decrease, close button"` and consuming `Tab`. Windows OS never received `Tab` to cycle shell surfaces.

3. **WindowStation Desktop Isolation & Broken UIA Event Listener**:
   - The UIA `FocusChangedHandler` thread in `sr-platform/src/events/windows.rs` was not attached to the interactive user desktop (`WinSta0\default`), lacked an STA message pump (`COINIT_APARTMENTTHREADED` with `PeekMessageW` / `DispatchMessageW`), and did not query element parents or HWNDs.
   - It failed to populate `container_name`, `window_class`, `item_index`, or `item_count`, so `handle_focus_event` in `sr-core` never transitioned into `NativeDesktop` or `NativeShell`.

---

## 2. Engineering Solution & Implemented Enhancements

1. **Live Key Pass-Through in `crates/sr-core/src/engine.rs`**:
   - In `handle_key_event`, whenever `self.app_mode != AppMode::VirtualBuffer`:
     - All navigation keys (`Tab`, `Shift+Tab`, `DownArrow`, `UpArrow`, `LeftArrow`, `RightArrow`, `Home`, `End`, `PageUp`, `PageDown`) and alphanumeric typing keys (`T`, `H`, initial letters) immediately return `KeyAction::PassThrough` to the OS.
     - Windows OS receives the keystroke natively: initial letter search jumps to desktop icons, arrow keys navigate the icon grid, and `Tab` cycles the Windows shell bar (Desktop $\rightarrow$ Start $\rightarrow$ Search $\rightarrow$ Task View $\rightarrow$ Taskbar apps $\rightarrow$ System Tray).
   - In `execute_command`: explicitly returned `None` for unhandled commands when in `NativeDesktop` or `NativeShell` to strictly prevent command fall-through to virtual buffer reading commands.
   - In `NativeDesktopState`: added `entered: bool` tracking to announce `FolderView list view <Name> <index> of <total>` on container entry and `<Name> <index> of <total>` on subsequent in-grid item navigation.

2. **Live Desktop & Shell UIA Event Listener in `crates/sr-platform/src/events/windows.rs`**:
   - Attached listener thread to interactive user desktop via `OpenDesktopW(w!("default"), ...)` and `SetThreadDesktop`.
   - Initialized COM with `COINIT_APARTMENTTHREADED` and integrated an active Win32 message pump (`PeekMessageW`, `TranslateMessage`, `DispatchMessageW`).
   - In `FocusChangedHandler`:
     - Queries `name`, `class_name`, `control_type`, `help_text`.
     - Inspects parent and grandparent hierarchy using `ControlViewWalker`.
     - Detects Desktop (`SysListView32`, `Progman`, `WorkerW`): queries Win32 `SendMessageW(parent_hwnd, LVM_GETITEMCOUNT)` and `LVM_GETNEXTITEM (LVNI_FOCUSED)` to extract 1-based `item_index` and `item_count`.
     - Detects Windows 11 Taskbar (`Taskbar`, `Shell_TrayWnd`, `Taskbar.TaskbarFrameAutomationPeer`, `InputSite.WindowClass`).
     - Immediately inspects `GetFocusedElement` on daemon startup to seed initial focus without waiting for the first keypress.

3. **CLI Live Startup Refactoring in `crates/sr-cli/src/main.rs`**:
   - When launched without `--fixture` and no foreground web document is present, initializes `app_mode = AppMode::NativeDesktop` without allocating a dummy virtual buffer.
   - Emits spoken event announcements directly via SAPI 5 synthesizer and logs clean text to stdout.

---

## 3. Live Verification on Local Windows 11 Machine

Verified live on Windows 11 using OpenDesk automation:
1. **Startup Focus Detection**:
   - On daemon launch, immediately detected desktop focus and spoke:
     ```
     FolderView    List view
     Learn about this picture
     2 of 11
     To move to items use the Arrow keys.
     To edit the selected item press f2.
     ```
2. **Pressing `Tab` Across Windows 11 Shell**:
   - Tab 1: `Taskbar  Task View  Button  one desktop`
   - Tab 2: `Widgets 24°C Çok bulutlu  Button`
   - Tab 3: `Start  Button`
   - Tab 4: `Search  Button`
   - Tab 5: `Search - Plitvice Lakes National Park, Croatia  Button`
   - Returned 0 spurious titlebar button announcements.
3. **Pressing `Win+D` Back to Desktop**:
   - Focus shifted back to Desktop list view:
     ```
     FolderView    List view
     Google Chrome
     3 of 11
     To move to items use the Arrow keys.
     To edit the selected item press f2.
     ```
4. **Pressing `T` on Desktop**:
   - Passed through to OS cleanly; did NOT announce "There are no tables on this page."
5. **Quality Gates Passed**:
   - `cargo fmt --check`: Clean pass.
   - `cargo clippy --all-targets --all-features -- -D warnings`: 0 errors, 0 warnings.
   - `cargo test --all`: 52 tests across all 4 crates passed (including all 65 differential scenarios).
