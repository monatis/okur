# JAWS Script & Function Catalog

## 1. Heading Navigation

| Script Name | Semantics & Purpose | Quick Nav | Keyboard | Context | Source File | Status | Caveats |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `MoveToNextHeading` | Moves to next HTML / Aria heading | `H` | `H` | Virtual Cursor | `Default.JSS` | Verified | Wraps to top at end of document |
| `MoveToPriorHeading` | Moves to prior heading | `Shift+H` | `Shift+H` | Virtual Cursor | `Default.JSS` | Verified | Wraps to bottom |
| `MoveToNextHeading1`..`6` | Moves to next heading of specific level (1-6) | `1`..`6` | `1`..`6` | Virtual Cursor | `Default.JSS` | Verified | Skips other heading levels |
| `MoveToPriorHeading1`..`6` | Moves to prior heading of specific level | `Shift+1`..`Shift+6` | `Shift+1`..`Shift+6` | Virtual Cursor | `Default.JSS` | Verified | Wraps within specified level |

---

## 2. Link Navigation

| Script Name | Semantics & Purpose | Quick Nav | Keyboard | Context | Source File | Status | Caveats |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `MoveToAnyNextLink` | Moves to next link (visited or unvisited) | *(none)* | `Tab` (DOM) | Virtual Cursor | `Default.JSS` | Verified | Does not filter visited state |
| `MoveToAnyPriorLink` | Moves to prior link | *(none)* | `Shift+Tab` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToNextUnvisitedLink` | Moves to next unvisited link | `U` | `U` | Virtual Cursor | `Default.JSS` | Verified | Skips visited links |
| `MoveToPriorUnvisitedLink` | Moves to prior unvisited link | `Shift+U` | `Shift+U` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToNextVisitedLink` | Moves to next visited link | `V` | `V` | Virtual Cursor | `Default.JSS` | Verified | Skips unvisited links |
| `MoveToPriorVisitedLink` | Moves to prior visited link | `Shift+V` | `Shift+V` | Virtual Cursor | `Default.JSS` | Verified | - |

---

## 3. Form Controls & Fields

