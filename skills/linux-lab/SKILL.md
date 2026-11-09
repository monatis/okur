---
name: linux-lab
description: Okur's headless Linux lab, a WSL distribution on old-pc, for building and testing the Linux driver. Use when building or testing anything for Linux (okur-linux, xa11y-linux, AT-SPI probes), or when syncing the repository into the lab.
---

# Linux lab (headless WSL on old-pc)

Linux is Okur's first cross-platform target (the owner, 2026-11-05;
`docs/plans/2026-11-05-linux-first.md`, research in `docs/references/linux-lab-proposal.md`).

## Where

- old-pc, WSL distribution **`okur`** (Ubuntu 26.04) at `D:\okur-linux\wsl`. The owner's own
  `Ubuntu-22.04` there is not ours: leave it.
- `C:\Users\ailabs\.wslconfig`: `guiApplications=false` (WSLg's window must never take the focus
  from a JAWS recording), 6 GB, 4 processors, swap on D:, and `localhostForwarding=false`: a
  Linux port forwarded to Windows made Windows Defender Firewall ask about COM Surrogate
  (`dllhost.exe`) on 2026-11-05, a prompt that sat on old-pc's screen through three JAWS rounds
  and that only the owner answers. The agent may change the file (the owner, 2026-11-05),
  keeping WSLg off and the swap off C:.
- The repository inside: `~/okur` (root's home in the distribution).

## Rules

- old-pc is the JAWS lab first. `run-batch.ps1` shuts WSL down before a batch; never start Linux
  work while a batch runs (`batch.done` missing), and never show Linux on old-pc's screen.
- No GUI, no audio to old-pc's speakers: everything runs under `a11y-stack.sh` (Xvfb, a private
  session bus, AT-SPI) and speech goes to a null sink.
- A build in the lab is old-pc's one heavy job; JAWS rounds come first.

## How

```bash
# once, after the distribution is installed (dns.sh: WSL's NAT fails on old-pc, its fallback needs public DNS)
bash skills/linux-lab/scripts/sync.sh
opendesk exec --peer old-pc --timeout 120 "wsl.exe -d okur -u root -- bash /mnt/d/okur-linux/dns.sh"
opendesk exec --peer old-pc --timeout 1800 "wsl.exe -d okur -u root -- bash /mnt/d/okur-linux/setup.sh"
# each time: send the committed branch, then build or test in the lab
bash skills/linux-lab/scripts/sync.sh
opendesk exec --peer old-pc --timeout 1800 "wsl.exe -d okur -u root -- bash -lc 'cd ~/okur && cargo test -p okur-core'"
# a command inside the headless desktop (Chrome, GTK and AT-SPI available)
opendesk exec --peer old-pc --timeout 600 "wsl.exe -d okur -u root -- bash -lc 'cd ~/okur && bash /mnt/d/okur-linux/a11y-stack.sh <command>'"
```

Anything longer than `opendesk exec`'s timeout (a first build, a probe) runs as a detached job.
The command is written to a bash file in the lab (no quoting through PowerShell), runs as root in
`~/okur` with rustup's PATH, and leaves `D:\okur-linux\jobs\<name>.log` and `<name>.done` (its exit
code):

```bash
bash skills/linux-lab/scripts/job.sh start probe1 'bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-chrome.sh'
bash skills/linux-lab/scripts/job.sh wait probe1 90   # in the background: until done, then the exit and the log's last 90 lines
bash skills/linux-lab/scripts/job.sh log probe1       # the log so far
bash skills/linux-lab/scripts/job.sh stop probe1      # the distribution terminated: the job and its stack (Xvfb, Chrome, Okur) end
```

A job checks the code first (`check-linux.sh`), then runs. Read the check's line before leaving a
long run alone: `live-web.sh` goes on without okur-ui where okur-ui does not build, and Okur's own
dialogs are then silent in every run (full4 at fa7403d8, stopped and run again).

## Live runs, traces and their replay

Okur's live runs of the corpus's scenarios on Linux. Each run is `okur run --engine no-speech`,
with a settings folder of its own: the settings JAWS recorded the scenario with
(`okur-lab reference-settings --evidence`). It keeps its speech log and its trace, what the driver
read for each AT-SPI event. A run is Okur's speech, never evidence.

```bash
# the check (clippy and the Linux crates' tests), then runs, in one job (ids: corpus/live/linux/*.json)
bash skills/linux-lab/scripts/job.sh start runs1 "bash /mnt/d/okur-linux/check-linux.sh; bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/live-web.sh 01-doc-say-all 301-desktop-tab-order ..."
# the runs and their traces, in one archive (about half a minute for 83 runs)
bash skills/linux-lab/scripts/fetch-runs.sh 01-doc-say-all 301-desktop-tab-order ...
# against JAWS's evidence, a baseline that may only improve; commit the runs and it apart from code
cargo run --bin okur-lab -- live-baseline linux [--write]
# each run replayed from its trace through the converter and the engine, against the run and JAWS
cargo run --bin okur-lab -- live-replay linux [--only <id prefix>] [--pages-after-focus] [--recorded-settings]
```

- The replay says what the run said on all but a run's Say All after a page load. Its speech
  reports no marks, so a Say All does not go down the page as the silent synthesizer's does live.
- `--recorded-settings` and `--pages-after-focus` try a change on the runs before the lab runs it.
- okur-ui's messages (a dialog's choices, its buttons, its close) are records of the trace since
  fa7403d8, so a run made after it replays its dialogs whole.
- A full run: the list of every scenario with an HTML fixture or a Linux replica and JAWS's
  evidence, ARIA-AT's left for a run of their own (512 scenarios, about ten hours), pushed to
  `D:\okur-linux\full3-ids.txt` and given to `live-web.sh` as `$(tr -d '\r' < /mnt/d/okur-linux/full3-ids.txt)`.
- Compare the replay with Windows' parity to find Linux's own gaps: the steps
  `corpus/parity-baseline.json` has matched and `corpus/live/linux-replay-baseline.json` has not.
- Before committing traces, check them for anything but fixture content (application names,
  addresses, e-mail addresses).

## Probes

- `probe-read-speed.sh <fixtures>`: a page read whole, in order and in parallel, three rounds each,
  with whether both reads gave the same tree.
- `capture-atspi-snapshots.sh` and `fetch-snapshots.sh`: AT-SPI snapshots of the HTML fixtures,
  for `okur-lab parity --api atspi`.
- The other `probe-*.sh` scripts are earlier questions (Chrome, GTK, X11 grabs, speech): read each
  one's header before running it.
