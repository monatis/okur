---
name: jaws-reference
description: Record or query live JAWS 2025 reference behavior on the remote peer old-pc (FSAPI COM script dispatch, keystrokes, speech-history capture, multi-trial scenarios). Use when you need ground truth for a JAWS announcement, key binding, or navigation boundary, when adding/re-recording corpus evidence, or when investigating a parity discrepancy.
---

# JAWS Reference (Claude Code adapter)

The canonical skill lives in `skills/jaws-reference/` (shared with other agent harnesses).
**Read `skills/jaws-reference/SKILL.md` first**, then the relevant file under
`skills/jaws-reference/references/` (fsapi, script-inventory, speech-capture,
experimentation, troubleshooting). This adapter only adds how to reach the JAWS machine.

## Where JAWS runs

The scripts talk to JAWS over in-process COM, so they must execute **on `old-pc`**, not here.

| | Local (dev box) | `old-pc` (reference) |
|---|---|---|
| OS | Windows 11 26200 | Windows 10 19045, interactive console session 1, user `ailabs` |
| Skill copy | `skills/jaws-reference/` | `C:\Users\ailabs\skills\jaws-reference\` |
| Dropbox | `C:\Users\Gigabyte\Dropbox\` | `C:\Users\ailabs\Dropbox\` |

Evidence must come from `old-pc` (`os_version` 10.0.19045). Never record reference evidence
on the dev box, even if JAWS happens to be running there.

## Running commands on old-pc

Either the `opendesk` MCP tools (`opendesk_exec` with `peer: "old-pc"`, plus `screenshot`,
`keyboard`, `ui`, `app` for GUI driving) or the CLI from Bash (works without MCP):

```bash
opendesk exec --peer old-pc --timeout 120 "powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\ailabs\skills\jaws-reference\scripts\probe.ps1"
```

`opendesk exec` runs as `ailabs` in the interactive session, which FSAPI requires.

`opendesk exec` stops waiting after `--timeout` seconds (default **30**). A scenario run is
cut off mid-trial, which leaves its fixture app and `okur-lab trace` running on `old-pc`. Pass
`--timeout 900` for single runs. For anything longer, use an OpenDesk background job (MCP
`opendesk_exec` with `background: true`, then `opendesk_job` with `poll` and `wait_ms`), which
holds no connection while it runs.

## Preconditions (every session)

1. **Probe**: run `probe.ps1` remotely; JAWS (`jfw`) must be running and speech history enabled.
2. **Skill copy in sync**: `lab-deploy.sh` (below) compares every script's hash on both sides
   and sends the ones that differ; "0 file(s) sent" means old-pc runs the repo's scripts.
3. **Serial only**: JAWS, focus, and clipboard on old-pc are global state. Never run two
   captures concurrently, and never touch old-pc's keyboard/clipboard while a capture runs.

## Deploying our tools to old-pc

Build locally, then install straight into place (OpenDesk 0.3 `opendesk push`, SHA-256
verified, the destination replaced only once the whole file arrived):

```bash
CARGO_TARGET_DIR=target/agent cargo build --release --bin okur
bash skills/jaws-reference/scripts/lab-deploy.sh            # --okur, --recorder, --recorder32
```

It sends `bin/okur.exe` and `bin/okur-lab.exe`, the skill's scripts, `corpus/fixtures` and `corpus/scenarios`, skipping
files already installed with the same hash. The Dropbox pair (`lab-push.ps1`, then `lab-pull.ps1
-PushId <id>` on old-pc) stays as a fallback; its `-PushId` is mandatory. After recording, check
the evidence's `provenance.capture_script_sha256` against your local `run-scenario.ps1` before
importing it.

## Recording evidence

Run long recordings with `run-batch.ps1` as a background job, writing to
`C:/Users/ailabs/sr-harness/evidence-out` (outside Dropbox); its `batch.log` lists each
scenario's exit code. Runs must be serial: never start a batch while another one, or a snapshot
capture, is running. New scenarios use `key` steps ("Down", "Shift+H", "Insert+Down"); see the
header of `run-scenario.ps1`.

In Git Bash, pass Windows paths to `opendesk exec` with forward slashes
(`C:/Users/ailabs/...`): backslashes before `$var` inside double quotes are mangled.

## Moving files (fixtures in, evidence out)

`opendesk push --peer old-pc <local> <remote>` and `opendesk pull --peer old-pc <remote> <local>`
copy one file, verified by SHA-256 (MCP: `opendesk_file`). `lab-fetch.sh <remote dir> <local
dir>` pulls a whole folder (evidence, traces, snapshots), skipping files already there. Copy
what it fetched into `corpus/` unmodified.

After copying evidence into `corpus/evidence/`, register it with `okur-lab corpus-manifest --write`
(the pre-commit gate rejects unregistered scenarios or mismatched counts).
