# Linux Lab: Research and Proposal (2026-11-05)

**Decided (the owner, 2026-11-05)**: Linux first; tested in a headless WSL distribution on the dev box instead of GitHub Actions (Actions has limits and wants a push per test run; it may serve the first Mac work later); yes to O2 (a mini PC later), O3, O5 and O6. The plan: [`../plans/2026-11-05-linux-first.md`](../plans/2026-11-05-linux-first.md).
**Status**: proposal for the owner. Research only: nothing was built, no cargo run, nothing run on
old-pc. Facts about old-pc come from the lead agent's read-only check of 2026-11-05. Facts about
the dev box come from reading its files and registry on 2026-11-05.
**Why**: the owner, 2026-11-05: no spare Mac to build and test on continuously; make Linux the
first cross-platform target, so that the shared/per-platform split is learned on Linux and macOS
then goes quickly. Could old-pc be the Linux lab through "the new WSL", which the owner read can
install a desktop environment?
**Read with**: [`AGENTS.md`](../../AGENTS.md) §1.3, [`crate-structure-proposal.md`](crate-structure-proposal.md)
§9 and §13.3, [`crates/xa11y/VENDORED.md`](../../crates/xa11y/VENDORED.md),
[`helper-ui-architecture.md`](helper-ui-architecture.md).

Source IDs (R-linux-NN, L-NN for this repository) are listed in section 10. Each finding is
tagged **fact** (documented, with its source), **observed** (reported by others, not by the
vendor), **vendor claim**, **inference** (our reasoning from facts) or **open question**.

## 1. Summary

- **Linux first is a sound idea.** A Linux driver is a second implementation of every port the
  runtime defines (keyboard hook, events, speech, UI, actions) and of the parity model, so it
  tests the cross-platform seams before macOS. And Linux's accessibility stack runs headless:
  Orca's own CI drives over 2,000 test cases through Xvfb and a private AT-SPI stack (R-linux-25),
  and xa11y upstream, which `crates/xa11y` is vendored from, tests its AT-SPI backend on GitHub's
  Ubuntu runners against GTK, Qt, Electron and other apps (R-linux-30). Continuous Linux testing
  needs no lab machine.
- **WSL with a desktop on old-pc is the weakest lab.** "WSL can install a desktop environment" is
  not a WSL feature: Microsoft says GUI app support "does not provide a full desktop experience"
  (R-linux-01), and the release notes up to WSL 3.0.5 (2026-11-03) add none (R-linux-04). Desktops
  in WSL are community xrdp/VNC setups or Kali's Win-KeX (R-linux-33). On old-pc, WSL brings known
  risks to the JAWS recordings: WSLg's `msrdc.exe` steals focus at random (open since 2022, reported
  on build 19045 in 2026; R-linux-11), WSLg plays Linux audio through old-pc's speakers (R-linux-03)
  where `okur-lab wait-silence` listens, and any key typed into a Linux window passes JAWS's
  keyboard hook first.
- **The finding that matters most is not the lab.** GNOME 50 and KDE Plasma 6.8 are Wayland-only
  (R-linux-14, R-linux-15). On Wayland a screen reader gets the keyboard only through the
  compositor's `org.freedesktop.a11y.KeyboardMonitor` D-Bus interface (R-linux-17), and both Mutter
  and KWin serve it only to the D-Bus client that owns the name `org.gnome.Orca.KeyboardMonitor`
  (R-linux-18, R-linux-20). Okur cannot use it in its own name today. This must be settled with
  upstream before an `okur-linux` driver is more than a lab tool.
- **Recommendation**: (1) GitHub Actions Linux jobs as the continuous lab (free for this public
  repository; R-linux-31); (2) the same headless stack in WSL on the dev box, without WSLg, for
  agents to debug interactively; (3) for real-desktop checks (GNOME on Wayland, the real kernel
  keyboard path, real audio, Orca beside Okur), a small dedicated Linux mini PC, or at no cost a
  Hyper-V VM on old-pc's D: drive, saved to disk during every JAWS round. WSL with a desktop on
  old-pc comes last.

## 2. Contradictions with the project's current assumptions (read first)

