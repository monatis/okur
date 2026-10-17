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
| `C` | `MoveToNextComboBox` | Combo Box (`Role::ComboBox`) | Forward | `{Label} edit combo {Value} {collapsed \| open}` |
| `Shift + C` | `MoveToPriorComboBox` | Combo Box (`Role::ComboBox`) | Prior | `{Label} edit combo {Value} {collapsed \| open}` |

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

### Who takes a key (`ScreenReaderEngine::handle_key_event`)

Each key goes to the first of these that applies (as of `b3b49cc`):

1. **The JAWS key** (Insert, or CapsLock in the laptop layout) and every chord with it: JAWS
   commands, in any mode.
2. **The Virtual Viewer** (Insert+Alt+W, the results viewer), while it is open.
3. **Menu mode**: a menu is open (`Menus::in_menu_mode`): the system said menu mode started
   (`EVENT_SYSTEM_MENUSTART`), a popup is open, or a menu item has the focus. Every key goes
   to the menu, Escape too. Menu mode ends with the menu-end event, or when a focus lands
   outside the menus (back in the page, or on a native control), which says "Leaving
   menus". It does not wait for an item's focus: Chrome's app menu opens with none, and the
   Down that focuses its first item must reach it.
4. **Forms mode**: a page's field, editable combo box, ARIA menu or application region has the
   focus. Keys go to the page; caret and edit keys are read, typed keys echoed by the typing
   echo setting. Escape leaves forms mode ("virtual PC Cursor"), except in an application
   region, whose keys are all the page's.
5. **Escape** closes the virtual cursor's element list, expanded combo box or dialog.
6. **The PC cursor**: the focus is outside a document (an application, the browser's address
   bar or toolbar). Keys go to the application; edit fields, and fields typed into such as a
   console, read their keys.
7. **The virtual PC cursor** in a page: Tab moves from the cursor, and reading and quick
   navigation keys are its commands.

The focus decides between 6 and 7 (`AppMode`: a page's document or not); a browser's own menu
over a page leaves forms mode as it was, so the page has its keys back when the menu closes.

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

---

## 4. ARIA ComboBox Interaction (`Alt+DownArrow`, `Alt+UpArrow`, `Escape`)

- **Role**: `Role::ComboBox`
- **Announcement Pattern**:
  - Closed: `{Label} edit combo {Value} collapsed`
  - Open: `{Label} edit combo {Value} open`
- **Keys**:
  - `Alt + DownArrow`: Expands combobox listbox and announces newly expanded state with active option.
  - `Alt + UpArrow` / `Escape`: Collapses the combobox listbox and announces `collapsed`.
  - `DownArrow` / `UpArrow` (while expanded): Cycles options in the combobox popup, updating value and announcing the selected option name.

---

## 5. Hierarchical TreeViews (`Role::TreeItem`)

- **Role**: `Role::TreeItem` (`Role::Tree`)
- **Announcement Pattern**:
  - Closed node: `{Label} closed level {N}`
  - Open node: `{Label} open level {N}`
  - Leaf node: `{Label} level {N}`
- **Keys**:
  - `RightArrow`:
    - On closed expandable item: Expands item and announces `open {Label} level {N}`.
    - On open expandable item: Navigates cursor to first child tree item.
  - `LeftArrow`:
    - On open expandable item: Collapses item and announces `closed {Label} level {N}`.
    - On leaf item or closed item: Navigates cursor up to parent tree item.

---

## 6. Range Widgets (Sliders & ProgressBars)

- **Slider (`Role::Slider`)**:
  - Announcement: `{Label} slider {Value}`
  - `RightArrow`: Increments slider value by step (or 1.0) and announces new numeric value.
  - `LeftArrow`: Decrements slider value by step (or 1.0) and announces new numeric value.
- **ProgressBar (`Role::ProgressBar`)**:
  - Announcement: `{Label} progress bar {Value}%`

