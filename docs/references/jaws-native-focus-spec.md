# JAWS Native Control Focus Speech: Evidence-Derived Specification

Observed black-box behavior of JAWS 2025.2505.43.400 on Windows 10 (`old-pc`) in the WinForms
fixture `corpus/fixtures/desktop-forms.ps1`. Each rule cites `scenario/step`. View a
transcript with `cargo run -p sr-cli -- evidence <scenario-id>`. Implemented in
`crates/sr-core/src/native_focus.rs`.

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

Menu speech is not implemented yet.