| # | Assumption | What the research found |
|---|---|---|
| C1 | A Linux keyboard hook is one more driver behind `KeyboardHook` (`okur-runtime/src/hook.rs`): a callback per key that returns `KeyAction` (pass or consume). | On Wayland the compositor decides which keys are consumed from a list the screen reader registers beforehand (`SetKeyGrabs`), and reports keys as D-Bus signals after the fact (R-linux-17). On X11, at-spi2-core's device grabs keys with synchronous XInput2 grabs (R-linux-22). Per-key decisions as on Windows' `WH_KEYBOARD_LL` do not exist on Wayland. The port needs a declared grab set that changes with the mode (browse mode grabs the quick keys, forms mode releases them). **inference** from facts |
| C2 | Linux key capture is open to any screen reader. | Mutter allows `KeyboardMonitor` only to the owner of `org.gnome.Orca.KeyboardMonitor` (or with its debug switch `A11yManagerWithoutAccessControl`, or in unsafe mode); KWin only to the owner of the same name (R-linux-18, R-linux-19, R-linux-20). libatspi requests `<app id>.KeyboardMonitor`, so a non-Orca AT asks under its own name and is refused (R-linux-22). **fact** (source code, main branches, 2026-11-05) |
| C3 | `xa11y-linux` covers Linux events. | Its subscription is per application (by pid), adds D-Bus match rules only, and does not call the registry's `RegisterEvent` (L-01). Applications using the ATK bridge send only the events some listener registered (R-linux-23). A screen reader needs a desktop-wide, registered subscription. Its "input" modules simulate input (XTest, uinput); nothing in the repository captures Linux keys. Its `lib.rs` doc still says Wayland input goes through the libei portal while the code uses `/dev/uinput` (L-01). **fact** (L-01, R-linux-23) |
| C4 | X11 is a usable Linux target. | GNOME's session is Wayland-only from Ubuntu 25.10 (GNOME 49) on (R-linux-14); Plasma 6.8 drops its X11 session (R-linux-15). X11 stays the established headless test path (Xvfb, used by Orca's CI in 2026, R-linux-25), but a product built only on X11 targets a shrinking set of desktops (Xfce, MATE, older LTS). **fact** / **inference** |
| C5 | WSL can run a desktop (the owner's reading). | Not an official capability (R-linux-01, R-linux-04). See section 4.1. **fact** |
| C6 | Disk is not a constraint. | old-pc: C: 11.8 GB free (the owner may free about 14 GB of caches), D: 342.8 GB free, E: 1.7 GB (lead agent, 2026-11-05). Dev box: C: 17.5 GB free and no other drive (L-08). Every Linux disk on old-pc must live on D:. **fact** |
| C7 | The dev box has no Linux. | It has the Store WSL 2.7.14 (file dated 2026-09-11) and a `.wslconfig` with `memory=6 GB`, `swap=8 GB`, but no registered distribution (L-08). The swap file goes to `%Temp%` on C: by default (R-linux-05), which a WSL session there would fill. **fact** (L-08) / **inference** |
| C8 | Microsoft documents systemd in WSL on Windows 10. | The 2022 announcement says systemd works on Windows 10 (R-linux-02); the current configuration page says the `[boot]` section of `wsl.conf` (where `systemd=true` goes) is only on Windows 11 and Server 2022 (R-linux-05). The two disagree; old-pc would have to show which is true. **open question** |

## 3. Question and scope

The decision: where Okur's Linux lab comes from, and whether making Linux the first
cross-platform target is worth it. In scope: WSL and WSLg on Windows 10 22H2, accessibility
inside Linux as a screen reader needs it (AT-SPI, Orca, audio, global keys), the lab options, the
coexistence with JAWS on old-pc, what the repository already has, and what an `okur-linux` driver
needs at minimum. Out of scope: Linux user-facing design beyond "Okur's Windows behaviour is the
reference", braille, packaging.

## 4. Findings

### 4.1 Q1: WSL, WSLg and desktops on Windows 10 22H2

1. **fact**: the Store WSL, with WSLg (Linux GUI apps) and systemd, became available on Windows 10
   21H1, 21H2 and 22H2 in November 2022, once KB5020030 was installed (R-linux-02). Microsoft's GUI
   apps page asks for Windows 10 build 19044 or later, WSL 2, and a vGPU driver for hardware
   OpenGL (R-linux-01). old-pc's build 19045.6456 is later than both.
2. **fact**: 19045.6456 is KB5066791 of 2025-10-14, Windows 10's last update without Extended
   Security Updates; Windows 10 support ended that day (R-linux-10). WSL ships through the Store,
   apart from Windows updates (R-linux-08); a July 2026 report says WSL's containers, a 2026 WSL
   update, install and run on Windows 10 (R-linux-12, **observed**). No source promises WSL updates
   for Windows 10 for any period. **open question**
3. **fact**: WSL 2 needs the Virtual Machine Platform; `wsl --install` enables what is needed and
   asks for a restart, from an administrator prompt (R-linux-07). old-pc is Windows 10 Enterprise
   and reports `HypervisorPresent = True`, so a hypervisor already runs (lead agent). Whether it is
   there for VBS, Virtual Machine Platform or Hyper-V is not known. `wsl.exe` exists in System32;
   that alone does not show whether WSL 2 or a distribution is installed. **open question**
4. **fact**: WSLg is a per-user "system distro" (Azure Linux 3.0) that runs Weston with its RDP
   backend, Xwayland and a PulseAudio server; Linux windows reach Windows as remoted windows over
   RDP, through an `mstsc.exe`-family client (`msrdc.exe` is the file in `C:\Program Files\WSL`
   on the dev box, L-08), and audio goes over the same RDP connection (R-linux-03).
5. **fact**: Microsoft: GUI app support relies on the Windows desktop and is not a full desktop;
   desktop-focused tools may not be supported (R-linux-01, page updated 2026-06-02). The WSL
   release list up to 3.0.5 (2026-11-03) has no desktop feature; 3.0's news is WSL containers
   (`wslc`, generally available in 3.0.1, 2026-09-29) (R-linux-04).
6. What the owner probably read about, all outside WSL itself:
   - **fact**: Kali's Win-KeX gives Kali in WSL 2 a desktop in a window, a "seamless" mode beside
     Windows' own windows, and an RDP "enhanced session" mode, each with optional sound
     (R-linux-33, updated 2025-06-18).
   - **observed**: community guides run Xfce in WSL through xrdp on port 3390 and connect with the
     Windows Remote Desktop client (R-linux-34); others run GNOME Shell on WSLg through Xwayland,
     which breaks on Ubuntu 26.04's Wayland-only GNOME 50 (R-linux-34, weak).
7. **inference**: what a full desktop in WSL on Windows 10 can be: Xfce or MATE on Xorg through
   xrdp (an X11 session; GNOME and Plasma no longer offer X11 sessions, C4) or a VNC server, shown
   in a Windows RDP or VNC client window. Neither is a Wayland GNOME session, which is where Linux
   screen reader users now are.
8. **fact**: systemd is the default for Ubuntu installed by `wsl --install`, needs WSL 0.67.6 or
   later, and does not keep the WSL instance alive by itself (R-linux-06). See C8 for Windows 10.

### 4.2 Q2: accessibility inside such a Linux

**The AT-SPI bus, GTK and Qt.**
- **fact**: AT-SPI2 runs without any desktop session: a private session bus (`dbus-run-session`),
  `at-spi-bus-launcher` and `at-spi2-registryd`, on an Xvfb display. Orca's integration harness
  does exactly this, forcing `GDK_BACKEND=x11` (R-linux-25); xa11y's upstream harness adds a small
  window manager (fluxbox) and sets the `org.a11y.Status` properties `IsEnabled` and
  `ScreenReaderEnabled` plus `ACCESSIBILITY_ENABLED=1` (R-linux-30).
- **fact**: real applications expose AT-SPI in such a stack: Orca's CI runs suites for GTK 3,
  GTK 4, VTE terminals, Chromium, Firefox, Atril and three file managers (R-linux-25); xa11y's CI
  covers AccessKit, Qt, GTK, Tauri, Electron and egui apps (R-linux-30).
- **observed**: in WSL without such a stack, GTK apps warn that they cannot reach the
  accessibility bus, and users silence the bridge with `NO_AT_BRIDGE=1` (R-linux-35). **inference**:
  inside WSL the same private stack as in CI works, whether or not systemd starts a user session;
  it does not depend on WSLg.

**Orca as a reference point.**
- **fact**: Orca runs in the same headless stack; its tests launch and drive a real Orca, press
  keys through `Atspi.generate_keyboard_event`, and read its speech from a dummy, silent speech
  server (R-linux-25). Orca 51.1 is the latest (2026-10-14; R-linux-24).
- Use: Orca shows which AT-SPI calls and events a working screen reader relies on. Its behaviour
  is not Okur's target (AGENTS.md §1.2).

**Audio.**
- **fact**: in WSLg, PulseAudio's sink and source plugins carry audio over RDP to Windows
  (R-linux-03); speech-dispatcher with espeak-ng would therefore play through the Windows
  machine's speakers. On old-pc that mixes Linux audio into the JAWS lab (section 7, risk K2).
- **fact**: speech-dispatcher's protocol (SSIP) sets rate and pitch on a -100 to 100 scale, has
  message priorities, and notifies begin, end, cancel and index marks, the last only when the
  synthesizer supports them (R-linux-37). Odilia, a Rust Linux screen reader, uses it (R-linux-39);
  `ssip-client-async` 0.19.0 is its Rust client (R-linux-38).
- **inference**: CI needs no sound device: Okur's own `SpeechLog` carries the words, and a null
  sink can prove that audio was produced if a test wants that.

**Global key capture (critical).**
- **fact, X11**: at-spi2-core's device API (used by Orca for its commands since Orca 46, R-linux-24)
  watches keys with XInput2 (`XI_KeyPress`/`XI_RawKeyPress` on the root window) and grabs chords
  with synchronous `XIGrabKeycode` grabs on the focused window (R-linux-22). This is what an X11
  `okur-linux` hook would copy. XTest-injected keys go through the X server like real ones.
- **fact, Wayland**: clients cannot see other clients' keys. GTK 4 no longer reports key presses to
  AT-SPI as GTK 3 did (R-linux-26). The answer is the `org.freedesktop.a11y.KeyboardMonitor`
  interface, served by the compositor: `WatchKeyboard` (see every key), `GrabKeyboard` (take every
  key), `SetKeyGrabs(modifiers, keystrokes)` (take listed chords, with custom modifier keys like an
  Insert or Caps Lock screen reader key, a lone modifier tap passed on unless pressed twice), and
  a `KeyEvent` signal with keysym, keycode and modifier state (R-linux-17). at-spi2-core supports it
  since 2.55.90 (R-linux-21); Mutter implements it (Mutter 48 on, per R-linux-28, weak), KWin
  implements it (R-linux-20; first release not verified), niri 25.08 implements it (R-linux-27).
- **fact**: the access rule of C2. Mutter: allowed sender `org.gnome.Orca.KeyboardMonitor`, or the
  debug property `A11yManagerWithoutAccessControl`, or unsafe mode (R-linux-18, R-linux-19).
  KWin: the caller must own `org.gnome.Orca.KeyboardMonitor`, else "access denied" (R-linux-20).
- **inference**: Okur could request that bus name on an unsandboxed session bus, but it would
  borrow Orca's name (against AGENTS.md §1.5's spirit), fail while Orca runs, and could break at
  any compositor release. The honest paths are an upstream change that admits any user-chosen
  screen reader, or the debug switch in the lab only. **open question for the owner** (O5)
