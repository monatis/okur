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
   to consume it, as Windows' low-level hook allows. X11 decides in advance: a key reaches Okur
   instead of the application only if it is grabbed. libatspi's X11 device grabs keys
   synchronously (`XIGrabKeycode`) and then lets each one go or keeps it, which keeps the
   engine's per-key decision at one round trip per grabbed key. Probe both: (a) a sync grab of the
   Okur key and of the keys the current mode takes (the declared grab set), and (b) the latency of
   a synchronous decision. Then design the port change before writing `okur-linux`'s hook.
2. `okur-linux`'s events: an `EventSource` over xa11y-linux, for every application (xa11y
   subscribes per process; Okur needs the desktop: subscribe as applications appear, or a
   desktop-wide subscription in xa11y-linux).
3. Speech: an SSIP client (speech-dispatcher's text protocol over its Unix socket; a small one of
   our own or `ssip-client-async`), with `ssip_scale` for rate and pitch.
4. Then the plan's §2.2: a first web scenario run live by `okur-linux` on Chrome in the lab.

The Windows work goes on beside it: round 1156 (the dictionary's Case Sensitive) was started on
old-pc after the probes.
