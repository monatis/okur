# Implementation Plan: Live Window Management & Browser Virtual Buffer Parity

- **Date**: 2026-10-13
- **Author**: Autonomous Pair Programming Agent
- **Target**: Genuine Freedom Scientific JAWS 2025 Parity across All Windows (Desktop, Shell, Native Apps, and Web Browsers)
- **Issue**: Screen reader daemon launched in terminal leaked terminal titlebar controls (minimize, maximize, close, scrollbar increase/decrease) into a static `VirtualBuffer`. When switching to Chrome with `Alt+Tab`, the screen reader remained locked in `VirtualBuffer`, trapping arrow keys and reciting terminal controls instead of navigating the web page.

---

## 1. Root Cause Analysis

1. **Bogus Fallback in `PlatformBridge::get_active_document`**:
   - `PlatformBridge::get_active_document(None)` fell back to `app.windows()?.into_iter().next()` when no web area was found.
   - When launched from Windows Terminal (`CASCADIA_HOSTING_WINDOW_CLASS`), this turned the terminal window into an `AomNode` tree containing the titlebar (Minimize, Maximize, Close) and scrollbars (Vertical Large Increase, Vertical Large Decrease, Vertical Small Decrease).
   - This dummy tree was loaded into `VirtualBuffer`, and `app_mode` was initialized to `AppMode::VirtualBuffer`.

2. **Missing Native Window Mode (`AppMode::NativeControl`)**:
   - The engine only handled `NativeDesktop`, `NativeExplorer`, and `NativeShell`.
   - All other native windows (Terminal, Notepad, Calculator, Chrome's native frame/address bar, Settings, Slack, etc.) were left in `VirtualBuffer` mode!
   - In `handle_key_event`, when `app_mode == AppMode::VirtualBuffer`, navigation keys (`DownArrow`, `UpArrow`, etc.) were consumed and dispatched to `execute_command` on the virtual buffer.

3. **Missing Dynamic Web Document Lifecycle**:
   - When switching to Google Chrome (`Alt+Tab`), the screen reader never detected that focus moved into a web document (`UIA_DocumentControlTypeId`).
   - It never loaded Chrome's live accessible document into `VirtualBuffer`.
   - The engine remained stuck with the terminal window buffer.

---

## 2. Architectural Solution

1. **Strict Document Boundary in `PlatformBridge` (`crates/sr-platform/src/bridge.rs`)**:
   - Eliminate the fallback to arbitrary window controls.
   - Add `PlatformError::NoDocumentFound`.
   - Only return `Ok(AomNode)` if a genuine web document / web area is present.

2. **Universal Window State Machine (`crates/sr-core/src/engine.rs`)**:
   - Support `AppMode::NativeControl` for all native Windows applications.
   - In `AppMode::NativeControl` (and all native modes), all navigation and typing keys pass through to the OS / application (`KeyAction::PassThrough`). Keys are never trapped.
   - In `handle_focus_event`, track `last_window_title`. When window focus changes, announce the window title followed by the focused control.

3. **Dynamic Web Document Extraction & Virtual Buffer Lifecycle (`crates/sr-platform/src/events/windows.rs` & `sr-core`)**:
   - Add `AccessibilityEvent::DocumentLoaded { title: String, root: AomNode }`.
   - In `FocusChangedHandler`:
     - Detect when focus is on or inside a web document (`UIA_DocumentControlTypeId`, `Chrome_RenderWidgetHostHWND`, `MozillaContentWindowClass`).
     - When entering a web document, extract the document tree via UIA into an `AomNode` and send `DocumentLoaded`.
     - In `ScreenReaderEngine`, replace the virtual buffer, set `app_mode = AppMode::VirtualBuffer`, and announce `<Title> \u{001d} Virtual PC Cursor \u{001e}`.
     - When focus leaves the document (to address bar or another app), transition to `AppMode::NativeControl` (or desktop/explorer/shell).

4. **Clean Live Startup (`crates/sr-cli/src/main.rs`)**:
   - When launched without `--fixture`, do not allocate a dummy virtual buffer from the terminal.
   - Initialize in `AppMode::NativeControl` (or current foreground context).

---

## 3. Verification Protocol

1. Unit tests for `AppMode::NativeControl` key pass-through and focus announcements.
2. Unit tests for `AccessibilityEvent::DocumentLoaded` buffer switching.
3. Live test: run `sr-cli run --engine sapi5` in terminal, switch to Chrome, verify web page lines are read and terminal controls are never spoken.
4. Verify `cargo fmt`, `cargo clippy`, and `cargo test`.
