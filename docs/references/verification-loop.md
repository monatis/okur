# The Verification Loop: Working With JAWS on old-pc and the Dev Box

This is how the screen reader is developed against JAWS: the machines, the tools, and the
lifecycle of one feature from discovery to merge. Hand this document to a new session along
with `AGENTS.md`, the active plan, and the latest handoff.

The rules in `AGENTS.md` (section 3, "Verification Integrity Rules") are not repeated here
in full. They decide what counts as evidence, and they win over anything below.

---

## 1. The two machines

| | Dev box (the user's) | `old-pc` (the agent's lab) |
|---|---|---|
| OS | Windows 11 (26200) | Windows 10 19045, interactive console session 1, user `ailabs` |
| JAWS | 2025, the user's own copy, **always running** (laptop layout) | 2025.2505.43.400 (`jfw`), configured for recording (§3) |
| Role | Code, build, tests, parity replay; live checks **only when the user allows** | Records JAWS evidence and traces; live checks of ours |
| Repo / skill | `C:\Users\Gigabyte\screen-reader` | Skill copy at `C:\Users\ailabs\skills\jaws-reference\` |
| Dropbox | `C:\Users\Gigabyte\Dropbox\sr-harness\` | `C:\Users\ailabs\Dropbox\sr-harness\` |

- **old-pc is the agent's lab.** JAWS, Chrome, windows and test state there may be stopped,
  restarted and reconfigured freely, with no need to ask. Never run two recordings at once:
  JAWS, the focus and the clipboard are global state.
- **The dev box is the user's machine.** They work on it and depend on its JAWS. Drive it only
  when the user says so for that occasion (§7).
- **Evidence comes only from old-pc.** Anything observed on the dev box is research.
- **Names from before the rename to Okur (2026-10-15) stay in the lab.** They are installed
  or recorded state, and "SR" reads as "screen reader":
  - the `sr-harness` folders (Dropbox and old-pc);
  - the "SR Speech Recorder" voice and its `sr_sapi_recorder64/32.dll` files (renaming them
    means reinstalling the voice and re-selecting it in JAWS's voice profile);
  - the fixtures' "SR … Fixture" window titles. These are in recorded JAWS speech and never
    change.

  The programs are `okur.exe`, the screen reader (`okur run` for live runs), and `okur-lab.exe`,
  the harness (the tracer, `wait-silence`, `snapshot`, `focus`, ...), both in `bin\` of
  `sr-harness`; the scripts call each for its own commands. They were one program until
  2026-10-21, when Defender flagged it for holding both.
  An old `sr-cli.exe` left on old-pc is unused.

## 2. Reaching old-pc

old-pc runs **`opendesk serve`**. After old-pc restarts, someone must launch it again, or
every call fails with `rendezvous error: target_offline`. On 2026-10-15 old-pc went offline
at about 01:30 and stayed so (the server was no longer listening). Check it with:

```bash
opendesk exec --peer old-pc --timeout 20 "Get-Date -Format HH:mm:ss"
```

- **`opendesk exec --peer old-pc "<powershell>"`** (CLI in `~/.cargo/bin`; the `opendesk` MCP
  tools work too) runs as `ailabs` in the interactive session, which FSAPI needs.
- **Timeouts.** `--timeout` defaults to 30 s; a cut-off scenario run leaves its fixture app and
  `okur-lab trace` running. Use `--timeout 900` for one scenario. Run anything longer detached
  (`Start-Process` of `run-batch.ps1`) and poll its `batch.log`.
- **Paths from Git Bash.** Use forward slashes (`C:/Users/ailabs/...`): backslashes before
  `$var` inside double quotes are mangled.
- **Elevation.** old-pc elevates silently (`ConsentPromptBehaviorAdmin=0`). Input that must
  reach elevated windows (JAWS's own dialogs) goes through
  `C:\Users\ailabs\sr-harness\recorder\elev.ps1 -KeyArgs "-Keys A,B -DelayMs N"`.
- **JAWS scripts.** JAWS's own scripts can be run through FSAPI `RunScript`
  (`SelectAVoiceProfile`, `JAWSWindow`).
- **Clock.** old-pc's clock runs about 20 minutes behind the dev box.

## 3. old-pc's JAWS configuration (all evidence reflects it)

- **Keyboard and modes.** In the user's `%APPDATA%\...\JAWS\2025\Settings\enu\DEFAULT.JCF`:
  - `KeyboardType=Laptop`: CapsLock and Insert are both the JAWS key;
  - `TypingEcho=0`;
  - `AutoFormsMode=0`.

  Replay mirrors these with `Settings::reference_machine()`. Each evidence file's
  `provenance.jaws_settings` records them.
- **Voice profile.** "SAPI 5X 64 - (factory)", voice **SR Speech Recorder**, punctuation Most,
  rate 11, Upper Case Pitch Change 20. `DEFAULT.JCF` has `[Voice Profiles]
  ActiveVoiceProfileName=SAPI 5X 64`. The original profile was Eloquence with punctuation off;
  to revert, choose it again with Insert+Ctrl+S on old-pc.
  - **Rate raised on 2026-10-18 at the user's suggestion** (14:14 old-pc time): `Rate=90` in
    every `[ENU-...]` context of `voiceprofiles\SAPI 5X 64.VPF`, which JAWS sends as SAPI rate
    10, the fastest (it sent 1 before). The speech history and the recorder's text do not
    depend on the rate; only the waiting shrinks (221: 95 s to 74 s). The profile before the
    change is kept as `C:\Users\ailabs\sr-harness\explore\SAPI 5X 64.VPF.before-rate`.
- **Week numbers and regional settings.** Format en-US (weeks from Sunday), system locale en-US,
  home location Türkiye, languages Turkish then English. JAWS's week numbers start on Monday
  whatever these say: with the home location set to the United States and English first, JAWS
  still said "Week 42" on Sunday 2026-10-18 (both restored the same day).
- **Speech recorder.** The SR Speech Recorder voice (`crates/okur-speech-recorder`) logs every
  fragment JAWS sends its synthesizer to `C:\Users\ailabs\sr-harness\recorder\spoken.jsonl`,
  then hands it to a real voice, so speech stays audible and its timing realistic.
  - It is registered under HKLM by `install-speech-recorder.ps1` (`-Uninstall` removes it).
  - JAWS must not have the voice loaded while `lab-pull.ps1` replaces the DLL.
- **Locale.** The UI language is Turkish (UIA localized strings are Turkish; never key behavior
  on them). The culture is en-US and the UTC offset +180.
- **Display.** 1920x1080 at 125% scaling.
- **Before recording:**
  - close stray Notepad, Chrome and Explorer windows;
  - minimize the JAWS window if it is in front (it took the foreground in trial 1 of 207);
  - clear stray `okur-lab trace` processes.

## 4. The lifecycle of one feature

### 4.1 Discover
Sources, in order of trust:
1. **The user's reports.** Live usability gaps come first.
2. **Recorded evidence:** `okur-lab evidence <scenario> [--trace]`.
3. **JAWS's help and manual:** `JFW.chm` and the keystroke lists. Text copies were kept in a
   session scratchpad and are not in the repo; extract them again from old-pc's JAWS install.
4. **JAWS's own behavior,** observed quickly with `okur-lab keys` and `okur-lab jaws-history` (§7).

Write down what JAWS should say before writing code.

### 4.2 Fixture and scenario
- **Fixtures** live in `corpus/fixtures/`: HTML pages, or PowerShell scripts that build
  WinForms windows (`desktop-*.ps1`).
- **Scenarios** live in `corpus/scenarios/NNN-name.json`: `fixture`, `target_app`, `launch`,
  `reset`, and `steps` (`key`, `repeat`, `key_down`/`key_up` to hold keys, `capture_clipboard`,
  `settle_max_ms`). See the header of `run-scenario.ps1`.
- **Unrecorded scenarios** wait in `corpus/pending/`. A scenario in `corpus/scenarios/` without
  evidence fails the parity test ("could not run"), so it moves there only once recorded.

### 4.3 Deploy to old-pc
1. **Build** the release binary into its own target dir (the user may hold `target/debug` with
   a running `okur run`):
   ```bash
   CARGO_TARGET_DIR=target/agent cargo build --release --bin okur
   ```
2. **Stage pending scenarios.** Move them into `corpus/scenarios/` and their fixtures into
   `corpus/fixtures/` in the working tree. The deploy sends only those folders.
3. **Deploy:** `bash skills/jaws-reference/scripts/lab-deploy.sh` (`--okur <exe>` for another
   build, `--recorder`/`--recorder32` for a new recorder build). It installs `okur.exe` and `okur-lab.exe`,
   the skill's scripts, fixtures and scenarios straight into place with `opendesk push` (OpenDesk
   0.3, 2026-10-18), which checks each file's SHA-256 and replaces the destination only once the
   whole file arrived. Files already installed with the same hash are not sent; a second run
   prints "0 file(s) sent". Since it compares every file, it also keeps the skill copy in sync.
4. **Fallback through Dropbox:** `lab-push.ps1 -Okur ...`, then `lab-pull.ps1 -PushId <id>` on
   old-pc. The PushId is mandatory: without it a stale manifest once "verified" stale files.

### 4.4 Record
- **One scenario:**
  ```bash
  opendesk exec --peer old-pc --timeout 900 "powershell -NoProfile -ExecutionPolicy Bypass -File C:/Users/ailabs/skills/jaws-reference/scripts/run-scenario.ps1 -ScenarioFile C:/Users/ailabs/corpus/scenarios/NNN.json -Repetitions 3 -OutputPath C:/Users/ailabs/Dropbox/sr-harness/evidence-out/NNN-evidence.json -TraceDir C:/Users/ailabs/Dropbox/sr-harness/evidence-out/traces"
  ```
- **Many scenarios:** run `run-batch.ps1 -ScenarioGlob ... -OutputDir
  C:/Users/ailabs/sr-harness/evidence-out -Trace` as an OpenDesk background job (MCP
  `opendesk_exec` with `background: true`, then `opendesk_job` `poll`): no connection is held and
  nothing waits on Dropbox. Its `batch.log` still lists each scenario's exit code and time. Keep
  the output out of Dropbox; `lab-fetch.sh` brings it home.
- **What a run records:**
  - each step's JAWS speech history (`speech_raw`);
  - the synthesizer text (`spoken`, `spoken_records` with pitch), up to JAWS's copy
    confirmation (`spoken_end`; skill, speech recorder section);
  - provenance: OS, JAWS version, settings, culture, capture-script hash;
  - with `-TraceDir`, our tracer's accessibility events for each trial.
- **A JAWS setting for a scenario** (`jaws_settings`): recorded through `run-batch.ps1`, which
  sets it and restarts JAWS, and puts old-pc's own settings back afterwards (skill, "Settings
  for a scenario").
- **Keys by their place:** the numpad as with Num Lock off (`Numpad5`, `Insert+Numpad5`), and
  `Comma`, `Period`, `Slash` by their US places, since JAWS takes keys by scan code and old-pc
  types on Turkish Q (`run-scenario.ps1`'s `$ScanTable`).
- **Snapshots** of a page's IA2 and UIA trees, for page fixtures, come from
  `capture-snapshot.ps1` (skill Workflow F).

### 4.5 Bring evidence home
1. **Fetch and copy:** `bash skills/jaws-reference/scripts/lab-fetch.sh
   C:/Users/ailabs/sr-harness/evidence-out <scratch dir>` pulls every file (hash-verified, only
   the ones not already there). Copy them unmodified into `corpus/evidence/` and
   `corpus/traces/` (snapshots into `corpus/snapshots/`).
2. **Check the capture-script hash.** `provenance.capture_script_sha256` must match the local
   `run-scenario.ps1`.
3. **Read it** with `okur-lab evidence NNN --trace`, which lists each step's JAWS speech, its
   variants, and the events traced meanwhile.
4. **Regenerate the generated files, never edit them:**
   - `okur-lab corpus-manifest --write`;
   - `okur-lab parity --write-baseline` and `okur-lab invariants --write-baseline` when the numbers
     change for a reason you state.
5. **Commit separately.** Evidence, traces and snapshots go in a commit of their own (`corpus:
   scenario NNN ...`): the pre-commit gate rejects evidence and engine code together.

### 4.6 Implement
- **Where code goes:**
  - `okur-core`: the engine, editing, speech, and `apps::*` for application-specific behavior,
    like JAWS app scripts;
  - `okur-platform`: the tracer, UIA/IA2/MSAA reads, TTS, the keyboard hook;
  - `xa11y`: the vendored accessibility layer.
- **No fixture-shaped special cases.** Code must not branch on fixture text or window titles.
- **Known gaps** become `#[ignore = "known gap: ..."]` tests that assert JAWS's behavior.

