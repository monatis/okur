#!/usr/bin/env bash
# Usage: preview-round.sh <fetched round dir> [okur-lab.exe]
#
# Replays a round's evidence before it is imported: the scenarios recorded so far (each
# <id>-evidence.json in the fetched dir, with its traces), their scenario files from
# corpus/pending, and the corpus's fixtures and snapshots, gathered into a scratch corpus next
# to the fetched dir, then `okur-lab parity -c` over it. A round of 48 scenarios takes hours;
# fetch what is done (lab-fetch.sh skips what it has) and preview it while the rest records.
# Nothing in corpus/ is touched.
set -eu
FETCHED="${1%/}"
OKUR_LAB="${2:-target/release/okur-lab.exe}"
SCRATCH="$FETCHED-preview"
rm -rf "$SCRATCH"
mkdir -p "$SCRATCH/scenarios" "$SCRATCH/evidence" "$SCRATCH/traces"
cp -r corpus/fixtures corpus/snapshots "$SCRATCH/"
count=0
for evidence in "$FETCHED"/*-evidence.json; do
    [ -e "$evidence" ] || continue
    id=$(basename "$evidence" -evidence.json)
    scenario="corpus/pending/$id.json"
    [ -e "$scenario" ] || scenario="corpus/scenarios/$id.json"
    if [ ! -e "$scenario" ]; then
        echo "no scenario file for $id" >&2
        continue
    fi
    cp "$scenario" "$SCRATCH/scenarios/"
    cp "$evidence" "$SCRATCH/evidence/"
    cp "$FETCHED"/traces/"$id"-trial*.jsonl "$SCRATCH/traces/" 2>/dev/null || true
    count=$((count + 1))
done
echo "$count scenario(s) in $SCRATCH"
"$OKUR_LAB" parity -c "$SCRATCH" 2>&1 | grep -E '^\[DIFF|^    (ours|jaws)|^JAWS parity' || true
