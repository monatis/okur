# Implementation Plan: Milestone 13 – Native Desktop & Windows OS Application Navigation Parity

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 13  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-table-headers-coordinates-and-mathml.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-table-headers-coordinates-and-mathml.md)  
**Parity Target**: Freedom Scientific JAWS 2025 Differential Behavior  

---

## 1. Overview & Problem Statement

Currently, when the screen reader is active and a user navigates to the Windows desktop (e.g. via `Win+M` or `Win+D`) and presses alphanumeric keys such as `T`, the virtual buffer interceptor treats the key as a quick-navigation shortcut and speaks:
`"There are no tables on this page."`

In Freedom Scientific JAWS (and NVDA):
1. **Desktop / Shell Context Differentiation**:
   - The desktop (`Progman` / `WorkerW` hosting `SHELLDLL_DefView` and `SysListView32`) and Windows shell surfaces are recognized as native UI controls rather than web virtual buffers.
2. **Web Quick-Nav Suppression**:
   - When focus is in a native desktop, File Explorer (`CabinetWClass`), taskbar (`Shell_TrayWnd`), or native dialog, Web Quick Navigation keys (`T`, `H`, `K`, `C`, `E`, `1`-`6`) are suppressed so they do not trigger web document search or error messages.
3. **First-Letter Item Navigation (Alphanumeric Initial Matching)**:
   - Alphanumeric keystrokes are passed directly to the native control (or handled by the screen reader's list tracker) to jump between items starting with that character (e.g., pressing `T` jumps to "This PC" or "Task Manager").
4. **Native Focus & Arrow Tracking**:
   - Navigating with Arrow keys, Tab, or initial letters announces:
     `"{Name} {Role} {Index} of {Total}"` (e.g. `"This PC 1 of 12"`, `"Recycle Bin 2 of 12"`).

---

## 2. Technical Architecture & Component Changes

### 2.1 Platform Subsystem (`crates/sr-platform`)
- **Window Class & Host Detection (`bridge.rs`, `events/`)**:
  - Implement window handle query to classify window hierarchy:
    - Shell Desktop: `Progman`, `WorkerW`, `SHELLDLL_DefView`.
    - File Explorer: `CabinetWClass`.
    - Taskbar & Start Menu: `Shell_TrayWnd`, `Windows.UI.Core.CoreWindow`.
    - Native Dialogs: Win32 `#32770`.
  - Expose `AppWindowContext` to inform the core engine whether the active focus belongs to a web virtual buffer host (Chrome/Edge/Firefox document) or a native desktop/shell application.
- **Native Focus Event Dispatching**:
  - Ensure UIA / Win32 focus change events on native list items transmit element name, role, selection state, and item index (`ItemIndex`, `ItemCount`).

### 2.2 Core Engine Subsystem (`crates/sr-core`)
- **Engine Mode & Virtual Buffer Intercept State (`engine.rs`, `cursor.rs`)**:
  - Introduce `DocumentMode`:
    - `VirtualBuffer`: Active for web pages and PDF documents where quick navigation keys are intercepted.
    - `NativeDesktop`: Active when focused on desktop list views and Windows shell components.
    - `NativeControl`: Active for standard native application dialogs and forms.
  - When in `NativeDesktop` or `NativeControl` mode:
    - Quick-navigation keys (`T`, `H`, `K`, etc.) are **not** captured by `VirtualBuffer::find_next_*()`.
    - Unhandled alphanumeric keys pass through to native input or trigger first-letter list item navigation.
- **First-Letter List Item Navigation**:
  - When a list view has focus, pressing a character key jumps to the next item matching that initial letter, announcing item name, selection, and position.
- **Speech Formatting (`speech.rs`)**:
  - Format native desktop item announcements matching JAWS 2025:
    `format_desktop_item(name, index, total)` -> `"{Name} {index} of {total}"`.

---

## 3. Autonomous Verification Plan (`old-pc` & Corpus)

1. **Remote Research & Reference Capture**:
   - Connect to `old-pc` via OpenDesk.
   - Send `Win+M` to minimize all windows and focus the Windows desktop.
   - Capture live JAWS 2025 speech transcripts when pressing:
     - `T` (jumping to items like "This PC").
     - Arrow keys (Up, Down, Left, Right).
     - Enter / Space on desktop items.
   - Contrast behavior when Chrome has focus vs when desktop has focus.
2. **Corpus Scenarios & Evidence (61–65)**:
   - Scenario 61: Desktop focus detection and suppression of table quick-nav `T`.
   - Scenario 62: Desktop first-letter navigation (`T` jumping to "This PC").
   - Scenario 63: Desktop arrow navigation and index announcement (`"{Name} {Index} of {Total}"`).
   - Scenario 64: Mode switching from web browser (`VirtualBuffer`) to desktop (`Win+M`) and back (`Alt+Tab`).
   - Scenario 65: File Explorer list view first-letter and arrow navigation.
3. **Execution & Regression Testing**:
   - Run `cargo test` across all scenarios.
   - Verify `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`.
