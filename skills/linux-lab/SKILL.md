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
  from a JAWS recording), 6 GB, 4 processors, swap on D:. The agent may change it (the owner,
  2026-11-05), keeping WSLg off and the swap off C:.
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
```
