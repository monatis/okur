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
cut off mid-trial, which leaves its fixture app and `okur trace` running on `old-pc`. Pass
`--timeout 900` for single runs. For anything longer, launch `run-batch.ps1` detached
(`Start-Process`) and poll `batch.log` in the shared Dropbox.

## Preconditions (every session)

1. **Probe**: run `probe.ps1` remotely; JAWS (`jfw`) must be running and speech history enabled.
2. **Skill copy in sync**: compare hashes before trusting remote scripts:
   ```bash
   opendesk exec --peer old-pc "Get-ChildItem C:\Users\ailabs\skills\jaws-reference\scripts\*.ps1 | % { (Get-FileHash $_.FullName).Hash.ToLower() + ' ' + $_.Name }"
   ```
   and `certutil -hashfile <file> SHA256` locally. If they differ, push the repo version (below).
3. **Serial only**: JAWS, focus, and clipboard on old-pc are global state. Never run two
   captures concurrently, and never touch old-pc's keyboard/clipboard while a capture runs.

## Deploying our tools to old-pc

Build locally, then push and pull with the hash-verified pair:

```bash
cargo build -p okur
powershell -NoProfile -ExecutionPolicy Bypass -File skills/jaws-reference/scripts/lab-push.ps1   # prints PushId
```
then on old-pc (via `opendesk_exec`):
`powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\ailabs\skills\jaws-reference\scripts\lab-pull.ps1 -PushId <id>`

`-PushId` is mandatory. Without it, a stale manifest that still matches stale files would
"verify" and silently install an old version; this happened once on 2026-10-14 and was caught by
the evidence `provenance.capture_script_sha256`. After recording, always check that hash against
your local `run-scenario.ps1` before importing evidence.

## Recording evidence

Run long recordings detached with `run-batch.ps1` (writes `batch.log` and a final `batch.done` to
its `-OutputDir`, ideally under `C:\Users\ailabs\Dropbox\sr-harness\`), then poll for `batch.done`
from the dev box. Runs must be serial: never start a batch while another one, or a snapshot
capture, is running. New scenarios use `key` steps ("Down", "Shift+H", "Insert+Down"); see the
header of `run-scenario.ps1`.

In Git Bash, pass Windows paths to `opendesk exec` with forward slashes
(`C:/Users/ailabs/...`): backslashes before `$var` inside double quotes are mangled.

## Moving files (fixtures in, evidence out)

Use the shared Dropbox: write to `C:\Users\Gigabyte\Dropbox\sr-harness\...` locally, then poll
on old-pc until `C:\Users\ailabs\Dropbox\sr-harness\...` exists **and its SHA-256 matches**
(Dropbox sync is asynchronous; never assume it has landed). Evidence flows back the same way.
For small text payloads, `opendesk exec` with a here-string is acceptable.

After copying evidence into `corpus/evidence/`, register it in `corpus/CORPUS_MANIFEST.json`
(the pre-commit gate rejects unregistered scenarios or mismatched counts).