### 4.7 Prove
1. `okur-lab parity`. Its two lines are the spoken layer ("JAWS spoken parity: X/Y") and the
   history layer ("JAWS parity (Ia2): X/Y"); `--spoken` lists spoken-layer differences.
2. `okur-lab parity --api uia` and `okur-lab invariants`.
3. **Live** on old-pc, or on the dev box when allowed (§7):
   `okur run --engine no-speech --speech-log <f> --debug-log <f>`. The debug log has every
   key, event, edit read, and any "slow event read" with its stages.
4. Claims in commits and handoffs quote the tool output.

### 4.8 Commit and merge
- **The gate.** `git commit` runs it: `cargo fmt --check`, no `unwrap`/`expect`/`panic!`/
  `todo!`/`dbg!`, clippy, `cargo test --workspace`, and reference-data separation.
  - **It judges the whole command and runs none of it if it refuses.** So run generators,
    `git add` and moving files in separate commands, and keep the commit message in a file
    (`git commit -F`).
  - **Untracked scenarios without evidence fail its tests.** Keep them in `corpus/pending/`.
- **Branches.** Work on a feature branch. When the phase is reviewed, fast-forward `main` to
  it and start the next branch from `main`. The user pushes to GitHub
  (`origin`, github.com/monatis/okur); agents do not push.
