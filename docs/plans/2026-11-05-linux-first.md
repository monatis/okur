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
- **On the application's own window, not its frame (2026-11-08, eb0f0ced).** Under a reparenting
  window manager the root's children are its frames, and X reports a grab on an ancestor of the
  focus window as the focus leaving for it: GTK deactivated and activated its window around
  every key, repeated its focus, and its popovers could not keep their state. The hook grabs on
  the window holding WM_STATE and lets a frame's grabs go as a window is reparented into it; a
  grab on the focus window itself moves no focus (one `WindowDeactivated` in a whole run of 304).

## 2. Weeks 2 to 6

**Done the same day** ([`../handoff/2026-11-05-okur-on-linux-and-number-processing.md`](../handoff/2026-11-05-okur-on-linux-and-number-processing.md)): §2.1's keyboard, events and speech in `okur-linux`, each passing live in the lab, and Okur run whole (the runtime's event loop with Linux's platform) speaking GTK 4's focus. The keyboard grabs each key under any modifier (64 combinations apart overwhelmed the X server). Next: §2.3 before §2.2, since a web scenario needs the page: Chrome's AT-SPI tree has the shape of its UI Automation tree (headings with `level`, "static" text leaves; `probe-atspi-tree.sh`), so an AT-SPI snapshot and its `to_aom` in `okur-linux-model`, captured live through xa11y-linux, then `DocumentLoaded`; then §2.2. **Done the same evening:** the page read through AT-SPI into the AOM, Okur reading doc-reading.html on load in the lab (`probe-okur-web.sh`, WEB PASS).

1. `okur-linux`: keyboard (X11 first, the declared grab sets the proposal's C1 calls for), events
   (xa11y-linux extended in place: desktop-wide, registered), speech (SSIP), enough for the virtual
   cursor on a web page.
2. A live run in WSL of the first web scenarios (document reading, quick navigation) by
   `okur-linux` on Chrome under Xvfb, its results written by a tool into a Linux conformance
   baseline that may only improve. **Done 2026-11-05** (`13e232c0`): nine scenarios run by
   `run-scenario-linux.py`, kept in `corpus/live/linux/`, `okur-lab live-baseline linux`: 27/28
   steps match JAWS's Windows evidence. On the way: the focus in a page marked as in its document,
   a deadlock in xa11y-linux's subscription, a reload read on `Document:LoadComplete`, keys sent by
   keycode. Next: more scenarios (quick navigation, links, forms, tables), each added run kept
   and the baseline written.
3. `okur-linux-model` and Linux snapshots of the HTML fixtures, replayed on every OS.
4. Invariants for GTK and Qt controls (a check box, a combo box, a dialog said as on Windows).
   **Begun 2026-11-08** ([`../handoff/2026-11-08-linux-desktop-controls.md`](../handoff/2026-11-08-linux-desktop-controls.md)):
   SR Desktop Fixture's GTK 4 replica (`corpus/fixtures/linux/desktop-forms.py`) runs 301, 303
   and 304 in the lab against JAWS's Windows evidence, 411/424 steps with the web runs; open: a
   window's focus said as it comes forward, the drop-down's place and open list, the tree's
   depth; then a dialog, and Qt.

**Status, 2026-11-09 (evening):**
- **Live runs.** 356 scenarios at fcd2601b (the full run `full2`), 40 of them again at ccb4c70a (`gtk1`): 5674/6004 steps match JAWS (`corpus/live/linux-baseline.json`, 53a1ba66).
- **Replay.** `okur-lab live-replay linux` replays them off the lab with today's code: 5702/6004 match JAWS. It is ratcheted (`corpus/live/linux-replay-baseline.json`).
- **Found by setting the replay beside Windows' parity** (the steps Windows matches and Linux does not, 119 at ccb4c70a) **and built:**
  - Chrome's site information button is a menu button (a6b5b100).
  - Chrome's own menus are menu mode, their highlighted items said with their places (69c99756, 3dc4cbca). Measured live in `gtk2`: "Context Menu | To navigate press Up or Down Arrow. | New tab Ctrl+T | 1 of 21", Escape "Leaving menus".
  - Page progress bars say their values and changes; a table's click listeners are read (157e2b01).
  - Say System Time and Date on Linux (adc94d7a).
  - A text field read at its caret as its focus is delivered, as Windows' focus carries it (ac1304bd). Measured in `gtk2`: 676's colors "navy on yellow", "red on white", "black on white".
  - The focus asked for what cannot take it goes to its page (0a81b747: Route PC to Virtual on a graphic).
  - Live regions due together read in the order they changed (91f803a4).
  - The runner: `during` keys, JAWS's TypeKey and TypeString, a step's `quiet_ms` and `settle_max_ms`, and the lab's fixture path compared as old-pc's.
- **Okur's own dialogs on Linux:** the GTK 4 renderer (5d0cd15c) builds and passes clippy and its tests in the lab. In `gtk2` the element lists run, and the arrows read Okur's GTK list ("Settings 2 of 15"). Open:
  - The dialog came 5.4 s after Insert+F7, its focus 7.7 s. okur-ui now uses GTK's cairo renderer (21546c2c); to measure.
  - Keys pressed before the dialog's focus arrives go to the page (the first Down in 222).
  - A GTK list item cost about 130 ms to read, up to three reads per arrow. The reads are now parallel (de428ec5); to measure.
  - The renderer's second increment: trees, tables, check lists, menus, the wizard, the main window, the viewer.
- **Known Linux gaps, not yet built:**
  - Alt+F on Chrome highlights no item on Linux, so Okur says the menu only at the first Down (Windows' Chrome highlights the first item as it opens).
  - A live region added after its page's read (finding 7 of the second review): since 90d63662 a page changing outside its regions is captured again, which finds a new region at its next capture; its first change before that is not said.
  - Keys pressed between Okur's dialog asked for and its focus come go to the page (Insert+F7 then a letter at once is the page's quick key). The engine could hold them for the dialog.
  - A dialog box's box is read once per window (79a1208e): a box whose buttons or text change after (a wizard's page) keeps the first read.
  - An alert inside a frame (GTK's info bar) is taken as the window, and its focus as one in a window behind (the fourth review, 9).
  - A GTK list's arrow costs up to three reads (its focus, the selection going, coming), about 70 ms each.
  - Chrome's tabs and address bar: after Escape in the address bar Linux's Chrome keeps its focus there, and Down reads the address (220/s9); after Ctrl+W, the closed tab's late load is announced as a page with no headings (220/s12). Both matched by saying nothing before.
  - After the Form Fields List moved into a text area, Escape went to Chrome live (228/s11) though the replay, with okur-ui's records, leaves forms mode as JAWS does: a debug line (f5c9f29d) names the application region if that is the cause.
  - The visited links of JAWS's machine (its Chrome profile's history) are not visited in the lab's fresh profile (682, 907, 908): the environment's, as the fonts are.
  - GTK's drop-down count and tree depth (301, 303, 304).
  - The regions entered on a check box's focus after Space (288/s2: "MainRegion FormRegion").
  - The fonts the lab has (Liberation Serif for Times New Roman, 209), which are the environment's.
