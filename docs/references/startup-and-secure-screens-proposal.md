# Starting Okur at sign-in and on secure screens, and quitting it: proposal

**Status:** approved by the owner, 2026-10-20. M0 to M3 are built as code (M3 never run);
nothing of M4 to M7. See [Status](#status) for what is done and what waits for old-pc.

## Status

As of 2026-10-20, branch `worktree-agent-a941eac1fa29ee91b`, rebased on `main` at `c719d58`.
Nothing was run on old-pc, and nothing was installed, signed, registered or trusted on the dev
box: only `okur doctor` (read only) and the test suite ran here.

**Commits.**

| Commit | What |
| --- | --- |
| `7694582` | M0 and M1: `okur doctor`, the library split, `okur-sr.exe`, one Okur per desktop |
| `e7a651f` | M2: Insert+F4, `confirm_exit`, the question, the exit path |
| `84e4036` | M3: `okur-setup` install and uninstall; `signing/dev-signing.ps1` |
| `a781583` | `okur run` says why it does not start; the screen-reader flag is set only when off |

**Checks, pasted from the tools** (the branch at `a781583`):

- `cargo test --workspace`: every test binary `ok`, none failed. It includes okur-corpus's
  parity ratchet, so every step's outcome is the committed `corpus/parity-baseline.json`'s.
- `okur parity`: "JAWS parity (Ia2): 2707/2904 steps match exactly; 74 accepted divergences not
  counted (0 scenarios could not run)"; "JAWS spoken parity: 2603/2852 steps match exactly".
  Before the rebase, on `3704ef4`, `okur parity`'s whole output was the same byte for byte
  before and after M0-M2 (2642/2859, spoken 2538/2807).
- `okur invariants`: "Buffer invariants: 0 violation(s)". `okur corpus-manifest`: "Corpus
  manifest is current." `corpus/` was not touched.
- `mt.exe -inputresource:<exe>;#1` on the debug build: `okur-sr.exe` has
  `level="asInvoker" uiAccess="true"`; `okur.exe`, `okur-sr-no-uiaccess.exe` and
  `okur-setup.exe` have `uiAccess="false"`.
- `okur doctor` on the dev box (a Medium copy of `okur.exe` from `target\`): integrity Medium
  (0x2000), TokenUIAccess 0, not elevated (limited token), signature "not signed", manifest
  "level=asInvoker uiAccess=false", screen reader flag on (JAWS), `okur_okur_v1 not present`,
  HKLM Configuration `"FreedomScientific_JAWS_v2025"`.

### M0. Facts

| Item | State | Where | Tests |
| --- | --- | --- | --- |
| `okur doctor [--pid N]`: session, desktop and input desktop, account and SID, integrity, `TokenUIAccess`, elevation and its type, whether the executable is under Program Files, its signature (`WinVerifyTrust`), its embedded manifest, the screen-reader flag, the copy running on this desktop, the AT key and both `Configuration` lists | done | `crates/okur/src/doctor.rs`; `okur-platform`'s `elevation.rs`, `desktop.rs`, `signature.rs`, `registry.rs`, `ease_of_access.rs` | `doctor::tests` (the report, Program Files), `elevation::tests`, `signature::tests`, `desktop::tests`, `ease_of_access::tests`, `registry::tests` |
| `okur-sr.exe --doctor-exit [file]`: the report of the UI Access process itself, to the console that started it or to a file (never on the secure desktop) | done | `crates/okur/src/sr.rs` | `sr::tests` |
| E0, E1, and §4's C9 and C14 verdicts updated | not done | old-pc | needs old-pc and Q1 |

### M1. Process hygiene

| Item | State | Where | Tests |
| --- | --- | --- | --- |
| The library split: `crates/okur` is a library (`run`, `startup`, `control`, `doctor`, `sr`) with `okur.exe` (console, no UI Access, runs from `target\` as before), `okur-sr.exe` (no console, `uiAccess="true"`) and `okur-sr-no-uiaccess.exe` (for installs whose signature does not verify) | done | `crates/okur/{build.rs,manifests/,src/}` | build; `mt.exe` above. `okur-sr` has no test binary of its own (a test program with its manifest would not start) |
| The keyboard hook on its own thread's desktop; the forced `OpenDesktopW("default")` is gone | done | `okur-platform/src/hook/windows.rs` | live only |
| The mutex `Local\Okur-<desktop>`, held by a thread of its own; an automatic start steps aside for a running copy, a manual start asks it to quit (4 s) and ends it if it stays | done | `okur-platform/src/instance.rs`, `crates/okur/src/startup.rs` | `startup::tests` (the instance rules against a stand-in), `instance::tests::one_copy_holds_the_lock_at_a_time` (a real mutex) |
| The control pipe `\\.\pipe\okur-<session>-<desktop>` (DACL: the user, SYSTEM, Administrators; local clients only; first instance only): `ping`, `status`, `quit`. `okur quit` and `okur-sr.exe --quit` | done | `instance.rs`, `crates/okur/src/control.rs` | `instance::tests::the_control_pipe_answers_one_line` (a real pipe), `control::tests` |
| `SPI_SETSCREENREADER` on while Okur runs (only when it was off, and cleared only then: JAWS may have it on), `SetProcessShutdownParameters(0x100, SHUTDOWN_NORETRY)`, a hidden top-level window for `WM_QUERYENDSESSION`/`WM_ENDSESSION`, and one exit path for every way out | done | `okur-platform/src/process.rs`, `crates/okur/src/run.rs` | live only |
| The Shift+Shift escape at `--ease-of-access` and `--sign-in` | done | `process.rs`, `startup.rs` | `process::tests` (a mock key state), `startup::tests::both_shifts_stop_an_automatic_start_only` |
| `elevation.rs` reads `TokenUIAccess` and the integrity level; the startup note about elevated windows depends on both | done | `okur-platform/src/elevation.rs` | `elevation::tests` |
| `okur-ui` started with a copy of Okur's token, UI Access cleared and integrity Medium, and allowed the foreground for each dialog | done, **only when Okur has UI Access**: an `okur run` started from an administrator terminal keeps its `okur-ui` at its own level, as before | `okur-platform/src/lowered_process.rs`, `ui_host.rs` | live only (`okur doctor --pid <okur-ui's pid>`) |
| Argument handling | done | `sr.rs` | `sr::tests` |

Decisions made on the way, for the owner to confirm or reverse:

- **No copy starts on the secure desktop** (`Winlogon`) until M6's secure mode: one there runs
  as SYSTEM and would have files, dialogs and logs within reach (R3). It exits silently.
- **`--sign-in` starts Okur as JAWS's default, Always, does,** until M4's resolver reads the
  user's choice. `/hardwarebuttonlaunch` counts as `--ease-of-access`.
- Ease of Access's start and exit notification (AccessibilityTemp and Win+U) is M4's, and
  `SetDefaultDllDirectories` is in §5.7's table (M6): neither is built.
- If `CreateProcessAsUserW` with the lowered token fails, `okur-ui` is started the plain way,
  at Okur's level, with a warning in the debug log.

### M2. Insert+F4

| Item | State | Where | Tests |
| --- | --- | --- | --- |
| `Command::ShutDown`, bound to JAWSKey+F4 (`ShutDownJAWS` in `Default.JKM`) and called by `ShutDownJAWS`, `Insert+F4` | done | `okur-core/src/command.rs` | `command::tests::the_jaws_key_and_f4_shut_okur_down` |
| `confirm_exit`, on by default (JAWS's "Prompt to Confirm When Exiting JAWS"), with a check box in Okur's settings dialog; off, Insert+F4 exits at once | done | `settings.rs`, `ui/settings.rs` | `ui::settings` tests, `engine::quit::tests::without_confirmation_insert_f4_exits_at_once` |
| The question as one of Okur's dialogs: its text static (new `ControlKind::Message`), OK the default, Cancel; Escape closes it and the focus returns quietly | done; **its words are placeholders** in `dialogs.ftl` (`quit-title`, `quit-question`, `quit-ok`, `quit-cancel`), commented to be replaced by 571's recording; JAWS's help gives no wording | `okur-core/src/ui/quit.rs`, `engine/quit.rs`, `okur-ui-model`, `okur-ui` (`template.rs`, `dialog.rs`), `okur-corpus`'s dialog mirror | `ui::quit::tests`, `engine::quit::tests`, `okur-ui`'s `a_message_is_static_text_above_the_buttons` |
| The exit path: keyboard unhooked first, events stopped, Okur's last words waited for at most 1.5 s, `okur-ui`'s input closed, the screen-reader flag cleared, the mutex released, the process ended. The engine only asks (`set_exit`); headless, as in replay, Insert+F4 does nothing | done | `crates/okur/src/run.rs`, `engine/quit.rs` | `engine::quit::tests::insert_f4_asks_first_and_ok_exits`; the process's end is live only |
| Scenario 571 imported, the J2 scenario ("Insert+F4, Enter") written and recorded, `okur parity --only 571` | not done | `corpus/` was not touched | needs old-pc |

Notes: Okur's last words stay "Okur exiting." (now `okur-exiting` in `commands.ftl`) until J2
records what JAWS says; whether JAWS says anything as its question closes is 571's step s3. With
no `okur-ui` to ask in, Insert+F4 exits at once. Insert+H's list of keys does not name Insert+F4
yet (it would change 526 and 527's replay).

### M3. Signed UI Access install (code only, never run)

| Item | State | Where | Tests |
| --- | --- | --- | --- |
| `okur-setup install [--from <folder>] [--silent] [--no-start] [--no-start-after-sign-in]`: unelevated, it starts itself elevated (`runas`) for the machine's part; every copy asked to quit over its pipe and stragglers in the install folder ended; the files copied to `%ProgramFiles%\Okur` (the UI Access `okur-sr.exe` only if `WinVerifyTrust` accepts it, else `okur-sr-no-uiaccess.exe` under that name; files in use renamed aside and deleted at the next start); the registry of §5.4 without HKLM `Configuration`; Okur started through the shell by the unelevated copy | done | `crates/okur-setup/src/{main,setup,plan,args}.rs` | `plan::tests`, `setup::tests` (install and uninstall into a temporary folder against `MemoryRegistry` and a stand-in machine), `args::tests` |
| `okur-setup uninstall [--silent]`: copies stopped, Okur's name out of HKLM's and the user's `Configuration` (other names kept), its keys and Run value deleted, its files gone (at the next start where in use), `%APPDATA%\Okur` kept | done | as above | `setup::tests::uninstall_removes_what_install_wrote_and_only_that` |
| `crates/okur-setup/signing/dev-signing.ps1` (§5.11): `-NewCertificate`, `-Sign`, `-Verify`, `-TrustOnLab`, `-UndoLab`, `-UndoDevBox`, each behind `ShouldProcess` (`-WhatIf`) | written, never run | as named | `signing_tests` read it as text: the certificate's scope, and no bcdedit, test signing, Trusted Publishers or exported key |

Notes, for the owner:

- **The Run key's value is written by default** (§5.4): until M4's choices, every user's
  sign-in then starts Okur, beside JAWS on old-pc. Install with `--no-start-after-sign-in`
  there.
- `secure-settings\` is not seeded (M6). The installer's words are English in its code, not
  in the catalog, until M7's installer dialogs. Publisher in Apps & features: the workspace's
  `authors`.

### Live checks on old-pc, with the exact steps

On the dev box, build and push (the lab's `lab-push.ps1` pushes only `okur.exe`: copy the
others to `C:\Users\ailabs\sr-harness\bin` the same way):

```powershell
cargo build --release -p okur -p okur-ui -p okur-setup
# target\release\: okur.exe, okur-sr.exe, okur-sr-no-uiaccess.exe, okur-ui.exe, okur-setup.exe
```

1. **M0, E1 without a certificate** (no owner's yes needed). On old-pc, unelevated:
   ```powershell
   cd C:\Users\ailabs\sr-harness\bin
   .\okur.exe doctor
   Start-Process .\okur-sr.exe -ArgumentList '--doctor-exit', "$env:TEMP\e1a.txt" -Wait; Get-Content $env:TEMP\e1a.txt
   ```
   Case (a): does the start fail ("A referral was returned from the server") or run without
   UI Access (`TokenUIAccess 0`)? Note which.
2. **M1, one copy per desktop.** With `.\okur.exe run --settings <lab settings>` running in a
   terminal, start a second one in another: the first says "Okur exiting." and ends, the second
   runs (`Get-Process okur` shows one). `.\okur.exe quit` ends it ("Okur was asked to quit, and
   is quitting."); `.\okur.exe doctor` then shows "Okur on this desktop: none answers".
   `Start-Process .\okur-sr-no-uiaccess.exe -ArgumentList '--ease-of-access'` twice: one
   process. Hold both Shift keys while starting it with `--sign-in`: no process. Listen: no
   console window flashes.
3. **M2.** With `okur.exe run` in front of Notepad: Insert+F4 opens "Exit Okur"; Escape, then
   Insert+Tab reads Notepad. Insert+F4, Enter: "Okur exiting.", the process is gone, and
   `okur keys`' modifier guard reports no held key. Then import 571 and write J2's scenario.
4. **M3 and E1 with the certificate** (needs the owner's yes, Q1). On the dev box:
   `crates\okur-setup\signing\dev-signing.ps1 -NewCertificate -WhatIf`, then without `-WhatIf`;
   then `-Sign -Thumbprint <THUMB> -BuildDir target\release`. Copy the `.cer` and the release
   programs to old-pc. On old-pc, elevated: `dev-signing.ps1 -TrustOnLab -CertificateFile
   <path>`. Then, unelevated: `.\okur-setup.exe install --from <folder> --no-start-after-sign-in`
   and accept its UAC prompt. Check:
   - `Get-AuthenticodeSignature "C:\Program Files\Okur\okur-sr.exe"` says Valid;
   - `Start-Process "C:\Program Files\Okur\okur-sr.exe" -ArgumentList '--doctor-exit',
     "$env:TEMP\e1c.txt" -Wait; Get-Content $env:TEMP\e1c.txt` (through the shell: `&` and
     `-NoNewWindow` use CreateProcess, which a UI Access program refuses with error 740, case
     (e)): TokenUIAccess 1, integrity High (an administrator), in Program Files yes, signature
     valid;
   - `okur doctor --pid <okur-ui's pid>`: integrity Medium, TokenUIAccess 0 (else the lowered
     start failed: the debug log says why);
   - `reg query "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Accessibility\ATs\okur_okur_v1"`
     and the Uninstall, App Paths and `HKLM\SOFTWARE\Okur` keys, pasted; Apps & features lists
     Okur;
   - elevated Notepad (`Start-Process notepad -Verb RunAs`): Okur reads its text and hears its
     keys (`--debug-log`);
   - sign out and in (the owner): no "apps are blocking shutdown" screen;
   - `okur-setup.exe uninstall`, then the same `reg query`s: every key gone, JAWS's names still in
     both `Configuration` lists; `C:\Program Files\Okur` gone (or empty until the next start).
   - Undo: `dev-signing.ps1 -UndoLab -Thumbprint <THUMB>` on old-pc (elevated).

**Scope.** What Okur needs so that a blind user can rely on it the way they rely on JAWS
from the moment the computer starts:

1. Okur starts by itself on the Windows sign-in screen and on the other secure screens
   (Ctrl+Alt+Delete, the credential screen after a lock, User Account Control prompts).
2. Okur starts by itself after the user signs in, with JAWS's three start options.
3. Insert+F4 quits Okur after JAWS's question (scenario `corpus/pending/571-desktop-quit-question.json`).
4. All of it works during development with a self-signed certificate, and with a real one
   at release.

**Code references** are to commit `8a01219`. Another engineer is committing in this tree;
line numbers will move.

**Clean room.** Everything about JAWS here comes from JAWS 2025's help, its key map
(`Default.JKM`), and what the registry, the process list and the service list on the dev box
show about the installed JAWS. No JAWS program file was read.

**How each claim is marked.**

- **documented**: a primary source says so (Microsoft, Apple, JAWS's help). Sources are in §8.
- **NVDA's code**: read in NVDA's source at tag `release-2026.2`.
- **observed**: read on the dev box (Windows 11 26200) on 2026-10-20, read-only.
- **reported**: a third-party report, not a primary source.
- **inferred**: my conclusion from the above, to be confirmed by an experiment in §6.

---

## 1. Summary

**What Okur needs, in order.**

1. **A product build that Windows will trust.** This build has an embedded manifest with
   `uiAccess="true"`, is signed, and is installed under `C:\Program Files\Okur`. Without it,
   Okur cannot read or type into elevated windows, cannot reliably take the foreground, and
   probably misses keys typed into elevated windows.
2. **Process hygiene.**
   - one Okur per desktop;
   - a way to ask a running Okur to quit;
   - no console window;
   - a clean exit when Windows shuts down;
   - the hook thread stays on its own desktop. Today it forces itself onto the desktop
     named `default`, which would break Okur on the secure desktop.
3. **Insert+F4.** JAWS's question, then an orderly exit.
4. **An installer and an uninstaller.** They register Okur with Windows as an assistive
   technology.
5. **Start after sign-in**, with JAWS's options.
6. **Lock-screen mode** for the user's Okur. The Windows lock screen runs on the user's own
   desktop, not on the secure desktop.
7. **Secure-desktop mode** for the copy of Okur that Windows starts as SYSTEM on the sign-in
   screen, the credential screen and UAC prompts.
8. **Release signing.**

**The recommended design in a paragraph.** Okur registers itself as an Ease of Access
assistive technology, as JAWS, NVDA and Narrator do:

- the key `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Accessibility\ATs\okur_okur_v1`,
  with `TerminateOnDesktopSwitch=0`;
- its name in the machine-wide `Accessibility\Configuration` value to start on the sign-in
  screen.

Windows' Assistive Technology Manager (`AtBroker.exe`) then starts `okur-sr.exe
--ease-of-access` as SYSTEM, in the user's session, on the secure desktop (`WinSta0\Winlogon`).
That copy sees that it runs on `Winlogon` and switches itself into a locked-down secure mode:

- it reads a read-only settings folder that only administrators can write;
- it writes nothing and logs nothing;
- it opens no files, links or dialogs (only the quit question, if JAWS asks it there);
- it uses no network and no clipboard.

The user's own copy runs on the user's desktop with `uiAccess`. It goes quiet whenever another
desktop takes the input, and it restricts itself while the session is locked. Each desktop has
at most one Okur, enforced by a named mutex per desktop. `uiAccess` matters only for the user's
copy: the secure copy runs as SYSTEM, where Microsoft says `uiAccess` "has no effect".

For development, a self-signed code-signing certificate does the job:

- the key stays on the dev box and cannot be exported;
- only the public part goes into `LocalMachine\Root` on old-pc;
- no `bcdedit` setting, no test-signing mode, no change to Secure Boot.

**Main corrections to the pasted answer** (details in §4):

- `SecureDesktopAccommodation` set to Okur's file name would turn Okur **off** on secure
  screens.
- Both `bcdedit` settings concern kernel-mode drivers and do nothing for a user-mode program.
  Each needs Secure Boot off, and `nointegritychecks` is ignored by modern Windows.
- No source says Windows refuses an unsigned program on the secure desktop. What needs the
  signature is `uiAccess`, which applies to the user's copy.
- The answer leaves out what actually makes Okur start: the `Configuration` values, and
  `TerminateOnDesktopSwitch`. The default for that value has Windows kill and restart Okur at
  every desktop switch.

---

## 2. How Windows does it

### 2.1 Words used below

| Term | What it is |
| --- | --- |
| **Session** | One sign-in. The console user is in session 1, a second user in session 2, and so on. Services run in session 0, which has no screen anyone sees [MS7]. |
| **Desktop** | A set of windows inside a session. A user's apps live on `WinSta0\Default`. |
| **Secure desktop** | `WinSta0\Winlogon`. It holds the sign-in screen, the Ctrl+Alt+Delete options, the credential screen after a lock, and UAC prompts when `PromptOnSecureDesktop=1` (the Windows default). Processes there run as SYSTEM. |
| **Lock screen** | The picture and clock shown after Win+L, from `LockApp.exe`. It runs on the user's own desktop, in the user's session, **not** on the secure desktop. NVDA's security advisory puts it this way: "The lock screen (LockApp.exe) is not a secure screen - it is the screen before the sign-in screen" [N12]. Pressing a key there brings the credential screen, which is on the secure desktop. |
| **Temporary locked desktops** | Out-of-box setup (OOBE) and the "I forgot my PIN" flow "run on a temporary user desktop" that looks like the lock screen [N13]. |

A low-level keyboard hook (`WH_KEYBOARD_LL`) sees only the input of the desktop its thread is
on. A global hook "affects all applications in the same desktop as the calling thread"
(**documented** [MS9]). So each desktop needs its own Okur with its own hook.

### 2.2 The assistive-technology registration

Every value Windows reads (**documented** [MS1]) sits under
`HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Accessibility\ATs\<CompanyName_ProductName_v#>`.
Writing it needs administrator rights.

| Value | Type | Required | Meaning |
| --- | --- | --- | --- |
| `ApplicationName` | REG_SZ | yes | Name shown in the Ease of Access Center. Plain text or `@dll,-resid`. |
| `Description` | REG_SZ | yes | Under 512 characters. Shown in the Ease of Access Center. |
| `Profile` | REG_SZ | yes | XML listing the accommodations, e.g. `<HCIModel><Accommodation type="severe vision"/></HCIModel>`. The values are case-sensitive. |
| `SimpleProfile` | REG_SZ | yes | A word or two such as `screenreader`. |
| `ATExe` | REG_SZ | yes (per the docs) | File name only. "Windows uses this value to determine whether the accessibility application is running." |
| `StartExe` | REG_SZ | yes | Full path that Windows starts. |
| `StartParams` | REG_SZ | no | Arguments added to `StartExe`. |
| `TerminateOnDesktopSwitch` | DWORD | no | **Absent or 1** (the default): Windows runs the AT in a job and "terminates and restarts the application on each transition to or from the secure desktop". **0**: Windows does not kill it. "The application continues to run on the previous desktop, and Windows starts a new instance on the new desktop if an instance is not already running there." |
| `CopySettingsToLockedDesktop` | DWORD | no | 1: Windows copies the AT's values under `HKCU\...\Accessibility\ATConfig\<AT key>` into the same place in the secure desktop's HKCU. |
| `PassiveAutoStartBehavior` | DWORD | no | 0 (default, legacy): the "Start after sign-in" box is checked in OOBE and the Control Panel, and Windows "automatically starts the AT after UAC and lock-screen". 1: Windows starts the AT only once per user session, at sign-in, and only if that box is checked. |
| `SecureDesktopAccommodation` | REG_SZ | no | The name of **another AT key** to run on the secure desktop instead: an alternate build, one of `Narrator`, `osk` or `magnifierpane`, or `none`. "The name of a nonexistent application" also means none. **Leave it out to run the AT itself on the secure desktop.** |

With `TerminateOnDesktopSwitch=0` the AT takes on duties (**documented** [MS1]):

- it must notice when its desktop is no longer active, go idle, and "relinquish control of
  hardware so the secure desktop version of the application can use it" (a braille display,
  for example);
- it must support "multiple running copies";
- it must tell Windows when it starts and exits outside Windows' control. It sets
  `HKCU\Software\Microsoft\Windows NT\CurrentVersion\AccessibilityTemp\<AT key>` to the DWORD
  3 (starting) or 2 (exiting), then injects Win+U with `SendInput`.

When Windows itself starts the AT from a hardware button, it adds `/hardwarebuttonlaunch`.

**What starts the AT.** Writing the key alone starts nothing. Windows reads a comma-separated
`Configuration` value under `...\Accessibility`:

| Where | Meaning | Source |
| --- | --- | --- |
| `HKLM\...\Accessibility\Configuration` | Start on the sign-in screen, for everyone. | **NVDA's code** [N1]: `AutoStartContext.ON_LOGON_SCREEN = HKEY_LOCAL_MACHINE`. **Observed**: holds `FreedomScientific_JAWS_v2025` on the dev box, where JAWS's "start at the Log On screen" is on by default. |
| `HKCU\...\Accessibility\Configuration` | Start after this user signs in. | **NVDA's code** [N1]: `AFTER_LOGON = HKEY_CURRENT_USER`. **Observed**: holds `freedomscientific_jaws_v2025` (lowercase) on the dev box. |

Microsoft's page [MS1] describes the HKCU value differently: Windows "will launch your
application immediately after it is installed" and "whenever the logon desktop is active".
That does not match NVDA's working code or what the dev box shows. Experiment E5 settles it.

The Control Panel page **Ease of Access Center → Change sign-in settings** has "At sign-in"
and "After sign-in" check boxes per AT, and needs an administrator. Microsoft says registered
third-party ATs "also appear in the Ease of Access Center" [MS1]. Whether Okur shows on that
page in Windows 10 and 11 is part of E5.

**Observed, dev box.** How the installed ATs register:

- **JAWS** (`ATs\FreedomScientific_JAWS_v2025`):
  - `ApplicationName` and `Description` are both "JAWS 2025";
  - `Profile`: mild and severe vision, mild and severe cognitive;
  - `SimpleProfile` is `screenreader`;
  - `StartExe` is `C:\Program Files\Freedom Scientific\JAWS\2025\fsATProxy.exe`;
  - `TerminateOnDesktopSwitch=0`;
  - **no `ATExe`, no `StartParams`, no `CopySettingsToLockedDesktop`, no
    `PassiveAutoStartBehavior`**.
- **Narrator**: `ATExe=Narrator.exe`, `CopySettingsToLockedDesktop=1`,
  `PassiveAutoStartBehavior=1`, `TerminateOnDesktopSwitch=0`.
- **Magnifier**: also has an undocumented `StopParams=/IM Magnify.exe`.
- **Running state**: Windows keeps it in `HKLM\...\Accessibility\Session1`:
  - `Configuration=freedomscientific_jaws_v2025`;
  - an empty `SecureConfiguration`;
  - an `ATConfig\Narrator` copy of Narrator's settings.

  **Inferred**: these are Windows' record of the ATs running in session 1, and the per-session
  copy behind `CopySettingsToLockedDesktop`.
- **Case**: the HKCU and session values hold JAWS's name in lowercase while the key and the
  HKLM value use mixed case (observed). Windows probably lowercases them when it writes
  (**inferred**). NVDA's own name is all lowercase (`nvda_nvda_v1`, [N2]), and NVDA compares
  names case-sensitively [N1]. **Recommendation**: name Okur's key in lowercase and compare
  names without regard to case.

### 2.3 AtBroker, accounts, sessions and the hand-over

**Observed**: `C:\Windows\System32\AtBroker.exe` describes itself as "Windows Assistive
Technology Manager". Microsoft does not document it beyond the registration page. What is
known:

- **The secure-desktop copy runs as SYSTEM, in the user's session, on `WinSta0\Winlogon`.**
  - Microsoft's page on AT security says access to UI under the system integrity level "is
    only possible if the process is launched in the user account control (UAC) desktop under
    SYSTEM (and system IL). In this case, setting UIAccess has no effect" (**documented**
    [MS2]).
  - NVDA writes its secure-screen logs to "the System profile's `%TEMP%` directory"
    (**NVDA's docs** [N10]).
  - Session 0 has no screen, so the copy cannot be there (**documented** [MS7]).
  - The SYSTEM account's HKCU is `HKEY_USERS\.DEFAULT`, which "is an alias for
    HKEY_USERS\S-1-5-18" (**documented** [X6]).
- **When Windows starts it there.** "The Ease of Access Center launches your application on
  the secure desktop if it was already running on the normal desktop, or if it is configured
  to run on the logon desktop" (**documented** [MS1]). "Already running" means that Windows
  knows it runs: either Windows started it, or it notified Windows with AccessibilityTemp and
  Win+U (§2.2). NVDA notifies on every start and exit, except on the secure desktop and in its
  Store version (**NVDA's code** [N3]).
- **What happens at a desktop switch** with `TerminateOnDesktopSwitch=0` (**documented** [MS1]):
  - the old copy keeps running on its desktop;
  - a new copy starts on the new desktop if none runs there;
  - both copies exist at once.

  NVDA's handling (**NVDA's code** [N4], [N5]):
  - The user's copy watches `EVENT_SYSTEM_DESKTOPSWITCH`. If `OpenInputDesktop` then fails, it
    concludes that a secure desktop came up. It clears its record of held modifiers ("We don't
    receive key up events for any keys down before switching"), cancels speech, says "Secure
    Desktop", and sleeps.
  - The secure copy has nothing that makes it exit when its desktop loses the input. So it
    simply stays, unheard (**inferred**). Experiment E4 checks whether it lives on after
    sign-in and after each UAC prompt.
- **Several users, remote sessions.** Each session has its own Winlogon desktop and gets its
  own copies (**inferred**).

### 2.4 Settings on the secure desktop

Windows offers one route: `CopySettingsToLockedDesktop` plus registry values under `ATConfig`.

- "Your accessibility application should write its settings at regular intervals or whenever
  the values change. Writing settings on application exit will not work."
- Its caution: "Because the registry keys described here are written in user mode, they are
  not secure ... should not read plug-in DLL names, and should not execute any commands found
  in strings" (**documented** [MS1]).

Narrator uses that route (observed). NVDA does not. It keeps a separate `systemConfig` folder
in its installation folder, which only administrators can write. The user fills it with "Use
currently saved settings during sign-in and on secure screens", which runs an elevated helper.
The copy leaves out executables and the folders that could hold unpackaged add-ons
(**NVDA's code** [N6], **NVDA's docs** [N9]).

On the dev box, `HKU\.DEFAULT` has no `Accessibility` key at all (**observed**). How JAWS
chooses its voice and rate on the sign-in screen is not in its help; experiment J8 finds out.

### 2.5 Voices and audio on the secure desktop

- **Voices.** Registered machine-wide voices are visible to SYSTEM; voices registered for one
  user are not (**inferred**). On the dev box all voices are machine-wide (**observed**):
  - SAPI 5 under `HKLM\SOFTWARE\Microsoft\Speech\Voices\Tokens`: David and Zira;
  - OneCore under `HKLM\SOFTWARE\Microsoft\Speech_OneCore\Voices\Tokens`: David, Mark, Zira
    and the Turkish Tolga;
  - none in HKCU.
- **Audio.** Narrator speaks on the sign-in screen with OneCore voices, so WinRT speech and a
  shared-mode audio stream work for SYSTEM in the user's session (**inferred** from Narrator's
  behavior). Okur's OneCore engine and its WASAPI stream (`crates/okur-platform/src/tts/onecore.rs`)
  must still be tested there (E3).

### 2.6 uiAccess: what it gives, what it needs

**What it gives** (**documented** [MS3], [MS4]). A process started with UIAccess can:

- "Set the foreground window."
- "Drive any application window by using the SendInput function."
- "Use read input for all integrity levels by using low-level hooks, raw input,
  GetKeyState, GetAsyncKeyState, and GetKeyboardInput."
- "Set journal hooks."
- "Use AttachThreadInput to attach a thread to a higher integrity input queue."

Its topmost windows also sit above other topmost windows [MS2].

**Its integrity level.**

- High for an administrator who signed in with a split token (the owner's case).
- Medium+16 for a standard user: "medium+" in Microsoft's words (**documented** [MS2]), "the
  current integrity plus 16" in Forshaw's [X2].
- Microsoft adds that a standard user's UIAccess process "cannot access elevated UI"
  [MS2].
- No UIAccess process reaches UI at the system integrity level [MS2].

**What it needs** (**documented** [MS3], [MS4]; Forshaw [X1], [X2]):

1. A manifest whose `requestedExecutionLevel` has `uiAccess="true"`. Use
   `level="asInvoker"`; never elevate the screen reader itself.
2. An Authenticode signature that chains "to a trusted root in the local machine Trusted
   Root Certification Authorities certificate store". Project Zero notes that no special EKU
   is required [X1], and Forshaw that self-signing works [X2].
3. A location only administrators can write:
   - `%ProgramFiles%` and its subfolders, including `Program Files (x86)`;
   - `%WinDir%`, except a list of subfolders that standard users can write.

   The policy "Only elevate UIAccess applications that are installed in secure locations"
   (`EnableSecureUIAPaths`) enforces the location, is on by default, and is on on the dev box
   (observed). "Windows enforces a PKI signature check ... regardless of the state of this
   security setting" [MS4].
4. Starting through the AppInfo service: `ShellExecute`, `Start-Process`, Explorer, or
   AtBroker. Plain `CreateProcess` of a UIAccess executable fails with
   `ERROR_ELEVATION_REQUIRED` (740) (**reported** [X3], [X4]; AutoHotkey's docs say UIA
   programs "can't be launched with CreateProcess" [X4]).

**What happens when a check fails.**

- Microsoft's policy page says the application "is started without UIAccess rights" [MS4].
- Field reports say the start fails with "A referral was returned from the server" [X3].

Experiment E1 settles which one applies to Okur.

**What it means for Okur's keyboard hook and UI Automation.**

- Okur's hook is a `WH_KEYBOARD_LL` hook. The documented UIAccess right to "read input for all
  integrity levels using low-level hooks" implies that a hook without UIAccess does **not** see
  keys typed into elevated windows (**inferred**; E1 checks today's Okur).
- The handoff `docs/handoff/2026-10-17-docs-sheets-settings.md` already shows the UI
  Automation half:
  - an Okur at Medium saw only empty panes in an elevated Settings dialog;
  - JAWS's `jfw.exe` runs at High (**observed** then).
- That fits JAWS being a UIAccess process started by an administrator (**inferred**).

**On the secure desktop.** UIAccess plays no part there: the copy runs as SYSTEM [MS2]. The
user's copy cannot read the secure desktop at all. NVDA: "A user instance of NVDA cannot read
the contents of the secure screen" [N5].

**Windows 11 Administrator protection** is opt-in, from 24H2. Under it, a UIAccess process
runs as the limited user at High integrity, without the profile separation the feature
otherwise gives (**documented** by Project Zero [X1]). Whether it can still read windows of
the hidden administrator account is untested (risk R7).

### 2.7 Windows 10 and 11

- `TerminateOnDesktopSwitch=0` has worked since Windows 8. Windows 7 killed ATs on every
  desktop switch, which is why NVDA had `nvda_eoaProxy.exe` until 2024.1 [N8].
- `PassiveAutoStartBehavior` is documented now, but which Windows versions honor it is not
  stated. Microsoft's own ATs set it to 1 on Windows 11 (observed).
- Low-level hooks are removed silently when one call exceeds `LowLevelHooksTimeout`, which is
  capped at 1 s from Windows 10 1709 [MS8]. Okur already reinstalls its hook after a slow key
  (`hook/windows.rs`).
- Windows 11 clean installs may run **Smart App Control**. It blocks apps that are "unsigned,
  or the signature is invalid" when its cloud service has no verdict [MS10]. Release builds
  therefore need a certificate from a public CA. A self-signed build will not run on such a
  machine.
- old-pc runs Windows 10 19045. The dev box runs Windows 11 Home 26200, which has neither
  Hyper-V nor Windows Sandbox (observed).

---

## 3. How NVDA does it, and how JAWS presents it

### 3.1 NVDA's mechanics (read at tag `release-2026.2`)

**History** [N8]:

- `nvda_service.exe` existed for Windows XP, which had no AT registration. It was removed in
  2017.4 (PR #7625).
- `nvda_eoaProxy.exe` kept Windows 7's Ease of Access from killing NVDA at desktop switches.
  It was removed in 2024.1 (#15577).
- Today Ease of Access starts `nvda.exe` directly.

**Two executables from the same code** [N7] (`source/setup.py`, `source/manifest.template.xml`):

- `nvda_noUIAccess.exe` is always built.
- `nvda_uiAccess.exe` is built only for signed builds (`--enable-uiAccess`).
- The manifest is `level="asInvoker" uiAccess="%(uiAccess)s"`, with Windows 10/11 supported-OS
  and per-monitor DPI entries.
- **Install** copies the UIAccess build to `nvda.exe` if it exists, else the other one
  (`installer.install`).
- **A portable copy** always uses `nvda_noUIAccess.exe` (`createPortableCopy`).

**Registration** (`installer.registerEaseOfAccess` [N2]), written to the 64-bit view of
`ATs\nvda_nvda_v1`:

- `ApplicationName`, `Description`;
- `Profile`: severe vision;
- `SimpleProfile=screenreader`;
- `ATExe=nvda.exe`;
- `StartExe=<install dir>\nvda.exe`;
- `StartParams=--ease-of-access`;
- `TerminateOnDesktopSwitch=0`.

Install also writes the Uninstall entry, `App Paths\nvda.exe` and `HKLM\SOFTWARE\NVDA`, and
sets the sign-in-screen autostart. Uninstall deletes the AT key and removes NVDA from the HKLM
`Configuration`.

**Start options** (`source/easeOfAccess.py` [N1], `source/config/__init__.py` [N6]):

- "Use NVDA during sign-in": NVDA's name in **HKLM** `Configuration`. If NVDA lacks the
  rights, it retries through the elevated helper `nvda_slave.exe config_setStartOnLogonScreen`.
- "Start NVDA after I sign in": its name in **HKCU** `Configuration`.

**Single instance and replacement** (`source/nvda.pyw` [N3], `projectDocs/design/startupShutdown.md` [N11]):

- A mutex named `Local\NVDA_<desktop name>`: one per session (the `Local\` namespace) and per
  desktop. NVDA waits 2 s for a previous copy to finish exiting.
- A new manual start finds the old copy's window with `FindWindow` and sends it `WM_QUIT`. If
  the old copy does not exit within 4 s, the new one terminates it.
- A start with `--ease-of-access` simply exits if a copy is already running.
- NVDA calls `ChangeWindowMessageFilter(WM_QUIT, MSGFLT_ALLOW)` so that a lower-integrity copy
  can ask it to quit.
- `SetProcessShutdownParameters(0x100, SHUTDOWN_NORETRY)` makes it the last application shut
  down, and keeps it out of the "apps are blocking shutdown" list, "If it were, the user
  would have no way of reading the dialog".
- It sets the system screen-reader flag (`SPI_SETSCREENREADER`) while running.

**Secure mode** (`nvda.pyw`, `source/utils/security.py` [N5], user guide [N9]):

- **Detection.** The desktop's name is `Winlogon` (`isRunningOnSecureDesktop`).
- **On the secure desktop** NVDA forces secure mode (unless the HKLM `serviceDebug` value is
  set), minimal mode, and the `systemConfig` folder.
- **Secure mode disables:**
  - saving configuration, gestures and profiles;
  - `-c` config paths;
  - updates and portable copies;
  - starting remote sessions by hand;
  - the add-on store;
  - the Python console;
  - the log viewer and **all logging**: "it is a security concern to log during secure mode
    (e.g. passwords are logged on secure screens)" [N10];
  - the braille and speech viewers;
  - opening documents.
- **Its exit dialog still opens there**, minus the debug-logging and update choices [N5].
- **The lock screen** (`winAPI/sessionTracking.py`, `utils/security.py` [N4], [N5]):
  - the user's copy tracks the lock state through WTS session queries;
  - while locked, it allows only an explicit list of "safe scripts" (focus, title, time,
    battery, object and review navigation, mouse, braille, touch);
  - it refuses objects "below the lock screen";
  - it treats the PIN-reset and OOBE desktops the same way.

**Settings for secure screens.** The `systemConfig` folder in the installation folder is
written only by an explicit, elevated copy (§2.4).

**Signing.**

- Releases are signed through SignPath (`ci/scripts/sign.ps1` calls `Submit-SigningRequest`).
- For developers, `projectDocs/dev/selfSignedBuild.md` [N7]:
  - `New-SelfSignedCertificate -Type Custom -KeyUsage DigitalSignature -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")`;
  - import into `Cert:\LocalMachine\Root`;
  - "remove the certificate when you are done testing".
- There is no `bcdedit` step anywhere.

**NVDA's secure-screen advisories** (lessons for Okur, [N12], [N13]):

| Advisory | Lesson |
| --- | --- |
| GHSA-h7pp-6jqw-g3pj | A clipboard error dialog on the secure screen led to a file dialog and "arbitrary code execution". |
| GHSA-3jj9-295f-h69w | The report-formatting dialog opened a shell on secure screens. |
| GHSA-vw48-6q24-rppq | Context help and the add-on manager allowed privilege escalation. |
| GHSA-mvc8-5rv9-w3hx, GHSA-585m-rpvv-93qg | The Python console was reachable from secure screens and the lock screen. |
| GHSA-354r-wr4v-cx28 | Debug logging on secure screens. |
| GHSA-rmq3-vvhq-gp32, GHSA-x2c3-c6j2-87w4 | Object navigation escaped the lock screen. Lock state taken from "the foreground process" was "unreliable". |
| GHSA-72mj-mqhj-qh4w | "NVDA assumed Windows is unlocked when starting". Fix: "initialize the session state by querying the session directly". |
| GHSA-fpwc-2gxx-j9v7 | The "forgot my PIN" desktop. |
| GHSA-grvr-j2h8-3qm4 | Speech Viewer text cached on the lock screen. |
| GHSA-8f8q-2jc3-6rf4 | The last braille text of the user's session stayed on the display on the secure screen. |
| GHSA-qf5h-qw92-rx2f, GHSA-q7c2-pgqm-vvw5 | DLL planting next to the installer, and self-compiled DLLs. |

### 3.2 How JAWS presents it (JAWS 2025 help, key map)

**Start options.** JAWS shows them in Options > Basics > "Automatically Start JAWS" (the Start
JAWS Options dialog) and in the Startup Wizard's Run JAWS Settings page (**documented** [J1],
[J2], [J3]):

| Option | Who can change it | Default | What the help says |
| --- | --- | --- | --- |
| Automatically start JAWS at the Log On screen | Administrators only. A standard user "will not hear it". | Checked | "JAWS will load and start when the Windows log on screen appears ... applies to all users who log on to this version of JAWS on this computer." |
| Automatically start JAWS after log on for all users | Administrators only | Cleared | "control how JAWS starts for all subsequent users who log on" |
| Start JAWS after log on for this user | Every user. It is the only option a standard user sees. | Always | A combo box: **Always, Never, Using the All Users Setting**. "The value selected ... overrides the value of the Automatically start JAWS after log on for all users check box." |
| Run JAWS from System Tray | User | Cleared | Takes effect at the next start. |
| Prompt to Confirm When Exiting JAWS | User | Checked | "When cleared, JAWS immediately closes when you use any of the commands to exit the program including INSERT+F4." |

Further documented points:

- "Having JAWS start automatically ... is also known as running JAWS as a service" [J1].
- **The escape hatch**: "press and hold both the LEFT SHIFT and RIGHT SHIFT keys as Windows
  boots, or after you have logged in, to prevent JAWS from starting" [J1].
- Manual start: Win+R, `JAWS2025`; `JAWS2025 /default` starts JAWS with factory settings [J1].
- Installing JAWS needs an administrator and a restart [J6].

**Quitting** (**documented** [J4], [J5], [J7], **observed** in the key map):

- `Default.JKM` binds `JAWSKey+F4=ShutDownJAWS`. Layered `JAWSKey+Space&F4=RestartWithoutDump`
  is a separate restart command.
- The keystroke list says "Shut Down JAWS: INSERT+F4". The tutorial says "Press INSERT+F4 to
  close JAWS".
- Options > Exit: "a dialog is brought up to confirm whether or not you wish to exit the JAWS
  application. Press ENTER to exit JAWS, or press ESC to return".
- JAWS's own hot-key help lists "Close JAWS at any time Insert+F4" (corpus evidence 527).
- The question's exact words are recorded by scenario 571. It is queued in round 62 of
  `docs/plans/2026-10-21-everyday-gaps-round-3.md` and not yet imported. Okur must take its
  words from that evidence, not from this page.

**Secure screens.** The help says little beyond the start option. "What's New" mentions fixes
"on the Windows Logon screen": no password characters spoken after the Language menu, the
input-language list, legal notices read before logon [J8].

**Observed on the dev box** (registry, service and process lists only):

- `HKLM\...\Run` has `JAWS = "C:\Program Files\Freedom Scientific\JAWS\2025\jfw.exe" /run`.
- `jfw.exe` runs as the user and was started by `explorer.exe`.
- `fsATProxy.exe` (the registered `StartExe`) runs as the user; its parent had exited.
- Two services are installed, both Manual and LocalSystem: `UtilityService_JAWS_2025` and
  `JTVNCProxy_2025`. No JAWS service starts automatically.

**Inferred**: JAWS starts after sign-in through the Run key (`/run`), and per-user choices are
resolved at that point. It also registers with Ease of Access for the secure screens. Because
the observable mechanism is ordinary Windows registration, Okur can offer the same user-facing
options with Windows' own mechanism.

---

## 4. The pasted answer, checked claim by claim

| # | Claim | Verdict | Why |
| --- | --- | --- | --- |
| C1 | Windows uses a registration workflow to decide which ATs run on the secure desktop. | **Holds** | [MS1]. |
| C2 | Create `HKLM\...\Accessibility\ATs\[Company_Product_v1]`. | **Holds** | Format `CompanyName_ProductName_v#`, elevated writer [MS1]. Use lowercase, since Windows lowercases names in the HKCU and session values (observed) and NVDA compares case-sensitively [N1]. |
| C3 | `ApplicationName`, the localized name. | **Holds** | Mandatory, plain or `@dll,-id` [MS1]. |
| C4 | `ATExe`, the file name. | **Holds** | Mandatory per [MS1]; used to tell whether the AT runs. JAWS leaves it out (observed), NVDA sets it [N2]. Set it. |
| C5 | `StartExe`, the full path. | **Holds** | [MS1]. |
| C6 | `SecureDesktopAccommodation` = `myscreenreader.exe` "so Windows knows it is intentionally designed for secure desktops". | **Wrong, and harmful** | The value names an **alternate AT key**, a Microsoft AT, or `none`. A name that is not a registered AT means no AT runs on the secure desktop [MS1]. Following the answer would turn Okur off exactly where it is needed. Leave the value out. |
| C7 | `CopySettingsToLockedDesktop=1` "clone[s] user-configured runtime choices to the secure logon hive". | **Partly** | It copies only what the AT writes under `HKCU\...\Accessibility\ATConfig\<key>`, and Microsoft warns that the data is untrusted [MS1]. Okur keeps its settings in TOML files (`settings_file.rs`), so the flag alone copies nothing. §5.7 proposes NVDA's explicit, admin-only copy. |
| C8 | (Omitted) the other values. | **Missing, and they matter** | `Description`, `Profile` and `SimpleProfile` are mandatory. `StartParams` lets Okur know Windows started it. `TerminateOnDesktopSwitch` absent means Windows kills and restarts the AT at every switch [MS1]. Without the HKLM `Configuration` value, nothing starts on the sign-in screen at all (§2.2). |
| C9 | "A binary running in the SYSTEM session on the secure desktop will be instantly blocked if it lacks a digital signature." | **Not supported; partly wrong** | No Microsoft source says so: neither the AT registration page [MS1] nor the AT security page [MS2]. "SYSTEM session" is wrong: the copy runs as SYSTEM in the user's session (§2.3). What needs a signature is **uiAccess**, and uiAccess "has no effect" for SYSTEM on the secure desktop [MS2]. A uiAccess-manifested binary that fails the signature check does fail to start in the user's session [X3], so the product build must be signed anyway. Whether AtBroker starts an unsigned, non-uiAccess build on the secure desktop is undocumented (**inferred**: yes). E2 checks it. Separately, WDAC and Smart App Control can block unsigned code everywhere when an administrator or a clean install turns them on [MS10], not specifically on the secure desktop. |
| C10 | A self-signed certificate with `New-SelfSignedCertificate`, imported into Trusted Root and Trusted Publishers, signed with `signtool`. | **Mostly holds** | Trusted Root (LocalMachine) is the requirement [MS3], [MS4], [X2]. **Trusted Publishers is not**: no uiAccess source names it, and adding it widens what the certificate is trusted for. Leave it out unless E1 shows otherwise. `signtool` or `Set-AuthenticodeSignature` both work. NVDA's own instructions do exactly Root, nothing else [N7]. |
| C11 | `bcdedit -set TESTSIGNING ON`. | **Wrong / irrelevant** | TESTSIGNING "controls whether Windows ... will load any type of test-signed kernel-mode code" [MS5]. Turning it on requires turning off Secure Boot ("The value is protected by Secure Boot policy") and can involve BitLocker. It puts a "Test Mode" watermark on the desktop, and it lets any certificate sign drivers [MS6]. It does nothing for a user-mode program or for uiAccess, whose check is "a trusted root in the local machine" store [MS4]. Never do it. |
| C12 | `bcdedit /set nointegritychecks on`, with Secure Boot off. | **Wrong / harmful** | "Disables integrity checks. Cannot be set when secure boot is enabled. This value is ignored by Windows 7 and Windows 8" [MS5]. It concerns boot and kernel code integrity, not Authenticode trust for uiAccess. Never do it. |
| C13 | `uiAccess="true"` "lets the binary bypass UAC UI restrictions and interact with high-privilege windows". | **Mostly holds** | It bypasses UIPI, the window-message barrier, not UAC elevation. For an administrator the process runs at High, for a standard user at Medium+16, which Microsoft says "cannot access elevated UI". It never reaches System-level UI [MS2]. It plays no part on the secure desktop. |
| C14 | It must be in Program Files or System32; "from a user directory Windows strips its high-privilege capabilities". | **Location holds; the consequence is disputed** | Program Files and Program Files (x86) with subfolders, and the Windows folder minus the user-writable subfolders [MS3], [MS4]. Microsoft says the process "is started without UIAccess rights". Field reports say the start fails with "A referral was returned from the server" [X3]. E1 decides. |
| C15 | Framing: register it and you are done. | **Incomplete** | Registration is the smallest part. Also needed: the `Configuration` values; `TerminateOnDesktopSwitch=0` with its duties (idle off-desktop, multiple copies, notify Windows); a secure mode that keeps a SYSTEM process from opening files, shells or logs; lock-screen mode in the user's copy; one copy per desktop; and the hook on its own desktop. |

---

## 5. Proposed design for Okur

### 5.1 What Okur does today (commit `8a01219`)

**`crates/okur/src/main.rs`, `Commands::Run`.** The `run` command:

- loads settings from `SettingsFolder::user()`;
- starts `SpeechThread`, `ProcessUiHost` (`okur-ui.exe`), `WindowsActions`, the keyboard hook
  and the event listener;
- prints a note when `elevation::is_elevated()` is false;
- waits for `tokio::signal::ctrl_c()`, then says "Okur exiting.".

It is a console program: it uses `println!` throughout.

**Settings** (`crates/okur-platform/src/settings_file.rs`):

- TOML files in `%APPDATA%\Okur`: `settings.toml`, `applications/*.toml`, `sites/*.toml`;
- the engine writes through `set_settings_store`;
- as SYSTEM, `%APPDATA%` would resolve to the system profile.

**`crates/okur-platform/src/hook/windows.rs:148-156`.** The hook thread calls
`OpenDesktopW("default")` and `SetThreadDesktop` before `SetWindowsHookExW(WH_KEYBOARD_LL)`.
This came in with commit `05732dd` (2026-10-13, before the audit), with no reason given. On the
secure desktop this would hook the user's desktop instead of `Winlogon`. It must go.

**Missing pieces.**

- No embedded manifest, no signature (observed on `target\debug\okur.exe`).
- No single-instance rule, no session or lock tracking, no `SPI_SETSCREENREADER`, no
  end-session handling.
- No quit command.

**`okur-ui`.**

- It is a GUI-subsystem program, started with `Command::new` (`CreateProcess`) from Okur's
  folder (`ui_host.rs`).
- It uses Win32 controls, including RichEdit.
- It opens no files or links (grep finds no `ShellExecute` or file dialog).
- `helper-ui-architecture.md` already decided that Okur gets UIAccess and `okur-ui` stays a
  normal process.

**Commands.**

- The `Command` enum and the binding table are in `crates/okur-core/src/command.rs`, for
  example `bind(Key::Escape, JAWS, RefreshScreen)`.
- Dispatch is in `crates/okur-core/src/engine/commands.rs`.
- Message wording is in `messages.rs`.

### 5.2 Components and binaries

**Source layout.** The code of `crates/okur` moves into a library. Two binaries call it.

| Binary | Subsystem | Manifest | Signed | Use |
| --- | --- | --- | --- | --- |
| `okur.exe` (existing) | console | `asInvoker`, `uiAccess="false"`, supported OS | dev: no | The developer CLI: `okur run`, `parity`, `snapshot`, `keys`, `doctor`. Runs from `target\` and from the lab folders on old-pc, as today. |
| `okur-sr.exe` (new) | windows (no console) | `asInvoker`, `uiAccess="true"`, supported OS, DPI | yes | The installed screen reader. Only ever runs from `C:\Program Files\Okur`. |
| `okur-ui.exe` (existing) | windows | `asInvoker`, `uiAccess="false"` | yes | Draws Okur's windows. |
| `okur-setup.exe` (new) | windows | `requireAdministrator` for the elevated part (see below) | yes | Install, uninstall, repair, and the administrator-only settings (sign-in-screen start, all-users start, copying settings to secure screens). Installed next to the others; it is the uninstaller. |

`okur-sr.exe` cannot run from `target\`, because UIAccess needs Program Files. That is why
the dev CLI keeps no UIAccess. NVDA splits its builds the same way (§3.1).

**Manifests from Rust.** In a `build.rs` for `crates/okur`, use per-binary linker arguments:

```text
cargo:rustc-link-arg-bin=okur-sr=/MANIFEST:EMBED
cargo:rustc-link-arg-bin=okur-sr=/MANIFESTINPUT:<crate>\manifests\common.manifest
cargo:rustc-link-arg-bin=okur-sr=/MANIFESTUAC:level='asInvoker' uiAccess='true'
```

The same goes for `okur` with `uiAccess='false'`. `/MANIFESTUAC` takes the `level=... uiAccess=...`
fragment [MS11]. Keep `trustInfo` out of the input manifest so that the linker's UAC fragment
does not collide with it. The `embed-manifest` crate (1.5, `ui_access(bool)`,
`requested_execution_level`) does the same with less hand-work.

Check the result with:

```text
"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\mt.exe" -inputresource:okur-sr.exe;#1 -out:con
```

(`mt.exe` is installed, observed.) Avoid `unwrap` and `expect` in `build.rs`: the gate scans
for them.

**Starting a UIAccess binary.**

- Anything that starts `okur-sr.exe` must go through `ShellExecuteExW`, not
  `std::process::Command` (§2.6 item 4). That covers `okur-setup`, `okur.exe --start` and
  Okur restarting itself.
- Okur starts `okur-ui.exe` with a token it has lowered on purpose: a duplicate of its own,
  with `TokenUIAccess` cleared and the integrity set to Medium (lowering is allowed).
  Otherwise the child inherits Okur's High integrity and UIAccess (**inferred**; check with
  `okur doctor --pid`). That keeps the decision in `helper-ui-architecture.md`: our harness can
  read Okur's dialogs.

**Where the hook runs.** In `okur-sr.exe`, on its own thread, on the **process's own
desktop**. Delete the forced `default` attach. A process started on `Winlogon` gets
`Winlogon` for its threads. A process started by the user gets `Default`.

**Signing in development** covers `okur-sr.exe` (required), plus `okur-ui.exe` and
`okur-setup.exe` (for consistency). At release, sign every shipped PE file and the
downloadable installer.

**`elevation.rs`.** Today it reports "not elevated" for a UIAccess process, because the token
is the limited one. It must also read `TokenUIAccess`; the startup note should depend on both.

### 5.3 Processes and privileges

| Process | Account | Session / desktop | Integrity | UIAccess | Started by |
| --- | --- | --- | --- | --- | --- |
| `okur-sr.exe` (user copy) | the user | user's session, `WinSta0\Default` | High (administrator) or Medium+16 (standard user) | yes | Ease of Access after sign-in, the Run-key resolver, or the user (Win+R `okur`) |
| `okur-sr.exe` (secure copy) | SYSTEM | user's session, `WinSta0\Winlogon` | System | not used | AtBroker: `StartExe` + `--ease-of-access` |
| `okur-ui.exe` (user) | the user | `Default` | Medium (lowered) | no | user copy |
| `okur-ui.exe` (secure) | SYSTEM | `Winlogon` | System | no | secure copy, **only** for the quit question if J4 shows JAWS asks it there |
| `okur-setup.exe` | administrator | `Default` | High (elevated) | no | the user (it elevates its own worker) |

Each copy knows its context from `GetUserObjectInformation(GetThreadDesktop(), UOI_NAME)`.
`Winlogon` means secure mode, as in NVDA [N5]. One more rule: a copy started with
`--ease-of-access` while the account is SYSTEM and the desktop is not `Winlogon` exits.

### 5.4 Registry and files

**Files.** Under `C:\Program Files\Okur\`: `okur-sr.exe`, `okur-ui.exe`, `okur-setup.exe`,
`secure-settings\settings.toml`, and documentation. The folder inherits Program Files'
permissions, so only administrators can write it.

**Machine registry** (written by `okur-setup` in the 64-bit view):

- `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Accessibility\ATs\okur_okur_v1`:

  | Value | Data |
  | --- | --- |
  | `ApplicationName` | `Okur` |
  | `Description` | `Okur screen reader` |
  | `Profile` | `<HCIModel><Accommodation type="severe vision"/><Accommodation type="mild vision"/></HCIModel>` |
  | `SimpleProfile` | `screenreader` |
  | `ATExe` | `okur-sr.exe` |
  | `StartExe` | `C:\Program Files\Okur\okur-sr.exe` |
  | `StartParams` | `--ease-of-access` |
  | `TerminateOnDesktopSwitch` | DWORD 0 |

  Not written: `SecureDesktopAccommodation` (we want Okur itself there) and
  `CopySettingsToLockedDesktop` (§5.7). `PassiveAutoStartBehavior` is left unset, as JAWS has
  it, until J3 shows whether JAWS comes back by itself after a UAC prompt or an unlock.
- `HKLM\...\Accessibility\Configuration`: `okur_okur_v1` is appended when "start on the
  sign-in screen" is on. Read-modify-write the comma-separated list. Never drop another AT
  (JAWS's name is there on the owner's machines).
- `HKLM\SOFTWARE\Okur`: `InstallDir`, `Version`, `StartAfterSignInAllUsers` (DWORD), and
  `LabSecureSpeechLog`, a lab-only switch (§5.7). Only administrators can write any of them.
- `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\okur.exe`: the default value is
  `C:\Program Files\Okur\okur-sr.exe`, so Win+R `okur` starts the screen reader, as `JAWS2025`
  starts JAWS.
- `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`, value `Okur`:
  `"C:\Program Files\Okur\okur-sr.exe" --sign-in` (see §5.5).
- `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Okur`:
  - `DisplayName`, `DisplayVersion`, `Publisher`;
  - `InstallLocation`;
  - `UninstallString` = `"C:\Program Files\Okur\okur-setup.exe" uninstall`;
  - `QuietUninstallString`, `EstimatedSize`, `DisplayIcon`.

**Per-user state.**

- `%APPDATA%\Okur\` holds the settings, as today.
- `HKCU\...\Accessibility\Configuration` holds `okur_okur_v1` while "this user" is set to
  Always.
- `HKCU\SOFTWARE\Okur\StartAfterSignIn` holds `always`, `never` or `all-users`; absent means
  JAWS's default, Always.

### 5.5 Start options, with JAWS's choices

Okur's settings show JAWS's three options with JAWS's defaults. Administrator options are
greyed out and silent for standard users, as JAWS's help describes.

| Okur option | Default | Where it lives | Mechanism |
| --- | --- | --- | --- |
| Start Okur on the sign-in screen (administrators) | on | HKLM `Configuration` | AtBroker starts the secure copy at boot, sign-out, lock and Ctrl+Alt+Delete, and starts it for UAC prompts whenever Windows knows Okur runs. |
| Start Okur after sign-in for all users (administrators) | off | `HKLM\SOFTWARE\Okur\StartAfterSignInAllUsers` | The Run-key resolver. |
| Start Okur after sign-in for this user: Always / Never / Use the all-users setting | Always | HKCU `Configuration` (Always) and `HKCU\SOFTWARE\Okur\StartAfterSignIn` | Always: Windows starts Okur right after sign-in through Ease of Access, as NVDA does. Never and "all users": Okur is removed from HKCU `Configuration`, and the Run-key resolver decides. |

**The Run-key resolver.** `okur-sr.exe --sign-in` runs at every sign-in, for every user. It
reads the user's choice; if that is absent, it uses Always. It starts the screen reader only
when:

- the choice is Always and Okur is not already running on this desktop (Ease of Access may
  have started it first), or
- the choice is "all users" and the machine value is on.

Otherwise it exits before making any sound. This also covers a new user profile, which has no
HKCU `Configuration` yet.

**Timing.** Explorer runs Run keys late after sign-in; Ease of Access starts ATs earlier
(**inferred**). E6 measures both against JAWS before this is settled.

**Rules for every automatic start** (`--ease-of-access` or `--sign-in`):

- If both Shift keys are held (`GetAsyncKeyState(VK_LSHIFT)` and `VK_RSHIFT`), exit
  silently. This is JAWS's documented escape hatch [J1], and it is Okur's own way out if a bad
  build ever makes the sign-in screen unusable.
- If a copy already runs on this desktop, exit silently (NVDA's `--ease-of-access` rule [N3]).
- When the copy actually starts, and is not on the secure desktop, notify Ease of Access: the
  AccessibilityTemp value 3, then Win+U, releasing held Shift, Ctrl and Alt first as NVDA does
  [N1]. Then Windows knows Okur runs and starts the secure copy for UAC prompts. On exit,
  notify with 2.

### 5.6 Single instance, the control channel and the hand-over

- **Mutex.** `Local\Okur-<desktop name>`, taken at startup. `Local\` scopes it to the session
  and the name scopes it to the desktop, so the user copy and the secure copy can coexist, as
  Microsoft requires [MS1]. If another copy is exiting, wait up to 2 s for the mutex (NVDA
  [N3]).
- **Control pipe.** `\\.\pipe\okur-<session id>-<desktop name>`, with an explicit DACL: the
  session's user, SYSTEM and Administrators. Messages: `quit` (no question), `status`, and
  `ping`.
  - **Manual start while a copy runs:** the new copy asks the old one to quit, then takes
    over. NVDA does this [N3]; whether JAWS does is J6.
  - `okur-sr.exe --quit` and `okur.exe quit` send `quit`.
  - The installer sends `quit` to every copy in every session.
  - The pipe replaces NVDA's `WM_QUIT` and `ChangeWindowMessageFilter` trick, which lets any
    process close the screen reader.
- **Leaving a desktop.**
  - **Detection.** Every copy listens for `EVENT_SYSTEM_DESKTOPSWITCH`. It compares the input
    desktop's name (from `OpenInputDesktop`) with its own. A failure to open it counts as
    "another desktop".
  - **When its desktop loses the input**, the copy immediately:
    - stops speech;
    - forgets held modifiers and the held JAWS key: no key-up events will come;
    - closes its `okur-ui` windows;
    - releases exclusive devices (braille, later);
    - clears the braille display (lesson of GHSA-8f8q);
    - sleeps: events are dropped and the hook passes every key on.
  - **When the input comes back**, it reads the focus again.
  - **What the user copy says** when the secure desktop comes and goes is JAWS's wording
    from J1, not NVDA's "Secure Desktop". Until J1 is recorded it says nothing, and a known-gap
    test records the gap.
- **At sign-in.**
  - The secure copy's desktop loses the input, so it stops speaking at once; it must never
    talk over the user's copy.
  - The user's copy starts (§5.5).
  - With `TerminateOnDesktopSwitch=0`, the secure copy most likely stays alive, asleep, and
    serves later UAC prompts and locks without a cold start (**inferred**). E4 checks this.
  - If Windows does restart it per prompt, the cold start becomes a measured budget: key to
    first speech on a UAC prompt.
- **Shutdown.**
  - Call `SetProcessShutdownParameters(0x100, SHUTDOWN_NORETRY)` at start [N11].
  - Create a hidden **top-level** window for `WM_QUERYENDSESSION` and `WM_ENDSESSION`.
    Message-only windows "do not receive broadcast messages".
  - On end-session, stop speech and exit fast; never block.
- **The screen-reader flag.** Set `SPI_SETSCREENREADER` on start, clear it on exit (NVDA
  [N3]). Applications use it to adapt.

### 5.7 Secure-desktop mode (the SYSTEM copy)

**Principle: speak and read; do nothing else.** A SYSTEM process on the sign-in screen is
reachable by anyone who can touch the keyboard. Every NVDA advisory in §3.1 came from a
feature that did more than read.

| Area | On the secure desktop |
| --- | --- |
| Speech, the keyboard hook, native focus reading, typing echo, reading keys (line, word, character, Say All in edits), Insert+T, Insert+Tab, Insert+B, Insert+F12, Insert+Escape | **On** |
| Password fields (UIA `IsPassword`) | Never speak the characters typed; say what JAWS says (J7) |
| Insert+F4 | JAWS's behavior there (J4). If JAWS asks the question, `okur-ui` shows it with buttons and static text only |
| Settings: reading | Only from `C:\Program Files\Okur\secure-settings\settings.toml`, parsed against a whitelist (voice, rate, pitch, volume, punctuation, typing echo, verbosity, keyboard layout and JAWS key). Out-of-range values fall back to defaults. Application and site layers are never copied: `sites/*.toml` names the user's web sites |
| Settings: writing, the settings dialogs, Quick Settings | **Off** (no `SettingsStore`). The command says JAWS's wording for an unavailable action if J-observation finds one, else a message from `messages.rs` |
| Virtual Viewer, help windows, Speech History viewer, element lists | **Off**. They show content, and the viewers can carry links. A list of what is on the secure desktop is harmless, but the first version keeps `okur-ui` off entirely |
| Opening files, folders, URLs, help, browsers, shells; spawning any process except `okur-ui.exe` from its own folder | **Never** (no `ShellExecute`, no `Command::new` other than that one path) |
| Clipboard | **Never** read or written |
| Logging (`--debug-log`, `--speech-log`, `tracing` files), crash dumps | **Never**. They would record passwords. A lab override is below the table |
| Network: updates, AI features, remote help | **Never** |
| Extensions (future SDK) | **None**, not even ones an administrator copied, until a reviewed design exists. NVDA's advisories show the risk |
| DLL loading | Call `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)` at process start, in both copies. Never load from the current directory or from user-writable paths |
| Speech history | Not kept (its viewer is off); anything in memory is cleared at every desktop switch |

**Getting settings to the secure screens.** This follows NVDA's model, the safer one (§2.4):

- **At install.** `okur-setup` copies the installing user's `settings.toml` through the
  whitelist into `secure-settings\`.
- **Later.** An Okur setting, "Use my current settings on the sign-in screen and secure
  screens", runs `okur-setup copy-secure-settings` elevated. It is only for administrators,
  as in NVDA. Whether JAWS follows the user's settings automatically (J8) decides whether to
  add Windows' `CopySettingsToLockedDesktop` route later, for the whitelist only, with strict
  parsing.

**Lab switch.** `HKLM\SOFTWARE\Okur\LabSecureSpeechLog` is set only on old-pc, by the
engineer. It lets the secure copy append its utterances to a file in an
administrators-only folder. It exists so that old-pc can record what Okur says on the sign-in
screen. Prefer the Speech Recorder voice (`crates/okur-speech-recorder`), which needs no Okur
code path: point the secure settings' SAPI 5 voice at it. Never ship the switch turned on.

### 5.8 Lock-screen mode (the user copy)

**Detection.**

- At startup, ask the session directly with `WTSQuerySessionInformation(WTSSessionInfoEx)`,
  which reports the lock state (lesson of GHSA-72mj).
- Then follow `WTSRegisterSessionNotification` (`WTS_SESSION_LOCK` and `WTS_SESSION_UNLOCK`).
- The foreground window is not a reliable lock signal [N12].
- Treat the OOBE and PIN-reset desktops as locked [N13].

**While locked:**

- Allow only an explicit list of commands, in the spirit of NVDA's "safe scripts": focus,
  title, time, battery, reading the focused control, and moving within the lock screen's own
  windows.
- Refuse the Window List (Insert+F10), the tray list (Insert+F11), the Virtual Viewer, the
  speech history, element lists, settings, clipboard commands and anything that opens
  `okur-ui`.
- **Drop events from every process except the lock screen and the windows it shows.**
  Otherwise a chat app's live region behind the lock screen would be read aloud to whoever
  stands at the locked computer. This needs a tested rule for "above the lock screen" (risk
  R4).

### 5.9 The quit command

- **Binding.** `bind(Key::F4, JAWS, ShutDown)`, from `JAWSKey+F4=ShutDownJAWS` in
  `Default.JKM`. Accept `ShutDownJAWS` as the parsed name, the way `RefreshScreen` is
  accepted. Layered `JAWSKey+Space, F4` (`RestartWithoutDump`) can come later.
- **Setting.** `confirm_exit`, on by default: JAWS's "Prompt to Confirm When Exiting JAWS".
- **With the setting on.** The engine describes the question as a dialog spec in okur-core.
  Its title, text, buttons, default button and the speech on opening all come from scenario
  571's evidence. `okur-ui` shows it.
  - Escape or the cancel button closes it and puts the focus back where it was. 571's step s4
    (Insert+Tab in Notepad) checks this.
  - The confirm button, or Enter if it is the default, starts the exit.
- **With the setting off.** Exit at once.
- **The exit.** The engine does not end the process. It hands an `ExitRequested` signal to the
  host loop in `main`, which then:
  1. unhooks the keyboard first, so no key is ever swallowed while exiting;
  2. releases any modifier Okur consumed;
  3. speaks JAWS's exit words, if JAWS says any (a new scenario, see J2), and waits for speech
     to end, at most about 1.5 s;
  4. stops the listeners and `okur-ui`;
  5. notifies Ease of Access with 2 (user copy only);
  6. clears `SPI_SETSCREENREADER`;
  7. releases the mutex and exits.

  `Ctrl+C` in the console CLI goes down the same path.
- **On the secure desktop.** JAWS's behavior there (J4) decides. If the secure copy may quit,
  Windows will not bring it back until the next desktop switch. That is the documented
  behavior, and the user hears whatever JAWS's equivalent case gives them.

### 5.10 Installer and uninstaller (`okur-setup.exe`)

**Install.**

1. Run unelevated first. Re-launch the same program elevated with `ShellExecuteEx` and the
   `runas` verb for the machine work. The unelevated parent waits.
2. Ask every running copy to quit over its pipe. Wait, then terminate stragglers; the secure
   copy runs as SYSTEM, and an elevated administrator can end it.
3. Copy the files.
   - A running executable can be renamed but not deleted. Rename it to `*.old` and schedule
     deletion with `MoveFileEx(..., MOVEFILE_DELAY_UNTIL_REBOOT)`, as NVDA's `tryRemoveFile`
     does.
   - Install the UIAccess `okur-sr.exe` only if its signature verifies on this machine
     (`WinVerifyTrust`). Otherwise install the no-UIAccess build under the same name and tell
     the user that elevated windows will be silent. NVDA does the same [N7].
4. Write the registry of §5.4, then seed `secure-settings\`.
5. Ask the start options with JAWS's defaults; a silent install uses the defaults.
   - The HKCU parts are written by the **unelevated** parent, so they land in the right user's
     hive even when an administrator elevated over the shoulder.
6. Start Okur from the unelevated parent with `ShellExecuteEx`, so that it runs as the user
   and not with the elevated token. NVDA hides "Start NVDA" when its installer runs elevated
   [N9]. No restart is needed: Okur installs no drivers (**inferred**; JAWS asks for one [J6]).

**Uninstall.**

- Stop all copies.
- Remove Okur's name from HKLM `Configuration` and from the current user's HKCU
  `Configuration`.
- Delete the AT key, the Run value, App Paths and the Uninstall key.
- Delete the files, renaming and scheduling at reboot where they are locked.
- Delete `secure-settings\`.
- Leave `%APPDATA%\Okur`, as NVDA does.
- Other users' HKCU `Configuration` may still name Okur. That is harmless, but E5 confirms
  that Windows ignores a name without an AT key.

**Upgrade.** Install over a running copy, with the steps above. The user copy is restarted at
the end. The secure copy comes back with the new code at the next desktop switch.

**Portable mode.** `okur.exe run` from any folder: the no-UIAccess build, with no automatic
start and no secure screens. Elevated windows can be read only if the user runs it as
administrator. This is what developers use today, and it stays.

**Later.** A signed MSI (WiX) wrapping the same steps, for administrators who deploy with
Group Policy. JAWS offers `/Type Silent`, `/Type SilentUninstall` and others [J6].
`okur-setup` should take `install --silent` and `uninstall --silent` from the start.

### 5.11 Development signing, step by step

What it changes: one certificate in the owner's personal store on the dev box, and its public
part in `LocalMachine\Root` on old-pc. No `bcdedit`, no test-signing mode, no Secure Boot
change, no Trusted Publishers. **Both steps need the owner's yes** (Q1).

**On the dev box**, in Windows PowerShell, unelevated, once:

```powershell
$cert = New-SelfSignedCertificate -Type Custom `
  -Subject "CN=Okur Development (not for release)" `
  -FriendlyName "Okur development code signing" `
  -KeyUsage DigitalSignature `
  -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}") `
  -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
  -KeyExportPolicy NonExportable `
  -NotAfter (Get-Date).AddMonths(12) `
  -CertStoreLocation Cert:\CurrentUser\My
$cert.Thumbprint          # write it down: <THUMB>
Export-Certificate -Cert $cert -FilePath "$env:USERPROFILE\okur-dev-codesign.cer"   # public part only
```

What the parameters do:

- **The extensions:**
  - `1.3.6.1.5.5.7.3.3` limits the certificate to code signing.
  - `2.5.29.19={text}` marks it an end entity, not a CA [MS12]. Even trusted as a root, it can
    sign code but not issue certificates.
- `NonExportable` keeps the private key on the dev box.

**After each release build**, on the dev box:

```powershell
& "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe" sign /fd SHA256 /sha1 <THUMB> /s My `
  target\release\okur-sr.exe target\release\okur-ui.exe target\release\okur-setup.exe
# or: Set-AuthenticodeSignature -FilePath target\release\okur-sr.exe -Certificate (Get-Item Cert:\CurrentUser\My\<THUMB>) -HashAlgorithm SHA256
```

A timestamp (`/tr http://timestamp.digicert.com /td SHA256`) is optional in development.
Without one, the signature stops verifying when the certificate expires: re-sign, or make a
new certificate. `Get-AuthenticodeSignature` shows `UnknownError` on the dev box, because the
root is not trusted there. That is expected.

**On old-pc**, elevated, once:

```powershell
Import-Certificate -FilePath C:\Users\ailabs\sr-harness\okur-dev-codesign.cer -CertStoreLocation Cert:\LocalMachine\Root
Get-AuthenticodeSignature "C:\Program Files\Okur\okur-sr.exe"     # Status: Valid
```

**Undo.**

- On old-pc: `Remove-Item Cert:\LocalMachine\Root\<THUMB>`.
- On the dev box: `Remove-Item Cert:\CurrentUser\My\<THUMB> -DeleteKey`.

**Risks and how they are scoped.**

- **Anyone holding the private key can make code that old-pc trusts** for UIAccess, and as
  "signed" generally. The scoping:
  - the key cannot be exported and lives only on the dev box;
  - the certificate is code-signing-only and not a CA;
  - it expires in 12 months;
  - only old-pc, the lab, trusts it, not the dev box.
- If the owner later installs Okur on the dev box itself (Q1), the same root goes there. The
  marginal risk is small, since the key already lives there.
- **Never** copy the `.pfx` anywhere. **Never** put the dev root on a machine someone else
  uses.

### 5.12 Release signing

What a real certificate changes:

- Every Windows machine trusts the chain through Microsoft's root program, so UIAccess works
  without touching the root store.
- Smart App Control and enterprise publisher rules accept it.
- SmartScreen builds reputation for the publisher over time. Since 2024 an EV certificate no
  longer bypasses SmartScreen: "EV-signed files now go through the same reputation-building
  process as OV certificates" [MS13].

The routes, from Microsoft's comparison [MS13]:

| Route | Cost | Fits Okur? |
| --- | --- | --- |
| **SignPath Foundation** (free for OSI-licensed open source; NVDA signs through SignPath) | free | Yes: Okur is Apache-2.0 on GitHub with CI. But the certificate is "issued to SignPath Foundation", so "SignPath Foundation" is the publisher users see in Properties and UAC. Binaries must be built by CI from the repository ("origin verification") [X5]. |
| **OV certificate** from a public CA | $150–300 a year | Yes. The key must sit on a hardware token or cloud HSM since June 2023 [MS13]. The publisher is the owner's name or company. Signing from CI needs a cloud-HSM product. |
| **Azure Artifact Signing** (formerly Trusted Signing) | about $9.99 a month | Only for organizations in the USA, Canada, the EU or the UK, and individuals in the USA or Canada [MS13]. An individual in Turkey cannot use it. |
| **EV certificate** | $400+ a year | No longer worth it for SmartScreen [MS13]. |

Timestamp every release signature (RFC 3161, `/tr ... /td SHA256`), so that it stays valid
after the certificate expires.

### 5.13 macOS, so that nothing here blocks it

- **Login window.** A third-party screen reader cannot replace VoiceOver there.
  - Pre-login LaunchAgents (`LimitLoadToSessionType=LoginWindow`) run "as root when the login
    window context starts up" and are terminated when the user logs in.
  - Apple's DTS adds that "A pre-login agent runs in a separate process from that which
    displays the login UI", that the accessibility APIs are "unlikely to work in this context",
    and that the password field uses secure event input, so keys there cannot be captured
    [X7].
- **FileVault unlock.** It happens before the encrypted volume holding Okur is unlocked, so no
  third-party code runs there (**inferred**). Apple's own VoiceOver support there is Apple's
  business.
- **After login.** A LaunchAgent in the Aqua session, or `SMAppService` (macOS 13+). Each user
  grants Accessibility and Input Monitoring once.
- **Design consequence.** Two platform traits in `okur-platform`:
  - `Autostart`. Windows implements all three of JAWS's options. On macOS the sign-in-screen
    option is reported unavailable, and Okur's settings say so ("VoiceOver reads the login
    window").
  - `SessionMonitor`, reporting normal, locked or secure. macOS sends lock and unlock through
    its session notifications.

  Lock-screen mode (§5.8) is cross-platform. Secure-desktop mode (§5.7) exists only on
  Windows.

---

## 6. Milestones, experiments and risks

### 6.1 How old-pc can be used for this

Facts that shape the plan (memory notes and `verification-loop.md`):

- `opendesk exec` runs as `ailabs` in session 1, unelevated. Elevation works without a prompt
  there: "old-pc UAC does not prompt".
- **After a restart or a sign-out nobody relaunches `opendesk serve`** until the owner is there.
- OpenDesk cannot type on the secure desktop: it runs as the user.
- The dev box itself shows `ConsentPromptBehaviorAdmin=0` (elevate without prompting) and
  `PromptOnSecureDesktop=0` (observed), so **no UAC prompt ever reaches a secure desktop on the
  dev box**. old-pc must be checked (E0).

So the plan sorts experiments into three kinds:

- **(A) Automatic**: the engineer alone, over OpenDesk.
- **(B) With the owner at old-pc**: anything that needs a restart, a sign-out, a password or
  answering a UAC prompt. Each such session has a short script.
- **(C) A VM.** Optional, if the owner wants unattended secure-screen tests (Q3). Hyper-V
  needs Windows Pro (the dev box is Home). A VirtualBox or Hyper-V guest can be sent keys on
  its sign-in screen from the host, and the Speech Recorder log can be read afterwards.

**Never test a new secure-desktop build on the dev box first.** A hook that swallows keys on
the sign-in screen can lock the owner out of their own computer (R1).

### 6.2 Experiments (do these first, in this order)

| # | Kind | What to do | What to observe | Undo |
| --- | --- | --- | --- | --- |
| E0 | A | On old-pc, unelevated, `reg query` the keys of §2.2 (`ATs`, HKLM/HKCU/`Session1` `Configuration`). Also read `...\Policies\System` (`EnableLUA`, `ConsentPromptBehaviorAdmin`, `PromptOnSecureDesktop`, `EnableSecureUIAPaths`), `(Get-CimInstance Win32_OperatingSystem).Caption` and the voice tokens. | Baseline: is JAWS registered as on the dev box, does UAC prompt, is the edition Pro (VM possible)? | Nothing changed. |
| E1 | A (owner's yes for the root) | Build `okur-sr.exe` (UIAccess) and `okur doctor`, which prints session, desktop, account, integrity, `TokenUIAccess`, elevation, signature status and AT registration. Start `okur-sr.exe --doctor-exit` (or `okur doctor` on its pid) through `Start-Process` (ShellExecute) in five cases: **(a)** unsigned, in `C:\Users\ailabs\sr-harness`; **(b)** signed, root not yet trusted, in `C:\Program Files\Okur`; **(c)** signed, root in `LocalMachine\Root` only, in Program Files; **(d)** as (c) but in sr-harness; **(e)** as (c) but started with `std::process::Command`. Then with (c) running as the screen reader: open `Start-Process notepad -Verb RunAs` and type in it. Repeat the elevated-Notepad check with today's `okur.exe run --debug-log`. | (a), (b), (d): does it fail with "A referral was returned from the server", or start without UIAccess? (c): `TokenUIAccess=1`, integrity High. (e): error 740? Elevated Notepad: does the debug log show the keys and does UIA read the edit? Today's Okur: are keys typed in elevated Notepad missing from its log? | Remove the root (§5.11); delete `C:\Program Files\Okur`; close Notepad. |
| E2 | B (or C) | Write the AT key (§5.4) by hand. Save HKLM `Configuration` to a file, then set it to `okur_okur_v1` alone, **so that JAWS and Okur never both start on the sign-in screen**. Switch old-pc's `ConsentPromptBehaviorAdmin` to 5 and `PromptOnSecureDesktop` to 1 (owner's yes, Q2). With the owner present: Win+L, a key, listen. Then Ctrl+Alt+Delete, listen. Then a UAC prompt (`Start-Process cmd -Verb RunAs` from the user session), listen, answer No. Do it for three builds: unsigned without UIAccess, signed with UIAccess, unsigned with UIAccess. Set Okur's secure voice to the Speech Recorder voice and read its log afterwards. | Does AtBroker start each build as SYSTEM on `Winlogon` (`okur doctor` output from a `--doctor-log` to an administrators-only path, lab switch on)? Is speech heard? | Restore `Configuration` from the saved file; restore the two UAC values; delete the AT key if done. |
| E3 | A, after E2 | On the secure desktop (or as SYSTEM on `Default` with Sysinternals `psexec -s -i 1`, a download that needs the owner's yes): OneCore and SAPI 5 speech, UIA reading of LogonUI and consent.exe. | Which engines work as SYSTEM; how long from start to first speech. | Nothing to undo. |
| E4 | B | Lock and unlock, three times; a UAC prompt; sign out and in. After each, list `okur-sr.exe` processes (session, account) elevated over OpenDesk. | Does the secure copy live on after sign-in and after each prompt, or is it started fresh each time? Does anything speak over anything else? Is any key lost or stuck after returning (Insert, Shift)? | Nothing to undo. |
| E5 | A | With Okur registered: open Control Panel > Ease of Access Center > Change sign-in settings. Toggle Okur's "At sign-in" and "After sign-in" boxes one at a time (the page elevates without a prompt on old-pc) and `reg query` after each. Also remove the AT key while HKCU still names Okur, then lock and unlock. | Is Okur listed? Which value does each box change: confirm NVDA's mapping (§2.2) or Microsoft's text? A dangling name: any error? | Put the boxes back; restore the values from E0. |
| E6 | B | Sign in, with Okur started by (a) HKCU `Configuration` and (b) the Run-key resolver. Then JAWS alone, as is. | Seconds from pressing Enter on the password to first speech, for each. | Restore the start options. |

**JAWS observations.** Black-box: keys in, speech out. Record with the Speech Recorder log
where JAWS's secure copy uses that voice; otherwise the owner listens and the result is
written as "observed by the owner, date".

| # | Kind | Question | How |
| --- | --- | --- | --- |
| J1 | B | What JAWS says on the sign-in screen at boot, on Ctrl+Alt+Delete, on the credential screen after a lock, and on a UAC prompt. What the **user's** JAWS says when the secure desktop comes and when it goes. | The E2/E4 session, with JAWS's own setup restored. |
| J2 | A | Insert+F4's question (571), then **Enter**: what JAWS says while exiting, and that the keys are normal afterwards. | Import 571. Add a scenario "Insert+F4, Enter", recorded through the Speech Recorder log, because FSAPI's history ends with JAWS. Restart JAWS afterwards (`jaws-process.ps1`). Last in its batch, like 571. |
| J3 | B | After quitting JAWS with Insert+F4, does a lock and unlock or a UAC prompt bring JAWS back by itself? | Decides `PassiveAutoStartBehavior`. |
| J4 | B | Insert+F4 on the sign-in screen: is there a question, does JAWS quit? | Decides §5.9's secure-desktop branch. |
| J5 | B | Both Shifts held at sign-in: JAWS does not start [J1]. | Confirms the documented escape hatch. |
| J6 | A | Starting JAWS (Win+R `jaws2025`) while it runs: what does it say or do? | Decides the replace-or-ignore rule of §5.6. |
| J7 | B | Typing in the password field on the sign-in screen (type `abc`, then clear the field; no real password): what does JAWS say per character? | Decides password echo. |
| J8 | B | Change the user's JAWS rate, lock, and listen on the credential screen: same rate as the user? | Decides whether secure settings follow the user automatically. |
| (opt.) | A | Which registry values change when JAWS's three Start options are toggled in Basics > Automatically Start JAWS. | Black-box observation of system state, not of JAWS files. Useful but not needed: Okur uses Windows' documented mechanism anyway. Skip it if the owner prefers. |

### 6.3 Milestones, smallest first

Each milestone is merged when the gate passes and its check below is pasted into the handoff
from a tool's output.

**M0. Facts.** E0 and E1, plus `okur doctor` as a new harness command.
- *Done when:* E1's table is filled from `okur doctor` output on old-pc, and §4's C9 and C14
  verdicts are updated with it.

**M1. Process hygiene, user session.**
- *Scope:*
  - the library split and `okur-sr.exe` (GUI subsystem; manifest per §5.2);
  - the hook on its own desktop;
  - the mutex, control pipe and `--quit`;
  - `SPI_SETSCREENREADER`, shutdown parameters and the end-session window;
  - the Shift+Shift escape;
  - `elevation.rs` with `TokenUIAccess`;
  - `okur-ui` started with the lowered token.
- *Check:*
  - Hermetic tests: argument handling, the instance rules (`--ease-of-access` with a running
    copy exits; a manual start replaces), the Shift+Shift rule (mock key state).
  - On old-pc: start twice and see one process; `okur-sr.exe --quit` and see none; sign out
    (owner) and see no "apps are blocking" screen.
  - Listen: no console window flashes.

**M2. Insert+F4.**
- *Scope:* the J2 scenario and 571 imported; `ShutDown` bound to JAWSKey+F4; `confirm_exit`;
  the dialog spec; the exit sequence.
- *Check:*
  - `okur parity --only 571` passes, along with the exit scenario.
  - On old-pc with the engineer's `okur run`: Insert+F4, Escape, Insert+Tab reads Notepad.
  - Then Insert+F4, Enter: the process is gone, and `okur keys`' modifier guard reports no
    held key.

**M3. Signed UIAccess install on old-pc.**
- *Scope:* `okur-setup install` and `uninstall`, without the sign-in-screen start yet; dev
  signing (§5.11).
- *Check:*
  - `okur doctor` shows UIAccess 1 and High.
  - Okur reads elevated Notepad: speech log lines, with the Notepad text.
  - `Get-AuthenticodeSignature` reports Valid.
  - Apps & features lists Okur, and uninstall removes every value of §5.4 (a `reg query`
    before and after, pasted).

**M4. Start after sign-in.**
- *Scope:* the three options and the Run-key resolver; Ease of Access notify on start and
  exit.
- *Check:*
  - Hermetic tests for the resolver's table: user choice × machine value × already running.
  - E6 with the owner. Listen: Okur speaks after sign-in within the time JAWS needs (E6), and
    a lock or UAC prompt then gets the secure copy (after M6).

**M5. Lock-screen mode.**
- *Scope:* WTS tracking with the initial query; the command allowlist; the event filter.
- *Check:*
  - Hermetic tests: each refused command says the refusal; an event from a non-lock-screen
    process is dropped while locked.
  - With the owner: Win+L, then Insert+F10 is refused and a test chat page's live region
    behind the lock screen is silent; unlock and it speaks again.

**M6. Secure desktop.**
- *Scope:*
  - registration with HKLM `Configuration`;
  - secure mode (§5.7) and the secure settings;
  - desktop-switch handling in both copies;
  - J1, J4 and J7 wording.
- *Check:*
  - Hermetic tests that every command off-limits in secure mode is refused, and that the
    secure settings parser rejects bad values.
  - With the owner, a scripted session: boot to the sign-in screen and hear Okur read the user
    tile and the password field; type `abc` and hear what J7 recorded; Ctrl+Alt+Delete and
    hear the options; a UAC prompt and hear it, then answer with Tab and Enter; lock and
    unlock.
  - After each step, E4's process list.
  - Listen for: no double speech, no stuck Insert, speech stopping the instant the screen
    changes.

**M7. Polish and release.**
- *Scope:* the start options in Okur's settings with JAWS's labels and defaults; "use my
  settings on secure screens"; upgrade-while-running; the silent switches; release signing
  through the chosen route; macOS `Autostart` (after-login only).
- *Check:*
  - An install over a running Okur keeps speech within a few seconds (the owner listens).
  - Uninstall leaves only `%APPDATA%\Okur`.

**Harness work that pays for itself** (fits AGENTS.md §3):

- `okur doctor`.
- `okur keys --desktop <name>`, which sends keys into a desktop the engineer created.
- A lab "test desktop" command. It creates a desktop, runs a fixture and a forced-secure-mode
  `okur-sr` on it as the user, switches to it, and **always switches back** after a timeout.
  This tests desktop-switch handling and secure-mode gating unattended. It is no substitute
  for the real SYSTEM copy.
- A Speech Recorder import for owner-assisted secure-screen sessions, so that those sessions
  produce evidence written by tooling.

### 6.4 Risks

| # | Risk | Mitigation |
| --- | --- | --- |
| R1 | A broken secure copy swallows keys on the sign-in screen and **locks the user out**. | Test only on old-pc and VMs; the Shift+Shift escape; the hook passes keys on whenever the engine is busy (already 150 ms) and while asleep. Narrator (Win+Ctrl+Enter) remains available on the sign-in screen. Keep JAWS installed. Turn "sign-in screen" on for the owner's own machines only after M6 passes. |
| R2 | Two screen readers on the sign-in screen (JAWS and Okur), both taking Insert. | The installer shows which other ATs are in HKLM `Configuration` and asks which one starts there (Q8); the lab tests run with Okur alone. |
| R3 | A SYSTEM process with any route to files, shells, links or logs is a privilege escalation. | §5.7's table is enforced in code paths, not by hiding menus. Hermetic tests per command. A review of every `Command::new`, `ShellExecute`, file write and `okur-ui` use against the secure flag. |
| R4 | Lock-screen leaks: speaking a chat message, a notification or window titles to someone at a locked computer. | The event filter and command allowlist of §5.8, with tests. When the state is unknown, behave as locked. |
| R5 | AtBroker's behavior is undocumented: how often it starts the secure copy, the HKCU `Configuration` semantics, `PassiveAutoStartBehavior`. | E2, E4 and E5 before M4 and M6; record the results in this file. |
| R6 | UIAccess side effects. A High-integrity Okur cannot use COM objects that Medium processes registered in the running object table: AutoHotkey's docs say UIA programs "can only access COM objects registered by other UIA programs" [X4]. That matters for future Office object-model work. | Keep such work in a Medium helper, as with `okur-ui`. |
| R7 | Windows 11 Administrator protection changes elevation; reading elevated windows under it is untested [X1]. | Add a VM test when a Windows 11 machine with the feature is available. |
| R8 | Self-signed builds won't run on Smart App Control machines, and AV heuristics may flag an unsigned keyboard hook. | Release signing (§5.12) before any public download. |
| R9 | An upgrade leaves an old secure copy running old code until the next switch or a restart. | The installer stops all copies (§5.10); `okur doctor` shows the version of each running copy. |

---

## 7. Open questions for the owner

1. **Development certificate.** May the engineer create the non-exportable code-signing
   certificate on the dev box (in your personal store) and trust its public part in old-pc's
   `LocalMachine\Root`? Should the dev box trust it too, once you want an installed Okur
   there?
2. **old-pc's UAC.** May old-pc's UAC be set to prompt on the secure desktop
   (`ConsentPromptBehaviorAdmin=5`, `PromptOnSecureDesktop=1`) during secure-screen sessions
   and put back afterwards? And may JAWS's sign-in-screen start on old-pc be turned off while
   Okur is tested there?
3. **Secure-screen testing.** Short in-person sessions at old-pc (each milestone has a
   script), a VM on a Windows Pro machine, or both? A VM needs a Pro host or VirtualBox, and a
   test account in the VM.
4. **Release signing.** SignPath Foundation (free; "SignPath Foundation" appears as the
   publisher; CI-built only) or an OV certificate in your or your company's name (paid; a
   hardware key; your name as publisher)?
5. **Names.** `C:\Program Files\Okur`, AT key `okur_okur_v1`, `okur-sr.exe` as the installed
   screen reader with `okur` as its Win+R name, `okur-setup.exe` for install and uninstall.
6. **Defaults.** Adopt JAWS's: sign-in screen on, all users off, this user Always, confirm
   on exit on?
7. **Secure-screen settings.** Explicit copy (NVDA's way, proposed) unless J8 shows that JAWS
   follows the user's settings automatically?
8. **Coexistence with JAWS.** When the installer finds JAWS (or NVDA) set to start on the
   sign-in screen, should it ask which one starts there, keep the other, or replace it?
9. **The optional JAWS start-options observation** in §6.2: watch which registry values JAWS
   changes, or skip it?

---

## 8. Sources

**Microsoft**

- [MS1] Ease of Access – Assistive Technology Registration, Microsoft Learn (2025-07-14): https://learn.microsoft.com/en-us/windows/win32/winauto/ease-of-access---assistive-technology-registration
- [MS2] Security Considerations for Assistive Technologies (2026-02-18): https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-securityoverview
- [MS3] SetWindowHookEx API with WH_JOURNALRECORD... (WinSDK blog archive, 2009; quotes the Vista UAC requirements paper, including UIAccess rights and allowed folders): https://learn.microsoft.com/en-us/archive/blogs/winsdk/setwindowhookex-api-with-wh_journalrecord-this-api-fails-with-access-denied-error-5
- [MS4] User Account Control: Only elevate UIAccess applications that are installed in secure locations: https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-10/security/threat-protection/security-policy-settings/user-account-control-only-elevate-uiaccess-applications-that-are-installed-in-secure-locations
- [MS5] BCDEdit /set (testsigning, nointegritychecks): https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/bcdedit--set
- [MS6] Loading Test Signed Code (TESTSIGNING): https://learn.microsoft.com/en-us/windows-hardware/drivers/install/the-testsigning-boot-configuration-option
- [MS7] Impact of Session 0 Isolation on Services and Drivers in Windows: https://learn.microsoft.com/en-us/previous-versions/windows/hardware/design/dn653293(v=vs.85)
- [MS8] LowLevelKeyboardProc: https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc
- [MS9] SetWindowsHookExW: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw
- [MS10] What is Smart App Control: https://support.microsoft.com/en-us/topic/what-is-smart-app-control-285ea03d-fa88-4d56-882e-6698afdb7003
- [MS11] /MANIFESTUAC linker option: https://learn.microsoft.com/en-us/cpp/build/reference/manifestuac-embeds-uac-information-in-manifest
- [MS12] Create a certificate for package signing (EKU and basic-constraints extensions): https://learn.microsoft.com/en-us/windows/msix/package/create-certificate-package-signing
- [MS13] Code signing options for Windows app developers (2026-08-29): https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options

**NVDA** (https://github.com/nvaccess/nvda, tag `release-2026.2`)

- [N1] `source/easeOfAccess.py`: `AutoStartContext`, `notify`, `setAutoStart`.
- [N2] `source/installer.py`: `registerEaseOfAccess`, `registerInstallation`, `install`, `createPortableCopy`, `_unregisterEaseOfAccessApp`; `source/config/registry.py` (`nvda_nvda_v1`, `serviceDebug`, `forceSecureMode`).
- [N3] `source/nvda.pyw`: mutex, `--ease-of-access`, secure-desktop switches, `ChangeWindowMessageFilter`, `SetProcessShutdownParameters`, Ease of Access notify, screen-reader flag.
- [N4] `source/winAPI/sessionTracking.py`, `source/IAccessibleHandler/__init__.py` (`processDesktopSwitchWinEvent`).
- [N5] `source/utils/security.py` (`isRunningOnSecureDesktop`, safe scripts, below-lock-screen check), `source/winAPI/secureDesktop.py`, `source/gui/exit.py`, `source/gui/blockAction.py`.
- [N6] `source/config/__init__.py`: `setStartOnLogonScreen`, `setStartAfterLogon`, `_setSystemConfig`; `source/nvda_slave.pyw`.
- [N7] `source/setup.py`, `source/manifest.template.xml`, `projectDocs/dev/selfSignedBuild.md`, `ci/scripts/sign.ps1`.
- [N8] PR #7625 (nvda_service removed, 2017.4): https://github.com/nvaccess/nvda/pull/7625 ; issue #15577 (nvda_eoaProxy removed, 2024.1): https://github.com/nvaccess/nvda/issues/15577
- [N9] NVDA User Guide (`user_docs/en/userGuide.md`): Use NVDA during sign-in, Secure Mode, Secure Screens, System Wide Parameters, Portable and Temporary Copy Restrictions, Command Line Options.
- [N10] `projectDocs/design/technicalDesignOverview.md` ("Logging in secure mode").
- [N11] `projectDocs/design/startupShutdown.md`.
- [N12] NVDA security advisories: https://github.com/nvaccess/nvda/security/advisories (GHSA-h7pp-6jqw-g3pj, GHSA-3jj9-295f-h69w, GHSA-vw48-6q24-rppq, GHSA-mvc8-5rv9-w3hx, GHSA-585m-rpvv-93qg, GHSA-354r-wr4v-cx28, GHSA-rmq3-vvhq-gp32, GHSA-x2c3-c6j2-87w4, GHSA-72mj-mqhj-qh4w, GHSA-grvr-j2h8-3qm4, GHSA-8f8q-2jc3-6rf4, GHSA-qf5h-qw92-rx2f, GHSA-q7c2-pgqm-vvw5).
- [N13] GHSA-fpwc-2gxx-j9v7 (OOBE and "forgot my PIN" temporary desktops): https://github.com/nvaccess/nvda/security/advisories/GHSA-fpwc-2gxx-j9v7

**JAWS 2025 help** (extracted CHM pages) **and key map**

- [J1] `JFW/using_jaws/starting_jaws_for_windows.htm`
- [J2] `JFW/using_jaws/the_jaws_application_window/basics_dialog.htm`
- [J3] `JFW/using_jaws/installing_jaws/Startup_Wizard.htm`
- [J4] `JFW/using_jaws/the_jaws_application_window/options_menu_overview.htm` (Exit)
- [J5] `JFW/keystrokes/jaws_keystrokes/miscellaneous_jaws_keystrokes.htm` (Shut Down JAWS, INSERT+F4); `tutorial/new_users_start_here/starting_jaws.htm`
- [J6] `JFW/using_jaws/installing_jaws/installing_jaws_for_windows.htm` (administrator, restart, `/Type Silent` and the other switches)
- [J7] `C:\ProgramData\Freedom Scientific\JAWS\2025\Scripts\enu\Default.JKM`: `JAWSKey+F4=ShutDownJAWS`, `JAWSKey+Space&F4=RestartWithoutDump`
- [J8] `JFW/whats_new/Whats_New.htm` (Logon-screen fixes)

**Others**

- [X1] Google Project Zero, "Windows Administrator Protection" (2026-02): https://projectzero.google/2026/02/windows-administrator-protection.html
- [X2] James Forshaw, "Accessing Access Tokens for UIAccess" (2019): https://www.tiraniddo.dev/2019/02/accessing-access-tokens-for-uiaccess.html
- [X3] x64dbg issue #3345 (CreateProcess 740; "A referral was returned by the server" with `uiAccess="true"`): https://github.com/x64dbg/x64dbg/issues/3345
- [X4] AutoHotkey v2 docs, "Run with UI Access": https://www.autohotkey.com/docs/v2/Program.htm
- [X5] SignPath Foundation conditions: https://signpath.org/terms ; origin verification: https://docs.signpath.io/origin-verification
- [X6] Raymond Chen, "The .Default user is not the default user": https://devblogs.microsoft.com/oldnewthing/20070302-00/?p=27783
- [X7] Apple Developer Forums, thread 692831 (Quinn, Apple DTS, on pre-login agents): https://developer.apple.com/forums/thread/692831

**Observed on the dev box** (read-only, 2026-10-20):

- `reg query` of `HKLM`, `HKCU` and `WOW6432Node` `...\Accessibility` (and `HKU\.DEFAULT`);
- `HKLM\...\Run` and `HKCU\...\Run`;
- `...\Policies\System` (`EnableLUA`, `ConsentPromptBehaviorAdmin`, `PromptOnSecureDesktop`,
  `EnableSecureUIAPaths`);
- `Win32_Service` and `Win32_Process` lists;
- AtBroker's file description;
- the voice tokens;
- the absence of a manifest and signature on `target\debug\okur.exe`;
- `signtool.exe` and `mt.exe` under `Windows Kits\10\bin\10.0.26100.0`.