- **Handoff.** At the end of a round:
  - write a dated handoff in `docs/handoff/` and the next plan in `docs/plans/`;
  - point `AGENTS.md` at both.

## 5. Tools

| Tool | What it does |
|---|---|
| `okur-lab parity [--spoken] [--api uia] [--write-baseline]` | Replays every recorded scenario through the engine (traces, snapshots, settings, clock) and compares with JAWS on both layers |
| `okur-lab evidence <id> [--trial N] [--trace]` | Readable evidence: steps, JAWS speech, variants, traced events |
| `okur-lab invariants`, `okur-lab corpus-manifest [--write]` | Buffer invariants; the corpus manifest |
| `okur-lab trace --out <f>` | Records accessibility events (format 7: focus chains, UIA focus and selection events, captions, dialog layouts, text and fonts) |
| `okur-lab snapshot --api ia2\|uia` | A page's accessibility tree |
| `okur run [--engine onecore\|sapi5\|no-speech] [--keyboard-layout laptop] [--speech-log] [--debug-log]` | The live screen reader |
| `okur-lab keys --allow <processes> [--title <text>] <steps>` | Sends scan-coded chords ("Insert+Alt+W", "down:Alt", "wait:500"), each only while an allowed application (and, with `--title`, a window whose title contains the text) is in front |
| `okur-lab jaws-history [--clear]` | JAWS's speech history through FSAPI; the clipboard's text is put back |
| `okur-lab wait-silence`, `okur-lab speak`, `okur voices` | Audio settling, speech checks |
| `run-scenario.ps1`, `run-batch.ps1` | Recording on old-pc (skill workflows D, G) |
| `lab-deploy.sh`, `lab-fetch.sh` | Hash-verified deployment to old-pc and fetching from it, through `opendesk push`/`pull` |
| `lab-push.ps1`, `lab-pull.ps1` | The same through Dropbox (fallback) |
| `capture-snapshot.ps1`, `install-speech-recorder.ps1`, `probe.ps1` | Snapshots, the recorder voice, a health check |

