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

## 2. Weeks 2 to 6

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