- **inference, WSLg**: WSLg's Weston gets its input from RDP, not from the kernel; no source shows it
  implementing `KeyboardMonitor`. In WSLg a Linux screen reader can watch keys only for X clients
  through Xwayland. And every key a person types into a WSLg window passes through Windows, where
  JAWS's hook sees it first. WSLg is not a place to test a screen reader's keyboard.
- **fact**: the WSL kernel builds `uinput` and `evdev` as modules (6.6.107.1 configuration,
  R-linux-13), so a uinput keyboard can be created inside WSL. **inference**: no WSL compositor reads
  kernel input devices (WSLg uses RDP; headless compositors have no input devices), so uinput there
  tests nothing on the real path. The kernel path (evdev, libinput, compositor, `KeyboardMonitor`)
  exists only on real hardware or a full VM.
- **What Orca does on Wayland today**: **fact**: with at-spi2-core 2.55.90 or later and a compositor
  that implements the interface, the device API uses `KeyboardMonitor` (watch plus grabs); without
  it, it falls back to the legacy toolkit-reported keys (R-linux-22). Orca 50 moved mouse review to
  the same device API (`PointerLocator`) (R-linux-24, R-linux-16). **observed**: Orca on Plasma's
  Wayland session still had key bugs in 2025–2026 (Shift chords, key repeat), some fixed (R-linux-29).

**Which session type gives a screen reader what it needs.**

