#!/usr/bin/env bash
# Okur's live runs of web scenarios on Linux (run-scenario-linux.py, one after another), each
# run file written to D:\okur-linux\runs\<id>.json for `okur-lab live-parity` on the dev box.
# Run in the lab, from ~/okur:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/live-web.sh 02-doc-next-line ...
set -uo pipefail
cd ~/okur
cargo build -q -p okur-linux --example okur_linux || exit 1
cargo build -q -p xa11y --features cli --bin xa11y || exit 1
cargo build -q -p okur-core --example script_key || exit 1
runs=/mnt/d/okur-linux/runs
mkdir -p "$runs"
for id in "$@"; do
    scenario=$(ls corpus/scenarios/"$id"*.json 2>/dev/null | head -1)
    if [ -z "$scenario" ]; then
        echo "$id: no scenario"
        continue
    fi
    python3 /mnt/d/okur-linux/run-scenario-linux.py "$scenario" "$runs/$(basename "$scenario")"
    pkill -f "user-data-dir=/tmp" 2>/dev/null
    sleep 1
done
