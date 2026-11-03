# Plan: Windows' own UI, round 2, then the crate structure (2026-11-02)

Follows [`../handoff/2026-11-02-windows-settings-and-programs.md`](../handoff/2026-11-02-windows-settings-and-programs.md).
The owner's order (2026-11-02): Windows' own UI, starting from "Add or remove programs" and
continuing with the rest; and, before any cross-platform work, a refactoring of the crate
structure ([`../references/crate-structure-proposal.md`](../references/crate-structure-proposal.md)).
Round 43's items ([`2026-11-02-everyday-gaps-round-43.md`](2026-11-02-everyday-gaps-round-43.md):
the Store install on old-pc with the owner, the review's open items) stay open after these.

**Status (2026-11-03, later,
[`../handoff/2026-11-03-windows-ui-round-2-recordings.md`](../handoff/2026-11-03-windows-ui-round-2-recordings.md)):**
old-pc back; §1's items 2 (but for the empty list at the window's opening), 3 and 4 done and
recorded; §2's Settings pages (home, Display, Sound) and the Run dialog recorded; Settings' app
list made fast. Next: 3159/s9 (Say Window's editable combo box; its structure done, JAWS's
lowercase type words there not), §1's item 5, then §2's rest: the Start menu (side by side
blocked while a hidden JAWS window holds the foreground: record it as a scenario from the
editor fixture with a Win step), the elevated tools, Task Manager, Device Manager and Services
(the harness has to deliver their keys first: verification-loop.md §8), File Explorer's
ribbon.

**Status (2026-11-03,
[`../handoff/2026-11-03-crate-structure-steps-0-3.md`](../handoff/2026-11-03-crate-structure-steps-0-3.md)):**
§3's steps 0 to 3 are built. Of §1, items 2 and 3 are built in the tracer and wait for 3156 to
be recorded again; of §2, Settings' home (3157) and Display (3158) are written and wait for
old-pc, which went offline. Next, when it answers: deploy, record 3157, 3158 and 3156, import,
then §1's items 4 and 5 and the rest of §2.

Rules as before: each behavior observed on old-pc (`side-by-side.ps1` to find it, a scenario to
record it), then built; a figure is copied from the tool's output; old-pc always has a round
queued; the dev box is shared (cargo on 4 cores, one heavy job at a time); traces are checked
for the lab machine's own data before they are committed (its installed programs, its feeds).

## 1. Round 1's open items

1. Windows 11's Installed apps, the owner's own screen, live on the dev box with the owner's
   permission: the Store app fix, the "More options" button and its menu (Modify, Uninstall).
2. A list view with the focus itself and a focused item: the tracer reads the focused item
   (MSAA `accFocus`), and the item is said ("Not Selected 7-Zip ...", 3156/s1); record again.
3. Focus events that follow one another within tens of milliseconds (3156/s6): said once, the
   last, live and in replay alike, without delaying a single focus.
4. Settings' Insert+T: the category above its navigation list ("Apps", 3155/s20).
5. The positions JAWS gives an opened app's link and button (3155/s13, s14).

## 2. More of Windows' own UI

Each found with `side-by-side.ps1` first (JAWS and Okur on the same keys), then recorded:
1. Settings beyond Apps: its home (the categories), a page with toggles and combo boxes
   (System > Display, Sound), its search box's suggestions on Windows 10.
2. The Start menu and its search; the Action Center and notifications; Quick Settings.
3. Task Manager (its processes list and tabs), the Run dialog's history.
4. Control Panel's home and its other pages; Device Manager and Services (MMC tree and list).
5. File Explorer's ribbon and its details pane, beyond the lists already covered.

## 3. The crate structure

Done 2026-11-03 for steps 0 to 3 (the owner took every default answer). The owner answers the proposal's six questions (§12). Then its steps 0 to 3 (baselines and the
layer check in report mode, lab code into `okur-lab`, the rename to `okur-windows`, the pure
converters into `okur-windows-model`), each proved by an unchanged `parity-snapshot.sh` output
and test list; steps 4 to 6 as their own stretch before macOS, step 5 as the SDK's M2.