| Session | AT-SPI | Global keys | Audio | Verdict |
|---|---|---|---|---|
| Xvfb + private AT-SPI stack (CI, WSL headless) | yes (R-linux-25, R-linux-30) | yes, XI2 + XTest (R-linux-22) | none needed; null sink | the continuous test bed; tests the X11 hook path |
| Headless Mutter + private AT-SPI stack | yes (R-linux-29: dogtail's hermetic mode) | `KeyboardMonitor` behind the Orca-name rule; keys injected through Mutter's RemoteDesktop D-Bus API; whether those injected keys reach `KeyEvent` is **open** | none needed | the Wayland test bed, to be proved by probe P5 |
| WSLg (Weston, RDP) | yes with a private stack | X clients only; keys pass JAWS's hook | over RDP to Windows' speakers | unsuitable |
| Xorg through xrdp in WSL or a VM (Xfce) | yes | XI2 | xrdp's audio redirection | works, but an X11 desktop only (C4) |
| GNOME on Wayland on real hardware or a full VM | yes | `KeyboardMonitor`, real kernel input | real (hardware) or none (VM basic session) | the real target; needed for releases, not for every commit |

### 4.3 Q3: lab options compared

Disk and RAM figures marked ≈ are estimates (**inference**), not measurements: Ubuntu Desktop
needs about 25 GB of disk and 4 GB of RAM (6 GB from 26.04) (R-linux-42, secondary); a headless
test distro (base system, Xvfb, AT-SPI, GTK, Mutter, Chrome, Firefox, speech-dispatcher, a Rust
toolchain and one Linux `target` directory) ≈ 15–20 GB.

| Option | Cost | Owner effort | Fidelity: desktop / a11y / audio / keyboard | Disk goes to | Impact on old-pc's JAWS recordings |
|---|---|---|---|---|---|
| A. GitHub Actions Linux jobs (Xvfb + private AT-SPI; later headless Mutter) | free for public repos, 4 vCPU, 16 GB RAM, 14 GB SSD per job (R-linux-31) | none | none / real AT-SPI, real Chrome, Firefox, GTK, Qt / log or null sink / XTest or compositor injection | the runner | none |
| B. WSL on the dev box, headless, no WSLg | free; WSL 2.7.14 already installed (L-08) | approve a distro; free ≈20 GB on C: (17.5 GB free, no D:) | as A, interactive for agents | C: (only drive) | none |
| C. Dedicated mini PC (Intel N150 class, 16 GB, 512 GB) | ≈ $150–250 (2024–25 listings, R-linux-41) | buy, install Ubuntu 26.04 or Fedora (GNOME, Wayland), SSH, LAN | full: real desktop, real a11y, real audio, real kernel keyboard path, Orca beside | its own SSD | none |
| D. Hyper-V VM on old-pc (Enterprise has Hyper-V, R-linux-09) | free | enable Hyper-V (admin, reboot), create the VM on D: | real GNOME Wayland desktop, real a11y, real kernel keyboard path (Hyper-V's synthetic keyboard, or uinput inside); no sound in the basic session (**inference**) | D: (342.8 GB free) | low if the VM is saved (`Stop-VM -Save`) before every round and its console window is never open; CPU and RAM shared with JAWS when running |
| E. VirtualBox or VMware Player on old-pc | free | install; with a hypervisor present, VirtualBox runs through Windows' hypervisor API in a slower mode (R-linux-36, community) | as D, slower | D: | as D, plus a third-party driver stack on the lab |
| F. WSL with a desktop on old-pc (xrdp/VNC, or Win-KeX) | free | `wsl --install` (admin, reboot), xrdp setup, distro on D: (`--location`) | X11 desktop only (C4); keyboard through JAWS's hook when typed; audio into old-pc's speakers | D: (distro), C: (swap file unless moved) | highest: focus stealing (R-linux-11), audio, prompts, CPU |
| G. Cloud VM with a desktop | from €6.49 a month at Hetzner (vendor page, R-linux-40); prices rose in 2026 | account, payment, SSH keys | real kernel and compositor, virtual display, no sound | the provider | none |

### 4.4 Q4: coexistence with JAWS on old-pc

- **inference**: enabling the Virtual Machine Platform or Hyper-V on old-pc does not add a
  hypervisor, since one already runs (`HypervisorPresent = True`). Microsoft says these features
  and memory integrity can cost some performance in games and does not quantify it (R-linux-35,
  via coverage). No source on JAWS and Hyper-V was found. **open question**: measure, do not assume
  (section 6, step W1-6).
- **fact**: `wsl --shutdown` stops every distribution and the WSL VM at once; `wsl --terminate`
  stops one (R-linux-08). Configuration takes effect only after the VM has stopped, about 8 seconds
  after the last shell closes (R-linux-05). The VM's idle timeout setting is Windows 11 only
  (R-linux-05). For Hyper-V, saving a VM frees its RAM and CPU (**inference**, standard Hyper-V
  behaviour, not re-checked here).
- **observed**: WSLg's `msrdc.exe` steals focus at random once a WSL instance starts, even without
  GUI apps; open since 2022-11-17, reported on Windows 10 22H2 (19045.6466) as well; turning
  `guiApplications` off helps some reporters and not others (R-linux-11). A first-run console
  window opens when a new distribution starts (R-linux-07).
- **inference**: any Linux window shown on old-pc (WSLg, an RDP or VNC client, VMConnect) is a
  Windows window: JAWS reads it, its hook sees the keys typed into it, and it can take focus. The
  lab must never show Linux on old-pc's screen during or near a round.
- **Remote driving**: from OpenDesk exec, `wsl.exe -d <distro> -e <command>` runs a Linux command
  (R-linux-08); inside, `xdotool` (XTest, X11 only), `ydotool` (uinput) or Mutter's RemoteDesktop
  D-Bus API inject keys. An SSH server in WSL listening for the LAN would need a port proxy and a
  firewall rule (admin, and possibly the kind of Windows Security Alert that once broke
  recordings) on an unpatched Windows 10 (R-linux-10): avoid it. For a Hyper-V VM, SSH outward
  from old-pc to the VM needs no listening port on Windows. **open question**: whether OpenDesk exec
  runs in `ailabs`' interactive session (WSL distributions are per user), and whether starting
  `wsl.exe` from it opens a console window that takes focus.
- **RAM and disk**: old-pc has 15.9 GB of RAM and a 4-core, 8-thread i7-7700HQ (lead agent). WSL's
  VM takes up to 50% of RAM by default, and its swap file is 25% of RAM in `%Temp%` on C:
  (R-linux-05): cap it (`memory=4GB`, `processors=2`, `swap=0` or `swapFile` on D:) and install the
  distribution on D: (`wsl --install --location`, or `[general] distributionInstallPath`;
  R-linux-08, R-linux-05). A Hyper-V VM: 4–6 GB RAM, 2 vCPUs, ≈40 GB disk on D:. Neither belongs in
  the Dropbox folder (**inference**: a growing VHDX would sync continuously).

### 4.5 Q5: what the repository has for Linux

- **`crates/xa11y/xa11y-linux`** (10,088 lines with tests, L-01): AT-SPI2 over zbus: tree reading
  (the `Provider`: roles, states, names, values, actions, window management through EWMH and Sway),
  push events per application (focus, state, name, value, text, children, selection, announcement,
  window create, destroy, activate, deactivate), input simulation (XTest on X11, `/dev/uinput` on
  Wayland), screenshots (X11 `GetImage`, Wayland portal). No key capture, no desktop-wide
  subscription, no `RegisterEvent` (C3). It keeps a crate-wide `indexing_slicing` allowance and has
  never been through clippy on Linux since the lints were added (VENDORED.md).
- **`crates/okur-runtime`** (L-02): the ports. `KeyboardHook` (callback per key returning
  `KeyAction`), `EventSource` with an `EventSink`, `TtsEngine` and `SynthesizerFactory` (which
  takes the closed `Synthesizer` enum `Auto`, `OneCore`, `Sapi5`, `NoSpeech`, `okur-core`
  `settings.rs`), `UiSpawner`, and `Platform`, in which actions, clock, sounds and start options are
  optional. All have mocks. Pitch is documented on SAPI's -10 to 10 scale; speech-dispatcher's is
  -100 to 100 (R-linux-37).
- **`crates/okur-windows-model`**: Windows data as data (traces, UIA and IA2 snapshots, conversion
  to the AOM and to engine events, privacy). It is the pattern a Linux model follows.
- **`crates/okur-ui`** (L-06): off Windows it reads and drops what Okur sends, so Okur runs without
  dialogs. `helper-ui-architecture.md` plans GTK 4 for Linux, best effort.
- **CI** (L-04): the `portable` job tests the portable crates (from `check-crate-layers.ps1
  -Portable`) on Ubuntu and macOS and runs `cargo check --workspace --all-targets` there, as
  information only (`continue-on-error: true`). The plan of 2026-11-03 reports that the workspace
  checks for Linux with no warning.
- **`.agents/crate-layers.json`** (L-03): kinds and allowed edges; `zbus` is already forbidden in
  neutral crates. No Linux crate is listed beyond the vendored ones.
- **Corpus** (L-07): 1,942 scenario files; 1,623 drive Chrome on HTML fixtures (44 HTML fixtures),
  319 drive Windows desktop applications. **inference**: the 1,623 web scenarios are the portable
  ones. JAWS's speech for a web page in Chrome does not depend on Windows, so the same keys on the
  same page in Chrome on Linux have a recorded target.

**The minimum `okur-linux` driver** (a `platform` crate, gated `#![cfg(target_os = "linux")]` at
its root, beside `okur-windows`), and what each piece needs:

| Port | Minimum | Built on | Gap to close first |
|---|---|---|---|
| `KeyboardHook` | X11: XI2 watch plus synchronous grabs of the chords Okur takes in the current mode, the Okur key as a custom modifier; Wayland: `KeyboardMonitor` `WatchKeyboard` + `SetKeyGrabs` | `x11rb` (already a dependency of xa11y-linux) with the XInput extension; zbus | C1 (declared grab sets: the key model of `crate-structure-proposal.md` §13.3 gains "which chords are consumed in this mode"); C2 (Orca-name rule) |
| `EventSource` | desktop-wide AT-SPI events, registered with the registry, mapped to `AccessibilityEvent` | xa11y-linux, extended in place (VENDORED.md: extend there) | C3; a Linux `FocusEvent` builder; menus have no AT-SPI open/close signal (xa11y's own test notes) |
| `SynthesizerFactory`/`TtsEngine` | speech-dispatcher over SSIP (rate, pitch, stop, index marks for Say All) | `ssip-client-async` or a small SSIP client of our own | the closed `Synthesizer` enum (T10: a synthesizer id from the platform's registry); pitch and rate scales |
| `UiSpawner` | the plain spawner; `okur-ui` drops messages | existing | GTK 4 renderer later |
| `ApplicationActions` | focus (`Component.GrabFocus`), default action (`Action.DoAction`), caret (`Text.SetCaretOffset`) | xa11y-linux | none for a first web reading |
| Program | `okur` built for Linux, choosing the driver by `cfg` | `okur` | the composition root's Linux branch |
| Parity model | `okur-linux-model` (neutral): AT-SPI snapshot and event-trace formats and their conversion to the AOM and to engine events; replayed on every OS | serde only | the `PlatformModel` trait of §13.3 |

### 4.6 What can be verified on Linux, and how

JAWS does not run on Linux. The reference is "the same command says the same thing": Okur on
Linux says what JAWS says on Windows for the same web page and keys, and what Okur on Windows says
for the same kind of control elsewhere. Orca is consulted for API usage only.

| # | Check | Reference | Where | Hermetic? |
|---|---|---|---|---|
| V1 | Portable crates' tests, parity replay of Windows recordings | as today | CI, every OS | yes |
| V2 | Linux snapshots of the 44 HTML fixtures, converted by `okur-linux-model`, replayed with each web scenario's keys | the scenario's JAWS evidence, through the existing comparator; differences that come from Chrome's AT-SPI exposure become written platform divergences | captured on a Linux lab, replayed in CI on every OS | yes once captured |
| V3 | Live: `okur-linux` reads Chrome on a fixture under Xvfb; keys injected with XTest pass the real XI2 hook; speech from `SpeechLog` | JAWS evidence and Okur-on-Windows parity output for the same scenario | Linux CI job, `#[ignore = "live: ..."]` tests run with `--ignored` | no (live job) |
| V4 | GTK 3/4 and Qt controls announced by role and state like their Windows counterparts (a check box, a combo box, a dialog) | Okur's Windows rules, as invariants | CI | yes, with AT-SPI snapshots |
| V5 | Wayland: `KeyboardMonitor` watch and grabs, real kernel key path, speech latency with real audio (`okur-lab latency`), Orca beside for API comparison | V3's expected speech; latency figures | mini PC or VM, per milestone | no |

## 5. Recommendation

Make Linux the first cross-platform target, as a time-boxed proving ground for the ports and the
parity model, with this ranked lab shortlist:

1. **GitHub Actions Linux jobs** (option A): the continuous lab. Free, unlimited for this public
   repository (R-linux-31), no hardware, no effect on old-pc, and the method is proven by Orca's
   and xa11y's own CI (R-linux-25, R-linux-30). It covers V1 to V4.
2. **WSL on the dev box, headless** (option B): the same scripts as CI, for agents to debug
   without waiting for a CI run. No WSLg (`guiApplications=false`), no audio device, nothing on the
   dev box's screen. It needs ≈20 GB that C: does not have today, and it counts as the dev box's
   one heavy job while it builds.
3. **A dedicated Linux mini PC** (option C) for V5: a real GNOME Wayland desktop, the real keyboard
   path, real audio, Orca beside Okur, always available, nothing shared with JAWS. If the owner
   prefers no purchase: **a Hyper-V VM on old-pc's D:** (option D), run only between rounds and
   saved during them.
4. Cloud VM (option G): workable for V5 without sound; monthly cost and an account.
5. WSL with a desktop on old-pc (option F): not recommended (sections 4.2, 4.4 and 7). The owner's
   question has a short answer: yes, old-pc could run a WSL desktop, through xrdp or Win-KeX on D:,
   but it would be an X11 desktop behind JAWS's hook, in a window that can take focus, with audio
   in the room the recordings listen to, on the lab's scarcest resource: its time.

What Linux first does and does not buy for macOS (**inference**): it builds the second
implementation of every port, the `PlatformModel` trait, a second model crate, the per-platform
key table and the declared grab sets, and a second CI lab. It does not answer macOS's own
questions: the Accessibility and Input Monitoring permissions, `CGEventTap` (per-event, like
Windows), `AXObserver` per application, and macOS speech. Whether GitHub's macOS runners can grant
those permissions to a test binary was not checked. **open question**

## 6. Phased plan

### Week 1: probes and skeleton (no hardware, old-pc untouched)

Each probe is a small program or script run in a CI job (or WSL on the dev box once approved),
with a pass criterion that can fail. What ran and what it printed goes in the handoff.

| Step | What | Pass criterion |
|---|---|---|
| W1-1 | CI job `linux-a11y-probe` (informational): Xvfb, `dbus-run-session`, `at-spi-bus-launcher`, `at-spi2-registryd`, `org.a11y.Status` set, a window manager; Chrome on `corpus/fixtures/doc-reading.html`; the vendored `xa11y` CLI dumps the tree | the fixture's headings and links appear by name and role |
| W1-2 | Probe P2: desktop-wide events with `RegisterEvent`, no sender filter; Tab through a GTK 4 app with XTest | one focus event per Tab, in order, from the right application |
| W1-3 | Probe P3: X11 hook: XI2 watch plus a synchronous grab of Insert+Down; keys sent with XTest | every key seen; the grabbed chord does not reach a GTK entry, other keys do |
| W1-4 | Probe P4: speech-dispatcher with espeak-ng into a null sink | begin, end and index-mark notifications for a marked text; non-silent audio on the sink's monitor |
| W1-5 | Probe P5: headless Mutter (dogtail's hermetic pattern, R-linux-29): `WatchKeyboard` under the name `org.okur.Okur.KeyboardMonitor`, then with the debug switch; keys injected through Mutter's RemoteDesktop API | the access rule confirmed (refused, then allowed); whether injected keys arrive as `KeyEvent` |
| W1-6 | Crate skeleton: `okur-linux` (platform) and `okur-linux-model` (neutral) in `crate-layers.json`, `#![cfg(target_os = "linux")]` at the driver's root, ports returning `RuntimeError` until built | `check-crate-layers.ps1` clean; the portable job lists `okur-linux-model` |
| W1-7 | Only if the owner chooses option D: on old-pc, between rounds, one baseline round before enabling Hyper-V and the same round after | the normalized transcripts and `okur-lab latency` figures of the two rounds match within their usual trial variance |

### Weeks 2 to 6: continuous testing

1. `okur-linux` keyboard (X11), events and speech, enough for the virtual cursor on a web page.
2. A live CI job (V3), informational: a first set of web scenarios (document reading 01–05, quick
   navigation) run by `okur-linux` on Chrome under Xvfb; its results generated by a tool into a
   Linux conformance baseline that may only improve, like the parity baseline.
3. `okur-linux-model` and V2: Linux snapshots of the 44 HTML fixtures, captured by tooling on a
   Linux lab with provenance (needs O6), replayed in CI on every OS.
4. V4 invariants for GTK and Qt controls.
5. The portable job becomes blocking for `okur-linux-model`; the live job stays informational
   until the owner decides otherwise.

### Later: the real-desktop lab (once O2 is decided)

GNOME on Wayland, Okur beside Orca, per milestone: the `KeyboardMonitor` path (with the debug
switch until O5 is resolved), real audio latency, a check that nothing differs from the CI
results. On a mini PC: SSH from the dev box, or OpenDesk if it has a Linux peer (**open question**).

### What the owner decides or does

| # | Decision or action | Needs |
|---|---|---|
| O1 | Linux first, time-boxed, as the proving ground before macOS; Linux stays Tier 2 for users | a decision |
| O2 | The real-desktop lab: buy a mini PC (recommended), or a Hyper-V VM on old-pc's D:, or none yet | purchase, or admin rights and a reboot of old-pc |
| O3 | Dev box: free ≈20 GB on C: and approve a headless WSL distribution; agree to `guiApplications=false` and a `swap` that does not land on C: in the dev box's `.wslconfig` (it is the owner's file) | free space; approval |
| O4 | If option D: enable Hyper-V on old-pc (`Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All`, R-linux-09), VM files on D: outside Dropbox, a Defender exclusion if scans slow it | administrator, reboot, attended (W1-7) |
| O5 | Whether Okur asks GNOME (Mutter) and KDE (KWin) to let screen readers other than Orca use `KeyboardMonitor` | public communication in the owner's name |
| O6 | Whether AT-SPI snapshots and traces captured by tooling on a Linux lab (CI or mini PC) may enter the corpus as platform data, beside the Windows snapshots captured on old-pc (they are Chrome's tree, not JAWS's speech) | a rule change in `verification-loop.md` |
| O7 | Virtualization in old-pc's firmware is likely on (a hypervisor already runs); nothing to do unless Hyper-V refuses to start | possibly BIOS access |

### Checks on old-pc for the lead agent, between recordings, read-only

Only if options D or F are still considered. None needs a window; run them once with the focus
tracer watching, since a console may appear.
- `Get-CimInstance Win32_OptionalFeature | Where-Object Name -in 'Microsoft-Windows-Subsystem-Linux','VirtualMachinePlatform','Microsoft-Hyper-V-All','HypervisorPlatform'`
- `Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard`
  (`VirtualizationBasedSecurityStatus`, `SecurityServicesRunning`: why a hypervisor is present)
- `Get-AppxPackage *WindowsSubsystemForLinux*` (the Store WSL, without starting it)
- `Test-Path $env:USERPROFILE\.wslconfig`
- `whoami` and `[Environment]::UserInteractive` from OpenDesk exec (WSL distributions are per user)
- Only if the Store WSL is installed: `wsl --version`, `wsl --status`, `wsl -l -v`

## 7. Risks to the JAWS lab, and how to avoid them

| # | Risk | Avoidance |
|---|---|---|
| K1 | Focus stolen by WSLg's `msrdc.exe` (R-linux-11) or a first-run console (R-linux-07) | no WSL on old-pc (recommended); if used: `guiApplications=false`, `wsl --shutdown` before every round, a focus-trace check after setup |
| K2 | Linux audio in the room `okur-lab wait-silence` measures | no Linux audio on old-pc: no WSLg audio, a null sink, a VM without a sound device |
| K3 | CPU and RAM taken from JAWS during a round (4 cores, 15.9 GB) | WSL shut down or the VM saved before each round; caps (`memory=4GB`, `processors=2`; 2 vCPUs for a VM) |
| K4 | A feature change and reboot alter the lab (timing, speech) | one attended change between rounds, a baseline round before and after (W1-7); the voice rate and the 1% volume re-checked after the reboot |
| K5 | Prompts: Windows Security Alert for a listening port, network-change notices when a virtual switch appears | no listening servers on old-pc; outbound SSH only; Hyper-V's Default Switch created during the attended change |
| K6 | C: full (11.8 GB free) | every VHDX, VM disk and swap file on D:; `distributionInstallPath`, `--location`, `swapFile` or `swap=0` |
| K7 | Keys typed into a Linux window pass JAWS's hook; JAWS reads Linux windows | never show Linux on old-pc's screen; inject keys inside Linux |
| K8 | Dropbox syncing a growing virtual disk | VM and distro folders outside Dropbox |
| K9 | New network exposure on a Windows 10 that has had no updates since 2025-10-14 (R-linux-10) | no inbound services; or no Linux on old-pc at all |
| K10 | old-pc's time: rounds are always queued (the owner's rule to keep it busy) | Linux work off old-pc (options A, B, C) |

## 8. Domain terms

| Term | Meaning here |
|---|---|
| AT-SPI2 | Linux's accessibility protocol over D-Bus: applications expose objects, a registry tracks them, screen readers query them and listen to their events |
| a11y bus | the separate D-Bus bus AT-SPI uses, started by `at-spi-bus-launcher` |
| registryd | `at-spi2-registryd`, the registry: lists applications and tells them which events listeners want |
| `KeyboardMonitor` | the compositor's D-Bus interface `org.freedesktop.a11y.KeyboardMonitor` that gives a screen reader keys on Wayland |
| compositor | on Wayland, the program that is the display server and window manager in one (Mutter for GNOME, KWin for KDE, Weston in WSLg) |
| Xvfb | an X server with no screen, for tests |
| XTest, XInput2 (XI2) | X11 extensions: XTest injects input, XI2 reports and grabs it |
| uinput, evdev | the kernel's virtual input device and its input event interface |
| WSLg | WSL's GUI support: Weston, Xwayland and PulseAudio in a system distro, remoted over RDP |
| SSIP | speech-dispatcher's protocol |
| declared grab set | (proposed) the chords Okur consumes in the current mode, given to the platform in advance |

## 9. Implications for requirements and tests

1. `okur-linux` exists only after probes P2 to P5 have run and their output is in a handoff.
2. The `KeyboardHook` port gains a declared grab set per mode; Windows and macOS may ignore it,
   Linux needs it. A test: switching from browse to forms mode changes the declared set, and the
   mock hook records the change.
3. A Linux `EventSource` registers its event types with the registry. A live test: a GTK app
   started after Okur still delivers focus events.
4. No Okur message, bus name or setting says "Orca"; any use of `org.gnome.Orca.KeyboardMonitor`
   is confined to lab code behind an explicit lab flag (AGENTS.md §1.5 by analogy).
5. The Linux synthesizer maps Okur's rate and pitch to SSIP's -100 to 100; a unit test pins the
   mapping at both ends and the middle.
6. Live Linux tests are `#[ignore = "live: ..."]` and run only in the Linux CI job; `cargo test`
   stays hermetic.
7. A Linux conformance baseline is generated by a tool and may only improve; differences caused by
   Chrome's Linux exposure are written divergences with the scenario id.
8. Nothing in the Linux lab runs on old-pc unless O2 chooses option D, and then only between rounds
   with K1 to K9 applied.

## 10. Sources

All web sources accessed 2026-11-05. Strength: high (official documentation or source code),
medium (official tracker, distribution announcement, reputable news), low (community guide,
search summary, vendor price page).

| ID | Title | URL | Date | Kind | Strength | Supports |
|---|---|---|---|---|---|---|
| R-linux-01 | Run Linux GUI apps with WSL | https://learn.microsoft.com/en-us/windows/wsl/tutorials/gui-apps | 2025-06-08, updated 2026-06-02 | official | high | Windows 10 19044+; vGPU driver; not a full desktop |
| R-linux-02 | WSL in the Microsoft Store is now generally available on Windows 10 and 11 | https://devblogs.microsoft.com/commandline/the-windows-subsystem-for-linux-in-the-microsoft-store-is-now-generally-available-on-windows-10-and-11/ | 2022-11-22 | official | high | Windows 10 21H1–22H2 with KB5020030; WSLg and systemd on Windows 10 |
| R-linux-03 | microsoft/wslg README | https://github.com/microsoft/wslg | current | official | high | system distro, Weston RDP backend, Xwayland, PulseAudio over RDP; Windows 10 supported |
| R-linux-04 | microsoft/WSL releases | https://github.com/microsoft/WSL/releases | 3.0.1 2026-09-29; 3.0.5 2026-11-03 | official | high | no desktop feature; WSL containers GA |
| R-linux-05 | Advanced settings configuration in WSL | https://learn.microsoft.com/en-us/windows/wsl/wsl-config | 2026-10-16 | official | high | memory 50%, swap 25% in %Temp%, guiApplications, Windows 11-only keys, `[boot]` Windows 11 only, distributionInstallPath, 8-second rule |
| R-linux-06 | Use systemd to manage Linux services with WSL | https://learn.microsoft.com/en-us/windows/wsl/systemd | 2025-01-13 | official | high | systemd default for Ubuntu; WSL 0.67.6+; services do not keep WSL alive |
| R-linux-07 | Install WSL | https://learn.microsoft.com/en-us/windows/wsl/install | 2025-06-09 | official | high | Windows 10 2004+; admin and restart; console window on first launch |
| R-linux-08 | Basic commands for WSL | https://learn.microsoft.com/en-us/windows/wsl/basic-commands | 2025-12-01 | official | high | `--location`, `--no-launch`, `--import`, `--shutdown`, `--terminate`, `--status`; Store updates |
| R-linux-09 | Install Hyper-V in Windows and Windows Server | https://learn.microsoft.com/en-us/windows-server/virtualization/hyper-v/get-started/install-hyper-v | 2025-05-23, updated 2026-02-16 | official | high | Windows 10 Pro/Enterprise; SLAT; 4 GB; enable command; restart; not Home |
| R-linux-10 | KB5066791, OS Build 19045.6456 (via search of Microsoft release health and Windows Latest) | https://learn.microsoft.com/en-us/windows/release-health/resolved-issues-windows-10-22H2 | 2025-10-14 | official (search summary) | medium | 19045.6456 = last non-ESU update; end of support 2025-10-14 |
| R-linux-11 | msrdc.exe randomly stealing focus (wslg #894) | https://github.com/microsoft/wslg/issues/894 | opened 2022-11-17, comments to 2026-10 | tracker | medium (observed) | focus stealing, also on 19045; mixed workaround results |
| R-linux-12 | Windows 10 is still getting Windows 11 features, but only for developers | https://www.windowslatest.com/2026/07/09/windows-10-is-still-getting-windows-11-features-but-its-only-for-developers/ | 2026-07-09 | news | low–medium | WSL containers run on Windows 10 (search summary) |
| R-linux-13 | WSL2 kernel config-wsl (linux-msft-wsl-6.6.y, 6.6.107.1) | https://raw.githubusercontent.com/microsoft/WSL2-Linux-Kernel/linux-msft-wsl-6.6.y/arch/x86/configs/config-wsl | current | official source | high | `CONFIG_INPUT_UINPUT=m`, `CONFIG_INPUT_EVDEV=m`, `CONFIG_SND=m` |
| R-linux-14 | Ubuntu 25.10 drops support for GNOME on Xorg | https://discourse.ubuntu.com/t/ubuntu-25-10-drops-support-for-gnome-on-xorg/62538 | 2025-06-10 | distribution announcement | medium–high | GNOME session Wayland-only; X11 apps through Xwayland |
| R-linux-15 | KDE Plasma 6.8 will go Wayland-exclusive | https://www.phoronix.com/news/KDE-Plasma-68-Wayland-Exclusive | 2025 | news | medium | Plasma 6.8 drops the X11 session |
| R-linux-16 | GNOME 50 release notes | https://release.gnome.org/50/ | 2026-03-18 | official | high | Orca mouse review on Wayland (the note does not itself state the X11 removal) |
| R-linux-17 | Mutter `org.freedesktop.a11y.xml` | https://gitlab.gnome.org/GNOME/mutter/-/raw/main/data/dbus-interfaces/org.freedesktop.a11y.xml | main, 2026-11-05 | official source | high | KeyboardMonitor and PointerLocator methods and signals |
| R-linux-18 | Mutter `meta-a11y-manager.c`, `meta-dbus-access-checker.c` | https://gitlab.gnome.org/GNOME/mutter/-/raw/main/src/backends/meta-a11y-manager.c | main, 2026-11-05 | official source | high | allowed sender `org.gnome.Orca.KeyboardMonitor`; unsafe mode and debug bypass |
| R-linux-19 | Mutter `org.gnome.Mutter.DebugControl.xml` | https://gitlab.gnome.org/GNOME/mutter/-/raw/main/data/dbus-interfaces/org.gnome.Mutter.DebugControl.xml | main, 2026-11-05 | official source | high | `A11yManagerWithoutAccessControl` property |
| R-linux-20 | KWin `src/a11ymanager.cpp` (GitHub mirror) | https://raw.githubusercontent.com/KDE/kwin/master/src/a11ymanager.cpp | master, 2026-11-05 | official source | high | serves the interface; caller must own `org.gnome.Orca.KeyboardMonitor` |
| R-linux-21 | at-spi2-core NEWS | https://gitlab.gnome.org/GNOME/at-spi2-core/-/raw/main/NEWS | to 2.60.x | official | high | 2.39.1 device API; 2.55.90 KeyboardMonitor grabs; X11 device fixes |
| R-linux-22 | at-spi2-core `atspi-device.c`, `atspi-device-x11.c`, `atspi-device-a11y-manager.c` | https://gitlab.gnome.org/GNOME/at-spi2-core/-/tree/main/atspi | main, 2026-11-05 | official source | high | backend choice; XI2 watch and `XIGrabKeycode` sync grabs; requests `<app id>.KeyboardMonitor` |
| R-linux-23 | at-spi2-core `atspi-event-listener.c`, `atk-adaptor/event.c` | https://gitlab.gnome.org/GNOME/at-spi2-core/-/tree/main | main, 2026-11-05 | official source | high | listeners call `RegisterEvent`; the ATK bridge emits only needed signals |
| R-linux-24 | Orca NEWS | https://gitlab.gnome.org/GNOME/orca/-/raw/main/NEWS | to 51.1, 2026-10-14 | official | high | device API since 41/46; keysym grabs 48; mouse review device API 50 |
| R-linux-25 | Orca tests README, integration wrapper, keyboard harness, CI | https://gitlab.gnome.org/GNOME/orca/-/tree/main/tests | main, 2026-11-05 | official source | high | Xvfb + private AT-SPI under dbus-run-session; GDK_BACKEND=x11; 350+ runs, 2,000+ cases; keys via Atspi.generate_keyboard_event; dummy speech server; CI suites |
| R-linux-26 | Need API for screen reader to monitor/grab keys (mutter #2078) | https://gitlab.gnome.org/GNOME/mutter/-/issues/2078 | c. 2021 | tracker | medium | GTK 4 drops toolkit key reporting; compositor API needed |
| R-linux-27 | niri 25.08 released (LWN) | https://lwn.net/Articles/1036625/ | 2025-09-03 | news | medium | niri implements KeyboardMonitor for Orca |
| R-linux-28 | RHEL-82072 (via search) | https://issues.redhat.com/browse/RHEL-82072 | 2025 | vendor tracker (search summary) | low | the interface landed in Mutter 48 |
| R-linux-29 | dogtail 2.1.2 on PyPI; KDE bugs 507144, 507545, 519143 | https://pypi.org/project/dogtail/ ; https://bugs.kde.org/show_bug.cgi?id=519143 | 2026-10-26; 2025–2026 | official project page; tracker | high; medium | hermetic mode: private buses, bare Mutter, RemoteDesktop input, CI container; Orca-on-Plasma-Wayland key bugs |
| R-linux-30 | xa11y `scripts/run_integ_tests.sh`, `scripts/setup_linux_a11y.sh`, CI workflow | https://github.com/xa11y/xa11y/blob/main/.github/workflows/ci.yml | main, 2026-11-05 | official source (upstream of our vendored crates) | high | Xvfb + fluxbox + private AT-SPI; Status flags; app matrix on ubuntu-latest; headless sway job; `modprobe uinput` on runners |
| R-linux-31 | GitHub-hosted runners | https://docs.github.com/en/actions/reference/runners/github-hosted-runners | current | official | high | public repos free and unlimited; 4 vCPU, 16 GB, 14 GB SSD; passwordless sudo |
| R-linux-32 | Ubuntu 24.04 runner image readme | https://raw.githubusercontent.com/actions/runner-images/main/images/ubuntu/Ubuntu2404-Readme.md | 20261026.354.1 | official | high | xvfb, dbus, Chrome, Chromium, Firefox installed |
| R-linux-33 | Win-KeX | https://www.kali.org/docs/wsl/win-kex/ | 2025-06-18 | official (Kali) | high | window, seamless and enhanced-session modes, sound |
| R-linux-34 | community WSL desktop guides (xrdp + Xfce on 3390; GNOME on WSLg gist) | https://dev.to/darksmile92/linux-on-windows-wsl-with-desktop-environment-via-rdp-522g ; https://gist.github.com/tdcosta100/e28636c216515ca88d1f2e7a2e188912 | various | community | low | xrdp desktops; fragile GNOME-on-WSLg |
| R-linux-35 | WSL issue #2016 and Ubuntu at-spi2-core bug reports (via search) | https://github.com/microsoft/WSL/issues/2016 | 2017– | tracker | low–medium | accessibility-bus warnings in WSL; `NO_AT_BRIDGE=1` |
| R-linux-36 | Options to optimize gaming performance in Windows 11 (via coverage); VirtualBox "green turtle" (Kicksecure wiki) | https://support.microsoft.com/article/a255f612-2949-4373-a566-ff6f3f474613 ; https://www.kicksecure.com/wiki/VirtualBox/Green_Turtle_Issue | 2022; current | official (via news); community | medium; low | VMP and memory integrity may cost performance, unquantified; VirtualBox slower under a hypervisor |
| R-linux-37 | speech-dispatcher `doc/ssip.texi` | https://raw.githubusercontent.com/brailcom/speechd/master/doc/ssip.texi | master | official | high | rate and pitch -100..100; priorities; event and index-mark notifications |
| R-linux-38 | crates.io: `atspi` 0.30.0, `ssip-client-async` 0.19.0, `accesskit_unix` 0.24.1 | https://crates.io/crates/ssip-client-async | 2026-05-06; 2025-06-08; 2026-10-25 | registry | high | versions |
| R-linux-39 | Odilia README | https://github.com/odilia-app/odilia | current | project | medium | Rust Linux screen reader, beta, not production ready, needs speech-dispatcher |
| R-linux-40 | Hetzner Cloud | https://www.hetzner.com/cloud/ | 2026-11-05 | vendor claim | low | from €6.49/month; cost-optimized line unavailable |
| R-linux-41 | N150 mini PC listings (Neowin, Liliputing, mightygadget, via search) | https://www.neowin.net/amp/this-intel-mini-pc-with-16-gb-ddr4-ram-and-512-gb-nvme-ssd-is-just-over-150/ | 2024–2025 | reviews | low | ≈ $150–250 for 16 GB / 512 GB |
| R-linux-42 | Ubuntu 26.04 raises desktop minimum RAM (Linuxiac) | https://linuxiac.com/ubuntu-26-04-lts-raises-desktop-minimum-ram-requirement/ | 2026 | news | low–medium | 6 GB RAM, 25 GB disk |

Repository sources, read 2026-11-05:

| ID | What | Supports |
|---|---|---|
| L-01 | `crates/xa11y/xa11y-linux` (`Cargo.toml`, `lib.rs`, `events.rs`, `input.rs`, `wayland_input.rs`, `session.rs`, tests) | coverage, per-pid subscription, no `RegisterEvent`, simulation only, stale libei doc |
| L-02 | `crates/okur-runtime/src` (`hook.rs`, `events.rs`, `synthesizer.rs`, `host.rs`, `ui_host.rs`) | the ports and their shapes |
| L-03 | `.agents/crate-layers.json` | kinds, allowed edges, `zbus` forbidden in neutral crates |
| L-04 | `.github/workflows/ci.yml` | the portable job on Ubuntu and macOS, informational |
| L-05 | `docs/references/crate-structure-proposal.md` §9, §13 | `PlatformModel`, key model, synthesizer id |
| L-06 | `crates/okur-ui/src/main.rs` | renderer drains input off Windows |
| L-07 | `corpus/scenarios`, `corpus/fixtures` (counts by `target_app` and extension) | 1,942 scenarios: 1,623 Chrome, 319 desktop; 44 HTML fixtures |
| L-08 | dev box: `C:\Program Files\WSL` (`wsl.exe` 2.7.14.0, 2026-09-11; `msrdc.exe`, `wslg.exe`), `%USERPROFILE%\.wslconfig`, `HKCU\...\Lxss` (no distributions), free space (C: 17.5 GB, no other drive) | dev box state |

One fetched page (the GitLab issue page of R-linux-26) contained text addressed to automated
readers; it was treated as page content and not acted on.