## 6. Reading JAWS correctly

- **History and spoken text differ.** The speech history keeps text from *before* JAWS's symbol
  processing: `$` stays `$`, and a space is an empty entry. What is spoken is in
  `observation.spoken`, through the recorder voice.
- **Role markers.** History lines can carry `\u{1d}…\u{1e}` around role words ("‹ Edit ›");
  the engine produces the same markers.
- **Typing echo** is never in the history, so it cannot be compared there.
- **Some words are spoken but not kept in the history.** A space read alone ("space"), and
  "blank" for an empty line or word reached with the reading keys or in the Virtual Viewer,
  reach the synthesizer while the history keeps the character itself (empty). Such a step has
  `observation.audio_without_text` set, and `spoken` shows what was said. Okur speaks them
  through `SpeechOutput::speak_character`, which keeps the same split. "blank" for an empty
  field's value on focus (F, E, Tab) is in the history.
- **Variants.** Trials can differ (focus races, Chrome timing); `okur-lab evidence` lists each
  variant. Parity accepts any recorded variant.
- **Contamination.** A step whose history includes the copy confirmation or a Windows
  notification fails as contaminated; never trim it.

## 7. Live checks on the dev box (only with the user's permission)

The night of 2026-10-15, with old-pc offline, the user allowed the dev box to be used until
the morning. This is how it was done; it needs fresh permission each time.

