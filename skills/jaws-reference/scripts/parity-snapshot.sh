#!/usr/bin/env bash
# Usage: parity-snapshot.sh <out-dir> [okur-lab.exe]
#
# Writes the whole output of the three parity runs (IA2, UIA, the speech recorder's spoken
# evidence) into <out-dir>. Take one before a change and one after, then compare them byte
# for byte: `diff -r before after`. A refactor or a fix that should change no recorded speech
# must leave them identical.
# Without an okur-lab.exe, it builds the working tree's (target/debug) first: a binary named
# by default was once older than the change, and both runs of a comparison used it (c56173d8,
# 2026-11-04). Naming one, build it from the tree being measured.
set -eu
OUT="$1"
if [ -n "${2:-}" ]; then
    OKUR_LAB="$2"
else
    CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" cargo build -q -p okur-lab --bin okur-lab
    OKUR_LAB=target/debug/okur-lab.exe
fi
mkdir -p "$OUT"
"$OKUR_LAB" parity --all > "$OUT/parity-all-ia2.txt" 2>&1 || true
"$OKUR_LAB" parity --all --api uia > "$OUT/parity-all-uia.txt" 2>&1 || true
"$OKUR_LAB" parity --spoken > "$OUT/parity-spoken.txt" 2>&1 || true
tail -n 1 "$OUT/parity-all-ia2.txt" "$OUT/parity-all-uia.txt" "$OUT/parity-spoken.txt"
