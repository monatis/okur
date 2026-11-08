#!/usr/bin/env bash
# AT-SPI snapshots of the corpus's HTML fixtures (docs/plans/2026-11-05-linux-first.md §2.3):
# each fixture loaded in Chrome as run-scenario-linux.py loads it, and read whole by okur-linux's
# example atspi_snapshot, as Okur reads a page. Written to
# D:\okur-linux\snapshots\<fixture key, dots for slashes>.chrome.atspi.json, for
# fetch-snapshots.sh on the dev box. Run in the lab, inside the accessibility stack:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/capture-atspi-snapshots.sh \
#       doc-reading.html aria-at/checkbox/checkbox.html ...
# Fixture keys as the scenarios name them, under corpus/fixtures/. Prints one line per fixture,
# and fails if one was not captured.
set -uo pipefail
cd ~/okur || exit 1
cargo build -q -p okur-linux --example atspi_snapshot || exit 1
out=/mnt/d/okur-linux/snapshots
mkdir -p "$out"
chrome_version=$(google-chrome --version | tr -d '\n')
status=0
for fixture in "$@"; do
    file="corpus/fixtures/$fixture"
    if [ ! -f "$file" ]; then
        echo "$fixture: no such fixture"
        status=1
        continue
    fi
    stem="${fixture%.html}"
    stem="${stem//\//.}"
    sha=$(sha256sum "$file" | cut -d' ' -f1)
    profile=$(mktemp -d)
    ./target/debug/examples/atspi_snapshot "fixtures/$fixture" "$sha" "$chrome_version" \
        "$out/$stem.chrome.atspi.json" &
    capture=$!
    sleep 2
    google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
        --force-renderer-accessibility "file://$PWD/$file" > /dev/null 2>&1 &
    if ! wait "$capture"; then
        echo "$fixture: NOT captured"
        status=1
    fi
    # Chrome's processes by their profile: google-chrome is a wrapper that leaves them running.
    pkill -f "user-data-dir=$profile" 2> /dev/null
    sleep 1
    rm -rf "$profile"
done
exit $status
