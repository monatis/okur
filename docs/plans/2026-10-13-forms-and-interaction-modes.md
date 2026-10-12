# Development Plan: Forms & Interaction Modes Parity

**Plan Date**: 2026-10-13  
**Target Milestone**: Milestone 3 – Forms & Interaction Modes (`16-form-next-control` through `20-form-checkbox-toggle`)  
**Prerequisite Handoff**: [`docs/handoff/2026-10-13-structural-navigation-and-quick-keys.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-structural-navigation-and-quick-keys.md)  
**Corpus Evidence**: [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/) (Scenarios `16` – `20`)  
**Fixture**: [`corpus/fixtures/forms-interaction.html`](file:///c:/Users/Gigabyte/screen-reader/corpus/fixtures/forms-interaction.html)  

---

## 1. Goal & Scope

Extend `sr-core` virtual cursor and buffer navigation to support JAWS forms mode and form control quick navigation keys:
- `16-form-next-control`: `F` key / `FocusToNextField` navigation across form controls (`Edit`, `Button`, `CheckBox`).
- `17-form-next-button`: `B` key / `FocusToNextButton` navigation to push buttons.
- `18-form-next-edit`: `E` key / `FocusToNextEdit` navigation to text input edit fields.
- `19-form-mode-entry-exit`: Virtual cursor / Forms mode transitions (Virtual Cursor Mode vs Forms Mode) triggered by `Enter` and `Escape`.
- `20-form-checkbox-toggle`: `X` key / `FocusToNextCheckBox` navigation and spacebar toggle state changes (`checked` / `not checked`).

---

## 2. Technical Design

1. **OS Accessibility Standard & Forms Interaction**:
   - Live form field interaction (typing, toggling checkboxes, pressing buttons) delegates to **[`PlatformBridge`](file:///c:/Users/Gigabyte/screen-reader/crates/sr-platform/src/bridge.rs)** delegating to **[`xa11y`](https://github.com/xa11y/xa11y)** (`press`, `toggle`, `set_value`, `focus`).
   - Canonical AOM JSON snapshot [`corpus/fixtures/forms-interaction.aom.json`](file:///c:/Users/Gigabyte/screen-reader/corpus/fixtures/forms-interaction.aom.json) will be generated for deterministic CI runs.
2. **Form Interaction State Machine in `VirtualCursor`**:
   - `InteractionState`:
     - `VirtualCursorMode`: Quick navigation and reading keys active.
     - `FormsMode`: Keystrokes pass through to active control; `Escape` exits back to `VirtualCursorMode`.
   - `enter_forms_mode(&mut self) -> Option<String>`
   - `exit_forms_mode(&mut self) -> Option<String>`
   - `toggle_active_control(&mut self) -> Option<String>` (spacebar toggle for checkboxes)
3. **Form Control Indices in `VirtualBuffer`**:
   - `form_controls: Vec<usize>` (all form controls: Edit, Button, CheckBox)
   - `buttons: Vec<usize>`
   - `edits: Vec<usize>`
   - `checkboxes: Vec<usize>`
4. **Form Quick Navigation Primitives**:
   - `move_next_form_control(&mut self) -> Option<String>` / `move_prior_form_control(&mut self) -> Option<String>`
   - `move_next_button(&mut self) -> Option<String>` / `move_prior_button(&mut self) -> Option<String>`
   - `move_next_edit(&mut self) -> Option<String>` / `move_prior_edit(&mut self) -> Option<String>`
   - `move_next_checkbox(&mut self) -> Option<String>` / `move_prior_checkbox(&mut self) -> Option<String>`
5. **Speech Announcements for Forms**:
   - Edit: `"{label} Edit"` / `"{label} Edit Type in text"`
   - Button: `"{label} Button"`
   - Checkbox: `"{label} Check Box not checked"` / `"{label} Check Box checked"`
   - Forms mode toggle cues: Enter/exit audio indicator tokens.

---

## 3. Implementation Steps

1. **Create Canonical AOM JSON Fixture**:
   - Create `corpus/fixtures/forms-interaction.aom.json` mirroring `forms-interaction.html`.
2. **Add Form Indices to `sr-core::VirtualBuffer`**:
   - Index `form_controls`, `buttons`, `edits`, `checkboxes` during linearization.
3. **Implement Interaction State & Control Actions in `VirtualCursor`**:
   - Add `forms_mode_active: bool`, `enter_forms_mode()`, `exit_forms_mode()`, and `toggle_control()`.
4. **Wire Differential Runner for Scenarios 16–20**:
   - Map `FocusToNextField`, `FocusToNextButton`, `FocusToNextEdit`, `FocusToNextCheckBox`, `Enter`, `Escape`, `Space`.
5. **Verification**:
   - Verify `sr-cli test --scenarios 16..20` achieves 100% parity.
   - Run quality gates (`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`).
