# Reference: Forms & Interaction Modes (JAWS 2025 Parity)

This reference document outlines the keyboard conventions, forms mode state machine, speech announcements, and virtual buffer navigation behaviors for Freedom Scientific JAWS 2025 form controls and interaction modes supported by the screen reader engine.

---

## 1. Form Navigation Keys & JAWS Script Functions

| Key / Command | JAWS Script Function | Target Control | Direction | Announcement Format |
|---|---|---|---|---|
| `F` | `FocusToNextField` | Form Controls (`Edit`, `Button`, `CheckBox`) | Forward | Dependent on control type (see below) |
| `Shift + F` | `FocusToPriorField` | Form Controls (`Edit`, `Button`, `CheckBox`) | Prior | Dependent on control type (see below) |
| `E` | `MoveToNextEdit` | Text Input Edit (`Role::Edit`) | Forward | `{Label}  \u001d Edit \u001e  \n{Value}` |
| `Shift + E` | `MoveToPriorEdit` | Text Input Edit (`Role::Edit`) | Prior | `{Label}  \u001d Edit \u001e  \n{Value}` |
| `B` | `MoveToNextButton` | Push Button (`Role::Button`) | Forward | `{Label}  \u001d Button \u001e` |
| `Shift + B` | `MoveToPriorButton` | Push Button (`Role::Button`) | Prior | `{Label}  \u001d Button \u001e` |
| `X` | `MoveToNextCheckbox` | Check Box (`Role::CheckBox`) | Forward | ` {Label}  \u001d check box \u001e   \u001d{checked \| not checked}\u001e` |
| `Shift + X` | `MoveToPriorCheckbox` | Check Box (`Role::CheckBox`) | Prior | ` {Label}  \u001d check box \u001e   \u001d{checked \| not checked}\u001e` |

---

## 2. Forms Mode vs. Virtual Cursor Mode Transitions

In screen readers (following JAWS 2025 standards and cross-referenced with NVDA browse mode / focus mode pass-through architectures):

1. **Virtual Cursor Mode (Browse Mode)**:
   - Default reading mode for web pages and documents.
   - Keystrokes are intercepted by the virtual cursor to navigate by line, word, character, paragraph, or quick navigation keys (`F`, `B`, `E`, `X`, `H`, `1`–`6`, `L`, etc.).

2. **Forms Mode (Focus / Pass-Through Mode)**:
   - **Entry Trigger**: Pressing `Enter` on an editable input field (`Role::Edit`) enters Forms Mode (`cursor.forms_mode_active = true`).
   - **Entry Announcement**: 
     ```
     {DocumentTitle} 
      \u001d FormRegion \u001e 
     {FieldLabel}  \u001d Edit \u001e   
     {FieldValue}
     Type in text.
     ```
   - **Exit Trigger**: Pressing `Escape` exits Forms Mode back to Virtual PC Cursor mode (`cursor.forms_mode_active = false`).
   - **Exit Announcement**: `"virtual PC Cursor"`.

---

## 3. Checkbox State Toggling (`Spacebar`)

- **Key**: `Space` (`{SPACE}`)
- **Behavior**: Toggles the boolean state of `node.states.checked` (`checked` <-> `not checked`).
- **Announcement**:
  - When checked:
    ```
    {DocumentTitle} 
      \u001d FormRegion \u001e 
    {Label}  \u001d check box \u001e   \u001dchecked\u001e  
    To clear checkmark press Spacebar.
    ```
  - When cleared:
    ```
    {DocumentTitle} 
      \u001d FormRegion \u001e 
    {Label}  \u001d check box \u001e   \u001dnot checked\u001e  
    To check press Spacebar.
    ```
