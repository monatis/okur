#!/usr/bin/env bash
# From the dev box: brings the Linux lab's live runs (D:\okur-linux\runs\<scenario>.json on
# old-pc, written by run-scenario-linux.py) into corpus/live/linux/, each as the runner wrote it,
# then compares them with JAWS's evidence. Run from the repository root:
#   bash skills/linux-lab/scripts/fetch-runs.sh [--peer old-pc] <scenario-id>...
# Then `cargo run --bin okur-lab -- live-baseline linux --write` and commit the runs and the
# baseline together, apart from engine code. A run is Okur's speech, never evidence.
set -euo pipefail
PEER=old-pc
ids=()
while [ $# -gt 0 ]; do
    case "$1" in
        --peer) PEER=$2; shift 2 ;;
        *) ids+=("$1"); shift ;;
    esac
done
if [ ${#ids[@]} -eq 0 ]; then
    echo "usage: fetch-runs.sh [--peer old-pc] <scenario-id>..." >&2
    exit 2
fi
mkdir -p corpus/live/linux
# The lab's runs and traces in one archive, pulled once: two pulls a run took half an hour for
# 83 runs (2026-11-08). The archive is made on old-pc, checked by its size, and unpacked here.
staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT
# The runs are copied first, a file still being written (a job's run in progress) skipped:
# Compress-Archive gives up on the whole archive at one locked file.
timeout 300 opendesk exec --peer "$PEER" --timeout 280 \
    "robocopy D:/okur-linux/runs D:/okur-linux/runs-snap *.json *.jsonl /MIR /R:0 /W:0 /NP /NFL /NDL /NJH /NJS | Out-Null; Compress-Archive -Path D:/okur-linux/runs-snap/* -DestinationPath D:/okur-linux/runs.zip -Force; (Get-Item D:/okur-linux/runs.zip).Length" \
    2>&1 | tr -d '\000\r' | tail -1 > "$staging/size"
if ! timeout 600 opendesk pull --peer "$PEER" D:/okur-linux/runs.zip "$staging/runs.zip" > /dev/null; then
    echo "fetch-runs: the runs' archive was not fetched" >&2
    exit 1
fi
size=$(wc -c < "$staging/runs.zip" | tr -d ' ')
if [ "$size" != "$(cat "$staging/size")" ]; then
    echo "fetch-runs: the archive is $size bytes here, $(cat "$staging/size") on old-pc" >&2
    exit 1
fi
unzip -q -o "$staging/runs.zip" -d "$staging/runs"
failed=
for id in "${ids[@]}"; do
    scenario=$(ls corpus/scenarios/"$id"*.json 2>/dev/null | head -1)
    if [ -z "$scenario" ]; then
        echo "$id: no scenario" >&2
        continue
    fi
    name=$(basename "$scenario")
    if [ ! -f "$staging/runs/$name" ]; then
        echo "$name: NOT in the lab's runs" >&2
        failed=1
        continue
    fi
    if ! grep -q '"platform": "linux"' "$staging/runs/$name"; then
        echo "$name: not a Linux run, left out" >&2
        continue
    fi
    cp "$staging/runs/$name" "corpus/live/linux/$name"
    # Its trace, where the run kept one (okur-lab live-replay linux).
    trace="${name%.json}.trace.jsonl"
    if [ -f "$staging/runs/$trace" ]; then
        cp "$staging/runs/$trace" "corpus/live/linux/$trace"
    else
        rm -f "corpus/live/linux/$trace"
    fi
    echo "fetched $name ($(grep -o '"commit": "[0-9a-f]*"' "corpus/live/linux/$name"))"
done
if [ -n "$failed" ]; then
    echo "fetch-runs: some runs were not fetched" >&2
    exit 1
fi
