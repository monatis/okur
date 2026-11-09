# Handoff 2026-11-09 (evening): okur-ui's wizards and trees on Linux, Okur's own dialogs told from their specs

After [`2026-11-09-keys-held-for-dialogs-and-full5.md`](2026-11-09-keys-held-for-dialogs-and-full5.md).

**Plan:** [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md); its status
section is current (f6872a11).

## What was done

**okur-ui on Linux draws what full5 found missing.** okur-ui logs each message it cannot draw;
counted over full5's 512 logs, the wizard came first (18 scenarios), then the Virtual Viewer
(14, but Okur keeps its keys and the window only shows it), then trees (6).
- **Wizards** (Flexible Web, the Startup Wizard; 6ad3b989, df7462e5):
  - a window titled by its page, the page's controls over Back, Next or Finish, and Cancel,
    labelled from the catalog (`wizard-back` and the others);
  - Back, Next and Finish are told to Okur, which turns the page (WizardPage) and sets the
    buttons (WizardButtons);
  - the new page is built beside the old one and focused before the old one goes: removed with
    the focus in it, GTK moved the focus and it was said (716/s7);
  - Enter on a check box, a radio button, a drop-down or a slider presses the default button,
    as in a Windows dialog: Enter on the start page's task did nothing (716/s29).
- **Trees** (the Regions List, Settings Center's; ad9029d7): a list view of GTK's tree list
  model, each row its expander. Items are told by index, as Windows' tree view does (Selected,
  ItemOpened); Right, Left and Space work as there.
