#!/usr/bin/env bash
# Usage: measure-edit-keys.sh <okur.exe> <process> <title> <log>
# (the keys are sent and the latency read by okur-lab.exe beside okur.exe, or $OKUR_KEYS)
#
# Live latency of everyday edit keys, on a machine the user lets the agent drive (dev box:
# only with their permission). Runs that okur (low volume) on the window whose title contains
# <title>, plays Ctrl+Home, 25 Right, 25 Left, 15 Ctrl+Right, 15 Ctrl+Left and 10 Down with
# human spacing, stops it, and prints `okur-lab latency`'s key-to-speech lines and the field reads.
# The window must be a text field ready for keys (with JAWS running, put JAWS in forms mode on
# it first: JAWS takes arrows in browse mode). Paths are Windows paths (C:/...).
set -eu
EXE="$1"; PROC="$2"; TITLE="$3"; LOG="$4"
KEYS="${OKUR_KEYS:-$(dirname "$EXE")/okur-lab.exe}"
rm -f "$LOG"
"$KEYS" front "$TITLE" > /dev/null
PID_FILE="$LOG.pid"
powershell -NoProfile -Command "\$p = Start-Process -FilePath '$EXE' -ArgumentList @('run','--volume','10','--debug-log','$LOG') -WindowStyle Hidden -PassThru; \$p.Id | Out-File -Encoding ascii '$PID_FILE'"
sleep 4
"$KEYS" front "$TITLE" > /dev/null
steps=("Ctrl+Home" "wait:600")
for _ in $(seq 25); do steps+=("Right" "wait:300"); done
for _ in $(seq 25); do steps+=("Left" "wait:300"); done
for _ in $(seq 15); do steps+=("Ctrl+Right" "wait:350"); done
for _ in $(seq 15); do steps+=("Ctrl+Left" "wait:350"); done
for _ in $(seq 10); do steps+=("Down" "wait:350"); done
"$KEYS" keys --allow "$PROC" --title "$TITLE" "${steps[@]}" > /dev/null || true
sleep 1
powershell -NoProfile -Command "\$id = [int](Get-Content '$PID_FILE'); Stop-Process -Id \$id -Confirm:\$false"
rm -f "$PID_FILE"
"$KEYS" latency "$LOG" | grep -E "step|key to speech|text context read"
