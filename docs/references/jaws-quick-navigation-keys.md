# JAWS 2025 Virtual Cursor Quick Navigation Keys

Functional key-to-command interface of JAWS 2025.2505.43.400. It was read from the
`[Quick Navigation Keys]` section of `Default.JKM`, JAWS's declarative key map, on `old-pc`
(2026-10-14). This is a list of keystrokes and command names only; no script code was read or
copied. Shift+key is always the "prior" variant of the same command.

**Binding model.** JAWS binds commands to logical key names (`SemiColon`, `Apostrophe`,
`Comma`, …; see `[Key Name Translations]`), which correspond to physical keys, not to the
character a layout produces. On the Turkish Q layout, the physical `;` key types `ş`, and JAWS
runs `MoveToNextOnMouseOverElement` for it (evidence 110/s8). Our screen reader binds by
physical key in the same way, so muscle memory transfers across keyboard layouts.

| Key | JAWS command | Our status |
|---|---|---|
| 1–6 | MoveToNextHeadingLevelN | implemented |
| 7, 8, 9, 0 | SpeakPlaceMarkerN(1..4) (Shift: MoveToPlaceMarkerN) | gap |
| A | MoveToNextRadioButton | implemented |
| B | MoveToNextButton | implemented |
| C | MoveToNextCombo | implemented |
| D | MoveToNextDifferentElement | gap |
| E | MoveToNextEdit | implemented |
| F | FocusToNextField | implemented |
| G | MoveToNextGraphic | implemented |
| H | moveToNextHeading | implemented |
| I | MoveToNextListItem | implemented |
| J | JumpToLine (Shift: JumpReturnFromLine) | gap |
| K | MoveToNextPlaceMarker | gap (previously misbound to links) |
| L | MoveToNextList | implemented |
| M | MoveToNextFrame | gap (frames not produced by the UIA mapping) |
| N | MoveToNextNonLinkText | gap |
| O | MoveToNextArticle | implemented (previously misbound to MathML) |
| P | SayNextParagraph | gap |
| Q | MoveToMainRegion (Shift+Q too) | implemented |
| R | MoveToNextRegion | implemented |
| S | MoveToNextSameElement | gap |
| T | MoveToNextTable | implemented |
| U | MoveToNextUnvisitedLink | partial: visited state is not exposed by UIA, so every link counts as unvisited |
| V | MoveToNextVisitedLink | gap (same reason) |
| W | MoveToNextWordFromList | gap |
| X | MoveToNextCheckbox | implemented |
| Y | MoveToNextGlanceHighlight | gap |
| Z | MoveToNextDivision | gap |
| `'` Apostrophe | MoveToNextTab | implemented |
| `;` SemiColon | MoveToNextOnMouseOverElement | bound; detection not yet possible through UIA, so it reports none |
| `/` Slash | MoveToNextOnClickElement | gap |
| `-` Dash | MoveToNextSeparator | gap |
| `=` Equals | MoveToFlowTo (Shift: MoveToFlowFrom) | gap |
| `\` Backslash | MoveToNextMailToLink | gap |
| Shift+`,` / Shift+`.` | StepToStartOfElement / StepToEndOfElement | gap |
| F3 / Shift+F3 | JAWSFindNext / JAWSFindPrior | implemented |

Other `Default.JKM` facts relevant to the virtual cursor:
- `[virtual keys]`: Enter → Enter, Space → VirtualSpacebar, Shift+F10 / Applications → VirtualContextMenu, and F8 → SelectHTMLElement.
- `[SayAll keys]`: PageUp / PageDown change the speech rate temporarily during Say All.
- `[Keyboard Layouts]`: Desktop, Laptop and Kinesis. CapsLock is a JAWS key modifier in the laptop layout (`[Laptop Modifiers]`).
