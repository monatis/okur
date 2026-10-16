#!/usr/bin/env bash
# Usage: parity-snapshot.sh <out-dir> [okur.exe]
#
# Writes the whole output of the three parity runs (IA2, UIA, the speech recorder's spoken
# evidence) into <out-dir>. Take one before a change and one after, then compare them byte
# for byte: `diff -r before after`. A refactor or a fix that should change no recorded speech
# must leave them identical.
set -eu
OUT="$1"
OKUR="${2:-target/agent/release/okur.exe}"
mkdir -p "$OUT"
"$OKUR" parity --all > "$OUT/parity-all-ia2.txt" 2>&1 || true
"$OKUR" parity --all --api uia > "$OUT/parity-all-uia.txt" 2>&1 || true
"$OKUR" parity --spoken > "$OUT/parity-spoken.txt" 2>&1 || true
tail -n 1 "$OUT/parity-all-ia2.txt" "$OUT/parity-all-uia.txt" "$OUT/parity-spoken.txt"
