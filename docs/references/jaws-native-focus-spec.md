# JAWS Native Control Focus Speech: Evidence-Derived Specification

Observed black-box behavior of JAWS 2025.2505.43.400 on Windows 10 (`old-pc`) in the WinForms
fixture `corpus/fixtures/desktop-forms.ps1`. Each rule cites `scenario/step`. View a
transcript with `cargo run -p okur -- evidence <scenario-id>`. Implemented in
`crates/okur-core/src/native_focus.rs`.

Notation is the same as `jaws-virtual-buffer-spec.md`: `‹ ›` stand for JAWS's role markers
and `|` for a line break in one step's speech. **[obs]** is deterministic across 5 trials;
**[hyp]** still needs an experiment.

## 1. Order of speech on a focus change

1. **The container, when focus enters it** from outside:
   - a named group: `Plan` [obs, 301/s4];
   - a list box: `Fruit: ‹ List box ›` [obs, 301/s6];
   - a tree view: `‹ Tree view ›` [obs, 301/s7];
   - a list view (Win32 `SysListView32`): `‹ List view ›` [obs, 301/s8].
   Windows, panes and unnamed groups announce nothing.
2. **The control:** name, role and state.
3. **Value, list-view columns, position:** for example `Germany`, `24 KB`, `2 of 3`.
4. **The usage hint** for the control type.

Positions are spoken for radio buttons, combo boxes, list items and tree items. Tabs and
buttons get none. When the control reports no position (WinForms reports none), JAWS still
gives one, counted among siblings of the same type. For a combo box, it is the selected
item's position among the combo's items.

## 2. Phrasing by control type (Tab focus) [obs, 301]

| Control | Speech |
|---|---|
| Edit | `Name: ‹ Edit › \| Ada \| Type in text.` (no value line when empty) |
| Check box, checked | `Subscribe to updates ‹ check box › ‹checked› \| To clear checkmark press Spacebar.` |
| Check box, not checked | `Remember me ‹ check box › ‹not checked› \| To check press Spacebar.` |
| Radio button | `Plan \| Pro ‹ radio button › ‹checked› \| 2 of 3 \| To change the selection press Up or Down Arrow.` |
| Combo box | `Country: ‹ Combo box › \| Germany \| 2 of 3 \| To change the selection use the Arrow keys.` |
| List box item | `Fruit: ‹ List box › \| Bananas \| 2 of 3 \| To move to an item press the Arrow keys.` |
| Tree item | `‹ Tree view › \| Documents ‹closed› \| 1 of 2 \| To move through or expand items use the Arrow keys.` |
| List view item | `‹ List view › \| budget.xlsx \| 24 KB \| 1 of 3 \| To move to items use the Arrow keys.` |
| Tab | `General ‹ Tab › ‹selected› \| To switch pages, press Control+PageDown.` |
| Button | `OK ‹ Button › \| To activate press spacebar.` |

Native buttons say "press spacebar". Buttons in a Chrome page say `To activate press Enter.`
[obs, 204/s10].

## 3. Arrow keys inside a container [obs, 303]

- **List box and list view:** item, columns and position, with no container and no hint.
  Examples: `Cherries | 3 of 3`, `notes.txt | 2 KB | 2 of 3` [303/s1, s10–s11].
- **Past the last list item:** silent [303/s2].
- **Tabs:** each tab is spoken in full with its hint, `Advanced ‹ Tab › ‹selected› | …` [303/s13].
- **Tree view** (known gap, needs a dedicated experiment):
  - Down speaks `level 0 | Pictures | 2 of 2`, but one trial of five said only `Pictures` [303/s5].
  - Up to an item already visited speaks `Documents | ‹closed›` with no position [303/s6].
  - Right Arrow expands: `Documents | ‹open› | 2 items` [303/s7].
  - Down into a child: `level 1 | Reports | 1 of 2` [303/s8].

## 4. Menus [obs, 302]

| Step | Speech |
|---|---|
| Alt | `Menu bar \| File \| To navigate press Left or Right Arrow.` |
| Down (opens the menu) | `menu \| File menu \| New \| To move through items press up or down arrow.` |
| Down | `Open...` |
| Right (next menu) | `Edit menu \| Copy` |
| Escape | `Edit \| sub menu \| Edit sub menu` |
| Escape (leaves the menus) | `Leaving menus \| Name: ‹ Edit › \| Ada \| Type in text.` |

Implemented by `native_focus::Menus`. JAWS defers the closed-popup announcement until no further event supersedes it: in s5 the next menu opens at once, and in s7 nothing follows. The engine models this with `on_idle`, which parity calls at the end of each step.

## 5. Changes without a focus change [obs, 304]

JAWS speaks changes only of the focused control (`EventConverter`, `native_focus::change_speech`).