- **Reviews:** the third pass's findings 1 to 8 built (3dc4cbca); 9 (one call per table part's default action) is kept. The fourth pass's (Okur's GTK dialogs, the read-again rule, the regions' order) built in 240fb118, 79a1208e and b3f3639f; 9 is kept.
- **Since the handoff of the evening (e8f41bfe):** okur-ui's messages in the trace (fa7403d8), `job.sh stop`, the fourth review's fixes, a dialog box read once (79a1208e), okur-ui woken for each message (b3f3639f), a page's changes outside its regions captured again (90d63662). The full run `full5` (512 scenarios at 240fb118) runs; what came after it is checked in the run after it.
- **Handoffs:** [`../handoff/2026-11-09-linux-replay-exact-and-the-second-review.md`](../handoff/2026-11-09-linux-replay-exact-and-the-second-review.md), [`../handoff/2026-11-09-linux-text-fields-live-regions-and-wide1.md`](../handoff/2026-11-09-linux-text-fields-live-regions-and-wide1.md).

### Okur's own dialogs on Linux (begun 2026-11-09)

Okur's dialogs (the element lists, Find, PlaceMarkers, Skim Reading, Settings Center) are
`okur-ui`'s, drawn from the generic `okur_ui_model::DialogSpec` (about ten control kinds, trees,
menus, the main window, the wizard). The design, from `helper-ui-architecture.md` (GTK 4 on
Linux):
- A GTK 4 renderer in `okur-ui` behind `cfg(target_os = "linux")`, through gtk4-rs, a
  Linux-target dependency. The dev box cannot build gtk4-sys for Linux (its build script finds
  GTK through pkg-config), so the dev box's Linux clippy keeps to the crates it names; the lab's
  `check-linux.sh` builds and lints `okur-ui` too, with `libgtk-4-dev` from `setup.sh`.
- Okur reads its own GTK dialogs through AT-SPI like any GTK application's (301, 303 and 304
  already do), so the same reading makes them right, as on Windows.
- In order: the element lists (a list and its buttons: Insert+F7, F6, F5), then Find, the
  PlaceMarker list, Skim Reading, then Settings Center's tree.
- Each built against its scenarios replayed live in the lab, as the web ones are.

## 3. Meanwhile

The Windows work goes on beside it (the dictionary's rest, the Settings Center features of the
review plan, §3). A build in the lab is old-pc's one heavy job; JAWS rounds come first.
