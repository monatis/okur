#!/usr/bin/env bash
# Probe 1 of docs/plans/2026-11-05-linux-first.md: Chrome on a fixture page, inside the headless
# desktop, read through AT-SPI by the vendored xa11y's command-line tool. Passes when the page's
# heading and a paragraph are in the tree by role and name. Run in the lab from ~/okur:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-chrome.sh [fixture.html]
set -uo pipefail
fixture="${1:-doc-reading.html}"
cd ~/okur
cargo build -q -p xa11y --features cli --bin xa11y || exit 1
profile=$(mktemp -d)
# As root (the lab has no other user) Chrome needs --no-sandbox; the renderer's accessibility on
# from the start, as a screen reader would turn it on.
google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
    --force-renderer-accessibility "file://$HOME/okur/corpus/fixtures/$fixture" > /tmp/chrome.log 2>&1 &
chrome=$!
# Chrome joins the AT-SPI desktop some seconds after its window, and its page's tree after that.
tree=
for _ in $(seq 30); do
    sleep 2
    tree=$(./target/debug/xa11y tree --app "Google Chrome" 2>&1)
    if echo "$tree" | grep -q "virtual buffer"; then break; fi
done
echo "--- xa11y apps"
./target/debug/xa11y apps
echo "--- xa11y tree (Chrome)"
echo "$tree" | head -60
kill "$chrome" 2>/dev/null
wait "$chrome" 2>/dev/null
rm -rf "$profile"
if echo "$tree" | grep -qi "heading.*The Accessibility Engine Specification" \
    && echo "$tree" | grep -q "virtual buffer"; then
    echo "PROBE 1 PASS: the heading and the first paragraph are in Chrome's AT-SPI tree"
else
    echo "PROBE 1 FAIL"
    exit 1
fi
