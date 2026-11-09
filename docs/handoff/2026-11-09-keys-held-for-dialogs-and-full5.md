# Handoff 2026-11-09 (late): keys held for Okur's dialogs, Okur's keys on Linux, full5 imported

After [`2026-11-09-linux-dialog-replay-and-full5.md`](2026-11-09-linux-dialog-replay-and-full5.md).

**Plan:** [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md); its status
section and known gaps are current (298f0c7c).

## What was done

**Keys typed before one of Okur's dialogs is up are its own** (e78db0b3, then two code reviews:
51547c3b, 298f0c7c; the GTK timing c47a5731).
- **The problem.** Insert+F7 then a letter at once: okur-ui takes a moment to show the Links
  List, and the letter went to the page as its quick key (the Linux lab's 222, its first Down).
- **The engine** (`okur-core/src/engine/held_keys.rs`): a key pressed after a dialog is asked
  for, and before okur-ui says it is up, is held. The modifiers, the toggles and Okur's chords
  are not held. As okur-ui says the dialog is shown (`FromUi::DialogShown`, by its id) the keys
  are pressed again in it, in order; past 2 s they go where the focus is. A key that finds the
  wait over goes after them. A second dialog asked for keeps them for itself.
- **Pressed again as they came** (`ApplicationActions::press_again`, `KeyEvent::native`):
  - Windows: by the virtual key, scan code and extended flag the hook got, all in one marked
    `SendInput`; a VK_PACKET's character from its scan code; an input method's key not.
  - Linux: by the keycode, through XTest (below).
  - Only the modifiers a held key had and the user does not hold are pressed around it. None
    the user holds is let go or pressed again: a left Shift put back while the user held the
    right one stayed down after them (the second review, 1).
- **okur-ui's word that a dialog is up** (`DialogShown`):
  - Win32: once the dialog is the foreground window.
  - GTK: at the first frame after its window turns active (c47a5731; see the research run
    below).
  - Parity's dialog mirror: at the dialog's first traced focus.
  - A focus matched by the title could be the same dialog just closed (the first review, 4).

**Okur presses keys itself on Linux** (f31824ee, 51547c3b, 298f0c7c;
`okur-linux/src/keyboard.rs`):
- XTest on the hook's connection, by the hook's keymap (refreshed on MappingNotify), checked for
  XTEST as the hook starts; the main block's keycode before the keypad's.
- The grab reads Okur's keys back as plain keys, by XTEST's keyboard attached to the hook's
  master, within 2 s: the Okur key held for CapsLock+O does not make its Down a command.
- While a key the engine kept is down its grab lasts, X sends every key to Okur, and a key
  replayed then goes nowhere: Okur's keys wait for that key's release (the first review, 1).
  A grab whose key is no longer down is over, its release missed; the shared state is reset as
  the hook starts.
- This also gives the laptop layout's reading commands in a text field their key on Linux
  (CapsLock+O's Down did nothing there).

**Okur's own dialogs are dialog boxes with their spec's default button** (f4fd1c14). GTK 4
marks no button IS_DEFAULT, so Insert+E said "Not in a dialog box" in Okur's Linux Links List
(the fifth review, 1). The engine keeps the dialogs it asks okur-ui to show
(`engine/own_dialogs.rs`); a focus in Okur's windows titled as one is in a dialog box, its
default the spec's where the platform names none.

**Linux, more:**
- An alert inside a window (GTK's info bar) is part of it, not a window (b375bbfa; the fourth
  review, 9).
- A dialog box's window is read within its containers and buttons, not its lists' items
  (a50440db; the fifth review, 4).
- The lab: a pending scenario can be run as research, its run kept apart in
  `D:\okur-linux\research` and never imported (f8d5cf36); a research scenario names the
  settings it needs (`okur_set`, 1779a4c5).

## The research run of 1205 (gtk4, at 298f0c7c)

Insert+F7, then S 30 ms after it, before the Links List is up. From Okur's log in the lab:
- S was held 31 ms after F7, and pressed again in the dialog as okur-ui said it was shown,
  604 ms after F7. It came back through the grab on the dialog's window and went to it; the
  list chose "Settings" (okur-ui's `dialog_changed`, selected 1).
- But GTK put the list's focus at its first layout, after the S, on its first row: AT-SPI's
  focus event was "Current Page Overview" (its snapshot already neither focused nor selected,
  so not said) and none came for "Settings". Down then went from Settings to the third row,
  and nothing was said before it.
- c47a5731 says the dialog is shown a frame later. The next job runs 1205 again.

## Figures

From the commands named:
- **Linux live, full5's 512 runs at 240fb118** (8c6ddc8d). `okur-lab live-baseline linux`:
  "6684/7935 steps match JAWS exactly; 166 accepted divergences not counted (512 runs, 0 could
  not be compared)". The last handoff had 6101/6511 over 417 runs.
- **Their replay with today's code.** `okur-lab live-replay linux`: "8071/8101 steps say what
  the run said; replayed, 6705/7935 match JAWS exactly (512 runs replayed, 0 without a trace, 0
  could not be replayed)".
- **Seven steps allowed to regress** against the earlier runs:
  - 595 and 605's s16 to s18: the run predates f5c9f29d (an arrowed field's caret at its start).
  - 728/s3: two live regions changed together, which JAWS's own trials say in both orders.
- **Windows parity unchanged.** `okur-lab parity`: "12686/13284 steps match exactly; 388
  accepted divergences not counted", after each code commit of this stretch.
- **The lab's check.** "LINUX CHECK PASS" at 298f0c7c (gtk4), with okur-ui's DialogShown and
  the new Linux key tests.

## Kept, with reasons

- The second review's 2: a key in the hook just as the held keys go goes before them, a race
  of a few milliseconds.
- The first review's 10: the replays hand pressed keys back only after key steps, and the
  mirror reports a held key to the dialog too; no recorded run holds a key.
- 219 (WhatsApp's patterns) on Windows: the replay maps a focused message to no node (another
  Chrome session's ids, and the message's name set on focus); not an engine fault as far as the
  replay can tell.
- Screen Layout and Smart Navigation (986, 987, 991, 992, 984): off by default in JAWS; they
  need where each element is on the screen, which the snapshots do not keep.

## Lab state

`gtk4` runs at 298f0c7c: the check (passed), the Chrome menu SelectChild probe (it found no menu:
its search needs the unnamed menu window, as Okur's driver reads it), 1205, then the 42
targeted scenarios (Okur's dialogs, forms, live regions, key echo).

## Next

1. When `gtk4` ends: fetch its runs, privacy-check them, write both baselines, commit apart from
   code; read 222 to 230's dialog runs for the list's first focus (the fifth review's 8).
2. Then a job at today's HEAD: the check (okur-ui's frame wait builds there first), 1205 again,
   1206 (the laptop layout's reading keys in a text area), 595, 605 and 728.
3. Overnight: ARIA-AT on Linux (`D:\okur-linux\aria-at-ids.txt`, 1154 scenarios).
4. The Chrome menu probe's search; Alt+F's first item.
