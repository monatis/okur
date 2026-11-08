#!/usr/bin/env bash
# How fast pages are read whole, in order and in parallel (the code review of 2026-11-08,
# finding 6): each fixture loaded in Chrome as run-scenario-linux.py loads it, and read by
# okur-linux's example atspi_read_speed, a few rounds each way. Run in the lab, inside the
# accessibility stack:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-read-speed.sh \
#       color-grid.html article.html ...
# Prints one line per round: each read's milliseconds, the objects, and whether both reads gave
# the same tree.
set -uo pipefail
cd ~/okur || exit 1
cargo build -q -p okur-linux --example atspi_read_speed || exit 1
for fixture in "$@"; do
    file="corpus/fixtures/$fixture"
    if [ ! -f "$file" ]; then
        echo "$fixture: no such fixture"
        continue
    fi
    profile=$(mktemp -d)
    ./target/debug/examples/atspi_read_speed "fixtures/$fixture" 3 &
    probe=$!
    sleep 2
    google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
        --force-renderer-accessibility "file://$PWD/$file" > /dev/null 2>&1 &
    wait "$probe" || echo "$fixture: NOT read"
    pkill -f "user-data-dir=$profile" 2> /dev/null
    sleep 1
    rm -rf "$profile"
done
