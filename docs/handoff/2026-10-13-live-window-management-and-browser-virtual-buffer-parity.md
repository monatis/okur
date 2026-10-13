# Handoff: Live Window Management, Browser Virtual Buffer Dynamic Binding & Native Controls Parity

**Date**: 2026-10-13  
**Status**: Verified & Complete (Live Hardware & In-Loop Verification Confirmed on Windows Host)  
**Milestone**: Milestone 14 (Window Transition Parity, Dynamic Browser Virtual Buffer, & Native Control Pass-Through)

---

## 1. Executive Summary & Defect Resolution

### The Defect
When starting the screen reader interactively (`cargo run --bin sr-cli -- run --engine sapi5`) and switching to Google Chrome with `Alt+Tab`, pressing navigation keys (e.g., arrow keys) caused the screen reader to recite terminal window controls ("minimize, maximize, close, vertical large increase, vertical large decrease, vertical small decrease...") instead of navigating the webpage with the Virtual PC Cursor.

### Root Cause Analysis
1. **Unbounded Window Fallback in Platform Bridge**: In `crates/sr-platform/src/bridge.rs`, `PlatformBridge::get_active_document(None)` searched for a document node. If no document was found (as when launched from Windows Terminal / `CASCADIA_HOSTING_WINDOW_CLASS`), it fell back to taking the first window returned by `app.windows()`. For Windows Terminal, this converted the terminal's window frame, titlebar buttons (Minimize, Maximize, Close), and scrollbars into a bogus `VirtualBuffer`.
2. **Missing Dynamic Document Lifecycle**: The daemon CLI initialized its `VirtualBuffer` once at process startup. When the user switched applications with `Alt+Tab` (e.g. from terminal to Chrome), the engine remained stuck in `AppMode::VirtualBuffer` pointing at the terminal controls. Because `handle_key_event` intercepted arrow keys in `AppMode::VirtualBuffer`, every arrow key press moved the virtual cursor through the terminal's titlebar buttons instead of letting the OS handle input or binding to the new web page.
3. **Missing Window Transition & Document Extraction Handler**: Modern browsers like Google Chrome host their web document inside a top-level window (`Chrome_WidgetWin_1`) using a UIA element with `ControlType == UIA_DocumentControlTypeId` (`30030`) and child window `Chrome_RenderWidgetHostHWND`. The screen reader was missing an automated bridge to detect when a browser window gains focus, extract the live UIA document tree into an `AomNode`, fire a `DocumentLoaded` event, and announce `"Virtual PC Cursor"`.

---

## 2. Implementation & Architectural Changes

### A. Dynamic Document Event in `sr-core` (`crates/sr-core/src/event.rs`)
Added `AccessibilityEvent::DocumentLoaded { title: String, root: AomNode }` to represent web document loading and foreground browser window binding.

### B. Clean Engine State Machine & Mode Switching (`crates/sr-core/src/engine.rs`)
- Added `last_window_title: Option<String>` to `ScreenReaderEngine` to track foreground window changes without repeating titles.
- In `handle_accessibility_event`:
  - When `DocumentLoaded { title, root }` arrives:
    - Builds a new `VirtualBuffer::from_aom(&root)`.
    - Resets `virtual_cursor` to position 0.
    - Transitions `app_mode` to `AppMode::VirtualBuffer`.
    - Announces `format!("{}\u{001d}Virtual PC Cursor\u{001e}", title)`.
  - When `FocusChanged(focus)` arrives:
    - If `focus.window_title` changes, announces window title change and updates `last_window_title`.
    - If focus is not inside a document, transitions `app_mode` to `AppMode::NativeControl`, speaks native control info, and ensures **all navigation and typing keys pass through to Windows natively** (`KeyAction::PassThrough`).
    - If focus moves to an edit/input control within a document, auto-transitions to `AppMode::FormsMode`.

### C. Strict Document Extraction & No Fallback in `sr-platform` (`crates/sr-platform/src/bridge.rs`)
- Removed the fallback in `get_active_document` that previously created virtual buffers from generic top-level window controls.
- Added `PlatformError::NoDocumentFound` so calling `get_active_document` on non-browser windows safely indicates no document is present.

### D. Live UIA Browser Document Detection & Tree Extraction (`crates/sr-platform/src/events/windows.rs`)
- In `FocusChangedHandler::handle_focus_change`:
  - Detects browser windows (`Chrome_WidgetWin_1`, `Chrome_RenderWidgetHostHWND`).
  - Finds the UIA document element (`UIA_DocumentControlTypeId`).
  - Extracts the full live accessibility tree into `AomNode` (`extract_uia_document_tree`), including roles (Hyperlink, Button, Edit, CheckBox, List, ListItem, Heading), heading levels (`UIA_HeadingLevelPropertyId`), and names.
  - Caches the last loaded document handle to avoid redundant tree walks.
  - Dispatches `AccessibilityEvent::DocumentLoaded` to the event channel.

