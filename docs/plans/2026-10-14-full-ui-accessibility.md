# Plan: Full UI Accessibility, Round 1 (2026-10-14)

**Branch**: `full-ui-accessibility` (from `master` @ `67c0787`).
**Preceded by**: `docs/plans/2026-10-14-verification-harness-rebuild.md`; handoff
`docs/handoff/2026-10-14-ia2-paragraphs-native-focus-actions.md`.

## Goal

Close the gaps that block everyday use outside of reading pages, with each fix recorded
from JAWS, replayed by `sr-cli parity`, and checked live on old-pc with
`sr-cli run --debug-log`.

After this round the work becomes a loop:
1. Pick a JAWS feature from the JAWS help, the user manual, or the settings and configuration.
2. Record it.
3. Implement it.
4. Prove parity.
5. Pick the next feature.

## Work items, in order

1. **State changes without a focus change.** The tracer records `state_change`, `value_change`, `name_change` and `selection` events, but nothing converts them yet.
   - Native cases (scenario 304):
     - Space on a check box.
     - Arrow keys on radios.
     - Arrow keys in a collapsed combo box, and its drop-down.
     - Expanding and collapsing a tree item. This closes the 303/s7 gap.
   - Web cases (scenario 205):
     - Space on a check box or radio in browse mode.
     - A combo box value change in forms mode.
2. **Edit review details** (305):
   - Shift+arrow selection.
   - Backspace and Delete, which speak what they remove.
   - Insert+Up in a native edit.
3. **Windows and dialogs** (306, new fixture `desktop-dialogs.ps1`):
   - A message box: title, text and the focused button.
   - A second window, Alt+Tab between windows, and Alt+F4 back.
   - Insert+T (window title, an existing known-gap test).
4. **The JAWS key.** Insert and CapsLock are consumed with their commands (Insert+T, Insert+F12, Insert+Up, Insert+Tab), for native applications as well as pages.
5. **Pages that change.** Live regions, and buffer refresh on content changes, for chats like WhatsApp Web where new messages arrive.
6. **Existing gaps:**
   - The tree position dropped in 303/s6.
   - Level keys from the top of a page (a fixture with two h1 sections).
   - Removing the Explorer and Desktop special cases in `handle_focus_event` (rule 7), once desktop and Explorer evidence exists.

## Rules for this round

- **The live pipeline is the replay pipeline.** Any new event kind is converted in `event_trace` and consumed through `AccessibilityEvent`, live and in replay alike.
- **Every live claim cites a `--debug-log` excerpt.** Every parity claim cites `sr-cli parity`.
