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
failed=
for id in "${ids[@]}"; do
    scenario=$(ls corpus/scenarios/"$id"*.json 2>/dev/null | head -1)
    if [ -z "$scenario" ]; then
        echo "$id: no scenario" >&2
        continue
    fi
    name=$(basename "$scenario")
    # A pull that fails (the peer briefly unreachable) is said, and the rest still fetched.
    if ! timeout 120 opendesk pull --peer "$PEER" "D:/okur-linux/runs/$name" "corpus/live/linux/$name" > /dev/null; then
        echo "$name: NOT fetched" >&2
        failed=1
        continue
    fi
    if ! grep -q '"platform": "linux"' "corpus/live/linux/$name"; then
        echo "$name: not a Linux run, removed" >&2
        rm -f "corpus/live/linux/$name"
        continue
    fi
    # Its trace, where the run kept one (okur-lab live-replay linux).
    trace="${name%.json}.trace.jsonl"
    if ! timeout 120 opendesk pull --peer "$PEER" "D:/okur-linux/runs/$trace" "corpus/live/linux/$trace" > /dev/null 2>&1; then
        rm -f "corpus/live/linux/$trace"
    fi
    echo "fetched $name ($(grep -o '"commit": "[0-9a-f]*"' "corpus/live/linux/$name"))"
done
if [ -n "$failed" ]; then
    echo "fetch-runs: some runs were not fetched" >&2
    exit 1
fi
