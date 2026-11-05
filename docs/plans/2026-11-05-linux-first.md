# Plan: Linux first, in headless WSL (2026-11-05)

**The owner, 2026-11-05:** Linux is the first cross-platform target, before macOS: no spare Mac
to build and test on continuously, while a second platform shapes the cross-platform seams (what
is shared, what each platform implements, how their differences are handled) and proves Okur
cross-platform. Testing runs in a **headless WSL distribution on old-pc** (`okur`, Ubuntu
26.04, on D:; the owner moved it there from the dev box the same day once old-pc's disk was
freed), not GitHub Actions (Actions has limits and wants a push per test run; it may serve the first Mac work later).
Yes to the proposal's other decisions: O5 (ask GNOME and KDE about `KeyboardMonitor` once a prototype exists), O6 (Linux AT-SPI snapshots
in the corpus as platform data with provenance, never as JAWS evidence), O2 (a mini PC for a real
desktop, later). No WSL desktop on old-pc. Linux stays Tier 2 for users (AGENTS.md §1.3).

Research and the reasons: [`../references/linux-lab-proposal.md`](../references/linux-lab-proposal.md)
(its section 2 lists where the project's assumptions do not hold on Linux).

## 0. The lab (set up 2026-11-05)

- old-pc, WSL 3.0.3: the distribution `okur` at `D:\okur-linux\wsl`; `C:\Users\ailabs\.wslconfig` turns
  WSLg off (it must never take the focus from a JAWS recording), caps it at 6 GB and 4
  processors, and puts its swap on D:. WSL's NAT networking fails there and falls back to its
  user-mode networking, whose DNS needs public servers (`skills/linux-lab/scripts/dns.sh`).
- `run-batch.ps1` shuts WSL down before every JAWS batch; Linux work runs between rounds.
- How to use it: `skills/linux-lab/SKILL.md` (`sync.sh` sends the committed branch, `setup.sh`,
  `a11y-stack.sh`).

## 1. Week 1: the lab and the probes

The WSL distribution (Ubuntu 26.04 or 24.04, headless) holds the same stack as Orca's and xa11y's
own CI: Xvfb, `dbus-run-session`, `at-spi-bus-launcher`, `at-spi2-registryd` with `org.a11y.Status`
set, a small window manager, Chrome, GTK 4, speech-dispatcher with espeak-ng into a null sink, and
a Rust toolchain from `rust-toolchain.toml`. One script under `skills/` (or `.agents/scripts/`)
sets it up and one starts the stack, so the same can run in Actions later. The repository is built
from the committed branch, sent by `sync.sh` as a git bundle; its Linux `target` stays inside
the distribution.

Probes, each with a pass criterion that can fail (proposal §6, W1-1 to W1-6):
1. Chrome on `corpus/fixtures/doc-reading.html`, read through the vendored xa11y: the headings and
   links by name and role.
2. Desktop-wide AT-SPI events registered with the registry (`RegisterEvent`), Tab through a GTK 4
   app: one focus event per Tab, in order.
3. An X11 key hook: XInput2 watch plus a synchronous grab of Insert+Down, keys sent with XTest:
   every key seen, the grabbed chord kept from the application.
4. speech-dispatcher: begin, end and index marks for a marked text.
5. Headless Mutter: `KeyboardMonitor` under Okur's own name refused, then allowed with the debug
   switch (the upstream question O5 rests on it).
6. The crates: `okur-linux` (platform, `#![cfg(target_os = "linux")]`) and `okur-linux-model`
   (neutral) in `.agents/crate-layers.json`, the gate clean.

### Results (2026-11-05, in the lab; each a script in `skills/linux-lab/scripts/`, run by `job.sh`)

| Probe | Result | What it showed |
|---|---|---|
| 1 `probe-chrome.sh` | PASS | Chrome 157's page tree read by xa11y's `tree`: the heading and the first paragraph by role and name. Chrome joins the AT-SPI desktop some seconds after its window: read too early, there is no application. |
| 2 `probe-events.sh` | PASS | Six Tabs: GTK 4 (`gtk4-widget-factory`) and Chrome sent **no** event to xa11y's match rules alone; with a listener registered with the registry (`RegisterEvent`, as libatspi does), one focus event per Tab, in order (6 and 7; Chrome's 7th is the page again after its toolbar), heard by xa11y too. xa11y-linux now registers (`27af48cc`). |
| 3 `probe-keys.sh` | PASS | X11: XInput2 raw events saw `a Down Insert Down b`; a passive grab of Insert gave the hook `Insert Down`; the application got `a Down b`. The Okur key as a grabbed key works, with every key still seen. |
| 4 `probe-speech.sh` | PASS | speech-dispatcher (Ubuntu 26.04's) with espeak-ng over SSIP (SSML): `begin m1 m2 end`, and sound on a PulseAudio null sink's monitor. |
| 5 `probe-mutter.sh` | PASS | Mutter 50.1 headless: `WatchKeyboard` under `org.okur.Okur.KeyboardMonitor` refused ("Access denied"); allowed with `MUTTER_DEBUG_A11Y_MANAGER_WITHOUT_ACCESS_CONTROL=1`; keys injected through Mutter's RemoteDesktop API **do** arrive as `KeyEvent` (a and Down, pressed and released), so the Wayland key path can be tested headless. |

Probe 6, the crates: `okur-linux` and `okur-linux-model` are in the workspace and
`crate-layers.json` (`2f432d52`). Week 2 starts with the keyboard port's design (the handoff
[`../handoff/2026-11-05-linux-lab-week-1.md`](../handoff/2026-11-05-linux-lab-week-1.md), Next 1).

