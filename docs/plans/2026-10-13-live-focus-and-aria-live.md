# Development Plan: Live Focus Tracking, Auto Forms Mode & ARIA Live Regions

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 7 – Live Focus Tracking, Auto Forms Mode & ARIA Live Regions  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-live-interception-and-interactive-loop.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-live-interception-and-interactive-loop.md)  
**Corpus Evidence**: [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/) (Scenarios `01` – `30` passing with 100% parity)

---

## 1. Goal & Scope

With keyboard interception, `ScreenReaderEngine`, and the live CLI daemon (`sr-cli run`) operational, Milestone 7 advances the screen reader's real-time environmental awareness:
1. **OS Focus Tracking & Automatic Forms Mode**:
   - Monitor system-wide keyboard focus changes via OS accessibility event listeners (`IUIAutomationFocusChangedEventHandler` on Windows, `kAXFocusedUIElementChangedNotification` on macOS).
   - Synchronize `VirtualCursor` offset with the focused OS element.
   - Automatically trigger Forms Mode when focus transitions into an editable control (`Role::Edit`), emitting the canonical JAWS entry prompt.
   - Automatically exit Forms Mode back to Virtual PC Cursor when focus moves outside form controls.
2. **ARIA Live Region Monitoring (`aria-live="polite"` / `"assertive"`)**:
   - Detect dynamic DOM / subtree mutations and live region announcements.
   - Respect politeness levels:
     - `polite`: Enqueued after current speech finishes.
     - `assertive`: Immediately purges current speech and speaks the update.
3. **Cross-Platform Event Dispatcher Trait**:
   - `EventListener` trait in `sr-platform` abstracting platform accessibility events.
   - Zero-cost conditional compilation per OS (Windows UIA vs. macOS AX).

---

## 2. Technical Design

1. **Accessibility Event Subsystem (`sr-platform::events`)**:
   - `AccessibilityEventListener` trait:
     ```rust
     pub trait AccessibilityEventListener: Send + Sync {
         fn start(&mut self, sender: tokio::sync::mpsc::UnboundedSender<AccessibilityEvent>) -> Result<(), PlatformError>;
         fn stop(&mut self) -> Result<(), PlatformError>;
     }
     ```
   - Windows implementation using `IUIAutomation::AddFocusChangedEventHandler`.
   - macOS implementation using `AXObserverCreate` / `CFRunLoopAddSource`.
2. **Engine Event Integration (`sr-core::engine`)**:
   - Integrate `AccessibilityEvent` channel into `ScreenReaderEngine`.
   - Update cursor position and mode state upon receiving focus events.
   - Manage speech queuing for live region alerts.
3. **Differential Verification Harness**:
   - Author Scenarios 31–35 covering focus tracking, auto-forms mode, and ARIA live regions.
   - Record reference trials on `old-pc` running Freedom Scientific JAWS 2025.

---

## 3. Implementation Steps

1. **Define Event Types and Traits in `sr-platform` & `sr-core`**:
   - Create `AccessibilityEvent`, `FocusEvent`, and `LiveRegionEvent`.
2. **Implement Windows UIA Focus & Structure Event Handlers**:
   - Implement COM event handlers in `sr-platform` under `#[cfg(target_os = "windows")]`.
3. **Connect Events to `ScreenReaderEngine`**:
   - Handle focus changes, cursor alignment, and automatic forms mode switching.
4. **Author Corpus Scenarios (31–35) and Verify**:
   - Create test fixtures with `aria-live` and form inputs.
   - Differential verification against live JAWS on `old-pc`.
5. **Quality Gate**:
   - Ensure clean `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and passing `cargo test`.
