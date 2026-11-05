# Handoff: the Linux lab's first week, in one day (2026-11-05)

After [`2026-11-05-scan-cursor-mouse-and-wpf-focus.md`](2026-11-05-scan-cursor-mouse-and-wpf-focus.md).
Plan: [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md) (its results table
is filled in); research: [`../references/linux-lab-proposal.md`](../references/linux-lab-proposal.md).

## The owner's decisions (2026-11-05)

- Linux is the first port after Windows, tested in a **headless WSL distribution on old-pc**
  (`okur`, Ubuntu 26.04.1, at `D:\okur-linux\wsl`), not GitHub Actions; yes to the proposal's O2,
  O5 and O6. The owner freed old-pc's C: (agent-rs, akana, the uv and Hugging Face caches; ggmlc
  kept).
- "You can change wslconfig yourself": the agent may edit `.wslconfig` on old-pc and the dev box,
  keeping WSLg off and the swap off C: (`skills/linux-lab/SKILL.md`).

## The lab

- `C:\Users\ailabs\.wslconfig`: `guiApplications=false`, 6 GB, 4 processors, swap on D:. WSL 3.0.3's
  NAT fails on old-pc and falls back to user-mode networking, whose DNS needs public servers
  (`dns.sh`).
- `setup.sh` installs the stack (Xvfb, AT-SPI, fluxbox, GTK 4 with GStreamer's base plugins, Chrome
  157, speech-dispatcher with espeak-ng, PulseAudio, Mutter 50.1, xdotool, xinput, python3-xlib,
  python3-speechd) and Rust 1.99.0.
- `a11y-stack.sh` runs a command in a private headless desktop (both AT-SPI switches set; its
  daemons end with the command).
- `sync.sh` sends the committed branch as a git bundle of only the commits the lab lacks (the whole
  branch took over five minutes), and every lab script.
- **`job.sh start|wait|log <name>`** (new): a long command detached in the lab, its log and exit
  code in `D:\okur-linux\jobs`. `opendesk exec` times out; the first detached launch through
  PowerShell's quoting never ran, and a wrapper named like the script it ran called itself until
  inotify ran out (the wrapper is now `<name>.job.sh`).
- `run-batch.ps1` shuts WSL down before every JAWS batch: Linux jobs run between rounds.

## Found in the lab (each probe a script, each run by `job.sh`, output quoted)