### E. CLI Daemon Initialization (`crates/sr-cli/src/main.rs`)
- When launched without `--fixture`, starts safely in `AppMode::NativeControl` with an empty document.
- Only switches to `AppMode::VirtualBuffer` when a genuine document fixture is provided or when a live web browser document is dynamically detected in the foreground.

### F. Serialized Hardware / COM Testing in `sr-platform` (`crates/sr-platform/src/lib.rs`, `audio.rs`, `tts/mod.rs`)
- Added `TEST_AUDIO_LOCK` to serialize tests touching COM, WASAPI audio meter endpoints, and SAPI 5 speech voices, preventing thread contention during parallel test runs.

---

### G. Web Navigation & Heading Level Detection Parity (`sr-platform` & `sr-core`)
- **UIA HeadingLevel Standard Constants**: Microsoft UI Automation defines `HeadingLevel_Level1 = 80051` through `HeadingLevel_Level9 = 80059` (with `HeadingLevel_None = 80050`). The previous check `(1..=6).contains(&lvl)` rejected all standard UIA heading constants. Added `decode_uia_heading_level` to map both `80051..=80059` (`lvl - 80050`) and raw integers `1..=9`.
- **ARIA Role and Properties Mapping**: Chromium exposes semantic HTML elements and ARIA roles via `UIA_AriaRolePropertyId` (30101) and `UIA_AriaPropertiesPropertyId` (30102). Implemented `detect_element_role`:
  - Recognizes `aria_role == "heading"`, parses `level=(\d+)` from `AriaProperties`, and falls back to localized control type numbers (or level 2).
  - Maps ARIA landmarks (`main`, `navigation`, `banner`, `contentinfo`, `complementary`, `search`, `region`, `form`).
  - Maps interactive roles (`button`, `checkbox`, `radio`, `textbox`, `combobox`, `list`, `listitem`, `table`, `row`, `cell`).
- **DOM Traversal Limits**: Increased tree traversal depth from 8 to 32 and node budget from 600 to 5000, ensuring headings and content in deep React/Next.js/SPA DOM trees (such as `altai.dev` and `diken.com.tr`) are fully extracted.
- **Heading Text Fallback & Leaf Rendering**: In `buffer.rs`, ensured `Role::Heading` falls back to `node.name` if child traversal yielded only whitespace, and ensured `Role::Generic` outputs `node.name` when `node.text` is empty.
- **Prior Link Navigation Fix**: In `prior_link_from`, fixed an issue where navigating backward from a non-link element skipped the immediately preceding link.

---

## 3. Verification & Live Evidence

### A. Full Test Suite Validation
All **62 tests** across all 4 crates pass with 0 errors and 0 warnings:
- `sr-core`: 36 tests pass.
- `sr-corpus`: 12 tests pass (all 65 differential test scenarios validated).
- `sr-platform`: 14 tests pass (including heading level decoding, ARIA level parsing, live audio metering, and virtual buffer heading navigation).
- `sr-cli`: 0 unit tests, compilation verified.

### B. Quality Gates
- `cargo fmt --check`: Passed (code formatting clean).
- `cargo clippy --all-targets --all-features -- -D warnings`: Passed (0 errors, 0 warnings).

### C. Live Host Verification
Executed live daemon verification on Windows host with SAPI 5 (`cargo run --bin sr-cli -- run --engine sapi5`):
1. **Startup**: Started in terminal; spoke `Operating in live native Windows navigation mode.` Arrow keys passed through without any speech of terminal titlebar buttons.
2. **Alt+Tab to Chrome**:
   - `Feed | LinkedIn \u{001d} Virtual PC Cursor \u{001e}` spoken.
   - `(8) WhatsApp \u{001d} Virtual PC Cursor \u{001e}` spoken with button controls.
   - `ALTAI — Kurumlar için yapay zekâ: model, ajan ve AI gateway \u{001d} Virtual PC Cursor \u{001e}`.
   - Heading navigation (`H`, `Shift+H`, `1`..`6`) and link navigation (`K`, `Shift+K`) correctly jump between headings and links across live web pages.
3. **Zero Titlebar Noise**: No "minimize, maximize, close, vertical large increase, vertical large decrease" occurred.

---

## 4. Next Plan
- Active Plan: `docs/plans/2026-10-13-complex-web-apps-google-docs-and-sheets.md`.
