# Handoff 2026-11-08: Linux, the desktop's controls on the GTK replica

After [`2026-11-07-excel-dialogs-and-old-pc-offline.md`](2026-11-07-excel-dialogs-and-old-pc-offline.md).
Plan: [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md) §2.4 (invariants
for GTK controls: a check box, a combo box, a dialog said as on Windows). The owner asked why the
Linux work had stalled behind Excel (2026-11-07) and said to switch to it; this stretch is that.

## What was done

**The rerun of the web scenarios was broken, and its runs were not kept.** 14671664 read the
runner's `--no-speech 3600` as no number of seconds, so Okur stopped after its 20-second default
and the 80-scenario rerun (`web2`) heard nothing past its start: 391 silent steps, each marked a
success. Fixed (1c314d12): the seconds are the first number that is no option's value, and the
runner fails a step, and its trial, when Okur has exited, so live-baseline does not compare it.
The fetched runs were discarded; the full rerun (`web3`, 80 scenarios) runs now (below).

**SR Desktop Fixture's GTK replica speaks as JAWS does on Windows**, through these changes, each
found by a live run in the lab:
- *Native focus with what surrounds it* (6f4d84c2, earlier) never read an ancestor: `get_parent`
  looks an element up in the provider's handle cache, where an element an event carries never
  is. xa11y-linux's `atspi_parent` finds it by the object's bus name and path (a307de9e).
- *Roles named by number, as libatspi names them* (a7d9b962, 8d062ceb): GTK 4's own GetRoleName
  answers ARIA's words ("radio", "row", "tree grid", "group", "generic"); libatspi names a role
  by its number. `get_role_name` now does the same, from a table of the 131 roles listed from the
  lab's libatspi 2.60.4, checked by a test against the AtspiRole enum xa11y's tests carry (role
  43 is "button" in 2.60, "push button" before; both map to a button).
- *A toolkit's own tree* (96d3ce9f): the engine told WinForms' tree (its open and closed said as
  states) from File Explorer's by matching "WindowsForms" in general code; `FocusEvent::classic_tree`
  now says it, set by Windows' `add_class_facts` and by okur-linux-model for GTK's tree.
  `parity-snapshot.sh` before and after: identical (IA2 12686/13284, UIA 11375/13170).
- *The keyboard hook's grabs moved GTK's focus with every key* (eb0f0ced). The hook grabbed on the
  root's children, which under a reparenting window manager (fluxbox in the lab; GNOME's, Xfce's
  and KDE's on X11 alike) are its frames; X reports a grab on an ancestor of the focus window as
  the focus leaving for it, and GDK deactivated and activated the window around every key (the
  event trace of 304 showed the pair for each). GTK repeated the focus after each, which hid
  state changes from the engine, and the drop-down's open list could not keep its item. The hook
  now grabs on the window holding WM_STATE and lets a frame's grabs go as the window manager
  reparents a window into it: one `WindowDeactivated` in a whole run of 304 afterwards.
  The runner activates a replica's window before its keys (5c5eb33b): without the grabs' focus
  noise, fluxbox's focus for a new window came too late in two runs of 301 out of three.
- *Repeated focus and changes within a control* (6702db71, eca6386d, c6625b41, a7d9b962,
  bc1f12f8, f715f99b): a focused control's state change is read as its focus is (named alike); a
  focus event for the object already focused is passed over when it reads as before (role and
  name); the same window activated again keeps its focus; a label renamed or an object selected
  without the focus has the focused control read again, said as a focus when it reads as
  another control (GTK's notebook's next tab) and as a value change when only its value changed
  (GTK's drop-down's item).
- *The replica* takes Right and Left on its tree (8e724639) and Down, Up and Alt+Down on its
  drop-down (15138163), as the WinForms fixture's controls do.

`okur-linux-model`'s tests now check what the engine says for each replica control against
JAWS's speech in 301's evidence (96d3ce9f): `cargo test -p okur-linux-model --lib` passes 24, 1
ignored (the drop-down's place, a known gap).

**The lab's sync failed silently three times** (6d3d525a): it pushed the thirty lab scripts one
by one before checking out, and outlasted its caller's timeout; the lab ran the commit before.
It now copies the scripts from the lab's checkout in the same call (12 s a sync) and fails,
saying so, unless the lab is at the dev box's HEAD.

## Figures

`okur-lab live-baseline linux` at 5a136b47 (runs at f715f99b for 301, 303, 304; the web runs
from 2026-11-06):

    Okur live on linux: 411/424 steps match JAWS exactly; 0 accepted divergences not counted (31 runs, 0 could not be compared)

The 13 steps that differ, all in the desktop scenarios:
- 301/s1: "SR Desktop Fixture" before Notes: Okur hears the replica's first focus only with the
  first key. JAWS says the window and its focus as the fixture starts.
- 301/s5, 304/s8-s10: the drop-down's place ("2 of 3"): GTK's closed drop-down exposes only its
  chosen item (known-gap test `a_drop_down_says_its_place`).
- 303/s5-s6: JAWS said "Pictures" and "Documents closed" without the place in this recording
  (another trial's variant has it).
- 303/s7-s8, 304/s16: the tree's levels and an opened item's count ("level 1 | Reports | 1 of
  2", "2 items"): GTK's tree rows give their place in the flat list ("2 of 4") and a `level`
  attribute that is not the depth (Pictures, at the top, says 2).
- 304/s11-s13: the drop-down's open list starts at its first item (Turkey), not the chosen one,
  and its Down and Enter did not choose Brazil.

The Windows gate passed on every commit (pre-commit hook); the Linux crates' gate in the lab:
`LINUX CHECK PASS` at f715f99b.

## Next

1. The web rerun `web3` (80 scenarios, at f715f99b): `bash skills/linux-lab/scripts/job.sh wait
   web3`, then `fetch-runs.sh $(cat <the 80 ids>)`, `okur-lab live-baseline linux`; it is the
   first web run with the hook's new grabs, so a regression there is the hook's.
2. 301/s1: on a window activated, read its focus and say the window with it, as JAWS does when
   a window comes forward.
3. The drop-down's open list (304/s11-s13), now that the window stays active through each key.
4. The tree's depth: what GTK's TreeExpander exposes (its `level` attribute, the list item's
   attributes) in `probe-gtk-tree.sh`, before a known-gap test.
5. Then the plan's §2.3 (Linux snapshots of the HTML fixtures) and more web scenarios.