- **Never stop, restart or reconfigure the user's JAWS** unless they say so for that occasion
  (on 2026-10-15 they allowed exiting it). They depend on it.
- **Keys:**
  - send them only with `okur-lab keys --allow <process>`, never into the Claude window or a
    window of the user's. Add `--title <text>` when one process owns the user's windows and
    yours (Windows Terminal runs every terminal window in one process);
  - the dev box's keyboard layout is Turkish Q: keys are sent by position, so the `I` key types
    a dotless "ı" and the key named `apostrophe` types "i" (`e x apostrophe t` types "exit");
  - our screen reader takes JAWS-key chords while it runs, from the user too: keep each live
    run short, stop it right after, and bring the test window to the front before starting it
    (it reads whatever window is in front, the Claude window included);
  - undo what you change (Win+Shift+M after Win+M; restore and save a test file you edited);
  - close every window you opened, checking its title first: Alt+F4 with the desktop in front
    opens the shutdown dialog.
- **JAWS's speech:** read it with `okur-lab jaws-history --clear`, which restores the clipboard.
  Do not read the user's conversations, for example JAWS reading the Claude window.
- **Ours:** run `okur run --engine no-speech ...` and stop it after each check: it reads
  whatever is in front.
- **The dev box's execution policy refuses `.ps1` files run directly.** Never change the
  policy. The skill's own scripts (`lab-push.ps1`) run as the skill documents them
  (`powershell -NoProfile -ExecutionPolicy Bypass -File ...`). For one-off work, write no new
  scripts: use `okur` commands or inline commands.
- **Research only.** Mark what you learn "observed on the dev box" in the code, and add an
  old-pc scenario so it becomes evidence. Windows 11 behavior (XAML switcher, Notepad's
  RichEdit, Windows search) may have no Windows 10 counterpart; say so.
- **The dev box's JAWS settings are the user's.** It says no "banner" on `news-article.html`,
  where old-pc's evidence does. Before calling a difference a gap, read a fixture that has
  evidence on the dev box and compare.