### The keyboard on X11: the design to probe next (2026-11-05, reversible)

Probe 3 and the synchronous grab probe (`probe-sync-grab.sh`: a key kept or replayed, 0.30 ms
median from the decision to the application) show a synchronous grab keeps the engine's per-key
decision: kept (`AsyncKeyboard`) or replayed to the application (`ReplayKeyboard`).

Which keys to grab. A set of the chords the keymap binds was considered first and set aside: the
bindings take "any" for modifiers (`command.rs`), and many keys are consumed outside the binding
table (Say All's keys, layers, the virtual cursor, Keyboard Help, which takes every key), so the
set would be large and still miss keys. The design to probe instead keeps Windows' model whole,
with no change to the `KeyboardHook` port:
- **Every key but the modifiers grabbed synchronously** through XInput2 (`XIGrabKeycode`, which
  answers per modifier set): a combination another client already grabbed (a window manager's
  shortcut) is reported back and stays that client's. The modifiers themselves are watched
  through raw events, as the Okur key is held (probe 3).
- **A watchdog:** a key the engine has not answered in a set time is replayed. X11 has no
  hook timeout; Windows removes a hook slower than `LowLevelHooksTimeout`, while a hung Okur on
  X11 would freeze the keyboard.
- **Probed (`probe-grab-all.sh`, 2026-11-05): it works.** 233 keycodes grabbed under 64
  modifier sets; a, h, w, Shift+c, Down, Control+x, b sent by XTest; the hook got all seven, kept
  h, held w until a 200 ms watchdog replayed it ("watchdog w 200.6"), and replayed the rest;
  the application got `a w C Down x b`, in order; 0.34 ms median (0.51 max) from the decision to
  the application. A run whose hook crashed on its first key left the keyboard frozen and the
  application got nothing: the watchdog is required, and must not live in the thread that can
  fail.
- **Where the grabs go: the application windows, not the root (probed the same day).** XInput2's
  reply reports no conflict with another client's core grab (fluxbox holds Alt+Tab: a client's
  `XGrabKey` of it got `BadAccess`, while the hook's grab of every key "succeeded"). On the root,
  the newest grab wins: Super+E, grabbed by another client before the hook, went to the hook and
  was replayed past that client to the application; Super+R, grabbed after it, went to the other
  client. X looks for a passive grab from the root down, so with the hook's grabs on the focused
  application window, a shortcut another client grabbed on the root comes first whenever it was
  grabbed: Super+E and Super+R both reached their client, every other key the hook
  (`hook: a h w c Down x b`, `target: a w C Down x b`, 0.30 ms median per replayed key).
  `okur-linux`'s hook therefore grabs on every top-level window (the root's children, as they
  are created), the window manager's root shortcuts staying its own.

## 2. Weeks 2 to 6

**Done the same day** ([`../handoff/2026-11-05-okur-on-linux-and-number-processing.md`](../handoff/2026-11-05-okur-on-linux-and-number-processing.md)): §2.1's keyboard, events and speech in `okur-linux`, each passing live in the lab, and Okur run whole (the runtime's event loop with Linux's platform) speaking GTK 4's focus. The keyboard grabs each key under any modifier (64 combinations apart overwhelmed the X server). Next: §2.3 before §2.2, since a web scenario needs the page: Chrome's AT-SPI tree has the shape of its UI Automation tree (headings with `level`, "static" text leaves; `probe-atspi-tree.sh`), so an AT-SPI snapshot and its `to_aom` in `okur-linux-model`, captured live through xa11y-linux, then `DocumentLoaded`; then §2.2. **Done the same evening:** the page read through AT-SPI into the AOM, Okur reading doc-reading.html on load in the lab (`probe-okur-web.sh`, WEB PASS).

1. `okur-linux`: keyboard (X11 first, the declared grab sets the proposal's C1 calls for), events
   (xa11y-linux extended in place: desktop-wide, registered), speech (SSIP), enough for the virtual
   cursor on a web page.
2. A live run in WSL of the first web scenarios (document reading, quick navigation) by
   `okur-linux` on Chrome under Xvfb, its results written by a tool into a Linux conformance
   baseline that may only improve.
3. `okur-linux-model` and Linux snapshots of the HTML fixtures, replayed on every OS.
4. Invariants for GTK and Qt controls (a check box, a combo box, a dialog said as on Windows).

## 3. Meanwhile

The Windows work goes on beside it (the dictionary's rest, the Settings Center features of the
review plan, §3). A build in the lab is old-pc's one heavy job; JAWS rounds come first.
