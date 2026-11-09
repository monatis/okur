# Handoff 2026-11-10 (night): old-pc offline; Linux replay fixes from the traces, Run Okur Manager

After [`2026-11-09-okur-ui-wizards-trees-and-own-dialogs.md`](2026-11-09-okur-ui-wizards-trees-and-own-dialogs.md).

**Plan:** [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md).

## Lab state: old-pc offline

From 22:32 on 2026-11-09, every `opendesk exec --peer old-pc` gave `rendezvous error:
target_offline` (checked again each hour or so through the night). `night2` was in its ARIA-AT
part (1154 scenarios, at ec1e5cac); its runs since the gap runs are on old-pc, not fetched. The
background wait (`job.sh wait night2 4`) printed nothing.

So this stretch worked from the traces already imported: the Linux replay
(`okur-lab live-replay linux --jaws`) and the Linux-only gaps (steps Windows' parity matches,
Linux's replay does not; 586 steps when this began, from the two baselines).

## What was done

**A wizard of Okur's turns its page as Windows' does** (48c6cdec):
- okur-linux-model tells the engine of the window in front renamed (`WindowRenamed`), as
  Windows' converter does. GTK 4 tells no window's activation over AT-SPI (743's trace has
  none for okur-ui), so a dialog or window whose state is active counts too.
- An own wizard's focus carries its page (`wizard_page`), the title the wizard takes from it:
  "Choose an element dialog | Which of the following elements ..." [obs, 743/s8], the
  question before and after the title where the page has one (743/s7).
- A read again of the focused control after a rename in it says nothing when the read failed
  or the object lost the focus: the wizard's old list row, read as its page went, said "To
  move to an item press the Arrow keys." alone (743/s9).
- `okur-lab live-replay` logs each record replayed and what the engine said to it
  (`RUST_LOG=okur_corpus=debug`): how 743/s9's stray hint was found.

**The PlaceMarker List on Linux** (49b9ef67):
- the dialog it opened over itself has it named first, "PlaceMarker List dialog | Add
  PlaceMarker dialog | Name: Edit" [obs, 702/s5, 704/s4]. Only there: a first try named every
  dialog still open before the new one, and 1157/s5 lost its match (JAWS names no Dictionary
  Manager before its Add Dictionary Definition). The engine knows the list's child
  (`placemarker_dialog_owner`);
- a column view with no rows, its scrolled window focused (GTK's), is the list view itself:
  "PlaceMarker List view | 0 items | To move to items use the Arrow keys." [obs, 704/s3];
- a column view's row focused and not selected says "Not Selected" [obs, 704/s11].

**okur-ui on Linux: the arrows go round a radio group** (9dc60a01), choosing the radio they
come to, as Win32's group does; GTK's left the group (763/s5: Down on Skim Reading's last
rule type went on to Rules History). Not built yet: GTK does not build on the dev box.

**Run Okur Manager (Insert+F2)** (5ea0ba33), JAWS's Run JAWS Manager, which Okur lacked: the
Linux runs showed it (688/s1 silent), Windows' parity never could (it replays JAWS's own
dialog).
- "Run Okur Manager dialog | List1 List view | Commands Search | 1 of 21": JAWS's 21 managers
  in its order, OK (the default) and Cancel [obs, 688/s1-s5].
- OK runs the manager from where the dialog was opened: Commands Search, Custom Label, Custom
  Label Delete All, Dictionary Manager, Flexible Web, Notification History, Quick Settings,
  Settings Center, the Skim Reading Tool. The other twelve say "Okur has no Keyboard Manager
  yet.", as Virtual HTML Features says of the lists Okur has not built.
- A manager that opens a dialog is said as its dialog takes the focus; the return is quiet
  only when none opened.

**Custom Label Delete All** (404de1ce): "Delete all Custom Labels? dialog | This will remove
all custom labels for the current document or web page | OK Button", OK and Cancel [obs,
688/s6; the trace's dialog lines]. OK takes the site's labels away and reads the page with its
own names, quietly [obs, 688/s7, s8].

## Figures

From the commands named, each after the commit given:
- `okur-lab live-replay linux`:
  - after 48c6cdec: "7820/8101 steps say what the run said; replayed, 7110/7935 match JAWS
    exactly" (7062 before);
  - after 49b9ef67: "7781/8101 steps say what the run said; replayed, 7146/7935 match JAWS
    exactly";
  - after 5ea0ba33 and 404de1ce: "7767/8101 steps say what the run said; replayed, 7145/7935
    match JAWS exactly". 688/s14 was let go (917b37e2): 688's run predates Run Okur Manager,
    so its trace has none of the dialog's events, and replayed, the keys after Insert+F2 are
    the dialog's.
- `okur-lab parity`: "12687/13284 steps match exactly; 388 accepted divergences not counted"
  (12686 before; 688/s26, Enter on Delete All's OK, now quiet as JAWS's).
- `okur-lab parity --api atspi`: "12329/13284 steps match exactly" (12328 before).

## Found, not fixed (for the lab)

- **Misspelled words on Linux (567, 284):** Okur reads AT-SPI's `invalid: spelling` runs, but
  Chrome in the lab marked none. Each run has a fresh Chrome profile; whether its spelling
  dictionary is downloaded in time, or at all, in the WSL lab is the first thing to look at
  (`<profile>/Dictionaries`).
- **748 and the Flexible Web runs that build on a saved rule:** each Linux run has fresh
  settings, so "View or change where rules apply" is off (no rule) and Alt+V does nothing.
  A run that needs a rule from an earlier scenario cannot pass alone on Linux.
- **Skim Reading (816, 760, 911) and the lists (224, 765, 926):** the dialog shown and no
  focus in it, or no keyboard; the keys went to the page. db1e6423's log lines say why.
- **Okur's Win32 dialogs:** okur-ui's Win32 dialog is owned by the Okur dialog in front as it
  opens, so on Windows the Dictionary Manager would be named before its Add Dictionary
  Definition, where JAWS names none (1157/s5). Parity cannot see it (it replays JAWS's
  dialogs). The owner should come from the engine, as on Linux.

## Next

1. **When old-pc is back:**
   - fetch night2's ARIA-AT runs, privacy-check them, a baseline of their own, committed apart
     from code;
   - `check-linux` at HEAD: the first build of af369c04, ccd734c3, db1e6423, 61108425, 8fe053c8
     and 9dc60a01 (GTK); `main` stays at 3ed18eb0 until it passes;
   - then the dialog and wizard runs again (224, 765, 808, 926, 760, 911, 816, 742, 743, 704,
     705, 748, 751, 241, 256, 274, 548, 549, 554, 301, 303, 304, 809, 567, 688, 763), and read
     okur-ui's "dialog {id} shown: active ..., focus ..." lines;
   - the spelling dictionary of the lab's Chrome.
2. The Win32 dialogs' owner from the engine (above).
3. Custom Label Delete (one label): what JAWS does, from its help, then on old-pc.
4. Run Okur Manager's other managers, as each is built.
