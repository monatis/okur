#!/usr/bin/env bash
# Okur's live runs of web scenarios on Linux (run-scenario-linux.py, one after another), each
# run file written to D:\okur-linux\runs\<id>.json for `okur-lab live-parity` on the dev box.
# Run in the lab, from ~/okur:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/live-web.sh 02-doc-next-line ...
set -uo pipefail
cd ~/okur
# Each run's lines in the job's log as they are printed: a file takes Python's output in
# blocks, and full5's log stood 25 minutes behind its runs (2026-11-09).
export PYTHONUNBUFFERED=1
cargo build -q -p okur --bin okur || exit 1
# Okur's own dialogs (Insert+F7 and the others), started by okur run beside it; the runs go
# on without them where it does not build (GTK 4's development files missing).
cargo build -q -p okur-ui --bin okur-ui || echo "okur-ui not built: Okur's dialogs not shown"
cargo build -q -p okur-lab --bin okur-lab || exit 1
cargo build -q -p xa11y --features cli --bin xa11y || exit 1
cargo build -q -p okur-core --example script_key || exit 1
runs=/mnt/d/okur-linux/runs
mkdir -p "$runs"
for id in "$@"; do
    # An id, or (for research, never imported) a scenario not yet recorded: corpus/pending/...
    scenario=$(ls corpus/scenarios/"$id"*.json 2>/dev/null | head -1)
    out="$runs/$(basename "$scenario")"
    case "$id" in corpus/pending/*.json)
        scenario=$id
        mkdir -p /mnt/d/okur-linux/research
        out=/mnt/d/okur-linux/research/$(basename "$scenario") ;;
    esac
    if [ -z "$scenario" ]; then
        echo "$id: no scenario"
        continue
    fi
    python3 /mnt/d/okur-linux/run-scenario-linux.py "$scenario" "$out"
    pkill -f "user-data-dir=/tmp" 2>/dev/null
    sleep 1
done