- **A dialog's first focus** (7c31a514, c47a5731): placed once the window is active and a frame
  of it is laid out, and only then is the dialog said shown. GTK gave no focus for a row
  focused before (gtk4's 223, 586, 682, Insert+F7 silent); gtk7's 222, 223 and 586 say it.
- **Plain names** for check boxes and radio buttons: GTK's had the mnemonic's "_".

**Okur's own dialogs told from their specs** (5798bd17, 1b3c9e1b; `engine/own_dialogs.rs`).
GTK gives less than Windows does. Where the platform read no dialog box
(`FocusEvent::dialog_box_read`, which Windows' converter sets for every focus), a focus in one
of Okur's dialogs gets from the spec:
- a control's group box: "Use Rule Type:", "Color or Attribute Rules" (812);
- a drop-down's place: "1 of 8" (812/s5);
- a tree row's item: its depth, its place among its siblings, its parent, whether it is open,
  and its count (241: "level 1 | Primary Navigation | 1 of 1 | Banner").

Windows' parity without the mark lost 39 steps, JAWS's own WPF dialogs among them; with it,
none.

**okur-ui's application name on Linux** (5798bd17). Every trace had none for okur-ui, so no
engine rule knew its windows as Okur's: the fill above, the keys held, the settings' place.
xa11y-linux kept an empty name, asked before the application's root was up. An empty name is
now asked again, and okur-ui also sets its application name. The next runs show it.

**The lab:**
- `gtk4` (42 runs, 4d9cb680) and `gtk7` (30 runs, 97aa07c0) imported.
- The Chrome menu probe now finds the menu's unnamed window from its activation (7c63aa79).
  Its answer: AT-SPI's SelectChild does not highlight Chrome's first item ("CHROME MENU SELECT
  DOES NOT WORK"), so Alt+F's first item needs another way.

## Figures

From the commands named:
- **Linux live** (97aa07c0). `okur-lab live-baseline linux`: "6806/7935 steps match JAWS
  exactly; 166 accepted divergences not counted (512 runs, 0 could not be compared)". full5's
  import had 6684/7935.
- **Their replay with today's code.** `okur-lab live-replay linux`: "8074/8101 steps say what
  the run said; replayed, 6824/7935 match JAWS exactly".
- **Windows parity unchanged.** `okur-lab parity`: "12686/13284 steps match exactly; 388
  accepted divergences not counted", after each code commit.
- **Allowed regressions, with reasons in the commits:**
  - gtk4's first focus (223, 586, 682);
  - gtk7's wizard Enter (716, 720, 743, 750), fixed in df7462e5;
  - 567/s11, s12: Up after its text step, now run, reads the line before the field. To look
    into.

## Kept, with reasons

- The second review's 2 (a key in the hook as the held keys go), the first review's 10 (the
  replays hand pressed keys back only after key steps).
- The spec's group boxes are wider than JAWS's in at least one dialog. The dictionary
  dialog's "Case Sensitive" is in "Applies to" in Okur's spec, in no group box in JAWS's
  (1156/s22). Okur's Win32 dialog likely says it too, unseen by parity, which replays JAWS's
  dialog.
- The tree's first focus (241/s1) was still silent in gtk7, before the tree fill and the
  application name; night2 shows it.

## Lab state

`night2` runs at ec1e5cac (night1b, at df7462e5, was stopped for the tree fill, 1b3c9e1b, and the focused field read again as it is typed in, ec1e5cac: 567's Insert+Up said "t" for "teh quick brown fxo", each Ctrl+Left the word before):
1. the check;
2. the research runs 1205 and 1206;
3. the 132 scenarios with Linux-only gaps (`D:\okur-linux\gaps1-ids.txt`: Windows' parity
   matches the step, Linux's replay does not), about 2.5 hours;
4. ARIA-AT's 1154 (`aria-at-ids.txt`), overnight.

## After night2 began (not yet built in the lab)

- Okur's wizards kept as dialogs, Next or Finish their default (b118ef81): Insert+E in a wizard on Linux.
- The sixth code review's fixes (af369c04): a tree's arrows told and its items changed in place; a kept item's children opened with it; a wizard page's first focus after its layout; a drop-down's Enter and its open list's Escape; unasked field reads (`FieldText`) never a key's answer; xa11y-linux's application looked up after the kinds' filter; a drop-down's replaced choices kept; a radio group's first focus on its checked option.
- The buffer's depth test steadier on the shared machine (17da7d0b).
- A window's focus looked for as it comes forward (ccd734c3): GTK reports none for the control focused as its window comes (548/s1, Insert+Tab silent); a named window, not Chromium's nor okur-ui's, is looked through 150 ms after it came, unless a focus came meanwhile.

The next job's check builds these first (okur-ui's GTK code does not build on the dev box); `main` stays at 3ed18eb0 until it passes.

## night2's gap runs, imported (f691a3c3), and what they showed

- **Figures.** `okur-lab live-baseline linux`: "6914/7935 steps match JAWS exactly; 166
  accepted divergences not counted (512 runs, 0 could not be compared)" (gtk7's: 6806). The
  Linux-only gaps (steps Windows' parity matches, Linux's replay does not) went from 944 steps
  to 716.
- **okur-ui is "okur-ui" on the bus at last**: 1205's trace names it, and the held S and Down
  went to "Settings" and on to the third row in order.
- **The dialogs' first focus is still a race in GTK.** 223's Links List had its first row's
  focus 3 ms after the dialog was shown; 224's Heading List, 765's Word List and 926's JAWS
  Find had none until the first Tab. db1e6423 has okur-ui check its window's focus a frame later,
  place it again where none is, and log the window's state and focus.
- **Skim Reading (760, 911):** the dialog shown, then the first Tab went to Chrome, which came
  back to the front: the dialog seems never to have had the keyboard. To look into in the lab.
- **Fixed from them, offline:**
  - a group box said twice, and a radio group's box said on each arrow as Windows' is
    (61108425);
  - "Color or Attribute Rules:" with its colon, which Okur's Win32 dialog lacked too, unseen by
    parity (61108425);
  - a wizard page's question as its dialog's text (8fe053c8);
  - check lists drawn on Linux, a row told its check (8fe053c8).
- **The replay with all of it** (`okur-lab live-replay linux` after 8fe053c8): "7822/8101 steps
  say what the run said; replayed, 7062/7935 match JAWS exactly". The runs are older than the
  code, so "say what the run said" fell. ffb74f47's message quoted wrong figures, written
  before the output; 8fe053c8 corrects them.
- **Left:**
  - 812's Rules History count (old-pc's JAWS kept 8 rules) and colors (Okur has 509, JAWS 514);
  - 809/s12 (Space on another check box than JAWS's);
  - 567's Up after typing, Chrome's address bar after Ctrl+T (no focus event on Linux), the
    closed tab's late load;
  - the desktop forms replica's radio group, which Down leaves in GTK.

## Next

1. **When `night2`'s gap runs end** (fetch mid-job; the snapshot skips the run in progress):
   - import them;
   - read 1205 (the frame wait) and 1206 (CapsLock+O in a text area);
   - read the dialogs' first focus and the application name in their traces.
2. **When ARIA-AT ends:** import its runs, a baseline of their own.
3. **The next GTK pieces:**
   - the check list (Settings Center's);
   - DialogMenu (a context menu);
   - DialogPane and DialogSwap (Settings Center's pane).
4. 567's Up after typed text; Alt+F's first item.