| Step | Speech |
|---|---|
| Space on a check box | `‹not checked›` / `‹checked›` |
| Arrow keys on radios | the newly checked radio, as on focus, within its group (`Team ‹ radio button › ‹checked› \| 3 of 3 \| hint`); unchecking the old radio is not spoken |
| Down/Up in a collapsed combo box | `Brazil \| 3 of 3` |
| Alt+Down (open the drop-down) | `Open List Box \| Germany \| 2 of 3` |
| Down in the open drop-down | `Brazil \| 3 of 3` (MSAA focus moves into its `ComboLBox`; spoken as the combo box's value) |
| Enter (close it) | the combo box as on focus, with its hint |
| Right on a tree item (expand) | `Documents \| ‹open› \| 2 items` |
| Left on a tree item (collapse) | `Documents \| ‹closed› \| 1 of 2` |

WinForms reports a combo box's expanded state only through MSAA, so the converter merges the focused object's MSAA state bits on focus and change events.

## 6. Windows and dialogs [obs, 306]

- **A new active window:** its title, then the focused control. The active window is the innermost window in the focus chain (UIA nests a dialog under its owner). Windows are told apart by title and class.
- **A Win32 dialog box (`#32770`, such as a message box):** `title ‹ dialog › | its static text | focused control`. Example: `SR Dialog Fixture ‹ dialog › | Save changes to the document? | Yes ‹ Button › | To activate press spacebar.`
- **A WinForms modal form:** its title alone. Its static text is not read (306/s10).
- **Insert+T:** `Title is | SR Dialog Fixture`.
- **Alt+Tab:** sometimes preceded by the task switcher (`Task Switching | To move to an item press the Arrow keys.`); this varies between trials (306/s7).
- [hyp] An unlabelled edit takes the window title as its name (306/s5).
- **A dialog box's labels** (a static text that names a control beside it) are not its
  static text: they are said with their controls. Observed on the dev box, 2026-10-17, with
  Okur's own settings dialog: `Okur Settings ‹ dialog › | Speech Synthesizer: ‹ Combo box ›
  | No speech | 4 of 4 | To change the selection use the Arrow keys. | Alt+y`.
- **Group boxes** name every control inside them, before its own name, on every focus:
  `Speech Rate: ‹ left right slider ›`, `Reading Show Virtual Viewer on Screen ‹ check
  box ›`. A group box frames its controls on screen but is their sibling, so the tracer finds
  it by geometry (`screen_lines::group_box_of`). Buttons outside every group box get none.
- **Access keys** come last, on arrival: as UI Automation gives them for a control named by
  a label (`Alt+v`, `Alt+r`), and as `alt+` and the letter written in the text for a
  control with its own (`alt+S` for "&Show Virtual Viewer on Screen", `alt+A` for "&Apply").
  A button without one (OK) says none.
- **A trackbar's value** is a percentage, said in words: `50 percent`.
- Research, dev box: these need old-pc scenarios before tests rely on JAWS evidence.

## 7. Editing text [obs, 305]

Implemented in `okur-core::editing`. JAWS speaks after the application handled the key, from the field's text, caret and selection (`TextContext`) before and after it.

| Key | Speech |
|---|---|
| Left / Right | the character at the caret; `blank` at the end |
| Ctrl+Left / Ctrl+Right | the word at the caret |
| Up / Down | the line at the caret |
| Home / End | nothing (single-line edit) |
| Shift+Left / Shift+Right | `d \| selected` (the character added, then "selected") |
| Shift+End and other Shift moves | `selected \| a` ("selected", then the text added) |
| Backspace | the deleted character |
| Delete | the character now at the caret |
| Insert+Up (Say Line) | `Name: ‹ Edit › \| d` (the field, then the line; an empty line gives the field alone) |
| Typing | nothing in the speech history (305/s10–s12). Audio was heard, but old-pc's JAWS has typing echo **off** (`TypingEcho=0`), so that audio was not echo. To be identified with the speech recorder voice. |

- **Reading the field:** a field is read through IA2 in Chromium pages, through UIA TextPattern elsewhere, and with window messages (`WM_GETTEXT`, `EM_GETSEL`) for subclassed Win32 edits that UIA does not expose (WinForms).
- **Open question:** 305/s13 (Shift+Home over `bo ` in `bo d`) speaks `selected | do`.

## 8. The JAWS key and JAWS's settings [obs, 309, 310]

- **The reference machine is not on JAWS's defaults.** Its `DEFAULT.JCF` sets:
  - `KeyboardType=Laptop`;
  - `TypingEcho=0` (echo off);
  - `AutoFormsMode=0` (manual forms mode).

  JAWS's defaults are the desktop layout, character echo and Auto forms mode. Evidence since 2026-10-14 records these files as `provenance.jaws_settings`. Replay uses them, falling back to `Settings::reference_machine()`.
- **Desktop layout:** Insert is the JAWS key.
- **Laptop layout:** CapsLock is the JAWS key, and Insert works as well (`CapsLock+T` and `Insert+T` both speak the title).
- **Choosing the JAWS key:** Settings Center can choose Insert, CapsLock or ScrollLock for a layout (`okur run --keyboard-layout`, `--okur-key`; Okur names it the Okur key).
- **CapsLock as the JAWS key, pressed twice quickly:** toggles Caps Lock and says `Caps lock on` or `Caps lock off`. Auto-repeat while it is held is not a second press.

## 9. Time and date [obs, 308]

The user locale's formats (old-pc: en-US).

| Keys | Speech |
|---|---|
| Insert+F12 | `10:19 PM` (short time) |
| Insert+F12 twice quickly | `10:19 PM \| Wednesday, October 14, 2026` (long date) |
| Ctrl+Insert+F12 | `10:19:27 PM` (long time) |
| Ctrl+Insert+F12 twice quickly | `10:19:31 PM \| Week 42` |

"Twice quickly" means within 500 ms. The week number follows the locale's week rule. In 2026 the US and ISO rules agree, so evidence cannot tell them apart.

## 10. The Virtual Viewer [obs, 311, 312]

**Insert+Alt+W (Virtualize Window)** says `Virtualizing current window | <first line>`. **Insert+Shift+V (Virtualize Current Control)** says `... current control ...`. The viewer's lines:

| Focus | Lines |
|---|---|
| A dialog box (`#32770`) | title; static text; its buttons on one line (`Yes No Cancel`) |
| A text field, whole window | the window title, then the field's lines (a label beside it is not included) |
| A text field, current control | the field's lines |
| Another control (WinForms button) | `Save... Button`, then a line holding a space |

Keys in the viewer:
- Up and Down read lines, and at the top or bottom read the same line again.
- Ctrl+Home and Ctrl+End read the first and last line.
- Left and Right read characters.
- Ctrl+Left and Ctrl+Right read words; a run of punctuation is a word (`Save`, `...`, `Button`).
- Insert+Up reads the line.
- Ctrl+A says `selected | <all lines>`.
- Ctrl+C says `Copied selection to clipboard`.
- Escape closes the viewer, and JAWS reads the focus again: `Save... ‹ Button ›`, or the edit's current line.
- Tab, Shift+Tab and Enter stay in the viewer silently. JAWS help says Tab closes it, but it did not close.
- Virtualizing again says `The current location is already in a virtual view`.

**JAWS defects we do not copy:**
- Its viewer drops characters outside the Basic Multilingual Plane (an emoji), and the leading tab.
- Its Ctrl+C reaches the clipboard as UTF-8 read as ANSI, which garbles non-ASCII text.

## 11. What is spoken: the speech history versus the synthesizer [obs, 307, 207]

JAWS's speech history keeps text from *before* its symbol processing: `(`, or an empty entry for a space, tab or line break. The synthesizer is then given the processed text. The harness records it with the **SR Speech Recorder** SAPI voice (`crates/okur-speech-recorder`, `observation.spoken`). `okur parity` compares both layers ("JAWS spoken parity").

**A character read on its own (Left/Right, Delete, the Virtual Viewer)** is always named, whatever the punctuation level. These are the exact strings JAWS sends a SAPI 5 voice (`okur-core::pronounce`):
- **Whitespace:** `space` (also a no-break space), `tab`, `blank` (a line break).
- **Names that keep the symbol:** `exclaim!`, `comma,`, `period.`, `question?`.
- **Other ASCII symbols:** `quote`, `number`, `dollar`, `percent`, `and`, `apostrophe`, `left paren`, `right paren`, `star`, `plus`, `dash`, `slash`, `colon`, `semi colon`, `less`, `equals`, `greater`, `at`, `left bracket`, `back slash`, `right bracket`.
- **Respelled for SAPI voices:** `carat`, `underline`, `grawv`, `left brace`, `vertical bar`, `right brace`, `til de`.
- **Non-ASCII symbols:**
  - `euro sign`, `em dash —`, `left quote`, `right quote`, `apostrophe` (for both ‘ and ’), `ellipsis …`, `bullet`;
  - `rightwards arrow`, `copyright`, `one half`, `not equal to`, `less-than or equal to`, `lamda`.
- **Emoji:** the Unicode name in capitals (`GRINNING FACE`).
- **Passed unchanged:** letters, digits, `é`, `ç`, Turkish letters, `°`, `×`, `÷`, `±`.
- **Capitals** are sent with pitch +4. The engine does not raise pitch yet.

The recorder voice uses JAWS's SAPI 5 (64-bit) symbol table. JAWS keeps one per synthesizer (`eloq.sbl`, `SAPI 5x 64.sbl`), so Eloquence may differ.

**Preformatted page text (`<pre>`)** keeps its tabs, runs of spaces and line breaks in the virtual buffer, as Chrome reports it and JAWS reads it (207).