| Probe | Result |
|---|---|
| 1 Chrome's tree through xa11y | `PROBE 1 PASS: the heading and the first paragraph are in Chrome's AT-SPI tree`. Chrome joins the AT-SPI desktop seconds after its window; read at 10 s, xa11y said `No applications found.` |
| 2 events while Tab moves | Before the fix: `RESULT gtk-alone: 6 Tabs; xa11y focus_changed: 0`, `RESULT chrome-alone: 6 Tabs; xa11y focus_changed: 0`; with a listener registered with the registry, 6 and 7. **Toolkits send nothing until someone calls `RegisterEvent`.** |
| 3 X11 keys | `watcher (raw): a Down Insert Down b`, `hook: Insert Down`, `target: a Down b`: XInput2 raw events see every key; a passive grab of Insert keeps Insert and the keys pressed with it from the application. |
| 4 speech-dispatcher | `notifications: begin m1 m2 end`, `monitor: 197044 bytes, peak 32768` (a PulseAudio null sink). |
| 5 Mutter's KeyboardMonitor | `WatchKeyboard: refused: ... AccessDenied: Access denied` under `org.okur.Okur.KeyboardMonitor`; `WatchKeyboard: allowed` with `MUTTER_DEBUG_A11Y_MANAGER_WITHOUT_ACCESS_CONTROL=1`; `KeyEvents heard: 4` from keys injected through Mutter's RemoteDesktop API: the proposal's open question (do injected keys reach `KeyEvent`?) is **yes**, so the Wayland key path can be tested headless. |

## Built

| Commit | What |
|---|---|
| `c1acaf29` | `job.sh`; probe 2; Chrome waited for; both AT-SPI switches; GStreamer for GTK 4's widget factory |
| `9b302db2` | Probes 3 to 5 and their packages |
| `27af48cc` | **xa11y-linux registers a subscription's events with the AT-SPI registry** (`object:`, `window:`, `focus:`, `document:`, scoped to the application's bus name, deregistered when the subscription ends); a test pins one class per event interface |
| `2f432d52` | `okur-linux` (platform, gated at its root) and `okur-linux-model` (neutral: SSIP's scale for Okur's rate and pitch) in the workspace and `crate-layers.json`; `sync.sh` incremental |

After `27af48cc`, in the lab (`job.sh start xa11yfix`): `cargo test -p xa11y-linux` exit 0,
`cargo clippy -p xa11y-linux --all-targets -- -D warnings -D clippy::string_slice` exit 0, and
probe 2:

```
RESULT gtk-alone: 6 Tabs; xa11y focus_changed: 6; registered focused: -
RESULT gtk-registered: 6 Tabs; xa11y focus_changed: 6; registered focused: 6
RESULT chrome-alone: 6 Tabs; xa11y focus_changed: 7; registered focused: -
RESULT chrome-registered: 6 Tabs; xa11y focus_changed: 6; registered focused: 6
PROBE 2 PASS: the registered listener heard one focus event per Tab in GTK 4 and Chrome
```

`cargo test -p okur-linux-model --lib`: `2 passed` (dev box). The Windows gate passed each commit.

## Next (Linux, week 2; the plan's §2)

1. **The keyboard port (the proposal's C1).** `KeyboardHook` asks the engine, key by key, whether
   to consume it, as Windows' low-level hook allows. X11 gives Okur only the keys it grabbed.
   **Measured (`probe-sync-grab.sh`, `77b69d78`):** a synchronous grab keeps the per-key
   decision. The hook grabbed h and Down with the keyboard frozen until it answered, kept h
   (`AsyncKeyboard`) and replayed Down (`ReplayKeyboard`):
   ```
   hook:   h Down h Down h Down h Down h Down h Down h Down h Down h Down h Down
   target: Down a Down a Down a Down a Down a Down a Down a Down a Down a Down a
   replayed Down, ms from the decision to the application: median 0.30, max 0.34
   SYNC GRAB PASS: h kept, Down replayed, a untouched, in order, ten times
   ```
   So the port can stay "the engine decides each key". Which keys to grab: see the plan's
   section "The keyboard on X11": every key but the modifiers grabbed through XInput2, with a
   watchdog, on the application windows rather than the root (there the newest grab wins and would
   take a window manager's shortcuts), probed and passing (`probe-grab-all.sh`: 233 keycodes x 64 modifier sets, the
   application got `a w C Down x b` in order, 0.34 ms median per replayed key); a set of bound
   chords was set aside.
2. `okur-linux`'s events: an `EventSource` over xa11y-linux, for every application (xa11y
   subscribes per process; Okur needs the desktop: subscribe as applications appear, or a
   desktop-wide subscription in xa11y-linux).
3. Speech: an SSIP client (speech-dispatcher's text protocol over its Unix socket; a small one of
   our own or `ssip-client-async`), with `ssip_scale` for rate and pitch.
4. Then the plan's §2.2: a first web scenario run live by `okur-linux` on Chrome in the lab.

## The Windows work beside it: the dictionary's rest

Three rounds were recorded on old-pc after the probes; none is imported yet (below).
- **1156** (Case Sensitive): Alt+S is not Case Sensitive but **Sound**, so the rule "TOTAL" was
  never case sensitive, and JAWS said "let sum equals" for "total": a rule matches whatever the
  case unless the box is checked. The scenario is to be recorded again with Alt+N.
- **1158** (the Add dialog walked by Tab; trials 1 and 2 agree, trial 3's Tabs never left
  Actual Word): Actual Word, Replacement Word, "Sound: read only edit | <None>", Select Sound...,
  Play Sound, "Applies to: Language: Combo box | All Languages | 1 of 9", "Speak replacement word
  using Language: Combo box | Default | 1 of 1", "Case Sensitive check box not checked", OK,
  Cancel; access keys from its trace: A, R, S, T, P, L, U, N.
- **1157** (a rule in a page, Chrome's own dictionary): its first recording read the heading,
  not the paragraph (the page's first line is its title); the second's trials 1 and 2 met the
  manager 1158's broken trial had left open ("The Dictionary Manager is already running"); its
  trial 3 is clean. To be recorded again.

`e3d18779` gives Okur's Add and Change dialog JAWS's controls, order and access keys (a rule
keeps the sound chosen; Play Sound plays it; the languages have Okur's one choice until JAWS's
nine are recorded), and fixes replay: Sound's "<None>", reported 2 ms before Tab's focus on it,
had become the replacement. Previews (`preview-round.sh`, then `okur-lab parity -c ... --spoken`):
1156 29/34 spoken (27 before), 1158 22/23 spoken. All scenarios after it (no other step moved;
the totals rose by 1155's 29 steps, 25 matched, since the last handoff's figures):

```
JAWS parity (Ia2): 11735/12291 steps match exactly; 382 accepted divergences not counted (0 scenarios could not run)
JAWS spoken parity: 11481/12357 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Uia): 10556/12177 steps match exactly; 382 accepted divergences not counted (16 scenarios could not run)
Buffer invariants: 0 violation(s)
```

**Why nothing was imported: a Windows Security Alert** ("Allow COM Surrogate to communicate on
these networks", `dllhost.exe`) has been on old-pc's screen since 11:43, most likely raised by a
port WSL forwarded to Windows during the Linux probes. Its window is in every trace of the three
rounds (`rundll32.exe`, not a reviewed process), and it may have taken keys. Only the owner
answers it. `localhostForwarding=false` is now in old-pc's `.wslconfig` so WSL does not raise it
again.

**Afterwards (the owner allowed the prompt; volume back to 1%):** 1156 (Alt+N, and the Language
combo walked), 1157 and 1158 were recorded again in one batch, no alert and no leftover window in
their traces, and imported (`48572983`):
- 1156: Case Sensitive checked, "TOTAL" leaves "total" as it is; cleared, "let sum equals".
  Language's nine: All Languages, English, Finnish, French, German, Italian, Portuguese, Spanish,
  Turkish (old-pc's voices' languages, it seems; Okur offers "All Languages" only).
- 1157: Chrome's own dictionary (Chrome.JDF); the rule is applied in the virtual cursor's line.
- 1158: the walk again, all three trials agreeing.

`okur-lab parity` after the import: `11812/12382 steps match exactly` (77/91 of the three, the
same in the spoken layer). Left in them, open: when the manager opens JAWS says only the focused
button and its title only on the first Tab ("Tree view notepad.jdf - JAWS Dictionary Manager |
Dictionary Entries Tree view"), in every recording so far; "File Name:" said once in these nine
trials and twice in 1154's and 1155's (Okur says it twice: left as it is); Chrome's return after
Alt+F4 ("<title> | <title> page | <the line>", Okur "<title> document <title>").
