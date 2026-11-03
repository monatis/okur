# Handoff: Windows' own UI round 2 recorded, the Store's UI Access, a slow list fixed (2026-11-03)

Follows [`2026-11-03-crate-structure-steps-0-3.md`](2026-11-03-crate-structure-steps-0-3.md).
Branch `windows-ui-round-2`, fast-forwarded into `main` (not pushed): `71e20df2` to `c72cac30` (the corpus of 3159 and 3160),
and this handoff.

## What the owner did

old-pc came back (it had rebooted at 05:55). The owner signed in to Partner Center and to the
Microsoft Store on old-pc's Chrome, and installed Okur from the Store (`Monatis.okur`
1.0.0.0). Their session left JAWS's voice at rate 20 (the profile saved 06:15);
`fix-rate.ps1` restored 90 before any round, and the volume was still 1%.

## The Store install answers E1

`okur-sr.exe` from the Store, activated as the Start menu does with `--doctor-exit`:

```
executable                           C:\Program Files\WindowsApps\Monatis.okur_1.0.0.0_x64__rfy6yt361pz56\okur-sr.exe
integrity                            High (0x3000)
TokenUIAccess                        1
reads elevated windows               yes
signature                            not signed
manifest                             level=asInvoker uiAccess=true
```

The Store's signature of the package is enough for UI Access; Okur needs no code-signing
certificate of its own. Recorded in
[`microsoft-store-release-research.md`](../references/microsoft-store-release-research.md) §8
and round 43's plan. Not yet seen: the Store's Okur reading an elevated window (a UAC consent),
its Ease of Access registration (old-pc's AT key is the classic `okur-setup` install's).

## Built

| What | Commit | Proof |
|---|---|---|
| Insert+Tab names the control's group (not a list): "Scale and layout \| Display resolution Combo box" | `fbb84406` | 3155/s1, 3158/s8, 3160/s13 match |
| Insert+T in Settings names the page: "Title is System", "Title is Apps"; the tracer records a Store app's window texts (`app_window_texts`, format 11, read in xa11y); `AppModule::title` | `fbb84406` | 3155/s20, 3157/s13, 3158/s14 match |
| A mouse read over a list's unvisited row stays out of traces | `779a2c09` | a trial of 3156 had held a program's publisher; recorded again, none kept |
| Settings' app list: each focus read in 55 ms, not 1,160 ms (the column header was searched for among all 155 items) | `2b7909ea` | `okur-lab trace --debug-log` before and after; a WinForms list still gives "Name", "Size" (303, research trial) |
| A focus set before its window comes to the front is read when it does (Programs and Features opened cold was silent) | `c6847627` | 3156's first trial now holds the window's focus (it never had in three rounds) |
| okur-ui's standard error into Okur's log (review S12) | `6c2ef062` | live on old-pc: okur-ui started, the Virtual Viewer opened, nothing logged; the lowered start not run live |
| An editable combo box said once, as its edit | `0cadcd16` | 3159/s6 matches; parity unchanged elsewhere (a first version lost 8 steps) |

Recorded and imported: 3155 again, 3156 again, 3157 (Settings' home), 3158 (Display), 3159 (the
Run dialog's Tab order), 3160 (Sound). Per scenario, `okur-lab parity --spoken --only <id>`:

```
3155  JAWS spoken parity: 18/20   (16/20 at the last import)
3156  JAWS spoken parity: 16/17   (15/17)
3157  JAWS spoken parity: 13/13
3158  JAWS spoken parity: 14/14
3159  JAWS spoken parity: 9/10
3160  JAWS spoken parity: 14/14
```

The whole corpus after the last import (`okur-lab parity`, `okur-lab parity --spoken`):

```
JAWS parity (Ia2): 11458/12043 steps match exactly; 365 accepted divergences not counted (0 scenarios could not run)
JAWS spoken parity: 11209/12109 steps match exactly (evidence recorded through the speech recorder voice)
```

## Found side by side, not recorded

- Settings > Time & Language > Language: the same on 9 keys of 9.
- Control Panel (All items): the same on 11 keys, but Okur's speech log has "Intelr Graphics
  Settings" for "Intel® Graphics Settings": the name likely comes through MSAA folded to ANSI.
  Its view lists the lab's installed components (Java, NVIDIA, B&O audio): not to be recorded
  as it is.
- The desktop after the Run dialog closed from Control Panel: Okur added "Program Manager |
  desktop" (319, recorded, matches: another path).

## Open

- 3156/s1: Insert+Tab as Programs and Features opens: the list was empty when its focus was
  read; JAWS reads its first program when asked. Only a read at Insert+Tab can see it.
- 3159/s9: Say Window lists an editable combo box as JAWS does: "Open: | Open: combo box
  collapsed | C: | Open: edit | C: | Open button".
- 3155/s13, s14: the positions JAWS gives an opened app's button and link.
- The Store's Okur: an elevated window read, its Ease of Access registration.
- The plan's §2 rest: the Start menu, Task Manager (it runs elevated: keys need UI Access),
  Device Manager, Services, File Explorer's ribbon.
- Windows 11's Installed apps on the dev box, with the owner's permission.
