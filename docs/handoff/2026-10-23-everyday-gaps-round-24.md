# Handoff: everyday-use gaps, round 24 (2026-10-23)

Follows [`2026-10-23-everyday-gaps-round-23.md`](2026-10-23-everyday-gaps-round-23.md) and covers
the commits after it (`67167a9` to `35a7714`, and this handoff's), under the plan
[`docs/plans/2026-10-23-everyday-gaps-round-23.md`](../plans/2026-10-23-everyday-gaps-round-23.md).
The next plan is
[`docs/plans/2026-10-23-everyday-gaps-round-24.md`](../plans/2026-10-23-everyday-gaps-round-24.md).

## The owner's actions and requests

- **Installed, elevated, on old-pc**: the UI Access notification probe
  (`install-notify-probe.ps1`) and Okur itself, signed with the development certificate
  (`install-okur.ps1`, which runs `okur-setup.exe install --no-start --no-start-after-sign-in`).
  The lab was paused for it (`pause-lab.ps1`: the batch and its waiters stopped, JAWS's own
  settings restored) and resumed after (`resume-lab.ps1`).
- **Closed the Explorer window** on `Dropbox\migration` that had kept fourteen desktop
  scenarios out of the corpus; round 262 recorded them again.
- **JAWS's voice profile** read Rate=20 after the owner's session (a live rate change JAWS does
  not save as 90); `fix-rate.ps1` set it back to 90 with JAWS stopped, writing the profile
  without a byte-order mark (its first version added one; fixed the same minute).

## Decided without the owner (reversible)

- **914, 915 (Typing Interrupt) and 917 (Reading Interrupt off) leave pending**: no trial
  settled. Their observations are in the customization map.
- **Visited links that Chrome on old-pc now remembers** (907, 908, 875, 876) are recorded
  differences, not accepted ones: the replay's page snapshot has no visited state.
- **T1, the engine in the keyboard hook, is put back**: no everyday page key takes 5 ms on the
  review's 25,032-node page, and live keys took 1.0 ms at p99 (below).

## What was done

### Verbosity (rounds 263, 263b, 262; recorded in `docs/references/messages-and-verbosity.md`)

- **Short messages above Beginner**: the quick keys' nothing-found messages, "No PlaceMarkers",
  "No clickable elements" (every "There are no ... on this page." has its `-short`, reached
  through the new `StaticMessage::short`); Insert+F5's "no form fields"; the table layer's
  "Table"; Route PC to Virtual Cursor's "PC to Virtual"; a list view's column past the last,
  "3 out of range" alone.
- **Left out at Advanced**: a table's edge ("end of row"), Route PC to Virtual Cursor, and a
  toolbar entered (said as a group name, `CONTROL_GROUP_NAME`).
- **Corpus**: 865-912 (24 web scenarios at both levels), 843-846, 849-854 and 859-862 (the
  desktop at both levels), 913 and 916; round 267 (918-953) records eighteen more now.

### Reading settings

- **Indentation** (`62b3ddb`, 729): the setting `indentation`, read from JAWS's
  `[Options] Indentation`; "Left Margin" before a page line at the margin on Say Line, its
  double press, and as a Say All starts.
- **Found in JAWS's shipped `DEFAULT.JCF`** (read-only, old-pc): `TypingInterrupt`,
  `ReadingInterrupt`, `KeyRepeat` (shipped 1, where the help gives it cleared) and `ScreenEcho`
  (0, 1, 2). Recorded (913, 916, 917): Insert+S cycles "All", "None", "Highlighted"; Insert+X
  alternates "Smart Navigation: Controls and Tables" and "Smart Navigation: Off"; with Reading
  Interrupt off, Insert+T during a Say All is answered in the middle of the reading and the Say
  All goes on. None of these is built yet.

### The performance review

- **P3, a key that changes nothing** (`16d2ede`): two `WM_NULL`s sent in turn to the focus
  window tell that the application has taken the key, and two reads after that, the same, end
  the wait. Live run 266 (`okur-lab latency`, text context read, 26): p50 / p90 / p99 16.0 /
  266.0 / 271.0 ms before, 20.0 / 32.0 / 35.0 after; speech the same at every step of 504,
  660 and 825.
- **P6** (`a8861c5`): the actions worker's UIA client gives up after 2 s connecting and 5 s
  answering. **P14, C4**: checked live (run 265: speech the same at every step of 504, 514,
  660, 825).
- **Measured, not changed**: `big_page_keys` now presses 90 everyday page keys and the first
  key after a refresh on 25,032 nodes: none takes 5 ms (Insert+F5 4.5 ms the slowest), the key
  after a refresh 0.5 ms. P5 is kept (the page must reach the engine before its first focus);
  C10, T13, T14 kept, with why. Details: `docs/backlog/performance-review-2026-10-23.md`.

### Starting at sign-in (M3)

- The signed install runs with UI Access: `okur-sr.exe --doctor-exit` on old-pc reported
  `integrity High (0x3000)`, `TokenUIAccess 1`, `elevated no (limited token)`, `reads elevated
  windows yes`, `in Program Files yes`, `signature valid`. Left: `okur-ui`'s lowered token, an
  elevated Notepad read by the installed Okur, sign-out, uninstall
  (`docs/references/startup-and-secure-screens-proposal.md`).

### Applications' own UI Automation notifications (820)

- Experiments 8-10 (`docs/references/notifications-research.md`): the probe with UI Access
  hears none of a WinForms window's notifications, nor does the plain one started after the
  window shows; WinForms does raise them (`RaiseAutomationNotification` returned True).
  Experiment 11, a registration on the control's own provider reached through its IAccessible
  (`29401b7`), runs after round 267.

### Harness

- `skills/jaws-reference/scripts/preview-round.sh`: replays the scenarios a round has recorded
  so far from a scratch corpus, before the import; this stretch's findings came from it while
  the rounds were still recording.
- `pause-lab.ps1`, `resume-lab.ps1`, `fix-rate.ps1` in `sr-harness\experiments`.

## Figures

Each is the named command's output at `35a7714`:

- `okur-lab parity`: "JAWS parity (Ia2): 7546/7815 steps match exactly; 187 accepted
  divergences not counted (0 scenarios could not run)" (6854/7096 at `67167a9`, before 82
  scenarios were added).
- `cargo test --workspace`: 1379 passed, 0 failed, 199 ignored.

## old-pc

- Round 267 (918-953) is recording; experiment 11 runs after it
  (`batches\2026-10-23-exp11.txt`).
- `sr-harness\bin\okur.exe` is the build at `16d2ede` (live run 266).
- `C:\Program Files\Okur` holds the signed install; it starts neither now nor at sign-in.