| Script Name | Semantics & Purpose | Quick Nav | Keyboard | Context | Source File | Status | Caveats |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `FocusToNextField` | Moves to next form field (input, button, combo) | `F` | `F` | Virtual Cursor | `Default.JSS` line 17353 | Verified | May trigger auto-forms mode if edit |
| `FocusToPriorField` | Moves to prior form field | `Shift+F` | `Shift+F` | Virtual Cursor | `Default.JSS` line 17369 | Verified | - |
| `MoveToNextButton` | Moves to next HTML button | `B` | `B` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToPriorButton` | Moves to prior HTML button | `Shift+B` | `Shift+B` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToNextEdit` | Moves to next edit box (input type=text) | `E` | `E` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToPriorEdit` | Moves to prior edit box | `Shift+E` | `Shift+E` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToNextCheckbox` | Moves to next checkbox | `X` | `X` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToPriorCheckbox` | Moves to prior checkbox | `Shift+X` | `Shift+X` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToNextRadioButton` | Moves to next radio button | `A` | `A` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToNextCombo` | Moves to next combo box (select) | `C` | `C` | Virtual Cursor | `Default.JSS` | Verified | - |

---

## 4. Reading Commands

| Script Name | Semantics & Purpose | Quick Nav | Keyboard | Context | Source File | Status | Caveats |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `SayLine` | Speaks current line at virtual cursor | *(none)* | `Insert+UpArrow` | Global / Virtual | `Default.JSS` line 3344 | Verified | Verbatim line text |
| `SayCharacter` | Speaks current character | *(none)* | `Insert+Numpad5` | Global / Virtual | `Default.JSS` | Verified | - |
| `SayWord` | Speaks current word | *(none)* | `Insert+Numpad5 (2x)` | Global / Virtual | `Default.JSS` | Verified | Spells on second press |
| `SayPriorLine` | Moves up and speaks prior line | *(none)* | `UpArrow` | Virtual Cursor | `Default.JSS` | Verified | - |
| `SayNextLine` | Moves down and speaks next line | *(none)* | `DownArrow` | Virtual Cursor | `Default.JSS` | Verified | - |
| `SayAll` | Continuous reading from cursor | *(none)* | `Insert+DownArrow` | Virtual Cursor | `Default.JSS` | Verified | Async output |

---

## 5. Speech History Commands

| Script Name | Semantics & Purpose | Quick Nav | Keyboard | Context | Source File | Status | Caveats |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `ClearSpeechHistory` | Clears internal speech history buffer | *(none)* | *(none)* | Global | `Default.JSS` line 23498 | Verified | Speaks cmsgClearSpeechHistory then clears |
| `CopySpeechHistoryToClipboard` | Copies speech history to Windows clipboard | *(none)* | `Insert+Space, Ctrl+H` | Global | `Default.JSS` line 23503 | Verified | Requires SpeechHistory=1 in JCF |

---

## 6. Virtual Cursor & Mode Transitions

| Script Name | Semantics & Purpose | Quick Nav | Keyboard | Context | Source File | Status | Caveats |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `VirtualCursorToggle` | Toggles virtual PC cursor on/off | *(none)* | `Insert+Z` | HTML / Browser | `Virtual.jss` | Verified | Disables quick keys when off |
| `RefreshVirtualBuffer` | Forces refresh of virtual document | *(none)* | `Insert+Escape` | HTML / Browser | `Virtual.jss` | Verified | Resets buffer cache |
| `EnterFormsMode` | Enters forms mode (direct keystrokes to DOM) | *(none)* | `Enter` (on input) | Form Field | `Virtual.jss` | Verified | Emits enter tone |
| `ExitFormsMode` | Exits forms mode back to virtual cursor | *(none)* | `Escape` | Form Field | `Virtual.jss` | Verified | Emits exit tone |

---

## 7. Tables & Landmarks

| Script Name | Semantics & Purpose | Quick Nav | Keyboard | Context | Source File | Status | Caveats |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `MoveToNextTable` | Moves to next table | `T` | `T` | Virtual Cursor | `Default.JSS` | Verified | Announces rows and columns |
| `MoveToPriorTable` | Moves to prior table | `Shift+T` | `Shift+T` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToMainRegion` | Moves to main landmark | `Q` | `Q` | Virtual Cursor | `Default.JSS` | Verified | Announces main region |
| `MoveToNextRegion` | Moves to next landmark | `R` | `R` | Virtual Cursor | `Default.JSS` | Verified | - |
| `MoveToNextList` | Moves to next list | `L` | `L` | Virtual Cursor | `Default.JSS` | Verified | Announces item count |
| `MoveToNextListItem` | Moves to next list item | `I` | `I` | Virtual Cursor | `Default.JSS` | Verified | - |

---

## 8. Command Discovery & Extensibility

To discover and validate new JAWS scripts for this inventory:

1. **Inspect Keymaps**: Search `C:\ProgramData\Freedom Scientific\JAWS\2025\Scripts\enu\Default.JKM` and `Chrome.jkm` under sections like `[Quick Navigation Keys]`.
2. **Inspect Script Sources**: Verify the script definition in `Default.JSS` or `Virtual.jss`. Note whether it is a `Script` or a `Function`.
3. **Construct a Test Fixture**: Build a minimal HTML page exhibiting the target element and open it in Chrome.
4. **Test via FSAPI**: Run `capture.ps1 -Script <ScriptName> -TargetProcess "chrome"`.
5. **Test via Keyboard**: Run `capture.ps1 -Keystroke "<Key>" -TargetProcess "chrome"`.
6. **Compare & Append**: Verify parity, document any caveats (e.g. wrap-around, unvisited filtering), and add to this inventory.
