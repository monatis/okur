# Handoff: everyday-use gaps, round 22 (2026-10-22)

Follows [`2026-10-22-everyday-gaps-round-21.md`](2026-10-22-everyday-gaps-round-21.md) and covers
the commits after it (`a439640` to `9d7e0a4`, and this handoff's), under the plan
[`docs/plans/2026-10-22-everyday-gaps-round-21.md`](../plans/2026-10-22-everyday-gaps-round-21.md).
The next plan is
[`docs/plans/2026-10-22-everyday-gaps-round-22.md`](../plans/2026-10-22-everyday-gaps-round-22.md).

## The owner's answers and requests

- **The development certificate is trusted on old-pc and `okur-lab-keys.exe` installed** (the
  owner, elevated, with `sr-harness\signing\trust-and-install-lab-keys.ps1`, which checks the
  certificate's thumbprint `7E6D5DC002749832FD07EDAB82A148467BF7DEAF` first). Checked after:
  the root is in `Cert:\LocalMachine\Root` and the key sender's signature is Valid.
- **JAWS was slowed for the owner on old-pc** (two `DecreaseVoiceRateTemporary` steps), and
  sped up again by them. JAWS had saved Rate=18 into its voice profile meanwhile; it was set
  back to 90 with JAWS stopped and started again (elevated, by encoded command). The probe
  passed after.

## Decided without the owner (reversible)

- **520/s3 is an accepted divergence**: its recording holds Windows' Win+X menu, which the
  harness's injected Windows+Insert+X opened, not JAWS's SayClipboard.
- **561 is retired for 825**: its editor fixture runs in powershell.exe, where JAWS's clipboard
  scripts answered otherwise ("Copied selection to clipboard" for SayClipboard).
- **A one-digit raised number by word** (824/s2, s5, s6) joins the one-character divergence.
- **Manage Rules' "Rules _Rules"** is an accepted phrase JAWS also says.

## What was done

### Features (each checked with `okur-lab parity` against old-pc's evidence)

- **The clipboard keys** (`8c01f9a`, 825): Windows+Insert+X reads the clipboard a line each;
  Insert+Space, C shows it in the viewer "Clipboard Text" (its title the first line, JAWS's
  closing line last; Escape returns to the application's window, announced again);
  Insert+Windows+C appends the selection; Ctrl+C or Ctrl+X after that asks "Overwrite
  Clipboard", and Yes copies or cuts. 825's spoken layer: 20/20.
- **Characters with their marks** (`d4beaf5`, 821): "Deletion t", "end Deletion" at a space.
- **A list box with the focus itself** says "List box" and its hint (`629d819`, 787/s18,
  798/s20); **a row coming back with changed texts** is said again (`522d67c`, 787/s16).
- 822 and 823 (the edits page at Intermediate and Advanced verbosity): the marks unchanged,
  48/48.

### Harness

- `okur-lab probe-notifications` (`b1dea4e`, `b3459e9`): registers for UI Automation
  notifications seven ways and logs the window's process's WinEvents. On old-pc, Calculator's
  reach every registration; a WinForms window's reach none, though JAWS speaks them.
- `run-scenario.ps1`: the `okur-lab-keys` path keeps its backslashes (`0bd91e5`; it had read
  `C:Program FilesOkur Lab...` since it was written); `key_ui_access` keys may go to JAWS's
  ResultsViewer (`8028c7b`).
- The verification loop's pitfalls (`d9abdca`): the harness copies JAWS's history through the
  clipboard and leaves it there (a clipboard scenario's steps read no history); Windows takes
  injected Windows-key chords; desktop fixtures run in powershell.exe.

## Figures

Each is the named command's output at `9d7e0a4`:

- `okur-lab parity`: "JAWS parity (Ia2): 6655/6888 steps match exactly; 185 accepted
  divergences not counted (0 scenarios could not run)". At the previous handoff: 6540/6776.
- `okur-lab parity`, spoken layer: "JAWS spoken parity: 6532/6870 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 465 scenarios, 1401 trials, 1394 successful.
- `cargo test --workspace`: 1360 passed, 0 failed, 199 ignored.

## Learned

- **Injected Windows-key chords are Windows'**: Windows+Insert+X opened Windows' Win+X menu
  from the harness's keys, with UI Access too, Windows key or Insert first. JAWS's scripts for
  such keys are called by name.
- **UI Access keys reach JAWS's windows**: Escape closes View Clipboard Text's window
  (ResultsViewer.exe), Y answers its Overwrite Clipboard question (jfw.exe). A UI Access
  program cannot be started with its output redirected (Windows asks for elevation).
- **The harness writes the clipboard**: each history read copies JAWS's speech history there.
- **JAWS's clipboard keys** (825): SayClipboard reads the text a line each; View Clipboard
  Text's window is a page, its title line first ("Clipboard Text"), then the text and "Press
  Escape to close the clipboard text viewer."; after an append, Ctrl+C asks before
  overwriting.
- **JAWS's main window can come to the front** after JAWS is restarted, which spoiled first
  trials (825/s18, round 256); minimizing it helped.
- **The MCP `opendesk_exec` tool hung twice for 30 minutes** where the command had finished;
  the `opendesk exec` command line with `timeout` did not.

## Recorded meanwhile, and queued

Rounds 253 to 257 are imported (824, 825); 254 to 256 were 561 and 825's attempts. old-pc is
idle.

## Not done, and why

- **UI Automation notifications from a WinForms window** reach no plain client on old-pc; 820
  replays with that gap (s1, s5, s8). Testing a UI Access client needs a probe in a signed
  program under Program Files, which the owner installs.
- Toasts Focus Assist hides; the rest of verbosity; Office (activation); the Okur cursor where
  the screen has no text.
