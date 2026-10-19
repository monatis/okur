# JAWS 2025 Virtual Cursor Keys

The virtual cursor's keys in JAWS 2025.2505.43.400, and which of them Okur has. They were read
from `Default.JKM`, JAWS's declarative key map, on `old-pc`:
- the `[Quick Navigation Keys]` section on 2026-10-14;
- `[Quick Navigation Keys]`, `[virtual keys]`, `[SayAll keys]` and `[Common Keys]` again on
  2026-10-19.

This is a list of keystrokes and command names only; no script code was read or copied. Okur's
side was checked against `crates/okur-core/src/command.rs` on 2026-10-19.

What the keys do is documented by Freedom Scientific:
- "Navigation Quick Keys for JAWS" (Surf's Up, support.freedomscientific.com/SurfsUp/QuickKeys.htm);
- "JAWS Keystrokes" (support.freedomscientific.com/content/html/jawshq/JAWS-Keystrokes.html,
  revised 2013, so older than the key map where they differ).

The scenario named for a gap records JAWS doing it, per the owner's rule: document, record on
old-pc, then build. They are written in `corpus/pending/` and queued on old-pc.

**Binding model.** JAWS binds commands to logical key names (`SemiColon`, `Apostrophe`,
`Comma`, …; see `[Key Name Translations]`), which correspond to physical keys, not to the
character a layout produces. On the Turkish Q layout, the physical `;` key types `ş`, and JAWS
runs `MoveToNextOnMouseOverElement` for it (evidence 110/s8). Okur binds by physical key in the
same way, so muscle memory transfers across keyboard layouts.

## Quick navigation keys

Shift with a key is the "prior" variant of the same command. Freedom Scientific: Navigation
Quick Keys are "on by default"; the commands to move by heading, paragraph, form control, table,
list, same or different element, next element and non-link text "all work while you are using
the Say All command".

| Key | JAWS command | Documented as | Okur | Scenario |
|---|---|---|---|---|
| 1–6 | MoveToNextHeadingLevelN | Heading at Level | built | 104, 275, 277 |
| 7, 8, 9, 0 | SpeakPlaceMarkerN(1..4) (Shift: MoveToPlaceMarkerN) | PlaceMarker 1 to 4 | gap | helper dialogs plan |
| A | MoveToNextRadioButton | Radio Button | built | |
| B | MoveToNextButton | Button | built | |
| C | MoveToNextCombo | Combo Box, List Box, or Tree View | built | |
| D | MoveToNextDifferentElement | Different Type Element | gap | 290 |
| E | MoveToNextEdit | Edit Box | built | |
| F | FocusToNextField | Form Control | built | |
| G | MoveToNextGraphic | Graphic | built | |
| H | moveToNextHeading | Heading | built | |
| I | MoveToNextListItem | Item in a List | built | |
| J | JumpToLine (Shift: JumpReturnFromLine) | Jump to Line, Return to Previous Line | gap | 299 |
| K | MoveToNextPlaceMarker | PlaceMarker | bound; PlaceMarkers not built | helper dialogs plan |
| L | MoveToNextList | List | built | |
| M | MoveToNextFrame | Frame | bound | |
| N | MoveToNextNonLinkText | Non-link Text | built | 289 |
| O | MoveToNextArticle | Article Region | built | |
| P | SayNextParagraph | Paragraph | built | 291 |
| Q | MoveToMainRegion (Shift+Q too) | Main Region | built | |
| R | MoveToNextRegion | Region | built | |
| S | MoveToNextSameElement | Same Type Element | gap | 290 |
| T | MoveToNextTable | Table | built | |
| U | MoveToNextUnvisitedLink | Unvisited Link | partial: every link counts as unvisited | |
| V | MoveToNextVisitedLink | Visited Link | gap: needs the visited state (IA2's traversed) | |
| W | MoveToNextWordFromList | Word from List | gap | 299 |
| X | MoveToNextCheckbox | Check Box | built | |
| Y | MoveToNextGlanceHighlight | Span (2013); Glance Highlight in the 2025 key map | gap | 292 |
| Z | MoveToNextDivision | Division | gap (its list, Ctrl+Insert+Z, is built) | 292 |
| `'` Apostrophe | MoveToNextTab | Tab Control | built | |
| `;` SemiColon | MoveToNextOnMouseOverElement | Move to OnMouseOver | bound; mouse-over detection missing | |
| `/` Slash | MoveToNextOnClickElement | Clickable Element | gap | 293 |
| `-` Dash | MoveToNextSeparator | Separator | gap | 292 |
| `=` Equals | MoveToFlowTo (Shift: MoveToFlowFrom) | | gap | 293 |
| `\` Backslash | MoveToNextMailToLink | | gap | 293 |
| Shift+`,` / Shift+`.` | StepToStartOfElement / StepToEndOfElement | Step Before / Past Element | gap | 294 |
| F3 / Shift+F3 | JAWSFindNext / JAWSFindPrior | JAWS Find Next / Previous | built | |

## Other virtual cursor keys (`[virtual keys]`)

| Keys | JAWS command | Okur | Scenario |
|---|---|---|---|
| Insert+Enter, Insert+Shift+Enter | MoveToNext/PriorNonLinkText | built | 289 |
| Ctrl+Insert+Right / Left | MoveToNext/PriorDifferentElement | gap | 290 |
| Ctrl+Insert+Up | MoveToPriorSameElement | gap | 290 |
| Alt+Insert+Home / End | MoveToFirstHeading / moveToLastHeading | built | 295 |
| Ctrl+Alt+Insert+1..6, with Shift | MoveToFirstHeadingLevelN / MoveToLastHeadingLevelN | built | 295 |
| Ctrl+Insert+Shift+Enter | moveToPriorHeading | gap | |
| Ctrl+Insert+Home / End | FocusToFirstField / FocusToLastField | built (the virtual cursor moves; the focus stays, as in JAWS) | 296 |
| Win+Alt+Down / Up | SayNextRow / SayPriorRow | built | 297 |
| Win+Alt+Right / Left | SayNextColumn / SayPriorColumn | built | 297 |
| Alt+Win+Numpad5 (Alt+Win+Comma) | ReadCurrentRow | built | 297 |
| Alt+Win+Period | ReadCurrentColumn | built | 297 |
| Ctrl+Win+J, Ctrl+Shift+Win+J | JumpToTableCell / JumpReturnFromTableCell | gap | |
| Alt+Win+Home / End | StepToStartOfElement / StepToEndOfElement | gap | 294 |
| Insert+N | NavigationModeToggle (quick keys on, off, Say All only) | built (Say All only acts as off until Say All takes keys) | 298 |
| Ctrl+Insert+N, Insert+Shift+Z | QuickNavigationKeysSuspendOrResume | built | 298 |
| Insert+X | SmartNavToggle | gap | |
| Insert+F3 | VirtualHTMLFeatures | built | |
| Insert+F5, F6, F7 | SelectAFormField, SelectAHeading, SelectALink | built | |
| Ctrl+Insert+A, B, C, E, G, L, O, P, Q, R, T, X, Z, `'` | the element lists | built | |
| Ctrl+Insert+Y | SelectAGlanceHighlight | gap (backlog: which text counts) | |
| Ctrl+Insert+Slash, Ctrl+Insert+SemiColon, Insert+Win+SemiColon | SelectAnOnClickElement, SelectAnOnMouseOverElement | gap | |
| Ctrl+Insert+Backslash | SelectAMailToLink | gap | |
| Ctrl+Shift+K, Ctrl+Win+K | SelectAPlaceMarker, DefineATempPlaceMarker | gap | helper dialogs plan |
| Ctrl+Insert+Enter | OnMouseOver | gap | |
| Shift+Insert+F1, Ctrl+Shift+Insert+F1 | DisplayBasicElementInfo / DisplayAdvancedElementInfo | gap | |
| Ctrl+Insert+F7 | IndicateLinkSourceURL | gap | |
| F8 | SelectHTMLElement | gap | |
| Ctrl+Win+Equals | SelectaDropTarget | gap | |
| Ctrl+Win+Dash | FilterLiveRegionAnnouncements | gap | |
| Applications, Shift+F10 | VirtualContextMenu | not checked | |
| Space, Enter, Shift+Enter, Ctrl+Enter, Ctrl+Shift+Enter | VirtualSpacebar, Enter, ShiftEnter, ControlEnter, ControlShiftEnter | Space and Enter built; the others not checked | |

`[SayAll keys]`: PageUp and PageDown change the speech rate during Say All (gap).

## Common keys that act in pages (`[Common Keys]`)

| Keys | JAWS command | Okur | Scenario |
|---|---|---|---|
| Alt+NumpadPlus / Alt+NumpadMinus | SayNextSentence / SayPriorSentence | built | 291 |
| Alt+Numpad5 | SaySentence | built | 291 |
| Ctrl+Numpad5 | SayParagraph | built | 291 |
| Insert+PageUp / Insert+Home | SayFromCursor / SayToCursor | built | 291 |
| Insert+Z | VirtualPCCursorToggle | built | 298 |
| Insert+Escape | RefreshScreen | built | 298 |
| Insert+Delete, Ctrl+Insert+Delete | RouteVirtualToPc, RoutePCToVirtual | gap | |
| Insert+F9 | SelectAFrame | gap | |
| Ctrl+Insert+Down, Ctrl+Shift+Insert+Down | StartSkimRead, SkimReadDialog | gap | |
| Insert+5 | SayColor | gap | |
| Ctrl+Insert+Tab | CreatePrompt | gap | |
| Insert+Space, then X | DoFlexibleWebDialog (Flexible Web) | gap | |

Other `Default.JKM` facts:
- `[Keyboard Layouts]`: Desktop, Laptop and Kinesis.
- CapsLock is a JAWS key modifier in the laptop layout (`[Laptop Modifiers]`).