- **Okur next to the user's JAWS** (2026-10-16):
  - JAWS takes arrows and letters in browse mode before the page sees them. Put JAWS in forms
    mode on the field first (a JAWS-only run: Tab, or E, then Enter), then start Okur.
  - JAWS sends its own copy of each navigation key it passes on, injected with its own
    `dwExtraInfo` (0x45789400 on the dev box), usually within 5 ms, but 230 ms for Ctrl+Home.
    Okur's hook skips these copies; older debug logs show every such key twice.
  - Test pages and web apps go in a Chrome with its own `--user-data-dir` in the scratchpad,
    never in the user's Chrome. Bring a test window forward with `okur-lab front <title>`. A
    mouse click on a Chrome window not in front only activates it.

## 8. Pitfalls already paid for

- **Files.** Perl `s|...|...|` mangles files whose text contains `|`; use the Edit tool.
  `cargo fmt` can reformat a region between your read and your edit, so re-read it first.
- **UIA.** Ask for the focused element once per event and read around that element: a second
  `GetFocusedElement` right after a window switch blocked for 7 s.
  - Never read a document's siblings or subtree on focus.
  - A focus or selection UIA's event names may differ from `GetFocusedElement`; read the
    event's sender.
- **JAWS takes navigation keys and sends its own copies.** Keys we send ourselves are marked
  (`dwExtraInfo` "OKUR"), and JAWS's copies of them are recognized.
- **A modifier pressed after the JAWS key is consumed.** The hook keeps it held itself.
- **Windows 10's task switcher sits behind `ForegroundStaging`**, and no WinEvent names it.
  Windows 11's is `XamlExplorerHostIslandWindow` and speaks only through UIA focus events.
- **On old-pc, Chrome needs time to start cold**, and the JAWS window can grab the foreground.
  `run-scenario.ps1` waits for it and checks the foreground.
- **A chord that opens one of JAWS's dialogs can leave a modifier down** (dev box,
  2026-10-17). JAWS runs elevated: while its dialog is in front, Windows drops our key-ups
  and hides the key state from us. Shift stayed down for every application after
  Ctrl+Shift+Insert+Down opened Skim Reading: the next Escape opened Chrome's Task Manager,
  Insert+F5 reloaded the page, and Ctrl+Right selected words. `okur-lab keys` now sends a chord in
  one batch, releases keys left down before its first step, and says when a higher window
  hides the key state. If keys act strangely after a JAWS dialog, suspect this first.
- **Okur beside JAWS: keys Okur passes on can still be JAWS's** (dev box, 2026-10-17). JAWS
  sees each key after Okur; in browse mode it takes arrows and letters for its virtual
  cursor, so a key Okur passes to the page in forms mode never reaches the page (WhatsApp's
  arrows through its messages, typing in a search box Okur focused). Put JAWS in forms mode
  too: Okur's `Enter` on the field, then a second `Enter` that Okur passes and JAWS takes; it
  stays in forms mode as the page moves the focus. In WhatsApp an `Up` reaching an empty
  message field opens "edit last message", and an `Escape` in browse mode closes the chat.
- **JAWS's dialogs are closed to normal-integrity tools.** They refuse our keys and
  `WM_CLOSE`, and UI Automation sees only their titles. Drive them through FSAPI: `RunScript`
  opens one by its script name (`SelectAPlaceMarker`, `QuickSettings`, `RunJAWSManager`,
  `DoFlexibleWebDialog`, the way a layered key would not), `RunScript("ReadBoxInTabOrder")`
  reads it, `RunFunction('TypeKey("Escape")')` closes it.

