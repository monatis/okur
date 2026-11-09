#!/usr/bin/env bash
# What a live region's change costs Okur on Linux (the code review of 2026-11-09, finding 2):
# Chrome on live-ticker.html, a page of about 2,000 objects whose status region ticks every
# second, with `okur run` (the silent synthesizer) for 30 s, Tab pressed six times. Prints how
# many times the region and the page were read again, their read times, and the Tabs said.
# Passes when the region was read for its ticks and spliced into the page, the page itself
# read again at most twice, and every Tab was said. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-live-ticker.sh
set -uo pipefail
cd ~/okur
cargo build -q -p okur --bin okur || exit 1
out=$(mktemp -d)
RUST_LOG=okur_linux=debug,okur_core=debug timeout 40 ./target/debug/okur run --engine no-speech \
    --settings "$out/settings" --speech-log "$out/speech.jsonl" > "$out/okur" 2>&1 &
okur=$!
sleep 4
profile=$(mktemp -d)
google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
    --force-renderer-accessibility "file://$HOME/okur/corpus/fixtures/linux/live-ticker.html" \
    > "$out/chrome.log" 2>&1 &
sleep 14
for _ in 1 2 3 4 5 6; do xdotool key Tab; sleep 2; done
wait "$okur"
pkill -f "user-data-dir=$profile" 2>/dev/null
rm -rf "$profile"
log=$(sed 's/\x1b\[[0-9;]*m//g' "$out/okur")
regions=$(grep -c 'a live region read again' <<< "$log")
pages=$(grep -c 'the changed page read again' <<< "$log")
echo "region reads: $regions; their times (ms):"
grep 'a live region read again' <<< "$log" | grep -o 'ms=[0-9]*' | sort -t= -k2 -n | tr '\n' ' '
echo
echo "page reads after a change: $pages"
grep -o 'page read[^"]*ms=[0-9]*' <<< "$log" | tail -10
echo "--- what was said for the Tabs"
python3 -c "
import json, sys
for line in open(sys.argv[1]):
    entry = json.loads(line)
    print(entry.get('kind'), '|', (entry.get('text') or '')[:80])" "$out/speech.jsonl" 2>/dev/null \
    | grep -v '^None | Tick\|Tick [0-9]' | tail -12
said=$(grep -c 'First\|Second\|Third\|Fourth\|Fifth\|Sixth' "$out/speech.jsonl" 2>/dev/null)
rm -rf "$out"
echo "Tabs said: ${said:-0} of 6"
if [ "$regions" -ge 5 ] && [ "$pages" -le 2 ] && [ "${said:-0}" -ge 6 ]; then
    echo "LIVE TICKER PASS"
else
    echo "LIVE TICKER FAIL"
    exit 1
fi
