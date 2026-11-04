# Handoff: table edges, laptop cursor keys, key echo (2026-11-04)

Follows [`2026-11-04-smart-navigation-and-quick-key-delay.md`](2026-11-04-smart-navigation-and-quick-key-delay.md).
Plan: [`../plans/2026-11-03-review-round-2-before-cross-platform.md`](../plans/2026-11-03-review-round-2-before-cross-platform.md),
§3. Branch `windows-ui-round-2`; `main` moved with every commit.

old-pc's Windows Security Alert is still open (the owner's to answer). Recordings in a page are
clean while it is (Chrome's alone in their traces, the mouse pointer moved off the alert first
with `sr-harness/mouse-off.ps1`, a move, no click); a desktop fixture's start brings the alert
forward (1142 again), so those wait.

## Found first-hand (old-pc, 3 trials each, imported)

- **1143, Smart Navigation at a table's other edges:** Right past a row that is not the last
  goes on to the next row's first cell, "Row 2 | Branch | Central | Column 1 of 3"; Up on the
  first row goes to "table with 3 columns and 3 rows"; Left on the first cell gives the
  synthesizer a line break alone; Ctrl+Right and Ctrl+Left in a cell say the cell whole and
  stay ("9 a dot m period. to 9 p dot m period.", read as a word); Up from "table end" goes back
  to the last cell with its place; Shift+T says no place after the first cell.
- **1145, the laptop layout's cursor keys** (Freedom Scientific's hotkeys list, laptop column):
  CapsLock+P "JAWS Cursor", CapsLock+Semicolon "virtual PC Cursor", CapsLock+Left Bracket
  "Route JAWS to PC", CapsLock+Apostrophe "Route PC to JAWS". Okur had none of them.
- **1146, keys that type nothing at typing echo Characters** (JAWS's default; the lab's is
  None): the synthesizer gets "Enter", "End", "F2", "Home", "Escape" before what the key does,
  out of the history; Shift, Control, Delete at a field's end and Page Down say nothing. Home
  says the character it lands on ("I", then "india"), End "blank".
- **1147, the same at the lab's echo, None:** no key is named; Home and End in a page's field
  still say the character, so those are the field's, the names the echo's (a WinForms one-line
  edit is silent, 305).
- **Chrome reports a field's caret before its text pattern has moved it:** 1147's trace read
  the caret 8 ms after End still at 0. End in a page's field is said from the key ("blank").
- **Chrome's UIA gives a plain table the ARIA role "grid"** on the Table type; an ARIA grid is
  the DataGrid type.

## Built

| Commit | What |
|---|---|
| `0a3fb3ae` | Laptop layout: the cursor keys; Left and Right Bracket keys by their place |
| `7e9c08f6` | Smart Navigation: the table's other edges, a cell whole on the word keys |
| `3523c8b6` | UIA: a page's plain table is a table, not a grid (53 UIA steps) |
| `698024e6` | Edits: Home and End in a page's field, Delete at a line's end, Page Up and Down in a one-line field |
| `b45e95fb` | Typing echo names Enter, Escape, Home, End and the function keys before what they do (`speak_key_label`; the speech thread lets the key's own speech follow the name, not cut it) |
| `c32b0ea7`, `f5467920`, `fd40e4dc`, `7db8906b` | corpus: 1143, 1145, 1146, 1147 |

Live with Okur on old-pc (`run-scenario.ps1 -Reader okur`, `b45e95fb`'s build): 1146's history
as JAWS's, and the debug log names Enter, End, F2, Home and Escape.

## Figures

At `c071fab8`:

```
JAWS parity (Ia2): 11608/12133 steps match exactly; 383 accepted divergences not counted (0 scenarios could not run)
JAWS spoken parity: 11354/12199 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Uia): 10434/12027 steps match exactly; 383 accepted divergences not counted (15 scenarios could not run)
Buffer invariants: 0 violation(s)
```

(`okur-lab parity`, `parity --spoken`, `parity --api uia`, `invariants`.) `cargo test --workspace
--no-fail-fast` (4 threads), summed: 1658 passed, 0 failed, 201 ignored. Every change was
compared over all three layers with `parity-snapshot.sh` before and after: no step got worse.

## Later the same day

- **1148 and 1149** (`c36225a6`): at typing echo Words and Both the same keys are named as at
  Characters, so the names follow any echo but None. At Both a one-letter word is not said
  again at Space; that was also 711/s7's accepted divergence ("c", "d", Backspace, Enter says
  only "Enter"), JAWS's rule rather than an accident: the divergence is removed and Okur matches
  (`73667384`).
- **Home in a page's field is said from the key** (`EditKey::LineStart`, `73667384`), as End
  is: after typing, the caret read at once still stood at the text's end.
- **Open: "Misspelled" in a page's field.** JAWS says "Misspelled" before Home's character on
  "Izmira" (1148/s7, 1149/s7); Okur says the mark when the field reports the word misspelled,
  but the traced reads of Chrome's field (IA2, `chromium_text_state`) carry no `invalid:spelling`
  run even three seconds after typing; 284's "‹Misspelled › ada" steps (s9, s12) differ the
  same way. The probe `okur-lab focus-text` (the focused Chromium object's raw IA2 attribute
  runs) was run on old-pc in the sign-up form's Full name field, focused there by Tab
  (`sr-harness/spell-probe.ps1`, "ada" and Space typed in forms mode, 3 s waited). With COM
  joined it reads the field as Okur does, `text "ada " caret 4 misspelled []`, and one attribute
  run, `0..4 background-color:...;language:en;writing-mode:lr;`: no `invalid:spelling`. UI
  Automation's annotation types on the field's range and each word are its reserved "not
  supported" (VT_UNKNOWN), and its range runs on past the field. So JAWS's "Misspelled" comes
  from somewhere neither read sees (the field's inner text object, or later): next, read the
  input's IA2 children's attributes.

At `73667384`: `JAWS parity (Ia2): 11623/12150 steps match exactly; 382 accepted divergences
not counted (0 scenarios could not run)` (`okur-lab parity`); `cargo test --workspace`: 1658
passed, 0 failed, 201 ignored.

## Next

1. When old-pc's Security Alert is answered: 1142, then the Scan Cursor in a WPF window (1013
   with `"trace_window": true`).
2. Key labels' dialog (Manage Key Labels) and the names at Words and Both, which 1146 did not
   cover; whether a layer's keys are named (inferred not).
3. Screen Layout's lines (984-993) need element positions in the snapshots.
4. The review's P3 and P10; the Office items when Office is back.