- **A key release refused leaves the key down for every scenario after it** (old-pc,
  2026-10-18). Left Ctrl stayed down: Ctrl+R became Insert+Ctrl+R (JAWS's regions list) and
  Win+R JAWS's Research It prompt. `run-scenario.ps1` now releases every key whatever failed,
  checks that its keys came up, and starts each trial with `clear-lab-state.ps1` (elevated)
  when a modifier is down or JAWS's main window is in the way. A trial records what it
  cleared as `lab_state_cleared`.
- **JAWS keeps its own record of a held JAWS key**, which Windows' key state does not show.
  After an Insert+F7 whose release met JAWS's elevated dialog, a plain Ctrl+R opened JAWS's
  regions list ("Document Regions") at every trial's start, though no key was down. A bare
  release of Insert and CapsLock resets it; every trial now starts with one, and a chord with a
  JAWS key that leaves a JAWS window in front is released elevated. If JAWS acts on keys as if
  Insert were held, suspect this.
- **A disturbed JAWS stays disturbed until it restarts.** After a day of stuck keys, JAWS stopped
  toggling Caps Lock on CapsLock pressed twice, at any gap, though no key was held; a restart
  cured it. Start long batches with `run-batch.ps1 -FreshJaws`, and run a two-minute canary
  scenario (221) before them. A key check between the presses of a double press must stay
  cheap: a process lookup there once stretched the gap.
- **JAWS's main window comes forward after JAWS's dialogs close** and keeps the foreground from
  ordinary processes. `clear-lab-state.ps1` hides it and passes the foreground on.
- **A JAWS prompt left open spoils a whole batch.** Close it from an elevated process with
  `BM_CLICK` on its button: neither OpenDesk's mouse nor our keys reach it.
- **Traces can carry personal data from other applications.** Windows 10's taskbar news and
  weather widget put links with the machine's location into traces through its name changes.
  The tracer now drops changes (and focus events) from applications not in front before
  reading them. Before committing a trace, search it for URLs (`grep -lE "https?://"`) and
  anything about the owner.
- **Events wait in the hook's queue while earlier ones are read.** A combo box's scrolling sent
  264 scroll-bar state changes as the Run dialog opened, and each was read: the focus came 5 s
  late, into the next step. The tracer now drops scroll-bar events and stamps each event with
  the time Windows raised it.
- **Evidence of JAWS's own dialogs cannot measure Okur's.** JAWS's dialogs run elevated, so
  our tracer sees them thinly ("&Move To Link", no role), and Okur draws its own dialogs.
  Replaying 222-226 compares Okur's reading of JAWS's dialog. Measuring Okur's dialog needs a
  live run of Okur with the same keys.
- **JAWS's Results Viewer gives the recordings nothing.** Once Skim Reading's Create Summary
  or the Word List's Summary opens it, no key gets any speech, ours or TypeKey's, Escape
  included (762, 767, 768, rounds 179 and 183). Left open, it took the focus back when a later
  trial's dialog closed, and that trial read it (763/s23, round 180). `run-scenario.ps1` now
  closes ResultsViewer and NotificationViewer before each trial
  (`lab_state_cleared.jaws_viewers_closed`).
- **`opendesk exec` can hang past its `--timeout`.** A deploy waited ten minutes on its first
  remote call (2026-10-21); stopping the hung `opendesk.exe` let the script go on. List them
  with `Get-Process opendesk` when a lab command takes far longer than it should.

## 9. Open questions to settle on old-pc

- How Windows 10's task switcher, desktop, Run dialog and Notepad behave against the dev box's
  Windows 11 observations (scenarios 316, 317, 319, 320).
- CapsLock+Comma and CapsLock+Period were typed into the document on the dev box: a keyboard
  layout matter (Turkish Q) or a different binding? JAWS binds physical keys by name in its
  `.jkm`; check `Default.JKM` (laptop section).
- JAWS's Insert+Tab inside a web page.
- Which window was "Document Regions" (225 and 226, trial 2): JAWS's regions list opened by a
  stuck Ctrl, most likely; confirm it does not recur with the key fix.
- JAWS's first Down in its font information message said "ESCAPE Press ESCAPE to close this
  message." (314/s3): where does "ESCAPE" come from?
